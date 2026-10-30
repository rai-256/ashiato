// SPDX-License-Identifier: AGPL-3.0-only
//! 戻す口（`POST /stays/restore`）の断りを HTTP の口から見る（CH08 Task 3 / design D4）。
//! 場面は premises.md の探り A1〜A3: P 10:00–11:00 の位置 5 分おき・B のはみ出しの位置。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt as _;
use uuid::Uuid;

use crate::purge_tests::{kept, purge_now, scene_erase, scene_location, scene_stay};
use crate::{testdb, App};

const TOKEN: &str = "test-token-0123456789abcdef";
const DAY: &str = "2026-10-01";

async fn app() -> App {
    App {
        // 戻した後の作り直しは走らせない（この試験の外の振る舞い）
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..App::for_test(testdb::pool().await, TOKEN)
    }
}

async fn restore(app: &App, stays: &[Uuid]) -> (StatusCode, String) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/stays/restore")
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"stay_ids": stays}).to_string()))
        .unwrap();
    let res = crate::router(app.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn t(hhmm: &str) -> String {
    format!("{DAY}T{hhmm}:00+09:00")
}

async fn stay(pool: &PgPool, user: Uuid, from: &str, to: &str) -> Uuid {
    scene_stay(pool, user, &t(from), &t(to)).await
}

async fn locations(pool: &PgPool, user: Uuid, hhmm: &[&str]) -> Vec<Uuid> {
    let mut ids = Vec::new();
    for w in hhmm {
        ids.push(scene_location(pool, user, &t(w)).await);
    }
    ids
}

/// 10:00–11:00 の滞在 P と、その位置を 5 分おき（10:00〜11:00）に置く。
async fn scene_p(pool: &PgPool, user: Uuid) -> Uuid {
    let p = stay(pool, user, "10:00", "11:00").await;
    let times: Vec<String> = (0..=12)
        .map(|i| format!("{:02}:{:02}", 10 + (i * 5) / 60, (i * 5) % 60))
        .collect();
    let refs: Vec<&str> = times.iter().map(String::as_str).collect();
    locations(pool, user, &refs).await;
    p
}

/// B を消し、P を消して本文ごと消す（探り A1〜A3 の順）。B の位置（B の時間にあるもの）は B と一緒に消える。
async fn erase_b_then_purge_p(pool: &PgPool, b: Uuid, p: Uuid) {
    scene_erase(pool, b).await;
    scene_erase(pool, p).await;
    purge_now(pool, p).await;
}

async fn marks(pool: &PgPool, ids: &[Uuid]) -> Vec<(Uuid, Option<String>)> {
    sqlx::query_as("SELECT id, deleted_by FROM core.event WHERE id = ANY($1) ORDER BY id")
        .bind(ids)
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn deleted_with_body(pool: &PgPool, ids: &[Uuid]) {
    for (id, mark) in marks(pool, ids).await {
        assert!(mark.is_some(), "{id} の削除の印が外れた");
        assert!(kept(pool, id).await, "{id} の本文が消えた");
    }
}

/// B の時間の位置すべてが削除の印を持ったまま、本文ごと消した時間（〜11:00）の外の位置と B は本文が残る。
async fn refused_state_holds(pool: &PgPool, user: Uuid, b: Uuid) {
    let all = locations_in(pool, user, "10:40", "11:05").await;
    for (id, mark) in marks(pool, &all).await {
        assert!(mark.is_some(), "{id} の削除の印が外れた");
    }
    let mut outside = locations_in(pool, user, "11:01", "11:05").await;
    assert!(!outside.is_empty());
    outside.push(b);
    deleted_with_body(pool, &outside).await;
}

/// B の時間にある位置の識別子。
async fn locations_in(pool: &PgPool, user: Uuid, from: &str, to: &str) -> Vec<Uuid> {
    sqlx::query_scalar(
        "SELECT id FROM core.event
          WHERE user_id = $1 AND logical_source = 'c01-location'
            AND event_time >= $2::timestamptz AND event_time <= $3::timestamptz",
    )
    .bind(user)
    .bind(t(from))
    .bind(t(to))
    .fetch_all(pool)
    .await
    .unwrap()
}

// Scenario: 本文ごと消した時間にすっぽり入る消した滞在の戻す求めは断られる
#[tokio::test]
async fn purge_restore_nested_stay_is_refused() {
    let (app, user) = (app().await, testdb::user());
    let p = scene_p(&app.pool, user).await;
    let b = stay(&app.pool, user, "10:15", "10:45").await;
    erase_b_then_purge_p(&app.pool, b, p).await;

    let (code, body) = restore(&app, &[b]).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    let (mark,): (Option<String>,) =
        sqlx::query_as("SELECT deleted_by FROM core.event WHERE id = $1")
            .bind(b)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(
        mark.as_deref(),
        Some("user"),
        "削除の印を持ったまま（作り直しで吸収されてもいない）"
    );
    assert!(kept(&app.pool, b).await, "本文が残る");
}

// Scenario: はみ出しが滞在にならない消した滞在の戻す求めは断られる
#[tokio::test]
async fn purge_restore_overhang_that_is_no_stay_is_refused() {
    let (app, user) = (app().await, testdb::user());
    let p = scene_p(&app.pool, user).await;
    locations(&app.pool, user, &["11:05", "11:10"]).await;
    let b = stay(&app.pool, user, "10:40", "11:05").await;
    erase_b_then_purge_p(&app.pool, b, p).await;

    let (code, body) = restore(&app, &[b]).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    refused_state_holds(&app.pool, user, b).await;
}

// Scenario: はみ出しが滞在になる消した滞在は戻せる
#[tokio::test]
async fn purge_restore_overhang_that_is_a_stay_can_be_restored() {
    let (app, user) = (app().await, testdb::user());
    let p = scene_p(&app.pool, user).await;
    locations(
        &app.pool,
        user,
        &["11:05", "11:10", "11:15", "11:20", "11:25", "11:30"],
    )
    .await;
    let b = stay(&app.pool, user, "10:40", "11:30").await;
    erase_b_then_purge_p(&app.pool, b, p).await;

    let (code, body) = restore(&app, &[b]).await;

    assert_eq!(code, StatusCode::OK, "{body}");
    let live: bool = sqlx::query_scalar("SELECT deleted_at IS NULL FROM core.event WHERE id = $1")
        .bind(b)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert!(live, "B が一覧に戻る");
}

// Scenario: 戻せない滞在を含む戻す求めは何も戻さない
#[tokio::test]
async fn purge_restore_a_request_with_a_refused_stay_restores_nothing() {
    let (app, user) = (app().await, testdb::user());
    let p = scene_p(&app.pool, user).await;
    let nested = stay(&app.pool, user, "10:15", "10:45").await;
    let other = stay(&app.pool, user, "14:00", "15:00").await;
    scene_erase(&app.pool, other).await;
    erase_b_then_purge_p(&app.pool, nested, p).await;

    let (code, body) = restore(&app, &[nested, other]).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    deleted_with_body(&app.pool, &[nested, other]).await;
}

// Scenario: 戻せない滞在の断りは本文を消した記録の断りと区別できる
#[tokio::test]
async fn purge_restore_refusal_differs_from_the_purged_record_refusal() {
    let (app, user) = (app().await, testdb::user());
    let p = scene_p(&app.pool, user).await;
    let b = stay(&app.pool, user, "10:15", "10:45").await;
    erase_b_then_purge_p(&app.pool, b, p).await;

    let (nested_code, nested_body) = restore(&app, &[b]).await;
    let (purged_code, purged_body) = restore(&app, &[p]).await;

    assert_eq!(nested_code, StatusCode::CONFLICT);
    assert_eq!(purged_code, StatusCode::CONFLICT);
    assert_ne!(nested_body, purged_body, "断りの理由が同じ");
}

// Scenario: 戻せるかを確かめる作り直しが落ちると何も戻さない
#[tokio::test]
async fn purge_restore_a_failing_rebuild_restores_nothing() {
    let (app, user) = (app().await, testdb::user());
    let p = scene_p(&app.pool, user).await;
    locations(&app.pool, user, &["11:05", "11:10"]).await;
    let b = stay(&app.pool, user, "10:40", "11:05").await;
    erase_b_then_purge_p(&app.pool, b, p).await;
    crate::deletion::RESTORE_REBUILD_SCOPE.with(|s| *s.borrow_mut() = Some("no-such-scope"));

    let (code, body) = restore(&app, &[b]).await;
    crate::deletion::RESTORE_REBUILD_SCOPE.with(|s| *s.borrow_mut() = None);

    assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    refused_state_holds(&app.pool, user, b).await;
}
