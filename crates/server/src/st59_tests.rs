#![allow(clippy::unwrap_used)]

use super::access_log::{AccessEntry, AccessFuture, AccessSink};
use super::testdb;
use super::web_session::{self, SharedSecrets};
use super::{router, App, Authn};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use base64::Engine as _;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tower::ServiceExt as _;

#[derive(Clone, Default)]
struct CapturedLog(Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for CapturedLog {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLog {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[test]
fn st59_web_password_verifier_is_argon2id() {
    let verifier = web_session::password_verifier_for_test("new-web-password-012345").unwrap();
    assert!(verifier.starts_with("$argon2id$"));
    assert!(web_session::verify_password_verifier(
        &verifier,
        "new-web-password-012345"
    ));
}

#[test]
fn st59_collector_token_generation_retries_secret_collisions() {
    let first = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([7u8; 32]);
    let verifier = web_session::password_verifier_for_test(&first).unwrap();
    let mut candidates = vec![[7u8; 32], [8u8; 32]].into_iter();
    let token = web_session::generate_collector_token_for_test(
        "api-token-for-test-012345",
        &first,
        &verifier,
        || candidates.next().unwrap(),
    );
    assert_ne!(token, first);
    assert!(web_session::check_web_password_verifier(Some(&verifier), &token).is_ok());
    assert!(web_session::check_web_session_key_verifier(
        Some(&token),
        "api-token-for-test-012345",
        &verifier,
    )
    .is_ok());
}

/// 指した環境変数を外し、drop で元に戻す（assert が落ちても戻る。review/code.md R25）。
struct EnvRemoved(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl EnvRemoved {
    fn remove(keys: &[&'static str]) -> Self {
        let saved = keys
            .iter()
            .map(|&key| (key, std::env::var_os(key)))
            .collect();
        for key in keys {
            std::env::remove_var(key);
        }
        Self(saved)
    }
}

impl Drop for EnvRemoved {
    fn drop(&mut self) {
        for (key, value) in self.0.drain(..) {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

const SERVICE_ENV: [&str; 2] = ["ASHIATO_SERVICE_MODE", "ASHIATO_SETTINGS_PATH"];

struct St59ServiceEnvironment {
    _lock: std::sync::MutexGuard<'static, ()>,
    _directory: tempfile::TempDir,
    old_mode: Option<std::ffi::OsString>,
    old_path: Option<std::ffi::OsString>,
}

impl Drop for St59ServiceEnvironment {
    fn drop(&mut self) {
        match self.old_mode.take() {
            Some(value) => std::env::set_var("ASHIATO_SERVICE_MODE", value),
            None => std::env::remove_var("ASHIATO_SERVICE_MODE"),
        }
        match self.old_path.take() {
            Some(value) => std::env::set_var("ASHIATO_SETTINGS_PATH", value),
            None => std::env::remove_var("ASHIATO_SETTINGS_PATH"),
        }
    }
}

fn st59_service_environment(api_token: &str, password: &str) -> St59ServiceEnvironment {
    // ST55 の試験と同じ鍵（同じ環境変数を取り合う。review/code.md R25）
    let lock = super::st55_tests::env_lock();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    let verifier = web_session::password_verifier_for_test(password).unwrap();
    std::fs::write(
        &path,
        serde_json::json!({
            "API_TOKEN": api_token,
            "WEB_PASSWORD_VERIFIER": verifier,
            "BIND": "127.0.0.1:18700"
        })
        .to_string(),
    )
    .unwrap();
    let old_mode = std::env::var_os("ASHIATO_SERVICE_MODE");
    let old_path = std::env::var_os("ASHIATO_SETTINGS_PATH");
    std::env::set_var("ASHIATO_SERVICE_MODE", "1");
    std::env::set_var("ASHIATO_SETTINGS_PATH", &path);
    St59ServiceEnvironment {
        _lock: lock,
        _directory: directory,
        old_mode,
        old_path,
    }
}

async fn st59_app() -> App {
    App::for_test(testdb::pool().await, "old-api-token-012345")
        .with_web_login("old-web-password-012345", 0)
}

fn web_password_request(
    cookie: Option<&str>,
    password: &str,
    new_password: &str,
    confirmation: &str,
) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/web-password")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "password": password,
                "new_password": new_password,
                "confirmation": confirmation,
            })
            .to_string(),
        ))
        .unwrap();
    if let Some(cookie) = cookie {
        request.headers_mut().insert(
            header::COOKIE,
            format!("ashiato_session={cookie}").parse().unwrap(),
        );
    }
    request
}

async fn response_body_json(response: axum::response::Response<Body>) -> serde_json::Value {
    use http_body_util::BodyExt as _;
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

async fn response_body_string(response: axum::response::Response<Body>) -> String {
    use http_body_util::BodyExt as _;
    String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap()
}

#[tokio::test]
async fn st59_web_password_rechecks_secrets_after_serialization_permit() {
    let _environment = st59_service_environment("old-api-token", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "old-api-token");
    let cookie = login(&app, app.web_password_for_test()).await;
    let permit = web_session::hold_login_check_for_test().await;
    let request = tokio::spawn(router(app.clone()).oneshot(web_password_request(
        Some(&cookie),
        "old-web-password-012345",
        "new-api-token-0123456789",
        "new-api-token-0123456789",
    )));
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    app.replace_secrets_for_test("new-api-token-0123456789", "old-web-password-012345");
    drop(permit);
    let response = request.await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_body_json(response).await,
        serde_json::json!({"error": "same_as_api_token"})
    );
}

async fn st59_change(
    app: &App,
    cookie: Option<&str>,
    password: &str,
    new_password: &str,
) -> axum::response::Response<Body> {
    router(app.clone())
        .oneshot(web_password_request(
            cookie,
            password,
            new_password,
            new_password,
        ))
        .await
        .unwrap()
}

fn collector_token_change_request(cookie: Option<&str>, password: &str) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/collector-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({ "password": password }).to_string(),
        ))
        .unwrap();
    if let Some(cookie) = cookie {
        request.headers_mut().insert(
            header::COOKIE,
            format!("ashiato_session={cookie}").parse().unwrap(),
        );
    }
    request
}

fn collector_setup_request(cookie: &str, password: &str) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/collector-setup")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({"password": password}).to_string(),
        ))
        .unwrap();
    request.headers_mut().insert(
        header::COOKIE,
        format!("ashiato_session={cookie}").parse().unwrap(),
    );
    request
}

#[tokio::test]
// Scenario: 変えた後は、前の値の記録は断られ、新しい値の記録は取り込まれる
async fn st59_collector_token_changes_the_token() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let response = router(app.clone())
        .oneshot(collector_token_change_request(
            Some(&cookie),
            app.web_password_for_test(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_json(response).await;
    let new_token = body["token"].as_str().unwrap();
    assert_ne!(new_token, "old-api-token-012345");
    let old_response = router(app.clone())
        .oneshot(events_bearer("old-api-token-012345"))
        .await
        .unwrap();
    assert_eq!(old_response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        super::decided(&app, &bearer(new_token)).await,
        Authn::Decided(Some(web_session::Caller::ApiToken))
    );
    let new_response = router(app).oneshot(events_bearer(new_token)).await.unwrap();
    assert_eq!(new_response.status(), StatusCode::OK);
}

async fn st59_change_collector(app: &App, cookie: &str) -> (StatusCode, serde_json::Value) {
    let response = router(app.clone())
        .oneshot(collector_token_change_request(
            Some(cookie),
            app.web_password_for_test(),
        ))
        .await
        .unwrap();
    let status = response.status();
    let body = response_body_json(response).await;
    (status, body)
}

#[tokio::test]
// Scenario: 今の合言葉が違えば、収集アプリの値は変わらない
async fn st59_collector_token_wrong_password_keeps_old_token() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let response = router(app.clone())
        .oneshot(collector_token_change_request(
            Some(&cookie),
            "wrong-web-password-012345",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_body_json(response).await,
        serde_json::json!({"error":"wrong_password"})
    );
    assert_eq!(
        super::decided(&app, &bearer("old-api-token-012345")).await,
        Authn::Decided(Some(web_session::Caller::ApiToken))
    );
}

#[tokio::test]
// Scenario: ログインの印が無い・収集アプリの値・衛星の資格情報では、収集アプリの値を変えられない
async fn st59_collector_token_rejects_non_web_callers() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let settings_path =
        std::path::PathBuf::from(std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap());
    let before_settings = std::fs::read(&settings_path).unwrap();
    assert_eq!(
        router(app.clone())
            .oneshot(collector_token_change_request(
                None,
                app.web_password_for_test()
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        router(app.clone())
            .oneshot({
                let mut r = collector_token_change_request(None, app.web_password_for_test());
                r.headers_mut().insert(
                    header::AUTHORIZATION,
                    "Bearer old-api-token-012345".parse().unwrap(),
                );
                r
            })
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let (satellite, identifier) = super::satellite_authn_tests::seed_satellite(&app.pool).await;
    let (_, satellite_token) = super::satellite_authn_tests::seed_credential(
        &app.pool,
        satellite,
        &identifier,
        chrono::Utc::now(),
    )
    .await;
    let mut satellite_request = collector_token_change_request(None, app.web_password_for_test());
    satellite_request.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {satellite_token}").parse().unwrap(),
    );
    assert_eq!(
        router(app)
            .oneshot(satellite_request)
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(std::fs::read(settings_path).unwrap(), before_settings);
}

#[tokio::test]
// Scenario: 収集アプリの値を変えても、画面のログインは切れない
async fn st59_collector_token_keeps_web_login() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    assert_eq!(st59_change_collector(&app, &cookie).await.0, StatusCode::OK);
    assert_eq!(read_events(&app, &cookie).await.status(), StatusCode::OK);
}

#[tokio::test]
// Scenario: 新しい値はサーバが作り、変えるたびに違う
async fn st59_collector_token_generates_distinct_values() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let first = st59_change_collector(&app, &cookie).await.1["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = st59_change_collector(&app, &cookie).await.1["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(first, second);
    assert!(first.len() >= 16 && second.len() >= 16);
    assert_ne!(first, "old-api-token-012345");
    assert_ne!(second, "old-api-token-012345");
    assert_ne!(first, "test-session-key-0123456789");
    assert_ne!(second, "test-session-key-0123456789");
    let verifier = web_session::password_verifier_for_test(app.web_password_for_test()).unwrap();
    assert!(web_session::check_web_password_verifier(Some(&verifier), &first).is_ok());
    assert!(web_session::check_web_password_verifier(Some(&verifier), &second).is_ok());
}

#[tokio::test]
// Scenario: 収集アプリの値を変えると、設定のファイルは収集アプリの値だけが変わる
async fn st59_collector_token_updates_only_api_token() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let path = std::path::PathBuf::from(std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap());
    let before: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let returned = st59_change_collector(&app, &cookie).await.1["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let after: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(
        before["WEB_PASSWORD_VERIFIER"],
        after["WEB_PASSWORD_VERIFIER"]
    );
    assert_eq!(before["BIND"], after["BIND"]);
    assert_ne!(before["API_TOKEN"], after["API_TOKEN"]);
    assert_eq!(after["API_TOKEN"], returned);
}

#[tokio::test]
// Scenario: 変えた後に起動し直しても、新しい収集アプリの値で動く
async fn st59_collector_token_uses_new_token_after_rebuilding_app() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let new_token = st59_change_collector(&app, &cookie).await.1["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = std::path::PathBuf::from(std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap());
    let settings = super::service::load_settings(&path).unwrap();
    assert_eq!(settings.0.get("API_TOKEN").unwrap(), &new_token);
    let rebuilt = App::for_test(testdb::pool().await, settings.0.get("API_TOKEN").unwrap());
    assert_eq!(
        super::decided(&rebuilt, &bearer(&new_token)).await,
        Authn::Decided(Some(web_session::Caller::ApiToken))
    );
}

#[tokio::test]
// Scenario: 失敗が回数の制限を超えると、正しい合言葉でも変えられない
async fn st59_both_failure_limiter_throttles_collector_token() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let settings_path =
        std::path::PathBuf::from(std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap());
    let before_settings = std::fs::read(&settings_path).unwrap();
    assert_eq!(
        router(app.clone())
            .oneshot(login_request("wrong-login-0123456789"))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        router(app.clone())
            .oneshot(collector_setup_request(&cookie, "wrong-setup-0123456789"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    for _ in 0..4 {
        assert_eq!(
            router(app.clone())
                .oneshot(web_password_request(
                    Some(&cookie),
                    "wrong-web-password-012345",
                    "new-web-password-012345",
                    "new-web-password-012345",
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    for _ in 0..4 {
        assert_eq!(
            router(app.clone())
                .oneshot(collector_token_change_request(
                    Some(&cookie),
                    "wrong-web-password-012345"
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        router(app.clone())
            .oneshot(web_password_request(
                Some(&cookie),
                "old-web-password-012345",
                "new-web-password-012345",
                "new-web-password-012345",
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        router(app)
            .oneshot(collector_token_change_request(
                Some(&cookie),
                "old-web-password-012345"
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(std::fs::read(settings_path).unwrap(), before_settings);
}

#[tokio::test]
// Scenario: 本文が JSON でない求めでは、何も変わらない
async fn st59_both_non_json_is_rejected() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let mut request = collector_token_change_request(Some(&cookie), app.web_password_for_test());
    request
        .headers_mut()
        .insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
    assert_eq!(
        router(app).oneshot(request).await.unwrap().status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
}

#[tokio::test]
// Scenario: 合言葉と収集アプリの値を変える求めの断りは、返した状態のとおりに残る
async fn st59_both_rejection_statuses_are_recorded() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let pool = testdb::pool().await;
    let expected: [(&str, &str, i16); 7] = [
        ("/web-password", "unauthorized", 401),
        ("/collector-token", "unauthorized", 401),
        ("/web-password", "forbidden", 409),
        ("/collector-token", "forbidden", 409),
        ("/web-password", "forbidden", 400),
        ("/web-password", "login_throttled", 429),
        ("/collector-token", "login_throttled", 429),
    ];
    let before: Vec<i64> = futures_util::future::join_all(expected.iter().map(|(route, outcome, status)| {
        let query_pool = pool.clone();
        async move {
            sqlx::query_scalar("SELECT count(*) FROM core.access_log WHERE route = $1 AND outcome = $2 AND status = $3")
                .bind(*route).bind(*outcome).bind(*status).fetch_one(&query_pool).await.unwrap()
        }
    })).await;
    for (route, response) in [
        (
            "/web-password",
            router(app.clone())
                .oneshot(web_password_request(
                    None,
                    "old-web-password-012345",
                    "new-web-password-012345",
                    "new-web-password-012345",
                ))
                .await
                .unwrap(),
        ),
        (
            "/collector-token",
            router(app.clone())
                .oneshot(collector_token_change_request(
                    None,
                    "old-web-password-012345",
                ))
                .await
                .unwrap(),
        ),
    ] {
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{route}");
    }
    let development = EnvRemoved::remove(&SERVICE_ENV);
    for response in [
        router(app.clone())
            .oneshot(web_password_request(
                Some(&cookie),
                "old-web-password-012345",
                "new-web-password-012345",
                "new-web-password-012345",
            ))
            .await
            .unwrap(),
        router(app.clone())
            .oneshot(collector_token_change_request(
                Some(&cookie),
                "old-web-password-012345",
            ))
            .await
            .unwrap(),
    ] {
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
    drop(development);
    let response = router(app.clone())
        .oneshot(web_password_request(
            Some(&cookie),
            "old-web-password-012345",
            "short",
            "short",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    for _ in 0..10 {
        let _ = router(app.clone())
            .oneshot(web_password_request(
                Some(&cookie),
                "wrong-web-password-012345",
                "new-web-password-012345",
                "new-web-password-012345",
            ))
            .await
            .unwrap();
    }
    for response in [
        router(app.clone())
            .oneshot(web_password_request(
                Some(&cookie),
                "old-web-password-012345",
                "new-web-password-012345",
                "new-web-password-012345",
            ))
            .await
            .unwrap(),
        router(app.clone())
            .oneshot(collector_token_change_request(
                Some(&cookie),
                "old-web-password-012345",
            ))
            .await
            .unwrap(),
    ] {
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
    for ((route, outcome, status), old) in expected.into_iter().zip(before) {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM core.access_log WHERE route = $1 AND outcome = $2 AND status = $3",
        )
        .bind(route)
        .bind(outcome)
            .bind(status)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, old + 1, "{route} {outcome} {status}");
    }
}

#[tokio::test]
// Scenario: 書き戻しに落ちた求めは、変えた行と打ち消しの行の 2 行で残る
// Scenario: 設定のファイルに書けなければ、何も変わらない
async fn st59_both_write_failure_returns_not_saved() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let pool = testdb::pool().await;
    let before_logs: Vec<i64> = futures_util::future::join_all(
        [
            ("/web-password", "ok", 204),
            ("/web-password", "forbidden", 500),
            ("/collector-token", "ok", 200),
            ("/collector-token", "forbidden", 500),
        ]
        .into_iter()
        .map(|(route, outcome, status)| {
            let query_pool = pool.clone();
            async move {
                sqlx::query_scalar(
                    "SELECT count(*) FROM core.access_log WHERE route = $1 AND outcome = $2 AND status = $3",
                )
                .bind(route)
                .bind(outcome)
                .bind(status as i16)
                .fetch_one(&query_pool)
                .await
                .unwrap()
            }
        }),
    )
    .await;
    let path = std::path::PathBuf::from(std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap());
    let before = std::fs::read(&path).unwrap();
    std::fs::create_dir(path.with_file_name("server.json.writing")).unwrap();
    let captured = CapturedLog::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);
    for response in [
        router(app.clone())
            .oneshot(web_password_request(
                Some(&cookie),
                "old-web-password-012345",
                "new-web-password-012345",
                "new-web-password-012345",
            ))
            .await
            .unwrap(),
        router(app.clone())
            .oneshot(collector_token_change_request(
                Some(&cookie),
                "old-web-password-012345",
            ))
            .await
            .unwrap(),
    ] {
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response_body_json(response).await,
            serde_json::json!({"error":"not_saved"})
        );
    }
    drop(guard);
    // 書き戻せなかった理由（秘密を含まない語）がログに残る（review/code.md R27）
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    for op in ["web_password.save", "collector_token.save"] {
        assert!(log.contains(op), "{op}: {log}");
    }
    assert_eq!(log.matches("settings_write_failed").count(), 2, "{log}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let writing = path.with_file_name("server.json.writing");
    assert!(writing.is_dir());
    std::fs::remove_dir(&writing).unwrap();
    assert!(!writing.exists());
    assert_eq!(
        router(app.clone())
            .oneshot(events_bearer("old-api-token-012345"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert!(login(&app, "old-web-password-012345").await.len() > 10);
    for ((route, outcome, status), old) in [
        ("/web-password", "ok", 204),
        ("/web-password", "forbidden", 500),
        ("/collector-token", "ok", 200),
        ("/collector-token", "forbidden", 500),
    ]
    .into_iter()
    .zip(before_logs)
    {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM core.access_log WHERE route = $1 AND outcome = $2 AND status = $3",
        )
        .bind(route)
        .bind(outcome)
        .bind(status as i16)
        .fetch_one(&pool)
        .await
        .unwrap();
        // 変えた行（`ok`）と打ち消しの行（`forbidden`/500）が、口ごとに 1 行ずつ増える（R23）
        assert_eq!(count, old + 1, "{route} {outcome} {status}");
    }
    // 打ち消しの行は変えた行の後に並ぶ（`ok` を先に書いてから書き戻す。data-sensitivity・design D1）
    for (route, ok_status) in [("/web-password", 204i16), ("/collector-token", 200)] {
        let (ok_at, refused_at): (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
            sqlx::query_as(
                "SELECT (SELECT max(at) FROM core.access_log WHERE route = $1 AND outcome = 'ok' AND status = $2),
                        (SELECT max(at) FROM core.access_log WHERE route = $1 AND outcome = 'forbidden' AND status = 500)",
            )
            .bind(route)
            .bind(ok_status)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(
            ok_at <= refused_at,
            "{route}: ok {ok_at} / forbidden {refused_at}"
        );
    }
}

#[tokio::test]
// Scenario: 合言葉と収集アプリの値を変える求めの値は、記録にもログにも応答の見出しにも残らない
async fn st59_both_change_values_are_not_logged() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let captured = CapturedLog::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);
    let wrong = router(app.clone())
        .oneshot(web_password_request(
            Some(&cookie),
            "wrong-web-password-012345",
            "new-web-password-012345",
            "new-web-password-012345",
        ))
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::FORBIDDEN);
    let wrong_headers = format!("{:?}", wrong.headers());
    assert!(!wrong_headers.contains("old-web-password-012345"));
    assert!(!wrong_headers.contains("wrong-web-password-012345"));
    let wrong_body = response_body_json(wrong).await;
    assert_eq!(wrong_body, serde_json::json!({"error":"wrong_password"}));
    let web = router(app.clone())
        .oneshot(web_password_request(
            Some(&cookie),
            "old-web-password-012345",
            "new-web-password-012345",
            "new-web-password-012345",
        ))
        .await
        .unwrap();
    assert_eq!(web.status(), StatusCode::NO_CONTENT);
    let web_headers = format!("{:?}", web.headers());
    assert!(!web_headers.contains("old-web-password-012345"));
    assert!(!web_headers.contains("new-web-password-012345"));
    assert!(!web_headers.contains("old-api-token-012345"));
    let new_cookie = cookie_value(&web);
    let collector = router(app)
        .oneshot(collector_token_change_request(
            Some(&new_cookie),
            "new-web-password-012345",
        ))
        .await
        .unwrap();
    assert_eq!(collector.status(), StatusCode::OK);
    let collector_headers = format!("{:?}", collector.headers());
    assert!(!collector_headers.contains("old-web-password-012345"));
    assert!(!collector_headers.contains("new-web-password-012345"));
    assert!(!collector_headers.contains("old-api-token-012345"));
    let token = response_body_json(collector).await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let pool = testdb::pool().await;
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT row_to_json(a)::text FROM core.access_log a WHERE route IN ('/web-password','/collector-token') ORDER BY at DESC LIMIT 4",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    for row in rows {
        assert!(!row.contains("old-web-password-012345"));
        assert!(!row.contains("wrong-web-password-012345"));
        assert!(!row.contains("new-web-password-012345"));
        assert!(!row.contains("old-api-token-012345"));
        assert!(!row.contains(&token));
        assert!(row.contains("/web-password") || row.contains("/collector-token"));
    }
    drop(guard);
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    for secret in [
        "old-web-password-012345",
        "wrong-web-password-012345",
        "new-web-password-012345",
        "old-api-token-012345",
        token.as_str(),
    ] {
        assert!(!log.contains(secret), "秘密が tracing に出た: {log}");
    }
}

#[tokio::test]
async fn st59_both_access_log_failure_returns_500_without_changes() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let base = st59_app().await;
    let cookie = login(&base, base.web_password_for_test()).await;
    let app = base.with_access_sink(FailingAccessSink);
    let path = std::path::PathBuf::from(std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap());
    let before = std::fs::read(&path).unwrap();
    for response in [
        router(app.clone())
            .oneshot(web_password_request(
                Some(&cookie),
                "old-web-password-012345",
                "new-web-password-012345",
                "new-web-password-012345",
            ))
            .await
            .unwrap(),
        router(app)
            .oneshot(collector_token_change_request(
                Some(&cookie),
                "old-web-password-012345",
            ))
            .await
            .unwrap(),
    ] {
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
    // `ok` の行を書く前に断るので、書き戻しも巻き戻しも起きず、ファイルはバイトのまま変わらない（R23）
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[tokio::test]
// 書き戻しに落ちた後、打ち消しの行も書けなくても、`not_saved` を返す（review/code.md R28）
async fn st59_both_write_failure_returns_not_saved_even_if_refusal_row_fails() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let base = st59_app().await;
    let cookie = login(&base, base.web_password_for_test()).await;
    let app = base.with_access_sink(FailingRefusalSink);
    let path = std::path::PathBuf::from(std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap());
    let before = std::fs::read(&path).unwrap();
    let writing = path.with_file_name("server.json.writing");
    std::fs::create_dir(&writing).unwrap();
    for response in [
        router(app.clone())
            .oneshot(web_password_request(
                Some(&cookie),
                "old-web-password-012345",
                "new-web-password-012345",
                "new-web-password-012345",
            ))
            .await
            .unwrap(),
        router(app.clone())
            .oneshot(collector_token_change_request(
                Some(&cookie),
                "old-web-password-012345",
            ))
            .await
            .unwrap(),
    ] {
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response_body_json(response).await,
            serde_json::json!({"error":"not_saved"})
        );
    }
    std::fs::remove_dir(&writing).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[tokio::test]
// 書き戻しより前の 500 の道のうち、起こせるもの（印の行の INSERT の失敗）でも、理由がログに出て
// `forbidden`/500 の行が残り、`ok` の行は残らない（review/code.md R27・R29）。
// 起こせない道: hash（argon2 の失敗）と収集アプリの値の生成は、差し込み口の無い純粋な計算。衝突の検査の
// `Ok(false)` は `validate_new_web_password` が先に同じ値を断るので通らない守りの道。どれも同じ
// `refuse_internal` を通る（R30）ので、行とログの形はこの試験の道と同じになる。
async fn st59_web_password_session_insert_failure_is_logged_and_recorded() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let db = testdb::isolated_with_app().await;
    let app = App::for_test(db.owner.clone(), "old-api-token-012345")
        .with_web_login("old-web-password-012345", 0);
    let cookie = login(&app, app.web_password_for_test()).await;
    for statement in [
        "CREATE FUNCTION public.st59_refuse_session() RETURNS trigger LANGUAGE plpgsql
           AS $$ BEGIN RAISE EXCEPTION 'st59 refuses'; END $$",
        "CREATE TRIGGER st59_refuse_session BEFORE INSERT ON core.web_session
           FOR EACH ROW EXECUTE FUNCTION public.st59_refuse_session()",
    ] {
        sqlx::query(statement).execute(&db.owner).await.unwrap();
    }
    let path = std::path::PathBuf::from(std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap());
    let before = std::fs::read(&path).unwrap();
    let captured = CapturedLog::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);
    let response = router(app.clone())
        .oneshot(web_password_request(
            Some(&cookie),
            "old-web-password-012345",
            "new-web-password-012345",
            "new-web-password-012345",
        ))
        .await
        .unwrap();
    drop(guard);
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("web_password.session_insert"), "{log}");
    for (outcome, status, expected) in [("forbidden", 500i16, 1i64), ("ok", 204, 0)] {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM core.access_log WHERE route = '/web-password' AND outcome = $1 AND status = $2",
        )
        .bind(outcome)
        .bind(status)
        .fetch_one(&db.owner)
        .await
        .unwrap();
        assert_eq!(count, expected, "{outcome} {status}");
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
// Scenario: 開発の形のサーバでは、画面から変えられないと出る
async fn st59_both_development_mode_is_not_changeable() {
    let _lock = super::st55_tests::env_lock();
    let _development = EnvRemoved::remove(&SERVICE_ENV);
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    assert_eq!(
        router(app)
            .oneshot(collector_token_change_request(
                Some(&cookie),
                "old-web-password-012345"
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
}

#[tokio::test]
// Scenario: 合言葉と収集アプリの値を変える求めは、変えたものも断ったものも残る
async fn st59_both_changes_are_access_logged() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let pool = testdb::pool().await;
    let before: Vec<i64> = futures_util::future::join_all([
        ("/web-password", "forbidden", 403),
        ("/web-password", "ok", 204),
        ("/collector-token", "forbidden", 403),
        ("/collector-token", "ok", 200),
    ].into_iter().map(|(route, outcome, status)| {
        let query_pool = pool.clone();
        async move {
        sqlx::query_scalar("SELECT count(*) FROM core.access_log WHERE route = $1 AND outcome = $2 AND status = $3")
            .bind(route).bind(outcome).bind(status as i16).fetch_one(&query_pool).await.unwrap()
        }
    })).await;
    assert_eq!(
        router(app.clone())
            .oneshot(web_password_request(
                Some(&cookie),
                "wrong-web-password-012345",
                "new-web-password-012345",
                "new-web-password-012345"
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        router(app.clone())
            .oneshot(collector_token_change_request(
                Some(&cookie),
                "wrong-web-password-012345"
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let web_response = router(app.clone())
        .oneshot(web_password_request(
            Some(&cookie),
            "old-web-password-012345",
            "new-web-password-012345",
            "new-web-password-012345",
        ))
        .await
        .unwrap();
    assert_eq!(web_response.status(), StatusCode::NO_CONTENT);
    let new_cookie = cookie_value(&web_response);
    assert_eq!(
        router(app)
            .oneshot(collector_token_change_request(
                Some(&new_cookie),
                "new-web-password-012345"
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    for ((route, outcome, status), old) in [
        ("/web-password", "forbidden", 403),
        ("/web-password", "ok", 204),
        ("/collector-token", "forbidden", 403),
        ("/collector-token", "ok", 200),
    ]
    .into_iter()
    .zip(before)
    {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM core.access_log WHERE route = $1 AND outcome = $2 AND status = $3")
            .bind(route).bind(outcome).bind(status as i16).fetch_one(&pool).await.unwrap();
        assert_eq!(count, old + 1, "{route} {outcome} {status}");
    }
}

#[tokio::test]
// Scenario: 新しい合言葉が検査に当たれば、何も変わらない
async fn st59_web_password_handler_applies_all_five_validation_rules() {
    let _environment = st59_service_environment("api-token-0123456789", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let cookie = login(&app, app.web_password_for_test()).await;
    let path = std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap();
    let before = std::fs::read(&path).unwrap();
    for (value, reason) in [
        ("123456789012345", "too_short"),
        ("change-me-web-password-012345", "placeholder"),
        ("api-token-0123456789", "same_as_api_token"),
        ("test-session-key-0123456789", "same_as_session_key"),
    ] {
        let response = st59_change(&app, Some(&cookie), app.web_password_for_test(), value).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_body_json(response).await,
            serde_json::json!({"error": reason})
        );
    }
    let mismatch = web_password_request(
        Some(&cookie),
        app.web_password_for_test(),
        "new-web-password-012345",
        "different-web-password-012345",
    );
    let mismatch = router(app.clone()).oneshot(mismatch).await.unwrap();
    assert_eq!(mismatch.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_body_json(mismatch).await,
        serde_json::json!({"error":"mismatch"})
    );
    assert_eq!(std::fs::read(path).unwrap(), before);
    let accepted = st59_change(
        &app,
        Some(&cookie),
        app.web_password_for_test(),
        "1234567890123456",
    )
    .await;
    assert_eq!(
        accepted.status(),
        StatusCode::NO_CONTENT,
        "16 文字は短すぎではない"
    );
}

#[tokio::test]
// Scenario: ログインの印が無い・収集アプリの値・衛星の資格情報では、合言葉を変えられない
async fn st59_web_password_handler_rejects_non_web_callers() {
    let _environment = st59_service_environment("api-token-0123456789", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let no_cookie = st59_change(
        &app,
        None,
        "old-web-password-012345",
        "new-web-password-012345",
    )
    .await;
    assert_eq!(no_cookie.status(), StatusCode::UNAUTHORIZED);
    let mut api_request = web_password_request(
        None,
        app.web_password_for_test(),
        "new-web-password-012345",
        "new-web-password-012345",
    );
    api_request.headers_mut().insert(
        header::AUTHORIZATION,
        "Bearer api-token-0123456789".parse().unwrap(),
    );
    let api = router(app.clone()).oneshot(api_request).await.unwrap();
    assert_eq!(api.status(), StatusCode::FORBIDDEN);
    assert_ne!(response_body_string(api).await, "wrong_password");
    let (satellite, identifier) =
        crate::satellite_authn_tests::seed_satellite(&testdb::pool().await).await;
    let (_, credential) = crate::satellite_authn_tests::seed_credential(
        &testdb::pool().await,
        satellite,
        &identifier,
        chrono::Utc::now(),
    )
    .await;
    let mut satellite_request = web_password_request(
        None,
        app.web_password_for_test(),
        "new-web-password-012345",
        "new-web-password-012345",
    );
    satellite_request.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {credential}").parse().unwrap(),
    );
    let satellite = router(app).oneshot(satellite_request).await.unwrap();
    assert_eq!(satellite.status(), StatusCode::FORBIDDEN);
    assert_ne!(response_body_string(satellite).await, "wrong_password");
}

#[tokio::test]
// Scenario: 画面の合言葉を変えると、設定のファイルは照らす値だけが変わる
async fn st59_web_password_handler_updates_only_the_verifier_key() {
    let _environment = st59_service_environment("api-token-0123456789", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let cookie = login(&app, app.web_password_for_test()).await;
    let path = std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap();
    let before: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let response = st59_change(
        &app,
        Some(&cookie),
        app.web_password_for_test(),
        "new-web-password-012345",
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let after: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(before["API_TOKEN"], after["API_TOKEN"]);
    assert_eq!(before["BIND"], after["BIND"]);
    assert_ne!(
        before["WEB_PASSWORD_VERIFIER"],
        after["WEB_PASSWORD_VERIFIER"]
    );
    assert!(web_session::verify_password_verifier(
        after["WEB_PASSWORD_VERIFIER"].as_str().unwrap(),
        "new-web-password-012345"
    ));
}

#[tokio::test]
// Scenario: 画面から合言葉を変えると、起動し直さずにほかのログインが使えなくなる
// Scenario: 合言葉を変えると、ほかのブラウザのログインは切れ、変えた画面はログインしたまま
async fn st59_web_password_handler_revokes_other_cookies_and_returns_new_one() {
    let _environment = st59_service_environment("api-token-0123456789", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let first = login(&app, app.web_password_for_test()).await;
    let second = login(&app, app.web_password_for_test()).await;
    let response = st59_change(
        &app,
        Some(&first),
        app.web_password_for_test(),
        "new-web-password-012345",
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let returned = cookie_value(&response);
    assert_eq!(
        read_events(&app, &second).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(read_events(&app, &returned).await.status(), StatusCode::OK);
    assert_eq!(
        read_events(&app, &first).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
// Scenario: 今の合言葉が違えば、新しい合言葉が検査に当たっても今の合言葉の違いで断る
async fn st59_web_password_handler_checks_old_password_before_new_rules() {
    let _environment = st59_service_environment("api-token-0123456789", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let cookie = login(&app, app.web_password_for_test()).await;
    let response = st59_change(&app, Some(&cookie), "wrong-web-password-012345", "short").await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_body_json(response).await,
        serde_json::json!({"error": "wrong_password"})
    );
}

#[tokio::test]
// Scenario: 合言葉を変えても、失敗の数えは数え直されない
async fn st59_web_password_handler_preserves_failure_limiter() {
    let _environment = st59_service_environment("api-token-0123456789", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let cookie = login(&app, app.web_password_for_test()).await;
    for _ in 0..9 {
        assert_eq!(
            st59_change(
                &app,
                Some(&cookie),
                "wrong-web-password-012345",
                "new-web-password-012345"
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        st59_change(
            &app,
            Some(&cookie),
            app.web_password_for_test(),
            "new-web-password-012345"
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        router(app.clone())
            .oneshot(login_request("wrong-web-password-012345"))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        router(app)
            .oneshot(login_request("new-web-password-012345"))
            .await
            .unwrap()
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn st59_web_password_handler_rejects_development_mode() {
    let _lock = super::st55_tests::env_lock();
    let _development = EnvRemoved::remove(&SERVICE_ENV);
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let cookie = login(&app, app.web_password_for_test()).await;
    let response = st59_change(
        &app,
        Some(&cookie),
        app.web_password_for_test(),
        "new-web-password-012345",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_body_json(response).await,
        serde_json::json!({"error": "not_changeable"})
    );
}

#[tokio::test]
async fn st59_web_password_handler_rejects_non_json() {
    let _environment = st59_service_environment("api-token-0123456789", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let cookie = login(&app, app.web_password_for_test()).await;
    let mut request = web_password_request(
        Some(&cookie),
        app.web_password_for_test(),
        "new-web-password-012345",
        "new-web-password-012345",
    );
    request
        .headers_mut()
        .insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
    assert_eq!(
        router(app).oneshot(request).await.unwrap().status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
}

#[tokio::test]
// Scenario: 変えた後に起動し直しても、新しい合言葉でログインでき、前の合言葉ではできない
async fn st59_web_password_handler_uses_new_password_after_rebuilding_app() {
    let _environment = st59_service_environment("api-token-0123456789", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let cookie = login(&app, app.web_password_for_test()).await;
    assert_eq!(
        st59_change(
            &app,
            Some(&cookie),
            app.web_password_for_test(),
            "new-web-password-012345"
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let settings_path =
        std::path::PathBuf::from(std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap());
    let settings = super::service::load_settings(&settings_path).unwrap();
    let verifier = settings.0.get("WEB_PASSWORD_VERIFIER").unwrap();
    let rebuilt = app.with_web_login_verifier(verifier, 0);
    assert_eq!(
        router(rebuilt.clone())
            .oneshot(login_request("old-web-password-012345"))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        router(rebuilt)
            .oneshot(login_request("new-web-password-012345"))
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn st59_web_password_handler_does_not_record_secrets() {
    let _environment = st59_service_environment("api-token-0123456789", "old-web-password-012345");
    let app = App::for_test(testdb::pool().await, "api-token-0123456789");
    let pool = testdb::pool().await;
    let cookie = login(&app, app.web_password_for_test()).await;
    let response = st59_change(
        &app,
        Some(&cookie),
        app.web_password_for_test(),
        "new-web-password-012345",
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let rows: Vec<String> = sqlx::query_scalar("SELECT row_to_json(a)::text FROM core.access_log a WHERE route = '/web-password' ORDER BY at DESC LIMIT 2").fetch_all(&pool).await.unwrap();
    assert!(rows
        .iter()
        .all(|row| !row.contains("old-web-password-012345")
            && !row.contains("new-web-password-012345")
            && !row.contains("api-token-0123456789")));
}

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// Scenario: 変えている最中に前の合言葉で照合が通ったログインの印は、変えた後は使えない
async fn st59_swap_login_cookie_uses_the_checked_tag_and_is_revoked_after_swap() {
    let _environment = st59_service_environment("old-api-token-012345", "old-web-password-012345");
    let app = st59_app().await;
    let change_cookie = login(&app, app.web_password_for_test()).await;
    let old_password = app.web_password_for_test().to_owned();
    let before_cookie_insert = app.arm_login_cookie_insert_hold_for_test();
    let request = tokio::spawn(router(app.clone()).oneshot(login_request(&old_password)));
    before_cookie_insert.await;
    let changed = router(app.clone())
        .oneshot(web_password_request(
            Some(&change_cookie),
            &old_password,
            "new-web-password-012345",
            "new-web-password-012345",
        ))
        .await
        .unwrap();
    assert_eq!(changed.status(), StatusCode::NO_CONTENT);
    app.release_login_cookie_insert_hold_for_test();
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

fn events_bearer(token: &str) -> Request<Body> {
    Request::builder()
        .uri("/contract")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
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

/// 打ち消しの行（`forbidden`/500）だけを書けない記録の口（review/code.md R28）。
struct FailingRefusalSink;

impl AccessSink for FailingRefusalSink {
    fn record(&self, entry: AccessEntry) -> AccessFuture {
        if entry.outcome == "forbidden" && entry.status == 500 {
            Box::pin(async { Err(sqlx::Error::RowNotFound) })
        } else {
            Box::pin(async { Ok(()) })
        }
    }
}

struct FailingAccessSink;

impl AccessSink for FailingAccessSink {
    fn record(&self, _entry: AccessEntry) -> AccessFuture {
        Box::pin(async { Err(sqlx::Error::RowNotFound) })
    }
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
