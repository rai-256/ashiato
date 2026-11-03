// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! サーバに組み込んだ画面の配布と `/api` の境界（ST52 / D13）。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use std::sync::{Arc, Mutex};
use tower::ServiceExt as _;

use super::*;

const TOKEN: &str = "test-token-0123456789abcdef";

#[derive(Clone, Default)]
struct Seen(Arc<Mutex<Vec<access_log::AccessEntry>>>);

impl access_log::AccessSink for Seen {
    fn record(&self, entry: access_log::AccessEntry) -> access_log::AccessFuture {
        self.0.lock().unwrap().push(entry);
        Box::pin(async { Ok(()) })
    }
}

fn request(method: Method, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .unwrap()
}

fn test_router(app: &App) -> axum::Router {
    router_with_assets(
        app.clone(),
        web_assets::Assets::from_static(&[
            (
                "/index.html",
                b"<html>ok</html>".as_slice(),
                "text/html; charset=utf-8",
            ),
            (
                "/assets/app.js",
                b"console.log(1)".as_slice(),
                "text/javascript",
            ),
        ]),
    )
}

async fn response(app: &App, req: Request<Body>) -> axum::response::Response<Body> {
    test_router(app).oneshot(req).await.unwrap()
}

async fn empty_response(app: &App, req: Request<Body>) -> axum::response::Response<Body> {
    router_with_assets(app.clone(), web_assets::Assets::empty())
        .oneshot(req)
        .await
        .unwrap()
}

#[tokio::test]
async fn st52_web_headers() {
    // Scenario: 画面のファイルには、今と同じ守りの見出しが付く
    let app = App::for_test(testdb::pool().await, TOKEN);
    let res = response(&app, request(Method::GET, "/", Body::empty())).await;
    assert_eq!(res.status(), StatusCode::OK);
    let asset = response(&app, request(Method::GET, "/assets/app.js", Body::empty())).await;
    assert_eq!(asset.status(), StatusCode::OK);
    assert_eq!(asset.headers()[header::CONTENT_TYPE], "text/javascript");
    let missing = response(&app, request(Method::GET, "/missing.js", Body::empty())).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(missing.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        res.headers()[header::CONTENT_TYPE],
        "text/html; charset=utf-8"
    );
    assert_eq!(
        res.headers()[header::CONTENT_SECURITY_POLICY],
        "default-src 'self'; frame-ancestors 'none'"
    );
    assert_eq!(res.headers()[header::CACHE_CONTROL], "no-store");
}

#[tokio::test]
async fn st52_web_api_prefix() {
    // Scenario: /api の道は、今の API と同じ応答と認可になる
    let app = App::for_test(testdb::pool().await, TOKEN);
    let prefixed = response(&app, request(Method::GET, "/api/contract", Body::empty())).await;
    let direct = response(&app, request(Method::GET, "/contract", Body::empty())).await;
    assert_eq!(prefixed.status(), direct.status());
    assert_eq!(
        prefixed.into_body().collect().await.unwrap().to_bytes(),
        direct.into_body().collect().await.unwrap().to_bytes()
    );

    let prefixed_health = response(&app, request(Method::GET, "/api/healthz", Body::empty())).await;
    let direct_health = response(&app, request(Method::GET, "/healthz", Body::empty())).await;
    assert_eq!(prefixed_health.status(), StatusCode::OK);
    assert_eq!(direct_health.status(), StatusCode::OK);
    assert_eq!(
        prefixed_health
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes(),
        direct_health
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
    );

    let unauthorized = response(&app, request(Method::GET, "/api/events", Body::empty())).await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    let unauthorized_direct = response(&app, request(Method::GET, "/events", Body::empty())).await;
    assert_eq!(unauthorized_direct.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn st52_web_access_log() {
    // Scenario: /api の道で読んだ画面の読み出しも、今と同じ読み出しの記録が残る
    let seen = Seen::default();
    let password = "st52-web-password";
    let app = App::for_test(testdb::pool().await, TOKEN)
        .with_web_login(password, 0)
        .with_access_sink(seen.clone());
    let login = response(
        &app,
        Request::builder()
            .method(Method::POST)
            .uri("/session")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "password": password }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(login.status(), StatusCode::NO_CONTENT);
    let cookie = login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    for uri in ["/api/events", "/events"] {
        let res = response(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header(header::COOKIE, cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
    }
    let entries: Vec<_> = seen
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|entry| entry.route == "/events")
        .cloned()
        .collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0], entries[1]);
}

#[tokio::test]
async fn st52_web_collector_paths() {
    // Scenario: 収集アプリの送り先の道は、今のまま受ける
    let app = App::for_test(testdb::pool().await, TOKEN);
    let source = testdb::source(&app.pool, "st52-heartbeat", 21_600).await;
    let user = testdb::user();
    let body = Body::from(
        serde_json::json!([{
            "id": uuid::Uuid::new_v4(),
            "user_id": user,
            "logical_source": source,
            "device_id": "st52-device",
            "emitted_at": "2026-05-01T00:00:00Z",
            "capturable": true,
            "blockers": [],
            "attempts": 1,
            "successes": 1,
            "raw": "{\"alive\":true}"
        }])
        .to_string(),
    );
    let prefixed = response(
        &app,
        Request::builder()
            .method(Method::POST)
            .uri("/heartbeat")
            .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .unwrap(),
    )
    .await;
    assert_eq!(prefixed.status(), StatusCode::OK);
}

#[tokio::test]
async fn st52_web_without_assets() {
    // Scenario: 画面を組み込まない組み立てでは、/ は 404 で API は動く
    let app = App::for_test(testdb::pool().await, TOKEN);
    assert_eq!(
        empty_response(&app, request(Method::GET, "/", Body::empty()))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let health = empty_response(&app, request(Method::GET, "/healthz", Body::empty())).await;
    assert_eq!(health.status(), StatusCode::OK);
    assert_eq!(health.into_body().collect().await.unwrap().to_bytes(), "ok");
}

#[tokio::test]
async fn st52_web_assets_only_get_and_head() {
    // 画面のファイルは GET / HEAD だけで返す。他の method は 405、無い道は 404（final review R14）
    let app = App::for_test(testdb::pool().await, TOKEN);
    let head = response(&app, request(Method::HEAD, "/", Body::empty())).await;
    assert_eq!(head.status(), StatusCode::OK);
    assert!(head
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .is_empty());
    for (method, uri) in [
        (Method::POST, "/"),
        (Method::PUT, "/assets/app.js"),
        (Method::DELETE, "/index.html"),
    ] {
        let res = response(&app, request(method.clone(), uri, Body::empty())).await;
        assert_eq!(
            res.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{method} {uri}"
        );
        assert_eq!(res.headers()[header::ALLOW], "GET,HEAD");
        assert_ne!(
            res.headers()
                .get(header::CONTENT_TYPE)
                .map(|v| v.as_bytes()),
            Some(b"text/html; charset=utf-8".as_slice()),
            "{method} {uri} が画面を返した"
        );
    }
    let missing = response(&app, request(Method::POST, "/missing.js", Body::empty())).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}
