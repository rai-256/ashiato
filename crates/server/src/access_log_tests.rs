// SPDX-License-Identifier: AGPL-3.0-only
//! 読み出しの記録（`core.access_log`）と応答の `Cache-Control` を、router を通して本物の DB で確かめる
//! （ST28 / design D8 / D10 / C4 / C5）。合言葉は試験の中で乱数から作る。
//!
//! 開発 DB は他の試験と共有で、router を通る求めは全部 1 行を足す。**行の数の増減は
//! 「この試験の書く口に届いた行」（`Tee`）で数える**。DB の全列を読む試験だけが表を直に読む。
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{header, Method, Request, Response};
use http_body_util::BodyExt as _;
use tower::ServiceExt as _;

use super::*;
use crate::access_log::{AccessEntry, AccessFuture, AccessSink, PgAccessSink};
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

/// 本物の書く口へ渡しつつ、この試験に届いた行を手元にも残す。
#[derive(Clone)]
struct Tee {
    real: Arc<PgAccessSink>,
    seen: Arc<Mutex<Vec<AccessEntry>>>,
}

impl AccessSink for Tee {
    fn record(&self, entry: AccessEntry) -> AccessFuture {
        self.seen.lock().unwrap().push(entry.clone());
        self.real.record(entry)
    }
}

/// 常に `Err` を返す偽の書く口（表の権限は触らない。design D8）。
struct FailingSink;

impl AccessSink for FailingSink {
    fn record(&self, _: AccessEntry) -> AccessFuture {
        Box::pin(async { Err(sqlx::Error::PoolClosed) })
    }
}

fn random_password() -> String {
    format!("pw-{}", uuid::Uuid::new_v4())
}

async fn tee_app(password: &str) -> (App, Arc<Mutex<Vec<AccessEntry>>>) {
    let pool = testdb::pool().await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let tee = Tee {
        real: Arc::new(PgAccessSink(pool.clone())),
        seen: seen.clone(),
    };
    let app = App::for_test(pool, TOKEN)
        .with_web_login(password, 0)
        .with_access_sink(tee);
    (app, seen)
}

async fn send(app: &App, req: Request<Body>) -> Response<Body> {
    router(app.clone()).oneshot(req).await.unwrap()
}

fn login_req(password: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/session")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({ "password": password }).to_string(),
        ))
        .unwrap()
}

/// ログインして印の値を返す。
async fn login(app: &App, password: &str) -> String {
    let res = send(app, login_req(password)).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let set = res.headers()[header::SET_COOKIE].to_str().unwrap();
    set.strip_prefix(&format!("{}=", web_session::COOKIE_NAME))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

fn get_with_cookie(uri: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(
            header::COOKIE,
            format!("{}={cookie}", web_session::COOKIE_NAME),
        )
        .body(Body::empty())
        .unwrap()
}

fn get_bare(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn bearer(method: Method, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap()
}

fn entry(
    credential: &'static str,
    route: &str,
    method: &str,
    outcome: &'static str,
    status: u16,
) -> AccessEntry {
    AccessEntry {
        via: "direct",
        credential,
        route: route.into(),
        method: method.into(),
        outcome,
        status,
    }
}

async fn body_string(res: Response<Body>) -> String {
    String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap()
}

/// Scenario: 記録の読み出しで読み出しの記録に 1 行増える
#[tokio::test]
async fn access_log_middleware_read_adds_one_row() {
    let password = random_password();
    let (app, seen) = tee_app(&password).await;
    let pool = testdb::pool().await;
    let t0: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let cookie = login(&app, &password).await;
    seen.lock().unwrap().clear();

    let res = send(&app, get_with_cookie("/events", &cookie)).await;
    assert_eq!(res.status(), StatusCode::OK);

    assert_eq!(
        *seen.lock().unwrap(),
        vec![entry("web_session", "/events", "GET", "ok", 200)],
        "読み出し 1 回で、画面のログイン・その口の行が 1 行"
    );
    let stored: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.access_log
          WHERE at >= $1 AND credential = 'web_session' AND route = '/events'
            AND method = 'GET' AND outcome = 'ok' AND status = 200 AND via = 'direct'",
    )
    .bind(t0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(stored >= 1, "DB の表に行が入っている");
}

/// Scenario: 断られた求めも読み出しの記録に残る
#[tokio::test]
async fn access_log_middleware_refused_requests_are_logged() {
    let password = random_password();
    let (app, seen) = tee_app(&password).await;

    let res = send(&app, get_bare("/events")).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = send(&app, login_req(&random_password())).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            entry("none", "/events", "GET", "unauthorized", 401),
            entry("none", "/session", "POST", "login_failed", 401),
        ]
    );
}

/// Scenario: 取り込みは読み出しの記録に残らない
#[tokio::test]
async fn access_log_middleware_ingest_is_not_logged() {
    let (app, seen) = tee_app(&random_password()).await;
    let source = testdb::source(&app.pool, "accesslog", 21_600).await;
    let body = serde_json::json!([{
        "id": uuid::Uuid::new_v4(),
        "user_id": testdb::user(),
        "logical_source": source,
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": "2026-03-01T01:00:00Z",
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": r#"{"a":1}"#,
        "payload": {},
    }]);

    let res = send(
        &app,
        bearer(Method::POST, "/ingest", Body::from(body.to_string())),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK, "取り込まれる");

    assert!(seen.lock().unwrap().is_empty(), "行は増えない");
}

/// Scenario: 読み出しの記録には中身も合言葉も残らない
#[tokio::test]
async fn access_log_middleware_keeps_no_content_or_secret() {
    let password = random_password();
    let (app, seen) = tee_app(&password).await;
    let pool = testdb::pool().await;
    let t0: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let arg = uuid::Uuid::new_v4().to_string();
    let wrong = random_password();

    let cookie = login(&app, &password).await;
    let res = send(
        &app,
        get_with_cookie(&format!("/stays?user_id={arg}&day={arg}"), &cookie),
    )
    .await;
    assert!(res.status().as_u16() > 0);
    let res = send(&app, login_req(&wrong)).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = send(
        &app,
        Request::builder()
            .method(Method::DELETE)
            .uri("/session")
            .header(
                header::COOKIE,
                format!("{}={cookie}", web_session::COOKIE_NAME),
            )
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let mut all: Vec<String> = seen
        .lock()
        .unwrap()
        .iter()
        .map(|e| format!("{e:?}"))
        .collect();
    // 表の全列を文字列にして読む（他の試験の行も混ざるが、乱数の値は他に無い）
    all.extend(
        sqlx::query_scalar::<_, String>("SELECT a::text FROM core.access_log a WHERE at >= $1")
            .bind(t0)
            .fetch_all(&pool)
            .await
            .unwrap(),
    );
    assert!(
        seen.lock().unwrap().iter().any(|e| e.route == "/stays"),
        "引数付きの読み出しが行になっている"
    );
    for text in &all {
        for secret in [&arg, &password, &wrong, &cookie, &TOKEN.to_owned()] {
            assert!(!text.contains(secret.as_str()), "行に値が残っている");
        }
    }
}

/// `/session` の method と分岐ごとに、何が 1 行として残るかの一覧（D8）。
/// 印は置かない（Scenario の印は別の試験が持つ）。
#[tokio::test]
async fn access_log_middleware_session_routes_each_write_one_row() {
    let password = random_password();
    let (app, seen) = tee_app(&password).await;
    let take = || std::mem::take(&mut *seen.lock().unwrap());

    let res = send(&app, get_bare("/session")).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        take(),
        vec![entry("none", "/session", "GET", "unauthorized", 401)]
    );

    let cookie = login(&app, &password).await;
    assert_eq!(
        take(),
        vec![entry("none", "/session", "POST", "login_ok", 204)]
    );

    let res = send(&app, get_with_cookie("/session", &cookie)).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        take(),
        vec![entry("web_session", "/session", "GET", "ok", 200)]
    );

    let res = send(&app, bearer(Method::GET, "/session", Body::empty())).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        take(),
        vec![entry("api_token", "/session", "GET", "ok", 200)]
    );

    let logout = |c: Option<&str>| {
        let mut b = Request::builder().method(Method::DELETE).uri("/session");
        if let Some(c) = c {
            b = b.header(header::COOKIE, format!("{}={c}", web_session::COOKIE_NAME));
        }
        b.body(Body::empty()).unwrap()
    };
    let res = send(&app, logout(Some(&cookie))).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        take(),
        vec![entry("web_session", "/session", "DELETE", "logout", 204)]
    );
    let res = send(&app, logout(None)).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        take(),
        vec![entry("none", "/session", "DELETE", "logout", 204)]
    );

    // 失敗が溜まると 429（login_throttled）。閾値は web_session の THROTTLE_MAX_FAILURES
    for _ in 0..10 {
        send(&app, login_req(&random_password())).await;
    }
    take();
    let res = send(&app, login_req(&password)).await;
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        take(),
        vec![entry("none", "/session", "POST", "login_throttled", 429)]
    );
}

/// Scenario: 読み出しの記録に書けないときは記録を返さない
#[tokio::test]
async fn access_log_middleware_write_failure_returns_no_records() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "accesslogfail", 21_600).await;
    let marker = format!("marker-{}", uuid::Uuid::new_v4());
    let seed = App::for_test(pool.clone(), TOKEN);
    let body = serde_json::json!([{
        "id": uuid::Uuid::new_v4(),
        "user_id": testdb::user(),
        "logical_source": source,
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": "2026-03-01T01:00:00Z",
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": format!(r#"{{"m":"{marker}"}}"#),
        "payload": {},
    }]);
    let res = send(
        &seed,
        bearer(Method::POST, "/ingest", Body::from(body.to_string())),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let app = seed.with_access_sink(FailingSink);
    let res = send(&app, bearer(Method::GET, "/events", Body::empty())).await;
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        !body_string(res).await.contains(&marker),
        "応答に記録が含まれない"
    );
}

/// Scenario: 読み出しの記録は記録の読み出しに出ない
#[tokio::test]
async fn access_log_middleware_rows_are_not_in_event_reads() {
    let (app, _) = tee_app(&random_password()).await;
    let marker = format!("/marker-{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO core.access_log (via, credential, route, method, outcome, status)
         VALUES ('direct', 'none', $1, 'GET', 'ok', 200)",
    )
    .bind(&marker)
    .execute(&app.pool)
    .await
    .unwrap();

    let res = send(&app, bearer(Method::GET, "/events", Body::empty())).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!body_string(res).await.contains(&marker));
}

/// Scenario: サーバの応答は写しを保存させない
#[tokio::test]
async fn response_no_store_server_every_response() {
    let password = random_password();
    let (app, _) = tee_app(&password).await;
    let source = testdb::source(&app.pool, "nostore", 21_600).await;
    let cookie = login(&app, &password).await;

    let ingest = bearer(
        Method::POST,
        "/ingest",
        Body::from(
            serde_json::json!([{
                "id": uuid::Uuid::new_v4(),
                "user_id": testdb::user(),
                "logical_source": source,
                "external_id": null,
                "device_id": "test-dev",
                "origin": "collected",
                "event_time": "2026-03-01T01:00:00Z",
                "tz_offset_min": 540,
                "tz_id": "Asia/Tokyo",
                "schema_version": 1,
                "raw": r#"{"a":1}"#,
                "payload": {},
            }])
            .to_string(),
        ),
    );
    let logout = Request::builder()
        .method(Method::DELETE)
        .uri("/session")
        .header(
            header::COOKIE,
            format!("{}={cookie}", web_session::COOKIE_NAME),
        )
        .body(Body::empty())
        .unwrap();
    let responses = [
        (
            "読み出し",
            send(&app, get_with_cookie("/events", &cookie)).await,
            200,
        ),
        ("取り込み", send(&app, ingest).await, 200),
        ("ログイン", send(&app, login_req(&password)).await, 204),
        ("ログアウト", send(&app, logout).await, 204),
        ("断られた求め", send(&app, get_bare("/events")).await, 401),
    ];
    for (what, res, status) in responses {
        assert_eq!(res.status().as_u16(), status, "{what}");
        let cc = res.headers()[header::CACHE_CONTROL].to_str().unwrap();
        assert!(cc.contains("no-store"), "{what}: Cache-Control = {cc}");
    }
}
