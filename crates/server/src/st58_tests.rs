// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! ST58 の更新履歴（Task 1）。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use crate::update_history::{record_rollback, record_startup, replay_pending, RollbackKind};
use tempfile::TempDir;

async fn rows(pool: &sqlx::PgPool) -> Vec<(String, Option<String>, String, String, Vec<String>)> {
    sqlx::query_as("SELECT outcome, from_version, to_version, via, applied_versions FROM core.update_history ORDER BY seq")
        .fetch_all(pool).await.unwrap()
}

async fn set_outcome_constraint(pool: &sqlx::PgPool, rolled_back: bool) {
    sqlx::query("ALTER TABLE core.update_history DROP CONSTRAINT update_history_outcome_check")
        .execute(pool)
        .await
        .unwrap();
    let values = if rolled_back {
        "'attempted', 'applied', 'nothing_to_apply', 'failed', 'backup_failed', 'rolled_back'"
    } else {
        "'attempted', 'applied', 'nothing_to_apply', 'failed', 'backup_failed'"
    };
    sqlx::query(&format!("ALTER TABLE core.update_history ADD CONSTRAINT update_history_outcome_check CHECK (outcome IN ({values}))")).execute(pool).await.unwrap();
}

// Scenario: 戻したの値の行は入る
#[tokio::test]
async fn st58_history_accepts_rolled_back() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    sqlx::query("INSERT INTO core.update_history (component, via, to_version, outcome) VALUES ('S-01', 'migrate', 'v', 'rolled_back')").execute(&pool).await.unwrap();
    assert_eq!(rows(&pool).await[0].0, "rolled_back");
}

// Scenario: 戻すと、更新の履歴に落ちた試みの結果と戻したの 2 行が足される
#[tokio::test]
async fn st58_history_failed_pair_is_ordered() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    sqlx::query("INSERT INTO core.update_history (component, via, from_version, to_version, outcome) VALUES ('S-01', 'migrate', 'old', 'new', 'attempted')").execute(&pool).await.unwrap();
    let mut conn = pool.acquire().await.unwrap();
    record_rollback(
        &mut conn,
        "new",
        "wrong",
        RollbackKind::MigrationFailed {
            failed_version: "bad",
            sqlstate: "23505",
        },
        &[],
        None,
    )
    .await
    .unwrap();
    let got = rows(&pool).await;
    assert_eq!(got.len(), 3);
    assert_eq!(
        (&got[1].0, &got[1].1, &got[1].2),
        (&"failed".into(), &Some("old".into()), &"new".into())
    );
    assert_eq!(
        (&got[2].0, &got[2].1, &got[2].2),
        (&"rolled_back".into(), &Some("new".into()), &"wrong".into())
    );
}

// Scenario: rolled_back を受け付けない DB に戻すと控えに書き、次に通った移行が表へ写す
#[tokio::test]
async fn st58_history_pending_replays_once() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    set_outcome_constraint(&pool, false).await;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("history-pending.jsonl");
    let mut conn = pool.acquire().await.unwrap();
    let result = record_rollback(
        &mut conn,
        "new",
        "old",
        RollbackKind::StartupFailed,
        &[("new-migration", "sha")],
        Some(&path),
    )
    .await;
    assert!(matches!(
        result,
        Err(crate::update_history::RollbackError::PendingWritten)
    ));
    assert_eq!(rows(&pool).await.len(), 0);
    set_outcome_constraint(&pool, true).await;
    replay_pending(&mut conn, &path).await.unwrap();
    assert_eq!(rows(&pool).await.len(), 1);
    replay_pending(&mut conn, &path).await.unwrap();
    assert_eq!(rows(&pool).await.len(), 1);
    assert_eq!(rows(&pool).await[0].4, vec!["new-migration"]);
}

// Scenario: 戻しても、試みの行は書き直さない
#[tokio::test]
async fn st58_history_rollback_preserves_attempt() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    sqlx::query("INSERT INTO core.update_history (component, via, from_version, to_version, outcome) VALUES ('S-01', 'migrate', 'old', 'new', 'attempted')").execute(&pool).await.unwrap();
    let mut conn = pool.acquire().await.unwrap();
    record_rollback(
        &mut conn,
        "new",
        "old",
        RollbackKind::MigrationFailed {
            failed_version: "bad",
            sqlstate: "23505",
        },
        &[],
        None,
    )
    .await
    .unwrap();
    assert_eq!(rows(&pool).await[0].0, "attempted");
}

// Scenario: 待ち受けを開かずに戻すと、更新の履歴に当てた版の名前を持つ戻したの 1 行が足される
#[tokio::test]
async fn st58_history_startup_rollback_lists_applied_versions() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let mut conn = pool.acquire().await.unwrap();
    record_rollback(
        &mut conn,
        "new",
        "old",
        RollbackKind::StartupFailed,
        &[("new-migration", "sha")],
        None,
    )
    .await
    .unwrap();
    let got = rows(&pool).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, "rolled_back");
    assert_eq!(got[0].4, vec!["new-migration"]);
}

// Scenario: 当てる版の無い更新で待ち受けを開かずに戻すと、戻したの行の当てた版は空
#[tokio::test]
async fn st58_history_no_migration_has_empty_applied_versions() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let mut conn = pool.acquire().await.unwrap();
    record_rollback(
        &mut conn,
        "new",
        "old",
        RollbackKind::StartupFailed,
        &[],
        None,
    )
    .await
    .unwrap();
    assert!(rows(&pool).await[0].4.is_empty());
}

// Scenario: 待ち受けを開かずに戻した後も、前の版は戻した先の版のまま
#[tokio::test]
async fn st58_history_startup_rollback_keeps_previous_version() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let mut conn = pool.acquire().await.unwrap();
    record_rollback(
        &mut conn,
        "new",
        "old",
        RollbackKind::StartupFailed,
        &[],
        None,
    )
    .await
    .unwrap();
    assert!(record_startup(&mut conn, "old", crate::MIGRATIONS)
        .await
        .is_none());
    assert_eq!(
        crate::update_history::record_attempt(&mut conn, "next").await,
        crate::update_history::Attempt::Written
    );
    assert_eq!(rows(&pool).await.last().unwrap().1, Some("old".into()));
}

// Scenario: 待ち受けを開かずに rolled_back を受け付けない DB に戻すと、控えに戻したの 1 行を書く
#[tokio::test]
async fn st58_history_pending_contains_only_rollback_for_startup() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    set_outcome_constraint(&pool, false).await;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("history-pending.jsonl");
    let mut conn = pool.acquire().await.unwrap();
    let result = record_rollback(
        &mut conn,
        "new",
        "old",
        RollbackKind::StartupFailed,
        &[("new-migration", "sha")],
        Some(&path),
    )
    .await;
    assert!(matches!(
        result,
        Err(crate::update_history::RollbackError::PendingWritten)
    ));
    let text = std::fs::read_to_string(&path).unwrap();
    let lines = text.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("rolled_back"));
    let pending: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(pending["outcome"], "rolled_back");
}
