// SPDX-License-Identifier: AGPL-3.0-only
//! 衛星の資格情報の判定（ST29 / design D8 / D18 / D19）。router を通して本物の DB で確かめる。
//! Task 4 / 5 の口がまだ無いので、衛星と資格情報の行は SQL で直に入れる（`seed_*`）。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, Response, StatusCode};
use http_body_util::BodyExt as _;
use tower::ServiceExt as _;

use super::*;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

/// 試験ごとに隔離した衛星 1 つを入れる。戻り値は（番号, 識別子）。
pub(crate) async fn seed_satellite(pool: &sqlx::PgPool) -> (uuid::Uuid, String) {
    let id = uuid::Uuid::new_v4();
    let identifier = format!("dev.t{}.booklog", id.simple());
    sqlx::query(
        "INSERT INTO core.satellite (id, identifier, name, registered_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(&identifier)
    .bind(format!("試験 {}", &id.simple().to_string()[..12]))
    .bind(chrono::Utc::now())
    .execute(pool)
    .await
    .unwrap();
    (id, identifier)
}

/// 資格情報 1 本を入れる。戻り値は（番号, 値）。値は `ashsat_<識別子>_<乱数>` の形（D6）。
pub(crate) async fn seed_credential(
    pool: &sqlx::PgPool,
    satellite: uuid::Uuid,
    identifier: &str,
    issued_at: chrono::DateTime<chrono::Utc>,
) -> (uuid::Uuid, String) {
    let id = uuid::Uuid::new_v4();
    let value = format!(
        "{}{identifier}_{}{}",
        satellite::CREDENTIAL_PREFIX,
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    sqlx::query(
        "INSERT INTO core.satellite_credential (id, satellite_id, token_sha256, issued_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(satellite)
    .bind(web_session::sha256(&value))
    .bind(issued_at)
    .execute(pool)
    .await
    .unwrap();
    (id, value)
}

pub(crate) fn random_password() -> String {
    format!("pw-{}", uuid::Uuid::new_v4())
}

pub(crate) async fn login(app: &App, password: &str) -> String {
    let res = router(app.clone())
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/session")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "password": password }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let set = res.headers()[header::SET_COOKIE].to_str().unwrap();
    set.strip_prefix(&format!("{}=", web_session::COOKIE_NAME))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

/// `Authorization: Bearer <value>`（と、あれば cookie）を付けた GET。
pub(crate) fn get_with(uri: &str, bearer: &str, cookie: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"));
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, format!("{}={c}", web_session::COOKIE_NAME));
    }
    b.body(Body::empty()).unwrap()
}

pub(crate) async fn body_string(res: Response<Body>) -> String {
    String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap()
}

async fn last_used(
    pool: &sqlx::PgPool,
    credential: uuid::Uuid,
) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar("SELECT last_used_at FROM core.satellite_credential WHERE id = $1")
        .bind(credential)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn fixed_now() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap()
}

// Scenario: ログインの印を併せて持っていても衛星の求めとして断る
#[tokio::test]
async fn satellite_authn_cookie_and_credential_is_refused_as_satellite() {
    let pool = testdb::pool().await;
    let password = random_password();
    let at = fixed_now();
    let app = App::for_test(pool.clone(), TOKEN)
        .with_web_login(&password, 0)
        .at(at);
    let (sat, ident) = seed_satellite(&pool).await;
    let (cred, value) = seed_credential(&pool, sat, &ident, at).await;
    let cookie = login(&app, &password).await;
    assert_eq!(last_used(&pool, cred).await, None, "使う前は NULL");

    let res = router(app.clone())
        .oneshot(get_with("/events", &value, Some(&cookie)))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert!(
        !res.headers().contains_key(header::SET_COOKIE),
        "印を出し直さない"
    );
    assert_eq!(body_string(res).await, "forbidden");
    assert_eq!(
        last_used(&pool, cred).await,
        Some(at),
        "last_used_at が求めの時刻"
    );
}

// Scenario: 取り消した資格情報はログインの印を併せて持っていても通らない
#[tokio::test]
async fn satellite_authn_revoked_credential_with_cookie_is_unauthorized() {
    let pool = testdb::pool().await;
    let password = random_password();
    let app = App::for_test(pool.clone(), TOKEN)
        .with_web_login(&password, 0)
        .at(fixed_now());
    let (sat, ident) = seed_satellite(&pool).await;
    let (cred, value) = seed_credential(&pool, sat, &ident, fixed_now()).await;
    sqlx::query("UPDATE core.satellite_credential SET revoked_at = $2 WHERE id = $1")
        .bind(cred)
        .bind(fixed_now())
        .execute(&pool)
        .await
        .unwrap();
    let cookie = login(&app, &password).await;

    let res = router(app.clone())
        .oneshot(get_with("/events", &value, Some(&cookie)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        last_used(&pool, cred).await,
        None,
        "取り消した 1 本は使われた印も付かない"
    );

    // 外した衛星の 1 本・知らない値も、印があっても通らない（D19（仮））
    let (sat2, ident2) = seed_satellite(&pool).await;
    let (_, value2) = seed_credential(&pool, sat2, &ident2, fixed_now()).await;
    sqlx::query("UPDATE core.satellite SET removed_at = $2 WHERE id = $1")
        .bind(sat2)
        .bind(fixed_now())
        .execute(&pool)
        .await
        .unwrap();
    for v in [value2, format!("ashsat_dev.x.y_{}", uuid::Uuid::new_v4())] {
        let res = router(app.clone())
            .oneshot(get_with("/events", &v, Some(&cookie)))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

// Scenario: 衛星の資格情報は日が経っても切れない
#[tokio::test]
async fn satellite_authn_credential_does_not_expire_after_400_days() {
    let pool = testdb::pool().await;
    let issued = fixed_now();
    let later = issued + chrono::Duration::days(400);
    let app = App::for_test(pool.clone(), TOKEN).at(later);
    let (sat, ident) = seed_satellite(&pool).await;
    let (cred, value) = seed_credential(&pool, sat, &ident, issued).await;

    let res = router(app.clone())
        .oneshot(get_with("/events", &value, None))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN, "401 でなく 403");
    assert_eq!(last_used(&pool, cred).await, Some(later));
}
