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
