// SPDX-License-Identifier: AGPL-3.0-only
//! 衛星の資格情報で断った求めの、読み出しの記録（ST29 / design D4 / D8 / D9）。本物の DB と router で確かめる。
//! 衛星と資格情報の行は SQL で直に入れる。行は衛星の番号で絞って読む（共有の表に他の試験の行が混ざるため）。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use tower::ServiceExt as _;

use super::*;
use crate::access_log::{AccessEntry, AccessSink as _, PgAccessSink};
use crate::satellite_authn_tests::{
    body_string, get_with, login, random_password, seed_credential, seed_satellite,
};
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap()
}

fn post_json(uri: &str, bearer: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// この衛星の行を、全列を文字列にして返す。
async fn rows_of(pool: &sqlx::PgPool, satellite: uuid::Uuid) -> Vec<String> {
    sqlx::query_scalar("SELECT a::text FROM core.access_log a WHERE satellite_id = $1")
        .bind(satellite)
        .fetch_all(pool)
        .await
        .unwrap()
}

type Row = (
    String,
    String,
    String,
    String,
    String,
    i16,
    Option<uuid::Uuid>,
    Option<uuid::Uuid>,
);

async fn typed_rows_of(pool: &sqlx::PgPool, satellite: uuid::Uuid) -> Vec<Row> {
    sqlx::query_as(
        "SELECT via, credential, route, method, outcome, status, satellite_id, satellite_credential_id
           FROM core.access_log WHERE satellite_id = $1 ORDER BY at",
    )
    .bind(satellite)
    .fetch_all(pool)
    .await
    .unwrap()
}

// Scenario: 衛星の資格情報で断った求めは、衛星と 1 本の番号つきで残る
#[tokio::test]
async fn satellite_access_refused_request_is_logged_with_numbers() {
    let pool = testdb::pool().await;
    let app = App::for_test(pool.clone(), TOKEN).at(now());
    let (sat, ident) = seed_satellite(&pool).await;
    let (cred_a, value_a) = seed_credential(&pool, sat, &ident, now()).await;
    let (cred_b, value_b) = seed_credential(&pool, sat, &ident, now()).await;

    let res = router(app.clone())
        .oneshot(get_with("/events", &value_a, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    assert_eq!(
        typed_rows_of(&pool, sat).await,
        vec![(
            "direct".into(),
            "satellite".into(),
            "/events".into(),
            "GET".into(),
            "forbidden".into(),
            403,
            Some(sat),
            Some(cred_a),
        )],
        "付けた 1 本の番号だけが入る"
    );
    let _ = cred_b;
    for text in rows_of(&pool, sat).await {
        for secret in [&ident, &value_a, &value_b] {
            assert!(
                !text.contains(secret.as_str()),
                "行に識別子か値が残っている"
            );
        }
    }
}

// Scenario: 衛星の資格情報で取り込みの口に送った求めも残る
#[tokio::test]
async fn satellite_access_ingest_request_is_logged() {
    let pool = testdb::pool().await;
    let app = App::for_test(pool.clone(), TOKEN).at(now());
    let (sat, ident) = seed_satellite(&pool).await;
    let (cred, value) = seed_credential(&pool, sat, &ident, now()).await;

    let res = router(app.clone())
        .oneshot(post_json("/ingest", &value, &serde_json::json!([])))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let rows = typed_rows_of(&pool, sat).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (
            rows[0].1.as_str(),
            rows[0].2.as_str(),
            rows[0].4.as_str(),
            rows[0].5,
            rows[0].7
        ),
        ("satellite", "/ingest", "forbidden", 403, Some(cred))
    );
}

// Scenario: 衛星の資格情報で送った記録は取り込まれない
#[tokio::test]
async fn satellite_access_sent_records_are_not_ingested() {
    let pool = testdb::pool().await;
    let app = App::for_test(pool.clone(), TOKEN).at(now());
    let (sat, ident) = seed_satellite(&pool).await;
    let (_, value) = seed_credential(&pool, sat, &ident, now()).await;
    let user = uuid::Uuid::new_v4();
    let source = testdb::source(&pool, "satelliteaccess", 21_600).await;

    let event = serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": source,
        "external_id": null, "device_id": "test-dev", "origin": "collected",
        "event_time": "2026-03-01T01:00:00Z", "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
        "schema_version": 1, "raw": r#"{"a":1}"#, "payload": {},
    });
    let heartbeat = serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": source,
        "device_id": "test-dev", "emitted_at": "2026-03-01T01:00:00Z", "capturable": true,
        "attempts": 1, "successes": 1, "raw": r#"{"a":1}"#,
    });
    let drop_report = crate::drops_tests::report(&source, user, "2026-03-01", &[(10, 5)]);
    // 本物の資格情報なら通る形であることを先に確かめる（テストが空振りしないように）
    let ok = router(app.clone())
        .oneshot(post_json("/ingest", TOKEN, &serde_json::json!([event])))
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    let count = |table: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>(&format!(
                "SELECT count(*) FROM core.{table} WHERE user_id = $1"
            ))
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    let before = (
        count("event").await,
        count("heartbeat").await,
        count("drop_report").await,
    );
    assert_eq!(before.0, 1);

    for (uri, body) in [
        ("/ingest", serde_json::json!([event])),
        ("/heartbeat", serde_json::json!([heartbeat])),
        ("/drops", serde_json::json!([drop_report])),
    ] {
        let mut changed = body.clone();
        if uri == "/ingest" {
            changed[0]["id"] = serde_json::json!(uuid::Uuid::new_v4());
            changed[0]["raw"] = serde_json::json!(r#"{"a":2}"#);
        }
        let res = router(app.clone())
            .oneshot(post_json(uri, &value, &changed))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "{uri}");
    }
    let after = (
        count("event").await,
        count("heartbeat").await,
        count("drop_report").await,
    );
    assert_eq!(after, before, "件数は増えない");
}

// Scenario: 衛星の資格情報ではログインの状態も読めない
#[tokio::test]
async fn satellite_access_session_state_is_forbidden_and_logged() {
    let pool = testdb::pool().await;
    let password = random_password();
    let app = App::for_test(pool.clone(), TOKEN)
        .with_web_login(&password, 0)
        .at(now());
    let (sat, ident) = seed_satellite(&pool).await;
    let (cred, value) = seed_credential(&pool, sat, &ident, now()).await;
    let cookie = login(&app, &password).await;

    for c in [None, Some(cookie.as_str())] {
        let res = router(app.clone())
            .oneshot(get_with("/session", &value, c))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        assert!(!res.headers().contains_key(header::SET_COOKIE));
        assert_eq!(body_string(res).await, "forbidden");
    }
    let rows = typed_rows_of(&pool, sat).await;
    assert_eq!(rows.len(), 2, "403 と同じ行が求めごとに 1 行");
    for r in rows {
        assert_eq!(
            (
                r.1.as_str(),
                r.2.as_str(),
                r.3.as_str(),
                r.4.as_str(),
                r.5,
                r.7
            ),
            ("satellite", "/session", "GET", "forbidden", 403, Some(cred))
        );
    }
}

/// 書き換えられない表の行の形を、固定した値で試す（D9）。全列を読んで期待の値と比べる。
#[tokio::test]
async fn satellite_access_row_shape_is_fixed() {
    let pool = testdb::pool().await;
    let (sat, cred) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let entry = AccessEntry::for_caller(
        Some(web_session::Caller::Satellite {
            satellite: sat,
            credential: cred,
        }),
        "forwarded",
        "/events".into(),
        "GET".into(),
        "forbidden",
        403,
    );
    assert_eq!(entry.credential, "satellite");
    PgAccessSink(pool.clone()).record(entry).await.unwrap();

    let (via, credential, route, method, outcome, status, sid, cid): Row =
        typed_rows_of(&pool, sat).await.pop().unwrap();
    assert_eq!(
        (
            via.as_str(),
            credential.as_str(),
            route.as_str(),
            method.as_str(),
            outcome.as_str(),
            status
        ),
        ("forwarded", "satellite", "/events", "GET", "forbidden", 403)
    );
    assert_eq!((sid, cid), (Some(sat), Some(cred)));
    // 列は 9（at・via・credential・route・method・outcome・status と 2 つの番号）。ほかに足されていない
    let columns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'access_log'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(columns, 9);
    // 衛星でない行には番号が入らない（型の作り方で守る約束）
    let other = AccessEntry::for_caller(
        Some(web_session::Caller::ApiToken),
        "direct",
        "/events".into(),
        "GET".into(),
        "ok",
        200,
    );
    assert_eq!(other.satellite, None);
}
