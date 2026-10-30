// SPDX-License-Identifier: AGPL-3.0-only
//! 作り直しが隠した、本文ごと消した時間帯と重なる滞在の本文が、同じまとまりで消えることを本物の PostgreSQL で確かめる
//! （CH08 Task 1 / design D2・D3）。本文ごと消す口の核は `purge_tests`、後から届いた位置は `purge_late_tests` が見る。
#![allow(clippy::unwrap_used)]

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::purge_tests::{
    at, emptied, kept, purge_now, scene_basic, scene_erase, scene_location, scene_mark_rebuild,
    scene_stay, scene_version,
};
use crate::stay_store::{self, DayEntry, EntryKind};
use crate::testdb;

const DAY: &str = "2026-10-01";

fn t(hhmm: &str) -> DateTime<Utc> {
    at(&format!("{DAY}T{hhmm}:00+09:00"))
}

async fn stay(pool: &PgPool, user: Uuid, from: &str, to: &str) -> Uuid {
    scene_stay(pool, user, &t(from).to_rfc3339(), &t(to).to_rfc3339()).await
}

async fn points(pool: &PgPool, user: Uuid, whens: &[&str]) {
    for when in whens {
        scene_location(pool, user, &format!("{DAY}T{when}:00+09:00")).await;
    }
}

async fn version(pool: &PgPool, id: Uuid, user: Uuid, no: i32, from: &str, to: &str) {
    scene_version(
        pool,
        id,
        user,
        no,
        &t(from).to_rfc3339(),
        &t(to).to_rfc3339(),
    )
    .await;
}

/// 10:00–11:00 の滞在を消して本文ごと消す。（滞在の識別子, 操作の識別子）
async fn purged(pool: &PgPool, user: Uuid) -> (Uuid, Uuid) {
    let (p, _) = scene_basic(pool, user).await;
    scene_erase(pool, p).await;
    let operation = purge_now(pool, p).await.operation_id.unwrap();
    (p, operation)
}

async fn rebuild(pool: &PgPool, user: Uuid) {
    stay_store::rebuild_day(pool, user, testdb::date(DAY))
        .await
        .unwrap();
}

async fn entries(pool: &PgPool, user: Uuid) -> Vec<DayEntry> {
    stay_store::day_view(
        pool,
        user,
        testdb::date(DAY),
        at("2027-01-01T00:00:00+09:00"),
    )
    .await
    .unwrap()
    .entries
}

/// (削除の時刻があるか, 削除の印)
async fn mark_of(pool: &PgPool, id: Uuid) -> (bool, Option<String>) {
    sqlx::query_as("SELECT deleted_at IS NOT NULL, deleted_by FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn listed(pool: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM core.event_live WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// (前の版の数, 本文の空になった前の版の数)
async fn versions_emptied(pool: &PgPool, id: Uuid) -> (i64, i64) {
    sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE raw = '' AND payload = '{}'::jsonb)
           FROM core.event_version WHERE event_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[derive(Debug, sqlx::FromRow)]
struct LateOperation {
    id: Uuid,
    parent_id: Option<Uuid>,
    stay_id: Uuid,
    stays: i32,
    locations: i32,
    versions: i32,
}

type SpanRow = (String, Option<i32>, DateTime<Utc>, DateTime<Utc>);

async fn late_operations(pool: &PgPool, user: Uuid) -> Vec<LateOperation> {
    sqlx::query_as(
        "SELECT id, parent_id, stay_id, stays, locations, versions
           FROM core.erasure_operation WHERE user_id = $1 AND kind = 'late' ORDER BY at, id",
    )
    .bind(user)
    .fetch_all(pool)
    .await
    .unwrap()
}

// Scenario: 作り直しで吸収された重なる滞在の本文も消える
#[tokio::test]
async fn purge_hidden_absorbed_overlapping_stay_is_emptied() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let l = stay(&pool, user, "10:30", "11:30").await;

    rebuild(&pool, user).await;

    assert!(emptied(&pool, l).await, "原文と解析済みが空でない");
    assert_eq!(mark_of(&pool, l).await, (true, Some("user:purged".into())));
    let erased: Vec<_> = entries(&pool, user)
        .await
        .into_iter()
        .filter(|e| e.kind == EntryKind::Erased)
        .map(|e| (e.start, e.end))
        .collect();
    assert_eq!(
        erased,
        vec![(t("10:00"), t("11:00"))],
        "本文ごと消した区間だけが並ばない（10:30 – 11:30 の「消した」の区間がある）"
    );
}

// Scenario: 作り直しで消した時間帯と重なるとして隠れた重なる滞在の本文も消える
#[tokio::test]
async fn purge_hidden_stay_hidden_by_another_erased_range_is_emptied() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let l = stay(&pool, user, "10:30", "11:40").await;
    let b2 = stay(&pool, user, "11:20", "11:25").await;
    scene_erase(&pool, b2).await;
    points(
        &pool,
        user,
        &["11:05", "11:12", "11:19", "11:26", "11:33", "11:40"],
    )
    .await;

    rebuild(&pool, user).await;

    assert!(emptied(&pool, l).await, "重なる滞在の本文が残っている");
    let (hidden, mark): (Uuid, Option<String>) = sqlx::query_as(
        "SELECT id, deleted_by FROM core.event
          WHERE user_id = $1 AND logical_source = 's01-stay' AND event_time = $2",
    )
    .bind(user)
    .bind(t("11:05"))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(mark.as_deref(), Some("rebuild:erased-range"));
    assert!(kept(&pool, hidden).await, "隠れた新しい滞在の本文が消えた");
}

// Scenario: 作り直しで隠れた重なる滞在の前の版の本文も消える
#[tokio::test]
async fn purge_hidden_versions_of_the_hidden_stay_are_emptied() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let l = stay(&pool, user, "10:30", "11:30").await;
    for no in 1..=3 {
        version(&pool, l, user, no, "10:20", "11:30").await;
    }

    rebuild(&pool, user).await;

    assert_eq!(versions_emptied(&pool, l).await, (3, 3));
}

// Scenario: 端が触れるだけの滞在は作り直しで隠れても本文が残る
#[tokio::test]
async fn purge_hidden_edge_touching_stay_keeps_its_body() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let touching = stay(&pool, user, "11:00", "12:00").await;

    rebuild(&pool, user).await;

    assert_eq!(
        mark_of(&pool, touching).await,
        (true, Some("rebuild:absorbed".into())),
        "隠れていない"
    );
    assert!(
        kept(&pool, touching).await,
        "端が触れるだけの滞在の本文が消えた"
    );
}

// Scenario: 範囲の行が無い本文を消した滞在の日に隠れた滞在は本文が残る
#[tokio::test]
async fn purge_hidden_day_window_without_a_range_row_keeps_the_body() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    // 範囲の行が無い本文を消した滞在（ST23 D14。その日全体として読む）
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload, deleted_at, deleted_by)
         VALUES ($1, $2, 's01-stay', $1::text, 'derived', $3, 540, 'Asia/Tokyo', 1, $1::text,
                 '', '{}'::jsonb, now(), 'user')",
    )
    .bind(id)
    .bind(user)
    .bind(t("09:00"))
    .execute(&pool)
    .await
    .unwrap();
    let l = stay(&pool, user, "15:00", "15:30").await;

    rebuild(&pool, user).await;

    assert_eq!(
        mark_of(&pool, l).await,
        (true, Some("rebuild:absorbed".into())),
        "隠れていない"
    );
    assert!(kept(&pool, l).await, "窓に範囲の行が無い滞在の本文が消えた");
}

// Scenario: 作り直しの前から隠れていた滞在の本文は作り直しで消えない
#[tokio::test]
async fn purge_hidden_stay_hidden_before_the_rebuild_keeps_its_body() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let touching = stay(&pool, user, "11:00", "12:00").await;
    scene_mark_rebuild(&pool, touching, "rebuild:erased-range").await;
    // 重なっていても、作り直しの前から隠れている（`Mark::Live` でない）なら消さない
    let hidden_overlap = stay(&pool, user, "10:30", "10:50").await;
    scene_mark_rebuild(&pool, hidden_overlap, "rebuild:absorbed").await;

    rebuild(&pool, user).await;

    assert!(kept(&pool, touching).await);
    assert!(kept(&pool, hidden_overlap).await);
}

// Scenario: 一覧に出ている重なる滞在は隠れるまで本文が残る
#[tokio::test]
async fn purge_hidden_listed_overlapping_stay_keeps_its_body_until_hidden() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let l = stay(&pool, user, "10:30", "11:30").await;
    points(&pool, user, &["11:10", "11:15", "11:20", "11:25", "11:30"]).await;

    rebuild(&pool, user).await;

    assert!(kept(&pool, l).await, "本文が消えた");
    assert!(listed(&pool, l).await, "一覧に出ていない");
}

// Scenario: 重なる滞在の本文が消せないと作り直しは何も残さない
#[tokio::test]
async fn purge_hidden_ledger_failure_rolls_the_rebuild_back() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let l = stay(&pool, user, "10:30", "11:30").await;

    let result =
        stay_store::rebuild_day_with_ledger_scope(&pool, user, testdb::date(DAY), "not-a-scope")
            .await;

    assert!(result.is_err(), "作り直しが成功した");
    assert_eq!(mark_of(&pool, l).await, (false, None));
    assert!(kept(&pool, l).await, "本文が消えた");
    assert!(listed(&pool, l).await, "一覧に出ていない");
    assert!(late_operations(&pool, user).await.is_empty());
}

// Scenario: 作り直しで本文を消した滞在は消した時間帯を広げない
#[tokio::test]
async fn purge_hidden_purged_stay_does_not_widen_the_erased_range() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    stay(&pool, user, "10:30", "11:30").await;
    rebuild(&pool, user).await;
    points(
        &pool,
        user,
        &[
            "11:10", "11:15", "11:20", "11:25", "11:30", "15:00", "15:05", "15:10", "15:15",
            "15:20", "15:25", "15:30",
        ],
    )
    .await;

    rebuild(&pool, user).await;

    let stays: Vec<_> = entries(&pool, user)
        .await
        .into_iter()
        .filter(|e| e.kind == EntryKind::Stay)
        .map(|e| (e.start, e.end))
        .collect();
    assert_eq!(
        stays,
        vec![(t("11:10"), t("11:30")), (t("15:00"), t("15:30"))]
    );
}

// Scenario: 作り直しで本文を消した滞在は後から本文を消した操作として台帳に残る
#[tokio::test]
async fn purge_hidden_ledger_gets_one_late_operation_row() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (p, operation) = purged(&pool, user).await;
    let l = stay(&pool, user, "10:30", "11:30").await;
    version(&pool, l, user, 1, "10:20", "11:30").await;
    version(&pool, l, user, 2, "10:30", "11:00").await;
    let before = late_operations(&pool, user).await.len();

    rebuild(&pool, user).await;

    let after = late_operations(&pool, user).await;
    assert_eq!(
        after.len(),
        before + 1,
        "後からの操作の行が 1 行増えていない"
    );
    let row = &after[after.len() - 1];
    assert_eq!(row.parent_id, Some(operation));
    assert_eq!(row.stay_id, p);
    assert_eq!(row.stays, 1, "滞在の件数");
    assert_eq!(row.locations, 0);
    assert_eq!(row.versions, 2, "前の版の件数");
    let ledger: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.erasure_ledger WHERE operation_id = $1 AND event_id = $2",
    )
    .bind(row.id)
    .bind(l)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ledger, 3, "記録ごと 1 行 + 前の版ごと 2 行");
}

// Scenario: 作り直しで本文を消した滞在の時間が範囲の行に残る
#[tokio::test]
async fn purge_hidden_span_rows_keep_the_times() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let l = stay(&pool, user, "10:30", "11:30").await;
    version(&pool, l, user, 1, "10:20", "11:30").await;

    rebuild(&pool, user).await;

    let operation = late_operations(&pool, user).await.remove(0).id;
    let spans: Vec<SpanRow> = sqlx::query_as(
        "SELECT scope, version_no, start_at, end_at FROM core.erasure_span
          WHERE operation_id = $1 AND event_id = $2 ORDER BY scope",
    )
    .bind(operation)
    .bind(l)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        spans,
        vec![
            ("event".to_string(), None, t("10:30"), t("11:30")),
            ("version".to_string(), Some(1), t("10:20"), t("11:30")),
        ]
    );
}

// Scenario: 2 つの本文ごと消した時間帯と重なる滞在は重なりの大きい方の操作に結ばれる
#[tokio::test]
async fn purge_hidden_larger_overlap_picks_the_parent_operation() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let second = stay(&pool, user, "11:00", "12:00").await;
    scene_erase(&pool, second).await;
    let second_operation = purge_now(&pool, second).await.operation_id.unwrap();
    stay(&pool, user, "10:40", "11:30").await;

    rebuild(&pool, user).await;

    let late = late_operations(&pool, user).await;
    assert_eq!(late.len(), 1);
    assert_eq!(late[0].parent_id, Some(second_operation));
    assert_eq!(late[0].stay_id, second);
}
