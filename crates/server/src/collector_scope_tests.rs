#![allow(clippy::unwrap_used)]
use super::*;
use crate::access_log::{AccessEntry, AccessFuture, AccessSink, PgAccessSink};
use crate::collector_scope::COLLECTOR_ALLOWED;
use crate::testdb;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use std::sync::{Arc, Mutex};
use tower::ServiceExt as _;

const TOKEN: &str = "test-token-0123456789abcdef";
#[derive(Clone)]
struct Tee {
    real: Arc<PgAccessSink>,
    seen: Arc<Mutex<Vec<AccessEntry>>>,
}
impl AccessSink for Tee {
    fn record(&self, e: AccessEntry) -> AccessFuture {
        self.seen.lock().unwrap().push(e.clone());
        self.real.record(e)
    }
}
async fn app() -> (App, Arc<Mutex<Vec<AccessEntry>>>) {
    let pool = testdb::pool().await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let app = App::for_test(pool.clone(), TOKEN).with_access_sink(Tee {
        real: Arc::new(PgAccessSink(pool)),
        seen: seen.clone(),
    });
    (app, seen)
}
fn req(method: Method, uri: &str, body: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"));
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    b.body(body.map_or_else(Body::empty, |value| Body::from(value.to_owned())))
        .unwrap()
}
async fn send(app: &App, r: Request<Body>) -> (StatusCode, String) {
    let res = crate::router(app.clone()).oneshot(r).await.unwrap();
    let s = res.status();
    let b =
        String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    (s, b)
}
fn row(seen: &Arc<Mutex<Vec<AccessEntry>>>, route: &str, method: &str, outcome: &str, status: u16) {
    let r = seen.lock().unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(
        (
            r[0].credential,
            r[0].route.as_str(),
            r[0].method.as_str(),
            r[0].outcome,
            r[0].status
        ),
        ("api_token", route, method, outcome, status)
    );
}
fn cookie_req(cookie: String, uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn st44_scope_allowlist_is_nine() {
    let expected = [
        (Method::POST, "/ingest"),
        (Method::POST, "/heartbeat"),
        (Method::POST, "/drops"),
        (Method::POST, "/media/missing"),
        (Method::PUT, "/media/originals/{sha256}"),
        (Method::GET, "/contract"),
        (Method::HEAD, "/media/originals/{sha256}"),
        (Method::GET, "/feelings/reminder"),
        (Method::GET, "/outages"),
    ];
    assert_eq!(COLLECTOR_ALLOWED.len(), 9);
    assert!(expected.iter().all(|x| COLLECTOR_ALLOWED.contains(x)));
    assert!(COLLECTOR_ALLOWED.iter().all(|x| expected.contains(x)));
    let (app, _) = app().await;
    for (m, u) in expected {
        let (s, _) = send(&app, req(m.clone(), u, None)).await;
        assert_ne!(s, StatusCode::FORBIDDEN, "{m} {u}");
    }
}
#[tokio::test]
async fn st44_scope_sends_pass() {
    // Scenario: API の合言葉で記録・生存信号・破棄の報告・原本の欠けを送れる
    let (app, _) = app().await;
    for u in ["/ingest", "/heartbeat", "/drops", "/media/missing"] {
        let (s, b) = send(&app, req(Method::POST, u, Some("{}"))).await;
        assert_ne!(s, StatusCode::FORBIDDEN, "{u}: {b}");
    }
}
#[tokio::test]
async fn st44_scope_original_put_head() {
    // Scenario: API の合言葉で写真の原本を置き、受け取り済みかを確かめられる
    let tmp = super::media_tests::Tmp::new();
    let root = tmp.made();
    let app = super::media_tests::app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = b"st44-original";
    let sha =
        super::media_tests::photo(&app, user, bytes, "photo.jpg", "2026-10-01T00:00:00Z").await;
    let (put_status, _) = super::media_tests::put(&app, &sha, user, bytes).await;
    assert_eq!(put_status, StatusCode::CREATED);
    assert_eq!(
        super::media_tests::head(&app, &sha, user).await,
        StatusCode::OK
    );
    assert!(
        root.join("2026/10").exists(),
        "原本が $TMPDIR の置き場にある"
    );
}
#[tokio::test]
async fn st44_scope_contract() {
    // Scenario: API の合言葉で送信契約の版を確かめられる
    let (app, seen) = app().await;
    let (s, b) = send(&app, req(Method::GET, "/contract", None)).await;
    assert_eq!(s, StatusCode::OK);
    assert!(b.contains("contract_version"));
    row(&seen, "/contract", "GET", "ok", 200);
}
#[tokio::test]
async fn st44_scope_feelings_reminder() {
    // Scenario: API の合言葉で気分の通知の判断を読める
    // Scenario: 収集アプリの資格情報で通知の判断を読める
    let (app, seen) = app().await;
    let (s, _) = send(
        &app,
        req(Method::GET, "/feelings/reminder?date=2026-10-01", None),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    row(&seen, "/feelings/reminder", "GET", "ok", 200);
}
#[tokio::test]
async fn st44_scope_feelings_reminder_post_403() {
    // Scenario: 収集アプリの資格情報では通知の時刻を変えられない
    let (app, seen) = app().await;
    let (s, b) = send(
        &app,
        req(
            Method::POST,
            "/feelings/reminder",
            Some(r#"{"at":"09:00"}"#),
        ),
    )
    .await;
    assert_eq!((s, b.as_str()), (StatusCode::FORBIDDEN, "forbidden"));
    row(&seen, "/feelings/reminder", "POST", "forbidden", 403);
}
#[tokio::test]
async fn st44_scope_outages() {
    // Scenario: API の合言葉で途絶の判定を読める
    let (app, seen) = app().await;
    let (s, b) = send(&app, req(Method::GET, "/outages", None)).await;
    assert_eq!(s, StatusCode::OK);
    assert!(b.contains("judged_at"));
    row(&seen, "/outages", "GET", "ok", 200);
}
#[tokio::test]
async fn st44_scope_feelings_403() {
    // Scenario: API の合言葉では気分の中身は読めない
    let (app, seen) = app().await;
    let (s, b) = send(&app, req(Method::GET, "/feelings", None)).await;
    assert_eq!((s, b.as_str()), (StatusCode::FORBIDDEN, "forbidden"));
    row(&seen, "/feelings", "GET", "forbidden", 403);
}
#[tokio::test]
async fn st44_scope_events_403_logged() {
    // Scenario: API の合言葉では記録を送れるが読めない
    // Scenario: API の合言葉で断った読み出しは、API の合言葉・403 の行で残る
    let (app, seen) = app().await;
    let (s, b) = send(&app, req(Method::GET, "/events", None)).await;
    assert_eq!((s, b.as_str()), (StatusCode::FORBIDDEN, "forbidden"));
    row(&seen, "/events", "GET", "forbidden", 403);
}
#[tokio::test]
async fn st44_scope_session_get_403() {
    // Scenario: API の合言葉ではログインの状態も読めない
    let (app, seen) = app().await;
    let (s, b) = send(&app, req(Method::GET, "/session", None)).await;
    assert_eq!((s, b.as_str()), (StatusCode::FORBIDDEN, "forbidden"));
    row(&seen, "/session", "GET", "forbidden", 403);
}
#[tokio::test]
async fn st44_scope_both_headers_403() {
    // Scenario: API の合言葉とログインの印を両方付けた読み出しは断られる
    let (app, seen) = app().await;
    let c = testdb::web_cookie(&app, app.web_password_for_test()).await;
    seen.lock().unwrap().clear();
    let mut r = req(Method::GET, "/events", None);
    r.headers_mut().insert(header::COOKIE, c.parse().unwrap());
    let (s, b) = send(&app, r).await;
    assert_eq!((s, b.as_str()), (StatusCode::FORBIDDEN, "forbidden"));
    row(&seen, "/events", "GET", "forbidden", 403);
}
#[tokio::test]
async fn st44_scope_cookie_only_reads() {
    // Scenario: ログインの印だけを付けた読み出しは通る
    let (app, seen) = app().await;
    let c = testdb::web_cookie(&app, app.web_password_for_test()).await;
    seen.lock().unwrap().clear();
    let (s, _) = send(&app, cookie_req(c, "/events")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(seen.lock().unwrap()[0].credential, "web_session");
}
#[tokio::test]
async fn st44_scope_web_session_unchanged() {
    // Scenario: 画面のログインは許可の一覧の外の口も通る
    let (app, _) = app().await;
    let c = testdb::web_cookie(&app, app.web_password_for_test()).await;
    for u in [
        "/stays",
        "/events",
        "/feelings?from=2026-10-01&to=2026-10-01",
    ] {
        assert_ne!(
            send(&app, cookie_req(c.clone(), u)).await.0,
            StatusCode::FORBIDDEN
        );
    }
}
#[tokio::test]
async fn st44_scope_wrong_bearer_with_cookie() {
    // Scenario: 一致しない Bearer とログインの印を両方付けた読み出しは印で通る
    let (app, seen) = app().await;
    let c = testdb::web_cookie(&app, app.web_password_for_test()).await;
    seen.lock().unwrap().clear();
    let mut r = cookie_req(c, "/events");
    r.headers_mut()
        .insert(header::AUTHORIZATION, "Bearer wrong-token".parse().unwrap());
    assert_eq!(send(&app, r).await.0, StatusCode::OK);
    assert_eq!(seen.lock().unwrap()[0].credential, "web_session");
}
#[tokio::test]
async fn st44_scope_allowed_reads_logged() {
    // Scenario: API の合言葉で通知に要る読みをすると、読み出しの記録に 1 行残る
    let (app, seen) = app().await;
    for (u, r) in [
        ("/contract", "/contract"),
        ("/feelings/reminder?date=2026-10-01", "/feelings/reminder"),
        ("/outages", "/outages"),
    ] {
        seen.lock().unwrap().clear();
        assert_eq!(
            send(&app, req(Method::GET, u, None)).await.0,
            StatusCode::OK
        );
        row(&seen, r, "GET", "ok", 200);
    }
}
