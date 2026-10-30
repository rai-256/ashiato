// SPDX-License-Identifier: AGPL-3.0-only
//! 1 日の並びで、本文ごと消した区間にすっぽり入る消した滞在が入れ子の項目になり、
//! 後から本文を消した件数が親の区間に足されることを本物の PostgreSQL で確かめる（CH08 Task 5 / design D6）。
//! 画面の側は `web/e2e/ch08-nested.spec.ts` が見る。
#![allow(clippy::unwrap_used)]

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::purge_late_tests::{app, location, send_one};
use crate::purge_tests::{at, purge_now, scene_erase, scene_location, scene_stay};
use crate::stay_store::{self, DayEntry, EntryKind};
use crate::testdb;

const DAY: &str = "2026-10-01";

fn t(hhmm: &str) -> DateTime<Utc> {
    at(&format!("{DAY}T{hhmm}:00+09:00"))
}

async fn stay(pool: &PgPool, user: Uuid, from: &str, to: &str) -> Uuid {
    scene_stay(pool, user, &t(from).to_rfc3339(), &t(to).to_rfc3339()).await
}

/// 消してから本文ごと消す。
async fn purge(pool: &PgPool, id: Uuid) {
    scene_erase(pool, id).await;
    purge_now(pool, id).await;
}

async fn erased_on(pool: &PgPool, user: Uuid, date: &str) -> Vec<DayEntry> {
    stay_store::day_view(
        pool,
        user,
        testdb::date(date),
        at("2027-01-01T00:00:00+09:00"),
    )
    .await
    .unwrap()
    .entries
    .into_iter()
    .filter(|e| e.kind == EntryKind::Erased)
    .collect()
}

async fn erased(pool: &PgPool, user: Uuid) -> Vec<DayEntry> {
    erased_on(pool, user, DAY).await
}

// Scenario: 本文ごと消した区間はすっぽり入る消した滞在を入れ子の項目に持つ
#[tokio::test]
async fn purge_nested_erased_stay_inside_is_a_nested_item() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let b = stay(&pool, user, "10:15", "10:45").await;
    scene_erase(&pool, b).await;
    let a = stay(&pool, user, "10:00", "11:00").await;
    purge(&pool, a).await;

    let rows = erased(&pool, user).await;

    assert_eq!(
        rows.len(),
        1,
        "10:15 – 10:45 の「消した」の区間が残っている"
    );
    assert_eq!((rows[0].start, rows[0].end), (t("10:00"), t("11:00")));
    assert!(rows[0].purged);
    let nested = rows[0].nested.as_ref().expect("入れ子の項目が無い");
    assert_eq!(nested.len(), 1);
    assert_eq!((nested[0].start, nested[0].end), (t("10:15"), t("10:45")));
    assert_eq!(nested[0].stay_ids, vec![b]);
    assert!(!nested[0].purged);
}

// Scenario: 本文ごと消した入れ子の滞在は別の区間にならない
#[tokio::test]
async fn purge_nested_purged_stay_inside_is_not_a_span() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let a = stay(&pool, user, "10:00", "11:00").await;
    purge(&pool, a).await;
    let b = stay(&pool, user, "10:15", "10:45").await;
    purge(&pool, b).await;

    let rows = erased(&pool, user).await;

    assert_eq!(rows.len(), 1, "本文ごと消した区間が 2 つ以上ある");
    assert_eq!((rows[0].start, rows[0].end), (t("10:00"), t("11:00")));
    assert_eq!(rows[0].purge_stay_id, Some(a));
    let nested = rows[0].nested.as_ref().expect("入れ子の項目が無い");
    assert_eq!(nested.len(), 1);
    assert_eq!((nested[0].start, nested[0].end), (t("10:15"), t("10:45")));
    assert!(nested[0].purged, "本文ごと消した印が無い");
}

// Scenario: 作り直しで隠れて本文を消した滞在も後から本文を消した件数に入る
#[tokio::test]
async fn purge_nested_hidden_by_rebuild_counts_as_late() {
    let app = app().await;
    let user = testdb::user();
    for when in ["10:00", "11:00"] {
        scene_location(&app.pool, user, &format!("{DAY}T{when}:00+09:00")).await;
    }
    let a = stay(&app.pool, user, "10:00", "11:00").await;
    purge(&app.pool, a).await;
    send_one(&app, location(user, "10:30", 35.1)).await;
    let l = stay(&app.pool, user, "10:30", "11:30").await;

    stay_store::rebuild_day(&app.pool, user, testdb::date(DAY))
        .await
        .unwrap();

    let emptied = crate::purge_tests::emptied(&app.pool, l).await;
    assert!(emptied, "重なる滞在の本文が消えていない");
    let rows = erased(&app.pool, user).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].late_purged, Some(2));
}

// Scenario: 入れ子の滞在の後から本文を消した件数は親の区間に足される
#[tokio::test]
async fn purge_nested_late_count_of_nested_goes_to_the_parent() {
    let app = app().await;
    let user = testdb::user();
    // 後からの位置は、そのとき本文ごと消してあった B の操作に付く（A はまだ消していない）。
    // その後で A を本文ごと消すと B は入れ子になり、B の操作の件数が A の区間に足される。
    let b = stay(&app.pool, user, "10:15", "10:45").await;
    purge(&app.pool, b).await;
    send_one(&app, location(user, "10:30", 35.1)).await;
    let a = stay(&app.pool, user, "10:00", "11:00").await;
    purge(&app.pool, a).await;

    let rows = erased(&app.pool, user).await;

    assert_eq!(rows.len(), 1, "本文ごと消した区間が 1 つでない");
    assert_eq!((rows[0].start, rows[0].end), (t("10:00"), t("11:00")));
    assert_eq!(rows[0].late_purged, Some(1));
}

// Scenario: 時間が同じ 2 つの本文ごと消した滞在は先に消した方が区間になる
#[tokio::test]
async fn purge_nested_same_time_purged_stays_first_one_is_the_span() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let first = stay(&pool, user, "10:00", "11:00").await;
    let second = stay(&pool, user, "10:00", "11:00").await;
    scene_erase(&pool, first).await;
    scene_erase(&pool, second).await;
    purge_now(&pool, first).await;
    purge_now(&pool, second).await;

    let rows = erased(&pool, user).await;

    assert_eq!(rows.len(), 1, "本文ごと消した区間が 1 つでない");
    assert_eq!(rows[0].purge_stay_id, Some(first));
    let nested = rows[0].nested.as_ref().expect("入れ子の項目が無い");
    assert_eq!(nested.len(), 1);
    assert_eq!(nested[0].stay_ids, vec![second]);
    assert!(nested[0].purged);
}

// 日の境: 日をまたぐ消した滞在は、区間に覆われる日では入れ子、残る日では「消した」の断片
#[tokio::test]
async fn purge_nested_across_midnight_is_nested_only_on_the_covered_day() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let b = scene_stay(
        &pool,
        user,
        "2026-10-01T23:30:00+09:00",
        "2026-10-02T00:30:00+09:00",
    )
    .await;
    scene_erase(&pool, b).await;
    let a = scene_stay(
        &pool,
        user,
        "2026-10-02T00:00:00+09:00",
        "2026-10-02T01:00:00+09:00",
    )
    .await;
    purge(&pool, a).await;

    let before = erased_on(&pool, user, "2026-10-01").await;
    let after = erased_on(&pool, user, "2026-10-02").await;

    assert_eq!(before.len(), 1);
    assert!(!before[0].purged);
    assert_eq!(before[0].stay_ids, Some(vec![b]));
    assert_eq!(before[0].nested, None);
    assert_eq!(after.len(), 1);
    assert!(after[0].purged);
    assert_eq!(
        after[0].nested.as_ref().map(|n| n[0].stay_ids.clone()),
        Some(vec![b])
    );
}

// 2 つの区間に跨る: 2 つの本文ごと消した区間の和に覆われる消した滞在は、重なりの大きい（同じなら早い）区間の下に入る
#[tokio::test]
async fn purge_nested_stay_over_two_spans_goes_under_the_earlier_one() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let b = stay(&pool, user, "10:00", "12:00").await;
    scene_erase(&pool, b).await;
    let a1 = stay(&pool, user, "10:00", "11:00").await;
    let a2 = stay(&pool, user, "11:00", "12:00").await;
    purge(&pool, a1).await;
    purge(&pool, a2).await;

    let rows = erased(&pool, user).await;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].purge_stay_id, Some(a1));
    assert_eq!(
        rows[0].nested.as_ref().map(|n| n[0].stay_ids.clone()),
        Some(vec![b])
    );
    assert_eq!(rows[1].nested, None);
}

// 親の選び方: 長い区間でも、ほかの本文ごと消した区間の和に覆われるなら入れ子になり、短い 2 つが区間として残る（D6 の 2）
#[tokio::test]
async fn purge_nested_long_stay_covered_by_shorter_union_is_nested() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let x = stay(&pool, user, "10:10", "10:50").await;
    let s1 = stay(&pool, user, "10:00", "10:30").await;
    let s2 = stay(&pool, user, "10:25", "11:00").await;
    purge(&pool, x).await;
    purge(&pool, s1).await;
    purge(&pool, s2).await;

    let rows = erased(&pool, user).await;

    assert_eq!(rows.len(), 2, "本文ごと消した区間が 3 つ並んでいる");
    assert_eq!(rows[0].purge_stay_id, Some(s1));
    assert_eq!(rows[1].purge_stay_id, Some(s2));
    let nested_ids: Vec<_> = rows
        .iter()
        .flat_map(|r| r.nested.iter().flatten().map(|n| n.stay_ids.clone()))
        .collect();
    assert_eq!(nested_ids, vec![vec![x]]);
}

// まとめの規則: 重なるか隣り合う本文を消していない入れ子は 1 つにまとめ、識別子を全部持つ
#[tokio::test]
async fn purge_nested_unpurged_neighbours_are_merged_and_purged_ones_are_not() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let b1 = stay(&pool, user, "10:10", "10:30").await;
    let b2 = stay(&pool, user, "10:30", "10:50").await;
    scene_erase(&pool, b1).await;
    scene_erase(&pool, b2).await;
    let a = stay(&pool, user, "10:00", "11:00").await;
    purge(&pool, a).await;
    let c1 = stay(&pool, user, "10:55", "10:58").await;
    purge(&pool, c1).await;
    let c2 = stay(&pool, user, "10:58", "10:59").await;
    purge(&pool, c2).await;

    let rows = erased(&pool, user).await;

    assert_eq!(rows.len(), 1);
    let nested = rows[0].nested.clone().expect("入れ子の項目が無い");
    let mut expected = vec![b1, b2];
    expected.sort_unstable();
    assert_eq!(nested.len(), 3, "{nested:?}");
    assert_eq!((nested[0].start, nested[0].end), (t("10:10"), t("10:50")));
    assert_eq!(nested[0].stay_ids, expected);
    assert!(!nested[0].purged);
    assert!(nested[1].purged && nested[2].purged);
}

// 範囲の行が無い本文を消した滞在（ST23 D14。その日全体として読む窓）は入れ子の親にならない（design D10（仮））。
// 窓は ST23 のとおり自分の区間として並び、ほかの本文ごと消した区間とその入れ子の項目を取り込まない。
#[tokio::test]
async fn purge_nested_day_window_without_a_range_row_is_not_a_parent() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let b = stay(&pool, user, "10:15", "10:45").await;
    scene_erase(&pool, b).await;
    let a = stay(&pool, user, "10:00", "11:00").await;
    purge(&pool, a).await;
    let window = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload, deleted_at, deleted_by)
         VALUES ($1, $2, 's01-stay', $1::text, 'derived', $3, 540, 'Asia/Tokyo', 1, $1::text,
                 '', '{}'::jsonb, now(), 'user')",
    )
    .bind(window)
    .bind(user)
    .bind(t("15:00"))
    .execute(&pool)
    .await
    .unwrap();

    let rows = erased(&pool, user).await;

    let ids: Vec<_> = rows.iter().map(|r| r.purge_stay_id).collect();
    assert_eq!(ids, vec![Some(window), Some(a)], "{rows:?}");
    assert_eq!(
        (rows[0].start, rows[0].end),
        (t("00:00"), at("2026-10-02T00:00:00+09:00"))
    );
    assert!(rows[0].nested.is_none(), "窓が入れ子を持った: {rows:?}");
    assert_eq!(rows[0].late_purged, Some(0));
    assert_eq!((rows[1].start, rows[1].end), (t("10:00"), t("11:00")));
    let nested = rows[1].nested.as_ref().expect("入れ子の項目が無い");
    assert_eq!(nested.len(), 1);
    assert_eq!(nested[0].stay_ids, vec![b]);
    assert!(!nested[0].purged);
}
