// SPDX-License-Identifier: AGPL-3.0-only
//! 画面のログイン（`/session`）を、router を通して本物の DB に対して確かめる（ST28 / design D1〜D3 / D17 / D18）。
//! 合言葉は試験の中で乱数から作る。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, Response};
use tower::ServiceExt as _;

use super::*;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

fn random_password() -> String {
    format!("pw-{}", uuid::Uuid::new_v4())
}

async fn app_with(password: &str, days: u32) -> App {
    App::for_test(testdb::pool().await, TOKEN).with_web_login(password, days)
}

async fn send(app: &App, req: Request<Body>) -> Response<Body> {
    router(app.clone()).oneshot(req).await.unwrap()
}

fn login_req(body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/session")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

fn login_body(password: &str) -> String {
    serde_json::json!({ "password": password }).to_string()
}

fn with_cookie(method: Method, uri: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, format!("{COOKIE}={cookie}"))
        .body(Body::empty())
        .unwrap()
}

const COOKIE: &str = web_session::COOKIE_NAME;

fn set_cookie(res: &Response<Body>) -> Option<String> {
    res.headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_owned())
}

/// `Set-Cookie` から印の値を取り出す。
fn token_of(set_cookie: &str) -> String {
    set_cookie
        .strip_prefix(&format!("{COOKIE}="))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

fn max_age(set_cookie: &str) -> u64 {
    set_cookie
        .split(';')
        .find_map(|p| p.trim().strip_prefix("Max-Age="))
        .unwrap()
        .parse()
        .unwrap()
}

/// ログインして印の値を返す。
async fn login(app: &App, password: &str) -> String {
    let res = send(app, login_req(&login_body(password))).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    token_of(&set_cookie(&res).unwrap())
}

async fn read_events(app: &App, cookie: &str) -> Response<Body> {
    send(app, with_cookie(Method::GET, "/events", cookie)).await
}

/// Scenario: 違う合言葉のログインの求めは断られ、印は発行されない
#[tokio::test]
async fn web_session_endpoint_wrong_password_is_refused() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let res = send(&app, login_req(&login_body(&random_password()))).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(set_cookie(&res).is_none(), "印が発行された");
}

/// Scenario: 合言葉を付けないログインの求めは断られる
#[tokio::test]
async fn web_session_endpoint_missing_password_is_refused() {
    let app = app_with(&random_password(), 0).await;
    for body in ["{}", "", "not json", r#"{"password":null}"#] {
        let res = send(&app, login_req(body)).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "本文: {body}");
        assert!(set_cookie(&res).is_none(), "印が発行された（本文: {body}）");
    }
}

/// Scenario: API の合言葉では画面にログインできない
#[tokio::test]
async fn web_session_endpoint_api_token_is_not_a_login() {
    let app = app_with(&random_password(), 0).await;
    let res = send(&app, login_req(&login_body(TOKEN))).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(set_cookie(&res).is_none());
}

/// Scenario: ログインの印は暗号化された接続でだけ送られる
#[tokio::test]
async fn web_session_endpoint_cookie_is_secure() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let res = send(&app, login_req(&login_body(&pw))).await;
    let sc = set_cookie(&res).unwrap();
    assert!(sc.split("; ").any(|a| a == "Secure"), "{sc}");
    assert!(sc.split("; ").any(|a| a == "HttpOnly"));
    assert!(!sc.contains("Domain"));
}

/// Scenario: ログインの印は別のサイトから始まった求めには付かない
#[tokio::test]
async fn web_session_endpoint_cookie_is_same_site_strict() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let res = send(&app, login_req(&login_body(&pw))).await;
    let sc = set_cookie(&res).unwrap();
    assert!(sc.split("; ").any(|a| a == "SameSite=Strict"), "{sc}");
}

/// Scenario: ログアウトした印を持ち出しても使えない
#[tokio::test]
async fn web_session_endpoint_logged_out_cookie_is_dead() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let token = login(&app, &pw).await;
    assert_eq!(read_events(&app, &token).await.status(), StatusCode::OK);

    let out = send(&app, with_cookie(Method::DELETE, "/session", &token)).await;
    assert_eq!(out.status(), StatusCode::NO_CONTENT);
    assert_eq!(max_age(&set_cookie(&out).unwrap()), 0);
    // 印が無くても 204
    let none = send(
        &app,
        Request::builder()
            .method(Method::DELETE)
            .uri("/session")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(none.status(), StatusCode::NO_CONTENT);

    let again = read_events(&app, &token).await;
    assert_eq!(again.status(), StatusCode::UNAUTHORIZED);
    let state = send(&app, with_cookie(Method::GET, "/session", &token)).await;
    assert_eq!(state.status(), StatusCode::UNAUTHORIZED);
}

/// Scenario: 失敗を重ねたログインは一時的に断られる
#[tokio::test]
async fn web_session_endpoint_failures_are_throttled() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    for _ in 0..10 {
        let res = send(&app, login_req(&login_body(&random_password()))).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    let res = send(&app, login_req(&login_body(&pw))).await;
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(set_cookie(&res).is_none(), "印が発行された");
}

#[tokio::test]
async fn web_session_endpoint_failure_waits_and_state_reports_caller() {
    let pw = random_password();
    let mut app = app_with(&pw, 0).await;
    app.login.failure_delay = std::time::Duration::from_millis(300);
    let t = std::time::Instant::now();
    send(&app, login_req(&login_body("x"))).await;
    assert!(t.elapsed() >= std::time::Duration::from_millis(300));

    let token = login(&app, &pw).await;
    let by_cookie = send(&app, with_cookie(Method::GET, "/session", &token)).await;
    let body = axum::body::to_bytes(by_cookie.into_body(), 4096)
        .await
        .unwrap();
    assert_eq!(&body[..], br#"{"credential":"web_session"}"#);
    let by_bearer = send(
        &app,
        Request::builder()
            .uri("/session")
            .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let body = axum::body::to_bytes(by_bearer.into_body(), 4096)
        .await
        .unwrap();
    assert_eq!(&body[..], br#"{"credential":"api_token"}"#);
}

/// Scenario: API の合言葉での読み書きは変わらない
#[tokio::test]
async fn web_session_endpoint_api_token_reads_and_writes() {
    let app = app_with(&random_password(), 0).await;
    let source = testdb::source(&app.pool, "wsapi", 21_600).await;
    let id = uuid::Uuid::new_v4();
    let body = serde_json::json!([{
        "id": id, "user_id": testdb::user(), "logical_source": source, "external_id": null,
        "device_id": "test-dev", "origin": "collected", "event_time": "2026-03-01T03:00:00Z",
        "tz_offset_min": 540, "tz_id": "Asia/Tokyo", "schema_version": 1,
        "raw": r#"{"a":1}"#, "payload": {},
    }]);
    let bearer = format!("Bearer {TOKEN}");
    let post = Request::builder()
        .method(Method::POST)
        .uri("/ingest")
        .header(header::AUTHORIZATION, &bearer)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = send(&app, post).await;
    assert_eq!(res.status(), StatusCode::OK);
    let get = Request::builder()
        .uri("/events")
        .header(header::AUTHORIZATION, &bearer)
        .body(Body::empty())
        .unwrap();
    let res = send(&app, get).await;
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 << 20)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains(&id.to_string()));
}

/// Scenario: 既定では日が経ってもログインは切れない
#[tokio::test]
async fn web_session_lifetime_default_never_expires() {
    let pw = random_password();
    let now = chrono::Utc::now();
    let app = app_with(&pw, 0).await.at(now);
    let token = login(&app, &pw).await;
    let later = app.clone().at(now + chrono::Duration::days(400));
    assert_eq!(read_events(&later, &token).await.status(), StatusCode::OK);
}

/// Scenario: ログインの印はブラウザを閉じても残り、使うたびに延びる
#[tokio::test]
async fn web_session_lifetime_cookie_persists_and_is_refreshed() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let res = send(&app, login_req(&login_body(&pw))).await;
    let login_cookie = set_cookie(&res).unwrap();
    assert!(max_age(&login_cookie) >= 86_400, "{login_cookie}");
    let token = token_of(&login_cookie);

    let read = read_events(&app, &token).await;
    assert_eq!(read.status(), StatusCode::OK);
    let read_cookie = set_cookie(&read).expect("読み出しの応答に印の出し直しが無い");
    assert!(max_age(&read_cookie) >= 86_400, "{read_cookie}");
    assert_eq!(token_of(&read_cookie), token);
}

/// Scenario: 画面の合言葉を変えると、それまでのログインはすべて使えなくなる
#[tokio::test]
async fn web_session_lifetime_password_change_revokes_all() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let a = login(&app, &pw).await;
    let b = login(&app, &pw).await;
    assert_eq!(read_events(&app, &a).await.status(), StatusCode::OK);

    let restarted = app.clone().with_web_login(&random_password(), 0);
    assert_eq!(
        read_events(&restarted, &a).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        read_events(&restarted, &b).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

/// Scenario: 期限を設定したときは、過ぎた印は使えない
#[tokio::test]
async fn web_session_lifetime_configured_expiry_applies() {
    let pw = random_password();
    let now = chrono::Utc::now();
    let app = app_with(&pw, 30).await.at(now);
    let token = login(&app, &pw).await;
    let inside = app.clone().at(now + chrono::Duration::days(29));
    assert_eq!(read_events(&inside, &token).await.status(), StatusCode::OK);
    let outside = app.clone().at(now + chrono::Duration::days(31));
    assert_eq!(
        read_events(&outside, &token).await.status(),
        StatusCode::UNAUTHORIZED
    );
}
