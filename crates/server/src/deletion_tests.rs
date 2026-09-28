// SPDX-License-Identifier: AGPL-3.0-only
//! 滞在を消す口と、同じまとまりで付く印・台帳・錠を本物の PostgreSQL で確かめる（ST22）。
#![allow(clippy::unwrap_used)]

use crate::{deletion, stay_store, testdb};
use axum::extract::State;
use chrono::{DateTime, Duration, Utc};

const TOKEN: &str = "test-token-0123456789abcdef";

#[derive(Clone, Default)]
struct Captured(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn t(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

fn auth() -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    headers
}

async fn app() -> crate::App {
    crate::App::for_test(testdb::pool().await, TOKEN)
}

async fn put_event(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    at: DateTime<Utc>,
) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    let raw = if source == "c01-location" {
        r#"{"lat":35.68,"lon":139.76,"acc_m":10}"#
    } else {
        "{}"
    };
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test','collected',$4,540,'Asia/Tokyo',1,$5,$6,$6::jsonb)",
    )
    .bind(id)
    .bind(user)
    .bind(source)
    .bind(at)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(raw)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn put_stay(pool: &sqlx::PgPool, user: uuid::Uuid, start: &str, end: &str) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    let payload = serde_json::json!({
        "start": t(start).to_rfc3339(),
        "end": t(end).to_rfc3339(),
        "lat": 35.68,
        "lon": 139.76,
    })
    .to_string();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,'s01-stay',$3,'derived',$4,540,'Asia/Tokyo',1,$3,$5,$5::jsonb)",
    )
    .bind(id)
    .bind(user)
    .bind(id.to_string())
    .bind(t(start))
    .bind(payload)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn erase(
    app: &crate::App,
    stay_id: uuid::Uuid,
    user_id: Option<uuid::Uuid>,
) -> Result<serde_json::Value, axum::http::StatusCode> {
    crate::stays_erase(
        State(app.clone()),
        auth(),
        axum::Json(crate::EraseRequest { stay_id, user_id }),
    )
    .await
    .map(|axum::Json(value)| serde_json::to_value(value).unwrap())
    .map_err(|(status, _)| status)
}

async fn mark(pool: &sqlx::PgPool, id: uuid::Uuid) -> (Option<DateTime<Utc>>, Option<String>) {
    sqlx::query_as("SELECT deleted_at, deleted_by FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn live_count(pool: &sqlx::PgPool, user: uuid::Uuid, source: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM core.event_live WHERE user_id = $1 AND logical_source = $2",
    )
    .bind(user)
    .bind(source)
    .fetch_one(pool)
    .await
    .unwrap()
}

// Scenario: 消した滞在は 1 日の並びの滞在として出なくなる
// Scenario: 消した印は作り直しの印と区別される
// Scenario: 滞在でない記録の識別子では消せない
// Scenario: 知らない識別子を消そうとすると断られる
// Scenario: 他の利用者を指定して消すことはできない
// Scenario: 資格情報の無い消す求めは断られる
#[tokio::test]
async fn erase_endpoint() {
    let app = app().await;
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-01T10:00:00+09:00",
        "2026-09-01T11:00:00+09:00",
    )
    .await;
    for minute in 0..=60 {
        put_event(
            &app.pool,
            user,
            "c01-location",
            t("2026-09-01T10:00:00+09:00") + Duration::minutes(minute),
        )
        .await;
    }
    let location = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-01T12:00:00+09:00"),
    )
    .await;

    let other = testdb::user();
    let captured = Captured::default();
    let sink = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || sink.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);
    assert_eq!(
        erase(&app, stay, Some(other)).await,
        Err(axum::http::StatusCode::NOT_FOUND)
    );
    drop(guard);
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("kind=\"erase_user_mismatch\""), "{log}");
    assert!(
        !log.contains(&stay.to_string()),
        "滞在の識別子がログに出た: {log}"
    );
    assert!(
        !log.contains(&user.to_string()),
        "利用者がログに出た: {log}"
    );
    assert!(
        !log.contains(&other.to_string()),
        "添えた利用者がログに出た: {log}"
    );
    assert_eq!(mark(&app.pool, stay).await, (None, None));
    assert_eq!(
        erase(&app, location, Some(user)).await,
        Err(axum::http::StatusCode::NOT_FOUND)
    );
    assert_eq!(mark(&app.pool, location).await, (None, None));
    assert_eq!(
        erase(&app, uuid::Uuid::new_v4(), Some(user)).await,
        Err(axum::http::StatusCode::NOT_FOUND)
    );
    let unauthorized = crate::stays_erase(
        State(app.clone()),
        axum::http::HeaderMap::new(),
        axum::Json(crate::EraseRequest {
            stay_id: stay,
            user_id: Some(user),
        }),
    )
    .await;
    assert!(matches!(
        unauthorized,
        Err((axum::http::StatusCode::UNAUTHORIZED, _))
    ));

    let response = erase(&app, stay, Some(user)).await.unwrap();
    assert_eq!(
        response,
        serde_json::json!({"erased":{"stays":1,"locations":61}})
    );
    let day = stay_store::day_view(
        &app.pool,
        user,
        testdb::date("2026-09-01"),
        t("2026-09-02T00:00:00+09:00"),
    )
    .await
    .unwrap();
    assert!(
        day.entries
            .iter()
            .all(|entry| entry.kind != stay_store::EntryKind::Stay),
        "消した滞在が 1 日の並びに残っている"
    );
    stay_store::rebuild_day(&app.pool, user, testdb::date("2026-09-01"))
        .await
        .unwrap();
    assert_eq!(mark(&app.pool, stay).await.1.as_deref(), Some("user"));
}

// Scenario: 滞在を消すとその時間の位置も読み出しから消える
// Scenario: 滞在の時間の外の位置は消えない
// Scenario: 同じ時間の PC のウィンドウの記録は消えない
#[tokio::test]
async fn erase_cascade() {
    let app = app().await;
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-02T10:00:00+09:00",
        "2026-09-02T11:00:00+09:00",
    )
    .await;
    let before = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-02T09:59:00+09:00"),
    )
    .await;
    let at_start = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-02T10:00:00+09:00"),
    )
    .await;
    let at_end = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-02T11:00:00+09:00"),
    )
    .await;
    let after = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-02T11:01:00+09:00"),
    )
    .await;
    let window = put_event(
        &app.pool,
        user,
        "c02-window",
        t("2026-09-02T10:30:00+09:00"),
    )
    .await;

    erase(&app, stay, None).await.unwrap();
    assert_eq!(
        mark(&app.pool, at_start).await.1.as_deref(),
        Some("user:cascade")
    );
    assert_eq!(
        mark(&app.pool, at_end).await.1.as_deref(),
        Some("user:cascade")
    );
    assert_eq!(mark(&app.pool, before).await, (None, None));
    assert_eq!(mark(&app.pool, after).await, (None, None));
    assert_eq!(mark(&app.pool, window).await, (None, None));
    assert_eq!(live_count(&app.pool, user, "c01-location").await, 2);
    assert_eq!(live_count(&app.pool, user, "c02-window").await, 1);
}

// Scenario: 二度目の消す求めで削除時刻が動かない
#[tokio::test]
async fn erase_is_idempotent() {
    let app = app().await;
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-03T10:00:00+09:00",
        "2026-09-03T11:00:00+09:00",
    )
    .await;
    assert_eq!(erase(&app, stay, None).await.unwrap()["erased"]["stays"], 1);
    let first = mark(&app.pool, stay).await;
    let ledger_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.deletion_ledger WHERE cause_event_id = $1")
            .bind(stay)
            .fetch_one(&app.pool)
            .await
            .unwrap();

    assert_eq!(
        erase(&app, stay, None).await.unwrap(),
        serde_json::json!({"erased":{"stays":0,"locations":0}})
    );
    assert_eq!(mark(&app.pool, stay).await, first);
    let ledger_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.deletion_ledger WHERE cause_event_id = $1")
            .bind(stay)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(ledger_after, ledger_before);
}

// Scenario: 台帳に書けないと滞在の印も付かない
#[tokio::test]
async fn erase_is_atomic() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let stay = put_stay(
        &pool,
        user,
        "2026-09-04T10:00:00+09:00",
        "2026-09-04T11:00:00+09:00",
    )
    .await;
    let location = put_event(&pool, user, "c01-location", t("2026-09-04T10:30:00+09:00")).await;

    assert!(
        deletion::erase_with_action(&pool, stay, None, "not-an-action")
            .await
            .is_err()
    );
    assert_eq!(mark(&pool, stay).await, (None, None));
    assert_eq!(mark(&pool, location).await, (None, None));
}

// Scenario: 消す操作と作り直しが同時に走っても消したことは残る
#[tokio::test]
async fn erase_locks_against_rebuild() {
    let pool = testdb::pool().await;
    let holder = testdb::pool().await;
    let user = testdb::user();
    let stay = put_stay(
        &pool,
        user,
        "2026-09-05T10:00:00+09:00",
        "2026-09-05T11:00:00+09:00",
    )
    .await;
    for minute in 0..=60 {
        put_event(
            &pool,
            user,
            "c01-location",
            t("2026-09-05T10:00:00+09:00") + Duration::minutes(minute),
        )
        .await;
    }

    let mut tx = holder.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text))")
        .bind(stay_store::LOCK_KEY)
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
    let erase_task = tokio::spawn({
        let pool = pool.clone();
        async move { deletion::erase(&pool, stay, None).await }
    });
    let rebuild_task = tokio::spawn({
        let pool = pool.clone();
        async move { stay_store::rebuild_day(&pool, user, testdb::date("2026-09-05")).await }
    });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(
        !erase_task.is_finished(),
        "消す操作が作り直しの錠を待っていない"
    );
    assert!(!rebuild_task.is_finished(), "作り直しが錠を待っていない");
    tx.rollback().await.unwrap();
    erase_task.await.unwrap().unwrap();
    rebuild_task.await.unwrap().unwrap();
    assert_eq!(mark(&pool, stay).await.1.as_deref(), Some("user"));
}

// Scenario: 滞在と連鎖した位置の消去が台帳に残る
#[tokio::test]
async fn erase_writes_ledger() {
    let app = app().await;
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-06T10:00:00+09:00",
        "2026-09-06T11:00:00+09:00",
    )
    .await;
    let mut locations = Vec::new();
    for minute in [0, 30, 60] {
        locations.push(
            put_event(
                &app.pool,
                user,
                "c01-location",
                t("2026-09-06T10:00:00+09:00") + Duration::minutes(minute),
            )
            .await,
        );
    }
    erase(&app, stay, None).await.unwrap();

    let rows: Vec<(uuid::Uuid, String, uuid::Uuid, String)> = sqlx::query_as(
        "SELECT event_id, logical_source, cause_event_id, mark
           FROM core.deletion_ledger WHERE cause_event_id = $1 ORDER BY seq",
    )
    .bind(stay)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 4);
    assert!(rows
        .iter()
        .any(|row| row == &(stay, "s01-stay".into(), stay, "user".into())));
    for location in locations {
        assert!(rows
            .iter()
            .any(|row| { row == &(location, "c01-location".into(), stay, "user:cascade".into()) }));
    }
}
