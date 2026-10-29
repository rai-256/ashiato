// SPDX-License-Identifier: AGPL-3.0-only
//! 契約の版と確かめる口 `GET /contract` を、router を通して本物の DB で確かめる（ST50 / design D4 / D15 / D23）。
//! 読み出しの記録は「この試験の書く口に届いた行」で数える（共有の表に他の試験の行が混ざるため）。
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use tower::ServiceExt as _;

use super::*;
use crate::access_log::{AccessEntry, AccessFuture, AccessSink, PgAccessSink};
use crate::contract_version::{requested_version, Unreadable, CONTRACT_HEADER};
use crate::satellite_authn_tests::{
    body_string, get_with, login, random_password, seed_credential, seed_satellite,
};
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

#[derive(Clone)]
struct Tee {
    real: Arc<PgAccessSink>,
    seen: Arc<Mutex<Vec<AccessEntry>>>,
}

impl AccessSink for Tee {
    fn record(&self, entry: AccessEntry) -> AccessFuture {
        self.seen.lock().unwrap().push(entry.clone());
        self.real.record(entry)
    }
}

async fn tee_app(password: &str) -> (App, Arc<Mutex<Vec<AccessEntry>>>) {
    let pool = testdb::pool().await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let tee = Tee {
        real: Arc::new(PgAccessSink(pool.clone())),
        seen: seen.clone(),
    };
    let app = App::for_test(pool, TOKEN)
        .with_web_login(password, 0)
        .with_access_sink(tee);
    (app, seen)
}

fn get_bare() -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri("/contract")
        .body(Body::empty())
        .unwrap()
}

// Scenario: 問い合わせの口は受け付ける一番新しい契約の版を答える
#[tokio::test]
async fn st50_contract_answers_version() {
    let (app, _) = tee_app(&random_password()).await;
    let app = app.with_contract_version(3);
    let res = router(app)
        .oneshot(get_with("/contract", TOKEN, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body_string(res).await).unwrap();
    assert_eq!(body, serde_json::json!({ "contract_version": 3 }));
}

#[tokio::test]
async fn st50_contract_default_is_the_constant() {
    let (app, _) = tee_app(&random_password()).await;
    let res = router(app)
        .oneshot(get_with("/contract", TOKEN, None))
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_str(&body_string(res).await).unwrap();
    assert_eq!(body["contract_version"], contract_version::CONTRACT_VERSION);
    assert_eq!(contract_version::CONTRACT_VERSION, 1);
}

// Scenario: 合言葉の無い問い合わせは断られる
#[tokio::test]
async fn st50_contract_requires_token() {
    let (app, _) = tee_app(&random_password()).await;
    let res = router(app.clone()).oneshot(get_bare()).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(!body_string(res).await.contains("contract_version"));
    let res = router(app)
        .oneshot(get_with("/contract", "wrong-token-0123456789abcdef", None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(!body_string(res).await.contains("contract_version"));
}

// Scenario: 画面のログインで問い合わせると 403 で断られる
#[tokio::test]
async fn st50_contract_web_session_403() {
    let password = random_password();
    let (app, _) = tee_app(&password).await;
    let cookie = login(&app, &password).await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/contract")
        .header(
            header::COOKIE,
            format!("{}={cookie}", web_session::COOKIE_NAME),
        )
        .body(Body::empty())
        .unwrap();
    let res = router(app.clone()).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert!(!body_string(res).await.contains("contract_version"));

    // 衛星の資格情報でも 403
    let (satellite, identifier) = seed_satellite(&app.pool).await;
    let (_, value) = seed_credential(&app.pool, satellite, &identifier, chrono::Utc::now()).await;
    let res = router(app)
        .oneshot(get_with("/contract", &value, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert!(!body_string(res).await.contains("contract_version"));
}

// D23（仮）: 合言葉が認められた問い合わせは取り込みの口に入れず、今の規則のまま 1 回 1 行を残す
#[tokio::test]
async fn st50_contract_access_logged() {
    let (app, seen) = tee_app(&random_password()).await;
    let res = router(app)
        .oneshot(get_with("/contract", TOKEN, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "1 回 1 行");
    let e = &seen[0];
    assert_eq!(
        (
            e.credential,
            e.route.as_str(),
            e.method.as_str(),
            e.outcome,
            e.status
        ),
        ("api_token", "/contract", "GET", "ok", 200)
    );
}

#[test]
fn st50_contract_header_reader() {
    let read = |v: Option<&str>| {
        let mut h = HeaderMap::new();
        if let Some(v) = v {
            h.insert(CONTRACT_HEADER, HeaderValue::from_str(v).unwrap());
        }
        requested_version(&h)
    };
    assert_eq!(read(None), Ok(1));
    assert_eq!(read(Some("1")), Ok(1));
    assert_eq!(read(Some("12")), Ok(12));
    for bad in ["0", "-1", "+2", "1.5", "", " 2", "2 ", "abc", "99999999999"] {
        assert_eq!(read(Some(bad)), Err(Unreadable), "{bad:?}");
    }
}

// ---- 見出しの検査と 426（6 つの口。Task 2 / D4・D16）-------------------------------------------

use crate::media::MediaConfig;
use crate::media_tests::{record, sha_of, unique_bytes};

/// 見出し（あれば）と合言葉を付けた要求を送り、状態符号と本文を返す。
async fn send(
    app: &App,
    method: Method,
    uri: &str,
    header_value: Option<&str>,
    body: Option<serde_json::Value>,
    bytes: Option<Vec<u8>>,
) -> (StatusCode, String) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"));
    if let Some(v) = header_value {
        b = b.header(CONTRACT_HEADER, v);
    }
    let req = match (body, bytes) {
        (Some(j), _) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string())),
        (None, Some(x)) => b.body(Body::from(x)),
        (None, None) => b.body(Body::empty()),
    }
    .unwrap();
    let res = router(app.clone()).oneshot(req).await.unwrap();
    (res.status(), body_string(res).await)
}

fn unsupported(supported: u32) -> String {
    serde_json::json!({ "error": "contract_unsupported", "contract_version": supported })
        .to_string()
}

async fn events_of(app: &App, source: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM core.event WHERE logical_source = $1")
        .bind(source)
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

fn heartbeat_item(source: &str, user: uuid::Uuid) -> serde_json::Value {
    serde_json::json!([{
        "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": source,
        "device_id": "test-dev", "emitted_at": "2026-05-01T00:00:00Z", "capturable": true,
        "blockers": [], "attempts": 6, "successes": 6,
        "raw": format!(r#"{{"n":"{}"}}"#, uuid::Uuid::new_v4()),
    }])
}

fn drop_item(source: &str, user: uuid::Uuid) -> serde_json::Value {
    let body = serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": source,
        "device_id": "test-dev", "reason": "age", "created_at": "2026-09-14T00:00:00Z",
        "range_start": "2026-09-14T00:00:00Z", "range_end": "2026-09-14T01:00:00Z",
        "count": 1, "hourly": [{"hour": "2026-09-14T00:00:00Z", "count": 1}],
    });
    let mut out = body.clone();
    out["raw"] = serde_json::json!(body.to_string());
    out
}

fn missing_item(source: &str, user: uuid::Uuid, sha: &str) -> serde_json::Value {
    serde_json::json!([{
        "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": source,
        "device_id": "phone-01", "sha256": sha, "reason": "vanished",
        "noticed_at": "2026-10-01T03:00:00Z",
    }])
}

async fn count_of(app: &App, table: &str, user: uuid::Uuid) -> i64 {
    sqlx::query_scalar(&format!(
        "SELECT count(*) FROM core.{table} WHERE user_id = $1"
    ))
    .bind(user)
    .fetch_one(&app.pool)
    .await
    .unwrap()
}

// Scenario: S-01 より新しい版を名乗る取り込みは 426 で断られ、何も入らない
#[tokio::test]
async fn st50_header_ingest_426() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-hdr-ingest", 21_600).await;
    let user = testdb::user();
    let sha = sha_of(&unique_bytes("st50-ingest"));
    let item = record(
        user,
        &source,
        &sha,
        "a.jpg",
        "2026-10-01T03:00:00Z",
        serde_json::json!({}),
    );
    let (status, body) = send(
        &app,
        Method::POST,
        "/ingest",
        Some("3"),
        Some(serde_json::json!([item])),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
    assert_eq!(body, unsupported(2));
    assert_eq!(events_of(&app, &source).await, 0, "何も格納しない");
}

// Scenario: 生存信号・破棄の報告・原本の欠けも新しすぎる版では 426 で断られる
#[tokio::test]
async fn st50_header_other_mouths_426() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-hdr-other", 21_600).await;
    let user = testdb::user();
    let sha = sha_of(&unique_bytes("st50-missing"));
    let cases = [
        ("/heartbeat", heartbeat_item(&source, user)),
        ("/drops", serde_json::json!([drop_item(&source, user)])),
        ("/media/missing", missing_item(&source, user, &sha)),
    ];
    for (uri, body) in cases {
        let (status, text) = send(&app, Method::POST, uri, Some("3"), Some(body), None).await;
        assert_eq!(status, StatusCode::UPGRADE_REQUIRED, "{uri}");
        assert_eq!(text, unsupported(2), "{uri}");
    }
    assert_eq!(count_of(&app, "heartbeat", user).await, 0);
    assert_eq!(count_of(&app, "drop_report", user).await, 0);
    assert_eq!(count_of(&app, "media_missing", user).await, 0);
}

// Scenario: 新しすぎる版を名乗る原本は置かれない
#[tokio::test]
async fn st50_header_original_426() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().join("media");
    std::fs::create_dir_all(&root).unwrap();
    let app = App::for_test(testdb::pool().await, TOKEN)
        .with_contract_version(2)
        .with_media(MediaConfig::with_dir(root.clone()));
    let bytes = unique_bytes("st50-original");
    let sha = sha_of(&bytes);
    let uri = format!("/media/originals/{sha}?user_id={}", testdb::user());
    let (status, body) = send(&app, Method::PUT, &uri, Some("3"), None, Some(bytes)).await;
    assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
    assert_eq!(body, unsupported(2));
    let placed = walk_files(&root);
    assert!(placed.is_empty(), "原本を置かない: {placed:?}");
}

fn walk_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(walk_files(&p));
        } else {
            out.push(p);
        }
    }
    out
}

// Scenario: 新しすぎる版を名乗る原本の有無の問い合わせは 426 だけを返す
#[tokio::test]
async fn st50_header_original_head_426() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().join("media");
    std::fs::create_dir_all(&root).unwrap();
    let app = App::for_test(testdb::pool().await, TOKEN)
        .with_contract_version(2)
        .with_media(MediaConfig::with_dir(root));
    let sha = sha_of(&unique_bytes("st50-head"));
    let uri = format!("/media/originals/{sha}?user_id={}", testdb::user());
    let (status, body) = send(&app, Method::HEAD, &uri, Some("3"), None, None).await;
    assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
    assert!(body.is_empty(), "HEAD は本文を持たない: {body:?}");
}

// Scenario: 見出しの無い要求は版 1 として今どおり受け付けられる
#[tokio::test]
async fn st50_header_absent_is_v1() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-hdr-absent", 21_600).await;
    let user = testdb::user();
    let (status, _) = send(
        &app,
        Method::POST,
        "/heartbeat",
        None,
        Some(heartbeat_item(&source, user)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(count_of(&app, "heartbeat", user).await, 1);
}

// Scenario: S-01 以下の版を名乗る要求は受け付けられる
#[tokio::test]
async fn st50_header_at_or_below_accepted() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-hdr-below", 21_600).await;
    let user = testdb::user();
    for v in ["1", "2"] {
        let (status, _) = send(
            &app,
            Method::POST,
            "/heartbeat",
            Some(v),
            Some(heartbeat_item(&source, user)),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "版 {v}");
    }
    assert_eq!(count_of(&app, "heartbeat", user).await, 2);
}

// Scenario: 読めない版の見出しは 426 で断られる
#[tokio::test]
async fn st50_header_unreadable_426() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-hdr-bad", 21_600).await;
    let user = testdb::user();
    for bad in ["abc", "0", "-1", ""] {
        let (status, body) = send(
            &app,
            Method::POST,
            "/heartbeat",
            Some(bad),
            Some(heartbeat_item(&source, user)),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UPGRADE_REQUIRED, "{bad:?}");
        assert_eq!(body, unsupported(2), "{bad:?}");
    }
    assert_eq!(count_of(&app, "heartbeat", user).await, 0);
}

// Scenario: 画面のログインの要求には版の検査が掛からない
#[tokio::test]
async fn st50_header_web_session_exempt() {
    let password = random_password();
    let (app, _) = tee_app(&password).await;
    let app = app.with_contract_version(2);
    let cookie = login(&app, &password).await;
    let source = testdb::source(&app.pool, "st50-hdr-web", 21_600).await;
    let user = testdb::user();
    let req = Request::builder()
        .method(Method::POST)
        .uri("/heartbeat")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::COOKIE,
            format!("{}={cookie}", web_session::COOKIE_NAME),
        )
        .header(CONTRACT_HEADER, "99")
        .body(Body::from(heartbeat_item(&source, user).to_string()))
        .unwrap();
    let res = router(app.clone()).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "版の検査は掛からない");
    assert_eq!(count_of(&app, "heartbeat", user).await, 1);
}
