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
fn archive_classifier_prefers_json_shapes_and_accounts_for_html() {
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
