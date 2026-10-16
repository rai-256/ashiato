// SPDX-License-Identifier: AGPL-3.0-only
//! アプリ利用の集計（ST14 / design D10（仮）/ D16）。開始日の借り方と格子の並び。**本物の PostgreSQL に対して**確かめる。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

/// 借りるソースと借り先を登録簿へ置く。借り先の開始日は `lender_started`。
async fn pair(pool: &sqlx::PgPool, lender_started: &str) -> (String, String) {
    let lender = testdb::source(pool, "st14-rollup-lender", SIX_HOURS).await;
    let borrower = testdb::source(pool, "st14-rollup-borrower", SIX_HOURS).await;
    testdb::set_started_on(pool, &lender, lender_started).await;
    sqlx::query("UPDATE core.source SET started_on_from = $2 WHERE logical_source = $1")
        .bind(&borrower)
        .bind(&lender)
        .execute(pool)
        .await
        .unwrap();
    (lender, borrower)
}

async fn started_on(pool: &sqlx::PgPool, name: &str) -> Option<chrono::NaiveDate> {
    sources(pool, &[name.to_string()]).await.unwrap()[0].collection_started_on
}

async fn touch_record(pool: &sqlx::PgPool, name: &str, at: &str) {
    touch_started_on(pool, name, at.parse().unwrap(), Arrival::Record)
        .await
        .unwrap();
}

/// Scenario: 集計の開始日はアプリ利用の開始日になる
#[tokio::test]
async fn st14_rollup_started_follows_app_usage() {
    let pool = testdb::pool().await;
    let (_lender, borrower) = pair(&pool, "2026-09-20").await;

    touch_record(&pool, &borrower, "2024-09-30T03:00:00Z").await;

    assert_eq!(
        started_on(&pool, &borrower).await,
        Some(testdb::date("2026-09-20")),
        "集計の収集開始日がアプリ利用のものになっていない"
    );
    let got = sources(&pool, std::slice::from_ref(&borrower))
        .await
        .unwrap();
    let u = testdb::user();
    assert_eq!(
        state_on(&pool, u, &got[0], "2025-01-01").await,
        DayState::BeforeStart,
        "集計の 2025-01-01 が導入前になっていない"
    );
}

/// Scenario: 集計の記録を送り直しても開始日は戻らない
#[tokio::test]
async fn st14_rollup_started_survives_resend() {
    let pool = testdb::pool().await;
    let (_lender, borrower) = pair(&pool, "2026-09-20").await;

    touch_record(&pool, &borrower, "2024-09-30T03:00:00Z").await;
    touch_record(&pool, &borrower, "2024-09-30T03:00:00Z").await;

    assert_eq!(
        started_on(&pool, &borrower).await,
        Some(testdb::date("2026-09-20"))
    );
    // 保存されている値も、自分では動かしていない
    let own: Option<chrono::NaiveDate> = sqlx::query_scalar(
        "SELECT collection_started_on FROM core.source WHERE logical_source = $1",
    )
    .bind(&borrower)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(own, None, "借りるソースの記録が自分の開始日を書いた");
}

/// Scenario: アプリ利用の開始日が遡ると集計の開始日も遡る
#[tokio::test]
async fn st14_rollup_started_follows_lender_backwards() {
    let pool = testdb::pool().await;
    let (lender, borrower) = pair(&pool, "2026-09-20").await;

    touch_record(&pool, &lender, "2026-09-15T03:00:00Z").await;

    assert_eq!(
        started_on(&pool, &borrower).await,
        Some(testdb::date("2026-09-15"))
    );
}

/// 借り先が引き継ぎの鎖を持つなら、鎖の根の開始日を借りる（D16）。
#[tokio::test]
async fn st14_rollup_started_borrows_from_chain_root() {
    let pool = testdb::pool().await;
    let (lender, borrower) = pair(&pool, "2026-09-20").await;
    let root = testdb::source(&pool, "st14-rollup-root", SIX_HOURS).await;
    testdb::set_started_on(&pool, &root, "2026-08-01").await;
    sqlx::query("UPDATE core.source SET succeeds = $2 WHERE logical_source = $1")
        .bind(&lender)
        .bind(&root)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        started_on(&pool, &borrower).await,
        Some(testdb::date("2026-08-01"))
    );
}

/// Scenario: 集計の格子が Must の後ろで書庫の前に出る
#[tokio::test]
async fn st14_rollup_order_between_must_and_archive() {
    let app = crate::App::for_test(testdb::pool().await, "test-token-0123456789abcdef");
    let mut h = axum::http::HeaderMap::new();
    h.insert(
        "authorization",
        "Bearer test-token-0123456789abcdef".parse().unwrap(),
    );
    let axum::Json(got) = crate::coverage_get(
        axum::extract::State(app),
        h,
        axum::extract::Query(crate::CoverageQuery {
            from: testdb::date("2026-03-01"),
            to: testdb::date("2026-03-07"),
            user_id: None,
        }),
    )
    .await
    .unwrap();
    let names: Vec<&str> = got.iter().map(|s| s.logical_source.as_str()).collect();
    assert_eq!(
        &names[..6],
        [
            "c01-location",
            "c01-app-usage",
            "c01-photo",
            "c02-window",
            "c02-browser-history",
            "c01-app-usage-rollup",
        ]
    );
    assert!(names.len() > 6, "書庫のソースが無いと並びを確かめられない");
    assert!(names[6..].iter().all(|n| n.starts_with("c03-")));
}

/// Scenario: 集計は達成に数えられない
#[test]
fn st14_rollup_order_not_in_achievement() {
    assert!(
        must_sources().iter().all(|(n, _)| n != ROLLUP_SOURCE),
        "集計の論理ソースが達成の 5 本に入っている"
    );
    assert_eq!(must_sources().len(), 5);
}
