// SPDX-License-Identifier: AGPL-3.0-only
//! 1 日の並びの要約・使った時間・ほかの記録の行・`GET /day`（ST25 / design D1 / D6 / D7）を、
//! **本物の PostgreSQL に対して**確かめる。利用者は試験ごとに新しくする。
//!
//! 時刻は `+09:00`（Asia/Tokyo）で書く。滞在の行は `day_view` が読む形で直に置く（作り直しを通さない）。
#![allow(clippy::unwrap_used)]

use crate::day::{self, DayResponse, DayRow, RowKind, SUMMARY_TOP};
use crate::testdb;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const DATE: &str = "2026-09-07";

/// 読む日より後の「いま」。離席中・今日の扱いは掛からない。
fn later() -> DateTime<Utc> {
    t("2026-09-20T00:00:00+09:00")
}

fn t(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

async fn ensure_source(pool: &PgPool, name: &str) {
    sqlx::query(
        "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ($1, $2, 86400, 'none') ON CONFLICT (logical_source) DO NOTHING",
    )
    .bind(name)
    .bind(format!("表示名 {name}"))
    .execute(pool)
    .await
    .unwrap();
}

/// 記録 1 件を直に置く。内容ごとに別のハッシュ。
async fn put(pool: &PgPool, user: Uuid, source: &str, device: &str, at: &str, payload: Value) {
    ensure_source(pool, source).await;
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,$4,'collected',$5::timestamptz,540,'Asia/Tokyo',1,$6,'{}',$7)",
    )
    .bind(Uuid::new_v4())
    .bind(user)
    .bind(source)
    .bind(device)
    .bind(at)
    .bind(Uuid::new_v4().to_string())
    .bind(&payload)
    .execute(pool)
    .await
    .unwrap();
}

async fn pc_foreground(pool: &PgPool, user: Uuid, at: &str, app: &str) {
    let p = json!({"kind": "foreground", "at": at, "app_name": app, "process_name": "x.exe", "title": "題"});
    put(pool, user, "c02-window", "pc-1", at, p).await;
}

async fn pc_away(pool: &PgPool, user: Uuid, from: &str, to: &str) {
    let p = json!({"kind": "idle", "transition": "leave", "at": from, "range_end": to});
    put(pool, user, "c02-window", "pc-1", from, p).await;
}

/// PC の前景の変化を止める印（除外の始まり）。使った時間の区切りに使う。
async fn pc_excluded(pool: &PgPool, user: Uuid, from: &str, to: &str) {
    let p = json!({"kind": "excluded", "at": from, "range_end": to, "excluded_count": 1});
    put(pool, user, "c02-window", "pc-1", from, p).await;
}

async fn visit(pool: &PgPool, user: Uuid, at: &str, url: &str) {
    let p = json!({"kind": "visit", "at": at, "browser": "Chrome", "title": "題", "url": url});
    put(pool, user, "c02-browser-history", "pc-1", at, p).await;
}

async fn phone_foreground(pool: &PgPool, user: Uuid, at: &str, package: &str, label: &str) {
    let p = json!({"event_type": 1, "package": package, "app_label": label});
    put(pool, user, "c01-app-usage", "phone-1", at, p).await;
}

/// 滞在の行を直に置く（`day_view` が読む形）。
async fn stay(pool: &PgPool, user: Uuid, start: &str, end: &str) -> Uuid {
    let id = Uuid::new_v4();
    let raw = json!({"start": t(start).to_rfc3339(), "end": t(end).to_rfc3339(), "lat": 35.0, "lon": 139.0});
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
    .bind(raw.to_string())
    .execute(pool)
    .await
    .unwrap();
    id
}

/// 位置の記録を `from` から `to` まで `step_min` 分ごとに置く（移動・記録なしの判定の材料）。
async fn locations(pool: &PgPool, user: Uuid, from: &str, to: &str, step_min: i64) {
    let mut at = t(from);
    while at <= t(to) {
        let p = json!({"lat": 35.0, "lon": 139.0 + at.timestamp() as f64 * 1e-7});
        put(pool, user, "c01-location", "test", &at.to_rfc3339(), p).await;
        at += chrono::Duration::minutes(step_min);
    }
}

async fn user_deletes(pool: &PgPool, id: Uuid) {
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'user' WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

async fn view(pool: &PgPool, user: Uuid, date: &str, now: DateTime<Utc>) -> DayResponse {
    day::day(pool, user, testdb::date(date), now).await.unwrap()
}

fn row_of(v: &DayResponse, kind: RowKind) -> &DayRow {
    v.entries
        .iter()
        .find(|r| r.kind == kind)
        .unwrap_or_else(|| panic!("{kind:?} の行が無い: {:?}", v.entries))
}

fn pc_secs(r: &DayRow, app: &str) -> i64 {
    r.summary
        .as_ref()
        .unwrap()
        .pc
        .as_ref()
        .unwrap()
        .top
        .iter()
        .find(|a| a.app == app)
        .unwrap_or_else(|| panic!("{app} が要約に無い"))
        .seconds
}

// ------------------------------------------------------------------ 3.1 使った時間（D6）

// Scenario: 離席の時間はアプリを使った時間に数えない
#[tokio::test]
async fn st25_usage_away_is_not_counted() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T10:00:00+09:00", "VS Code").await;
    pc_away(
        &pool,
        u,
        "2026-09-07T10:15:00+09:00",
        "2026-09-07T10:40:00+09:00",
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T10:50:00+09:00", "Slack").await;

    let v = view(&pool, u, DATE, later()).await;
    assert_eq!(pc_secs(row_of(&v, RowKind::Stay), "VS Code"), 25 * 60);
}

// Scenario: 10 分に満たない離席も使った時間から差し引く
#[tokio::test]
async fn st25_usage_short_away_is_subtracted_too() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T10:00:00+09:00", "VS Code").await;
    pc_away(
        &pool,
        u,
        "2026-09-07T10:20:00+09:00",
        "2026-09-07T10:26:00+09:00",
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T10:30:00+09:00", "Slack").await;

    let v = view(&pool, u, DATE, later()).await;
    assert_eq!(pc_secs(row_of(&v, RowKind::Stay), "VS Code"), 24 * 60);
}

// Scenario: スマホの使った時間は 1 回 10 分で打ち切る
#[tokio::test]
async fn st25_usage_phone_use_is_capped_at_ten_minutes() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T21:00:00+09:00",
        "2026-09-07T23:50:00+09:00",
    )
    .await;
    phone_foreground(&pool, u, "2026-09-07T22:00:00+09:00", "com.map", "地図").await;
    phone_foreground(&pool, u, "2026-09-07T23:30:00+09:00", "com.mail", "メール").await;

    let v = view(&pool, u, DATE, later()).await;
    let phone = row_of(&v, RowKind::Stay)
        .summary
        .clone()
        .unwrap()
        .phone
        .unwrap();
    let map = phone.top.iter().find(|a| a.app == "地図").unwrap();
    assert_eq!(map.seconds, 10 * 60);
}

#[tokio::test]
async fn st25_usage_phone_background_of_same_app_closes_the_use() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T21:00:00+09:00",
        "2026-09-07T23:50:00+09:00",
    )
    .await;
    // 表示名つきの前景を、パッケージ名だけの「後ろに下がった」で閉じる
    phone_foreground(&pool, u, "2026-09-07T22:00:00+09:00", "com.map", "地図").await;
    let bg = json!({"event_type": 2, "package": "com.map"});
    put(
        &pool,
        u,
        "c01-app-usage",
        "phone-1",
        "2026-09-07T22:04:00+09:00",
        bg,
    )
    .await;

    let v = view(&pool, u, DATE, later()).await;
    let phone = row_of(&v, RowKind::Stay)
        .summary
        .clone()
        .unwrap()
        .phone
        .unwrap();
    assert_eq!(phone.top[0].app, "地図");
    assert_eq!(phone.top[0].seconds, 4 * 60);
}

#[tokio::test]
async fn st25_usage_outside_the_row_is_not_counted() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T10:00:00+09:00",
        "2026-09-07T10:30:00+09:00",
    )
    .await;
    // 9:50 に前に出て 10:50 まで続く。行 10:00 – 10:30 のぶんだけが数えられる
    pc_foreground(&pool, u, "2026-09-07T09:50:00+09:00", "VS Code").await;
    pc_excluded(
        &pool,
        u,
        "2026-09-07T10:50:00+09:00",
        "2026-09-07T10:51:00+09:00",
    )
    .await;

    let v = view(&pool, u, DATE, later()).await;
    assert_eq!(pc_secs(row_of(&v, RowKind::Stay), "VS Code"), 30 * 60);
}

// ------------------------------------------------------------------ 3.2 要約（D7）

#[test]
fn st25_summary_top_is_pinned_to_three() {
    assert_eq!(SUMMARY_TOP, 3);
}

// Scenario: 滞在の行に PC のアプリの上位 3 つと時間が出る
#[tokio::test]
async fn st25_summary_pc_top_three_with_time() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T09:00:00+09:00", "VS Code").await; // 90 分
    pc_foreground(&pool, u, "2026-09-07T10:30:00+09:00", "Chrome").await; // 50 分
    pc_foreground(&pool, u, "2026-09-07T11:20:00+09:00", "Slack").await; // 20 分
    pc_foreground(&pool, u, "2026-09-07T11:40:00+09:00", "Terminal").await; // 5 分
    pc_excluded(
        &pool,
        u,
        "2026-09-07T11:45:00+09:00",
        "2026-09-07T11:50:00+09:00",
    )
    .await;

    let v = view(&pool, u, DATE, later()).await;
    let pc = row_of(&v, RowKind::Stay)
        .summary
        .clone()
        .unwrap()
        .pc
        .unwrap();
    assert_eq!(pc.seconds, 165 * 60, "2 時間 45 分（Terminal を含む合計）");
    let top: Vec<(&str, i64)> = pc.top.iter().map(|a| (a.app.as_str(), a.seconds)).collect();
    assert_eq!(
        top,
        [("VS Code", 5400), ("Chrome", 3000), ("Slack", 1200)],
        "Terminal は出ない"
    );
}

// Scenario: 同じ時間のアプリは名前の順に並ぶ
#[tokio::test]
async fn st25_summary_ties_are_in_name_order() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T09:00:00+09:00", "Slack").await;
    pc_foreground(&pool, u, "2026-09-07T09:20:00+09:00", "Mail").await;
    pc_excluded(
        &pool,
        u,
        "2026-09-07T09:40:00+09:00",
        "2026-09-07T09:41:00+09:00",
    )
    .await;

    let v = view(&pool, u, DATE, later()).await;
    let pc = row_of(&v, RowKind::Stay)
        .summary
        .clone()
        .unwrap()
        .pc
        .unwrap();
    let names: Vec<&str> = pc.top.iter().map(|a| a.app.as_str()).collect();
    assert_eq!(names, ["Mail", "Slack"]);
}

// Scenario: ドメインは www を外して小文字で束ねる
#[tokio::test]
async fn st25_summary_domains_are_bundled_without_www() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    visit(
        &pool,
        u,
        "2026-09-07T10:00:00+09:00",
        "https://WWW.Example.com/a",
    )
    .await;
    visit(
        &pool,
        u,
        "2026-09-07T10:01:00+09:00",
        "https://example.com/b",
    )
    .await;

    let v = view(&pool, u, DATE, later()).await;
    let b = row_of(&v, RowKind::Stay)
        .summary
        .clone()
        .unwrap()
        .browser
        .unwrap();
    assert_eq!(b.count, 2);
    assert_eq!(b.top.len(), 1);
    assert_eq!(
        (b.top[0].domain.as_str(), b.top[0].count),
        ("example.com", 2)
    );
}

// Scenario: ブラウザは見たサイトのドメインの上位 3 つと件数が出る
#[tokio::test]
async fn st25_summary_browser_top_three_domains() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    for (minute, (host, n)) in [
        ("github.com", 18),
        ("docs.rs", 7),
        ("example.com", 5),
        ("news.example.org", 2),
    ]
    .into_iter()
    .enumerate()
    {
        for i in 0..n {
            let at = format!("2026-09-07T10:{:02}:{:02}+09:00", minute % 60, i);
            visit(&pool, u, &at, &format!("https://{host}/p{i}")).await;
        }
    }

    let v = view(&pool, u, DATE, later()).await;
    let b = row_of(&v, RowKind::Stay)
        .summary
        .clone()
        .unwrap()
        .browser
        .unwrap();
    assert_eq!(b.count, 32);
    let top: Vec<(&str, i64)> = b.top.iter().map(|d| (d.domain.as_str(), d.count)).collect();
    assert_eq!(
        top,
        [("github.com", 18), ("docs.rs", 7), ("example.com", 5)]
    );
}

// Scenario: スマホのアプリの上位 3 つと時間が出る
#[tokio::test]
async fn st25_summary_phone_top_three_in_a_move() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T10:00:00+09:00",
    )
    .await;
    stay(
        &pool,
        u,
        "2026-09-07T11:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    locations(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
        5,
    )
    .await;
    // 移動 10:00 – 11:00 に 4 つのアプリ。3 / 5 / 7 / 9 分
    for (at, pkg, label) in [
        ("10:10", "com.a", "アルファ"),
        ("10:13", "com.b", "ベータ"),
        ("10:18", "com.c", "ガンマ"),
        ("10:25", "com.d", "デルタ"),
    ] {
        phone_foreground(&pool, u, &format!("2026-09-07T{at}:00+09:00"), pkg, label).await;
    }
    let bg = json!({"event_type": 2, "package": "com.d"});
    put(
        &pool,
        u,
        "c01-app-usage",
        "phone-1",
        "2026-09-07T10:34:00+09:00",
        bg,
    )
    .await;

    let v = view(&pool, u, DATE, later()).await;
    let mv = row_of(&v, RowKind::Move);
    let phone = mv.summary.clone().unwrap().phone.unwrap();
    assert_eq!(phone.seconds, (3 + 5 + 7 + 9) * 60);
    let top: Vec<(&str, i64)> = phone
        .top
        .iter()
        .map(|a| (a.app.as_str(), a.seconds))
        .collect();
    assert_eq!(top, [("デルタ", 540), ("ガンマ", 420), ("ベータ", 300)]);
}

// Scenario: 書庫の記録は表示名ごとの件数で出る
#[tokio::test]
async fn st25_summary_archive_counts_by_display_name() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    for i in 0..12 {
        let at = format!("2026-09-07T10:{i:02}:00+09:00");
        let p = json!({"title": "題", "url": format!("https://example.com/{i}")});
        put(&pool, u, "c03-chrome-history", "import", &at, p).await;
    }

    let v = view(&pool, u, DATE, later()).await;
    let s = row_of(&v, RowKind::Stay).summary.clone().unwrap();
    let a = s.archive.unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].display_name, "表示名 c03-chrome-history");
    assert_eq!(a[0].count, 12);
}

// Scenario: 記録の無いソースの要約は出ない
#[tokio::test]
async fn st25_summary_sources_without_records_are_omitted() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T10:00:00+09:00", "Code").await;
    pc_excluded(
        &pool,
        u,
        "2026-09-07T10:10:00+09:00",
        "2026-09-07T10:11:00+09:00",
    )
    .await;

    let v = view(&pool, u, DATE, later()).await;
    let json = serde_json::to_value(row_of(&v, RowKind::Stay)).unwrap();
    let keys: Vec<&String> = json["summary"].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["pc"], "{json}");
}

// Scenario: 記録なしの行にもその時間の要約が出る
#[tokio::test]
async fn st25_summary_no_record_row_has_summary() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    pc_foreground(&pool, u, "2026-09-07T20:00:00+09:00", "Code").await;
    pc_excluded(
        &pool,
        u,
        "2026-09-07T23:00:00+09:00",
        "2026-09-07T23:01:00+09:00",
    )
    .await;

    let v = view(&pool, u, DATE, later()).await;
    let r = row_of(&v, RowKind::NoRecord);
    assert_eq!(pc_secs(r, "Code"), 3 * 3600);
}

// Scenario: 消した行には要約が出ない
#[tokio::test]
async fn st25_summary_erased_row_has_none() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let id = stay(
        &pool,
        u,
        "2026-09-07T10:00:00+09:00",
        "2026-09-07T11:00:00+09:00",
    )
    .await;
    user_deletes(&pool, id).await;
    pc_foreground(&pool, u, "2026-09-07T10:10:00+09:00", "Code").await;

    let v = view(&pool, u, DATE, later()).await;
    let erased = serde_json::to_value(row_of(&v, RowKind::Erased)).unwrap();
    assert!(erased.get("summary").is_none(), "{erased}");
}

// Scenario: 日付をまたぐ行の要約は両方の日で同じ
#[tokio::test]
async fn st25_summary_stay_across_midnight_is_the_same_on_both_days() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T23:00:00+09:00",
        "2026-09-08T02:00:00+09:00",
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T23:30:00+09:00", "Code").await;
    pc_excluded(
        &pool,
        u,
        "2026-09-08T01:30:00+09:00",
        "2026-09-08T01:31:00+09:00",
    )
    .await;

    let before = view(&pool, u, "2026-09-07", later()).await;
    let after = view(&pool, u, "2026-09-08", later()).await;
    let (a, b) = (
        row_of(&before, RowKind::Stay),
        row_of(&after, RowKind::Stay),
    );
    assert_eq!(a.summary, b.summary);
    assert_eq!(pc_secs(a, "Code"), 2 * 3600);
}

// ------------------------------------------------------------------ 3.3 ほかの記録の行（D7）

// Scenario: どの行にも入らない時間の記録はほかの記録の行に出る
#[tokio::test]
async fn st25_other_erased_time_gets_an_other_row() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let id = stay(
        &pool,
        u,
        "2026-09-07T10:00:00+09:00",
        "2026-09-07T11:00:00+09:00",
    )
    .await;
    user_deletes(&pool, id).await;
    for i in 0..30 {
        let at = format!("2026-09-07T10:{:02}:00+09:00", i * 2);
        pc_foreground(&pool, u, &at, "Code").await;
    }

    let v = view(&pool, u, DATE, later()).await;
    let kinds: Vec<RowKind> = v.entries.iter().map(|r| r.kind).collect();
    let at = kinds.iter().position(|k| *k == RowKind::Erased).unwrap();
    assert_eq!(kinds[at + 1], RowKind::Other, "消した行の直後: {kinds:?}");
    let (erased, other) = (&v.entries[at], &v.entries[at + 1]);
    assert_eq!(
        (other.start, other.end),
        (
            t("2026-09-07T10:00:00+09:00"),
            t("2026-09-07T11:00:00+09:00")
        )
    );
    assert_eq!((erased.start, erased.end), (other.start, other.end));
    assert!(erased.summary.is_none(), "消した行は要約を持たない");
    assert!(other.summary.as_ref().unwrap().pc.is_some());
    // 開くとその 30 件に届く（同じ範囲を /day/records で読む）
    let records = crate::day_records::records(
        &pool,
        u,
        testdb::date(DATE),
        Some(other.start),
        Some(other.end),
        later(),
    )
    .await
    .unwrap();
    assert_eq!(records.len(), 30);
}

// Scenario: 今日の最後の位置の後の記録はほかの記録の行に出る
#[tokio::test]
async fn st25_other_after_the_last_location_today() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 記録なしとみなす間隔を 3 時間にして、13:00 – 14:30 が「記録なし」の行にならないようにする
    crate::stay_store::set_criteria(&pool, u, None, None, Some(180))
        .await
        .unwrap();
    locations(
        &pool,
        u,
        "2026-09-07T12:00:00+09:00",
        "2026-09-07T13:00:00+09:00",
        5,
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T13:10:00+09:00", "Code").await;

    let now = t("2026-09-07T14:30:00+09:00");
    let v = view(&pool, u, DATE, now).await;
    let last = v.entries.last().unwrap();
    assert_eq!(last.kind, RowKind::Other, "{:?}", v.entries);
    assert_eq!(
        (last.start, last.end),
        (t("2026-09-07T13:00:00+09:00"), now)
    );
    assert_eq!(pc_secs(last, "Code"), 80 * 60);
}

#[tokio::test]
async fn st25_other_row_needs_a_record() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    crate::stay_store::set_criteria(&pool, u, None, None, Some(180))
        .await
        .unwrap();
    locations(
        &pool,
        u,
        "2026-09-07T12:00:00+09:00",
        "2026-09-07T13:00:00+09:00",
        5,
    )
    .await;
    let id = stay(
        &pool,
        u,
        "2026-09-07T10:00:00+09:00",
        "2026-09-07T11:00:00+09:00",
    )
    .await;
    user_deletes(&pool, id).await;

    let v = view(&pool, u, DATE, t("2026-09-07T14:30:00+09:00")).await;
    assert!(
        v.entries.iter().all(|r| r.kind != RowKind::Other),
        "位置以外の記録が無ければ足さない: {:?}",
        v.entries
    );
}

// ------------------------------------------------------------------ 3.4 `GET /day`

const TOKEN: &str = "test-token-0123456789abcdef";

fn auth() -> axum::http::HeaderMap {
    let mut h = axum::http::HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

async fn call(
    headers: axum::http::HeaderMap,
    user: Uuid,
    date: &str,
) -> Result<Value, (axum::http::StatusCode, String)> {
    let app = crate::App::for_test(testdb::pool().await, TOKEN);
    let q = crate::DayQuery {
        date: date.into(),
        user_id: Some(user),
    };
    crate::day_get(axum::extract::State(app), headers, axum::extract::Query(q))
        .await
        .map(|axum::Json(v)| serde_json::to_value(v).unwrap())
}

#[tokio::test]
async fn st25_api_day_entries_are_day_view_rows_with_summary() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let kept = stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T10:00:00+09:00",
    )
    .await;
    let gone = stay(
        &pool,
        u,
        "2026-09-07T11:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    user_deletes(&pool, gone).await;
    pc_foreground(&pool, u, "2026-09-07T09:10:00+09:00", "Code").await;
    pc_foreground(&pool, u, "2026-09-07T11:10:00+09:00", "Code").await;

    let got = call(auth(), u, DATE).await.unwrap();
    assert_eq!(got["date"], DATE);
    assert!(got["criteria"].is_array());
    let entries = got["entries"].as_array().unwrap();
    // 滞在・消した・ほかの記録が時刻順に並ぶ
    let stay_row = entries.iter().find(|e| e["kind"] == "stay").unwrap();
    assert_eq!(stay_row["id"], kept.to_string());
    assert!(stay_row["summary"]["pc"]["seconds"].as_i64().unwrap() > 0);
    let erased = entries.iter().find(|e| e["kind"] == "erased").unwrap();
    assert!(erased.get("summary").is_none());
    assert_eq!(erased["stay_ids"][0], gone.to_string());
    assert!(entries.iter().any(|e| e["kind"] == "other"));
    let starts: Vec<&str> = entries
        .iter()
        .map(|e| e["start"].as_str().unwrap())
        .collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(starts, sorted, "始まりの時刻順");
}

#[tokio::test]
async fn st25_api_day_invalid_date_is_400() {
    let err = call(auth(), testdb::user(), "2026-13-40")
        .await
        .unwrap_err();
    assert_eq!(err.0, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn st25_api_day_without_credentials_is_401() {
    let err = call(axum::http::HeaderMap::new(), testdb::user(), DATE)
        .await
        .unwrap_err();
    assert_eq!(err.0, axum::http::StatusCode::UNAUTHORIZED);
}
