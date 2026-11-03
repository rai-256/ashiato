//! サーバの系統ごとの契約・既定値・画面の印（ST54 Task 2）。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
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

// Scenario: 開発版のサーバは、契約の問い合わせに系統 dev を答える
#[tokio::test]
async fn st54_contract_answers_dev_channel() {
    let response = contract(app(Channel::Dev).await).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Ashiato-Channel"], "dev");
}

// Scenario: 正式版のサーバは、契約の問い合わせに系統 official を答える
#[tokio::test]
async fn st54_contract_answers_official_channel() {
    let response = contract(app(Channel::Official).await).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Ashiato-Channel"], "official");
}

// Scenario: 開発版のサーバの既定は、正式版と別の待ち受け・クッキー・退避の置き場
#[test]
fn st54_dev_defaults_are_separate() {
    assert_ne!(
        net_guard::default_bind(Channel::Dev),
        net_guard::default_bind(Channel::Official)
    );
    assert_ne!(
        web_session::cookie_name(Channel::Dev),
        web_session::cookie_name(Channel::Official)
    );
    assert_ne!(
        migrate_backup::default_dir(Channel::Dev),
        migrate_backup::default_dir(Channel::Official)
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
    let response = router(app).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .starts_with("ashiato_session_dev="));
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

// Scenario: 系統の名前の環境変数を渡しても、系統は変わらない
#[test]
fn st54_channel_environment_does_not_override_build_channel() {
    assert_eq!(
        ashiato_build_stamp::channel("0.1.0-dev.20261102"),
        Channel::Dev
    );
    assert_eq!(ashiato_build_stamp::channel("0.1.0"), Channel::Official);
}
