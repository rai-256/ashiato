// SPDX-License-Identifier: AGPL-3.0-only
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

async fn response(app: &App, req: Request<Body>) -> axum::response::Response<Body> {
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
    .oneshot(req)
    .await
    .unwrap()
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
    let direct = router(app.clone())
        .oneshot(request(Method::GET, "/contract", Body::empty()))
        .await
        .unwrap();
    assert_eq!(prefixed.status(), direct.status());
    assert_eq!(
        prefixed.into_body().collect().await.unwrap().to_bytes(),
        direct.into_body().collect().await.unwrap().to_bytes()
    );

    let unauthorized = response(&app, request(Method::GET, "/api/events", Body::empty())).await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn st52_web_access_log() {
    // Scenario: /api の道で読んだ画面の読み出しも、今と同じ読み出しの記録が残る
    let seen = Seen::default();
    let app = App::for_test(testdb::pool().await, TOKEN).with_access_sink(seen.clone());
    let res = response(&app, request(Method::GET, "/api/events", Body::empty())).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(seen.0.lock().unwrap().last().unwrap().route, "/events");
}

#[tokio::test]
async fn st52_web_collector_paths() {
    // Scenario: 収集アプリの送り先の道は、今のまま受ける
    let app = App::for_test(testdb::pool().await, TOKEN);
    let body = Body::from("[]");
    let prefixed = response(
        &app,
        Request::builder()
            .method(Method::POST)
            .uri("/api/heartbeat")
            .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .unwrap(),
    )
    .await;
    assert_ne!(prefixed.status(), StatusCode::NOT_FOUND);
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
    assert_eq!(
        router(app)
            .oneshot(request(Method::GET, "/healthz", Body::empty()))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}
