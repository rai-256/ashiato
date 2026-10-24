// SPDX-License-Identifier: AGPL-3.0-only
//! 試験の足場そのものの繋ぎ先を見る（CH04 / design D1・D3）。
#![allow(clippy::unwrap_used)]

use crate::testdb;

/// 接続先の `current_database()`。
async fn current_database(pool: &sqlx::PgPool) -> String {
    sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// URL の末尾（database 名）。
fn database_of(url: &str) -> &str {
    &url[url.rfind('/').unwrap() + 1..]
}

// Scenario: 試験の足場の繋ぎ先は、その回の database である
#[tokio::test]
async fn ch04_testdb_points_to_the_run_database() {
    let owner = testdb::pool().await;
    let app = testdb::app_pool().await;
    let owner_db = current_database(&owner).await;
    let app_db = current_database(&app).await;
    assert_eq!(
        owner_db, app_db,
        "pool() と app_pool() が別の database を指す"
    );
    assert!(
        owner_db.starts_with("ashiato_run_"),
        "回の database でない: {owner_db}"
    );
    assert_eq!(database_of(&testdb::url()), owner_db);
}

// Scenario: 試験が置いた記録は、ashiato database に入らない
#[tokio::test]
async fn ch04_testdb_records_stay_out_of_ashiato() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "ch04-out", 21_600).await;
    testdb::put_event(&pool, testdb::user(), &source, "2001-02-03T12:00:00+09:00").await;

    let in_run: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.event WHERE logical_source = $1")
            .bind(&source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(in_run, 1, "回の database に記録が無い");

    let run_url = testdb::url();
    let ashiato_url = format!("{}ashiato", &run_url[..run_url.rfind('/').unwrap() + 1]);
    let ashiato = sqlx::PgPool::connect(&ashiato_url).await.unwrap();
    // ashiato に表が無ければ「無い」と読む。
    let has_table: bool = sqlx::query_scalar("SELECT to_regclass('core.event') IS NOT NULL")
        .fetch_one(&ashiato)
        .await
        .unwrap();
    let in_ashiato: i64 = if has_table {
        sqlx::query_scalar("SELECT count(*) FROM core.event WHERE logical_source = $1")
            .bind(&source)
            .fetch_one(&ashiato)
            .await
            .unwrap()
    } else {
        0
    };
    ashiato.close().await;
    assert_eq!(in_ashiato, 0, "試験の記録が ashiato database に入った");
}

// Scenario: 回の database でも、消せない表は行の削除と TRUNCATE を断る
#[tokio::test]
async fn ch04_testdb_run_database_keeps_the_immutable_triggers() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "ch04-trg", 21_600).await;
    testdb::put_event(&pool, testdb::user(), &source, "2001-02-03T12:00:00+09:00").await;

    let delete = sqlx::query("DELETE FROM core.event WHERE logical_source = $1")
        .bind(&source)
        .execute(&pool)
        .await;
    assert!(delete.is_err(), "行の削除が通った");
    let truncate = sqlx::query("TRUNCATE core.event").execute(&pool).await;
    assert!(truncate.is_err(), "TRUNCATE が通った");

    for name in ["event_no_delete", "event_no_truncate"] {
        let enabled: String = sqlx::query_scalar(
            "SELECT tgenabled::text FROM pg_trigger
              WHERE tgname = $1 AND tgrelid = 'core.event'::regclass",
        )
        .bind(name)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(enabled, "O", "{name} が常時有効でない");
    }
}
