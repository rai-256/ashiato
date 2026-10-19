// SPDX-License-Identifier: AGPL-3.0-only
//! 原本の欠けの受け口（ST09 tasks 7.1 / design D11）を、本物の PostgreSQL に対して確かめる。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use tower::ServiceExt as _;

use crate::media_missing::MediaMissingResult;
use crate::testdb;
use crate::{coverage, router, App};

const TOKEN: &str = "test-token-0123456789abcdef";

async fn app() -> App {
    App::for_test(testdb::pool().await, TOKEN)
}

fn sha() -> String {
    format!(
        "{:x}",
        sha2::Sha256::digest(uuid::Uuid::new_v4().as_bytes())
    )
}

use sha2::Digest as _;

fn report(user: uuid::Uuid, sha: &str, reason: &str) -> serde_json::Value {
    serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": "c01-photo",
        "device_id": "phone-01", "sha256": sha, "reason": reason,
        "noticed_at": "2026-10-01T03:00:00Z",
    })
}

async fn post(
    app: &App,
    body: serde_json::Value,
    auth: bool,
) -> (StatusCode, Vec<MediaMissingResult>) {
    let mut b = Request::builder()
        .method(Method::POST)
        .uri("/media/missing")
        .header("content-type", "application/json");
    if auth {
        b = b.header("authorization", format!("Bearer {TOKEN}"));
    }
    let res = router(app.clone())
        .oneshot(b.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    let results = serde_json::from_slice::<Vec<serde_json::Value>>(&bytes)
        .map(|v| {
            v.into_iter()
                .map(|r| MediaMissingResult {
                    id: r["id"].as_str().and_then(|s| s.parse().ok()),
                    duplicate: r["duplicate"].as_bool().unwrap_or(false),
                    accepted: r["accepted"].as_bool().unwrap_or(false),
                    error: None,
                })
                .collect()
        })
        .unwrap_or_default();
    (status, results)
}

async fn count(app: &App, user: uuid::Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM core.media_missing WHERE user_id = $1")
        .bind(user)
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

/// Scenario: 送る前に端末から消えた写真の原本は欠けとして残る
#[tokio::test]
async fn st09_miss_vanished_original_is_kept_with_reason_and_hash() {
    let app = app().await;
    let user = testdb::user();
    let sha = sha();
    let (code, res) = post(
        &app,
        serde_json::json!([report(user, &sha, "vanished")]),
        true,
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert!(res[0].accepted && !res[0].duplicate);
    let row: (String, String, String) = sqlx::query_as(
        "SELECT reason, sha256, logical_source FROM core.media_missing WHERE user_id = $1",
    )
    .bind(user)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(row, ("vanished".into(), sha, "c01-photo".into()));
}

/// Scenario: 原本の欠けは破棄の報告に載らない
#[tokio::test]
async fn st09_miss_does_not_count_as_drop() {
    let app = app().await;
    let user = testdb::user();
    let source = testdb::source(&app.pool, "miss-cov", 21_600).await;
    let (code, _) = post(
        &app,
        serde_json::json!([report(user, &sha(), "vanished")]),
        true,
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    let drops: i64 = sqlx::query_scalar("SELECT count(*) FROM core.drop_report WHERE user_id = $1")
        .bind(user)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(drops, 0);
    let day = testdb::date("2026-10-01");
    let src = coverage::SourceRow {
        logical_source: source.clone(),
        named_source: source.clone(),
        display_name: source.clone(),
        expected_gap_sec: 21_600,
        collection_started_on: Some(testdb::date("2026-05-01")),
        retired_on: None,
        chain: vec![source],
    };
    let got = coverage::of_source(&app.pool, Some(user), &src, day, day)
        .await
        .unwrap();
    assert_eq!(got.days[0].dropped_count, 0);
    assert_ne!(got.days[0].state, coverage::DayState::Dropped);
}

/// Scenario: 原本の欠けの報告を送り直しても 1 件のまま
#[tokio::test]
async fn st09_miss_resend_stays_one() {
    let app = app().await;
    let user = testdb::user();
    let item = report(user, &sha(), "changed");
    let (_, first) = post(&app, item.clone(), true).await;
    let (code, second) = post(&app, serde_json::json!([item]), true).await;
    assert!(!first[0].duplicate);
    assert_eq!(code, StatusCode::OK);
    assert!(second[0].accepted && second[0].duplicate);
    assert_eq!(count(&app, user).await, 1);
}

#[tokio::test]
async fn st09_miss_bad_items_are_refused_one_by_one() {
    let app = app().await;
    let user = testdb::user();
    let mut bad_reason = report(user, &sha(), "gone");
    bad_reason["reason"] = serde_json::json!("gone");
    let mut bad_sha = report(user, "ABC", "vanished");
    bad_sha["sha256"] = serde_json::json!("ABC");
    let mut video = report(user, &sha(), "vanished");
    video["logical_source"] = serde_json::json!("c01-video");
    let good = report(user, &sha(), "unreadable");
    let (code, res) = post(
        &app,
        serde_json::json!([bad_reason, bad_sha, video, good]),
        true,
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    let accepted: Vec<bool> = res.iter().map(|r| r.accepted).collect();
    assert_eq!(accepted, [false, false, false, true]);
    assert_eq!(count(&app, user).await, 1);
    // 1 件も受け付けなければ 400
    let (code, _) = post(
        &app,
        serde_json::json!([report(user, "x", "vanished")]),
        true,
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn st09_miss_reused_id_with_other_content_is_refused() {
    let app = app().await;
    let user = testdb::user();
    let item = report(user, &sha(), "vanished");
    post(&app, item.clone(), true).await;
    let mut other = item;
    other["sha256"] = serde_json::json!(sha());
    let (code, res) = post(&app, other, true).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(!res[0].accepted);
    assert_eq!(count(&app, user).await, 1);
}

/// Scenario: 資格情報の無い原本の欠けの報告は断られる
#[tokio::test]
async fn st09_miss_without_credentials_is_refused() {
    let app = app().await;
    let user = testdb::user();
    let (code, _) = post(
        &app,
        serde_json::json!([report(user, &sha(), "vanished")]),
        false,
    )
    .await;
    assert_eq!(code, StatusCode::UNAUTHORIZED);
    assert_eq!(count(&app, user).await, 0);
}
