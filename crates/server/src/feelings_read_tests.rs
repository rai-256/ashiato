// SPDX-License-Identifier: AGPL-3.0-only
//! 主観の読み出し（ST17 / Task 4。design D6 / D7 / D8）。
//!
//! 前半（`st17_read_rules_`）は DB を持たない純粋な関数の試験、後半（`st17_read_api_`）は
//! 取り込み口で書いて `GET /feelings` で読む。**利用者で隔離する**（`testdb::user()`）。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::feelings::{
    self, day_feelings, DayFeelings, DayStatus, Phase, Placement, Record, ScaleOut,
    ABSORB_HOPS_MAX, RANGE_MAX_DAYS,
};
use crate::feelings_ingest_tests::{
    app, auth, item, nonce_for, put_stay, send, t, valence, write_day,
};
use crate::stay_store::Mark;
use crate::testdb;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

// ================================================================ 純粋な規則

fn day_rec(date: &str, written_at: &str) -> Record {
    Record {
        id: uuid::Uuid::new_v4(),
        placement: Placement::Day,
        event_time: t(&format!("{date}T00:00:00+09:00")),
        written_at: written_at.to_string(),
        ingested_at: t("2026-10-14T00:00:00Z"),
        scales: vec![ScaleOut {
            id: "valence".into(),
            version: 1,
            value: 1,
        }],
        note: None,
    }
}

fn stay_rec(start: &str, end: &str, written_at: &str, shown: bool) -> Record {
    let stay = uuid::Uuid::new_v4();
    Record {
        id: uuid::Uuid::new_v4(),
        placement: Placement::Stay {
            original: stay,
            shown: shown.then_some(stay),
            end: t(end),
        },
        event_time: t(start),
        written_at: written_at.to_string(),
        ingested_at: t("2026-10-14T00:00:00Z"),
        scales: Vec::new(),
        note: None,
    }
}

/// 1 日だけ読んで、その日を返す
fn one_day(recs: &[Record], first: Option<&str>, today: &str, date: &str) -> DayFeelings {
    let mut v = day_feelings(recs, first.map(d), d(today), d(date), d(date));
    assert_eq!(v.len(), 1);
    v.pop().unwrap()
}

fn phase_of_day(date: &str, written_at: &str) -> Phase {
    let day = one_day(&[day_rec(date, written_at)], Some(date), "2030-01-01", date);
    day.day.records[0].phase
}

// Scenario: 翌朝に書いた昨日の記録は後
#[test]
fn st17_read_rules_next_morning_is_after() {
    assert_eq!(
        phase_of_day("2026-10-07", "2026-10-08T08:10:00+09:00"),
        Phase::After
    );
}

// Scenario: 当日の夜に書いたその日の記録は前
#[test]
fn st17_read_rules_same_night_is_before() {
    assert_eq!(
        phase_of_day("2026-10-07", "2026-10-07T21:30:00+09:00"),
        Phase::Before
    );
}

// Scenario: 未来の日に書いた記録は前
#[test]
fn st17_read_rules_future_day_is_before() {
    assert_eq!(
        phase_of_day("2026-10-10", "2026-10-07T12:00:00+09:00"),
        Phase::Before
    );
}

// Scenario: 滞在が終わった後に書いた記録は後
#[test]
fn st17_read_rules_after_the_stay_ended_is_after() {
    let r = stay_rec(
        "2026-10-07T09:12:00+09:00",
        "2026-10-07T11:40:00+09:00",
        "2026-10-07T12:00:00+09:00",
        true,
    );
    let day = one_day(&[r], Some("2026-10-07"), "2030-01-01", "2026-10-07");
    assert_eq!(day.stays[0].records[0].phase, Phase::After);
    // 終わる前なら前（境は原文の target.end）
    let r = stay_rec(
        "2026-10-07T09:12:00+09:00",
        "2026-10-07T11:40:00+09:00",
        "2026-10-07T10:00:00+09:00",
        true,
    );
    let day = one_day(&[r], Some("2026-10-07"), "2030-01-01", "2026-10-07");
    assert_eq!(day.stays[0].records[0].phase, Phase::Before);
}

// Scenario: 日付が変わった直後に書いた前の日の記録は後
#[test]
fn st17_read_rules_just_after_midnight_is_after() {
    assert_eq!(
        phase_of_day("2026-10-07", "2026-10-08T00:00:30+09:00"),
        Phase::After
    );
    // 地域のずれが違う書き方でも、同じ瞬間なら同じ答え（UTC の 15:00:30 は +09:00 の 0:00:30）
    assert_eq!(
        phase_of_day("2026-10-07", "2026-10-07T15:00:30+00:00"),
        Phase::After
    );
}

// Scenario: 日の記録がある日は記入あり
#[test]
fn st17_read_rules_day_record_is_recorded() {
    let r = day_rec("2026-10-05", "2026-10-05T21:00:00+09:00");
    let day = one_day(&[r], Some("2026-10-05"), "2026-10-07", "2026-10-05");
    assert_eq!(day.status, DayStatus::Recorded);
}

// Scenario: 滞在にだけ書いた日も記入あり
#[test]
fn st17_read_rules_stay_only_is_recorded() {
    let r = stay_rec(
        "2026-10-05T09:00:00+09:00",
        "2026-10-05T10:00:00+09:00",
        "2026-10-05T11:00:00+09:00",
        true,
    );
    let day = one_day(&[r], Some("2026-10-05"), "2026-10-07", "2026-10-05");
    assert_eq!(day.status, DayStatus::Recorded);
    assert!(day.day.records.is_empty());
}

// Scenario: 前だけ書いた日も記入あり
#[test]
fn st17_read_rules_before_only_is_recorded() {
    let r = day_rec("2026-10-05", "2026-10-03T12:00:00+09:00");
    let day = one_day(&[r], Some("2026-10-03"), "2026-10-07", "2026-10-05");
    assert_eq!(day.status, DayStatus::Recorded);
    assert!(day.day.latest.after.is_none());
}

// Scenario: 書いていない日は未記入
#[test]
fn st17_read_rules_no_record_is_missing() {
    let day = one_day(&[], Some("2026-10-01"), "2026-10-07", "2026-10-04");
    assert_eq!(day.status, DayStatus::Missing);
}

// Scenario: 最初の記録より前の日は使い始める前
#[test]
fn st17_read_rules_before_the_first_record_is_before_start() {
    let day = one_day(&[], Some("2026-10-01"), "2026-10-07", "2026-09-30");
    assert_eq!(day.status, DayStatus::BeforeStart);
    // 記録がまだ 1 件も無い利用者の過去の日も「使い始める前」
    let day = one_day(&[], None, "2026-10-07", "2026-10-04");
    assert_eq!(day.status, DayStatus::BeforeStart);
}

// Scenario: 未来の日は未記入に数えない
#[test]
fn st17_read_rules_future_is_not_missing() {
    let day = one_day(&[], Some("2026-10-01"), "2026-10-07", "2026-10-08");
    assert_eq!(day.status, DayStatus::Future);
}

// Scenario: 今日まだ書いていなければ今日は未記入
#[test]
fn st17_read_rules_today_without_a_record_is_missing() {
    let day = one_day(&[], Some("2026-10-01"), "2026-10-07", "2026-10-07");
    assert_eq!(day.status, DayStatus::Missing);
}

// Scenario: 日をまたぐ滞在の記録は始まりの日に数える
#[test]
fn st17_read_rules_overnight_stay_counts_on_its_start_day() {
    let r = stay_rec(
        "2026-10-04T23:10:00+09:00",
        "2026-10-05T07:30:00+09:00",
        "2026-10-05T08:00:00+09:00",
        true,
    );
    let days = day_feelings(
        &[r],
        Some(d("2026-10-04")),
        d("2026-10-07"),
        d("2026-10-04"),
        d("2026-10-05"),
    );
    assert_eq!(days[0].status, DayStatus::Recorded);
    assert_eq!(days[0].stays.len(), 1);
    assert_eq!(days[1].status, DayStatus::Missing);
    assert!(days[1].stays.is_empty());
}

// Scenario: 主観の無い日の今日は Asia/Tokyo の日付で決まる
#[test]
fn st17_read_rules_today_is_the_tokyo_date() {
    // UTC で 2026-10-07T16:00 は Asia/Tokyo で 2026-10-08 01:00
    let today = feelings::jst_date(t("2026-10-07T16:00:00Z"));
    assert_eq!(today, d("2026-10-08"));
    let day = one_day(&[], Some("2026-10-01"), &today.to_string(), "2026-10-08");
    assert_eq!(day.status, DayStatus::Missing);
}

#[test]
fn st17_read_rules_constants_are_fixed_by_name() {
    assert_eq!(RANGE_MAX_DAYS, 62);
    assert_eq!(ABSORB_HOPS_MAX, 16);
}

#[test]
fn st17_read_rules_hidden_stay_counts_but_is_not_shown() {
    let r = stay_rec(
        "2026-10-05T09:00:00+09:00",
        "2026-10-05T10:00:00+09:00",
        "2026-10-05T11:00:00+09:00",
        false,
    );
    let day = one_day(&[r], Some("2026-10-05"), "2026-10-07", "2026-10-05");
    assert_eq!(day.status, DayStatus::Recorded);
    assert!(day.stays.is_empty());
    assert_eq!((day.stays_with_records, day.hidden_stay_records), (1, 1));
}

#[test]
fn st17_read_rules_resolve_stops_after_the_hop_limit() {
    let ids: Vec<uuid::Uuid> = (0..=ABSORB_HOPS_MAX + 2)
        .map(|_| uuid::Uuid::new_v4())
        .collect();
    let mut marks = HashMap::new();
    let mut into = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        marks.insert(*id, Mark::Absorbed);
        into.insert(*id, ids.get(i + 1).copied());
    }
    // 16 回たどった先が生きていれば着く / 17 回目なら着かない
    marks.insert(ids[ABSORB_HOPS_MAX], Mark::Live);
    assert_eq!(
        feelings::resolve_stay(ids[0], &marks, &into),
        Some(ids[ABSORB_HOPS_MAX])
    );
    marks.insert(ids[ABSORB_HOPS_MAX], Mark::Absorbed);
    marks.insert(ids[ABSORB_HOPS_MAX + 1], Mark::Live);
    assert_eq!(feelings::resolve_stay(ids[0], &marks, &into), None);
}

// ================================================================ GET /feelings

async fn read(
    app: &App,
    user: uuid::Uuid,
    from: &str,
    to: &str,
) -> Result<serde_json::Value, (StatusCode, String)> {
    let q = FeelingsQuery {
        from: from.into(),
        to: to.into(),
        user_id: Some(user),
    };
    let Json(r) = feelings_get(State(app.clone()), auth(), Query(q)).await?;
    Ok(serde_json::to_value(r).unwrap())
}

/// `date` の 1 日だけ読む
async fn read_day(app: &App, user: uuid::Uuid, date: &str) -> serde_json::Value {
    read(app, user, date, date).await.unwrap()["days"][0].clone()
}

async fn write_stay_feeling(
    app: &App,
    user: uuid::Uuid,
    stay: uuid::Uuid,
    (start, end): (&str, &str),
    written_at: &str,
    v: i64,
) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    let raw = serde_json::json!({
        "feeling": id, "nonce": nonce_for(id),
        "target": {"kind": "stay", "stay": stay, "start": start, "end": end},
        "written_at": written_at, "written_tz": "Asia/Tokyo",
        "scales": valence(v), "note": null,
    })
    .to_string();
    let r = send(app, item(id, user, start, &raw, serde_json::json!({}))).await;
    assert!(r.accepted, "{:?}", r.error);
    id
}

const A: (&str, &str) = ("2026-10-07T09:12:00+09:00", "2026-10-07T11:40:00+09:00");
const B: (&str, &str) = ("2026-10-07T14:00:00+09:00", "2026-10-07T15:00:00+09:00");

async fn put_stay_row(app: &App, user: uuid::Uuid, span: (&str, &str)) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    put_stay(app, user, id, span.0, span.1).await;
    id
}

async fn mark_stay(app: &App, stay: uuid::Uuid, by: Option<&str>) {
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = $2 WHERE id = $1")
        .bind(stay)
        .bind(by)
        .execute(&app.pool)
        .await
        .unwrap();
}

async fn absorb(app: &App, user: uuid::Uuid, from: uuid::Uuid, into: uuid::Uuid) {
    let cid: i64 = sqlx::query_scalar(
        "INSERT INTO core.stay_criteria (user_id, radius_m, min_minutes, gap_minutes, sources)
         VALUES ($1, 100, 10, 10, '{c01-location}') RETURNING id",
    )
    .bind(user)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.stay_absorbed (event_id, user_id, into_event_id, criteria_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(from)
    .bind(user)
    .bind(into)
    .bind(cid)
    .execute(&app.pool)
    .await
    .unwrap();
    mark_stay(app, from, Some("rebuild:absorbed")).await;
}

/// 利用者の最初の記録を置く（使い始めの日を決める）
async fn first_record(app: &App, user: uuid::Uuid) {
    let (_, r) = write_day(
        app,
        user,
        "2026-10-01",
        "2026-10-01T21:00:00+09:00",
        valence(0),
        None,
    )
    .await;
    assert!(r.accepted);
}

fn id_of(f: &serde_json::Value) -> String {
    f["id"].as_str().unwrap().to_string()
}

// Scenario: 同じ日に 2 回書くと最新は後に書いたほう
#[tokio::test]
async fn st17_read_api_latest_is_the_later_write() {
    let app = app().await;
    let user = testdb::user();
    let (_, r1) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:30:00+09:00",
        valence(1),
        None,
    )
    .await;
    let (second, r2) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:31:00+09:00",
        valence(-1),
        None,
    )
    .await;
    assert!(r1.accepted && r2.accepted);
    let day = read_day(&app, user, "2026-10-07").await;
    let latest = &day["day"]["latest"]["before"];
    assert_eq!(id_of(latest), second.to_string());
    assert_eq!(latest["scales"][0]["value"], -1);
    let times: Vec<&str> = day["day"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["written_at"].as_str().unwrap())
        .collect();
    assert_eq!(
        times,
        ["2026-10-07T21:31:00+09:00", "2026-10-07T21:30:00+09:00"]
    );
}

// Scenario: 前と後の最新は別々に返る
#[tokio::test]
async fn st17_read_api_before_and_after_are_separate() {
    let app = app().await;
    let user = testdb::user();
    let (b, _) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:30:00+09:00",
        valence(1),
        None,
    )
    .await;
    let (a, _) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-08T08:10:00+09:00",
        valence(2),
        None,
    )
    .await;
    let day = read_day(&app, user, "2026-10-07").await;
    assert_eq!(id_of(&day["day"]["latest"]["before"]), b.to_string());
    assert_eq!(id_of(&day["day"]["latest"]["after"]), a.to_string());
    assert_eq!(day["day"]["latest"]["before"]["phase"], "before");
    assert_eq!(day["day"]["latest"]["after"]["phase"], "after");
}

// Scenario: 滞在ごとの記録は滞在ごとに返る
#[tokio::test]
async fn st17_read_api_stays_are_separate() {
    let app = app().await;
    let user = testdb::user();
    let (a, b) = (
        put_stay_row(&app, user, A).await,
        put_stay_row(&app, user, B).await,
    );
    let fa = write_stay_feeling(&app, user, a, A, "2026-10-07T12:00:00+09:00", 1).await;
    let fb = write_stay_feeling(&app, user, b, B, "2026-10-07T16:00:00+09:00", 2).await;
    let day = read_day(&app, user, "2026-10-07").await;
    let stays = day["stays"].as_array().unwrap();
    assert_eq!(stays.len(), 2);
    for (stay, f) in [(a, fa), (b, fb)] {
        let s = stays
            .iter()
            .find(|s| s["stay"] == stay.to_string())
            .unwrap();
        assert_eq!(id_of(&s["latest"]["after"]), f.to_string());
        assert!(s["latest"]["before"].is_null());
    }
}

// Scenario: 吸収された滞在の気分は吸収先に出る
#[tokio::test]
async fn st17_read_api_absorbed_stay_feeling_shows_on_the_target() {
    let app = app().await;
    let user = testdb::user();
    let (a, b) = (
        put_stay_row(&app, user, A).await,
        put_stay_row(&app, user, B).await,
    );
    let f = write_stay_feeling(&app, user, a, A, "2026-10-07T12:00:00+09:00", 1).await;
    absorb(&app, user, a, b).await;
    let day = read_day(&app, user, "2026-10-07").await;
    let stays = day["stays"].as_array().unwrap();
    assert_eq!(stays.len(), 1);
    assert_eq!(stays[0]["stay"], b.to_string());
    assert_eq!(id_of(&stays[0]["records"][0]), f.to_string());
    assert_eq!(stays[0]["records"][0]["target_stay"], a.to_string());
    assert_eq!(day["hidden_stay_records"], 0);
}

// Scenario: 消した滞在の気分は滞在ごとの欄に出ない
#[tokio::test]
async fn st17_read_api_deleted_stay_feeling_is_not_listed() {
    let app = app().await;
    let user = testdb::user();
    let a = put_stay_row(&app, user, A).await;
    let f = write_stay_feeling(&app, user, a, A, "2026-10-07T12:00:00+09:00", 1).await;
    mark_stay(&app, a, Some("user")).await;
    let day = read_day(&app, user, "2026-10-07").await;
    assert!(day["stays"].as_array().unwrap().is_empty());
    let deleted: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT deleted_at FROM core.event WHERE id = $1")
            .bind(f)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(deleted.is_none(), "記録そのものは削除の印を持たない");
}

// Scenario: 消した滞在の気分は数だけが返る
#[tokio::test]
async fn st17_read_api_deleted_stay_feeling_is_only_counted() {
    let app = app().await;
    let user = testdb::user();
    first_record(&app, user).await;
    let a = put_stay_row(&app, user, A).await;
    write_stay_feeling(&app, user, a, A, "2026-10-07T12:00:00+09:00", 1).await;
    write_stay_feeling(&app, user, a, A, "2026-10-07T12:05:00+09:00", 2).await;
    mark_stay(&app, a, None).await; // deleted_by が NULL も本人の削除
    let day = read_day(&app, user, "2026-10-07").await;
    assert_eq!(day["hidden_stay_records"], 2);
    assert_eq!(day["status"], "recorded");
}

// Scenario: 記録のある滞在の数は書き直しを 1 と数える
#[tokio::test]
async fn st17_read_api_stays_with_records_counts_rewrites_once() {
    let app = app().await;
    let user = testdb::user();
    let (a, b) = (
        put_stay_row(&app, user, A).await,
        put_stay_row(&app, user, B).await,
    );
    write_stay_feeling(&app, user, a, A, "2026-10-07T12:00:00+09:00", 1).await;
    write_stay_feeling(&app, user, a, A, "2026-10-07T12:01:00+09:00", 2).await;
    write_stay_feeling(&app, user, b, B, "2026-10-07T16:00:00+09:00", 0).await;
    let day = read_day(&app, user, "2026-10-07").await;
    assert_eq!(day["stays_with_records"], 2);
}

// Scenario: 範囲を消して隠れた滞在の気分は滞在ごとの欄に出ない
#[tokio::test]
async fn st17_read_api_erased_range_stay_feeling_is_not_listed() {
    let app = app().await;
    let user = testdb::user();
    let a = put_stay_row(&app, user, A).await;
    write_stay_feeling(&app, user, a, A, "2026-10-07T12:00:00+09:00", 1).await;
    mark_stay(&app, a, Some("rebuild:erased-range")).await;
    let day = read_day(&app, user, "2026-10-07").await;
    assert!(day["stays"].as_array().unwrap().is_empty());
    assert_eq!(day["hidden_stay_records"], 1);
}

async fn erase_body(app: &App, user: uuid::Uuid, id: uuid::Uuid) {
    let mut tx = app.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, 's01-feeling', 'event', 'check')",
    )
    .bind(id)
    .bind(user)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn delete_mark(app: &App, id: uuid::Uuid) {
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'check' WHERE id = $1")
        .bind(id)
        .execute(&app.pool)
        .await
        .unwrap();
}

// Scenario: 消したことにした主観は読み出しに出ない
#[tokio::test]
async fn st17_read_api_deleted_mark_hides_the_record() {
    let app = app().await;
    let user = testdb::user();
    let (first, _) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:30:00+09:00",
        valence(1),
        None,
    )
    .await;
    let (last, _) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:31:00+09:00",
        valence(-1),
        None,
    )
    .await;
    delete_mark(&app, last).await;
    let day = read_day(&app, user, "2026-10-07").await;
    assert_eq!(id_of(&day["day"]["latest"]["before"]), first.to_string());
    assert_eq!(day["day"]["records"].as_array().unwrap().len(), 1);
}

// Scenario: 本文を消去した主観は読み出しに出ない
#[tokio::test]
async fn st17_read_api_erased_body_hides_the_record() {
    let app = app().await;
    let user = testdb::user();
    let (first, _) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:30:00+09:00",
        valence(1),
        None,
    )
    .await;
    let (last, _) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:31:00+09:00",
        valence(-1),
        None,
    )
    .await;
    erase_body(&app, user, last).await;
    let day = read_day(&app, user, "2026-10-07").await;
    assert_eq!(id_of(&day["day"]["latest"]["before"]), first.to_string());
    assert_eq!(day["day"]["records"].as_array().unwrap().len(), 1);
}

// Scenario: 消したことにした記録しか無い日は未記入
#[tokio::test]
async fn st17_read_api_only_a_deleted_record_is_missing() {
    let app = app().await;
    let user = testdb::user();
    first_record(&app, user).await;
    let (only, _) = write_day(
        &app,
        user,
        "2026-10-05",
        "2026-10-05T21:00:00+09:00",
        valence(1),
        None,
    )
    .await;
    delete_mark(&app, only).await;
    assert_eq!(
        read_day(&app, user, "2026-10-05").await["status"],
        "missing"
    );
}

// Scenario: 本文を消去した記録しか無い日は未記入
#[tokio::test]
async fn st17_read_api_only_an_erased_record_is_missing() {
    let app = app().await;
    let user = testdb::user();
    first_record(&app, user).await;
    let (only, _) = write_day(
        &app,
        user,
        "2026-10-05",
        "2026-10-05T21:00:00+09:00",
        valence(1),
        None,
    )
    .await;
    erase_body(&app, user, only).await;
    assert_eq!(
        read_day(&app, user, "2026-10-05").await["status"],
        "missing"
    );
}

// Scenario: 読み出しは記入の日時と D-01 に入った時刻を別々に返す
#[tokio::test]
async fn st17_read_api_written_and_ingested_are_separate() {
    let app = app().await;
    let user = testdb::user();
    write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-08T08:10:00+09:00",
        valence(1),
        None,
    )
    .await;
    let day = read_day(&app, user, "2026-10-07").await;
    let f = &day["day"]["records"][0];
    assert_eq!(f["written_at"], "2026-10-08T08:10:00+09:00");
    let ingested: DateTime<Utc> = f["ingested_at"].as_str().unwrap().parse().unwrap();
    // 書いた日時（原文）は 2026-10-08。入った時刻は取り込んだいま
    assert!(
        (Utc::now() - ingested).num_seconds().abs() < 120,
        "入った時刻は書いた日時とは別の欄"
    );
    assert_eq!(f["phase"], "after");
}

// Scenario: 感度で主観を絞らない
#[tokio::test]
async fn st17_read_api_sensitivity_does_not_filter() {
    let app = app().await;
    let user = testdb::user();
    let (id, _) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:30:00+09:00",
        valence(1),
        None,
    )
    .await;
    sqlx::query("UPDATE core.event SET sensitivity = 0 WHERE id = $1")
        .bind(id)
        .execute(&app.pool)
        .await
        .unwrap();
    let day = read_day(&app, user, "2026-10-07").await;
    assert_eq!(id_of(&day["day"]["latest"]["before"]), id.to_string());
}

// Scenario: 別の利用者の主観は読み出せない
#[tokio::test]
async fn st17_read_api_other_users_are_not_read() {
    let app = app().await;
    let (ua, ub) = (testdb::user(), testdb::user());
    let (a, _) = write_day(
        &app,
        ua,
        "2026-10-07",
        "2026-10-07T21:30:00+09:00",
        valence(1),
        None,
    )
    .await;
    let (b, _) = write_day(
        &app,
        ub,
        "2026-10-07",
        "2026-10-07T21:30:00+09:00",
        valence(2),
        None,
    )
    .await;
    let day = read_day(&app, ua, "2026-10-07").await;
    let ids: Vec<String> = day["day"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(id_of)
        .collect();
    assert_eq!(ids, [a.to_string()]);
    assert!(!day.to_string().contains(&b.to_string()));
}

// Scenario: 長すぎる範囲は断られる
#[tokio::test]
async fn st17_read_api_too_long_a_range_is_rejected() {
    let app = app().await;
    let user = testdb::user();
    // 62 日は通り、63 日は 400
    let ok = read(&app, user, "2026-08-01", "2026-10-01").await.unwrap();
    assert_eq!(ok["days"].as_array().unwrap().len(), 62);
    let e = read(&app, user, "2026-08-01", "2026-10-02")
        .await
        .unwrap_err();
    assert_eq!(e.0, StatusCode::BAD_REQUEST);
    // 逆順の範囲も断る
    let e = read(&app, user, "2026-10-02", "2026-10-01")
        .await
        .unwrap_err();
    assert_eq!(e.0, StatusCode::BAD_REQUEST);
}

// Scenario: 解釈できない日付の主観の求めは断られる
#[tokio::test]
async fn st17_read_api_unparsable_date_is_rejected() {
    let app = app().await;
    let user = testdb::user();
    let e = read(&app, user, "2026-13-40", "2026-10-01")
        .await
        .unwrap_err();
    assert_eq!(e.0, StatusCode::BAD_REQUEST);
    let e = read(&app, user, "2026-10-01", "x").await.unwrap_err();
    assert_eq!(e.0, StatusCode::BAD_REQUEST);
}

// Scenario: 資格情報の無い主観の求めは断られる
#[tokio::test]
async fn st17_read_api_without_credentials_is_rejected() {
    let app = app().await;
    let q = FeelingsQuery {
        from: "2026-10-01".into(),
        to: "2026-10-01".into(),
        user_id: None,
    };
    let Err(e) = feelings_get(State(app), HeaderMap::new(), Query(q)).await else {
        panic!("資格情報の無い求めが通った");
    };
    assert_eq!(e.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn st17_read_api_today_and_first_day_are_returned() {
    let app = app().await;
    let user = testdb::user();
    first_record(&app, user).await;
    let r = read(&app, user, "2026-10-01", "2026-10-01").await.unwrap();
    assert_eq!(r["first_day"], "2026-10-01");
    assert_eq!(
        r["today"],
        crate::stay_store::jst_date(Utc::now()).to_string()
    );
    // 未来の日に書いた記録は読めて、状態は「これから」
    let (id, _) = write_day(
        &app,
        user,
        "2030-01-10",
        "2026-10-14T12:00:00+09:00",
        valence(1),
        None,
    )
    .await;
    let day = read_day(&app, user, "2030-01-10").await;
    assert_eq!(day["status"], "future");
    assert_eq!(id_of(&day["day"]["latest"]["before"]), id.to_string());
}
