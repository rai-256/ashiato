// SPDX-License-Identifier: AGPL-3.0-only
//! 契約の版と確かめる口 `GET /contract` を、router を通して本物の DB で確かめる（ST50 / design D4 / D15 / D23）。
//! 読み出しの記録は「この試験の書く口に届いた行」で数える（共有の表に他の試験の行が混ざるため）。
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use tower::ServiceExt as _;

use super::*;
use crate::access_log::{AccessEntry, AccessFuture, AccessSink, PgAccessSink};
use crate::contract_version::{requested_version, Unreadable, CONTRACT_HEADER};
use crate::satellite_authn_tests::{
    body_string, get_with, login, random_password, seed_credential, seed_satellite,
};
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

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

fn get_bare() -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri("/contract")
        .body(Body::empty())
        .unwrap()
}

// Scenario: 問い合わせの口は受け付ける一番新しい契約の版を答える
#[tokio::test]
async fn st50_contract_answers_version() {
    let (app, _) = tee_app(&random_password()).await;
    let app = app.with_contract_version(3);
    let res = router(app)
        .oneshot(get_with("/contract", TOKEN, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body_string(res).await).unwrap();
    assert_eq!(body, serde_json::json!({ "contract_version": 3 }));
}

#[tokio::test]
async fn st50_contract_default_is_the_constant() {
    let (app, _) = tee_app(&random_password()).await;
    let res = router(app)
        .oneshot(get_with("/contract", TOKEN, None))
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_str(&body_string(res).await).unwrap();
    assert_eq!(body["contract_version"], contract_version::CONTRACT_VERSION);
    assert_eq!(contract_version::CONTRACT_VERSION, 1);
}

// Scenario: 合言葉の無い問い合わせは断られる
#[tokio::test]
async fn st50_contract_requires_token() {
    let (app, _) = tee_app(&random_password()).await;
    let res = router(app.clone()).oneshot(get_bare()).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(!body_string(res).await.contains("contract_version"));
    let res = router(app)
        .oneshot(get_with("/contract", "wrong-token-0123456789abcdef", None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(!body_string(res).await.contains("contract_version"));
}

// Scenario: 画面のログインで問い合わせると 403 で断られる
#[tokio::test]
async fn st50_contract_web_session_403() {
    let password = random_password();
    let (app, _) = tee_app(&password).await;
    let cookie = login(&app, &password).await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/contract")
        .header(
            header::COOKIE,
            format!("{}={cookie}", web_session::COOKIE_NAME),
        )
        .body(Body::empty())
        .unwrap();
    let res = router(app.clone()).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert!(!body_string(res).await.contains("contract_version"));

    // 衛星の資格情報でも 403
    let (satellite, identifier) = seed_satellite(&app.pool).await;
    let (_, value) = seed_credential(&app.pool, satellite, &identifier, chrono::Utc::now()).await;
    let res = router(app)
        .oneshot(get_with("/contract", &value, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert!(!body_string(res).await.contains("contract_version"));
}

// D23（仮）: 合言葉が認められた問い合わせは取り込みの口に入れず、今の規則のまま 1 回 1 行を残す
#[tokio::test]
async fn st50_contract_access_logged() {
    let (app, seen) = tee_app(&random_password()).await;
    let res = router(app)
        .oneshot(get_with("/contract", TOKEN, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "1 回 1 行");
    let e = &seen[0];
    assert_eq!(
        (
            e.credential,
            e.route.as_str(),
            e.method.as_str(),
            e.outcome,
            e.status
        ),
        ("api_token", "/contract", "GET", "ok", 200)
    );
}

#[test]
fn st50_contract_header_reader() {
    let read = |v: Option<&str>| {
        let mut h = HeaderMap::new();
        if let Some(v) = v {
            h.insert(CONTRACT_HEADER, HeaderValue::from_str(v).unwrap());
        }
        requested_version(&h)
    };
    assert_eq!(read(None), Ok(1));
    assert_eq!(read(Some("1")), Ok(1));
    assert_eq!(read(Some("12")), Ok(12));
    for bad in ["0", "-1", "+2", "1.5", "", " 2", "2 ", "abc", "99999999999"] {
        assert_eq!(read(Some(bad)), Err(Unreadable), "{bad:?}");
    }
}
