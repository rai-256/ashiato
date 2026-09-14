// SPDX-License-Identifier: AGPL-3.0-only
//! 停止と破棄（範囲で置く）。**本物の PostgreSQL に対して**確かめる。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

/// 停止は時刻の範囲として残る（FR-34 / 深掘り Q3）。
///
/// Scenario: 半日の停止が時刻の範囲で残る
#[tokio::test]
async fn stop_is_stored_as_time_range() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "stopspan", SIX_HOURS, Some("2026-05-01")).await;
    // 2026-05-01 09:00〜18:00 JST = 00:00〜09:00 UTC
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "stopped",
        "2026-05-01T00:00:00Z",
        Some("2026-05-01T09:00:00Z"),
    )
    .await;
    let row: (
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT started_at, ended_at FROM core.coverage_span
          WHERE logical_source = $1 AND kind = 'stopped'",
    )
    .bind(&s.logical_source)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0.to_rfc3339(), "2026-05-01T00:00:00+00:00");
    assert_eq!(
        row.1.map(|t| t.to_rfc3339()),
        Some("2026-05-01T09:00:00+00:00".to_string())
    );
}

/// 破棄は期間と件数で残る（FR-9）。
///
/// Scenario: 破棄が期間と件数で残る
#[tokio::test]
async fn drop_is_stored_with_count() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "dropspan", SIX_HOURS, Some("2026-05-01")).await;
    sqlx::query(
        "INSERT INTO core.coverage_span
           (id, user_id, logical_source, kind, started_at, ended_at, event_count)
         VALUES ($1,$2,$3,'dropped','2026-05-01T00:00:00Z','2026-05-02T00:00:00Z',1234)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(u)
    .bind(&s.logical_source)
    .execute(&pool)
    .await
    .unwrap();
    let row: (
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<i32>,
    ) = sqlx::query_as(
        "SELECT started_at, ended_at, event_count FROM core.coverage_span
          WHERE logical_source = $1 AND kind = 'dropped'",
    )
    .bind(&s.logical_source)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.1.is_some());
    assert_eq!(row.2, Some(1234));
}

/// 逆順の範囲は DB が拒む（tasks 1.3）。**アプリ層だけに置かない** ——
/// psql を直に叩く経路（PERM-8）が素通りする。
#[tokio::test]
async fn span_rejects_reversed_range() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "revspan", SIX_HOURS, None).await;
    let bad = sqlx::query(
        "INSERT INTO core.coverage_span
           (id, user_id, logical_source, kind, started_at, ended_at)
         VALUES ($1,$2,$3,'stopped','2026-05-02T00:00:00Z','2026-05-01T00:00:00Z')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(u)
    .bind(&s.logical_source)
    .execute(&pool)
    .await;
    assert!(bad.is_err(), "終わりが始まりより前の範囲が入った");
}
