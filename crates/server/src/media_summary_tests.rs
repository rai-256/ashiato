// SPDX-License-Identifier: AGPL-3.0-only
//! 量と原本の進みの読み出し（ST09 tasks 7.2 / design D12）を、本物の PostgreSQL と置き場に対して確かめる。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use tower::ServiceExt as _;

use crate::media::MediaConfig;
use crate::media_tests::{app_with, photo, put, record, sha_of, store, unique_bytes, Tmp};
use crate::{router, testdb, App};

const TOKEN: &str = "test-token-0123456789abcdef";
const TAKEN: &str = "2026-10-17T05:30:12Z";
const MB: i64 = 1_000_000;

async fn get(app: &App, user: uuid::Uuid, auth: bool) -> (StatusCode, serde_json::Value) {
    let mut b = Request::builder()
        .method(Method::GET)
        .uri(format!("/media/summary?user_id={user}"));
    if auth {
        b = b.header("authorization", format!("Bearer {TOKEN}"));
    }
    let res = router(app.clone())
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

fn source<'a>(body: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    body["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["logical_source"] == name)
        .unwrap()
}

/// 画像の記録 1 件（サイズつき）を格納して内容ハッシュを返す。
async fn sized(
    app: &App,
    user: uuid::Uuid,
    src: &str,
    size: i64,
    extra: serde_json::Value,
) -> String {
    let sha = sha_of(&unique_bytes("sized"));
    let mut extra = extra;
    extra["size_bytes"] = serde_json::json!(size);
    store(app, vec![record(user, src, &sha, "n.jpg", TAKEN, extra)]).await;
    sha
}

async fn miss(app: &App, user: uuid::Uuid, sha: &str, reason: &str) {
    sqlx::query(
        "INSERT INTO core.media_missing (id, user_id, device_id, logical_source, sha256, reason, noticed_at)
         VALUES ($1,$2,'d','c01-photo',$3,$4, now())",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(sha)
    .bind(reason)
    .execute(&app.pool)
    .await
    .unwrap();
}

/// Scenario: 記録の件数とバイトがソースごとに読める
#[tokio::test]
async fn st09_sum_records_and_bytes_per_source() {
    let tmp = Tmp::new();
    let app = app_with(tmp.made()).await;
    let user = testdb::user();
    sized(&app, user, "c01-photo", 2 * MB, serde_json::json!({})).await;
    sized(&app, user, "c01-photo", 3 * MB, serde_json::json!({})).await;
    sized(&app, user, "c01-screenshot", MB, serde_json::json!({})).await;
    let (code, body) = get(&app, user, true).await;
    assert_eq!(code, StatusCode::OK);
    let photo = source(&body, "c01-photo");
    assert_eq!(
        (photo["records"].as_i64(), photo["record_bytes"].as_i64()),
        (Some(2), Some(5 * MB))
    );
    let shot = source(&body, "c01-screenshot");
    assert_eq!(
        (shot["records"].as_i64(), shot["record_bytes"].as_i64()),
        (Some(1), Some(MB))
    );
}

/// Scenario: 置いた原本と待っている原本の件数が読める
#[tokio::test]
async fn st09_sum_stored_and_waiting_originals() {
    let tmp = Tmp::new();
    let app = app_with(tmp.made()).await;
    let user = testdb::user();
    for tag in ["a", "b"] {
        let bytes = unique_bytes(tag);
        let sha = photo(&app, user, &bytes, &format!("{tag}.jpg"), TAKEN).await;
        let (code, _) = put(&app, &sha, user, &bytes).await;
        assert_eq!(code, StatusCode::CREATED);
    }
    photo(&app, user, &unique_bytes("c"), "c.jpg", TAKEN).await;
    let (_, body) = get(&app, user, true).await;
    let photo = source(&body, "c01-photo");
    assert_eq!(photo["records"], 3);
    assert_eq!(photo["originals_stored"]["count"], 2);
    assert_eq!(photo["originals_waiting"], 1);
}

/// Scenario: 原本の欠けが理由ごとに読める
#[tokio::test]
async fn st09_sum_missing_by_reason() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let user = testdb::user();
    let a = sized(&app, user, "c01-photo", MB, serde_json::json!({})).await;
    let b = sized(&app, user, "c01-photo", MB, serde_json::json!({})).await;
    sized(&app, user, "c01-photo", MB, serde_json::json!({})).await;
    miss(&app, user, &a, "vanished").await;
    miss(&app, user, &b, "vanished").await;
    let (_, body) = get(&app, user, true).await;
    let photo = source(&body, "c01-photo");
    assert_eq!(
        photo["missing"],
        serde_json::json!([{"reason": "vanished", "count": 2}])
    );
    // 欠けた原本は「待っている」に数えない
    assert_eq!(photo["originals_waiting"], 1);
}

/// Scenario: 量の読み出しに記録の値が出ない
#[tokio::test]
async fn st09_sum_exposes_no_record_values() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let user = testdb::user();
    let sha = sha_of(&unique_bytes("private"));
    store(
        &app,
        vec![record(
            user,
            "c01-photo",
            &sha,
            "secret-name.jpg",
            TAKEN,
            serde_json::json!({"size_bytes": MB, "latitude": 35.123456, "longitude": 139.654321}),
        )],
    )
    .await;
    let (_, body) = get(&app, user, true).await;
    let text = body.to_string();
    for needle in [
        "secret-name",
        "35.12",
        "139.65",
        "latitude",
        "longitude",
        "display_name",
        &sha,
    ] {
        assert!(!text.contains(needle), "{needle} が出ている: {text}");
    }
}

/// Scenario: 書けなかった失敗は起動し直すまで読み出しに出る
#[tokio::test]
async fn st09_sum_storage_error_lasts_until_restart() {
    let tmp = Tmp::new();
    let app = app_with(tmp.root()).await; // 置き場が無い
    let user = testdb::user();
    let bytes = unique_bytes("unwritable");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    let (code, _) = put(&app, &sha, user, &bytes).await;
    assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE);
    let (_, first) = get(&app, user, true).await;
    assert_eq!(first["last_storage_error"]["kind"], "dir_missing");
    assert!(first["last_storage_error"]["at"].is_string());
    // 起動し直す = 状態を持たない新しい設定
    let restarted =
        App::for_test(testdb::pool().await, TOKEN).with_media(MediaConfig::with_dir(tmp.root()));
    let (_, second) = get(&restarted, user, true).await;
    assert!(second["last_storage_error"].is_null());
}

/// Scenario: 資格情報の無い量の読み出しは断られる
#[tokio::test]
async fn st09_sum_without_credentials_is_refused() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let user = testdb::user();
    sized(&app, user, "c01-photo", MB, serde_json::json!({})).await;
    let (code, body) = get(&app, user, false).await;
    assert_eq!(code, StatusCode::UNAUTHORIZED);
    assert!(body["sources"].is_null());
}
