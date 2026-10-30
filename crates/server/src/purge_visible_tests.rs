// SPDX-License-Identifier: AGPL-3.0-only
//! 確認の口の「一覧に出ている重なる滞在」（CH08 Task 4 / design D7）を HTTP の口から見る。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use serde_json::{json, Value};
use tower::ServiceExt as _;
use uuid::Uuid;

use crate::purge_tests::{kept, scene_erase, scene_stay, scope_counts};
use crate::{testdb, App};

const TOKEN: &str = "test-token-0123456789abcdef";
const DAY: &str = "2026-10-01";

async fn app() -> App {
    App::for_test(testdb::pool().await, TOKEN)
}

async fn send(app: &App, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"));
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or_else(Body::empty, |v| Body::from(v.to_string())))
        .unwrap();
    let res = crate::router(app.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

fn t(hhmm: &str) -> String {
    format!("{DAY}T{hhmm}:00+09:00")
}

async fn preview(app: &App, stay: Uuid) -> Value {
    let (code, body) = send(
        app,
        Method::GET,
        &format!("/stays/purge/preview?stay_id={stay}"),
        None,
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{body}");
    body
}

/// 10:00–11:00 の滞在を消す。
async fn erased_p(app: &App, user: Uuid) -> Uuid {
    let p = scene_stay(&app.pool, user, &t("10:00"), &t("11:00")).await;
    scene_erase(&app.pool, p).await;
    p
}

// Scenario: 確認は一覧に出ている重なる滞在の件数を消える件数の外で返す
#[tokio::test]
async fn purge_visible_overlap_is_counted_outside_the_counts() {
    let (app, user) = (app().await, testdb::user());
    let p = erased_p(&app, user).await;
    scene_stay(&app.pool, user, &t("10:30"), &t("11:30")).await;

    let body = preview(&app, p).await;

    assert_eq!(body["visible_overlaps"], 1, "{body}");
    assert_eq!(body["counts"]["stays"], 1, "{body}");
}

// Scenario: 端が触れるだけの一覧に出ている滞在は重なる滞在に数えない
#[tokio::test]
async fn purge_visible_touching_edge_is_not_an_overlap() {
    let (app, user) = (app().await, testdb::user());
    let p = erased_p(&app, user).await;
    scene_stay(&app.pool, user, &t("11:00"), &t("12:00")).await;

    let body = preview(&app, p).await;

    assert_eq!(body["visible_overlaps"], 0, "{body}");
}

// Scenario: 確認の後に一覧に出ている重なる滞在が増えても本文ごと消せる
#[tokio::test]
async fn purge_visible_overlap_added_after_preview_does_not_block_purge() {
    let (app, user) = (app().await, testdb::user());
    let p = erased_p(&app, user).await;
    let seen = preview(&app, p).await;
    assert_eq!(seen["visible_overlaps"], 0, "{seen}");
    let expected = scope_counts(&app.pool, p).await;
    let l = scene_stay(&app.pool, user, &t("10:30"), &t("11:30")).await;

    let (code, body) = send(
        &app,
        Method::POST,
        "/stays/purge",
        Some(json!({"stay_id": p, "expected": serde_json::to_value(expected).unwrap()})),
    )
    .await;

    assert_eq!(code, StatusCode::OK, "{body}");
    assert!(kept(&app.pool, l).await);
}
