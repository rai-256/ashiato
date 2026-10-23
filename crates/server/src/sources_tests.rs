// SPDX-License-Identifier: AGPL-3.0-only
//! `GET /sources`（ST13 Task 8 / design D10）を、router を通して本物の PostgreSQL に対して確かめる。
//! 書庫は試験ごとに固有の利用者・固有のソースに置く（台帳は追記だけ）。共有の登録簿の行は読むだけ。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{DateTime, Duration, Utc};
use tower::ServiceExt as _;

use super::*;
use crate::{outage, outage_store, testdb};

const TOKEN: &str = "test-token-0123456789abcdef";
const DAYS: i32 = 75;

fn now() -> DateTime<Utc> {
    "2026-10-21T03:00:00Z".parse().unwrap()
}

async fn app() -> App {
    App::for_test(testdb::pool().await, TOKEN).at(now())
}

async fn call(app: &App, user: uuid::Uuid, auth: bool) -> (StatusCode, String) {
    let mut req = Request::builder()
        .method(Method::GET)
        .uri(format!("/sources?user_id={user}"));
    if auth {
        req = req.header(header::AUTHORIZATION, format!("Bearer {TOKEN}"));
    }
    let res = router(app.clone())
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

async fn row_of(user: uuid::Uuid, name: &str) -> serde_json::Value {
    let (status, body) = call(&app().await, user, true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    v["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["logical_source"] == name)
        .unwrap_or_else(|| panic!("{name} の行が無い: {body}"))
        .clone()
}

/// 日数 75 の書庫のソース。
async fn archive_source(pool: &sqlx::PgPool) -> String {
    let name = testdb::source(pool, "sources-arc", 5_184_000).await;
    sqlx::query("UPDATE core.source SET alert_after_days = $2 WHERE logical_source = $1")
        .bind(&name)
        .bind(DAYS)
        .execute(pool)
        .await
        .unwrap();
    name
}

/// 最後の書庫を `ago` 前に置く。
async fn ledger(pool: &sqlx::PgPool, user: uuid::Uuid, source: &str, ago: Duration) {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome, created_at, discovered_at)
         VALUES ($1, $2, 'v1', 'read', $3, $3) RETURNING id",
    )
    .bind(user)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(now() - ago)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.archive_ledger_source (ledger_id, logical_source, inserted_count, duplicate_count)
         VALUES ($1, $2, 1, 0)",
    )
    .bind(id)
    .bind(source)
    .execute(pool)
    .await
    .unwrap();
}

/// 途切れの判定が、このソースを途切れとするか。
async fn judged_outage(pool: &sqlx::PgPool, user: uuid::Uuid, source: &str) -> bool {
    testdb::set_started_on(pool, source, "2025-01-01").await;
    let inputs = outage_store::inputs(pool, user, now()).await.unwrap();
    outage::judge(&inputs, now())
        .iter()
        .any(|o| o.logical_source == source)
}

// Scenario: 書庫のソースは手作業と起点つきの通知までの残り日数が出る
#[tokio::test]
async fn sources_archive_53_days_ago_shows_manual_and_days_left() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    let name = archive_source(&pool).await;
    ledger(&pool, user, &name, Duration::days(53)).await;
    let r = row_of(user, &name).await;
    assert_eq!(r["alert_after_days"], 75);
    assert_eq!(r["alert_days_left"], 22);
    assert_eq!(r["alert_passed"], false);
    assert_eq!(r["no_archive"], false);
    assert_eq!(r["last_archive_on"], "2026-08-29");
    assert!(
        r.get("latest_at").is_none(),
        "いちばん新しい記録の時刻は返さない"
    );
}

// Scenario: 通知の日を過ぎた書庫のソースはそのことが出る
#[tokio::test]
async fn sources_archive_80_days_ago_is_passed_without_negative_days() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    let name = archive_source(&pool).await;
    ledger(&pool, user, &name, Duration::days(80)).await;
    let r = row_of(user, &name).await;
    assert_eq!(r["alert_passed"], true);
    assert!(r["alert_days_left"].is_null(), "負の残り日数は出ない: {r}");
    assert_eq!(r["last_archive_on"], "2026-08-02");
    assert!(judged_outage(&pool, user, &name).await);
}

// Scenario: 通知が鳴る 1 時間前の書庫のソースは残り 1 日と出る
#[tokio::test]
async fn sources_archive_one_hour_before_alert_shows_one_day_left() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    let name = archive_source(&pool).await;
    ledger(&pool, user, &name, Duration::days(74) + Duration::hours(23)).await;
    let r = row_of(user, &name).await;
    assert_eq!(r["alert_days_left"], 1);
    assert_eq!(r["alert_passed"], false);
    assert!(
        !judged_outage(&pool, user, &name).await,
        "途切れの判定と一致"
    );
}

// Scenario: 通知が鳴った 1 時間後の書庫のソースは過ぎたと出る
#[tokio::test]
async fn sources_archive_one_hour_after_alert_is_passed() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    let name = archive_source(&pool).await;
    ledger(&pool, user, &name, Duration::days(75) + Duration::hours(1)).await;
    let r = row_of(user, &name).await;
    assert_eq!(r["alert_passed"], true);
    assert!(r["alert_days_left"].is_null());
    assert!(
        judged_outage(&pool, user, &name).await,
        "途切れの判定と一致"
    );
}

#[tokio::test]
async fn sources_archive_exactly_75_days_is_not_passed_like_outage() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    let name = archive_source(&pool).await;
    ledger(&pool, user, &name, Duration::days(75)).await;
    let r = row_of(user, &name).await;
    assert_eq!(r["alert_passed"], false);
    // 鳴るちょうどの時刻は過ぎていないが「あと 0 日」は出さない（design D21（仮）。final review R9）。
    assert_eq!(r["alert_days_left"], 1);
    assert!(
        !judged_outage(&pool, user, &name).await,
        "ちょうどは途切れでない"
    );
}

// Scenario: 書庫も記録も無い書庫のソースはまだ書庫が無いと出る
#[tokio::test]
async fn sources_archive_without_ledger_or_records_says_no_archive() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    let name = archive_source(&pool).await;
    let r = row_of(user, &name).await;
    assert_eq!(r["no_archive"], true);
    assert!(r["alert_days_left"].is_null());
    assert!(r["last_archive_on"].is_null());
    assert_eq!(r["alert_passed"], false);
}

// Scenario: 退役したソースは通知までの残り日数を出さない
#[tokio::test]
async fn sources_retired_archive_has_no_days_left() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    let name = archive_source(&pool).await;
    ledger(&pool, user, &name, Duration::days(53)).await;
    testdb::retire(&pool, &name, "2026-09-01").await;
    let r = row_of(user, &name).await;
    assert_eq!(r["retired_on"], "2026-09-01");
    assert!(r["alert_days_left"].is_null());
    assert!(
        r["last_archive_on"].is_null(),
        "退役した行は起点の日も出さない"
    );
    assert_eq!(r["alert_passed"], false);
    assert_eq!(r["no_archive"], false);
}

#[tokio::test]
async fn sources_location_is_device_every_60_seconds() {
    let r = row_of(testdb::user(), "c01-location").await;
    assert_eq!(r["acquisition"], "device");
    assert_eq!(r["fetch_trigger"], "interval");
    assert_eq!(r["fetch_interval_sec"], 60);
    assert!(r["alert_after_days"].is_null());
}

// Scenario: 休止の対象から外れていないことが位置の格子の見出しに出る
#[tokio::test]
async fn sources_device_notice_reads_latest_location_heartbeat_raw() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    let (status, body) = call(&app().await, user, true).await;
    assert_eq!(status, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(v["device_notice"]["unused_app_restrictions"].is_null());
    for (at, state) in [
        ("2026-10-21T01:00:00Z", "exempt"),
        ("2026-10-21T02:00:00Z", "not_exempt"),
    ] {
        sqlx::query(
            "INSERT INTO core.heartbeat
               (id, user_id, logical_source, device_id, emitted_at, capturable, blockers,
                attempts, successes, content_hash, raw)
             VALUES ($1,$2,'c01-location','test',$3::timestamptz,true,'{}',1,1,$4,$5)",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(user)
        .bind(at)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(serde_json::json!({"unused_app_restrictions": state}))
        .execute(&pool)
        .await
        .unwrap();
    }
    let (_, body) = call(&app().await, user, true).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["device_notice"]["unused_app_restrictions"], "not_exempt");
}

/// 最新は届いた順で選ぶ（code_verify R24・design D23（仮））。時計が 1 日進んでいた端末の古い `not_exempt` が、
/// 時計を直して除外も済ませた後の `exempt` に勝たない。
#[tokio::test]
async fn sources_device_notice_follows_the_latest_received_heartbeat() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    for (emitted, received, state) in [
        (
            "now() + interval '1 day'",
            "now() - interval '1 hour'",
            "not_exempt",
        ),
        ("now()", "now()", "exempt"),
    ] {
        sqlx::query(&format!(
            "INSERT INTO core.heartbeat
               (id, user_id, logical_source, device_id, emitted_at, received_at, capturable, blockers,
                attempts, successes, content_hash, raw)
             VALUES ($1,$2,'c01-location','test',{emitted},{received},true,'{{}}',1,1,$3,$4)"
        ))
        .bind(uuid::Uuid::new_v4())
        .bind(user)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(serde_json::json!({"unused_app_restrictions": state}))
        .execute(&pool)
        .await
        .unwrap();
    }
    let (_, body) = call(&app().await, user, true).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["device_notice"]["unused_app_restrictions"], "exempt");
}

#[tokio::test]
async fn sources_device_notice_is_null_when_latest_heartbeat_raw_is_not_json() {
    let (pool, user) = (testdb::pool().await, testdb::user());
    sqlx::query(
        "INSERT INTO core.heartbeat
           (id, user_id, logical_source, device_id, emitted_at, capturable, blockers,
            attempts, successes, content_hash, raw)
         VALUES ($1,$2,'c01-location','test','2026-10-21T03:00:00Z'::timestamptz,true,'{}',1,1,$3,'not json')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(&pool)
    .await
    .unwrap();
    let (status, body) = call(&app().await, user, true).await;
    assert_eq!(status, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(v["device_notice"]["unused_app_restrictions"].is_null());
    assert!(v["sources"].as_array().is_some_and(|a| !a.is_empty()));
}

// Scenario: ソースの一覧は認証の無い要求には返らない
#[tokio::test]
async fn sources_without_credentials_is_401_without_list() {
    let (status, body) = call(&app().await, testdb::user(), false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(!body.contains("logical_source"), "一覧が返った: {body}");
}
