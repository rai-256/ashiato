// SPDX-License-Identifier: AGPL-3.0-only
//! 衛星の登録・外す・一覧の口（ST29 / design D7 / D11 / D16）。本物の DB と router で確かめる。
//! 共有の DB に他の試験の衛星が混ざるので、識別子と名前には試験ごとの接尾を付けて隔離する。
//! 最初の 2 つを外す試験は 409 だけを見て、状態を変えない（D14）。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt as _;

use super::*;
use crate::satellite_authn_tests::{body_string, login, random_password, seed_credential};
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap()
}

struct Ctx {
    pool: sqlx::PgPool,
    app: App,
    cookie: String,
    tag: String,
}

impl Ctx {
    async fn new() -> Self {
        let pool = testdb::pool().await;
        let password = random_password();
        let app = App::for_test(pool.clone(), TOKEN)
            .with_web_login(&password, 0)
            .at(now());
        let cookie = login(&app, &password).await;
        let tag = uuid::Uuid::new_v4().simple().to_string();
        Self {
            pool,
            app,
            cookie,
            tag,
        }
    }

    /// 試験ごとの識別子（`dev.t<乱数>.<part>`）と名前。
    fn identifier(&self, part: &str) -> String {
        format!("dev.t{}.{part}", self.tag)
    }

    fn name(&self, part: &str) -> String {
        format!("{part} {}", &self.tag[..12])
    }

    /// 画面のログイン付きで求める。
    async fn web(&self, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, String) {
        send(&self.app, method, uri, Some(&self.cookie), None, body).await
    }

    async fn register(&self, name: &str, identifier: &str) -> (StatusCode, String) {
        self.web(
            Method::POST,
            "/satellites",
            Some(json!({ "name": name, "identifier": identifier })),
        )
        .await
    }

    async fn list(&self) -> Vec<Value> {
        let (status, body) = self.web(Method::GET, "/satellites", None).await;
        assert_eq!(status, StatusCode::OK);
        serde_json::from_str(&body).unwrap()
    }

    /// この試験の接尾を持つ衛星だけ（並走する試験の行は混ざらない）。
    async fn mine(&self) -> Vec<Value> {
        self.list()
            .await
            .into_iter()
            .filter(|s| s["identifier"].as_str().unwrap().contains(&self.tag))
            .collect()
    }

    /// 断った識別子の衛星が一覧に無い（最初の 2 つは元からあるので `except` で除く）。
    async fn assert_none_of(&self, ids: &[&str], except: &[&str]) {
        for s in self.list().await {
            let id = s["identifier"].as_str().unwrap();
            assert!(
                !ids.contains(&id) || except.contains(&id),
                "断った識別子が一覧にある: {id}"
            );
        }
    }

    /// この識別子の衛星を一覧から探す。
    async fn find(&self, identifier: &str) -> Option<Value> {
        self.list()
            .await
            .into_iter()
            .find(|s| s["identifier"] == identifier)
    }
}

async fn send(
    app: &App,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, String) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, format!("{}={c}", web_session::COOKIE_NAME));
    }
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(v) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string())),
        None => b.body(Body::empty()),
    }
    .unwrap();
    let res = router(app.clone()).oneshot(req).await.unwrap();
    (res.status(), body_string(res).await)
}

fn kind(body: &str) -> String {
    serde_json::from_str::<Value>(body).unwrap()["error"]
        .as_str()
        .unwrap()
        .to_owned()
}

// Scenario: 形に合う識別子で衛星を登録できる
#[tokio::test]
async fn satellite_register_valid_identifier_appears_in_list() {
    let c = Ctx::new().await;
    let (id, name) = (c.identifier("booklog"), c.name("読書ログ"));

    let (status, body) = c.register(&name, &id).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let s = c.find(&id).await.expect("一覧に出る");
    assert_eq!(s["name"], name.as_str());
    assert_eq!(s["removed_at"], Value::Null);
    assert_eq!(s["credentials"], json!([]));
    assert!(s["registered_at"].is_string());
    assert_eq!(s["builtin"], false);
}

// Scenario: 形に合わない識別子の衛星は登録されない
#[tokio::test]
async fn satellite_register_malformed_identifier_is_refused() {
    let c = Ctx::new().await;
    let long = format!("dev.{}", "a".repeat(61));
    let ids = [
        "booklog",
        "Dev.example.booklog",
        "dev..booklog",
        "dev.example.",
        "dev_example.booklog",
        "dev.example:booklog",
        long.as_str(),
    ];
    for id in ids {
        let (status, body) = c.register(&c.name("形"), id).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{id}");
        assert_eq!(kind(&body), "malformed", "{id}");
    }
    c.assert_none_of(&ids, &[]).await;
    assert!(c.mine().await.is_empty(), "名前の行も増えていない");
}

// Scenario: 本体が使う識別子の衛星は登録されない
#[tokio::test]
async fn satellite_register_builtin_identifier_is_refused() {
    let c = Ctx::new().await;
    for id in ["ashiato.booklog", "ashiato.ai-external"] {
        let (status, body) = c.register(&c.name("本体"), id).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{id}");
        assert_eq!(kind(&body), "reserved", "{id}");
    }
    c.assert_none_of(&["ashiato.booklog"], &[]).await;
    let external = c.find("ashiato.ai-external").await.unwrap();
    assert_eq!(external["name"], "外部 AI", "最初の衛星は変わらない");
}

// Scenario: 本体の前置きの形で始まる識別子の衛星は登録されない
#[tokio::test]
async fn satellite_register_body_prefix_identifier_is_refused() {
    let c = Ctx::new().await;
    let ids = ["c03-takeout.example", "s01-x.example"];
    for id in ids {
        let (status, body) = c.register(&c.name("前置き"), id).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{id}");
        assert_eq!(kind(&body), "body_prefix", "{id}");
    }
    c.assert_none_of(&ids, &[]).await;
}

// Scenario: 64 文字の識別子の衛星は登録できる
#[tokio::test]
async fn satellite_register_64_char_identifier_is_accepted() {
    let c = Ctx::new().await;
    // `dev.t<32 桁>.` の後ろを埋めて全体を 64 文字にする
    let head = format!("dev.t{}.", c.tag);
    let id = format!("{head}{}", "a".repeat(64 - head.len()));
    assert_eq!(id.len(), 64);

    let (status, body) = c.register(&c.name("長い"), &id).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(c.find(&id).await.is_some());
}

// Scenario: 登録している衛星と同じ識別子の衛星は登録されない
#[tokio::test]
async fn satellite_register_same_identifier_is_refused() {
    let c = Ctx::new().await;
    let (id, first) = (c.identifier("booklog"), c.name("一度目"));
    assert_eq!(c.register(&first, &id).await.0, StatusCode::CREATED);

    let (status, body) = c.register(&c.name("二度目"), &id).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(kind(&body), "identifier_taken");

    let same: Vec<Value> = c
        .list()
        .await
        .into_iter()
        .filter(|s| s["identifier"] == id.as_str())
        .collect();
    assert_eq!(same.len(), 1);
    assert_eq!(same[0]["name"], first.as_str());
}

// Scenario: 登録している衛星と同じ名前の衛星は登録されない
#[tokio::test]
async fn satellite_register_same_name_is_refused() {
    let c = Ctx::new().await;
    let other = c.identifier("other");
    for name in ["外部 AI", " 外部 AI "] {
        let (status, body) = c.register(name, &other).await;
        assert_eq!(status, StatusCode::CONFLICT, "{name:?}");
        assert_eq!(kind(&body), "name_taken", "{name:?}");
    }
    assert!(c.find(&other).await.is_none());
}

// Scenario: 名前が空白だけか 41 文字の衛星は登録されない
#[tokio::test]
async fn satellite_register_blank_or_long_name_is_refused() {
    let c = Ctx::new().await;
    let other = c.identifier("other");
    for name in ["   ".to_owned(), "あ".repeat(41)] {
        let (status, body) = c.register(&name, &other).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(kind(&body), "name_invalid");
    }
    assert!(c.find(&other).await.is_none());

    let padded = format!(" {} ", c.name("家計簿"));
    let (status, body) = c.register(&padded, &other).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(c.find(&other).await.unwrap()["name"], padded.trim());
}

// Scenario: 最初の AI の 2 つは外せない
#[tokio::test]
async fn satellite_register_builtin_satellites_cannot_be_removed() {
    let c = Ctx::new().await;
    for identifier in ["ashiato.ai-external", "ashiato.ai-local"] {
        let s = c.find(identifier).await.expect("最初から登録されている");
        let (status, body) = c
            .web(
                Method::DELETE,
                &format!("/satellites/{}", s["id"].as_str().unwrap()),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT, "{identifier}");
        assert_eq!(kind(&body), "builtin", "{identifier}");
        assert_eq!(
            c.find(identifier).await.unwrap()["removed_at"],
            Value::Null,
            "外されていないまま"
        );
    }
}

/// 外すと資格情報が全部止まり、同じ識別子で登録し直すと同じ行が戻る（D7 / Q2）。
#[tokio::test]
async fn satellite_register_removed_identifier_returns_same_row() {
    let c = Ctx::new().await;
    let (id, first) = (c.identifier("again"), c.name("最初"));
    assert_eq!(c.register(&first, &id).await.0, StatusCode::CREATED);
    let before = c.find(&id).await.unwrap();
    let sat: uuid::Uuid = before["id"].as_str().unwrap().parse().unwrap();
    let (cred, _) = seed_credential(&c.pool, sat, &id, now()).await;

    let uri = format!("/satellites/{sat}");
    assert_eq!(
        c.web(Method::DELETE, &uri, None).await.0,
        StatusCode::NO_CONTENT
    );
    let revoked: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT revoked_at FROM core.satellite_credential WHERE id = $1")
            .bind(cred)
            .fetch_one(&c.pool)
            .await
            .unwrap();
    assert_eq!(revoked, Some(now()), "外した時刻で取り消される");
    let removed = c.find(&id).await.unwrap();
    assert!(removed["removed_at"].is_string());
    assert_eq!(removed["credentials"], json!([]), "取り消したものは出ない");
    let (status, body) = c.web(Method::DELETE, &uri, None).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "外した衛星をもう一度は外せない"
    );
    assert_eq!(kind(&body), "removed");
    let missing = format!("/satellites/{}", uuid::Uuid::new_v4());
    assert_eq!(
        c.web(Method::DELETE, &missing, None).await.0,
        StatusCode::NOT_FOUND
    );

    let second = c.name("再登録");
    let (status, body) = c.register(&second, &id).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let back = c.find(&id).await.unwrap();
    assert_eq!(back["id"], before["id"], "同じ行");
    assert_eq!(back["name"], second.as_str(), "名前は求めの名前");
    assert_eq!(back["registered_at"], before["registered_at"]);
    assert_eq!(back["removed_at"], Value::Null);
    assert_eq!(back["credentials"], json!([]), "資格情報は戻らない");
}

// Scenario: API の合言葉では衛星を登録できない
// （発行と取り消しの口は Task 5。入ったらここへ足す）
#[tokio::test]
async fn satellite_admin_only_api_token_is_forbidden() {
    let c = Ctx::new().await;
    let (id, name) = (c.identifier("apitoken"), c.name("合言葉"));
    assert_eq!(c.register(&name, &id).await.0, StatusCode::CREATED);
    let sat = c.find(&id).await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let before = c.mine().await;

    let api = |method, uri: String, body| {
        let app = c.app.clone();
        async move { send(&app, method, &uri, None, Some(TOKEN), body).await }
    };
    for (method, uri, body) in [
        (Method::GET, "/satellites".to_owned(), None),
        (
            Method::POST,
            "/satellites".to_owned(),
            Some(json!({ "name": c.name("別"), "identifier": c.identifier("other") })),
        ),
        (Method::DELETE, format!("/satellites/{sat}"), None),
    ] {
        let (status, _) = api(method.clone(), uri.clone(), body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
    }
    assert_eq!(c.mine().await, before, "一覧は変わらない");
}

// Scenario: 資格情報が無いと衛星の一覧も読めない
#[tokio::test]
async fn satellite_admin_only_without_credential_is_unauthorized() {
    let c = Ctx::new().await;
    let (status, _) = send(&c.app, Method::GET, "/satellites", None, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let body = json!({ "name": c.name("無し"), "identifier": c.identifier("none") });
    let (status, _) = send(&c.app, Method::POST, "/satellites", None, None, Some(body)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// Scenario: 最初から外部 AI とローカル AI が登録されている
#[tokio::test]
async fn satellite_builtin_pair_is_listed() {
    let c = Ctx::new().await;
    for (name, identifier) in [
        ("外部 AI", "ashiato.ai-external"),
        ("ローカル AI", "ashiato.ai-local"),
    ] {
        let s = c.find(identifier).await.expect(identifier);
        assert_eq!(s["name"], name);
        assert_eq!(s["removed_at"], Value::Null);
        assert_eq!(s["builtin"], true);
    }
}
