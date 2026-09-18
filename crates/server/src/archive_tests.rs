// SPDX-License-Identifier: AGPL-3.0-only
//! ST12 の書庫台帳移行を本物の PostgreSQL で確かめる。
#![allow(clippy::unwrap_used)]

use crate::testdb;

/// Scenario: 書庫のソースは 60 日で登録されている
/// Scenario: 取り込み器のソースは 1 日で登録されている
/// Scenario: 本人が変えた想定間隔は移行を当て直しても戻らない
#[tokio::test]
async fn archive_migration_registers_sources_and_preserves_interval() {
    let pool = testdb::pool().await;
    let (archive_sources,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM core.source WHERE logical_source LIKE 'c03-%'",
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
        assert!(sqlx::query(sql).bind(id).execute(&pool).await.is_err(), "{sql} が通っている");
    }
}
