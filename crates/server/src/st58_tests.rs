// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! ST58 の更新履歴（Task 1）。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use crate::update_history::{record_rollback, record_startup, replay_pending, RollbackKind};
use crate::update_status::read_status;
use serde_json::json;
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

fn status_dir() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("settings")).unwrap();
    std::fs::write(
        dir.path().join("settings/check.json"),
        json!({"enabled": true, "since": "2026-11-12T00:00:00Z"}).to_string(),
    )
    .unwrap();
    dir
}

fn outcome(result: &str, reason: serde_json::Value) -> serde_json::Value {
    json!({
        "at": "2026-11-12T01:02:03Z",
        "result": result,
        "to_version": "0.5.0",
        "from_version": "0.4.0",
        "reason": reason,
    })
}

// Scenario: 読めない更新の記録は画面に出さない
#[test]
fn st58_status_ignores_invalid_outcome_line() {
    let dir = status_dir();
    std::fs::create_dir_all(dir.path().join("update")).unwrap();
    std::fs::write(
        dir.path().join("update/outcomes.jsonl"),
        format!(
            "{}\n{{\"to_version\":\"<script>\"}}\n",
            outcome("applied", serde_json::Value::Null)
        ),
    )
    .unwrap();
    assert!(
        read_status(dir.path(), "0.4.0", "2026-11-12T02:00:00Z".parse().unwrap())
            .last_update
            .is_none()
    );
}

// Scenario: 1 MB を超える更新の記録は画面に出さない
#[test]
fn st58_status_ignores_outcomes_over_one_megabyte() {
    let dir = status_dir();
    std::fs::create_dir_all(dir.path().join("update")).unwrap();
    std::fs::write(
        dir.path().join("update/outcomes.jsonl"),
        vec![b'x'; 1024 * 1024 + 1],
    )
    .unwrap();
    assert!(
        read_status(dir.path(), "0.4.0", "2026-11-12T02:00:00Z".parse().unwrap())
            .last_update
            .is_none()
    );
}

// Scenario: 戻した後も、新しい版を更新ありに出す
#[test]
fn st58_status_keeps_available_state_after_rollback() {
    let dir = status_dir();
    std::fs::create_dir_all(dir.path().join("update")).unwrap();
    std::fs::write(
        dir.path().join("status.json"),
        json!({"latest":"0.5.0","found_at":"2026-11-12T00:00:00Z","checked_at":"2026-11-12T00:00:00Z","last_ok_at":"2026-11-12T00:00:00Z","notified_version":null,"stale_notified":false}).to_string(),
    ).unwrap();
    std::fs::write(
        dir.path().join("update/outcomes.jsonl"),
        format!(
            "{}\n",
            outcome(
                "rolled_back",
                json!({"kind":"server_not_listening","failed_version":null,"sqlstate":null})
            )
        ),
    )
    .unwrap();
    let result = read_status(dir.path(), "0.4.0", "2026-11-12T02:00:00Z".parse().unwrap());
    assert_eq!(result.state, "available");
    assert_eq!(result.last_update.unwrap().result, "rolled_back");
}

fn st58_status_reads_result(result: &str, reason: serde_json::Value) {
    let dir = status_dir();
    std::fs::create_dir_all(dir.path().join("update")).unwrap();
    std::fs::write(
        dir.path().join("update/outcomes.jsonl"),
        format!("{}\n", outcome(result, reason)),
    )
    .unwrap();
    assert_eq!(
        read_status(dir.path(), "0.4.0", "2026-11-12T02:00:00Z".parse().unwrap())
            .last_update
            .unwrap()
            .result,
        result
    );
}

#[test]
fn st58_status_reads_applied() {
    st58_status_reads_result("applied", serde_json::Value::Null);
}

#[test]
fn st58_status_reads_rolled_back() {
    st58_status_reads_result(
        "rolled_back",
        json!({"kind":"backup_failed","failed_version":null,"sqlstate":null}),
    );
}

#[test]
fn st58_status_reads_not_applied() {
    st58_status_reads_result(
        "not_applied",
        json!({"kind":"db_ahead","failed_version":null,"sqlstate":null}),
    );
}
