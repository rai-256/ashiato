//! サーバの系統ごとの契約・既定値・画面の印（ST54 Task 2）。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use tower::ServiceExt as _;

use super::*;
use crate::testdb;
use ashiato_build_stamp::Channel;

const TOKEN: &str = "st54-test-token-0123456789";

async fn app(channel: Channel) -> App {
    App::for_test(testdb::pool().await, TOKEN).with_channel(channel)
}

async fn contract(app: App) -> axum::response::Response<Body> {
    let request = Request::builder()
        .method(Method::GET)
        .uri("/contract")
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap();
    router(app).oneshot(request).await.unwrap()
}

async fn contract_body(response: axum::response::Response<Body>) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

// Scenario: 開発版のサーバは、契約の問い合わせに系統 dev を答える
#[tokio::test]
async fn st54_contract_answers_dev_channel() {
    let response = contract(app(Channel::Dev).await).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Ashiato-Channel"], "dev");
    assert_eq!(contract_body(response).await, br#"{"contract_version":1}"#);
}

// Scenario: 正式版のサーバは、契約の問い合わせに系統 official を答える
#[tokio::test]
async fn st54_contract_answers_official_channel() {
    let response = contract(app(Channel::Official).await).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Ashiato-Channel"], "official");
    assert_eq!(contract_body(response).await, br#"{"contract_version":1}"#);
}

// Scenario: 開発版のサーバの既定は、正式版と別の待ち受け・クッキー・退避の置き場
#[test]
fn st54_dev_defaults_are_separate() {
    assert_eq!(net_guard::default_bind(Channel::Dev), "127.0.0.1:18788");
    assert_eq!(
        web_session::cookie_name(Channel::Dev),
        "ashiato_session_dev"
    );
    assert_eq!(
        migrate_backup::default_dir(Channel::Dev),
        "backups/pre-migrate-dev"
    );
}

// Scenario: 正式版のサーバの既定は、今のまま
#[test]
fn st54_official_defaults_are_unchanged() {
    assert_eq!(
        net_guard::default_bind(Channel::Official),
        "127.0.0.1:18787"
    );
    assert_eq!(
        web_session::cookie_name(Channel::Official),
        "ashiato_session"
    );
    assert_eq!(
        migrate_backup::default_dir(Channel::Official),
        "backups/pre-migrate"
    );
}

// Scenario: 開発版のサーバの画面のログインは、開発版の名前のクッキーだけを使う
#[tokio::test]
async fn st54_dev_web_login_uses_dev_cookie() {
    let password = "st54-web-password-0123456789";
    let app = app(Channel::Dev).await.with_web_login(password, 0);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/session")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(r#"{{"password":"{password}"}}"#)))
        .unwrap();
    let app_router = router(app);
    let response = app_router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(set_cookie.starts_with("ashiato_session_dev="));
    let dev_cookie = set_cookie.split(';').next().unwrap();

    let response = app_router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/session")
                .header(header::COOKIE, dev_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app_router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/session")
                .header(header::COOKIE, "ashiato_session=wrong-channel")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// Scenario: 待ち受けと退避の置き場を明示で渡せば、開発版でもその値を使う
#[test]
fn st54_explicit_bind_and_backup_dir_win_for_dev() {
    assert_eq!(
        net_guard::bind_for(Channel::Dev, Some("127.0.0.1:19999")),
        "127.0.0.1:19999"
    );
    assert_eq!(
        migrate_backup::dir_for(Channel::Dev, Some("custom/backups")),
        "custom/backups"
    );
}
