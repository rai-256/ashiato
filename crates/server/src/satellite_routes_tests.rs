// SPDX-License-Identifier: AGPL-3.0-only
//! 衛星の資格情報では、まだどの口も通らない（ST29 / design D8 / D14）。
//! route は手で列挙せず、契約（`ApiDoc::openapi()`）から取り、`lib.rs` の `.route(` の path と突き合わせる。
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use tower::ServiceExt as _;
use utoipa::OpenApi as _;

use super::*;
use crate::satellite_authn_tests::{body_string, seed_credential, seed_satellite};
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap()
}

/// 資格情報の要らない口（method, path）。
fn is_exempt(method: &Method, path: &str) -> bool {
    path == "/healthz" || (path == "/session" && matches!(*method, Method::POST | Method::DELETE))
}

/// `lib.rs` の router の部分（試験の mod より前）。
fn lib_source() -> &'static str {
    let all = include_str!("lib.rs");
    let end = all
        .find("\n#[cfg(test)]\nmod tests {")
        .expect("lib.rs の試験の mod が見つからない");
    &all[..end]
}

/// `\.route\(\s*"([^"]+)"` と同じ取り方（path が次の行に送られた形も拾う）。
fn routes_in_lib() -> BTreeSet<String> {
    let src = lib_source();
    let mut out = BTreeSet::new();
    for (at, _) in src.match_indices(".route(") {
        let rest = src[at + ".route(".len()..].trim_start();
        if let Some(rest) = rest.strip_prefix('"') {
            let path = &rest[..rest.find('"').unwrap()];
            if !path.is_empty() {
                out.insert(path.to_owned());
            }
        }
    }
    out
}

/// 契約の（method, path）。
fn contract_operations() -> Vec<(Method, String)> {
    let doc = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let mut out = Vec::new();
    for (path, item) in doc["paths"].as_object().unwrap() {
        for method in item.as_object().unwrap().keys() {
            let m = match method.as_str() {
                "get" => Method::GET,
                "post" => Method::POST,
                "put" => Method::PUT,
                "delete" => Method::DELETE,
                "patch" => Method::PATCH,
                other => panic!("知らない method {other}（path {path}）"),
            };
            out.push((m, path.clone()));
        }
    }
    out
}

/// path の `{…}` を形の合う値（uuid）で埋める。
fn fill(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        out.push_str(&uuid::Uuid::new_v4().to_string());
        rest = &rest[rest.find('}').unwrap() + 1..];
    }
    out.push_str(rest);
    out
}

fn request(method: &Method, uri: &str, bearer: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().method(method.clone()).uri(uri);
    if let Some(v) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {v}"));
    }
    if *method == Method::GET || *method == Method::DELETE {
        b.body(Body::empty()).unwrap()
    } else {
        b.header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{}"))
            .unwrap()
    }
}

// Scenario: 衛星の資格情報では、資格情報の要らない口のほかに 2xx を返す口が無い
#[tokio::test]
async fn every_route_refuses_satellite() {
    // 字面から見えない route ができる書き方は、この試験の抽出を広げるまで許さない（D14）
    let src = lib_source();
    for banned in [".merge(", ".nest(", ".nest_service("] {
        assert!(
            !src.contains(banned),
            "lib.rs に {banned} がある。この試験の route の抽出をその Router まで広げる"
        );
    }

    // 契約の path と、lib.rs の `.route(` の path が一致する（/selftest/panic は既定の構成に無い）
    let operations = contract_operations();
    let mut contract: BTreeSet<String> = operations.iter().map(|(_, p)| p.clone()).collect();
    contract.insert("/healthz".into()); // 契約に載せない生存確認
    let mut in_lib = routes_in_lib();
    in_lib.remove("/selftest/panic"); // 既定の構成に無い（環境変数で開けたときだけ生える）
    let only_lib: Vec<_> = in_lib.difference(&contract).collect();
    let only_contract: Vec<_> = contract.difference(&in_lib).collect();
    assert!(
        only_lib.is_empty() && only_contract.is_empty(),
        "lib.rs だけにある path: {only_lib:?} / 契約だけにある path: {only_contract:?}"
    );

    let pool = testdb::pool().await;
    let app = App::for_test(pool.clone(), TOKEN).at(now());
    let (sat, ident) = seed_satellite(&pool).await;
    let (_, value) = seed_credential(&pool, sat, &ident, now()).await;

    let mut called = BTreeSet::new();
    for (method, path) in &operations {
        if is_exempt(method, path) {
            continue;
        }
        let res = router(app.clone())
            .oneshot(request(method, &fill(path), Some(&value)))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "{method} {path}");
        assert_eq!(body_string(res).await, "forbidden", "{method} {path}");
        called.insert((method.to_string(), path.clone()));
    }
    assert!(!called.is_empty());

    // 読み出しの記録に、この route の衛星の 403 の行が 1 行ずつ
    let rows: Vec<(String, String, String, String, i16)> = sqlx::query_as(
        "SELECT route, method, credential, outcome, status FROM core.access_log
          WHERE satellite_id = $1",
    )
    .bind(sat)
    .fetch_all(&pool)
    .await
    .unwrap();
    let mut logged = BTreeSet::new();
    for (route, method, credential, outcome, status) in &rows {
        assert_eq!(
            (credential.as_str(), outcome.as_str(), *status),
            ("satellite", "forbidden", 403),
            "{method} {route}"
        );
        assert!(
            logged.insert((method.clone(), route.clone())),
            "{method} {route} の行が 2 行以上ある"
        );
    }
    assert_eq!(logged, called, "呼んだ口と記録の行が合わない");
    assert_eq!(rows.len(), called.len());
}

// Scenario: 資格情報の要らない口は衛星の資格情報の有無で応答が変わらない
#[tokio::test]
async fn exempt_routes_ignore_satellite() {
    let pool = testdb::pool().await;
    let app = App::for_test(pool.clone(), TOKEN).at(now());
    let (sat, ident) = seed_satellite(&pool).await;
    let (_, value) = seed_credential(&pool, sat, &ident, now()).await;

    let wrong_login = |bearer: Option<&str>| {
        let mut b = Request::builder()
            .method(Method::POST)
            .uri("/session")
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(v) = bearer {
            b = b.header(header::AUTHORIZATION, format!("Bearer {v}"));
        }
        b.body(Body::from(
            serde_json::json!({ "password": format!("wrong-{}", uuid::Uuid::new_v4()) })
                .to_string(),
        ))
        .unwrap()
    };
    type Case = (&'static str, Box<dyn Fn(Option<&str>) -> Request<Body>>);
    let cases: Vec<Case> = vec![
        (
            "GET /healthz",
            Box::new(|b| request(&Method::GET, "/healthz", b)),
        ),
        ("POST /session", Box::new(wrong_login)),
        (
            "DELETE /session",
            Box::new(|b| request(&Method::DELETE, "/session", b)),
        ),
    ];
    for (name, make) in cases {
        let without = router(app.clone()).oneshot(make(None)).await.unwrap();
        let with = router(app.clone())
            .oneshot(make(Some(&value)))
            .await
            .unwrap();
        assert_eq!(with.status(), without.status(), "{name}");
        assert_eq!(
            body_string(with).await,
            body_string(without).await,
            "{name}"
        );
    }
}
