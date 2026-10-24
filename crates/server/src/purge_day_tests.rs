// SPDX-License-Identifier: AGPL-3.0-only
//! 1 日の並び（`GET /day` の読み出し）に出る「本文ごと消した」の区間を、本物の PostgreSQL で確かめる
//! （ST23 Task 9.1 / design D12 / D13）。画面の側は `web/e2e/st23-purge.spec.ts` が見る。
#![allow(clippy::unwrap_used)]

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::purge_late_tests::{app, location, purged_scene, send_one};
use crate::purge_tests::{at, purge_now, scene_erase, scene_stay};
use crate::stay_store::{day_view, DayEntry, EntryKind};
use crate::testdb;

fn far_future() -> DateTime<Utc> {
    at("2027-01-01T00:00:00+09:00")
}

async fn erased_rows(pool: &sqlx::PgPool, user: Uuid) -> Vec<DayEntry> {
    day_view(pool, user, testdb::date("2026-10-01"), far_future())
        .await
        .unwrap()
        .entries
        .into_iter()
        .filter(|e| e.kind == EntryKind::Erased)
        .collect()
}

// Scenario: 本文ごと消した区間は印を持ち、戻すための識別子を持たない
#[tokio::test]
async fn purge_day_purged_span_has_the_mark_and_no_restore_ids() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let stay = scene_stay(
        &pool,
        user,
        "2026-10-01T10:05:00+09:00",
        "2026-10-01T11:48:00+09:00",
    )
    .await;
    scene_erase(&pool, stay).await;
    purge_now(&pool, stay).await;

    let rows = erased_rows(&pool, user).await;

    assert_eq!(rows.len(), 1);
    assert!(rows[0].purged, "本文ごと消した印が無い");
    assert_eq!(rows[0].stay_ids, None, "戻すための識別子を持っている");
    assert_eq!(rows[0].start, at("2026-10-01T10:05:00+09:00"));
    assert_eq!(rows[0].end, at("2026-10-01T11:48:00+09:00"));
}

// Scenario: 本文ごと消した区間は指定した滞在と後から届いた件数を持つ
#[tokio::test]
async fn purge_day_purged_span_carries_the_stay_and_the_late_count() {
    let app = app().await;
    let user = testdb::user();
    let stay = purged_scene(&app.pool, user).await;
    send_one(&app, location(user, "10:30", 35.1)).await;
    send_one(&app, location(user, "10:40", 35.2)).await;

    let rows = erased_rows(&app.pool, user).await;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].purge_stay_id, Some(stay));
    assert_eq!(rows[0].late_purged, Some(2));
}

// Scenario: 本文ごと消した時間と消した時間は隣り合ってもまとまらない
#[tokio::test]
async fn purge_day_adjacent_purged_and_erased_stay_apart() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let purged = scene_stay(
        &pool,
        user,
        "2026-10-01T10:00:00+09:00",
        "2026-10-01T11:00:00+09:00",
    )
    .await;
    let erased = scene_stay(
        &pool,
        user,
        "2026-10-01T11:00:00+09:00",
        "2026-10-01T11:30:00+09:00",
    )
    .await;
    scene_erase(&pool, purged).await;
    scene_erase(&pool, erased).await;
    purge_now(&pool, purged).await;

    let rows = erased_rows(&pool, user).await;

    assert_eq!(rows.len(), 2, "まとまっている");
    assert!(rows[0].purged);
    assert_eq!(rows[0].end, at("2026-10-01T11:00:00+09:00"));
    assert!(!rows[1].purged);
    assert_eq!(rows[1].start, at("2026-10-01T11:00:00+09:00"));
    assert_eq!(rows[1].stay_ids, Some(vec![erased]));
}

// Scenario: 本文ごと消した時間と重なる消した時間は差し引いて出る
#[tokio::test]
async fn purge_day_overlapping_erased_time_is_subtracted() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let purged = scene_stay(
        &pool,
        user,
        "2026-10-01T10:00:00+09:00",
        "2026-10-01T11:00:00+09:00",
    )
    .await;
    let erased = scene_stay(
        &pool,
        user,
        "2026-10-01T10:30:00+09:00",
        "2026-10-01T11:30:00+09:00",
    )
    .await;
    scene_erase(&pool, purged).await;
    scene_erase(&pool, erased).await;
    purge_now(&pool, purged).await;

    let rows = erased_rows(&pool, user).await;

    let spans: Vec<_> = rows.iter().map(|r| (r.purged, r.start, r.end)).collect();
    assert_eq!(
        spans,
        vec![
            (
                true,
                at("2026-10-01T10:00:00+09:00"),
                at("2026-10-01T11:00:00+09:00")
            ),
            (
                false,
                at("2026-10-01T11:00:00+09:00"),
                at("2026-10-01T11:30:00+09:00")
            ),
        ]
    );
    assert_eq!(rows[1].stay_ids, Some(vec![erased]));
}

// 画面が読む `GET /day` の応答にも、印・指定した滞在・件数が載り、戻すための識別子は載らない。
#[tokio::test]
async fn purge_day_response_carries_the_purge_fields() {
    let app = app().await;
    let user = testdb::user();
    let stay = purged_scene(&app.pool, user).await;
    send_one(&app, location(user, "10:30", 35.3)).await;

    let day = crate::day::day(&app.pool, user, testdb::date("2026-10-01"), far_future())
        .await
        .unwrap();
    let json = serde_json::to_value(&day).unwrap();
    let row = json["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "erased")
        .unwrap();

    assert_eq!(row["purged"], true);
    assert_eq!(row["purge_stay_id"], stay.to_string());
    assert_eq!(row["late_purged"], 1);
    assert!(
        row.get("stay_ids").is_none(),
        "戻すための識別子を持っている"
    );
}
