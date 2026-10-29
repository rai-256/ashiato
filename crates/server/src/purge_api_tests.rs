// SPDX-License-Identifier: AGPL-3.0-only
//! 本文ごと消す口（`GET /stays/purge/preview`・`POST /stays/purge`）と、戻す口の断り（`POST /stays/restore`）を
//! HTTP の口から見る（ST23 Task 4 / design D1 / D6）。範囲の数え方と消去の核は `purge_tests` が見る。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt as _;
use uuid::Uuid;

use crate::deletion::PurgeCounts;
use crate::purge_tests::{
    at, emptied, kept, purge_now, scene_basic, scene_erase, scene_event, scene_location,
    scene_mark_rebuild, scene_stay, scene_version, scope_counts,
};
use crate::{testdb, App};

const TOKEN: &str = "test-token-0123456789abcdef";

async fn app() -> App {
    App {
        // 戻した後の作り直しは走らせない（この試験の外の振る舞い）
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..App::for_test(testdb::pool().await, TOKEN)
    }
}

async fn send(
    app: &App,
    method: Method,
    uri: &str,
    auth: bool,
    body: Option<Value>,
) -> (StatusCode, String) {
    let mut b = Request::builder().method(method).uri(uri);
    if auth {
        b = b.header(header::AUTHORIZATION, format!("Bearer {TOKEN}"));
    }
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or_else(Body::empty, |v| Body::from(v.to_string())))
        .unwrap();
    let res = crate::router(app.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn json_of(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|_| panic!("JSON でない応答: {body}"))
}

fn counts_json(c: &PurgeCounts) -> Value {
    serde_json::to_value(c).unwrap()
}

async fn preview(app: &App, stay: Uuid) -> (StatusCode, String) {
    send(
        app,
        Method::GET,
        &format!("/stays/purge/preview?stay_id={stay}"),
        true,
        None,
    )
    .await
}

async fn purge_req(app: &App, body: Value) -> (StatusCode, String) {
    send(app, Method::POST, "/stays/purge", true, Some(body)).await
}

async fn purge_with(app: &App, stay: Uuid, expected: &PurgeCounts) -> (StatusCode, String) {
    purge_req(
        app,
        json!({"stay_id": stay, "expected": counts_json(expected)}),
    )
    .await
}

async fn restore(app: &App, stays: &[Uuid]) -> (StatusCode, String) {
    send(
        app,
        Method::POST,
        "/stays/restore",
        true,
        Some(json!({"stay_ids": stays})),
    )
    .await
}

/// 範囲の全記録の `raw` を id 順に読む（前後を比べる）。
async fn raws(pool: &PgPool, ids: &[Uuid]) -> Vec<(Uuid, String)> {
    sqlx::query_as("SELECT id, raw FROM core.event WHERE id = ANY($1) ORDER BY id")
        .bind(ids)
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn ledger_rows(pool: &PgPool, user: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM core.erasure_operation WHERE user_id = $1")
        .bind(user)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// 消した滞在 1 つと位置 3 件（10:00〜11:00）。
async fn scene_erased(pool: &PgPool, user: Uuid) -> (Uuid, Vec<Uuid>) {
    let (stay, locations) = scene_basic(pool, user).await;
    scene_erase(pool, stay).await;
    (stay, locations)
}

// ------------------------------------------------------------------ 消す口

// Scenario: 消した滞在を本文ごと消すと 200 で件数が返る
#[tokio::test]
async fn purge_api_returns_counts_on_success() {
    let (app, user) = (app().await, testdb::user());
    let (stay, _) = scene_erased(&app.pool, user).await;
    let expected = scope_counts(&app.pool, stay).await;

    let (code, body) = purge_with(&app, stay, &expected).await;

    assert_eq!(code, StatusCode::OK, "{body}");
    let v = json_of(&body);
    assert_eq!(v["counts"]["stays"], 1);
    assert_eq!(v["counts"]["locations"], 3);
    assert_eq!(v["counts"]["archive_locations"], 0);
    assert_eq!(v["counts"]["archive_items"], 0);
    assert_eq!(v["counts"]["versions"], 0);
    assert!(v["operation_id"].is_string());
}

// Scenario: 消えていない滞在は本文ごと消せない
#[tokio::test]
async fn purge_api_refuses_a_stay_that_is_not_erased() {
    let (app, user) = (app().await, testdb::user());
    let (stay, locations) = scene_basic(&app.pool, user).await;

    let (code, body) = purge_with(&app, stay, &PurgeCounts::default()).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    assert_eq!(json_of(&body)["reason"], "not_erased");
    assert!(kept(&app.pool, stay).await);
    assert!(kept(&app.pool, locations[0]).await);
}

// Scenario: 作り直しが隠しただけの滞在は本文ごと消せない
#[tokio::test]
async fn purge_api_refuses_a_stay_only_hidden_by_rebuild() {
    let (app, user) = (app().await, testdb::user());
    let (stay, _) = scene_basic(&app.pool, user).await;
    scene_mark_rebuild(&app.pool, stay, "rebuild:absorbed").await;

    let (code, body) = purge_with(&app, stay, &PurgeCounts::default()).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    assert_eq!(json_of(&body)["reason"], "not_erased");
    assert!(kept(&app.pool, stay).await);
}

// Scenario: 滞在でない記録の識別子では本文ごと消せない
#[tokio::test]
async fn purge_api_refuses_a_record_that_is_not_a_stay() {
    let (app, user) = (app().await, testdb::user());
    let (_, locations) = scene_erased(&app.pool, user).await;

    let (code, _) = purge_with(&app, locations[0], &PurgeCounts::default()).await;

    assert_eq!(code, StatusCode::NOT_FOUND);
    assert!(kept(&app.pool, locations[0]).await);
}

// Scenario: 他の利用者を指定して本文ごと消すことはできない
#[tokio::test]
async fn purge_api_refuses_another_user() {
    let (app, user) = (app().await, testdb::user());
    let (stay, _) = scene_erased(&app.pool, user).await;
    let expected = scope_counts(&app.pool, stay).await;

    let (code, _) = purge_req(
        &app,
        json!({"stay_id": stay, "user_id": Uuid::new_v4(), "expected": counts_json(&expected)}),
    )
    .await;

    assert_eq!(code, StatusCode::NOT_FOUND);
    assert!(kept(&app.pool, stay).await);
}

// Scenario: 資格情報の無い本文ごと消す求めは断られる
#[tokio::test]
async fn purge_api_requires_credentials() {
    let (app, user) = (app().await, testdb::user());
    let (stay, _) = scene_erased(&app.pool, user).await;

    let (code, _) = send(
        &app,
        Method::POST,
        "/stays/purge",
        false,
        Some(json!({"stay_id": stay, "expected": counts_json(&PurgeCounts::default())})),
    )
    .await;

    assert_eq!(code, StatusCode::UNAUTHORIZED);
    assert!(kept(&app.pool, stay).await);
}

// Scenario: 知らない識別子を本文ごと消そうとすると断られる
#[tokio::test]
async fn purge_api_refuses_an_unknown_id() {
    let app = app().await;

    let (code, _) = purge_with(&app, Uuid::new_v4(), &PurgeCounts::default()).await;

    assert_eq!(code, StatusCode::NOT_FOUND);
}

// Scenario: 本文ごと消した滞在をもう一度指定しても何も起きない
#[tokio::test]
async fn purge_api_second_press_changes_nothing() {
    let (app, user) = (app().await, testdb::user());
    let (stay, _) = scene_erased(&app.pool, user).await;
    let expected = scope_counts(&app.pool, stay).await;
    let (first, _) = purge_with(&app, stay, &expected).await;
    assert_eq!(first, StatusCode::OK);
    let before = ledger_rows(&app.pool, user).await;

    let (code, body) = purge_with(&app, stay, &expected).await;

    assert_eq!(code, StatusCode::OK, "{body}");
    let v = json_of(&body);
    assert_eq!(
        counts_json(&PurgeCounts::default()),
        v["counts"],
        "消した件数は 0"
    );
    assert_eq!(
        ledger_rows(&app.pool, user).await,
        before,
        "台帳に行が増えない"
    );
}

// Scenario: 組に入らない記録が範囲にあると断られる
#[tokio::test]
async fn purge_api_refuses_a_source_outside_every_group() {
    let (app, user) = (app().await, testdb::user());
    let (stay, locations) = scene_erased(&app.pool, user).await;
    let photo = scene_event(
        &app.pool,
        user,
        "c01-photo",
        "2026-10-01T10:30:00+09:00",
        r#"{"lat":35.68}"#,
    )
    .await;
    scene_mark_rebuild(&app.pool, photo, "user:cascade").await;
    sqlx::query(
        "INSERT INTO core.deletion_ledger (event_id, user_id, logical_source, action, cause_event_id, mark)
         VALUES ($1, $2, 'c01-photo', 'erase', $3, 'user:cascade')",
    )
    .bind(photo)
    .bind(user)
    .bind(stay)
    .execute(&app.pool)
    .await
    .unwrap();
    let ids = [vec![stay, photo], locations].concat();
    let before = raws(&app.pool, &ids).await;

    let (code, body) = purge_with(&app, stay, &PurgeCounts::default()).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    assert_eq!(json_of(&body)["reason"], "unknown_source");
    assert_eq!(raws(&app.pool, &ids).await, before, "範囲のどの本文も残る");
}

// ------------------------------------------------------------------ 確認の口

// Scenario: 確認は消える件数をソースの組ごとに返す
#[tokio::test]
async fn purge_api_preview_counts_by_group() {
    let (app, user) = (app().await, testdb::user());
    let pool = &app.pool;
    // 位置 70 件・前の版 20 件の滞在に、書庫の位置 3 件と位置を持つ書庫の項目 1 件が重なる
    let stay = scene_stay(
        pool,
        user,
        "2026-10-01T10:05:00+09:00",
        "2026-10-01T11:48:00+09:00",
    )
    .await;
    for minute in 0..70 {
        let when = at("2026-10-01T10:05:00+09:00") + chrono::Duration::minutes(minute);
        scene_location(pool, user, &when.to_rfc3339()).await;
    }
    for no in 1..=20 {
        scene_version(
            pool,
            stay,
            user,
            no,
            "2026-10-01T10:05:00+09:00",
            "2026-10-01T11:00:00+09:00",
        )
        .await;
    }
    for minute in [10, 20, 30] {
        let when = format!("2026-10-01T10:{minute}:00+09:00");
        scene_event(
            pool,
            user,
            "c03-timeline-visit",
            &when,
            r#"{"lat":35.68,"lon":139.76}"#,
        )
        .await;
    }
    scene_event(
        pool,
        user,
        "c03-youtube-watch",
        "2026-10-01T10:50:00+09:00",
        r#"{"title":"視聴","locationInfos":[{"name":"東京"}]}"#,
    )
    .await;
    scene_erase(pool, stay).await;

    let (code, body) = preview(&app, stay).await;

    assert_eq!(code, StatusCode::OK, "{body}");
    let v = json_of(&body);
    assert_eq!(v["counts"]["stays"], 1);
    assert_eq!(v["counts"]["locations"], 70);
    assert_eq!(v["counts"]["archive_locations"], 3);
    assert_eq!(v["counts"]["archive_items"], 1);
    assert_eq!(v["counts"]["versions"], 20);
    assert_eq!(v["purged"], false);
}

/// 同じ原因で消えた記録を 1 件足す（`erase` の連鎖と同じ印と台帳の行）。
async fn mark_cascaded(pool: &PgPool, user: Uuid, cause: Uuid, source: &str, when: &str) -> Uuid {
    let id = scene_event(pool, user, source, when, r#"{"lat":35.68,"lon":139.76}"#).await;
    sqlx::query(
        "UPDATE core.event SET deleted_at = now(), deleted_by = 'user:cascade' WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.deletion_ledger (event_id, user_id, logical_source, action, cause_event_id, mark)
         VALUES ($1, $2, $3, 'erase', $4, 'user:cascade')",
    )
    .bind(id)
    .bind(user)
    .bind(source)
    .bind(cause)
    .execute(pool)
    .await
    .unwrap();
    id
}

// Scenario: 確認を求めても何も消えない
#[tokio::test]
async fn purge_api_preview_changes_nothing() {
    let (app, user) = (app().await, testdb::user());
    let (stay, locations) = scene_erased(&app.pool, user).await;
    let ids = [vec![stay], locations].concat();
    let before = raws(&app.pool, &ids).await;
    let ledger_before = ledger_rows(&app.pool, user).await;

    let (code, body) = preview(&app, stay).await;

    assert_eq!(code, StatusCode::OK, "{body}");
    assert_eq!(raws(&app.pool, &ids).await, before, "本文は確認の前と同じ");
    assert_eq!(ledger_rows(&app.pool, user).await, ledger_before);
}

// Scenario: 本文ごと消した滞在の確認は 0 件を返す
#[tokio::test]
async fn purge_api_preview_of_a_purged_stay_is_zero() {
    let (app, user) = (app().await, testdb::user());
    let (stay, _) = scene_erased(&app.pool, user).await;
    purge_now(&app.pool, stay).await;

    let (code, body) = preview(&app, stay).await;

    assert_eq!(code, StatusCode::OK, "{body}");
    let v = json_of(&body);
    assert_eq!(v["counts"], counts_json(&PurgeCounts::default()));
    assert_eq!(v["purged"], true);
}

// Scenario: 滞在でない記録の識別子の確認は断られる
#[tokio::test]
async fn purge_api_preview_refuses_a_record_that_is_not_a_stay() {
    let (app, user) = (app().await, testdb::user());
    let (_, locations) = scene_erased(&app.pool, user).await;

    let (code, _) = preview(&app, locations[0]).await;

    assert_eq!(code, StatusCode::NOT_FOUND);
}

// Scenario: 知らない識別子の確認は断られる
#[tokio::test]
async fn purge_api_preview_refuses_an_unknown_id() {
    let app = app().await;

    let (code, _) = preview(&app, Uuid::new_v4()).await;

    assert_eq!(code, StatusCode::NOT_FOUND);
}

// Scenario: 消えていない滞在の確認は断られる
#[tokio::test]
async fn purge_api_preview_refuses_a_stay_that_is_not_erased() {
    let (app, user) = (app().await, testdb::user());
    let (stay, _) = scene_basic(&app.pool, user).await;

    let (code, body) = preview(&app, stay).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
}

// Scenario: 資格情報の無い確認の求めは断られる
#[tokio::test]
async fn purge_api_preview_requires_credentials() {
    let (app, user) = (app().await, testdb::user());
    let (stay, _) = scene_erased(&app.pool, user).await;

    let (code, _) = send(
        &app,
        Method::GET,
        &format!("/stays/purge/preview?stay_id={stay}"),
        false,
        None,
    )
    .await;

    assert_eq!(code, StatusCode::UNAUTHORIZED);
}

// ------------------------------------------------------------------ 確認と消す口の件数

// Scenario: 確認の後に位置が届くと消さずに断られる
#[tokio::test]
async fn purge_api_new_point_after_preview_refuses_and_keeps_every_body() {
    let (app, user) = (app().await, testdb::user());
    let (stay, locations) = scene_erased(&app.pool, user).await;
    let (_, body) = preview(&app, stay).await;
    let shown: PurgeCounts = serde_json::from_value(json_of(&body)["counts"].clone()).unwrap();
    // 確認の後で、消えた滞在の中の時刻に位置が届き、同じ原因で消える
    let late = mark_cascaded(
        &app.pool,
        user,
        stay,
        "c01-location",
        "2026-10-01T10:45:00+09:00",
    )
    .await;
    let ids = [vec![stay, late], locations].concat();
    let before = raws(&app.pool, &ids).await;

    let (code, body) = purge_with(&app, stay, &shown).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    let v = json_of(&body);
    assert_eq!(v["reason"], "counts_changed");
    assert_eq!(
        v["counts"]["locations"],
        shown.locations + 1,
        "数え直した件数は位置が 1 件多い"
    );
    assert_eq!(raws(&app.pool, &ids).await, before, "どの記録の本文も残る");
}

// Scenario: 件数が一致すれば本文ごと消える
#[tokio::test]
async fn purge_api_matching_counts_empty_every_body() {
    let (app, user) = (app().await, testdb::user());
    let (stay, locations) = scene_erased(&app.pool, user).await;
    let (_, body) = preview(&app, stay).await;
    let shown: PurgeCounts = serde_json::from_value(json_of(&body)["counts"].clone()).unwrap();

    let (code, body) = purge_with(&app, stay, &shown).await;

    assert_eq!(code, StatusCode::OK, "{body}");
    assert_eq!(json_of(&body)["counts"], counts_json(&shown));
    assert!(emptied(&app.pool, stay).await);
    for id in locations {
        assert!(emptied(&app.pool, id).await, "位置の本文が残っている");
    }
}

// ------------------------------------------------------------------ 戻す口の断り

// Scenario: 本文を消した滞在は戻せない
#[tokio::test]
async fn purge_api_a_purged_stay_cannot_be_restored() {
    let (app, user) = (app().await, testdb::user());
    let (stay, locations) = scene_erased(&app.pool, user).await;
    purge_now(&app.pool, stay).await;

    let (code, body) = restore(&app, &[stay]).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    let marked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.event WHERE id = ANY($1) AND deleted_at IS NOT NULL",
    )
    .bind([vec![stay], locations].concat())
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(marked, 4, "削除の印は付いたまま");
}

// Scenario: 本文を消した記録を含む戻す求めは断られ、何も戻らない
#[tokio::test]
async fn purge_api_a_request_with_a_purged_record_restores_nothing() {
    let (app, user) = (app().await, testdb::user());
    let (purged, _) = scene_erased(&app.pool, user).await;
    purge_now(&app.pool, purged).await;
    let other = scene_stay(
        &app.pool,
        user,
        "2026-10-01T14:00:00+09:00",
        "2026-10-01T15:00:00+09:00",
    )
    .await;
    scene_erase(&app.pool, other).await;

    let (code, body) = restore(&app, &[purged, other]).await;

    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    let still: bool =
        sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM core.event WHERE id = $1")
            .bind(other)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(still, "本文を消していない滞在も消えたまま");
}

// Scenario: 本文ごと消した時間と境界で位置を共有しても隣の消した滞在は戻せる
#[tokio::test]
async fn purge_api_a_neighbour_sharing_a_boundary_point_can_be_restored() {
    let (app, user) = (app().await, testdb::user());
    let first = scene_stay(
        &app.pool,
        user,
        "2026-10-01T10:00:00+09:00",
        "2026-10-01T11:00:00+09:00",
    )
    .await;
    let neighbour = scene_stay(
        &app.pool,
        user,
        "2026-10-01T11:00:00+09:00",
        "2026-10-01T11:30:00+09:00",
    )
    .await;
    let shared = scene_location(&app.pool, user, "2026-10-01T11:00:00+09:00").await;
    scene_erase(&app.pool, first).await;
    scene_erase(&app.pool, neighbour).await;
    purge_now(&app.pool, first).await;

    let (code, body) = restore(&app, &[neighbour]).await;

    assert_eq!(code, StatusCode::OK, "{body}");
    let stay_back: bool =
        sqlx::query_scalar("SELECT deleted_at IS NULL FROM core.event WHERE id = $1")
            .bind(neighbour)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(stay_back, "隣の滞在は戻る");
    let (empty, still_deleted): (bool, bool) =
        sqlx::query_as("SELECT raw = '', deleted_at IS NOT NULL FROM core.event WHERE id = $1")
            .bind(shared)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(
        empty && still_deleted,
        "共有の位置は本文が空のまま消えたまま"
    );
}

// ------------------------------------------------------------------ 台帳は読み出しと書き出しに出ない

// Scenario: 消去の操作の台帳は記録の読み出しと書き出しに出ない
#[tokio::test]
async fn purge_api_the_erasure_ledger_is_not_in_the_reads_or_the_export() {
    let (app, user) = (app().await, testdb::user());
    let (stay, _) = scene_erased(&app.pool, user).await;
    let (_, body) = preview(&app, stay).await;
    let shown: PurgeCounts = serde_json::from_value(json_of(&body)["counts"].clone()).unwrap();
    let (_, body) = purge_with(&app, stay, &shown).await;
    let operation = json_of(&body)["operation_id"].as_str().unwrap().to_owned();

    let day = send(
        &app,
        Method::GET,
        &format!("/day?date=2026-10-01&user_id={user}"),
        true,
        None,
    )
    .await;
    let records = send(
        &app,
        Method::GET,
        &format!("/day/records?date=2026-10-01&user_id={user}"),
        true,
        None,
    )
    .await;
    let events = send(&app, Method::GET, "/events", true, None).await;

    for (name, (code, text)) in [
        ("1 日の並び", day),
        ("記録の一覧", records),
        ("全記録", events),
    ] {
        assert_eq!(code, StatusCode::OK, "{name}: {text}");
        assert!(!text.contains(&operation), "{name} に操作の識別子が出た");
        assert!(!text.contains("erasure_"), "{name} に消去の台帳が出た");
    }
    for table in ["erasure_ledger", "erasure_operation", "erasure_span"] {
        assert!(
            matches!(
                crate::export::class_of(table),
                Some(crate::export::Class::Exclude { .. })
            ),
            "{table} は書き出しの対象外"
        );
    }
}
