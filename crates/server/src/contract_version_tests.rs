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

// ---- 知らない欄 `unknown_field` と 1 件の `contract_version`（Task 3 / D2・D4）

/// 1 件に最上位の鍵を足す。
fn with(mut item: serde_json::Value, key: &str, value: serde_json::Value) -> serde_json::Value {
    item[key] = value;
    item
}

fn first_of(array: &serde_json::Value) -> serde_json::Value {
    array.as_array().unwrap()[0].clone()
}

/// 結果の配列の `error` を並べる（受け付けたものは null）。
fn errors_of(body: &str) -> Vec<serde_json::Value> {
    let v: serde_json::Value = serde_json::from_str(body).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|r| r["error"].clone())
        .collect()
}

fn media_item(user: uuid::Uuid, source: &str, tag: &str) -> serde_json::Value {
    let sha = sha_of(&unique_bytes(tag));
    record(
        user,
        source,
        &sha,
        "a.jpg",
        "2026-10-01T03:00:00Z",
        serde_json::json!({}),
    )
}

// Scenario: 知らない欄を持つ記録は unknown_field で断られ、ほかの記録は入る
#[tokio::test]
async fn st50_unknown_field_ingest() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let source = testdb::source(&app.pool, "st50-unk-ingest", 21_600).await;
    let user = testdb::user();
    let items = vec![
        with(
            media_item(user, &source, "unk-1"),
            "mood_score",
            serde_json::json!(3),
        ),
        media_item(user, &source, "unk-2"),
        media_item(user, &source, "unk-3"),
    ];
    let (status, body) = send(
        &app,
        Method::POST,
        "/ingest",
        None,
        Some(serde_json::Value::Array(items)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let results: serde_json::Value = serde_json::from_str(&body).unwrap();
    let results = results.as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["accepted"], false);
    assert_eq!(results[0]["error"], "unknown_field");
    assert_eq!(results[1]["accepted"], true);
    assert_eq!(results[2]["accepted"], true);
    assert_eq!(
        events_of(&app, &source).await,
        2,
        "知らない欄の 1 件は入らない"
    );
}

// Scenario: 生存信号・破棄の報告・原本の欠けの知らない欄も unknown_field で断られる
#[tokio::test]
async fn st50_unknown_field_other_types() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let source = testdb::source(&app.pool, "st50-unk-other", 21_600).await;
    let user = testdb::user();
    let sha = sha_of(&unique_bytes("st50-unk-missing"));
    let extra = |v: serde_json::Value| {
        serde_json::Value::Array(vec![with(first_of(&v), "mood_score", serde_json::json!(3))])
    };
    let cases = [
        ("/heartbeat", extra(heartbeat_item(&source, user))),
        (
            "/drops",
            extra(serde_json::json!([drop_item(&source, user)])),
        ),
        ("/media/missing", extra(missing_item(&source, user, &sha))),
    ];
    for (uri, body) in cases {
        let (status, text) = send(&app, Method::POST, uri, None, Some(body), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: 1 件も受け付けない");
        assert_eq!(
            errors_of(&text),
            vec![serde_json::json!("unknown_field")],
            "{uri}"
        );
    }
    assert_eq!(count_of(&app, "heartbeat", user).await, 0);
    assert_eq!(count_of(&app, "drop_report", user).await, 0);
    assert_eq!(count_of(&app, "media_missing", user).await, 0);
}

// Scenario: 知らない欄の判定は S-01 が知る欄で行う
#[tokio::test]
async fn st50_unknown_field_by_known_fields() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-unk-known", 21_600).await;
    let user = testdb::user();
    let mut item = media_item(user, &source, "unk-known");
    item["end_time"] = serde_json::json!("2026-10-01T04:00:00Z");
    item["end_tz_offset_min"] = serde_json::json!(540);
    item["end_tz_id"] = serde_json::json!("Asia/Tokyo");
    let (status, body) = send(
        &app,
        Method::POST,
        "/ingest",
        Some("1"),
        Some(serde_json::json!([item])),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(errors_of(&body), vec![serde_json::Value::Null]);
    let end: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT end_time FROM core.event WHERE logical_source = $1")
            .bind(&source)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(end.unwrap().to_rfc3339(), "2026-10-01T04:00:00+00:00");
}

// Scenario: payload の中の欄は知らない欄として断られない
#[tokio::test]
async fn st50_unknown_field_payload_inner() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let source = testdb::source(&app.pool, "st50-unk-payload", 21_600).await;
    let user = testdb::user();
    let sha = sha_of(&unique_bytes("unk-payload"));
    let item = record(
        user,
        &source,
        &sha,
        "a.jpg",
        "2026-10-01T03:00:00Z",
        serde_json::json!({ "mood_score": 3 }),
    );
    let (status, body) = send(
        &app,
        Method::POST,
        "/ingest",
        None,
        Some(serde_json::json!([item])),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_ne!(errors_of(&body)[0], "unknown_field");
    assert_eq!(events_of(&app, &source).await, 1);
}

// Scenario: 画面のログインで届く知らない欄は今どおり読み飛ばされる
#[tokio::test]
async fn st50_unknown_field_web_session() {
    let password = random_password();
    let (app, _) = tee_app(&password).await;
    let cookie = login(&app, &password).await;
    let source = testdb::source(&app.pool, "st50-unk-web", 21_600).await;
    let user = testdb::user();
    let item = with(
        media_item(user, &source, "unk-web"),
        "mood_score",
        serde_json::json!(3),
    );
    let req = Request::builder()
        .method(Method::POST)
        .uri("/ingest")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::COOKIE,
            format!("{}={cookie}", web_session::COOKIE_NAME),
        )
        .body(Body::from(serde_json::json!([item]).to_string()))
        .unwrap();
    let res = router(app.clone()).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_string(res).await;
    assert_ne!(errors_of(&body)[0], "unknown_field");
    assert_eq!(
        events_of(&app, &source).await,
        1,
        "今どおり読み飛ばして入る"
    );
}

// 知らない欄は `malformed` にしない（携帯が恒久として捨てる種別）。型に `deny_unknown_fields` を付けない
#[test]
fn st50_unknown_field_not_malformed() {
    let user = uuid::Uuid::new_v4();
    let hb = first_of(&heartbeat_item("s", user));
    let dr = drop_item("s", user);
    let ms = first_of(&missing_item("s", user, &sha_of(b"x")));
    let ig = media_item(user, "s", "nm");
    for (name, ok) in [
        (
            "heartbeat",
            serde_json::from_value::<heartbeat::HeartbeatRequest>(with(hb, "x", 1.into())).is_ok(),
        ),
        (
            "drops",
            serde_json::from_value::<drops::DropReportRequest>(with(dr, "x", 1.into())).is_ok(),
        ),
        (
            "media_missing",
            serde_json::from_value::<media_missing::MediaMissingRequest>(with(ms, "x", 1.into()))
                .is_ok(),
        ),
        (
            "ingest",
            serde_json::from_value::<ingest::IngestRequest>(with(ig, "x", 1.into())).is_ok(),
        ),
    ] {
        assert!(
            ok,
            "{name}: 知らない欄で解釈が失敗する（deny_unknown_fields）"
        );
    }
}

// 型の知る欄は schema から作る（手で並べない）。4 つの型が `contract_version` を知る
#[test]
fn st50_unknown_field_known_from_schema() {
    for known in [
        contract_version::known_fields::<ingest::IngestRequest>(),
        contract_version::known_fields::<heartbeat::HeartbeatRequest>(),
        contract_version::known_fields::<drops::DropReportRequest>(),
        contract_version::known_fields::<media_missing::MediaMissingRequest>(),
    ] {
        assert!(known.iter().any(|f| f == "contract_version"), "{known:?}");
        assert!(known.iter().any(|f| f == "id"), "{known:?}");
        assert!(!known.iter().any(|f| f == "mood_score"));
    }
}

// Scenario: S-01 より新しい版の生存信号・破棄の報告・原本の欠けの 1 件は contract_unsupported で断られる
#[tokio::test]
async fn st50_item_version_other_types() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let source = testdb::source(&app.pool, "st50-item-other", 21_600).await;
    let user = testdb::user();
    let sha = sha_of(&unique_bytes("st50-item-missing"));
    let pair = |a: serde_json::Value, b: serde_json::Value| {
        serde_json::Value::Array(vec![
            with(a, "contract_version", 2.into()),
            with(b, "contract_version", 1.into()),
        ])
    };
    let hb = heartbeat_item(&source, user);
    // 原本の欠けは画像のソース名しか受け付けない
    let ms = missing_item("c01-photo", user, &sha);
    let cases = [
        (
            "/heartbeat",
            pair(first_of(&hb), first_of(&heartbeat_item(&source, user))),
        ),
        (
            "/drops",
            pair(drop_item(&source, user), drop_item(&source, user)),
        ),
        (
            "/media/missing",
            pair(
                first_of(&ms),
                first_of(&missing_item("c01-photo", user, &sha)),
            ),
        ),
    ];
    for (uri, body) in cases {
        let (status, text) = send(&app, Method::POST, uri, Some("1"), Some(body), None).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {text}");
        assert_eq!(
            errors_of(&text),
            vec![
                serde_json::json!("contract_unsupported"),
                serde_json::Value::Null
            ],
            "{uri}"
        );
    }
    assert_eq!(count_of(&app, "heartbeat", user).await, 1);
    assert_eq!(count_of(&app, "drop_report", user).await, 1);
    assert_eq!(count_of(&app, "media_missing", user).await, 1);
}

// 正の整数でない 1 件の版も contract_unsupported（記録の口）。版の欄が無い・null は版 1
#[tokio::test]
async fn st50_item_version_ingest_not_positive() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let source = testdb::source(&app.pool, "st50-item-bad", 21_600).await;
    let user = testdb::user();
    let bads = [
        serde_json::json!(0),
        serde_json::json!(-1),
        serde_json::json!("1"),
        serde_json::json!(1.5),
        serde_json::json!(2),
    ];
    let mut items: Vec<serde_json::Value> = bads
        .iter()
        .enumerate()
        .map(|(i, v)| {
            with(
                media_item(user, &source, &format!("bad-{i}")),
                "contract_version",
                v.clone(),
            )
        })
        .collect();
    items.push(with(
        media_item(user, &source, "null"),
        "contract_version",
        serde_json::Value::Null,
    ));
    items.push(media_item(user, &source, "absent"));
    let (_, text) = send(
        &app,
        Method::POST,
        "/ingest",
        None,
        Some(serde_json::Value::Array(items)),
        None,
    )
    .await;
    let mut want = vec![serde_json::json!("contract_unsupported"); 5];
    want.extend([serde_json::Value::Null, serde_json::Value::Null]);
    assert_eq!(errors_of(&text), want);
    assert_eq!(events_of(&app, &source).await, 2);
}

// 口の検査を通らずに格納の本体へ届いた、列（integer）に収まらない版は、黙って版 1 にせず断る（final review R12）
#[tokio::test]
async fn st50_item_version_out_of_column_range_is_refused_not_v1() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let source = testdb::source(&app.pool, "st50-item-range", 21_600).await;
    let user = testdb::user();
    let item = with(
        media_item(user, &source, "range"),
        "contract_version",
        serde_json::json!(i64::from(i32::MAX) + 1),
    );
    let req: IngestRequest = serde_json::from_value(item).unwrap();
    let res = ingest_request(&app, req, Writer::Collector, Settle::InTx)
        .await
        .unwrap();
    assert!(!res.accepted);
    assert_eq!(
        serde_json::to_value(res.error).unwrap(),
        serde_json::json!("contract_unsupported")
    );
    assert_eq!(events_of(&app, &source).await, 0, "版 1 として入った");
}

// ---- 記録の契約の版・移行・錠（Task 4 / D1・D10・D19（仮）・D20）-----------------------------------

const ST50_NAME: &str = "202610300030_contract_version";
const ST50_DOWN: &str = include_str!("../../../migrations/202610300030_contract_version.down.sql");

fn st50_up() -> &'static str {
    crate::MIGRATIONS
        .iter()
        .find(|(name, _)| *name == ST50_NAME)
        .expect("ST50 の移行が MIGRATIONS に無い")
        .1
}

fn id_of(item: &serde_json::Value) -> uuid::Uuid {
    item["id"].as_str().unwrap().parse().unwrap()
}

async fn st50_event_version(pool: &sqlx::PgPool, id: uuid::Uuid) -> Option<i32> {
    sqlx::query_scalar("SELECT contract_version FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn st50_send_ingest(
    app: &App,
    header_value: Option<&str>,
    items: Vec<serde_json::Value>,
) -> serde_json::Value {
    let (status, text) = send(
        app,
        Method::POST,
        "/ingest",
        header_value,
        Some(serde_json::Value::Array(items)),
        None,
    )
    .await;
    assert!(
        status.is_success() || status == StatusCode::BAD_REQUEST,
        "{status}: {text}"
    );
    serde_json::from_str(&text).unwrap()
}

// Scenario: 記録は 1 件ごとの契約の版を持つ
#[tokio::test]
async fn st50_record_version_per_item() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-rec-per", 21_600).await;
    let user = testdb::user();
    let one = with(
        media_item(user, &source, "per-1"),
        "contract_version",
        1.into(),
    );
    let two = with(
        media_item(user, &source, "per-2"),
        "contract_version",
        2.into(),
    );
    let res = st50_send_ingest(&app, Some("2"), vec![one.clone(), two.clone()]).await;
    assert_eq!(res[0]["accepted"], true, "{res}");
    assert_eq!(res[1]["accepted"], true, "{res}");
    assert_eq!(st50_event_version(&app.pool, id_of(&one)).await, Some(1));
    assert_eq!(st50_event_version(&app.pool, id_of(&two)).await, Some(2));
}

// Scenario: 契約の版の欄が無い記録は版 1 になる
#[tokio::test]
async fn st50_record_version_default_one() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let source = testdb::source(&app.pool, "st50-rec-def", 21_600).await;
    let item = media_item(testdb::user(), &source, "def");
    assert!(item.get("contract_version").is_none());
    let res = st50_send_ingest(&app, None, vec![item.clone()]).await;
    assert_eq!(res[0]["accepted"], true, "{res}");
    assert_eq!(st50_event_version(&app.pool, id_of(&item)).await, Some(1));
}

// Scenario: S-01 より新しい版の 1 件は contract_unsupported で断られ、ほかは入る
#[tokio::test]
async fn st50_record_version_too_new_item() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let source = testdb::source(&app.pool, "st50-rec-new", 21_600).await;
    let user = testdb::user();
    let new = with(
        media_item(user, &source, "new-2"),
        "contract_version",
        2.into(),
    );
    let old = with(
        media_item(user, &source, "new-1"),
        "contract_version",
        1.into(),
    );
    let res = st50_send_ingest(&app, Some("1"), vec![new.clone(), old.clone()]).await;
    assert_eq!(res[0]["accepted"], false, "{res}");
    assert_eq!(res[0]["error"], "contract_unsupported", "{res}");
    assert_eq!(res[1]["accepted"], true, "{res}");
    assert_eq!(events_of(&app, &source).await, 1);
    assert_eq!(st50_event_version(&app.pool, id_of(&old)).await, Some(1));
}

// Scenario: 収集アプリ以外が書いた記録の契約の版は空
#[tokio::test]
async fn st50_record_version_null_for_others() {
    let password = random_password();
    let (app, _) = tee_app(&password).await;
    let user = testdb::user();

    // 画面のログイン。欄に版を書いても空のまま
    let cookie = login(&app, &password).await;
    let source = testdb::source(&app.pool, "st50-rec-web", 21_600).await;
    let web = with(
        media_item(user, &source, "web"),
        "contract_version",
        1.into(),
    );
    let req = Request::builder()
        .method(Method::POST)
        .uri("/ingest")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::COOKIE,
            format!("{}={cookie}", web_session::COOKIE_NAME),
        )
        .body(Body::from(serde_json::json!([web.clone()]).to_string()))
        .unwrap();
    let res = router(app.clone()).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(st50_event_version(&app.pool, id_of(&web)).await, None);

    // S-01 の中の書庫の取り込み（`store_one`）
    let archive_source = testdb::source(&app.pool, "st50-rec-store", 21_600).await;
    let archived = media_item(user, &archive_source, "store");
    let request: IngestRequest = serde_json::from_value(archived.clone()).unwrap();
    crate::store_one(&app.pool, request).await.unwrap();
    assert_eq!(st50_event_version(&app.pool, id_of(&archived)).await, None);

    // 滞在の作り直し。位置は収集アプリの資格情報で入れる（版 1）
    let stay_user = testdb::user();
    let start: chrono::DateTime<chrono::Utc> = "2026-05-20T01:00:00Z".parse().unwrap();
    let points: Vec<serde_json::Value> = (0..=30)
        .map(|i| {
            serde_json::json!({
                "id": uuid::Uuid::new_v4(), "user_id": stay_user, "logical_source": "c01-location",
                "device_id": "test-dev", "origin": "collected",
                "event_time": start + chrono::Duration::minutes(i),
                "tz_offset_min": 540, "tz_id": "Asia/Tokyo", "schema_version": 1,
                "raw": format!(r#"{{"lat":35.0,"lon":139.0,"acc_m":12,"n":{i}}}"#),
                "payload": {"lat": 35.0, "lon": 139.0, "acc_m": 12, "n": i},
            })
        })
        .collect();
    let res = st50_send_ingest(&app, None, points).await;
    assert_eq!(res[0]["accepted"], true, "{res}");
    crate::stay_store::rebuild_day(&app.pool, stay_user, testdb::date("2026-05-20"))
        .await
        .unwrap();
    let stays: Vec<Option<i32>> = sqlx::query_scalar(
        "SELECT contract_version FROM core.event
          WHERE user_id = $1 AND logical_source = 's01-stay' AND origin = 'derived'",
    )
    .bind(stay_user)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert!(!stays.is_empty(), "滞在が作られていない");
    assert!(stays.iter().all(Option::is_none), "{stays:?}");
    let located: Vec<Option<i32>> = sqlx::query_scalar(
        "SELECT contract_version FROM core.event WHERE user_id = $1 AND logical_source = 'c01-location'",
    )
    .bind(stay_user)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert!(
        located.iter().all(|v| *v == Some(1)),
        "収集アプリの位置は版 1"
    );
}

// Scenario: 契約の版だけが違う同じ記録は 1 件に畳まれる
#[tokio::test]
async fn st50_record_version_dedup() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-rec-dedup", 21_600).await;
    let user = testdb::user();
    let sha = sha_of(&unique_bytes("st50-dedup"));
    let make = |v: i64| {
        with(
            record(
                user,
                &source,
                &sha,
                "a.jpg",
                "2026-10-01T03:00:00Z",
                serde_json::json!({}),
            ),
            "contract_version",
            v.into(),
        )
    };
    let first = make(1);
    let second = make(2);
    let res = st50_send_ingest(&app, Some("2"), vec![first.clone(), second]).await;
    assert_eq!(res[0]["duplicate"], false, "{res}");
    assert_eq!(res[1]["duplicate"], true, "{res}");
    assert_eq!(events_of(&app, &source).await, 1);
    assert_eq!(st50_event_version(&app.pool, id_of(&first)).await, Some(1));
}

// Scenario: 外部サービス由来の更新で契約の版が動き、前の値は前の版に残る
#[tokio::test]
async fn st50_record_version_external_update() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source_of_kind(&app.pool, "st50-rec-upd", "record").await;
    let user = testdb::user();
    let make = |raw: &str, v: i64, updated: &str| {
        serde_json::json!({
            "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": source,
            "external_id": "ext-1", "device_id": "test-dev", "origin": "collected",
            "event_time": "2026-10-01T03:00:00Z", "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
            "schema_version": 1, "raw": raw, "payload": {"v": raw},
            "source_updated_at": updated, "contract_version": v,
        })
    };
    let first = make("one", 1, "2026-10-01T03:00:00Z");
    let res = st50_send_ingest(&app, Some("2"), vec![first.clone()]).await;
    assert_eq!(res[0]["accepted"], true, "{res}");
    let second = make("two", 2, "2026-10-02T03:00:00Z");
    let res = st50_send_ingest(&app, Some("2"), vec![second]).await;
    assert_eq!(res[0]["accepted"], true, "{res}");
    let id = id_of(&first);
    assert_eq!(st50_event_version(&app.pool, id).await, Some(2));
    let previous: Vec<Option<i32>> =
        sqlx::query_scalar("SELECT contract_version FROM core.event_version WHERE event_id = $1")
            .bind(id)
            .fetch_all(&app.pool)
            .await
            .unwrap();
    assert_eq!(previous, vec![Some(1)]);
}

// Scenario: 契約の版はスキーマ版と別に動く
#[tokio::test]
async fn st50_record_version_schema_independent() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-rec-schema", 21_600).await;
    let item = with(
        media_item(testdb::user(), &source, "schema"),
        "contract_version",
        2.into(),
    );
    assert_eq!(item["schema_version"], 1);
    let res = st50_send_ingest(&app, Some("2"), vec![item.clone()]).await;
    assert_eq!(res[0]["accepted"], true, "{res}");
    let (schema, contract): (i32, Option<i32>) =
        sqlx::query_as("SELECT schema_version, contract_version FROM core.event WHERE id = $1")
            .bind(id_of(&item))
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!((schema, contract), (1, Some(2)));
}

// Scenario: 要求の見出しの版は記録の契約の版を決めない
#[tokio::test]
async fn st50_record_version_not_from_header() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_contract_version(2);
    let source = testdb::source(&app.pool, "st50-rec-hdr", 21_600).await;
    let item = media_item(testdb::user(), &source, "hdr");
    let res = st50_send_ingest(&app, Some("2"), vec![item.clone()]).await;
    assert_eq!(res[0]["accepted"], true, "{res}");
    assert_eq!(st50_event_version(&app.pool, id_of(&item)).await, Some(1));
}

// ---- 移行と錠（使い捨ての DB）

/// 使い捨ての DB に版を当てる。`upto` があれば、その名前の版の手前まで。
async fn st50_db(upto: Option<&str>) -> (sqlx::PgPool, impl std::future::Future<Output = ()>) {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    for (label, sql) in crate::MIGRATIONS {
        if Some(*label) == upto {
            break;
        }
        sqlx::raw_sql(sql)
            .execute(&fresh)
            .await
            .unwrap_or_else(|e| panic!("{label}: {e}"));
    }
    (fresh, drop)
}

/// 契約の版の列を使わずに行を置く（ST50 の前の形でも置ける）。
async fn st50_put_event(pool: &sqlx::PgPool, source: &str, origin: &str) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ($1, $2, $3, $4, $5, '2026-09-08T02:00:00Z', 540, 'Asia/Tokyo',
                 1, $6, '{\"v\":1}', '{\"v\":1}')",
    )
    .bind(id)
    .bind(uuid::Uuid::nil())
    .bind(source)
    .bind((origin == "collected").then_some("dev"))
    .bind(origin)
    .bind(format!("hash-{id}"))
    .execute(pool)
    .await
    .unwrap();
    id
}

/// 更新前の版を前の版の表へ写す（契約の版の列を使わない。ST50 の前の形でも置ける）。
async fn st50_put_version(pool: &sqlx::PgPool, id: uuid::Uuid) {
    sqlx::query(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
         SELECT id, user_id, logical_source, 1, event_time, content_hash, raw, payload
           FROM core.event WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

async fn st50_locks_armed(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar(
        "SELECT (SELECT tgenabled = 'O' FROM pg_trigger WHERE tgname = 'event_version_immutable')
            AND (SELECT tgenabled = 'O' FROM pg_trigger WHERE tgname = 'event_requires_version')",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

// Scenario: ST50 より前に収集アプリから入った記録は版 1 になる
#[tokio::test]
async fn st50_migration_backfill() {
    let (fresh, drop) = st50_db(Some(ST50_NAME)).await;
    let location = st50_put_event(&fresh, "c01-location", "collected").await;
    let window = st50_put_event(&fresh, "c02-window", "collected").await;
    let feeling = st50_put_event(&fresh, "s01-feeling", "authored").await;
    for id in [location, window] {
        st50_put_version(&fresh, id).await;
    }
    sqlx::raw_sql(st50_up()).execute(&fresh).await.unwrap();
    let events: Vec<(uuid::Uuid, Option<i32>)> =
        sqlx::query_as("SELECT id, contract_version FROM core.event")
            .fetch_all(&fresh)
            .await
            .unwrap();
    let versions: Vec<(uuid::Uuid, Option<i32>)> =
        sqlx::query_as("SELECT event_id, contract_version FROM core.event_version")
            .fetch_all(&fresh)
            .await
            .unwrap();
    let armed = st50_locks_armed(&fresh).await;
    // 2 回目も落ちない
    let twice = sqlx::raw_sql(st50_up()).execute(&fresh).await;
    drop.await;
    let of = |rows: &[(uuid::Uuid, Option<i32>)], id| rows.iter().find(|r| r.0 == id).map(|r| r.1);
    assert_eq!(of(&events, location), Some(Some(1)));
    assert_eq!(of(&events, window), Some(Some(1)));
    assert_eq!(of(&events, feeling), Some(None));
    assert_eq!(of(&versions, location), Some(Some(1)));
    assert_eq!(of(&versions, window), Some(Some(1)));
    assert!(armed, "埋め戻しで外した錠と門が張り直されていない");
    twice.unwrap();
}

// Scenario: 前の版の契約の版は書き換えられない
#[tokio::test]
async fn st50_lock_version_row() {
    let (fresh, drop) = st50_db(None).await;
    let id = st50_put_event(&fresh, "c01-location", "collected").await;
    st50_put_version(&fresh, id).await;
    let err = sqlx::query("UPDATE core.event_version SET contract_version = 2 WHERE event_id = $1")
        .bind(id)
        .execute(&fresh)
        .await;
    let kept: Option<i32> =
        sqlx::query_scalar("SELECT contract_version FROM core.event_version WHERE event_id = $1")
            .bind(id)
            .fetch_one(&fresh)
            .await
            .unwrap();
    drop.await;
    let message = err
        .expect_err("前の版の契約の版が書き換えられた")
        .to_string();
    assert!(message.contains("履歴は追記のみ"), "{message}");
    assert_eq!(kept, None);
}

// Scenario: 収集した記録の契約の版だけを前の版なしで書き換えると拒まれる
#[tokio::test]
async fn st50_lock_collected_rewrite() {
    let (fresh, drop) = st50_db(None).await;
    let id = st50_put_event(&fresh, "c01-location", "collected").await;
    let bare = sqlx::query("UPDATE core.event SET contract_version = 2 WHERE id = $1")
        .bind(id)
        .execute(&fresh)
        .await;
    let after_bare = st50_event_version(&fresh, id).await;

    // 更新前の版（契約の版も同じ）を同じまとまりに積めば通る
    let mut tx = fresh.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload, contract_version)
         SELECT id, user_id, logical_source, 1, event_time, content_hash, raw, payload, contract_version
           FROM core.event WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET contract_version = 2 WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let with_version = tx.commit().await;

    // 前の版の契約の版が更新前と違えば通らない
    let mut tx = fresh.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload, contract_version)
         SELECT id, user_id, logical_source, 2, event_time, content_hash, raw, payload, 7
           FROM core.event WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET contract_version = 3 WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let wrong_version = tx.commit().await;
    let after = st50_event_version(&fresh, id).await;
    drop.await;

    assert!(bare.is_err(), "前の版なしの書き換えが通った");
    assert_eq!(after_bare, None);
    with_version.expect("前の版を積めば通る");
    assert!(
        wrong_version.is_err(),
        "更新前と違う契約の版の前の版で通った"
    );
    assert_eq!(after, Some(2));
}

async fn st50_version_columns(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name IN ('event', 'event_version')
            AND column_name = 'contract_version'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn st50_migration_down_up() {
    let (fresh, drop) = st50_db(None).await;
    sqlx::raw_sql(crate::migration_ledger::BOOTSTRAP)
        .execute(&fresh)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO migration_ledger.applied (name, sha256, build_version, build_commit, before_ledger)
         VALUES ($1, 'x', 'v', 'c', false)",
    )
    .bind(ST50_NAME)
    .execute(&fresh)
    .await
    .unwrap();
    let id = st50_put_event(&fresh, "c01-location", "collected").await;
    st50_put_version(&fresh, id).await;

    sqlx::raw_sql(ST50_DOWN).execute(&fresh).await.unwrap();
    let columns = st50_version_columns(&fresh).await;
    let recorded: i64 =
        sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied WHERE name = $1")
            .bind(ST50_NAME)
            .fetch_one(&fresh)
            .await
            .unwrap();
    // 2 回目の戻し・進め直し・もう一度
    let down_twice = sqlx::raw_sql(ST50_DOWN).execute(&fresh).await;
    let up = sqlx::raw_sql(st50_up()).execute(&fresh).await;
    let up_twice = sqlx::raw_sql(st50_up()).execute(&fresh).await;
    let columns_after = st50_version_columns(&fresh).await;
    let armed = st50_locks_armed(&fresh).await;
    drop.await;

    assert_eq!(columns, 0, "列が残っている");
    assert_eq!(recorded, 0, "版の記録の行が残っている");
    down_twice.unwrap();
    up.unwrap();
    up_twice.unwrap();
    assert_eq!(columns_after, 2, "進め直しで列が戻っていない");
    assert!(armed);
}

// ===== Task 5: 配った版（版 1）の封筒の例を受け付け続ける（D8）=====

const V1_INGEST_C01: &str = include_str!("../tests/fixtures/contract/v1/ingest-c01-location.json");
const V1_INGEST_C02: &str = include_str!("../tests/fixtures/contract/v1/ingest-c02-window.json");
const V1_INGEST_END: &str = include_str!("../tests/fixtures/contract/v1/ingest-end-time.json");
const V1_HEARTBEAT_C01: &str = include_str!("../tests/fixtures/contract/v1/heartbeat-c01.json");
const V1_HEARTBEAT_C02: &str = include_str!("../tests/fixtures/contract/v1/heartbeat-c02.json");
const V1_DROPS: &str = include_str!("../tests/fixtures/contract/v1/drops.json");
const V1_MEDIA_MISSING: &str = include_str!("../tests/fixtures/contract/v1/media-missing.json");
const V1_WIRE: &str = include_str!("../tests/fixtures/contract/v1/wire.json");

/// 例を読み、送るたびに別の 1 件になるよう `id`・持ち主・ソースだけ差し替える（形は例のまま）。
/// 原本の欠けは画像のソース（登録簿にある本物の名前）でなければ断られるので、ソースは例のまま。
fn v1_fresh(text: &str, user: uuid::Uuid, source: &str) -> serde_json::Value {
    let mut item: serde_json::Value = serde_json::from_str(text).unwrap();
    item["id"] = serde_json::json!(uuid::Uuid::new_v4());
    item["user_id"] = serde_json::json!(user);
    if item.get("sha256").is_none() {
        item["logical_source"] = serde_json::json!(source);
    }
    item
}

// Scenario: 版 1 の封筒の例は受け付けられる
#[tokio::test]
async fn st50_released_v1_accepted() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let user = testdb::user();
    let cases = [
        ("/ingest", V1_INGEST_C01),
        ("/ingest", V1_INGEST_C02),
        ("/ingest", V1_INGEST_END),
        ("/heartbeat", V1_HEARTBEAT_C01),
        ("/heartbeat", V1_HEARTBEAT_C02),
        ("/drops", V1_DROPS),
        ("/media/missing", V1_MEDIA_MISSING),
    ];
    for (uri, text) in cases {
        for header_value in [None, Some("1")] {
            let source = testdb::source(&app.pool, "st50-rel-v1", 21_600).await;
            let item = v1_fresh(text, user, &source);
            let (status, body) = send(
                &app,
                Method::POST,
                uri,
                header_value,
                Some(serde_json::Value::Array(vec![item])),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{uri} {header_value:?}: {body}");
            let results: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(
                results[0]["accepted"], true,
                "{uri} {header_value:?}: {body}"
            );
            assert_eq!(
                results[0]["error"],
                serde_json::Value::Null,
                "{uri}: {body}"
            );
        }
    }
}

// Scenario: ST41 の終わりの時刻を持つ記録は版 1 として受け付けられる
#[tokio::test]
async fn st50_released_v1_end_time() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let user = testdb::user();
    let source = testdb::source(&app.pool, "st50-rel-end", 21_600).await;
    let item = v1_fresh(V1_INGEST_END, user, &source);
    let (status, body) = send(
        &app,
        Method::POST,
        "/ingest",
        Some("1"),
        Some(serde_json::Value::Array(vec![item])),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(errors_of(&body), vec![serde_json::Value::Null]);
    let row: (
        Option<chrono::DateTime<chrono::Utc>>,
        Option<i32>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT end_time, end_tz_offset_min, end_tz_id FROM core.event WHERE logical_source = $1",
    )
    .bind(&source)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(row.0.unwrap().to_rfc3339(), "2026-09-13T01:12:03.456+00:00");
    assert_eq!(row.1, Some(540));
    assert_eq!(row.2.as_deref(), Some("Asia/Tokyo"));
}

// Scenario: S-01 は版 1 の口の形の例に従う
#[tokio::test]
async fn st50_released_v1_wire() {
    let wire: serde_json::Value = serde_json::from_str(V1_WIRE).unwrap();
    let text = |key: &str| wire[key].as_str().unwrap().to_string();
    let app = App::for_test(testdb::pool().await, TOKEN);
    let answer_key = text("contract_answer_key");

    // 問い合わせの口: 道・答えの鍵
    let (status, body) = send(&app, Method::GET, &text("contract_path"), None, None, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let answer: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        answer.as_object().unwrap().keys().collect::<Vec<_>>(),
        vec![&answer_key]
    );
    assert!(answer[&answer_key].as_u64().unwrap() >= 1);

    // 見出しの名前（定数は例と同じ名前）
    assert_eq!(CONTRACT_HEADER, text("header_name"));

    // 新しすぎる版の断り: 例の見出しの名前で送る
    let too_new = (answer[&answer_key].as_u64().unwrap() + 1).to_string();
    let req = Request::builder()
        .method(Method::POST)
        .uri("/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(text("header_name"), too_new)
        .body(Body::from("[]"))
        .unwrap();
    let res = router(app).oneshot(req).await.unwrap();
    assert_eq!(
        res.status().as_u16() as u64,
        wire["unsupported_status"].as_u64().unwrap()
    );
    let refused: serde_json::Value = serde_json::from_str(&body_string(res).await).unwrap();
    assert_eq!(
        refused[text("unsupported_body_error_key")],
        text("unsupported_body_error_value")
    );
    assert_eq!(
        refused[text("unsupported_body_version_key")],
        answer[&answer_key]
    );
    assert_eq!(refused.as_object().unwrap().len(), 2);
}
