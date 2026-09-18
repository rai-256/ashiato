// SPDX-License-Identifier: AGPL-3.0-only
//! ST12 の書庫台帳移行を本物の PostgreSQL で確かめる。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use crate::{heartbeat, store_heartbeat, store_one, IngestRequest, StoreOutcome};
use std::path::Path;

fn archive_request(user_id: uuid::Uuid, raw: &str) -> IngestRequest {
    IngestRequest {
        id: uuid::Uuid::new_v4(),
        user_id,
        logical_source: "c03-youtube-watch".into(),
        external_id: None,
        device_id: Some("s01-c03".into()),
        origin: "collected".into(),
        event_time: chrono::DateTime::parse_from_rfc3339("2026-09-12T03:00:00Z")
            .unwrap()
            .to_utc(),
        tz_offset_min: 0,
        tz_id: "UTC".into(),
        schema_version: 1,
        unit_system: None,
        crs: None,
        source_updated_at: None,
        external_ref: None,
        raw: raw.into(),
        payload: serde_json::json!({}),
    }
}

/// 格納関門は HTTP の JSON 解釈を通さなくても、新規・重複・削除済み・拒否を区別する。
#[tokio::test]
async fn store_one_outcome_distinguishes_archive_results() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let first = archive_request(user, r#"{"watch":"first"}"#);

    // Scenario: 同じ書庫をもう一度置いても行が増えない
    assert!(matches!(
        store_one(&pool, first.clone()).await.unwrap(),
        StoreOutcome::Inserted(_)
    ));
    assert!(matches!(
        store_one(&pool, first.clone()).await.unwrap(),
        StoreOutcome::Duplicate(_)
    ));

    // Scenario: 消した記録は書庫を置き直しても戻らない
    sqlx::query("UPDATE core.event SET deleted_at = now() WHERE id = $1")
        .bind(first.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store_one(&pool, first).await.unwrap(),
        StoreOutcome::DuplicateOfDeleted(_)
    ));

    let mut invalid = archive_request(user, r#"{"watch":"invalid"}"#);
    invalid.origin = "unknown".into();
    assert!(matches!(
        store_one(&pool, invalid).await.unwrap(),
        StoreOutcome::Rejected(_)
    ));
}

/// 取り込み器は HTTP を経ずに生存信号を 1 件だけ格納でき、再起動後も重複しない。
#[tokio::test]
async fn store_heartbeat_keeps_the_http_idempotency_rule() {
    let pool = testdb::pool().await;
    let request = heartbeat::HeartbeatRequest {
        id: uuid::Uuid::new_v4(),
        user_id: testdb::user(),
        logical_source: "s01-archive-inbox".into(),
        device_id: Some("s01-c03".into()),
        emitted_at: chrono::DateTime::parse_from_rfc3339("2026-09-12T03:00:00Z")
            .unwrap()
            .to_utc(),
        capturable: true,
        blockers: vec![],
        attempts: 2,
        successes: 2,
        raw: r#"{"archive_inbox":true}"#.into(),
    };
    assert!(
        !store_heartbeat(&pool, request.clone())
            .await
            .unwrap()
            .duplicate
    );
    assert!(store_heartbeat(&pool, request).await.unwrap().duplicate);
}

/// Scenario: 最終日はいちばん新しい出来事の日
/// Scenario: 日本時間で日をまたぐ出来事は日本時間の日になる
#[tokio::test]
async fn archives_status_uses_ledger_max_event_time_in_japan() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let ledger: i64 = sqlx::query_scalar(
        "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome, created_at)
         VALUES ($1, repeat('b', 64), 'test', 'read', '2026-09-12T00:00:00Z') RETURNING id",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.archive_ledger_source (ledger_id, logical_source, max_event_at)
         VALUES ($1, 'c03-youtube-watch', '2026-09-11T15:30:00Z')",
    )
    .bind(ledger)
    .execute(&pool)
    .await
    .unwrap();

    let status = crate::archives_status_for(&pool, user).await.unwrap();
    let source = status
        .sources
        .into_iter()
        .find(|source| source.logical_source == "c03-youtube-watch")
        .unwrap();
    assert_eq!(source.last_event_on.as_deref(), Some("2026-09-12"));
}

/// Scenario: 設定を指定しなければ写しが残る
#[test]
fn archive_config_defaults_and_rejects_a_misspelling() {
    let env = std::collections::BTreeMap::new();
    let config = crate::archive::config::from_values(&env).unwrap();
    assert!(config.keep_copies);
    assert!(config.user_id.is_none());

    let mut invalid = std::collections::BTreeMap::new();
    invalid.insert("ASHIATO_ARCHIVE_KEEP_COPIES".into(), "flase".into());
    assert!(crate::archive::config::from_values(&invalid).is_err());
}

fn write_file(path: &Path, name: &str, content: &[u8]) {
    std::fs::write(path.join(name), content).unwrap();
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    use std::io::Write as _;
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    for (name, contents) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(contents).unwrap();
    }
    zip.finish().unwrap();
}

/// Scenario: 分割書庫は 1 本ずつ読まれる
/// Scenario: 読めない形の書庫は台帳に残る
#[test]
fn archive_open_lists_each_zip_and_classifies_unreadable_formats() {
    let root = std::env::temp_dir().join(format!("ashiato-archive-open-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let zip_path = root.join("takeout-20260912-001.zip");
    write_zip(&zip_path, &[("Takeout/YouTube/watch-history.json", b"[]")]);

    let files = crate::archive::open::open_archive(&zip_path).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "Takeout/YouTube/watch-history.json");
    assert_eq!(files[0].bytes, b"[]");

    let invalid = root.join("takeout-20260912.tgz");
    std::fs::write(&invalid, b"not a tar file").unwrap();
    assert_eq!(
        crate::archive::open::open_archive(&invalid)
            .unwrap_err()
            .kind(),
        "unsupported_format"
    );

    std::fs::write(&zip_path, b"not a zip").unwrap();
    assert_eq!(
        crate::archive::open::open_archive(&zip_path)
            .unwrap_err()
            .kind(),
        "broken_zip"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Scenario: 対象でないファイルからは記録が作られない
/// Scenario: HTML のマイアクティビティは読めなかったものとして残る
/// Scenario: JSON と同居する HTML は読まなかったに数える
#[test]
fn archive_classify_prefers_json_shapes_and_accounts_for_html() {
    let files = vec![
        crate::archive::open::ArchiveFile { path: "日本語/視聴.json".into(), bytes: br#"[{"header":"YouTube","time":"2026-01-01T00:00:00Z","titleUrl":"https://www.youtube.com/watch?v=x"}]"#.to_vec() },
        crate::archive::open::ArchiveFile { path: "YouTube/watch-history.html".into(), bytes: b"<html/>".to_vec() },
        crate::archive::open::ArchiveFile { path: "Photos/a.jpg".into(), bytes: b"photo".to_vec() },
    ];
    let result = crate::archive::classify::classify_files(&files);
    assert_eq!(result.known.len(), 1);
    assert_eq!(result.skipped, 2, "JSONと同居するHTMLと写真は読まない");
    assert_eq!(result.unreadable, 0);
    let html_only =
        crate::archive::classify::classify_files(&[crate::archive::open::ArchiveFile {
            path: "My Activity.html".into(),
            bytes: b"<html/>".to_vec(),
        }]);
    assert_eq!(html_only.unreadable, 1);
}

#[test]
fn archive_worker_inspects_a_candidate_before_parsing_it() {
    let root = std::env::temp_dir().join(format!("ashiato-inspect-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("one.zip");
    write_zip(
        &path,
        &[(
            "watch.json",
            br#"[{"header":"YouTube","time":"x","titleUrl":"https://youtube.com/watch?v=x"}]"#,
        )],
    );
    let candidate = crate::archive::scan::ScanCandidate {
        path,
        from_downloads: false,
        sha256: "x".into(),
        disposition: crate::archive::scan::ScanDisposition::Read,
    };
    let inspected = crate::archive::worker::inspect(candidate).unwrap();
    assert_eq!(inspected.known, 1);
    std::fs::remove_dir_all(root).unwrap();
}

/// Scenario: 原文は書庫のバイト列の一部と一致する
#[test]
fn archive_slice_returns_each_array_element_as_original_bytes() {
    let input = br#"[ {"title":"a]","nested":{"x":1}}, {"title":"b\"q"} ]"#;
    let items = crate::archive::slice::array_items(input).unwrap();
    assert_eq!(
        items,
        [
            br#"{"title":"a]","nested":{"x":1}}"#.as_slice(),
            br#"{"title":"b\"q"}"#.as_slice()
        ]
    );
}

/// 非 UTF-8 の項目をそのまま `raw` に渡すと、原文を文字列として保持する
/// 格納口との契約を破る。壊れた項目として数え、格納しない。
#[test]
fn archive_slice_rejects_a_non_utf8_item() {
    let input = b"[{\"title\":\"\xFF\"}]";
    assert!(crate::archive::slice::array_items(input).is_err());
}

/// 1 MiB の読み取り境界の前後にある項目でも、原文を欠かさず切り出す。
#[test]
fn archive_slice_keeps_an_item_across_a_mebibyte_boundary() {
    let padding = "x".repeat(1024 * 1024);
    let input = format!("[{{\"title\":\"{padding}\"}},{{\"title\":\"second\"}}]");
    let items = crate::archive::slice::array_items(input.as_bytes()).unwrap();

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].len(), 1024 * 1024 + 12);
    assert_eq!(items[1], br#"{"title":"second"}"#);
}

/// 200 MiB の Records.json でも、読み手へは常に 1 項目ずつ渡す。
#[test]
fn archive_slice_large_streams_one_item_at_a_time() {
    use std::io::Write as _;

    let path = std::env::temp_dir().join(format!("ashiato-Records-{}.json", uuid::Uuid::new_v4()));
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(b"[").unwrap();
    for item in 0..200 {
        if item != 0 {
            file.write_all(b",").unwrap();
        }
        write!(file, "{{\"location\":\"{}\"}}", "x".repeat(1024 * 1024)).unwrap();
    }
    file.write_all(b"]").unwrap();
    drop(file);

    let mut received = 0usize;
    let mut in_flight = 0usize;
    let mut max_in_flight = 0usize;
    crate::archive::slice::stream_array_items(std::fs::File::open(&path).unwrap(), |item| {
        in_flight += 1;
        max_in_flight = max_in_flight.max(in_flight);
        assert!(item.starts_with(br#"{"location":"#));
        received += 1;
        in_flight -= 1;
        Ok(())
    })
    .unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(received, 200);
    assert_eq!(max_in_flight, 1);
}

/// Scenario: ずれを持つ時刻はそのずれで残る
/// Scenario: UTC しか持たない時刻は UTC で残る
/// Scenario: UTC しか持たない時刻には取得元が地域を持たなかった印が付く
#[test]
fn archive_tz_uses_source_offset_or_marks_utc_as_unknown() {
    let offset = crate::archive::timezone::from_rfc3339("2026-01-02T03:04:05+09:00").unwrap();
    assert_eq!(
        (offset.offset_min, offset.id.as_str(), offset.from_source),
        (540, "Etc/GMT-9", true)
    );
    let utc = crate::archive::timezone::from_rfc3339("2026-01-02T03:04:05Z").unwrap();
    assert_eq!(
        (utc.offset_min, utc.id.as_str(), utc.from_source),
        (0, "UTC", false)
    );

    let explicit =
        crate::archive::timezone::from_timestamp("2026-01-02T03:04:05Z", Some(540)).unwrap();
    assert_eq!(
        (
            explicit.offset_min,
            explicit.id.as_str(),
            explicit.from_source
        ),
        (540, "Etc/GMT-9", true)
    );
}

/// Scenario: タイムラインの訪問と経路の点は別の論理ソースに入る
#[test]
fn archive_parse_timeline_separates_all_record_kinds() {
    let input = br#"{"semanticSegments":[{"visit":{"startTime":"2026-01-01T00:00:00Z"},"activity":{"startTime":"2026-01-01T00:00:01Z"},"timelinePath":[{"time":"2026-01-01T00:01:00Z"}]}],"rawSignals":[{"time":"2026-01-01T00:02:00Z"}]}"#;
    let records = crate::archive::timeline::parse(input).unwrap();
    assert_eq!(
        records
            .iter()
            .map(|record| record.logical_source)
            .collect::<Vec<_>>(),
        [
            "c03-timeline-visit",
            "c03-timeline-move",
            "c03-timeline-route",
            "c03-timeline-signal"
        ]
    );
}

/// Scenario: 移行前のロケーション履歴は読み終えると退役する
#[test]
fn archive_parse_legacy_accepts_records_and_semantic_history_timestamps() {
    let records = crate::archive::legacy::parse_records(
        br#"{"locations":[{"timestamp":"2024-08-31T23:00:00Z"}]}"#,
    )
    .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].logical_source, "c03-legacy-location");
    assert_eq!(
        records[0].event_time.to_rfc3339(),
        "2024-08-31T23:00:00+00:00"
    );

    let semantic = crate::archive::legacy::parse_semantic(
        br#"{"timelineObjects":[{"placeVisit":{"duration":{"startTimestampMs":"1725148800000"}}},{"activitySegment":{"duration":{"startTimestamp":"2024-09-01T01:00:00Z"}}}]}"#,
    )
    .unwrap();
    assert_eq!(
        semantic
            .iter()
            .map(|record| record.logical_source)
            .collect::<Vec<_>>(),
        ["c03-legacy-visit", "c03-legacy-activity"]
    );
}

#[test]
fn archive_parse_youtube_separates_watch_and_decodes_search_query() {
    let rows = crate::archive::youtube::parse(br#"[{"titleUrl":"https://youtube.com/watch?v=x"},{"titleUrl":"https://youtube.com/results?search_query=%E4%BA%AC%E9%83%BD"}]"#).unwrap();
    assert_eq!(rows[0].logical_source, "c03-youtube-watch");
    assert_eq!(rows[1].logical_source, "c03-youtube-search");
    assert_eq!(rows[1].search_query.as_deref(), Some("京都"));
}

/// Scenario: 記録から運んだ書庫が分かる
#[test]
fn archive_requests_keep_archive_hash_and_inner_path_in_payload() {
    let requests = crate::archive::worker::requests_for_file(
        crate::archive::classify::KnownKind::YouTubeWatch,
        "Takeout/YouTube/watch-history.json",
        br#"[{"time":"2026-09-12T03:00:00Z","titleUrl":"https://youtube.com/watch?v=x"}]"#,
        uuid::Uuid::nil(),
        "a".repeat(64),
    )
    .unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].logical_source, "c03-youtube-watch");
    assert_eq!(requests[0].device_id.as_deref(), Some("s01-c03"));
    assert_eq!(requests[0].payload["archive_sha256"], "a".repeat(64));
    assert_eq!(
        requests[0].payload["inner_path"],
        "Takeout/YouTube/watch-history.json"
    );
}

/// Scenario: 端末から書き出したタイムラインを専用のフォルダに置くと読まれる
#[test]
fn archive_requests_accept_timeline_segments() {
    let requests = crate::archive::worker::requests_for_file(
        crate::archive::classify::KnownKind::Timeline,
        "Timeline.json",
        br#"{"semanticSegments":[{"visit":{"startTime":"2026-09-12T03:00:00Z"},"activity":{"startTime":"2026-09-12T04:00:00Z"},"timelinePath":[{"time":"2026-09-12T05:00:00Z"}]}],"rawSignals":[{"time":"2026-09-12T06:00:00Z"}]}"#,
        uuid::Uuid::nil(),
        "b".repeat(64),
    )
    .unwrap();
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[0].logical_source, "c03-timeline-visit");
    assert_eq!(requests[3].logical_source, "c03-timeline-signal");
}

/// Scenario: 移行前のロケーション履歴は読み終えると退役する
#[test]
fn archive_requests_accept_legacy_records_and_semantic_history() {
    let records = crate::archive::worker::requests_for_file(
        crate::archive::classify::KnownKind::Records,
        "Records.json",
        br#"{"locations":[{"timestamp":"2024-08-31T23:00:00Z"}]}"#,
        uuid::Uuid::nil(),
        "c".repeat(64),
    )
    .unwrap();
    assert_eq!(records[0].logical_source, "c03-legacy-location");
    let semantic = crate::archive::worker::requests_for_file(
        crate::archive::classify::KnownKind::SemanticHistory,
        "Semantic Location History.json",
        br#"{"timelineObjects":[{"placeVisit":{"duration":{"startTimestamp":"2024-08-31T23:00:00Z"}}},{"activitySegment":{"duration":{"startTimestampMs":"1725148800000"}}}]}"#,
        uuid::Uuid::nil(), "d".repeat(64),
    ).unwrap();
    assert_eq!(
        semantic
            .iter()
            .map(|item| item.logical_source.as_str())
            .collect::<Vec<_>>(),
        ["c03-legacy-visit", "c03-legacy-activity"]
    );
}

/// Scenario: 移行前のロケーション履歴は読み終えると退役する
#[tokio::test]
async fn archive_parse_legacy_extends_source_retirement_only_forward() {
    let pool = testdb::pool().await;
    sqlx::query(
        "UPDATE core.source SET retired_on = NULL WHERE logical_source LIKE 'c03-legacy-%'",
    )
    .execute(&pool)
    .await
    .unwrap();
    crate::archive::worker::retire_legacy_sources(
        &pool,
        chrono::DateTime::parse_from_rfc3339("2024-08-31T03:00:00Z")
            .unwrap()
            .to_utc(),
    )
    .await
    .unwrap();
    let first: chrono::NaiveDate = sqlx::query_scalar(
        "SELECT retired_on FROM core.source WHERE logical_source = 'c03-legacy-location'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(first.to_string(), "2024-09-01");
    crate::archive::worker::retire_legacy_sources(
        &pool,
        chrono::DateTime::parse_from_rfc3339("2024-08-01T23:00:00Z")
            .unwrap()
            .to_utc(),
    )
    .await
    .unwrap();
    let kept: chrono::NaiveDate = sqlx::query_scalar(
        "SELECT retired_on FROM core.source WHERE logical_source = 'c03-legacy-location'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(kept, first);
}

#[tokio::test]
async fn archive_parse_myactivity_registers_a_source_once() {
    let pool = testdb::pool().await;
    let source = crate::archive::myactivity::source_name("マップ");
    crate::archive::worker::ensure_myactivity_source(&pool, &source, "マップ")
        .await
        .unwrap();
    crate::archive::worker::ensure_myactivity_source(&pool, &source, "マップ")
        .await
        .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.source WHERE logical_source = $1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

/// Scenario: 専用のフォルダの書庫は取り込み済みへ移る
/// Scenario: 取り込み済みに同じ名前があっても上書きしない
#[test]
fn archive_move_keeps_existing_file_and_uses_a_numbered_name() {
    let root = std::env::temp_dir().join(format!("ashiato-archive-move-{}", uuid::Uuid::new_v4()));
    let inbox = root.join("inbox");
    let processed = inbox.join("取り込み済み");
    std::fs::create_dir_all(&processed).unwrap();
    std::fs::write(processed.join("one.zip"), b"old").unwrap();
    let source = inbox.join("one.zip");
    std::fs::write(&source, b"new").unwrap();
    let moved = crate::archive::worker::move_to_processed(&source).unwrap();
    assert_eq!(moved.file_name().unwrap(), "one (2).zip");
    assert_eq!(std::fs::read(processed.join("one.zip")).unwrap(), b"old");
    assert_eq!(std::fs::read(moved).unwrap(), b"new");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn archive_parse_myactivity_source_name_uses_ascii_or_a_stable_hash() {
    assert_eq!(
        crate::archive::myactivity::source_name("Google Search"),
        "c03-myactivity-google-search"
    );
    let japanese = crate::archive::myactivity::source_name("マップ");
    assert!(japanese.starts_with("c03-myactivity-u"));
    assert_eq!(japanese.len(), "c03-myactivity-u".len() + 12);
}

#[test]
fn archive_parse_chrome_time_usec_is_converted_from_windows_epoch_to_utc() {
    assert_eq!(
        crate::archive::chrome::time_usec_to_utc(11644473600000000)
            .unwrap()
            .to_rfc3339(),
        "1970-01-01T00:00:00+00:00"
    );
}

/// Scenario: ダウンロードのフォルダの他のファイルは読まれない
/// Scenario: 書き込み途中のファイルは読まれない
/// Scenario: 名前が書き込み途中でなくなったファイルは読まれる
#[tokio::test]
async fn archive_scan_only_queues_stable_supported_inbox_files() {
    let pool = testdb::pool().await;
    let root = std::env::temp_dir().join(format!("ashiato-archive-scan-{}", uuid::Uuid::new_v4()));
    let inbox = root.join("inbox");
    let downloads = root.join("downloads");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::create_dir_all(&downloads).unwrap();
    write_file(&inbox, "timeline.json", b"[]");
    write_file(&inbox, "writing.zip.part", b"unfinished");
    write_file(&downloads, "takeout-20260912.zip", b"zip");
    write_file(&downloads, "photo.zip", b"not an archive inbox file");

    let config = crate::archive::config::ArchiveConfig {
        inbox_dir: inbox.clone(),
        downloads_dir: downloads.clone(),
        copy_dir: root.join("copies"),
        keep_copies: true,
        user_id: Some(testdb::user()),
        scan_sec: 120,
    };
    let user = config.user_id.unwrap();

    assert!(crate::archive::scan::scan_once(&pool, &config, user)
        .await
        .unwrap()
        .is_empty());
    let queued = crate::archive::scan::scan_once(&pool, &config, user)
        .await
        .unwrap();
    let paths: Vec<_> = queued
        .iter()
        .map(|candidate| candidate.path.file_name().unwrap().to_owned())
        .collect();
    assert_eq!(paths, ["timeline.json", "takeout-20260912.zip"]);

    std::fs::rename(inbox.join("writing.zip.part"), inbox.join("writing.zip")).unwrap();
    let queued = crate::archive::scan::scan_once(&pool, &config, user)
        .await
        .unwrap();
    assert_eq!(queued.len(), 2, "既に安定したファイルだけが残る");
    let queued = crate::archive::scan::scan_once(&pool, &config, user)
        .await
        .unwrap();
    assert_eq!(queued.len(), 3);
    assert!(queued
        .iter()
        .any(|candidate| candidate.path.file_name().unwrap() == "writing.zip"));

    std::fs::remove_dir_all(root).unwrap();
}

/// Scenario: 読んだ書庫は次の走査で読み直されない
/// Scenario: 同じ書庫を置き直すと台帳に 1 行残る
/// Scenario: ダウンロードのフォルダに残り続ける書庫は台帳を増やさない
#[tokio::test]
async fn archive_scan_marks_a_previously_read_archive_without_requeueing_it() {
    let pool = testdb::pool().await;
    let root = std::env::temp_dir().join(format!("ashiato-archive-known-{}", uuid::Uuid::new_v4()));
    let inbox = root.join("inbox");
    let downloads = root.join("downloads");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::create_dir_all(&downloads).unwrap();
    write_file(&downloads, "takeout-known.zip", b"already read");
    let user = testdb::user();
    let config = crate::archive::config::ArchiveConfig {
        inbox_dir: inbox.clone(),
        downloads_dir: downloads.clone(),
        copy_dir: root.join("copies"),
        keep_copies: true,
        user_id: Some(user),
        scan_sec: 120,
    };

    assert!(crate::archive::scan::scan_once(&pool, &config, user)
        .await
        .unwrap()
        .is_empty());
    let first = crate::archive::scan::scan_once(&pool, &config, user)
        .await
        .unwrap();
    let sha256 = first[0].sha256.clone();
    sqlx::query("INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome) VALUES ($1, $2, $3, 'read')")
        .bind(user).bind(&sha256).bind(crate::archive::PARSER_VERSION).execute(&pool).await.unwrap();

    let later = crate::archive::scan::scan_once(&pool, &config, user)
        .await
        .unwrap();
    assert!(matches!(
        later[0].disposition,
        crate::archive::scan::ScanDisposition::AlreadyRead
    ));
    std::fs::remove_dir_all(root).unwrap();
}

/// Scenario: 大きさが変わり続けているファイルは読まれない
#[tokio::test]
async fn archive_scan_waits_for_size_to_stop_and_reuses_its_hash() {
    let pool = testdb::pool().await;
    let root =
        std::env::temp_dir().join(format!("ashiato-archive-changing-{}", uuid::Uuid::new_v4()));
    let inbox = root.join("inbox");
    let downloads = root.join("downloads");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::create_dir_all(&downloads).unwrap();
    write_file(&inbox, "growing.json", b"one");
    let user = testdb::user();
    let config = crate::archive::config::ArchiveConfig {
        inbox_dir: inbox.clone(),
        downloads_dir: downloads,
        copy_dir: root.join("copies"),
        keep_copies: true,
        user_id: Some(user),
        scan_sec: 120,
    };
    let hashes = std::sync::atomic::AtomicUsize::new(0);
    let hash = |path: &Path| {
        hashes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(format!("test-{}", std::fs::metadata(path).unwrap().len()))
    };

    assert!(
        crate::archive::scan::scan_once_with_hasher(&pool, &config, user, &hash)
            .await
            .unwrap()
            .is_empty()
    );
    write_file(&inbox, "growing.json", b"two-more");
    assert!(
        crate::archive::scan::scan_once_with_hasher(&pool, &config, user, &hash)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(hashes.load(std::sync::atomic::Ordering::SeqCst), 2);
    let ready = crate::archive::scan::scan_once_with_hasher(&pool, &config, user, &hash)
        .await
        .unwrap();
    assert_eq!(ready.len(), 1);
    assert_eq!(
        hashes.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "安定した2回目でハッシュを取り直さない"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Scenario: 読んでいる間に置いた書庫は読み終えた後に読まれる
#[tokio::test]
async fn archive_reader_processes_candidates_one_at_a_time_in_discovery_order() {
    let (sender, receiver) = tokio::sync::mpsc::channel(2);
    let first_started = std::sync::Arc::new(tokio::sync::Notify::new());
    let release_first = std::sync::Arc::new(tokio::sync::Notify::new());
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let started = first_started.clone();
    let release = release_first.clone();
    let recorded = seen.clone();
    let reader = tokio::spawn(crate::archive::worker::read_in_order(
        receiver,
        move |candidate| {
            let started = started.clone();
            let release = release.clone();
            let recorded = recorded.clone();
            async move {
                if candidate.path == std::path::Path::new("first.zip") {
                    started.notify_one();
                    release.notified().await;
                }
                recorded.lock().unwrap().push(candidate.path);
            }
        },
    ));
    sender
        .send(crate::archive::scan::ScanCandidate {
            path: "first.zip".into(),
            from_downloads: false,
            sha256: "first".into(),
            disposition: crate::archive::scan::ScanDisposition::Read,
        })
        .await
        .unwrap();
    first_started.notified().await;
    sender
        .send(crate::archive::scan::ScanCandidate {
            path: "second.zip".into(),
            from_downloads: false,
            sha256: "second".into(),
            disposition: crate::archive::scan::ScanDisposition::Read,
        })
        .await
        .unwrap();
    assert!(seen.lock().unwrap().is_empty());
    release_first.notify_one();
    drop(sender);
    reader.await.unwrap();
    assert_eq!(
        *seen.lock().unwrap(),
        ["first.zip", "second.zip"].map(std::path::PathBuf::from)
    );
}

/// Scenario: 読んでいる間に置いた書庫は読み終えた後に読まれる
#[tokio::test]
async fn archive_worker_starts_and_records_a_stable_archive() {
    let pool = testdb::pool().await;
    let root =
        std::env::temp_dir().join(format!("ashiato-archive-worker-{}", uuid::Uuid::new_v4()));
    let inbox = root.join("inbox");
    let downloads = root.join("downloads");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::create_dir_all(&downloads).unwrap();
    write_zip(
        &inbox.join("takeout-20260912.zip"),
        &[(
            "Takeout/YouTube/watch-history.json",
            br#"[{"titleUrl":"https://youtube.com/watch?v=x"}]"#,
        )],
    );
    let user = testdb::user();
    crate::archive::worker::spawn_inspecting(
        pool.clone(),
        crate::archive::config::ArchiveConfig {
            inbox_dir: inbox.clone(),
            downloads_dir: downloads,
            copy_dir: root.join("copies"),
            keep_copies: true,
            user_id: Some(user),
            scan_sec: 1,
        },
        user,
    );

    let recorded = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM core.archive_ledger WHERE user_id = $1")
                    .bind(user)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            if count == 1 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await;
    std::fs::remove_dir_all(root).unwrap();
    assert!(recorded.is_ok(), "取り込み器が5秒以内に台帳へ記録しない");
}

/// Scenario: 書庫のソースは 60 日で登録されている
/// Scenario: 取り込み器のソースは 1 日で登録されている
/// Scenario: 本人が変えた想定間隔は移行を当て直しても戻らない
#[tokio::test]
async fn archive_migration_registers_sources_and_preserves_interval() {
    let pool = testdb::pool().await;
    let (archive_sources,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM core.source WHERE logical_source LIKE 'c03-%'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(archive_sources, 10, "書庫の固定ソースは 10 本");

    let (gap, kind): (i32, String) = sqlx::query_as(
        "SELECT expected_gap_sec, external_id_kind
           FROM core.source WHERE logical_source = 's01-archive-inbox'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(gap, 86_400, "取り込み器は 1 日ごとの生存信号");
    assert_eq!(kind, "none");

    sqlx::query("UPDATE core.source SET expected_gap_sec = 2_592_000 WHERE logical_source = 'c03-youtube-watch'")
        .execute(&pool)
        .await
        .unwrap();
    crate::migrate(&pool).await.unwrap();
    let (kept,): (i32,) = sqlx::query_as(
        "SELECT expected_gap_sec FROM core.source WHERE logical_source = 'c03-youtube-watch'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(kept, 2_592_000, "本人が変えた想定間隔を戻さない");
}

/// Scenario: 台帳の行は書き換えられない
#[tokio::test]
async fn archive_ledgers_are_append_only() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome)
         VALUES ($1, repeat('a', 64), 'test', 'read') RETURNING id",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    for sql in [
        "UPDATE core.archive_ledger SET outcome = 'unreadable' WHERE id = $1",
        "DELETE FROM core.archive_ledger WHERE id = $1",
    ] {
        assert!(
            sqlx::query(sql).bind(id).execute(&pool).await.is_err(),
            "{sql} が通っている"
        );
    }
}
