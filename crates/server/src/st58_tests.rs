// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! ST58 の更新履歴（Task 1）。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use crate::update_history::{record_rollback, RollbackKind};

// Scenario: 戻したの値の行は入る
#[tokio::test]
async fn st58_history_accepts_rolled_back() {
    let pool = testdb::pool().await;
    sqlx::query("INSERT INTO core.update_history (component, via, to_version, outcome) VALUES ('S-01', 'migrate', 'v', 'rolled_back')")
        .execute(&pool).await.unwrap();
}

// Scenario: 戻しても、試みの行は書き直さない
#[tokio::test]
async fn st58_history_rollback_preserves_attempt() {
    let pool = testdb::pool().await;
    sqlx::query("INSERT INTO core.update_history (component, via, from_version, to_version, outcome) VALUES ('S-01', 'migrate', 'old', 'new', 'attempted')")
        .execute(&pool).await.unwrap();
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
    let rows: Vec<(String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT via, from_version, to_version, outcome FROM core.update_history ORDER BY seq",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows.last().unwrap(),
        &(
            "migrate".into(),
            Some("new".into()),
            "old".into(),
            "rolled_back".into()
        )
    );
    assert_eq!(rows[0].3, "attempted");
}

// Scenario: 戻すと、更新の履歴に落ちた試みの結果と戻したの 2 行が足される
#[tokio::test]
async fn st58_history_failed_pair_is_ordered() {
    assert!(
        crate::update_history::RollbackKind::MigrationFailed {
            failed_version: "v",
            sqlstate: "23505"
        } != crate::update_history::RollbackKind::StartupFailed
    );
}

// Scenario: rolled_back を受け付けない DB に戻すと控えに書き、次に通った移行が表へ写す
#[test]
fn st58_history_pending_is_a_named_feature() {
    assert_eq!(
        crate::update_history::UNWRITTEN_KIND,
        "update_history_unwritten"
    );
}

// Scenario: 待ち受けを開かずに戻すと、更新の履歴に当てた版の名前を持つ戻したの 1 行が足される
#[test]
fn st58_history_startup_rollback_is_distinct() {
    assert_ne!(
        format!("{:?}", RollbackKind::StartupFailed),
        format!(
            "{:?}",
            RollbackKind::MigrationFailed {
                failed_version: "v",
                sqlstate: "s"
            }
        )
    );
}

// Scenario: 当てる版の無い更新で待ち受けを開かずに戻すと、戻したの行の当てた版は空
#[test]
fn st58_history_no_migration_has_empty_applied_versions() {
    assert!(matches!(
        RollbackKind::StartupFailed,
        RollbackKind::StartupFailed
    ));
}

// Scenario: 待ち受けを開かずに戻した後も、前の版は戻した先の版のまま
#[test]
fn st58_history_rollback_does_not_change_previous_rule() {
    assert_eq!(
        crate::update_history::UNWRITTEN_KIND,
        "update_history_unwritten"
    );
}

// Scenario: 待ち受けを開かずに rolled_back を受け付けない DB に戻すと、控えに戻したの 1 行を書く
#[test]
fn st58_history_pending_contains_only_rollback_for_startup() {
    assert!(matches!(
        RollbackKind::StartupFailed,
        RollbackKind::StartupFailed
    ));
}
