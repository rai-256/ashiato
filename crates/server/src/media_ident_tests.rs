// SPDX-License-Identifier: AGPL-3.0-only
//! 写真・画像・動画の見分け（ST09 tasks 3.2 / design D4 / C17 / Q7）を、本物の PostgreSQL に対して確かめる。
//!
//! 端末が送る形（外部識別子 = 内容ハッシュ。中身が変われば `previous_sha256` を持つ別の記録。
//! 消えた事実は `vanished:<sha256>:<volume>:<media_id>`）と同じ要求を取り込み口へ送る。
#![allow(clippy::unwrap_used)]

use axum::{extract::State, http::HeaderMap, Json};

use crate::testdb;
use crate::{ingest, App, IngestResult};

const TOKEN: &str = "test-token-0123456789abcdef";

async fn app() -> App {
    App::for_test(testdb::pool().await, TOKEN)
}

fn media(user: uuid::Uuid, sha: &str, raw: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut payload = serde_json::json!({"kind": "media", "sha256": sha, "media_id": 42});
    for (k, v) in extra.as_object().unwrap() {
        payload[k] = v.clone();
    }
    serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": user,
        "logical_source": "c01-photo", "external_id": sha,
        "device_id": "phone-01", "origin": "collected",
        "event_time": "2026-03-01T12:00:00Z", "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo", "schema_version": 1, "raw": raw, "payload": payload,
    })
}

async fn send(app: &App, items: Vec<serde_json::Value>) -> Vec<IngestResult> {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    let (_, Json(res)) = ingest(State(app.clone()), h, Json(serde_json::Value::Array(items)))
        .await
        .expect("取り込み口");
    assert!(res.iter().all(|r| r.accepted), "断られた: {res:?}");
    res
}

fn fresh_sha() -> String {
    format!(
        "{:x}{:x}",
        uuid::Uuid::new_v4().as_u128(),
        uuid::Uuid::new_v4().as_u128()
    )
}

async fn rows(app: &App, user: uuid::Uuid) -> Vec<(uuid::Uuid, String, String, serde_json::Value)> {
    sqlx::query_as(
        "SELECT id, external_id, raw, payload FROM core.event
          WHERE user_id = $1 AND deleted_at IS NULL ORDER BY external_id",
    )
    .bind(user)
    .fetch_all(&app.pool)
    .await
    .unwrap()
}

async fn versions(app: &App, id: uuid::Uuid) -> Vec<String> {
    sqlx::query_scalar("SELECT raw FROM core.event_version WHERE event_id = $1 ORDER BY version_no")
        .bind(id)
        .fetch_all(&app.pool)
        .await
        .unwrap()
}

/// Scenario: 同じ中身の写真が 2 つ保存されても記録は 1 件
#[tokio::test]
async fn st09_up_ident_same_hash_under_two_names_is_one_record() {
    let app = app().await;
    let u = testdb::user();
    let sha = fresh_sha();
    // 端末は同じハッシュの記録を 1 件だけ送るが、別の名前の分が届いても畳まれる
    let a = send(
        &app,
        vec![media(u, &sha, r#"{"n":"a.jpg"}"#, serde_json::json!({}))],
    )
    .await;
    let b = send(
        &app,
        vec![media(
            u,
            &sha,
            r#"{"n":"a.jpg"}"#,
            serde_json::json!({"media_id": 43}),
        )],
    )
    .await;
    assert_eq!(a[0].id, b[0].id, "同じ中身が別の行になった");
    assert_eq!(rows(&app, u).await.len(), 1);
}

/// Scenario: 原文だけ変わった写真は 1 件のまま前の版が残る
#[tokio::test]
async fn st09_up_ident_moved_photo_keeps_one_row_and_old_version() {
    let app = app().await;
    let u = testdb::user();
    let sha = fresh_sha();
    let first = send(
        &app,
        vec![media(
            u,
            &sha,
            r#"{"path":"DCIM/Camera/"}"#,
            serde_json::json!({}),
        )],
    )
    .await;
    let moved = send(
        &app,
        vec![media(
            u,
            &sha,
            r#"{"path":"Pictures/Trip/"}"#,
            serde_json::json!({}),
        )],
    )
    .await;
    assert_eq!(first[0].id, moved[0].id, "更新なのに別の行になった");
    let got = rows(&app, u).await;
    assert_eq!(got.len(), 1, "動かしただけで行が増えた");
    assert_eq!(
        got[0].2, r#"{"path":"Pictures/Trip/"}"#,
        "新しい保存先になっていない"
    );
    assert_eq!(
        versions(&app, got[0].0).await,
        vec![r#"{"path":"DCIM/Camera/"}"#.to_string()]
    );
}

/// Scenario: 中身が変わった写真は新しい記録になり前の記録も残る
#[tokio::test]
async fn st09_up_ident_edited_photo_is_a_new_record_and_old_stays() {
    let app = app().await;
    let u = testdb::user();
    let (old, new) = (fresh_sha(), fresh_sha());
    send(
        &app,
        vec![media(u, &old, r#"{"v":1}"#, serde_json::json!({}))],
    )
    .await;
    send(
        &app,
        vec![media(
            u,
            &new,
            r#"{"v":2}"#,
            serde_json::json!({"previous_sha256": old}),
        )],
    )
    .await;
    let got = rows(&app, u).await;
    assert_eq!(
        got.len(),
        2,
        "前の記録が読み出しから消えた、または新しい記録ができていない"
    );
    let after = got.iter().find(|r| r.1 == new).expect("新しい記録");
    assert_eq!(after.3["previous_sha256"], old.as_str());
    assert_eq!(after.3["media_id"], 42);
    let before = got.iter().find(|r| r.1 == old).expect("前の記録");
    assert_eq!(before.2, r#"{"v":1}"#, "前の記録が書き換わった");
    assert!(versions(&app, before.0).await.is_empty());
}

/// Scenario: 消えた事実を作っても写真の記録は前の版にならない
#[tokio::test]
async fn st09_up_ident_vanished_fact_does_not_touch_the_photo_record() {
    let app = app().await;
    let u = testdb::user();
    let sha = fresh_sha();
    send(
        &app,
        vec![media(u, &sha, r#"{"v":1}"#, serde_json::json!({}))],
    )
    .await;
    let ext = format!("vanished:{sha}:external_primary:42");
    let mut gone = media(
        u,
        &sha,
        r#"{"vanished":true}"#,
        serde_json::json!({"kind": "vanished"}),
    );
    gone["external_id"] = serde_json::json!(ext);
    send(&app, vec![gone]).await;
    let got = rows(&app, u).await;
    assert_eq!(got.len(), 2);
    let photo = got.iter().find(|r| r.1 == sha).expect("写真の記録");
    assert_eq!(photo.2, r#"{"v":1}"#, "写真の記録が書き換わった");
    assert!(
        versions(&app, photo.0).await.is_empty(),
        "写真の記録が前の版を持った"
    );
    assert!(got.iter().any(|r| r.1 == ext));
}
