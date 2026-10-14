// SPDX-License-Identifier: AGPL-3.0-only
//! 途切れの通知の登録簿と台帳を、全移行を当てた後の状態で見る（ST14 Task 1 / design D2 / D7）。
#![allow(clippy::unwrap_used)]

use crate::archive::{worker::ensure_myactivity_source, ARCHIVE_ALERT_AFTER_DAYS};
use crate::testdb;

const MIGRATION: &str = include_str!("../../../migrations/202610141130_outage_alert.sql");

/// 移行の「値を当てる」部分（`UPDATE` だけ）。当て直しの試験はここだけを当てる ——
/// 列や表の `ALTER` / `CREATE` は他の試験の登録と表の錠を取り合ってデッドロックする。
/// 版全体の 2 回目の適用は `tools/check-immutable.sh` が見る。
fn data_part() -> &'static str {
    let from = MIGRATION.find("-- 判定の対象外の印").unwrap();
    let to = MIGRATION.find("-- 途切れと戻りと届けたことの台帳").unwrap();
    &MIGRATION[from..to]
}

async fn target_of(pool: &sqlx::PgPool, name: &str) -> Option<bool> {
    sqlx::query_scalar("SELECT outage_target FROM core.source WHERE logical_source = $1")
        .bind(name)
        .fetch_optional(pool)
        .await
        .unwrap()
}

async fn days_of(pool: &sqlx::PgPool, name: &str) -> Option<i32> {
    sqlx::query_scalar("SELECT alert_after_days FROM core.source WHERE logical_source = $1")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Scenario: 時計の測定と集計と滞在と個人属性は対象外で登録されている
#[tokio::test]
async fn st14_reg_excluded_sources_are_marked() {
    let pool = testdb::pool().await;
    // 登録簿に本当にある行だけを見る（ST17 / ST21 が先に入っていなければ無い名前もある）
    for name in [
        "c01-clock",
        "c01-app-usage-rollup",
        "s01-stay",
        "s01-attribute",
    ] {
        if let Some(target) = target_of(&pool, name).await {
            assert!(!target, "{name} が対象のまま");
        }
    }
    for name in [
        "c01-location",
        "c01-app-usage",
        "c01-photo",
        "c02-window",
        "c02-browser-history",
        "s01-archive-inbox",
    ] {
        if let Some(target) = target_of(&pool, name).await {
            assert!(target, "{name} が対象外");
        }
    }
    let archive: Vec<String> = sqlx::query_scalar(
        "SELECT logical_source FROM core.source WHERE logical_source LIKE 'c03-%' AND NOT outage_target",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(archive.is_empty(), "書庫のソースが対象外: {archive:?}");
}

/// Scenario: 書庫のソースは 75 日で登録されている
#[tokio::test]
async fn st14_reg_archive_sources_are_75_days() {
    let pool = testdb::pool().await;
    ensure_myactivity_source(&pool, "c03-myactivity-st14reg", "検索")
        .await
        .unwrap();
    assert_eq!(ARCHIVE_ALERT_AFTER_DAYS, 75);
    let rows: Vec<(String, Option<i32>)> =
        sqlx::query_as("SELECT logical_source, alert_after_days FROM core.source WHERE logical_source LIKE 'c03-%'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(rows.len() >= 10, "書庫のソースが少ない: {}", rows.len());
    for (name, days) in &rows {
        // 実行時に足す行は別の試験が日数なしで直に入れることがある（`day_records_tests`）。
        // 実行時の経路は `st14_reg_runtime_myactivity_source_is_75_days` が `ensure_myactivity_source` 越しに見る
        if name.starts_with("c03-myactivity-") {
            continue;
        }
        assert_eq!(*days, Some(ARCHIVE_ALERT_AFTER_DAYS), "{name}");
    }
    let others: Vec<String> = sqlx::query_scalar(
        "SELECT logical_source FROM core.source
          WHERE logical_source NOT LIKE 'c03-%' AND logical_source NOT LIKE 't-%'
            AND alert_after_days IS NOT NULL",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(others.is_empty(), "書庫以外が日数を持つ: {others:?}");
}

/// Scenario: 本人が変えた通知までの日数は移行を当て直しても戻らない
#[tokio::test]
async fn st14_reg_user_changed_days_survive_remigration() {
    let pool = testdb::pool().await;
    // 他の試験と共有する本物の行は取引の外で書き換えない: 取引の中で当てて戻す
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "UPDATE core.source SET alert_after_days = 60 WHERE logical_source = 'c03-youtube-watch'",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::raw_sql(data_part()).execute(&mut *tx).await.unwrap();
    let days: Option<i32> = sqlx::query_scalar(
        "SELECT alert_after_days FROM core.source WHERE logical_source = 'c03-youtube-watch'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(days, Some(60));
}

/// Scenario: 汚れた集計の開始日は移行で直る
#[tokio::test]
async fn st14_reg_dirty_rollup_started_on_is_repaired() {
    let pool = testdb::pool().await;
    // 他の試験と共有する本物の行は書き換えない: トランザクションの中で当てて戻す
    let mut tx = pool.begin().await.unwrap();
    for name in ["c01-app-usage", "c01-app-usage-rollup"] {
        sqlx::query(
            "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
             VALUES ($1, $1, 21600, 'none') ON CONFLICT DO NOTHING",
        )
        .bind(name)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE core.source SET collection_started_on = '2026-09-20' WHERE logical_source = 'c01-app-usage'")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE core.source SET collection_started_on = '2024-09-30' WHERE logical_source = 'c01-app-usage-rollup'")
        .execute(&mut *tx)
        .await
        .unwrap();
    let events_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.event WHERE logical_source = 'c01-app-usage-rollup'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    sqlx::raw_sql(data_part()).execute(&mut *tx).await.unwrap();
    let (started, from): (chrono::NaiveDate, Option<String>) = sqlx::query_as(
        "SELECT collection_started_on, started_on_from FROM core.source WHERE logical_source = 'c01-app-usage-rollup'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(started, testdb::date("2026-09-20"));
    assert_eq!(from.as_deref(), Some("c01-app-usage"));
    let events_after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.event WHERE logical_source = 'c01-app-usage-rollup'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(events_before, events_after);
    tx.rollback().await.unwrap();
}

async fn ledger_row(pool: &sqlx::PgPool) -> i64 {
    let source = testdb::source(pool, "st14-ledger", 3600).await;
    sqlx::query_scalar(
        "INSERT INTO core.outage_ledger (user_id, logical_source, kind, last_alive_at)
         VALUES ($1, $2, 'opened', now()) RETURNING id",
    )
    .bind(testdb::user())
    .bind(source)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Scenario: 途切れの台帳の行は書き換えられない
#[tokio::test]
async fn st14_reg_ledger_rows_are_append_only() {
    let pool = testdb::pool().await;
    let id = ledger_row(&pool).await;
    let update = sqlx::query("UPDATE core.outage_ledger SET judged_at = now() WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;
    assert!(update.is_err(), "更新が通った");
    let delete = sqlx::query("DELETE FROM core.outage_ledger WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;
    assert!(delete.is_err(), "削除が通った");
}

/// 途切れの始まり（`opened`）は最後に生きていた時刻を必ず持つ（design D7 / review/code.md R6）。
/// 戻り・届けたことは持たない
#[tokio::test]
async fn st14_reg_ledger_opened_requires_last_alive_at() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "st14-ledger-shape", 3600).await;
    let opened = sqlx::query(
        "INSERT INTO core.outage_ledger (user_id, logical_source, kind, last_alive_at)
         VALUES ($1, $2, 'opened', NULL)",
    )
    .bind(testdb::user())
    .bind(&source)
    .execute(&pool)
    .await;
    let err = opened.expect_err("最後に生きていた時刻の無い opened が通った");
    assert!(
        err.to_string()
            .contains("outage_ledger_opened_has_last_alive"),
        "{err}"
    );
    let id = ledger_row(&pool).await;
    for (kind, device) in [("closed", None), ("delivered", Some("dev-a"))] {
        let with_alive = sqlx::query(
            "INSERT INTO core.outage_ledger (user_id, logical_source, kind, episode_id, device_id, last_alive_at)
             VALUES ($1, $2, $3, $4, $5, now())",
        )
        .bind(testdb::user())
        .bind(&source)
        .bind(kind)
        .bind(id)
        .bind(device)
        .execute(&pool)
        .await;
        assert!(with_alive.is_err(), "{kind} が最後に生きていた時刻を持てた");
    }
}

/// 「本人が書いた・導出した記録を持つか」（`has_non_collected`）を引く部分索引がある（design D27（仮）/ review/code.md R3）。
/// 収集した記録しか持たないソースで一致が無いとき、利用者 × ソースの全行を読まないため
#[tokio::test]
async fn st14_reg_event_has_non_collected_partial_index() {
    let pool = testdb::pool().await;
    let def: Option<String> = sqlx::query_scalar(
        "SELECT indexdef FROM pg_indexes
          WHERE schemaname = 'core' AND tablename = 'event' AND indexname = 'event_non_collected_by_source'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    let def = def.expect("部分索引 event_non_collected_by_source が無い");
    assert!(def.contains("(user_id, logical_source)"), "{def}");
    assert!(def.contains("WHERE (origin <> 'collected'::text)"), "{def}");
}

/// Scenario: 途切れの台帳に記録の中身が載らない
#[tokio::test]
async fn st14_reg_ledger_has_no_record_content_columns() {
    let pool = testdb::pool().await;
    let mut columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'outage_ledger'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    columns.sort();
    let mut expected = [
        "id",
        "user_id",
        "logical_source",
        "kind",
        "episode_id",
        "device_id",
        "last_alive_at",
        "judged_at",
    ]
    .map(String::from)
    .to_vec();
    expected.sort();
    assert_eq!(columns, expected);
    assert!(columns.iter().any(|c| c == "user_id"));
}

/// Scenario: 実行時に足された製品のソースも 75 日で登録される
#[tokio::test]
async fn st14_reg_runtime_myactivity_source_is_75_days() {
    let pool = testdb::pool().await;
    let name = format!("c03-myactivity-st14-{}", uuid::Uuid::new_v4());
    ensure_myactivity_source(&pool, &name, "テスト")
        .await
        .unwrap();
    assert_eq!(days_of(&pool, &name).await, Some(75));
    sqlx::query("DELETE FROM core.source WHERE logical_source = $1")
        .bind(&name)
        .execute(&pool)
        .await
        .unwrap();
}

/// Scenario: 新しく登録したソースは判定の対象になる
#[tokio::test]
async fn st14_reg_default_new_source_is_a_target() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "st14-default", 3600).await;
    assert_eq!(target_of(&pool, &name).await, Some(true));
    assert_eq!(days_of(&pool, &name).await, None);
}
