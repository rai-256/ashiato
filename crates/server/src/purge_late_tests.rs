// SPDX-License-Identifier: AGPL-3.0-only
//! 本文ごと消した時間帯に、取り込みの口（`POST /ingest`）から後から届いた位置を、本物の PostgreSQL で確かめる
//! （ST23 Task 5 / design D8（仮）・D14。申し送り R8）。範囲の数え方と消去の核は `purge_tests` が見る。
#![allow(clippy::unwrap_used)]

use axum::extract::State;
use axum::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::purge_tests::{
    at, emptied, kept, purge_now, scene_basic, scene_erase, scene_mark_rebuild, scene_stay,
};
use crate::{testdb, App, IngestResult};

const TOKEN: &str = "test-token-0123456789abcdef";
const DAY: &str = "2026-10-01";

fn quiet_app(pool: PgPool) -> App {
    App {
        // 取り込みの後の作り直しは走らせない（この試験の外の振る舞い）
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..App::for_test(pool, TOKEN)
    }
}

pub(crate) async fn app() -> App {
    quiet_app(testdb::pool().await)
}

fn headers() -> axum::http::HeaderMap {
    let mut h = axum::http::HeaderMap::new();
    h.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    h
}

/// 位置の記録 1 件ぶんの JSON（`c01-location`。`lat` を変えれば内容の鍵が変わる）。
pub(crate) fn location(user: Uuid, hhmm: &str, lat: f64) -> serde_json::Value {
    let raw = format!(r#"{{"lat":{lat},"lon":139.76,"acc_m":10}}"#);
    serde_json::json!({
        "id": Uuid::new_v4(),
        "user_id": user,
        "logical_source": "c01-location",
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": format!("{DAY}T{hhmm}:00+09:00"),
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": raw,
        "payload": {"lat": lat, "lon": 139.76, "acc_m": 10},
    })
}

async fn send(app: &App, items: Vec<serde_json::Value>) -> Vec<IngestResult> {
    let (_, Json(results)) = crate::ingest(State(app.clone()), headers(), Json(items.into()))
        .await
        .expect("取り込み口");
    results
}

pub(crate) async fn send_one(app: &App, item: serde_json::Value) -> IngestResult {
    send(app, vec![item]).await.remove(0)
}

/// 10:00–11:00 の滞在を「消した」にして本文ごと消す。
pub(crate) async fn purged_scene(pool: &PgPool, user: Uuid) -> Uuid {
    let (stay, _) = scene_basic(pool, user).await;
    scene_erase(pool, stay).await;
    purge_now(pool, stay).await;
    stay
}

async fn mark_of(pool: &PgPool, id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT deleted_by FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn live(pool: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM core.event_live WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn row_count(pool: &PgPool, user: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = 'c01-location'",
    )
    .bind(user)
    .fetch_one(pool)
    .await
    .unwrap()
}

// Scenario: 本文ごと消した時間に端末から届いた位置は本文が消える
#[tokio::test]
async fn purge_late_device_location_inside_the_span_is_emptied() {
    let app = app().await;
    let user = testdb::user();
    purged_scene(&app.pool, user).await;

    let res = send_one(&app, location(user, "10:30", 35.1)).await;

    assert!(res.accepted, "取り込みの口が受理していない");
    let id = res.id.unwrap();
    assert_eq!(mark_of(&app.pool, id).await.as_deref(), Some("user:late"));
    assert!(emptied(&app.pool, id).await, "原文と解析済みが空でない");
}

// Scenario: 端末から届いた位置は本文を持ったまま確定しない
#[tokio::test]
async fn purge_late_ledger_row_is_written_in_the_ingest_transaction() {
    let app = app().await;
    let user = testdb::user();
    purged_scene(&app.pool, user).await;

    let id = send_one(&app, location(user, "10:30", 35.2))
        .await
        .id
        .unwrap();

    // 受理された直後に読む。台帳の行の txid が、取り込みにしか書かない稼働記録の行の xmin（取り込みのトランザクション）と同じ
    let same: Vec<bool> = sqlx::query_scalar(
        "SELECT l.txid::xid::text = c.xmin::text
           FROM core.erasure_ledger l
           JOIN core.event e ON e.id = l.event_id
           JOIN core.coverage c ON c.user_id = e.user_id AND c.logical_source = e.logical_source
                AND c.day = (e.event_time AT TIME ZONE 'Asia/Tokyo')::date
          WHERE l.event_id = $1 AND l.scope = 'event'",
    )
    .bind(id)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(
        same,
        vec![true],
        "台帳の行が取り込みと別のトランザクションにある"
    );
    assert!(emptied(&app.pool, id).await);
}

// Scenario: 後から届いて本文を消した記録は元の操作に結ばれる
#[tokio::test]
async fn purge_late_arrivals_are_tied_to_the_original_operation() {
    let app = app().await;
    let user = testdb::user();
    let stay = purged_scene(&app.pool, user).await;

    let first = send_one(&app, location(user, "10:20", 35.3))
        .await
        .id
        .unwrap();
    let second = send_one(&app, location(user, "10:40", 35.4))
        .await
        .id
        .unwrap();

    let rows: Vec<(Uuid, String, i32)> = sqlx::query_as(
        "SELECT l.event_id, op.kind, op.locations
           FROM core.erasure_operation parent
           JOIN core.erasure_operation op ON op.parent_id = parent.id
           JOIN core.erasure_ledger l ON l.operation_id = op.id
          WHERE parent.stay_id = $1 AND parent.kind = 'user'
          ORDER BY l.event_id",
    )
    .bind(stay)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    let mut want = vec![
        (first, "late".to_owned(), 1),
        (second, "late".to_owned(), 1),
    ];
    want.sort();
    assert_eq!(rows, want);
}

// Scenario: 本文ごと消した時間の外に届いた位置は本文が残る
#[tokio::test]
async fn purge_late_outside_the_span_keeps_the_body() {
    let app = app().await;
    let user = testdb::user();
    purged_scene(&app.pool, user).await;

    let id = send_one(&app, location(user, "11:30", 35.5))
        .await
        .id
        .unwrap();

    assert!(kept(&app.pool, id).await, "時間帯の外の本文が消えた");
    assert_eq!(mark_of(&app.pool, id).await, None);
    assert!(live(&app.pool, id).await, "読み出しに出ていない");
}

// Scenario: 一緒に本文を消した滞在のはみ出しは時間帯に入らない
#[tokio::test]
async fn purge_late_swallowed_stay_overflow_is_not_in_the_span() {
    let app = app().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&app.pool, user).await;
    let swallowed = scene_stay(
        &app.pool,
        user,
        &format!("{DAY}T10:30:00+09:00"),
        &format!("{DAY}T11:30:00+09:00"),
    )
    .await;
    scene_mark_rebuild(&app.pool, swallowed, "rebuild:absorbed").await;
    scene_erase(&app.pool, stay).await;
    purge_now(&app.pool, stay).await;

    let id = send_one(&app, location(user, "11:15", 35.6))
        .await
        .id
        .unwrap();

    assert!(kept(&app.pool, id).await, "はみ出しの時間の本文が消えた");
    assert!(live(&app.pool, id).await);
}

// Scenario: 範囲の行が無い時間帯に後から届いた記録は本文を消さない
#[tokio::test]
async fn purge_late_without_a_span_row_marks_only() {
    let app = app().await;
    let user = testdb::user();
    // 範囲の行を持たない、本文を消した滞在（移行の前の手作業・壊れた行）を、門を通して作る
    let stay = Uuid::new_v4();
    let mut tx = app.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload, deleted_at, deleted_by)
         VALUES ($1, $2, 's01-stay', $1::text, 'derived', $3, 540, 'Asia/Tokyo', 1, $1::text,
                 '', '{}'::jsonb, now(), 'user')",
    )
    .bind(stay)
    .bind(user)
    .bind(at(&format!("{DAY}T10:00:00+09:00")))
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.deletion_ledger (event_id, user_id, logical_source, action, cause_event_id, mark)
         VALUES ($1, $2, 's01-stay', 'erase', $1, 'user')",
    )
    .bind(stay)
    .bind(user)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let id = send_one(&app, location(user, "15:00", 35.7))
        .await
        .id
        .unwrap();

    assert_eq!(mark_of(&app.pool, id).await.as_deref(), Some("user:late"));
    assert!(kept(&app.pool, id).await, "範囲の行が無いのに本文を消した");
    let ops: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.erasure_operation WHERE stay_id = $1")
            .bind(stay)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(ops, 0);
}

// Scenario: 本文を消した位置を同じ内容で送り直しても戻らない
#[tokio::test]
async fn purge_late_resending_an_emptied_location_does_not_bring_it_back() {
    let app = app().await;
    let user = testdb::user();
    let stay = scene_stay(
        &app.pool,
        user,
        &format!("{DAY}T10:00:00+09:00"),
        &format!("{DAY}T11:00:00+09:00"),
    )
    .await;
    // 内容の鍵は取り込みの口が計算するので、最初の 1 件も取り込みの口から入れる
    let first = location(user, "10:30", 35.68);
    let stored = send_one(&app, first.clone()).await.id.unwrap();
    scene_erase(&app.pool, stay).await;
    purge_now(&app.pool, stay).await;
    assert!(emptied(&app.pool, stored).await, "前提: 本文は消えている");
    let before = row_count(&app.pool, user).await;

    // 同じ内容を別の識別子で送り直す
    let mut again = first;
    again["id"] = serde_json::json!(Uuid::new_v4());
    let sent = send_one(&app, again).await;

    assert!(sent.accepted);
    assert!(sent.duplicate, "重複として受理されていない");
    assert!(emptied(&app.pool, stored).await, "本文が戻った");
    assert_eq!(row_count(&app.pool, user).await, before, "行が増えた");
}

// Scenario: 作り直しが失敗しても後から届いた位置に印が付く
#[tokio::test]
async fn purge_late_mark_survives_a_failing_rebuild() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    let app = App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| {
            Box::pin(async { Err(anyhow::anyhow!("作り直しの失敗")) })
        }),
        ..App::for_test(pool.clone(), TOKEN)
    };

    let res = send_one(&app, location(user, "10:30", 35.8)).await;

    assert!(res.accepted);
    let id = res.id.unwrap();
    assert_eq!(mark_of(&pool, id).await.as_deref(), Some("user:late"));
    assert!(!live(&pool, id).await, "読み出しに出ている");
    let x = crate::export::excluded_sql("now()");
    let sql = format!(
        "WITH {} SELECT e.id FROM core.event_live e WHERE e.id = $1 AND {}",
        x.ctes(),
        x.event_kept("e")
    );
    let exported: Vec<(Uuid,)> = sqlx::query_as(&sql)
        .bind(id)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(exported.is_empty(), "書き出しに出ている");
}

// Scenario: 本文ごと消す操作と同じ時間帯の取り込みが同時に走っても位置の本文は残らない
#[tokio::test]
async fn purge_late_concurrent_purge_and_ingest_leave_no_body() {
    let pool = testdb::pool().await;
    let other = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    let counts = crate::purge_tests::scope_counts(&pool, stay).await;
    let app_b = quiet_app(other.clone());
    let item = location(user, "10:30", 35.9);
    let id: Uuid = serde_json::from_value(item["id"].clone()).unwrap();

    let purge = tokio::spawn({
        let pool = pool.clone();
        async move { crate::deletion::purge(&pool, stay, None, counts).await }
    });
    let ingest = tokio::spawn(async move { send(&app_b, vec![item]).await });
    let (purged, sent) = (purge.await.unwrap(), ingest.await.unwrap());

    // 後から届いた位置は取り込みの transaction の中で本文ごと空にされるので、どちらが先でも数え直した件数は変わらない
    assert!(
        purged.is_ok(),
        "本文ごと消す操作が成功していない: {purged:?}"
    );
    assert!(sent[0].accepted);
    assert!(emptied(&pool, id).await, "10:30 の位置の本文が残っている");
}
