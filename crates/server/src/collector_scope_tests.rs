#![allow(clippy::unwrap_used)]
use super::*;
use crate::access_log::{AccessEntry, AccessFuture, AccessSink, PgAccessSink};
use crate::collector_scope::COLLECTOR_ALLOWED;
use crate::satellite_routes_tests::is_exempt as satellite_is_exempt;
use crate::testdb;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use std::sync::{Arc, Mutex};
use tower::ServiceExt as _;
use utoipa::OpenApi as _;

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

async fn isolated_app() -> (testdb::TestDb, App, Arc<Mutex<Vec<AccessEntry>>>) {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let app = App::for_test(pool.clone(), TOKEN).with_access_sink(Tee {
        real: Arc::new(PgAccessSink(pool)),
        seen: seen.clone(),
    });
    (db, app, seen)
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

fn no_credentials_req(method: &Method, uri: &str, body: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().method(method.clone()).uri(uri);
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    b.body(body.map_or_else(Body::empty, |value| Body::from(value.to_owned())))
        .unwrap()
}

fn contract_operations() -> Vec<(Method, String)> {
    let doc = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let mut out = Vec::new();
    for (path, item) in doc["paths"].as_object().unwrap() {
        for method in item.as_object().unwrap().keys() {
            let method = match method.as_str() {
                "get" => Method::GET,
                "post" => Method::POST,
                "put" => Method::PUT,
                "delete" => Method::DELETE,
                "patch" => Method::PATCH,
                "head" => Method::HEAD,
                "options" => Method::OPTIONS,
                "trace" => panic!("trace の口は試験していない（path {path}）"),
                _ => continue,
            };
            out.push((method, path.clone()));
        }
    }
    out
}

fn fill(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let close = rest[open..].find('}').unwrap() + open;
        let name = &rest[open + 1..close];
        out.push_str(match name {
            "sha256" => "0000000000000000000000000000000000000000000000000000000000000000",
            "id" | "cid" => "00000000-0000-0000-0000-000000000000",
            _ => panic!("形を埋める値が無い path parameter: {name}"),
        });
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

fn is_exempt(method: &Method, path: &str) -> bool {
    path == "/healthz" || (path == "/session" && matches!(*method, Method::POST | Method::DELETE))
}

async fn protected_row_counts(pool: &sqlx::PgPool) -> Vec<i64> {
    let tables = [
        "core.event",
        "core.stay_criteria",
        "core.stay_absorbed",
        "core.place",
        "core.attribute_kind",
        "core.attribute_kind_name",
        "core.satellite",
        "core.satellite_credential",
        "core.export_ledger",
    ];
    let mut counts = Vec::with_capacity(tables.len());
    for table in tables {
        counts.push(
            sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) FROM {table}"))
                .fetch_one(pool)
                .await
                .unwrap(),
        );
    }
    counts
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

#[tokio::test]
async fn st44_routes_without_credentials() {
    // Scenario: 資格情報を求めない口は生存確認・ログイン・ログアウトの 3 つだけ
    let (app, _) = app().await;
    let mut operations = contract_operations();
    operations.extend([
        (Method::GET, "/healthz".to_owned()),
        (Method::POST, "/session".to_owned()),
        (Method::DELETE, "/session".to_owned()),
    ]);

    for (method, path) in operations {
        let body = (method == Method::POST && path == "/session").then_some("{}");
        let (status, _) = send(&app, no_credentials_req(&method, &fill(&path), body)).await;
        let expected = match (method.clone(), path.as_str()) {
            (Method::GET, "/healthz") => StatusCode::OK,
            (Method::DELETE, "/session") => StatusCode::NO_CONTENT,
            _ => StatusCode::UNAUTHORIZED,
        };
        assert_eq!(status, expected, "{method} {path}");
    }
}

#[tokio::test]
async fn st44_routes_api_token_outside_403() {
    // Scenario: API の合言葉では許可の一覧の外の口はすべて断られる
    let (_db, app, seen) = isolated_app().await;
    let pool = app.pool.clone();
    let before = protected_row_counts(&pool).await;
    let operations = contract_operations();
    let contract: std::collections::BTreeSet<_> = operations.iter().cloned().collect();
    let allowed: std::collections::BTreeSet<_> = COLLECTOR_ALLOWED
        .iter()
        .map(|(method, path)| (method.clone(), (*path).to_owned()))
        .collect();

    assert!(COLLECTOR_ALLOWED
        .iter()
        .all(|(method, path)| contract.contains(&(method.clone(), (*path).to_owned()))));
    let exemptions = [
        (Method::GET, "/healthz"),
        (Method::POST, "/session"),
        (Method::DELETE, "/session"),
    ];
    assert_eq!(
        exemptions
            .iter()
            .filter(|(method, path)| is_exempt(method, path))
            .count(),
        3
    );
    assert!(exemptions
        .iter()
        .all(|(method, path)| is_exempt(method, path) && satellite_is_exempt(method, path)));
    assert!(contract_operations()
        .into_iter()
        .filter(|(method, path)| !exemptions
            .iter()
            .any(|(exempt_method, exempt_path)| method == exempt_method && path == exempt_path))
        .all(|(method, path)| {
            !is_exempt(&method, &path) && !satellite_is_exempt(&method, &path)
        }));

    let mut called = std::collections::BTreeSet::new();
    for (method, path) in operations {
        if is_exempt(&method, &path) || allowed.contains(&(method.clone(), path.clone())) {
            continue;
        }
        let (status, body) = send(&app, req(method.clone(), &fill(&path), Some("{}"))).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
        if method != Method::HEAD {
            assert_eq!(body, "forbidden", "{method} {path}");
        }
        called.insert((method.to_string(), path));
    }
    assert!(!called.is_empty());
    let after = protected_row_counts(&pool).await;
    assert_eq!(before, after);

    let rows = seen.lock().unwrap();
    let logged: std::collections::BTreeSet<_> = rows
        .iter()
        .map(|entry| (entry.method.clone(), entry.route.clone()))
        .collect();
    assert_eq!(rows.len(), called.len());
    assert_eq!(logged, called);
    assert!(rows.iter().all(|entry| {
        entry.credential == "api_token" && entry.outcome == "forbidden" && entry.status == 403
    }));
}

#[tokio::test]
async fn st44_routes_allowlist_in_contract() {
    let contract: std::collections::BTreeSet<_> = contract_operations().into_iter().collect();
    let allowed: std::collections::BTreeSet<_> = COLLECTOR_ALLOWED
        .iter()
        .map(|(method, path)| (method.clone(), (*path).to_owned()))
        .collect();
    assert_eq!(allowed.len(), 9);
    assert!(allowed.is_subset(&contract));
}
