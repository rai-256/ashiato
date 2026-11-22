#![allow(clippy::unwrap_used)]

use super::access_log::{AccessEntry, AccessFuture, AccessSink};
use super::testdb;
use super::web_session::{self, SharedSecrets};
use super::{router, App, Authn};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tower::ServiceExt as _;

#[test]
fn st59_swap_bundle_contains_the_three_shared_values() {
    let bundle = SharedSecrets::from_password(
        "api-token-for-test".to_owned(),
        "session-key-for-test",
        "old-password-for-test",
    );
    assert_eq!(bundle.api_token(), "api-token-for-test");
    assert!(!bundle.tag().is_empty());
}

#[tokio::test]
// Scenario: 変えている最中に前の合言葉で照合が通ったログインの印は、変えた後は使えない
async fn st59_swap_login_cookie_uses_the_old_tag_and_is_revoked_after_swap() {
    let app = App::for_test(testdb::pool().await, "old-api-token");
    let old_password = app.web_password_for_test().to_owned();
    let succeeded = app.login.check_succeeded_for_test();
    let request = tokio::spawn(router(app.clone()).oneshot(login_request(&old_password)));
    succeeded.await;
    app.replace_secrets_for_test("new-api-token", "new-web-password-012345");
    let response = request.await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let old_cookie = cookie_value(&response);
    assert_eq!(
        read_events(&app, &old_cookie).await.status(),
        StatusCode::UNAUTHORIZED
    );

    let new_login = router(app.clone())
        .oneshot(login_request("new-web-password-012345"))
        .await
        .unwrap();
    assert_eq!(new_login.status(), StatusCode::NO_CONTENT);
    let new_cookie = cookie_value(&new_login);
    assert_eq!(
        read_events(&app, &new_cookie).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
// Scenario: 変える前に始まった求めの応答は、変えた画面の新しい印を上書きしない
async fn st59_swap_refresh_does_not_set_cookie_after_replacement() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let armed = Arc::new(AtomicBool::new(false));
    let app =
        App::for_test(testdb::pool().await, "old-api-token").with_access_sink(BlockingEvents {
            entered: entered.clone(),
            release: release.clone(),
            armed: armed.clone(),
        });
    let old_cookie = login(&app, app.web_password_for_test()).await;
    armed.store(true, Ordering::Release);
    let request = tokio::spawn(router(app.clone()).oneshot(read_request(&old_cookie)));
    entered.notified().await;
    app.replace_secrets_for_test("new-api-token", "new-web-password-012345");
    release.notify_one();
    let response = request.await.unwrap().unwrap();
    armed.store(false, Ordering::Release);
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(header::SET_COOKIE).is_none());

    let new_login = router(app.clone())
        .oneshot(login_request("new-web-password-012345"))
        .await
        .unwrap();
    let new_cookie = cookie_value(&new_login);
    assert_eq!(
        read_events(&app, &new_cookie).await.status(),
        StatusCode::OK
    );
}

#[test]
fn st59_swap_limiter_is_not_part_of_the_bundle() {
    let bundle = SharedSecrets::from_password("token".into(), "session-key", "password");
    assert_eq!(bundle.api_token(), "token");
    assert!(!bundle.tag().is_empty());
}

#[tokio::test]
async fn st59_swap_replacement_keeps_limiter_and_replaces_api_token() {
    let app = App::for_test(testdb::pool().await, "old-api-token");
    let old_password = app.web_password_for_test().to_owned();
    for _ in 0..10 {
        let response = router(app.clone())
            .oneshot(login_request("wrong-web-password-012345"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    app.replace_secrets_for_test("new-api-token", &old_password);
    let throttled = router(app.clone())
        .oneshot(login_request(&old_password))
        .await
        .unwrap();
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);

    let old = bearer("old-api-token");
    let new = bearer("new-api-token");
    assert_eq!(super::decided(&app, &old).await, Authn::Decided(None));
    assert_eq!(
        super::decided(&app, &new).await,
        Authn::Decided(Some(web_session::Caller::ApiToken))
    );
}

#[test]
fn st59_settings_replaces_one_key_and_preserves_the_other_entries() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    std::fs::write(
        &path,
        br#"{"API_TOKEN":"old-api-token","WEB_PASSWORD_VERIFIER":"old-verifier","BIND":"127.0.0.1:18700"}"#,
    )
    .unwrap();
    super::service::replace_setting_at(&path, "API_TOKEN", "new-api-token").unwrap();

    let settings = super::service::load_settings(&path).unwrap();
    assert_eq!(
        settings.0.get("API_TOKEN"),
        Some(&"new-api-token".to_owned())
    );
    assert_eq!(
        settings.0.get("WEB_PASSWORD_VERIFIER"),
        Some(&"old-verifier".to_owned())
    );
    assert_eq!(settings.0.get("BIND"), Some(&"127.0.0.1:18700".to_owned()));
}

#[test]
fn st59_settings_writes_a_file_loadable_by_load_settings() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    std::fs::write(&path, br#"{"API_TOKEN":"old-api-token"}"#).unwrap();

    super::service::replace_setting_at(&path, "WEB_PASSWORD_VERIFIER", "new-verifier").unwrap();

    let settings = super::service::load_settings(&path).unwrap();
    assert_eq!(
        settings.0.get("WEB_PASSWORD_VERIFIER"),
        Some(&"new-verifier".to_owned())
    );
}

#[test]
fn st59_settings_keeps_the_original_when_writing_file_cannot_be_created() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    std::fs::write(&path, br#"{"API_TOKEN":"old-api-token"}"#).unwrap();
    std::fs::create_dir(path.with_file_name("server.json.writing")).unwrap();
    assert!(super::service::replace_setting_at(&path, "API_TOKEN", "new-api-token").is_err());
    assert_eq!(
        std::fs::read(&path).unwrap(),
        br#"{"API_TOKEN":"old-api-token"}"#
    );
}

#[test]
fn st59_settings_does_not_write_when_original_json_is_malformed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    std::fs::write(&path, br#"{"API_TOKEN":}"#).unwrap();
    assert!(super::service::replace_setting_at(&path, "API_TOKEN", "new-api-token").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), br#"{"API_TOKEN":}"#);
}

#[test]
fn st59_settings_overwrites_a_stale_writing_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    let writing = path.with_file_name("server.json.writing");
    std::fs::write(&path, br#"{"API_TOKEN":"old-api-token"}"#).unwrap();
    std::fs::write(&writing, b"stale").unwrap();
    super::service::replace_setting_at(&path, "API_TOKEN", "new-api-token").unwrap();

    assert!(!writing.exists());
    assert_eq!(
        super::service::load_settings(&path)
            .unwrap()
            .0
            .get("API_TOKEN"),
        Some(&"new-api-token".to_owned())
    );
}

fn login_request(password: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/session")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({ "password": password }).to_string(),
        ))
        .unwrap()
}

fn read_request(cookie: &str) -> Request<Body> {
    Request::builder()
        .uri("/events")
        .header(header::COOKIE, format!("ashiato_session={cookie}"))
        .body(Body::empty())
        .unwrap()
}

async fn read_events(app: &App, cookie: &str) -> axum::response::Response<Body> {
    router(app.clone())
        .oneshot(read_request(cookie))
        .await
        .unwrap()
}

async fn login(app: &App, password: &str) -> String {
    let response = router(app.clone())
        .oneshot(login_request(password))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    cookie_value(&response)
}

fn cookie_value(response: &axum::response::Response<Body>) -> String {
    response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split('=')
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

fn bearer(token: &str) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    headers
}

#[derive(Clone)]
struct BlockingEvents {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    armed: Arc<AtomicBool>,
}

impl AccessSink for BlockingEvents {
    fn record(&self, _entry: AccessEntry) -> AccessFuture {
        if self.armed.load(Ordering::Acquire) {
            let entered = self.entered.clone();
            let release = self.release.clone();
            Box::pin(async move {
                entered.notify_one();
                release.notified().await;
                Ok(())
            })
        } else {
            Box::pin(async { Ok(()) })
        }
    }
}
