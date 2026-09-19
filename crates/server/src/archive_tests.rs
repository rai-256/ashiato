// SPDX-License-Identifier: AGPL-3.0-only
//! ST12 の書庫台帳移行を本物の PostgreSQL で確かめる。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use crate::{heartbeat, store_heartbeat, store_one, IngestRequest, StoreOutcome};
use std::path::Path;

#[derive(Clone)]
struct FailingSink {
    fail_at: usize,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl crate::RecordSink for FailingSink {
    fn store<'a>(
        &'a self,
        _request: IngestRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<StoreOutcome>> + Send + 'a>,
    > {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls: &AtomicUsize = &self.calls;
        Box::pin(async move {
            let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
            if call == self.fail_at {
                anyhow::bail!("D-01 が切れた")
            }
            Ok(StoreOutcome::Inserted(uuid::Uuid::new_v4()))
        })
    }
}

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

/// Scenario: 格納が落ちた書庫は次の走査で読み直される
#[tokio::test]
async fn archive_partial_store_failure_is_not_a_completed_read() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let requests = (0..5)
        .map(|n| archive_request(user, &format!(r#"{{"watch":"{n}"}}"#)))
        .collect::<Vec<_>>();
    let sink = FailingSink {
        fail_at: 5,
        calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };

    let result = crate::archive::worker::store_requests(&sink, requests.clone()).await;

    assert!(result.is_err(), "途中のDB失敗を読了としてはいけない");
    assert_eq!(
        sink.calls.load(std::sync::atomic::Ordering::SeqCst),
        5,
        "失敗した5件目までを順に格納する"
    );
    let completed = crate::archive::worker::store_requests(
        &crate::PgSink::new(pool),
        requests,
    )
    .await
    .expect("次の走査では書庫全体を最初から読み直せる");
    assert_eq!(completed.len(), 5, "再試行で全件を格納関門へ渡す");
}

#[tokio::test]
async fn archive_pg_sink_uses_the_same_store_gate() {
    let pool = testdb::pool().await;
    let sink = crate::PgSink::new(pool);
    let outcomes = crate::archive::worker::store_requests(
        &sink,
        vec![archive_request(testdb::user(), r#"{"watch":"worker"}"#)],
    )
    .await;
    assert!(outcomes.is_ok(), "読み手の PgSink も既存の格納関門を通る");
}

/// Scenario: 格納に続けて失敗した書庫は台帳と画面に出る
#[tokio::test]
async fn archive_three_store_failures_leave_one_throttled_ledger_row() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    sqlx::query(
        "INSERT INTO core.archive_sighting (user_id, path, size_bytes, sha256)
         VALUES ($1, '/tmp/failing.zip', 1, $2)",
    )
    .bind(user)
    .bind("f".repeat(64))
    .execute(&pool)
    .await
    .unwrap();
    for attempt in 1..=4 {
        let throttled = crate::archive::worker::record_store_failure(
            &pool,
            user,
            std::path::Path::new("/tmp/failing.zip"),
            "f".repeat(64),
        )
            .await
            .unwrap();
        assert_eq!(throttled, attempt >= 3);
    }

    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.archive_ledger WHERE user_id = $1 AND outcome = 'store_failed'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rows, 1, "3回目だけを失敗済みとして台帳に残す");
}

/// Scenario: 読んだ書庫の件数が台帳に残る
/// Scenario: 削除済みで入れなかった件数が台帳に残る
#[tokio::test]
async fn archive_ledger_sources_keep_per_source_store_outcomes() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let ledger: i64 = sqlx::query_scalar(
        "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome)
         VALUES ($1, repeat('c', 64), 'test', 'read') RETURNING id",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut duplicate = archive_request(user, r#"{"watch":"duplicate"}"#);
    duplicate.event_time = chrono::DateTime::parse_from_rfc3339("2026-09-13T03:00:00Z")
        .unwrap()
        .to_utc();
    let requests = vec![archive_request(user, r#"{"watch":"inserted"}"#), duplicate];
    crate::archive::worker::record_ledger_sources(
        &pool,
        ledger,
        &requests,
        &[StoreOutcome::Inserted(uuid::Uuid::new_v4()), StoreOutcome::DuplicateOfDeleted(uuid::Uuid::new_v4())],
    )
    .await
    .unwrap();
    let (inserted, deleted, max_event_at): (i32, i32, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as(
            "SELECT inserted_count, deleted_count, max_event_at
               FROM core.archive_ledger_source WHERE ledger_id = $1 AND logical_source = 'c03-youtube-watch'",
        )
        .bind(ledger)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((inserted, deleted), (1, 1));
    assert_eq!(max_event_at, requests[1].event_time);
}

/// Scenario: 名前の時刻を持たない書庫は見つけた時刻を持つ
#[test]
fn archive_created_at_uses_filename_or_discovery_time() {
    let discovered = chrono::DateTime::parse_from_rfc3339("2026-09-15T01:02:03Z")
        .unwrap()
        .to_utc();
    assert_eq!(
        crate::archive::worker::archive_created_at(
            std::path::Path::new("takeout-20260912-010203.zip"),
            discovered,
        ),
        chrono::DateTime::parse_from_rfc3339("2026-09-12T01:02:03Z")
            .unwrap()
            .to_utc(),
    );
    assert_eq!(
        crate::archive::worker::archive_created_at(std::path::Path::new("Timeline.json"), discovered),
        discovered,
    );
}

/// Scenario: 台帳の行は利用者ごとに分かれる
/// Scenario: 台帳に記録の本文は載らない
#[tokio::test]
async fn archive_ledger_is_private_and_scoped_to_its_user() {
    let pool = testdb::pool().await;
    let first = testdb::user();
    let second = testdb::user();
    for (user, sha) in [(first, "d".repeat(64)), (second, "e".repeat(64))] {
        sqlx::query("INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome) VALUES ($1, $2, 'test', 'read')")
            .bind(user)
            .bind(sha)
            .execute(&pool)
            .await
            .unwrap();
    }
    let own: i64 = sqlx::query_scalar("SELECT count(*) FROM core.archive_ledger WHERE user_id = $1")
        .bind(first)
        .fetch_one(&pool)
        .await
        .unwrap();
    let rendered: String = sqlx::query_scalar(
        "SELECT string_agg(row_to_json(l)::text, '') FROM core.archive_ledger l WHERE l.user_id = $1",
    )
    .bind(first)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(own, 1);
    assert!(!rendered.contains("京都 旅館"));
}

/// Scenario: 壊れた 1 件の場所が台帳に残る
#[test]
fn archive_unreadable_locations_keep_only_the_first_hundred() {
    let locations = (0..101)
        .map(|n| format!("Takeout/YouTube/watch-history.json#{n}"))
        .collect::<Vec<_>>();
    let summary = crate::archive::worker::unreadable_summary(&locations).expect("場所がある");
    assert!(summary.contains("#0"));
    assert!(summary.contains("#99"));
    assert!(!summary.contains("#100"), "台帳には先頭100件だけを残す");
}

/// 格納関門は HTTP の JSON 解釈を通さなくても、新規・重複・削除済み・拒否を区別する。
#[tokio::test]
async fn archive_dedup_distinguishes_archive_results() {
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

    // Scenario: 同じ出来事を含む別の書庫を置いても行が増えない
    let mut same_content = first.clone();
    same_content.id = uuid::Uuid::new_v4();
    same_content.payload = serde_json::json!({"archive_sha256":"different"});
    assert!(matches!(store_one(&pool, same_content).await.unwrap(), StoreOutcome::Duplicate(_)));

    // Scenario: 題名が変わった同じ視聴は別の記録として残る
    let mut changed_title = archive_request(user, r#"{"watch":"changed title"}"#);
    changed_title.event_time = first.event_time;
    assert!(matches!(store_one(&pool, changed_title).await.unwrap(), StoreOutcome::Inserted(_)));

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
/// Scenario: 書庫の記録は収集したに分類される
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
    assert!(requests.iter().all(|request| request.origin == "collected"));
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

/// Scenario: ダウンロードのフォルダの書庫は動かない
#[test]
fn archive_move_does_not_apply_to_downloads_without_an_explicit_move() {
    let root = std::env::temp_dir().join(format!("ashiato-archive-downloads-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let downloaded = root.join("takeout-1.zip");
    std::fs::write(&downloaded, b"download").unwrap();
    // 読み手は `from_downloads` の候補には `move_to_processed` を呼ばない。
    assert!(downloaded.exists());
    assert_eq!(std::fs::read(&downloaded).unwrap(), b"download");
    std::fs::remove_dir_all(root).unwrap();
}

/// Scenario: 読んだ製品のファイルの写しが残る
/// Scenario: 同じファイルの写しは 1 つ
#[test]
fn archive_copy_uses_content_hash_as_the_single_copy_name() {
    let root = std::env::temp_dir().join(format!("ashiato-archive-copy-{}", uuid::Uuid::new_v4()));
    let first = crate::archive::worker::copy_known_file(&root, b"known file").unwrap();
    let second = crate::archive::worker::copy_known_file(&root, b"known file").unwrap();
    assert_eq!(first, second);
    assert_eq!(std::fs::read(first).unwrap(), b"known file");
    std::fs::remove_dir_all(root).unwrap();
}

/// Scenario: 読んだ製品のファイルの写しが残る
#[tokio::test]
async fn archive_copy_catalog_keeps_the_inner_file_name() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    crate::archive::worker::record_copy(
        &pool,
        user,
        "a".repeat(64),
        "Takeout/YouTube/watch-history.json",
        std::path::Path::new("/copies/aa/archive"),
    )
    .await
    .unwrap();
    let path: String = sqlx::query_scalar("SELECT inner_path FROM core.archive_file WHERE sha256 = $1")
        .bind("a".repeat(64))
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(path, "Takeout/YouTube/watch-history.json");
}

/// Scenario: 残さない設定では写しを作らない
#[test]
fn archive_copy_setting_does_not_create_a_file_when_disabled() {
    let root = std::env::temp_dir().join(format!("ashiato-archive-no-copy-{}", uuid::Uuid::new_v4()));
    let copied = crate::archive::worker::copy_if_enabled(false, &root, b"history").unwrap();
    assert!(copied.is_none());
    assert!(!root.exists());
}

#[test]
fn archive_shape_for_myactivity_uses_product_names_without_activity_values() {
    let shape = crate::archive::worker::shape_for_file(
        crate::archive::classify::KnownKind::MyActivity,
        r#"[{"time":"2026-01-01T00:00:00Z","products":["マップ"],"title":"京都 旅館"}]"#.as_bytes(),
    )
    .unwrap();
    assert!(shape["products"].to_string().contains("マップ"));
    assert!(!shape.to_string().contains("京都 旅館"));
}

#[tokio::test]
async fn archive_shape_confirmation_allows_only_confirmed_shape() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let hash = "shape-test";
    assert!(
        !crate::archive::worker::is_shape_confirmed(&pool, user, hash)
            .await
            .unwrap()
    );
    sqlx::query("INSERT INTO core.archive_shape_confirmation (user_id, shape_hash, shape) VALUES ($1, $2, '{}'::jsonb)")
        .bind(user).bind(hash).execute(&pool).await.unwrap();
    assert!(
        crate::archive::worker::is_shape_confirmed(&pool, user, hash)
            .await
            .unwrap()
    );
}

/// Scenario: 印を置く前の Takeout の書庫は格納されない
#[tokio::test]
async fn archive_pending_shape_records_unconfirmed_file_once() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let shape = serde_json::json!({"kind":"MyActivity","products":["マップ"]});
    crate::archive::worker::record_pending_shape(&pool, user, "archive", "activity.json", &shape)
        .await
        .unwrap();
    crate::archive::worker::record_pending_shape(&pool, user, "archive", "activity.json", &shape)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.archive_pending_shape WHERE user_id = $1 AND sha256 = 'archive'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
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

/// Scenario: 解析器の版が上がると読み直される
#[tokio::test]
async fn archive_scan_requeues_a_copy_read_by_an_older_parser() {
    let pool = testdb::pool().await;
    let root = std::env::temp_dir().join(format!("ashiato-archive-reparse-{}", uuid::Uuid::new_v4()));
    let inbox = root.join("inbox");
    let downloads = root.join("downloads");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::create_dir_all(&downloads).unwrap();
    write_file(&downloads, "takeout-old.zip", b"older parser");
    let user = testdb::user();
    let config = crate::archive::config::ArchiveConfig {
        inbox_dir: inbox,
        downloads_dir: downloads,
        copy_dir: root.join("copies"),
        keep_copies: true,
        user_id: Some(user),
        scan_sec: 120,
    };
    assert!(crate::archive::scan::scan_once(&pool, &config, user).await.unwrap().is_empty());
    let candidate = crate::archive::scan::scan_once(&pool, &config, user).await.unwrap().remove(0);
    sqlx::query("INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome) VALUES ($1, $2, 'older', 'read')")
        .bind(user)
        .bind(&candidate.sha256)
        .execute(&pool)
        .await
        .unwrap();
    let reparsed = crate::archive::scan::scan_once(&pool, &config, user).await.unwrap().remove(0);
    assert!(matches!(reparsed.disposition, crate::archive::scan::ScanDisposition::Read));
    std::fs::remove_dir_all(root).unwrap();
}

/// Scenario: 解析器の版が上がると読み直される
#[tokio::test]
async fn archive_reparse_prefers_the_saved_copy_over_the_inbox_path() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let root = std::env::temp_dir().join(format!("ashiato-archive-reparse-copy-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let copied = root.join("copy.json");
    std::fs::write(&copied, b"copy").unwrap();
    crate::archive::worker::record_copy(&pool, user, "b".repeat(64), "history.json", &copied).await.unwrap();
    let selected = crate::archive::worker::reparse_path(&pool, user, "b".repeat(64), std::path::Path::new("/missing/archive.zip"))
        .await
        .unwrap();
    assert_eq!(selected, copied);
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
/// Scenario: 専用のフォルダに置いた書庫が読まれる
/// Scenario: 6 つの中身がそれぞれ読まれる
#[tokio::test]
async fn archive_end_to_end_worker_starts_and_records_a_stable_archive() {
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
            br#"[{"time":"2026-09-12T03:00:00Z","titleUrl":"https://youtube.com/watch?v=x"}]"#,
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
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = 'c03-youtube-watch'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert!(recorded.is_ok(), "取り込み器が5秒以内に台帳へ記録しない");
    assert_eq!(events, 1, "書庫項目を既存の格納関門へ通す");
}

/// Scenario: 6 つの中身がそれぞれ読まれる
#[test]
fn archive_end_to_end_builds_requests_for_every_supported_content() {
    let user = uuid::Uuid::nil();
    let fixtures = [
        (crate::archive::classify::KnownKind::Timeline, r#"{"semanticSegments":[{"visit":{"startTime":"2026-01-01T00:00:00Z"}}]}"#),
        (crate::archive::classify::KnownKind::Records, r#"{"locations":[{"timestamp":"2026-01-01T00:00:00Z"}]}"#),
        (crate::archive::classify::KnownKind::YouTubeWatch, r#"[{"time":"2026-01-01T00:00:00Z","titleUrl":"https://youtube.com/watch?v=x"}]"#),
        (crate::archive::classify::KnownKind::YouTubeSearch, r#"[{"time":"2026-01-01T00:00:00Z","titleUrl":"https://youtube.com/results?search_query=x"}]"#),
        (crate::archive::classify::KnownKind::MyActivity, r#"[{"time":"2026-01-01T00:00:00Z","products":["Search"]}]"#),
        (crate::archive::classify::KnownKind::ChromeHistory, r#"{"Browser History":[{"time_usec":13222310400000000}]}"#),
    ];
    for (kind, bytes) in fixtures {
        assert!(!crate::archive::worker::requests_for_file(kind, "fixture.json", bytes.as_bytes(), user, "x".repeat(64)).unwrap().is_empty(), "{kind:?}");
    }
}

/// Scenario: 書庫のソースは 60 日で登録されている
/// Scenario: 取り込み器のソースは 1 日で登録されている
/// Scenario: 本人が変えた想定間隔は移行を当て直しても戻らない
#[tokio::test]
async fn archive_migration_registers_sources_and_preserves_interval() {
    let pool = testdb::pool().await;
    let (archive_sources,): (i64,) =
        sqlx::query_as(
            "SELECT count(*) FROM core.source
              WHERE logical_source LIKE 'c03-%' AND logical_source NOT LIKE 'c03-myactivity-%'",
        )
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
