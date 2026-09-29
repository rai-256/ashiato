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

async fn restore(
    app: &crate::App,
    stay_ids: Vec<uuid::Uuid>,
    user_id: Option<uuid::Uuid>,
) -> Result<serde_json::Value, axum::http::StatusCode> {
    crate::stays_restore(
        State(app.clone()),
        auth(),
        axum::Json(crate::RestoreRequest { stay_ids, user_id }),
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

async fn put_dwell(pool: &sqlx::PgPool, user: uuid::Uuid, at: &str, minutes: i64) {
    let start = t(at);
    let times: Vec<_> = (0..=minutes)
        .map(|minute| start + Duration::minutes(minute))
        .collect();
    let raw = r#"{"lat":35.68,"lon":139.76,"acc_m":10}"#;
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         SELECT gen_random_uuid(), $1, 'c01-location', 'test', 'collected', event_time,
                540, 'Asia/Tokyo', 1, gen_random_uuid()::text, $3, $3::jsonb
           FROM unnest($2::timestamptz[]) AS event_time",
    )
    .bind(user)
    .bind(&times)
    .bind(raw)
    .execute(pool)
    .await
    .unwrap();
}

// Scenario: 詳細にその時間の記録の件数がソースごとに出る
// Scenario: 削除済みの記録は件数に数えない
#[tokio::test]
async fn stays_detail_counts() {
    let app = app().await;
    let user = testdb::user();
    let location = testdb::source(&app.pool, "detail-location", 21_600).await;
    let windows = testdb::source(&app.pool, "detail-windows", 21_600).await;
    sqlx::query("UPDATE core.source SET display_name = $2 WHERE logical_source = $1")
        .bind(&location)
        .bind("位置")
        .execute(&app.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE core.source SET display_name = $2 WHERE logical_source = $1")
        .bind(&windows)
        .bind("PC のウィンドウ")
        .execute(&app.pool)
        .await
        .unwrap();

    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-28T10:00:00+09:00",
        "2026-09-28T11:00:00+09:00",
    )
    .await;
    for minute in 0..12 {
        let at = format!("2026-09-28T10:{minute:02}:00+09:00");
        put_event(&app.pool, user, &location, t(&at)).await;
    }
    for _ in 0..3 {
        put_event(&app.pool, user, &windows, t("2026-09-28T10:30:00+09:00")).await;
    }
    for minute in 0..4 {
        let at = format!("2026-09-28T10:{minute:02}:00+09:00");
        testdb::put_deleted_event(&app.pool, user, &location, &at).await;
    }

    let axum::Json(detail) = crate::stays_detail_get(
        State(app.clone()),
        auth(),
        axum::extract::Query(crate::StaysDetailQuery {
            stay_id: stay,
            user_id: Some(user),
        }),
    )
    .await
    .unwrap();
    let json = serde_json::to_value(detail).unwrap();
    assert_eq!(json["stay_id"], stay.to_string());
    assert_eq!(
        json["counts"],
        serde_json::json!([
            {"logical_source": location, "display_name": "位置", "count": 8},
            {"logical_source": windows, "display_name": "PC のウィンドウ", "count": 3}
        ])
    );

    assert!(matches!(
        crate::stays_detail_get(
            State(app.clone()),
            axum::http::HeaderMap::new(),
            axum::extract::Query(crate::StaysDetailQuery {
                stay_id: stay,
                user_id: Some(user),
            }),
        )
        .await,
        Err((axum::http::StatusCode::UNAUTHORIZED, _))
    ));
    assert!(matches!(
        crate::stays_detail_get(
            State(app),
            auth(),
            axum::extract::Query(crate::StaysDetailQuery {
                stay_id: uuid::Uuid::new_v4(),
                user_id: Some(user),
            }),
        )
        .await,
        Err((axum::http::StatusCode::NOT_FOUND, _))
    ));
}

#[tokio::test]
async fn stays_detail_query_uses_user_time_index() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source(&pool, "detail-index", 21_600).await;

    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         SELECT gen_random_uuid(), $1, $2, 'test', 'collected',
                '2026-09-28T00:00:00+00:00'::timestamptz + make_interval(mins => i),
                0, 'UTC', 1, gen_random_uuid()::text, '{}', '{}'::jsonb
           FROM generate_series(0, 20_000) AS i",
    )
    .bind(user)
    .bind(&source)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql("ANALYZE core.event")
        .execute(&pool)
        .await
        .unwrap();

    let plan: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (COSTS OFF)
           SELECT e.logical_source, s.display_name, count(*)
             FROM core.event_live e
             JOIN core.source s ON s.logical_source = e.logical_source
            WHERE e.user_id = $1
              AND e.logical_source <> 's01-stay'
              AND e.event_time BETWEEN $2 AND $3
            GROUP BY e.logical_source, s.display_name",
    )
    .bind(user)
    .bind(t("2026-09-28T00:00:00+00:00"))
    .bind(t("2026-09-28T12:00:00+00:00"))
    .fetch_all(&pool)
    .await
    .unwrap();
    let plan = plan
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        plan.contains("event_by_user_time_live"),
        "詳細集計が user_id, event_time のライブ索引を使わない:\n{plan}"
    );
}

// Scenario: 消した滞在を戻すと一覧に戻る
// Scenario: 戻すと連鎖で消えた位置も戻る
// Scenario: 資格情報の無い戻す求めは断られる
// Scenario: 知らない識別子を戻そうとすると断られる
#[tokio::test]
async fn restore_endpoint() {
    let pool = testdb::pool().await;
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..crate::App::for_test(pool, TOKEN)
    };
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-20T10:00:00+09:00",
        "2026-09-20T11:00:00+09:00",
    )
    .await;
    let location = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-20T10:30:00+09:00"),
    )
    .await;
    erase(&app, stay, Some(user)).await.unwrap();

    let unauthorized = crate::stays_restore(
        State(app.clone()),
        axum::http::HeaderMap::new(),
        axum::Json(crate::RestoreRequest {
            stay_ids: vec![stay],
            user_id: Some(user),
        }),
    )
    .await;
    assert!(matches!(
        unauthorized,
        Err((axum::http::StatusCode::UNAUTHORIZED, _))
    ));

    let unknown = uuid::Uuid::new_v4();
    assert_eq!(
        restore(&app, vec![stay, unknown], Some(user)).await,
        Err(axum::http::StatusCode::NOT_FOUND)
    );
    assert_eq!(mark(&app.pool, stay).await.1.as_deref(), Some("user"));
    assert_eq!(
        mark(&app.pool, location).await.1.as_deref(),
        Some("user:cascade")
    );

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
        restore(&app, vec![stay], Some(other)).await,
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

    assert_eq!(
        restore(&app, vec![stay], Some(user)).await.unwrap(),
        serde_json::json!({"restored":{"stays":1,"locations":1}})
    );
    assert_eq!(mark(&app.pool, stay).await, (None, None));
    assert_eq!(mark(&app.pool, location).await, (None, None));
    let day = stay_store::day_view(
        &app.pool,
        user,
        testdb::date("2026-09-20"),
        t("2026-09-21T00:00:00+09:00"),
    )
    .await
    .unwrap();
    let restored = day
        .entries
        .iter()
        .find(|entry| entry.id == Some(stay))
        .expect("戻した滞在が 1 日の並びに無い");
    assert_eq!(restored.start, t("2026-09-20T10:00:00+09:00"));
    assert_eq!(restored.end, t("2026-09-20T11:00:00+09:00"));
    assert_eq!(live_count(&app.pool, user, "c01-location").await, 1);
}

#[tokio::test]
async fn restoring_one_of_overlapping_erases_keeps_the_other_cause_hidden() {
    let pool = testdb::pool().await;
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..crate::App::for_test(pool, TOKEN)
    };
    let user = testdb::user();
    let first = put_stay(
        &app.pool,
        user,
        "2026-09-22T10:00:00+09:00",
        "2026-09-22T11:00:00+09:00",
    )
    .await;
    let second = put_stay(
        &app.pool,
        user,
        "2026-09-22T10:30:00+09:00",
        "2026-09-22T11:30:00+09:00",
    )
    .await;
    let location = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-22T10:45:00+09:00"),
    )
    .await;

    erase(&app, first, None).await.unwrap();
    erase(&app, second, None).await.unwrap();
    assert_eq!(
        restore(&app, vec![first], None).await.unwrap()["restored"]["locations"],
        0
    );
    assert_eq!(
        mark(&app.pool, location).await.1.as_deref(),
        Some("user:cascade")
    );
    let latest_cause: uuid::Uuid = sqlx::query_scalar(
        "SELECT cause_event_id FROM core.deletion_ledger WHERE event_id = $1 ORDER BY seq DESC LIMIT 1",
    )
    .bind(location)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(latest_cause, second);

    restore(&app, vec![second], None).await.unwrap();
    assert_eq!(mark(&app.pool, location).await, (None, None));
}

#[tokio::test]
async fn malformed_erased_stay_end_is_safe_for_listing_and_rebuild() {
    let app = app().await;
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-23T10:00:00+09:00",
        "2026-09-23T11:00:00+09:00",
    )
    .await;
    erase(&app, stay, None).await.unwrap();
    sqlx::query("UPDATE core.event SET payload = jsonb_set(payload, '{end}', '\"not-a-time\"') WHERE id = $1")
        .bind(stay)
        .execute(&app.pool)
        .await
        .unwrap();

    let view = stay_store::day_view(
        &app.pool,
        user,
        testdb::date("2026-09-23"),
        t("2026-09-24T00:00:00+09:00"),
    )
    .await
    .expect("壊れた end を一覧で安全に扱う");
    assert!(view
        .entries
        .iter()
        .any(|entry| entry.kind == stay_store::EntryKind::Erased));
    stay_store::rebuild_day(&app.pool, user, testdb::date("2026-09-23"))
        .await
        .expect("壊れた end を作り直しで安全に扱う");
}

// Scenario: 後から届いて印が付いた位置も戻すと戻る
#[tokio::test]
async fn restore_includes_late_arrivals() {
    let pool = testdb::pool().await;
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..crate::App::for_test(pool, TOKEN)
    };
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-21T10:00:00+09:00",
        "2026-09-21T11:00:00+09:00",
    )
    .await;
    erase(&app, stay, None).await.unwrap();
    let late = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-21T10:30:00+09:00"),
    )
    .await;
    stay_store::rebuild_day(&app.pool, user, testdb::date("2026-09-21"))
        .await
        .unwrap();
    assert_eq!(mark(&app.pool, late).await.1.as_deref(), Some("user:late"));

    assert_eq!(
        restore(&app, vec![stay], None).await.unwrap(),
        serde_json::json!({"restored":{"stays":1,"locations":1}})
    );
    assert_eq!(mark(&app.pool, late).await, (None, None));
    let axum::Json(events) = crate::events(State(app.clone()), auth()).await.unwrap();
    assert!(events.iter().any(|event| event.id == late));
}

// Scenario: 別の操作で消した記録は戻らない
// Scenario: 消えていない滞在を戻しても何も起きない
#[tokio::test]
async fn restore_is_scoped() {
    let pool = testdb::pool().await;
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..crate::App::for_test(pool, TOKEN)
    };
    let user = testdb::user();
    let first = put_stay(
        &app.pool,
        user,
        "2026-09-22T10:00:00+09:00",
        "2026-09-22T11:00:00+09:00",
    )
    .await;
    let second = put_stay(
        &app.pool,
        user,
        "2026-09-22T10:00:00+09:00",
        "2026-09-22T11:00:00+09:00",
    )
    .await;
    let location = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-22T10:30:00+09:00"),
    )
    .await;

    erase(&app, first, None).await.unwrap();
    restore(&app, vec![first], None).await.unwrap();
    erase(&app, second, None).await.unwrap();
    erase(&app, first, None).await.unwrap();
    restore(&app, vec![first], None).await.unwrap();

    assert_eq!(mark(&app.pool, first).await, (None, None));
    assert_eq!(mark(&app.pool, second).await.1.as_deref(), Some("user"));
    assert_eq!(
        mark(&app.pool, location).await.1.as_deref(),
        Some("user:cascade"),
        "最新の台帳行が別の滞在を原因とする位置を戻した"
    );

    let live = put_stay(
        &app.pool,
        user,
        "2026-09-22T13:00:00+09:00",
        "2026-09-22T14:00:00+09:00",
    )
    .await;
    let ledger_before: i64 = sqlx::query_scalar("SELECT count(*) FROM core.deletion_ledger")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(
        restore(&app, vec![live], None).await.unwrap(),
        serde_json::json!({"restored":{"stays":0,"locations":0}})
    );
    let ledger_after: i64 = sqlx::query_scalar("SELECT count(*) FROM core.deletion_ledger")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(ledger_after, ledger_before);
}

// Scenario: 戻すと重なって隠れていた滞在も戻る
#[tokio::test]
async fn restore_unhides_overlapping_stays() {
    let app = app().await;
    let user = testdb::user();
    put_dwell(&app.pool, user, "2026-09-23T08:00:00+09:00", 110).await;
    put_dwell(&app.pool, user, "2026-09-23T10:00:00+09:00", 60).await;
    put_dwell(&app.pool, user, "2026-09-23T11:10:00+09:00", 50).await;
    stay_store::rebuild_day(&app.pool, user, testdb::date("2026-09-23"))
        .await
        .unwrap();
    let erased: uuid::Uuid = sqlx::query_scalar(
        "SELECT id FROM core.event_live
          WHERE user_id = $1 AND logical_source = 's01-stay'
            AND event_time = $2",
    )
    .bind(user)
    .bind(t("2026-09-23T10:00:00+09:00"))
    .fetch_one(&app.pool)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'user' WHERE id = $1")
        .bind(erased)
        .execute(&app.pool)
        .await
        .unwrap();
    stay_store::set_criteria(&app.pool, user, None, None, Some(15))
        .await
        .unwrap();
    stay_store::rebuild_day(&app.pool, user, testdb::date("2026-09-23"))
        .await
        .unwrap();
    let hidden: uuid::Uuid = sqlx::query_scalar(
        "SELECT id FROM core.event
          WHERE user_id = $1 AND logical_source = 's01-stay'
            AND event_time = $2 AND deleted_by = 'rebuild:erased-range'",
    )
    .bind(user)
    .bind(t("2026-09-23T08:00:00+09:00"))
    .fetch_one(&app.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.deletion_ledger
           (event_id, user_id, logical_source, action, cause_event_id, mark)
         VALUES ($1, $2, 's01-stay', 'erase', $1, 'user')",
    )
    .bind(erased)
    .bind(user)
    .execute(&app.pool)
    .await
    .unwrap();

    restore(&app, vec![erased], None).await.unwrap();

    assert_eq!(mark(&app.pool, hidden).await, (None, None));
    let span: (DateTime<Utc>, serde_json::Value) =
        sqlx::query_as("SELECT event_time, payload FROM core.event_live WHERE id = $1")
            .bind(hidden)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(
        stay_store::span_of(span.0, &span.1).1,
        t("2026-09-23T12:00:00+09:00")
    );
}

// Scenario: 戻すと台帳に戻した行が積まれ、消した行は残る
#[tokio::test]
async fn restore_writes_ledger() {
    let pool = testdb::pool().await;
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..crate::App::for_test(pool, TOKEN)
    };
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-24T10:00:00+09:00",
        "2026-09-24T11:00:00+09:00",
    )
    .await;
    let location = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-24T10:30:00+09:00"),
    )
    .await;
    erase(&app, stay, None).await.unwrap();
    restore(&app, vec![stay], None).await.unwrap();

    for event_id in [stay, location] {
        let rows: Vec<(String, uuid::Uuid, String)> = sqlx::query_as(
            "SELECT action, cause_event_id, mark FROM core.deletion_ledger
              WHERE event_id = $1 ORDER BY seq",
        )
        .bind(event_id)
        .fetch_all(&app.pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "erase");
        assert_eq!(rows[1].0, "restore");
        assert_eq!(rows[0].1, stay);
        assert_eq!(rows[1].1, stay);
        assert_eq!(rows[0].2, rows[1].2);
    }
}

// Scenario: 消した滞在は 1 日の並びの滞在として出なくなる
// Scenario: 消した位置から作られていた滞在は一覧から外れる
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

fn location_item(id: uuid::Uuid, user: uuid::Uuid, at: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "user_id": user,
        "logical_source": "c01-location",
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": at,
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": r#"{"lat":35.68,"lon":139.76,"acc_m":10}"#,
        "payload": {"lat":35.68,"lon":139.76,"acc_m":10},
    })
}

#[tokio::test]
async fn erase_rebuilds_day() {
    let pool = testdb::pool().await;
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = calls.clone();
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(move |pool, user, day| {
            let captured = captured.clone();
            Box::pin(async move {
                let committed: bool = sqlx::query_scalar(
                    "SELECT deleted_at IS NOT NULL FROM core.event
                      WHERE user_id = $1 AND logical_source = 's01-stay'",
                )
                .bind(user)
                .fetch_one(&pool)
                .await?;
                captured.lock().unwrap().push((day, committed));
                Ok(())
            })
        }),
        ..crate::App::for_test(pool, TOKEN)
    };
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-07T23:50:00+09:00",
        "2026-09-08T00:10:00+09:00",
    )
    .await;

    erase(&app, stay, None).await.unwrap();

    assert_eq!(
        *calls.lock().unwrap(),
        vec![
            (testdb::date("2026-09-07"), true),
            (testdb::date("2026-09-08"), true),
        ],
        "JST で触れた各日を、削除の commit 後に一度ずつ作り直していない"
    );
}

// Scenario: 作り直しが失敗しても消したことは残る
#[tokio::test]
async fn erase_survives_rebuild_failure() {
    let pool = testdb::pool().await;
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| {
            Box::pin(async { Err(anyhow::anyhow!("lat=35.68 lon=139.76")) })
        }),
        ..crate::App::for_test(pool, TOKEN)
    };
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-09T10:00:00+09:00",
        "2026-09-09T11:00:00+09:00",
    )
    .await;
    let location = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-09T10:30:00+09:00"),
    )
    .await;
    let captured = Captured::default();
    let sink = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || sink.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);

    let response = erase(&app, stay, None).await.unwrap();
    drop(guard);

    assert_eq!(
        response,
        serde_json::json!({"erased":{"stays":1,"locations":1}})
    );
    assert_eq!(mark(&app.pool, stay).await.1.as_deref(), Some("user"));
    assert_eq!(
        mark(&app.pool, location).await.1.as_deref(),
        Some("user:cascade")
    );
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    let failure = log
        .lines()
        .find(|line| line.contains("kind=\"stay.rebuild\"") && line.contains("ERROR"))
        .expect("作り直しの失敗が stay.rebuild として残っていない");
    assert!(
        failure.contains(&user.to_string()),
        "利用者が無い: {failure}"
    );
    assert!(failure.contains("day=2026-09-09"), "日が無い: {failure}");
    assert!(failure.contains("failure=other"), "種別が無い: {failure}");
    assert!(
        !failure.contains("35.68"),
        "位置の値がログに出た: {failure}"
    );
    assert!(
        !failure.contains("139.76"),
        "位置の値がログに出た: {failure}"
    );
}

// Scenario: 消した時間に後から届いた位置は行として残る
// Scenario: 消した時間に後から届いた位置は読み出しに出ない
// Scenario: 後から届いて印が付いた位置は台帳に消した行を持つ
// Scenario: 消した時間の外に届いた位置には印が付かない
#[tokio::test]
async fn late_arrival_is_marked() {
    let pool = testdb::pool().await;
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..crate::App::for_test(pool, TOKEN)
    };
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-10T10:00:00+09:00",
        "2026-09-10T11:00:00+09:00",
    )
    .await;
    erase(&app, stay, None).await.unwrap();
    let late = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-10T10:30:00+09:00"),
    )
    .await;
    let outside = put_event(
        &app.pool,
        user,
        "c01-location",
        t("2026-09-10T11:30:00+09:00"),
    )
    .await;

    stay_store::rebuild_day(&app.pool, user, testdb::date("2026-09-10"))
        .await
        .unwrap();

    assert_eq!(mark(&app.pool, late).await.1.as_deref(), Some("user:late"));
    assert_eq!(mark(&app.pool, outside).await, (None, None));
    assert_eq!(live_count(&app.pool, user, "c01-location").await, 1);
    let row_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM core.event WHERE id = $1)")
            .bind(late)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(row_exists, "後から届いた位置の行を捨てた");
    let ledger: Vec<(String, uuid::Uuid, String)> = sqlx::query_as(
        "SELECT action, cause_event_id, mark FROM core.deletion_ledger WHERE event_id = $1",
    )
    .bind(late)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(ledger, vec![("erase".into(), stay, "user:late".into())]);

    stay_store::rebuild_day(&app.pool, user, testdb::date("2026-09-10"))
        .await
        .unwrap();
    let ledger_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.deletion_ledger WHERE event_id = $1")
            .bind(late)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(ledger_count, 1, "同じ後着位置の台帳を二度書いた");

    let axum::Json(hidden) = crate::events(State(app.clone()), auth()).await.unwrap();
    assert!(
        hidden.iter().all(|record| record.id != late),
        "消した時間の後着位置が読み出しに出ている"
    );

    // Task 4 の API を先取りせず、同じ原因の印を外すという戻す操作の DB 結果を作る。
    sqlx::query(
        "UPDATE core.event SET deleted_at = NULL, deleted_by = NULL
          WHERE id = $1 OR id IN (
            SELECT event_id FROM core.deletion_ledger
             WHERE cause_event_id = $1 AND action = 'erase'
          )",
    )
    .bind(stay)
    .execute(&app.pool)
    .await
    .unwrap();

    let axum::Json(restored) = crate::events(State(app.clone()), auth()).await.unwrap();
    assert!(
        restored.iter().any(|record| record.id == late),
        "戻した後着位置が実際の読み出し経路に出ていない"
    );
}

/// 削除より先に始まった取り込みが、削除の commit 後に確定する競合を固定する。
#[tokio::test]
async fn late_arrival_is_marked_after_concurrent_commit() {
    let app = app().await;
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-12T10:00:00+09:00",
        "2026-09-12T11:00:00+09:00",
    )
    .await;
    let late = uuid::Uuid::new_v4();
    let mut ingest_tx = app.pool.begin().await.unwrap();
    let raw = r#"{"lat":35.68,"lon":139.76,"acc_m":10}"#;
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,'c01-location','test','collected',$3,540,'Asia/Tokyo',1,$4,$5,$5::jsonb)",
    )
    .bind(late)
    .bind(user)
    .bind(t("2026-09-12T10:30:00+09:00"))
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(raw)
    .execute(&mut *ingest_tx)
    .await
    .unwrap();

    erase(&app, stay, None).await.unwrap();
    ingest_tx.commit().await.unwrap();

    let (ingested_at, deleted_at): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
        "SELECT e.ingest_time, s.deleted_at
           FROM core.event e JOIN core.event s ON s.id = $2
          WHERE e.id = $1",
    )
    .bind(late)
    .bind(stay)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(
        ingested_at < deleted_at,
        "回帰テストの前提（取り込み transaction が削除より先に開始）が成立していない"
    );

    stay_store::rebuild_day(&app.pool, user, testdb::date("2026-09-12"))
        .await
        .unwrap();

    assert_eq!(mark(&app.pool, late).await.1.as_deref(), Some("user:late"));
}

#[tokio::test]
async fn ingest_response_unchanged_after_erase() {
    let app = app().await;
    let user = testdb::user();
    let stay = put_stay(
        &app.pool,
        user,
        "2026-09-11T10:00:00+09:00",
        "2026-09-11T11:00:00+09:00",
    )
    .await;
    erase(&app, stay, None).await.unwrap();
    let id = uuid::Uuid::new_v4();
    let item = location_item(id, user, "2026-09-11T10:30:00+09:00");

    let (first_code, axum::Json(first)) = crate::ingest(
        axum::extract::State(app.clone()),
        auth(),
        axum::Json(item.clone()),
    )
    .await
    .unwrap();
    let (second_code, axum::Json(second)) =
        crate::ingest(axum::extract::State(app.clone()), auth(), axum::Json(item))
            .await
            .unwrap();

    assert_eq!(first_code, axum::http::StatusCode::OK);
    assert_eq!(second_code, axum::http::StatusCode::OK);
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::json!([{"id":id,"duplicate":false,"accepted":true,"error":null}])
    );
    assert_eq!(
        serde_json::to_value(second).unwrap(),
        serde_json::json!([{"id":id,"duplicate":true,"accepted":true,"error":null}])
    );
    assert_eq!(mark(&app.pool, id).await.1.as_deref(), Some("user:late"));
}
