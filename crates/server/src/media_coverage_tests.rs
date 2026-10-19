// SPDX-License-Identifier: AGPL-3.0-only
//! 写真と動画の稼働状況（ST09 tasks 8 / design D7・D13・D14・D24）を、本物の PostgreSQL に対して確かめる。
//!
//! 受け口（`/heartbeat` `/ingest`）を通して入れ、`coverage` の読み出しで確かめる。
//! 登録簿の本物の 4 つ（`c01-photo` ほか）は他の試験と共有なので、同じ印（`started_on_basis = heartbeat`）を
//! 持つテスト用のソースで確かめる（`testdb::source_media`）。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::coverage::DayState;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

async fn app() -> App {
    App::for_test(testdb::pool().await, TOKEN)
}

fn auth() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

fn hb(source: &str, user: uuid::Uuid, emitted: &str) -> serde_json::Value {
    serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": source,
        "device_id": "test-dev", "emitted_at": emitted, "capturable": true,
        "blockers": [], "attempts": 6, "successes": 6,
        "raw": format!(r#"{{"at":"{emitted}","n":"{}"}}"#, uuid::Uuid::new_v4()),
    })
}

fn ev(source: &str, user: uuid::Uuid, event_time: &str) -> serde_json::Value {
    serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": source,
        "external_id": null, "device_id": "test-dev", "origin": "collected",
        "event_time": event_time, "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
        "schema_version": 1, "raw": format!(r#"{{"t":"{}"}}"#, uuid::Uuid::new_v4()), "payload": {},
    })
}

async fn post_hb(app: &App, body: serde_json::Value) {
    let (_, Json(res)) = heartbeat_post(State(app.clone()), auth(), Json(body))
        .await
        .expect("生存信号の受け口");
    assert!(res.iter().all(|r| r.accepted), "{res:?}");
}

async fn post_ev(app: &App, body: serde_json::Value) {
    let (_, Json(res)) = ingest(State(app.clone()), auth(), Json(body))
        .await
        .expect("取り込み口");
    assert!(res.iter().all(|r| r.accepted), "{res:?}");
}

async fn started_on(pool: &sqlx::PgPool, source: &str) -> Option<chrono::NaiveDate> {
    sqlx::query_scalar("SELECT collection_started_on FROM core.source WHERE logical_source = $1")
        .bind(source)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// 登録簿から引いた行で、ある 1 日の状態を引く。
async fn state_on(pool: &sqlx::PgPool, user: uuid::Uuid, source: &str, day: &str) -> DayState {
    let rows = coverage::sources(pool, &[source.to_string()])
        .await
        .unwrap();
    let d = testdb::date(day);
    coverage::of_source(pool, Some(user), &rows[0], d, d)
        .await
        .unwrap()
        .days[0]
        .state
}

// ------------------------------------------------------------------ 8.1 接続の欄

/// Scenario: 圏外の写真の生存信号は取れる状態のまま接続が無かったことが残る
#[tokio::test]
async fn st09_cov_net_offline_photo_heartbeat_stays_capturable_and_keeps_network() {
    let app = app().await;
    let s = testdb::source_media(&app.pool, "net-off").await;
    let u = testdb::user();
    let mut b = hb(&s, u, "2026-10-21T03:00:00Z");
    b["network"] = serde_json::json!(false);
    post_hb(&app, b).await;

    assert_eq!(
        state_on(&app.pool, u, &s, "2026-10-21").await,
        DayState::AliveNoRecord
    );
    let net: Option<bool> =
        sqlx::query_scalar("SELECT network FROM core.heartbeat WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(net, Some(false));
}

/// Scenario: 接続の有無の欄を持たない生存信号も受け付ける
#[tokio::test]
async fn st09_cov_net_heartbeat_without_the_field_is_accepted_as_unknown() {
    let app = app().await;
    let s = testdb::source(&app.pool, "net-none", 21_600).await;
    let u = testdb::user();
    post_hb(&app, hb(&s, u, "2026-10-21T03:00:00Z")).await;
    let net: Option<bool> =
        sqlx::query_scalar("SELECT network FROM core.heartbeat WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(net, None);
}

// ------------------------------------------------------------------ 8.2 収集開始日

/// Scenario: 撮影日が 5 年前の写真が届いても写真の収集開始日は動かない
#[tokio::test]
async fn st09_cov_start_a_five_year_old_photo_does_not_move_start() {
    let app = app().await;
    let s = testdb::source_media(&app.pool, "start-old").await;
    let u = testdb::user();
    post_hb(&app, hb(&s, u, "2026-10-20T03:00:00Z")).await;
    assert_eq!(
        started_on(&app.pool, &s).await,
        Some(testdb::date("2026-10-20"))
    );
    post_ev(&app, serde_json::json!([ev(&s, u, "2021-10-20T03:00:00Z")])).await;
    assert_eq!(
        started_on(&app.pool, &s).await,
        Some(testdb::date("2026-10-20"))
    );
}

/// Scenario: 写真の収集開始日は最初の生存信号の日になる
#[tokio::test]
async fn st09_cov_start_first_heartbeat_day_is_the_start() {
    let app = app().await;
    let s = testdb::source_media(&app.pool, "start-first").await;
    let u = testdb::user();
    post_ev(
        &app,
        serde_json::json!([
            ev(&s, u, "2024-01-01T03:00:00Z"),
            ev(&s, u, "2026-10-19T03:00:00Z")
        ]),
    )
    .await;
    post_hb(&app, hb(&s, u, "2026-10-19T15:30:00Z")).await; // JST で 10-20
    assert_eq!(
        started_on(&app.pool, &s).await,
        Some(testdb::date("2026-10-20"))
    );
}

/// Scenario: 生存信号の無い写真のソースは記録があっても開始していない
#[tokio::test]
async fn st09_cov_start_records_without_heartbeat_are_not_started() {
    let app = app().await;
    let s = testdb::source_media(&app.pool, "start-none").await;
    let u = testdb::user();
    post_ev(&app, serde_json::json!([ev(&s, u, "2025-05-01T03:00:00Z")])).await;
    assert_eq!(started_on(&app.pool, &s).await, None);
}

/// Scenario: 入れ直した後の生存信号では写真の収集開始日は後ろへ動かない
#[tokio::test]
async fn st09_cov_start_a_later_heartbeat_does_not_move_start_back() {
    let app = app().await;
    let s = testdb::source_media(&app.pool, "start-later").await;
    let u = testdb::user();
    post_hb(&app, hb(&s, u, "2026-10-20T03:00:00Z")).await;
    post_hb(&app, hb(&s, u, "2026-12-01T03:00:00Z")).await;
    assert_eq!(
        started_on(&app.pool, &s).await,
        Some(testdb::date("2026-10-20"))
    );
}

/// Scenario: スクリーンショットと画像と動画の収集開始日も最初の生存信号の日になる
#[tokio::test]
async fn st09_cov_start_the_other_media_sources_start_on_the_first_heartbeat() {
    let app = app().await;
    let u = testdb::user();
    for prefix in ["start-shot", "start-image", "start-video"] {
        let s = testdb::source_media(&app.pool, prefix).await;
        post_ev(&app, serde_json::json!([ev(&s, u, "2023-03-03T03:00:00Z")])).await;
        post_hb(&app, hb(&s, u, "2026-10-20T03:00:00Z")).await;
        assert_eq!(
            started_on(&app.pool, &s).await,
            Some(testdb::date("2026-10-20")),
            "{prefix}"
        );
    }
}

// ------------------------------------------------------------------ 8.3 状態の判定

/// 収集開始日を直に置いた媒体のソース。
async fn started_media(pool: &sqlx::PgPool, prefix: &str, start: &str) -> String {
    let s = testdb::source_media(pool, prefix).await;
    testdb::set_started_on(pool, &s, start).await;
    s
}

/// Scenario: 見始めた日より前の撮影日の写真がある日は記録ありになる
#[tokio::test]
async fn st09_cov_state_a_photo_before_the_start_makes_the_day_recorded() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = started_media(&pool, "st-rec", "2026-10-20").await;
    testdb::put_event(&pool, u, &s, "2025-05-01T03:00:00Z").await;
    assert_eq!(
        state_on(&pool, u, &s, "2025-05-01").await,
        DayState::Recorded
    );
}

/// Scenario: 見始めた日より前で写真の無い日は導入前のまま
#[tokio::test]
async fn st09_cov_state_a_day_before_the_start_without_a_photo_is_before_start() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = started_media(&pool, "st-pre", "2026-10-20").await;
    testdb::put_event(&pool, u, &s, "2025-05-01T03:00:00Z").await;
    assert_eq!(
        state_on(&pool, u, &s, "2025-05-02").await,
        DayState::BeforeStart
    );
}

/// Scenario: 見始めた日より前の記録ありの日は達成の分母に入らない
#[tokio::test]
async fn st09_cov_state_days_before_the_start_stay_out_of_the_denominator() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = started_media(&pool, "st-denom", "2026-10-20").await;
    for k in 1..=30 {
        let day = testdb::date("2026-10-20") - chrono::Duration::days(k);
        testdb::put_event(&pool, u, &s, &format!("{day}T03:00:00Z")).await;
    }
    testdb::put_heartbeat(&pool, u, &s, "2026-10-20T03:00:00Z", true).await;
    let got = coverage::achievement(
        &pool,
        Some(u),
        testdb::date("2026-10-25"),
        &[(s.clone(), coverage::Subject::Usage)],
    )
    .await
    .unwrap();
    // 10-20 から 10-24 の 5 日。生存信号は 10-20 の 1 日だけ
    assert_eq!(got.sources[0].denominator, 5);
    assert_eq!(got.sources[0].achieved_days, 1);
}

/// Scenario: 成功条件 1 の写真はカメラの写真だけを数える
#[test]
fn st09_cov_state_success_condition_one_counts_only_the_camera_photo() {
    let names: Vec<String> = coverage::must_sources()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(names.len(), 5);
    assert!(names.iter().any(|n| n == "c01-photo"));
    for other in ["c01-screenshot", "c01-image", "c01-video"] {
        assert!(!names.iter().any(|n| n == other), "{other}");
    }
}

/// Scenario: 生存信号の無い写真のソースでも記録のある日は記録あり
#[tokio::test]
async fn st09_cov_state_a_record_day_is_recorded_without_any_heartbeat() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = testdb::source_media(&pool, "st-nohb").await;
    testdb::put_event(&pool, u, &s, "2025-05-01T03:00:00Z").await;
    assert_eq!(started_on(&pool, &s).await, None);
    assert_eq!(
        state_on(&pool, u, &s, "2025-05-01").await,
        DayState::Recorded
    );
    assert_eq!(
        state_on(&pool, u, &s, "2025-05-02").await,
        DayState::BeforeStart
    );
}

/// Scenario: 消えた事実の記録がある日は記録ありになる
#[tokio::test]
async fn st09_cov_state_a_vanished_fact_makes_the_day_recorded() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = started_media(&pool, "st-vanish", "2026-10-20").await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-11-03").await,
        DayState::Outage,
        "対照: 記録が無ければ途絶"
    );
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test','collected','2026-11-03T03:00:00Z',540,'Asia/Tokyo',1,$4,'{}',
                 '{\"kind\":\"vanished\"}')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(u)
    .bind(&s)
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        state_on(&pool, u, &s, "2026-11-03").await,
        DayState::Recorded
    );
}

/// Scenario: 見始めた日より前の写真は途絶の判定に使われない
#[tokio::test]
async fn st09_cov_state_photos_before_the_start_do_not_make_an_outage() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let s = started_media(&pool, "st-nooutage", "2026-10-20").await;
    testdb::put_event(&pool, u, &s, "2026-10-10T03:00:00Z").await;
    let rows = coverage::sources(&pool, std::slice::from_ref(&s))
        .await
        .unwrap();
    let got = coverage::of_source(
        &pool,
        Some(u),
        &rows[0],
        testdb::date("2026-10-01"),
        testdb::date("2026-10-19"),
    )
    .await
    .unwrap();
    for c in &got.days {
        let want = if c.day == testdb::date("2026-10-10") {
            DayState::Recorded
        } else {
            DayState::BeforeStart
        };
        assert_eq!(c.state, want, "{}", c.day);
    }
    assert!(
        got.days.iter().all(|c| c.state != DayState::Outage),
        "見始めた日より前に始まる途絶の期間があってはならない"
    );
}
