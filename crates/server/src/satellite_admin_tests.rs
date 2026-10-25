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
        (
            Method::POST,
            format!("/satellites/{sat}/credentials"),
            Some(json!({ "label": "携帯" })),
        ),
        (
            Method::DELETE,
            format!("/satellites/{sat}/credentials/{}", uuid::Uuid::new_v4()),
            None,
        ),
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

impl Ctx {
    /// 衛星を登録して番号を返す。
    async fn register_sat(&self, part: &str) -> (uuid::Uuid, String) {
        let (id, name) = (self.identifier(part), self.name(part));
        assert_eq!(self.register(&name, &id).await.0, StatusCode::CREATED);
        let sat = self.find(&id).await.unwrap()["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        (sat, id)
    }

    /// 発行して（番号, 値）を返す。
    async fn issue(&self, sat: uuid::Uuid, label: &str) -> (uuid::Uuid, String) {
        let (status, body) = self
            .web(
                Method::POST,
                &format!("/satellites/{sat}/credentials"),
                Some(json!({ "label": label })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let v: Value = serde_json::from_str(&body).unwrap();
        (
            v["id"].as_str().unwrap().parse().unwrap(),
            v["value"].as_str().unwrap().to_owned(),
        )
    }

    async fn revoke(&self, sat: uuid::Uuid, cid: uuid::Uuid) -> StatusCode {
        self.web(
            Method::DELETE,
            &format!("/satellites/{sat}/credentials/{cid}"),
            None,
        )
        .await
        .0
    }

    /// その値を付けて記録の読み出しを求める。
    async fn read_with(&self, value: &str) -> StatusCode {
        send(&self.app, Method::GET, "/events", None, Some(value), None)
            .await
            .0
    }

    async fn creds_of(&self, identifier: &str) -> Vec<Value> {
        self.find(identifier).await.unwrap()["credentials"]
            .as_array()
            .unwrap()
            .clone()
    }
}

// Scenario: 発行すると、衛星の識別子の入った値が 1 回だけ返る
#[tokio::test]
async fn satellite_credential_issue_value_carries_identifier() {
    let c = Ctx::new().await;
    let (sat, id) = c.register_sat("issue").await;
    let (cred, value) = c.issue(sat, "携帯").await;
    let rest = value
        .strip_prefix(&format!("ashsat_{id}_"))
        .unwrap_or_else(|| panic!("値の頭が違う"));
    assert_eq!(rest.len(), 43);
    assert!(rest
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'));
    // 期待値は実装の外（DB の sha256）で作る（review/code.md R4）
    let same: bool = sqlx::query_scalar(
        "SELECT token_sha256 = sha256(convert_to($2, 'UTF8'))
           FROM core.satellite_credential WHERE id = $1",
    )
    .bind(cred)
    .bind(&value)
    .fetch_one(&c.pool)
    .await
    .unwrap();
    assert!(same, "保存は値全体の SHA-256");
}

// Scenario: 発行した値は二度と読み出せない
#[tokio::test]
async fn satellite_credential_issue_value_is_never_listed() {
    let c = Ctx::new().await;
    let (sat, id) = c.register_sat("hidden").await;
    let (cred, value) = c.issue(sat, "携帯").await;
    let random = value
        .strip_prefix(&format!("ashsat_{id}_"))
        .unwrap()
        .to_owned();
    let (_, listed) = c.web(Method::GET, "/satellites", None).await;
    assert!(!listed.contains(&value) && !listed.contains(&random));
    let entry = c.find(&id).await.unwrap();
    assert_eq!(entry["name"], c.name("hidden").as_str());
    let own = c.creds_of(&id).await;
    assert_eq!(own.len(), 1);
    assert_eq!(own[0]["label"], "携帯");
    assert_eq!(own[0]["id"], cred.to_string());
    assert!(own[0]["issued_at"].is_string());
    let mut keys: Vec<_> = own[0].as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["id", "issued_at", "label", "last_used_at"]);
    let row: String = sqlx::query_scalar(
        "SELECT row_to_json(t)::text FROM core.satellite_credential t WHERE id = $1",
    )
    .bind(cred)
    .fetch_one(&c.pool)
    .await
    .unwrap();
    assert!(
        !row.contains(&value) && !row.contains(&random),
        "表のどの列にも無い"
    );
}

// Scenario: 1 つの衛星に 2 本の資格情報を発行できる
#[tokio::test]
async fn satellite_credential_issue_two_for_one_satellite() {
    let c = Ctx::new().await;
    let (sat, id) = c.register_sat("two").await;
    let (_, a) = c.issue(sat, "携帯").await;
    let (_, b) = c.issue(sat, "ノート PC").await;
    assert_ne!(a, b);
    assert_eq!(c.creds_of(&id).await.len(), 2);
}

// Scenario: 外した衛星には資格情報を発行できない
#[tokio::test]
async fn satellite_credential_issue_removed_satellite_is_conflict() {
    let c = Ctx::new().await;
    let (sat, id) = c.register_sat("gone").await;
    assert_eq!(
        c.web(Method::DELETE, &format!("/satellites/{sat}"), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    let (status, body) = c
        .web(
            Method::POST,
            &format!("/satellites/{sat}/credentials"),
            Some(json!({ "label": "携帯" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(kind(&body), "removed");
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.satellite_credential WHERE satellite_id = $1",
    )
    .bind(sat)
    .fetch_one(&c.pool)
    .await
    .unwrap();
    assert_eq!(n, 0, "{id}");
    let missing = format!("/satellites/{}/credentials", uuid::Uuid::new_v4());
    let (status, _) = c
        .web(Method::POST, &missing, Some(json!({ "label": "" })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// Scenario: 41 文字の名前の資格情報は発行されない
#[tokio::test]
async fn satellite_credential_issue_label_length_edge() {
    let c = Ctx::new().await;
    let (sat, id) = c.register_sat("label").await;
    let uri = format!("/satellites/{sat}/credentials");
    let (status, body) = c
        .web(
            Method::POST,
            &uri,
            Some(json!({ "label": "あ".repeat(41) })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(kind(&body), "name_invalid");
    assert_eq!(
        c.web(
            Method::POST,
            &uri,
            Some(json!({ "label": "あ".repeat(40) }))
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        c.web(Method::POST, &uri, Some(json!({}))).await.0,
        StatusCode::CREATED,
        "名前は空でもよい"
    );
    assert_eq!(c.creds_of(&id).await.len(), 2);
}

// Scenario: 使われた資格情報には最後に使われた時刻が残る
#[tokio::test]
async fn satellite_credential_issue_last_used_is_recorded() {
    let c = Ctx::new().await;
    let (sat, id) = c.register_sat("used").await;
    let (used, value) = c.issue(sat, "使った").await;
    let (unused, _) = c.issue(sat, "まだ").await;
    assert_eq!(c.read_with(&value).await, StatusCode::FORBIDDEN);
    let own = c.creds_of(&id).await;
    let last = |cid: uuid::Uuid| {
        own.iter().find(|v| v["id"] == cid.to_string()).unwrap()["last_used_at"].clone()
    };
    let at: chrono::DateTime<chrono::Utc> = serde_json::from_value(last(used)).unwrap();
    assert_eq!(at, now());
    assert_eq!(last(unused), Value::Null);
}

// Scenario: 取り消した資格情報は次の求めから使えない
#[tokio::test]
async fn satellite_credential_revoke_stops_only_that_one() {
    let c = Ctx::new().await;
    let (sat, id) = c.register_sat("revoke").await;
    let (a, va) = c.issue(sat, "A").await;
    let (b, vb) = c.issue(sat, "B").await;
    assert_eq!(c.revoke(sat, a).await, StatusCode::NO_CONTENT);
    assert_eq!(c.read_with(&va).await, StatusCode::UNAUTHORIZED);
    assert_eq!(c.read_with(&vb).await, StatusCode::FORBIDDEN);
    let own = c.creds_of(&id).await;
    assert_eq!(own.len(), 1);
    assert_eq!(own[0]["id"], b.to_string());
    let revoked: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT revoked_at FROM core.satellite_credential WHERE id = $1")
            .bind(a)
            .fetch_one(&c.pool)
            .await
            .unwrap();
    assert_eq!(revoked, Some(now()), "行は消さず時刻を付ける");
    assert_eq!(
        c.revoke(sat, uuid::Uuid::new_v4()).await,
        StatusCode::NOT_FOUND
    );
    let (other, _) = c.register_sat("revoke-other").await;
    assert_eq!(
        c.revoke(other, b).await,
        StatusCode::NOT_FOUND,
        "別の衛星の番号では取り消せない"
    );
}

// Scenario: 衛星を外すとその衛星の資格情報が全部止まる
#[tokio::test]
async fn satellite_credential_revoke_on_removal_stops_all() {
    let c = Ctx::new().await;
    let (books, books_id) = c.register_sat("books").await;
    let (money, money_id) = c.register_sat("money").await;
    let (_, b1) = c.issue(books, "1").await;
    let (_, b2) = c.issue(books, "2").await;
    let (_, m1) = c.issue(money, "1").await;
    assert_eq!(
        c.web(Method::DELETE, &format!("/satellites/{books}"), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(c.read_with(&b1).await, StatusCode::UNAUTHORIZED);
    assert_eq!(c.read_with(&b2).await, StatusCode::UNAUTHORIZED);
    assert_eq!(c.read_with(&m1).await, StatusCode::FORBIDDEN);
    assert!(c.find(&books_id).await.unwrap()["removed_at"].is_string());
    assert_eq!(c.creds_of(&money_id).await.len(), 1);
}

// Scenario: 外した衛星を同じ識別子で登録し直すと前の衛星の続きになる
#[tokio::test]
async fn satellite_credential_revoke_reregistered_keeps_old_credentials_dead() {
    let c = Ctx::new().await;
    let (sat, id) = c.register_sat("back").await;
    let before = c.find(&id).await.unwrap();
    let (_, value) = c.issue(sat, "前").await;
    assert_eq!(
        c.web(Method::DELETE, &format!("/satellites/{sat}"), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(c.register(&c.name("読書メモ"), &id).await.0, StatusCode::OK);
    let back = c.find(&id).await.unwrap();
    assert_eq!(back["id"], before["id"]);
    assert_eq!(back["registered_at"], before["registered_at"]);
    assert_eq!(back["name"], c.name("読書メモ").as_str());
    assert_eq!(back["removed_at"], Value::Null);
    assert_eq!(c.creds_of(&id).await.len(), 0);
    assert_eq!(c.read_with(&value).await, StatusCode::UNAUTHORIZED);
}

// Scenario: 最初の AI にも資格情報を発行して取り消せる
#[tokio::test]
async fn satellite_credential_revoke_builtin_ai() {
    let c = Ctx::new().await;
    let local = c.find("ashiato.ai-local").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let (cid, value) = c.issue(local, "試験").await;
    assert_eq!(c.read_with(&value).await, StatusCode::FORBIDDEN);
    assert_eq!(c.revoke(local, cid).await, StatusCode::NO_CONTENT);
    assert_eq!(c.read_with(&value).await, StatusCode::UNAUTHORIZED);
    assert_eq!(
        c.find("ashiato.ai-local").await.unwrap()["removed_at"],
        Value::Null
    );
}
