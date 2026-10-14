// SPDX-License-Identifier: AGPL-3.0-only
//! 格子で数えない記録（取りこぼしの主張・時計の測定）と、取りこぼしの期間（ST14 / design D4 / D15）。
//! **本物の PostgreSQL に対して**確かめる。
//!
//! 数えない組は `(logical_source, payload->>'kind')` の完全一致なので、登録簿の本物の名前
//! （`c02-window` / `c01-app-usage`）を使う。利用者は試験ごとに固有なので他の試験とは混ざらない。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde_json::json;

const WINDOW: &str = "c02-window";
const USAGE: &str = "c01-app-usage";

fn row(name: &str, gap: i32, started: &str) -> SourceRow {
    SourceRow {
        logical_source: name.to_string(),
        named_source: name.to_string(),
        display_name: name.to_string(),
        expected_gap_sec: gap,
        collection_started_on: Some(testdb::date(started)),
        retired_on: None,
        chain: vec![name.to_string()],
    }
}

/// 種類つきの記録を直に置く。`at` は RFC3339。
async fn put_kind(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    at: &str,
    payload: serde_json::Value,
) {
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test','collected',$4::timestamptz,540,'Asia/Tokyo',1,$5,'{}',$6)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(source)
    .bind(at)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(payload)
    .execute(pool)
    .await
    .unwrap();
}

async fn put_clock(pool: &sqlx::PgPool, user: uuid::Uuid, at: &str) {
    put_kind(pool, user, WINDOW, at, json!({"kind": "clock-skew"})).await;
}

/// 取りこぼしの期間を主張する記録。出来事の時刻は `end`（D15）。
async fn put_gap(pool: &sqlx::PgPool, user: uuid::Uuid, begin: &str, end: &str, reason: &str) {
    put_kind(
        pool,
        user,
        USAGE,
        end,
        json!({"kind": "gap", "begin": begin, "end": end, "reason": reason}),
    )
    .await;
}

async fn cell(pool: &sqlx::PgPool, user: uuid::Uuid, s: &SourceRow, day: &str) -> DayCell {
    let d = testdb::date(day);
    of_source(pool, Some(user), s, d, d)
        .await
        .unwrap()
        .days
        .remove(0)
}

// Scenario: 時計の測定しか無い日は記録ありにならない
#[tokio::test]
async fn st14_grid_nonobs_clock_only_day_is_not_recorded() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(WINDOW, SIX_HOURS, "2026-05-01");
    for h in [3, 9, 15] {
        put_clock(&pool, u, &format!("2026-06-10T{h:02}:00:00+09:00")).await;
    }
    testdb::put_heartbeat(&pool, u, WINDOW, "2026-06-10T03:00:00Z", false).await;
    let c = cell(&pool, u, &s, "2026-06-10").await;
    assert_eq!(c.state, DayState::AliveNotCapturable);
    // 数えないのは格子だけ。件数の列は記録そのものを数え続ける
    assert_eq!(c.event_count, 0);
}

// Scenario: 時計の測定しか無く生存信号も無い日は途絶になりうる
#[tokio::test]
async fn st14_grid_nonobs_clock_only_without_heartbeat_can_be_outage() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(WINDOW, SIX_HOURS, "2026-05-01");
    for day in ["09", "10", "11"] {
        put_clock(&pool, u, &format!("2026-06-{day}T12:00:00+09:00")).await;
    }
    assert_eq!(state_on(&pool, u, &s, "2026-06-10").await, DayState::Outage);
}

// Scenario: 時計の測定だけが届く期間も途絶になる
#[tokio::test]
async fn st14_grid_nonobs_hourly_clock_only_period_is_outage() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(WINDOW, SIX_HOURS, "2026-05-01");
    for day in ["09", "10", "11"] {
        for h in 0..24 {
            put_clock(&pool, u, &format!("2026-06-{day}T{h:02}:00:00+09:00")).await;
        }
    }
    assert_eq!(state_on(&pool, u, &s, "2026-06-10").await, DayState::Outage);
}

// Scenario: 前景の記録がある日は時計の測定があっても記録あり
#[tokio::test]
async fn st14_grid_nonobs_foreground_day_stays_recorded() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(WINDOW, SIX_HOURS, "2026-05-01");
    for h in 0..24 {
        put_clock(&pool, u, &format!("2026-06-10T{h:02}:00:00+09:00")).await;
    }
    put_kind(
        &pool,
        u,
        WINDOW,
        "2026-06-10T10:00:00+09:00",
        json!({"kind": "foreground", "app_name": "x"}),
    )
    .await;
    let c = cell(&pool, u, &s, "2026-06-10").await;
    assert_eq!(c.state, DayState::Recorded);
    assert_eq!(c.event_count, 1, "格子の件数は観測だけ");
}

const TOKEN: &str = "t14-grid-token-0123456789abcdef";

fn ingest_body(user: uuid::Uuid, at: &str, kind: &str, n: usize) -> serde_json::Value {
    serde_json::Value::Array(
        (0..n)
            .map(|i| {
                json!({
                    "id": uuid::Uuid::new_v4(),
                    "user_id": user,
                    "logical_source": WINDOW,
                    "external_id": null,
                    "device_id": "test-dev",
                    "origin": "collected",
                    "event_time": at,
                    "tz_offset_min": 540,
                    "tz_id": "Asia/Tokyo",
                    "schema_version": 1,
                    "raw": format!(r#"{{"i":{i}}}"#),
                    "payload": {"kind": kind},
                })
            })
            .collect(),
    )
}

async fn ingest_clock(user: uuid::Uuid, at: &str, n: usize) -> sqlx::PgPool {
    let app = crate::App::for_test(testdb::pool().await, TOKEN);
    let mut h = HeaderMap::new();
    h.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    let _ = crate::ingest(
        State(app.clone()),
        h,
        Json(ingest_body(user, at, "clock-skew", n)),
    )
    .await
    .expect("取り込み口");
    app.pool.clone()
}

// Scenario: 格子で数えない記録も読み出せる
#[tokio::test]
async fn st14_grid_nonobs_clock_record_is_readable() {
    let u = testdb::user();
    let pool = ingest_clock(u, "2026-06-10T12:00:00+09:00", 1).await;
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.event_live
          WHERE user_id = $1 AND logical_source = $2 AND payload->>'kind' = 'clock-skew'",
    )
    .bind(u)
    .bind(WINDOW)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 1);
}

// Scenario: 格子で数えない記録も稼働記録の件数には入る
#[tokio::test]
async fn st14_grid_nonobs_clock_records_count_in_coverage_rows() {
    let u = testdb::user();
    let pool = ingest_clock(u, "2026-06-10T12:00:00+09:00", 3).await;
    let n: i32 = sqlx::query_scalar(
        "SELECT event_count FROM core.coverage
          WHERE user_id = $1 AND logical_source = $2 AND day = '2026-06-10'",
    )
    .bind(u)
    .bind(WINDOW)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 3);
}

// Scenario: 取りこぼしの期間を主張する 1 件だけでは記録ありにならない
#[tokio::test]
async fn st14_grid_nonobs_a_gap_claim_alone_is_not_recorded() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(USAGE, SIX_HOURS, "2026-05-01");
    // 取りこぼしの期間は 06-10 の始まりで終わる（`[begin, end)`）ので 06-10 には重ならない。主張の記録は 06-10 にある
    put_gap(
        &pool,
        u,
        "2026-06-08T08:00:00+09:00",
        "2026-06-10T00:00:00+09:00",
        "retention",
    )
    .await;
    testdb::put_heartbeat(&pool, u, USAGE, "2026-06-10T03:00:00Z", true).await;
    // 主張の記録は出来事の時刻（= 終わり）の日 06-10 にあるが、その日の格子には数えない
    let c = cell(&pool, u, &s, "2026-06-10").await;
    assert_eq!(c.state, DayState::AliveNoRecord);
    assert_eq!(c.event_count, 0);
    assert!(c.gaps.is_empty());
}

// Scenario: 取りこぼしの期間の日は取得できる状態の信号があっても取れない状態になる
#[tokio::test]
async fn st14_grid_gap_overlap_beats_capturable_heartbeat() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(USAGE, SIX_HOURS, "2026-05-01");
    put_gap(
        &pool,
        u,
        "2026-06-08T08:00:00+09:00",
        "2026-06-10T08:00:00+09:00",
        "retention",
    )
    .await;
    testdb::put_heartbeat(&pool, u, USAGE, "2026-06-09T03:00:00Z", true).await;
    let c = cell(&pool, u, &s, "2026-06-09").await;
    assert_eq!(c.event_count, 0);
    assert_eq!(c.state, DayState::AliveNotCapturable);
}

// Scenario: イベントの記録がある日は取りこぼしの期間が重なっても記録あり
#[tokio::test]
async fn st14_grid_gap_recorded_day_stays_recorded() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(USAGE, SIX_HOURS, "2026-05-01");
    put_gap(
        &pool,
        u,
        "2026-06-09T20:00:00+09:00",
        "2026-06-10T04:00:00+09:00",
        "retention",
    )
    .await;
    put_kind(
        &pool,
        u,
        USAGE,
        "2026-06-10T10:00:00+09:00",
        json!({"event_type": 1, "package": "p"}),
    )
    .await;
    let c = cell(&pool, u, &s, "2026-06-10").await;
    assert_eq!(c.state, DayState::Recorded);
    assert_eq!(c.gaps.len(), 1, "期間は状態に関わらず返る");
}

// Scenario: 取りこぼしの期間より停止が優先される
#[tokio::test]
async fn st14_grid_gap_stop_beats_gap() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(USAGE, SIX_HOURS, "2026-05-01");
    put_gap(
        &pool,
        u,
        "2026-06-09T20:00:00+09:00",
        "2026-06-10T04:00:00+09:00",
        "retention",
    )
    .await;
    // 06-10 を丸ごと覆う停止（JST 06-10 00:00〜06-11 00:00）
    testdb::put_span(
        &pool,
        u,
        USAGE,
        "stopped",
        "2026-06-09T15:00:00Z",
        Some("2026-06-10T15:00:00Z"),
    )
    .await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-06-10").await,
        DayState::Stopped
    );
}

// Scenario: 終わりの時刻ちょうどの日には重ならない
#[tokio::test]
async fn st14_grid_gap_end_exactly_at_midnight_does_not_overlap() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(USAGE, SIX_HOURS, "2026-05-01");
    put_gap(
        &pool,
        u,
        "2026-09-01T08:00:00+09:00",
        "2026-09-03T00:00:00+09:00",
        "retention",
    )
    .await;
    let r = of_source(
        &pool,
        Some(u),
        &s,
        testdb::date("2026-09-01"),
        testdb::date("2026-09-03"),
    )
    .await
    .unwrap();
    assert!(r.days[2].gaps.is_empty(), "09-03 は持たない");
    assert_eq!(r.days[1].gaps.len(), 1, "09-02 は持つ");
    assert_eq!(r.days[0].gaps.len(), 1, "09-01 は持つ");
}

// Scenario: UTC の真夜中に終わる取りこぼしは日本時間のその日に重なる
#[tokio::test]
async fn st14_grid_gap_utc_midnight_end_overlaps_the_jst_day() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(USAGE, SIX_HOURS, "2026-05-01");
    put_gap(
        &pool,
        u,
        "2026-09-01T08:00:00+09:00",
        "2026-09-03T00:00:00Z",
        "retention",
    )
    .await;
    let c = cell(&pool, u, &s, "2026-09-03").await;
    assert_eq!(c.gaps.len(), 1);
}

// Scenario: 稼働状況の応答の日が取りこぼしの期間と理由を持つ
#[tokio::test]
async fn st14_grid_gap_day_carries_span_and_reason() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(USAGE, SIX_HOURS, "2026-05-01");
    put_gap(
        &pool,
        u,
        "2026-09-01T08:00:00+09:00",
        "2026-09-03T09:00:00+09:00",
        "retention",
    )
    .await;
    let c = cell(&pool, u, &s, "2026-09-02").await;
    assert_eq!(c.gaps.len(), 1);
    let g = &c.gaps[0];
    assert_eq!(g.reason, "retention");
    assert_eq!(g.begin.to_rfc3339(), "2026-08-31T23:00:00+00:00");
    assert_eq!(g.end.to_rfc3339(), "2026-09-03T00:00:00+00:00");
}

// 壊れた begin（存在しない日付）が 1 件あっても稼働状況ぜんぶは落ちず、その 1 件は配られない
#[tokio::test]
async fn st14_grid_gap_unreadable_begin_is_skipped() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = row(USAGE, SIX_HOURS, "2026-05-01");
    put_gap(
        &pool,
        u,
        "2026-02-30T08:00:00+09:00",
        "2026-06-10T08:00:00+09:00",
        "retention",
    )
    .await;
    testdb::put_heartbeat(&pool, u, USAGE, "2026-06-10T03:00:00Z", true).await;
    let c = cell(&pool, u, &s, "2026-06-10").await;
    assert!(c.gaps.is_empty());
}
