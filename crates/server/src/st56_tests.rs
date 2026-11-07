#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use tower::ServiceExt as _;

use super::*;
use crate::testdb;

const TOKEN: &str = "token-for-test-0123456789";

#[derive(Clone, Default)]
struct Captured(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn setup_request(body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/collector-setup")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

async fn login_cookie(app: &App, password: &str) -> String {
    let response = crate::router(app.clone())
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/session")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({"password": password}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

async fn failed_login(app: &App) {
    let response = crate::router(app.clone())
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/session")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({"password": "wrong-login-0123456789"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

fn with_cookie(mut req: Request<Body>, cookie: &str) -> Request<Body> {
    req.headers_mut()
        .insert(header::COOKIE, cookie.parse().unwrap());
    req
}

async fn app() -> App {
    App::for_test(testdb::pool().await, TOKEN)
}

#[tokio::test]
async fn st56_collector_setup_requires_login_cookie() {
    // Scenario: ログインの印が無ければ答えない
    let response = crate::router(app().await)
        .oneshot(setup_request(
            r#"{"password":"test-web-password-0123456789"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn st56_collector_setup_rejects_collector_credential() {
    // Scenario: 収集アプリの資格情報では答えない
    let mut request = setup_request(r#"{"password":"test-web-password-0123456789"}"#);
    request.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {TOKEN}").parse().unwrap(),
    );
    let response = crate::router(app().await).oneshot(request).await;
    assert_eq!(response.unwrap().status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn st56_collector_setup_returns_token_for_web_login() {
    let app = app().await;
    let cookie = login_cookie(&app, app.web_password_for_test()).await;
    let response = crate::router(app)
        .oneshot(with_cookie(
            setup_request(r#"{"password":"test-web-password-0123456789"}"#),
            &cookie,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({"token": TOKEN})
    );
}

#[tokio::test]
async fn st56_collector_setup_wrong_password_is_forbidden() {
    let app = app().await;
    let cookie = login_cookie(&app, app.web_password_for_test()).await;
    let response = crate::router(app)
        .oneshot(with_cookie(
            setup_request(r#"{"password":"wrong-password-0123456789"}"#),
            &cookie,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({"error": "wrong_password"})
    );
}

#[tokio::test]
async fn st56_collector_setup_logs_request_without_secrets() {
    // Scenario: 求めは読み出しの記録に残り、値は残らない
    let app = app().await;
    let pool = testdb::pool().await;
    let cookie = login_cookie(&app, app.web_password_for_test()).await;
    let captured = Captured::default();
    let sink = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || sink.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);

    let mut wrong_request = with_cookie(
        setup_request(r#"{"password":"wrong-password-0123456789"}"#),
        &cookie,
    );
    wrong_request
        .headers_mut()
        .insert("x-forwarded-for", "198.51.100.56".parse().unwrap());
    let wrong = crate::router(app.clone())
        .oneshot(wrong_request)
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::FORBIDDEN);
    let mut right_request = with_cookie(
        setup_request(r#"{"password":"test-web-password-0123456789"}"#),
        &cookie,
    );
    right_request
        .headers_mut()
        .insert("x-forwarded-for", "198.51.100.56".parse().unwrap());
    let right = crate::router(app).oneshot(right_request).await.unwrap();
    assert_eq!(right.status(), StatusCode::OK);
    drop(guard);

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT outcome, row_to_json(a)::text FROM core.access_log a
         WHERE route = '/collector-setup' AND via = 'forwarded'
         ORDER BY at DESC",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "ok");
    assert_eq!(rows[1].0, "forbidden");
    for (_, row) in rows {
        assert!(!row.contains("wrong-password-0123456789"));
        assert!(!row.contains("test-web-password-0123456789"));
        assert!(!row.contains(TOKEN));
    }
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(!log.contains("wrong-password-0123456789"));
    assert!(!log.contains("test-web-password-0123456789"));
    assert!(!log.contains(TOKEN));
}

#[tokio::test]
async fn st56_collector_setup_shares_login_failure_limit() {
    // Scenario: 失敗が回数の制限を超えると、正しい合言葉でも答えない
    let app = app().await;
    let cookie = login_cookie(&app, app.web_password_for_test()).await;
    for _ in 0..9 {
        let response = crate::router(app.clone())
            .oneshot(with_cookie(
                setup_request(r#"{"password":"wrong-password-0123456789"}"#),
                &cookie,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    failed_login(&app).await;
    let response = crate::router(app)
        .oneshot(with_cookie(
            setup_request(r#"{"password":"test-web-password-0123456789"}"#),
            &cookie,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn st56_collector_setup_rejects_non_json_and_empty_password() {
    // review/code.md R40: ログインの口と同じく、JSON でない本文は 415、空の合言葉は失敗に数えて 403
    let app = app().await;
    let cookie = login_cookie(&app, app.web_password_for_test()).await;
    let mut plain = setup_request(r#"{"password":"test-web-password-0123456789"}"#);
    plain
        .headers_mut()
        .insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
    let response = crate::router(app.clone())
        .oneshot(with_cookie(plain, &cookie))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let response = crate::router(app)
        .oneshot(with_cookie(setup_request(r#"{"password":""}"#), &cookie))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
