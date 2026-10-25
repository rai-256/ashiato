// SPDX-License-Identifier: AGPL-3.0-only
//! 1 日の並びの要約・使った時間・ほかの記録の行・`GET /day`（ST25 / design D1 / D6 / D7）を、
//! **本物の PostgreSQL に対して**確かめる。利用者は試験ごとに新しくする。
//!
//! 時刻は `+09:00`（Asia/Tokyo）で書く。滞在の行は `day_view` が読む形で直に置く（作り直しを通さない）。
#![allow(clippy::unwrap_used)]

use crate::day::{self, DayResponse, DayRow, GapKind, RowKind, AWAY_ROW_MIN_MINUTES, SUMMARY_TOP};
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

/// 登録簿の表示名。書庫のソースは ST12 の移行が本物の名前で置くので、試験は決め打ちしない。
async fn display_name(pool: &PgPool, name: &str) -> String {
    sqlx::query_scalar("SELECT display_name FROM core.source WHERE logical_source = $1")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
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

// 前の日 23:50 から続く PC の前景は、今日 0:00 から始まる行の使った時間に数える（final review R10）。
// 読む窓が今日の頭から始まっても、窓より前に始まった前景を読む
#[tokio::test]
async fn st25_usage_foreground_from_the_previous_day_counts_from_midnight() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T00:00:00+09:00",
        "2026-09-07T01:00:00+09:00",
    )
    .await;
    pc_foreground(&pool, u, "2026-09-06T23:50:00+09:00", "VS Code").await;
    pc_foreground(&pool, u, "2026-09-07T00:20:00+09:00", "Slack").await;
    pc_excluded(
        &pool,
        u,
        "2026-09-07T00:30:00+09:00",
        "2026-09-07T00:31:00+09:00",
    )
    .await;

    let v = view(&pool, u, DATE, later()).await;
    let r = row_of(&v, RowKind::Stay);
    assert_eq!(r.start, t("2026-09-07T00:00:00+09:00"));
    assert_eq!(pc_secs(r, "VS Code"), 20 * 60);
    assert_eq!(pc_secs(r, "Slack"), 10 * 60);
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
async fn st25_usage_phone_overlapping_uses_of_same_app_count_once() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T21:00:00+09:00",
        "2026-09-07T23:50:00+09:00",
    )
    .await;
    // 後ろに下がった記録が無いまま同じアプリの前景が続く: 10:00–10:10 と 10:05–10:15 は重なる
    phone_foreground(&pool, u, "2026-09-07T22:00:00+09:00", "com.map", "地図").await;
    phone_foreground(&pool, u, "2026-09-07T22:05:00+09:00", "com.map", "地図").await;
    phone_foreground(&pool, u, "2026-09-07T22:30:00+09:00", "com.mail", "メール").await;

    let v = view(&pool, u, DATE, later()).await;
    let phone = row_of(&v, RowKind::Stay)
        .summary
        .clone()
        .unwrap()
        .phone
        .unwrap();
    let map = phone.top.iter().find(|a| a.app == "地図").unwrap();
    assert_eq!(map.seconds, 15 * 60, "{phone:?}");
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

// `about:blank` や `chrome://newtab` はサイトの上位に混ざらない（件数には数える。R9）
#[tokio::test]
async fn st25_summary_non_http_pages_are_not_top_domains() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    stay(
        &pool,
        u,
        "2026-09-07T09:00:00+09:00",
        "2026-09-07T12:00:00+09:00",
    )
    .await;
    for (i, url) in [
        "about:blank",
        "about:blank#2",
        "chrome://newtab/",
        "chrome://newtab/?2",
        "https://example.com/a",
    ]
    .iter()
    .enumerate()
    {
        visit(&pool, u, &format!("2026-09-07T10:0{i}:00+09:00"), url).await;
    }

    let v = view(&pool, u, DATE, later()).await;
    let b = row_of(&v, RowKind::Stay)
        .summary
        .clone()
        .unwrap()
        .browser
        .unwrap();
    assert_eq!(b.count, 5);
    let domains: Vec<&str> = b.top.iter().map(|d| d.domain.as_str()).collect();
    assert_eq!(domains, ["example.com"]);
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
    assert_eq!(
        a[0].display_name,
        display_name(&pool, "c03-chrome-history").await
    );
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

// Scenario: 今日の最後の位置の後の記録は記録なしの行に出る
#[tokio::test]
async fn st25_other_after_the_last_location_today() {
    let pool = testdb::pool().await;
    let u = testdb::user();
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
    // 既定の間隔（10 分）では day_view が末尾を記録なしにする。ほかの記録の行は足さない（Q7）
    assert_eq!(last.kind, RowKind::NoRecord, "{:?}", v.entries);
    assert!(v.entries.iter().all(|r| r.kind != RowKind::Other));
    assert_eq!(
        (last.start, last.end),
        (t("2026-09-07T13:00:00+09:00"), now)
    );
    assert_eq!(pc_secs(last, "Code"), 80 * 60);
}

// 記録なしとみなす間隔を 3 時間にして day_view が末尾を覆わないときだけ、ほかの記録の行が出る
#[tokio::test]
async fn st25_other_after_the_last_location_with_a_long_gap() {
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
    pc_foreground(&pool, u, "2026-09-07T13:10:00+09:00", "Code").await;

    let now = t("2026-09-07T14:30:00+09:00");
    let v = view(&pool, u, DATE, now).await;
    let last = v.entries.last().unwrap();
    assert_eq!(last.kind, RowKind::Other, "{:?}", v.entries);
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

// ------------------------------------------------------------------ 取れていない時間（4.1 / D9）

async fn pc_stopped(pool: &PgPool, user: Uuid, from: &str, to: &str) {
    let p = json!({"kind": "powered-off", "at": from, "range_end": to});
    put(pool, user, "c02-window", "pc-1", from, p).await;
}

async fn pc_excluded_n(pool: &PgPool, user: Uuid, from: &str, to: &str, count: i64) {
    let p = json!({"kind": "excluded", "at": from, "range_end": to, "excluded_count": count});
    put(pool, user, "c02-window", "pc-1", from, p).await;
}

async fn phone_gap(pool: &PgPool, user: Uuid, begin: &str, end: &str, reason: &str) {
    let p = json!({"kind": "gap", "begin": begin, "end": end, "reason": reason});
    put(pool, user, "c01-app-usage", "phone-1", end, p).await;
}

fn gaps_of(v: &DayResponse, kind: GapKind) -> Vec<&day::Gap> {
    v.gaps.iter().filter(|g| g.kind == kind).collect()
}

#[test]
fn st25_gaprow_away_minimum_is_pinned_to_ten_minutes() {
    assert_eq!(AWAY_ROW_MIN_MINUTES, 10);
}

// 2 台の PC が同じ時刻に離席すると、種類と時刻だけでは 2 つの行を区別できない。端末を持たせる（R12(d)）
#[tokio::test]
async fn st25_gaprow_carries_the_device() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    for device in ["pc-1", "pc-2"] {
        let (from, to) = ("2026-09-07T10:15:00+09:00", "2026-09-07T10:45:00+09:00");
        let p = json!({"kind": "idle", "transition": "leave", "at": from, "range_end": to});
        put(&pool, u, "c02-window", device, from, p).await;
    }
    let v = view(&pool, u, DATE, later()).await;
    let mut devices: Vec<Option<&str>> = gaps_of(&v, GapKind::Away)
        .iter()
        .map(|g| g.device_id.as_deref())
        .collect();
    devices.sort();
    assert_eq!(devices, [Some("pc-1"), Some("pc-2")], "{:?}", v.gaps);
}

#[tokio::test]
async fn st25_gaprow_ten_minute_away_is_a_row() {
    // Scenario: 10 分以上の離席は行になる
    let pool = testdb::pool().await;
    let u = testdb::user();
    pc_away(
        &pool,
        u,
        "2026-09-07T10:15:00+09:00",
        "2026-09-07T10:25:00+09:00",
    )
    .await;
    let v = view(&pool, u, DATE, later()).await;
    let g = gaps_of(&v, GapKind::Away);
    assert_eq!(g.len(), 1, "{:?}", v.gaps);
    assert_eq!(g[0].start, t("2026-09-07T10:15:00+09:00"));
    assert_eq!(g[0].end, t("2026-09-07T10:25:00+09:00"));
}

#[tokio::test]
async fn st25_gaprow_away_under_ten_minutes_is_not_a_row() {
    // Scenario: 10 分に満たない離席は行にならない
    let pool = testdb::pool().await;
    let u = testdb::user();
    pc_away(
        &pool,
        u,
        "2026-09-07T11:00:00+09:00",
        "2026-09-07T11:09:59+09:00",
    )
    .await;
    let v = view(&pool, u, DATE, later()).await;
    assert!(gaps_of(&v, GapKind::Away).is_empty(), "{:?}", v.gaps);
}

#[tokio::test]
async fn st25_gaprow_away_now_is_a_row_today() {
    // Scenario: いま離席中の時間は今日の行になる
    let pool = testdb::pool().await;
    let u = testdb::user();
    let enter = json!({"kind": "idle", "transition": "enter", "at": "2026-09-07T14:05:00+09:00"});
    put(
        &pool,
        u,
        "c02-window",
        "pc-1",
        "2026-09-07T14:05:00+09:00",
        enter,
    )
    .await;
    let now = t("2026-09-07T14:30:00+09:00");
    let v = view(&pool, u, DATE, now).await;
    let g = gaps_of(&v, GapKind::Away);
    assert_eq!(g.len(), 1, "{:?}", v.gaps);
    assert_eq!(g[0].start, t("2026-09-07T14:05:00+09:00"));
    assert_eq!(g[0].end, now);
    assert!(
        v.gaps.iter().all(|g| g.end <= now) && v.entries.iter().all(|r| r.start <= now),
        "いまより後の時刻を持つ行は無い"
    );
}

#[tokio::test]
async fn st25_gaprow_away_now_from_the_previous_day_is_a_row_on_both_days() {
    // Scenario: いま離席中の時間は今日の行になる
    // 0 時をまたいで離席している間も、今日の側に 0:00 からいままでの行が出る（code-verify R21）
    let pool = testdb::pool().await;
    let u = testdb::user();
    let enter = json!({"kind": "idle", "transition": "enter", "at": "2026-09-06T23:50:00+09:00"});
    put(
        &pool,
        u,
        "c02-window",
        "pc-1",
        "2026-09-06T23:50:00+09:00",
        enter,
    )
    .await;
    let now = t("2026-09-07T00:30:00+09:00");
    let v = view(&pool, u, DATE, now).await;
    let today = gaps_of(&v, GapKind::Away);
    assert_eq!(today.len(), 1, "{today:?}");
    assert_eq!(
        (today[0].start, today[0].end),
        (t("2026-09-07T00:00:00+09:00"), now)
    );
    let v = view(&pool, u, "2026-09-06", now).await;
    let before = gaps_of(&v, GapKind::Away);
    assert_eq!(before.len(), 1, "{before:?}");
    assert_eq!(
        (before[0].start, before[0].end),
        (
            t("2026-09-06T23:50:00+09:00"),
            t("2026-09-07T00:00:00+09:00")
        )
    );
}

#[tokio::test]
async fn st25_gaprow_stopped_is_clipped_to_the_day_end() {
    // Scenario: PC の停止は行になる
    let pool = testdb::pool().await;
    let u = testdb::user();
    pc_stopped(
        &pool,
        u,
        "2026-09-07T23:40:00+09:00",
        "2026-09-08T08:10:00+09:00",
    )
    .await;
    let v = view(&pool, u, DATE, later()).await;
    let g = gaps_of(&v, GapKind::Stopped);
    assert_eq!(g.len(), 1, "{:?}", v.gaps);
    assert_eq!(g[0].start, t("2026-09-07T23:40:00+09:00"));
    assert_eq!(g[0].end, t("2026-09-08T00:00:00+09:00"));
}

#[tokio::test]
async fn st25_gaprow_excluded_is_a_count_row_until_the_next_foreground() {
    // Scenario: 除外は件数だけの行になる
    let pool = testdb::pool().await;
    let u = testdb::user();
    pc_foreground(&pool, u, "2026-09-07T16:00:00+09:00", "Code").await;
    pc_excluded_n(
        &pool,
        u,
        "2026-09-07T16:05:00+09:00",
        "2026-09-07T16:05:00+09:00",
        3,
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T16:25:00+09:00", "Code").await;
    let v = view(&pool, u, DATE, later()).await;
    let g = gaps_of(&v, GapKind::Excluded);
    assert_eq!(g.len(), 1, "{:?}", v.gaps);
    assert_eq!(g[0].start, t("2026-09-07T16:05:00+09:00"));
    assert_eq!(g[0].end, t("2026-09-07T16:25:00+09:00"));
    assert_eq!(g[0].count, Some(3));
}

#[tokio::test]
async fn st25_gaprow_excluded_range_end_is_a_lower_bound() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    pc_excluded_n(
        &pool,
        u,
        "2026-09-07T16:05:00+09:00",
        "2026-09-07T16:40:00+09:00",
        1,
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T16:25:00+09:00", "Code").await;
    let v = view(&pool, u, DATE, later()).await;
    let g = gaps_of(&v, GapKind::Excluded);
    assert_eq!(g[0].end, t("2026-09-07T16:40:00+09:00"));
}

#[tokio::test]
async fn st25_gaprow_excluded_split_by_the_send_trigger_is_one_row() {
    // Scenario: 送る契機で区切られた除外は 1 行にまとまる
    let pool = testdb::pool().await;
    let u = testdb::user();
    pc_excluded_n(
        &pool,
        u,
        "2026-09-07T16:05:00+09:00",
        "2026-09-07T16:10:00+09:00",
        2,
    )
    .await;
    pc_excluded_n(
        &pool,
        u,
        "2026-09-07T16:10:00+09:00",
        "2026-09-07T16:15:00+09:00",
        1,
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T16:25:00+09:00", "Code").await;
    let v = view(&pool, u, DATE, later()).await;
    let g = gaps_of(&v, GapKind::Excluded);
    assert_eq!(g.len(), 1, "{:?}", v.gaps);
    assert_eq!(g[0].start, t("2026-09-07T16:05:00+09:00"));
    assert_eq!(g[0].end, t("2026-09-07T16:25:00+09:00"));
    assert_eq!(g[0].count, Some(3));
}

#[tokio::test]
async fn st25_gaprow_excluded_separated_by_a_foreground_stay_apart() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    pc_excluded_n(
        &pool,
        u,
        "2026-09-07T16:05:00+09:00",
        "2026-09-07T16:10:00+09:00",
        2,
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T16:09:00+09:00", "Code").await;
    pc_excluded_n(
        &pool,
        u,
        "2026-09-07T16:10:00+09:00",
        "2026-09-07T16:15:00+09:00",
        1,
    )
    .await;
    pc_foreground(&pool, u, "2026-09-07T16:25:00+09:00", "Code").await;
    let v = view(&pool, u, DATE, later()).await;
    assert_eq!(gaps_of(&v, GapKind::Excluded).len(), 2, "{:?}", v.gaps);
}

#[tokio::test]
async fn st25_gaprow_phone_gap_keeps_its_reason() {
    // Scenario: スマホの取得元に残っていなかった期間は行になる
    let pool = testdb::pool().await;
    let u = testdb::user();
    phone_gap(
        &pool,
        u,
        "2026-09-07T06:00:00+09:00",
        "2026-09-07T09:00:00+09:00",
        "retention",
    )
    .await;
    let v = view(&pool, u, DATE, later()).await;
    let g = gaps_of(&v, GapKind::Unavailable);
    assert_eq!(g.len(), 1, "{:?}", v.gaps);
    assert_eq!(g[0].start, t("2026-09-07T06:00:00+09:00"));
    assert_eq!(g[0].end, t("2026-09-07T09:00:00+09:00"));
    assert_eq!(g[0].reason.as_deref(), Some("retention"));
}

#[tokio::test]
async fn st25_gaprow_clock_skew_gap_has_a_different_reason() {
    // Scenario: 時計のずれで取り直さなかった期間は別の文字の行になる
    let pool = testdb::pool().await;
    let u = testdb::user();
    phone_gap(
        &pool,
        u,
        "2026-09-07T06:00:00+09:00",
        "2026-09-07T09:00:00+09:00",
        "clock_skew_abandoned",
    )
    .await;
    let v = view(&pool, u, DATE, later()).await;
    let g = gaps_of(&v, GapKind::Unavailable);
    assert_eq!(g.len(), 1, "{:?}", v.gaps);
    assert_eq!(g[0].reason.as_deref(), Some("clock_skew_abandoned"));
}

// ------------------------------------------------------------------ ブラウザの最後に届いた時刻（4.2 / D10）

const BROWSER: &str = "c02-browser-history";

/// 収集開始日は論理ソースごと（利用者ごとではない）に 1 つ。並走する試験が書き合わないよう、
/// 開始日を置く試験は順に走らせる。
static BROWSER_STARTED_ON: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// ブラウザの訪問を、届いた時刻を指定して置く。
async fn visit_ingested(pool: &PgPool, user: Uuid, at: &str, ingested: &str) {
    ensure_source(pool, BROWSER).await;
    let p = json!({"kind": "visit", "at": at, "browser": "Chrome", "title": "題", "url": "https://example.com/"});
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, ingest_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'pc-1','collected',$4::timestamptz,$5::timestamptz,540,'Asia/Tokyo',1,$6,'{}',$7)",
    )
    .bind(Uuid::new_v4())
    .bind(user)
    .bind(BROWSER)
    .bind(at)
    .bind(ingested)
    .bind(Uuid::new_v4().to_string())
    .bind(&p)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn st25_browser_not_yet_arrived_is_a_row() {
    let _serial = BROWSER_STARTED_ON.lock().await;
    // Scenario: まだ届いていないブラウザ履歴は行になる
    let pool = testdb::pool().await;
    let u = testdb::user();
    visit_ingested(
        &pool,
        u,
        "2026-09-06T08:50:00+09:00",
        "2026-09-06T09:00:00+09:00",
    )
    .await;
    testdb::set_started_on(&pool, BROWSER, "2026-09-01").await;
    let now = t("2026-09-07T15:00:00+09:00");
    let v = view(&pool, u, DATE, now).await;
    assert_eq!(v.browser_through, Some(t("2026-09-06T09:00:00+09:00")));
}

#[tokio::test]
async fn st25_browser_deleted_rows_still_count_as_arrived() {
    let _serial = BROWSER_STARTED_ON.lock().await;
    let pool = testdb::pool().await;
    let u = testdb::user();
    visit_ingested(
        &pool,
        u,
        "2026-09-06T08:50:00+09:00",
        "2026-09-06T09:00:00+09:00",
    )
    .await;
    visit_ingested(
        &pool,
        u,
        "2026-09-06T10:50:00+09:00",
        "2026-09-06T11:00:00+09:00",
    )
    .await;
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'user' WHERE user_id = $1 AND ingest_time > '2026-09-06T10:00:00+09:00'")
        .bind(u)
        .execute(&pool)
        .await
        .unwrap();
    testdb::set_started_on(&pool, BROWSER, "2026-09-01").await;
    let v = view(&pool, u, DATE, t("2026-09-07T15:00:00+09:00")).await;
    assert_eq!(v.browser_through, Some(t("2026-09-06T11:00:00+09:00")));
}

#[tokio::test]
async fn st25_browser_arrived_after_the_day_end_has_no_row() {
    let _serial = BROWSER_STARTED_ON.lock().await;
    // Scenario: 届いた日にはまだ届いていない行は出ない
    let pool = testdb::pool().await;
    let u = testdb::user();
    visit_ingested(
        &pool,
        u,
        "2026-09-07T08:50:00+09:00",
        "2026-09-08T00:00:00+09:00",
    )
    .await;
    testdb::set_started_on(&pool, BROWSER, "2026-09-01").await;
    let v = view(&pool, u, DATE, later()).await;
    assert_eq!(v.browser_through, None);
}

#[tokio::test]
async fn st25_browser_before_collection_started_has_no_row() {
    let _serial = BROWSER_STARTED_ON.lock().await;
    // Scenario: ブラウザ履歴を集めていない日にはまだ届いていない行は出ない
    let pool = testdb::pool().await;
    let u = testdb::user();
    visit_ingested(
        &pool,
        u,
        "2026-09-06T08:50:00+09:00",
        "2026-09-06T09:00:00+09:00",
    )
    .await;
    testdb::set_started_on(&pool, BROWSER, "2026-09-10").await;
    let v = view(&pool, u, DATE, later()).await;
    assert_eq!(v.browser_through, None);
}

#[tokio::test]
async fn st25_browser_without_any_arrival_has_no_row() {
    let _serial = BROWSER_STARTED_ON.lock().await;
    let pool = testdb::pool().await;
    let u = testdb::user();
    let v = view(&pool, u, DATE, later()).await;
    assert_eq!(v.browser_through, None);
}

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

// ------------------------------------------------------------------ 記録なしの事情（Task 5 / D11）

const LOCATION: &str = "c01-location";

/// 登録簿の収集開始日は全試験で共有するので、事情の試験は他の試験が付ける日より後の日を読む。
const REASON_DATE: &str = "2030-01-05";

fn reasons_now() -> DateTime<Utc> {
    t("2031-01-01T00:00:00+09:00")
}

/// 08:00 – 08:20 と 16:40 – 17:00 に位置があり、間 08:20 – 16:40 が「記録なし」になる日。
async fn gap_day(pool: &PgPool, user: Uuid) {
    locations(
        pool,
        user,
        "2030-01-05T08:00:00+09:00",
        "2030-01-05T08:20:00+09:00",
        5,
    )
    .await;
    locations(
        pool,
        user,
        "2030-01-05T16:40:00+09:00",
        "2030-01-05T17:00:00+09:00",
        5,
    )
    .await;
}

fn no_record_at<'a>(v: &'a DayResponse, start: &str) -> &'a DayRow {
    v.entries
        .iter()
        .find(|e| e.kind == RowKind::NoRecord && e.start == t(start))
        .unwrap_or_else(|| panic!("{start} から始まる記録なしの行が無い: {:#?}", v.entries))
}

fn reasons_of(r: &DayRow) -> Vec<(day::ReasonKind, Option<i64>)> {
    r.reasons.iter().map(|x| (x.kind, x.count)).collect()
}

/// 書庫の位置の点を `n` 件、`from` から 1 分おきに置く。
async fn archive_points(pool: &PgPool, user: Uuid, source: &str, from: &str, n: i64) {
    for i in 0..n {
        let at = t(from) + chrono::Duration::minutes(i);
        put(
            pool,
            user,
            source,
            "archive",
            &at.to_rfc3339(),
            json!({"lat": 35.0, "lon": 139.0 + i as f64 * 1e-5}),
        )
        .await;
    }
}

#[tokio::test]
async fn st25_reasons_before_collection_started() {
    // Scenario: 収集を始める前の日の記録なしにはそう添えられる
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 収集開始日をこの試験の間だけ 2002 年に置き、終わったら元の値へ戻す。2002 年なら REASON_DATE（2030 年）を読む並走の試験には
    // 「開始前」が混ざらない。他の試験が過去の日（2000 年など）を残していても、そのままだと 2001-01-05 が開始後になり落ちる
    let before: Option<chrono::NaiveDate> = sqlx::query_scalar(
        "SELECT collection_started_on FROM core.source WHERE logical_source = $1",
    )
    .bind(LOCATION)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE core.source SET collection_started_on = '2002-01-01' WHERE logical_source = $1",
    )
    .bind(LOCATION)
    .execute(&pool)
    .await
    .unwrap();
    let v = view(&pool, u, "2001-01-05", reasons_now()).await;
    sqlx::query("UPDATE core.source SET collection_started_on = $2 WHERE logical_source = $1 AND collection_started_on = '2002-01-01'")
        .bind(LOCATION)
        .bind(before)
        .execute(&pool)
        .await
        .unwrap();
    let r = no_record_at(&v, "2001-01-05T00:00:00+09:00");
    assert_eq!(r.end, t("2001-01-06T00:00:00+09:00"));
    assert_eq!(reasons_of(r), vec![(day::ReasonKind::BeforeStart, None)]);
}

#[tokio::test]
async fn st25_reasons_dropped_with_count() {
    // Scenario: 端末が破棄した時間の記録なしには件数が添えられる
    let pool = testdb::pool().await;
    let u = testdb::user();
    testdb::put_drop(
        &pool,
        u,
        LOCATION,
        Some(("2030-01-05T13:00:00+09:00", "2030-01-05T14:00:00+09:00")),
        &[("2030-01-05T04:00:00Z", 60)],
    )
    .await;
    let v = view(&pool, u, REASON_DATE, reasons_now()).await;
    let r = no_record_at(&v, "2030-01-05T00:00:00+09:00");
    assert_eq!(reasons_of(r), vec![(day::ReasonKind::Dropped, Some(60))]);
}

#[tokio::test]
async fn st25_reasons_legacy_dropped_span_counts_its_own_events() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    sqlx::query(
        "INSERT INTO core.coverage_span (id, user_id, logical_source, kind, started_at, ended_at, event_count)
         VALUES ($1,$2,$3,'dropped','2030-01-05T13:00:00+09:00','2030-01-05T14:00:00+09:00',7)",
    )
    .bind(Uuid::new_v4())
    .bind(u)
    .bind(LOCATION)
    .execute(&pool)
    .await
    .unwrap();
    let v = view(&pool, u, REASON_DATE, reasons_now()).await;
    let r = no_record_at(&v, "2030-01-05T00:00:00+09:00");
    assert_eq!(reasons_of(r), vec![(day::ReasonKind::Dropped, Some(7))]);
}

#[tokio::test]
async fn st25_reasons_archive_counts_both_routes() {
    // Scenario: 書庫に位置がある時間の記録なしには件数が添えられる
    let pool = testdb::pool().await;
    let u = testdb::user();
    archive_points(
        &pool,
        u,
        "c03-timeline-route",
        "2030-01-05T02:10:00+09:00",
        10,
    )
    .await;
    archive_points(
        &pool,
        u,
        "c03-legacy-location",
        "2030-01-05T02:10:00+09:00",
        4,
    )
    .await;
    let v = view(&pool, u, REASON_DATE, reasons_now()).await;
    let r = no_record_at(&v, "2030-01-05T00:00:00+09:00");
    assert_eq!(reasons_of(r), vec![(day::ReasonKind::Archive, Some(14))]);
}

#[tokio::test]
async fn st25_reasons_stopped() {
    // Scenario: 本人が止めていた時間の記録なしにはそう添えられる
    let pool = testdb::pool().await;
    let u = testdb::user();
    testdb::put_span(
        &pool,
        u,
        LOCATION,
        "stopped",
        "2030-01-05T20:00:00+09:00",
        Some("2030-01-05T22:00:00+09:00"),
    )
    .await;
    let v = view(&pool, u, REASON_DATE, reasons_now()).await;
    let r = no_record_at(&v, "2030-01-05T00:00:00+09:00");
    assert_eq!(reasons_of(r), vec![(day::ReasonKind::Stopped, None)]);
}

#[tokio::test]
async fn st25_reasons_overlap_in_the_fixed_order() {
    // Scenario: 事情が重なると並べて添えられる
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 置く順は並びと逆
    archive_points(
        &pool,
        u,
        "c03-timeline-route",
        "2030-01-05T02:10:00+09:00",
        3,
    )
    .await;
    testdb::put_drop(
        &pool,
        u,
        LOCATION,
        Some(("2030-01-05T13:00:00+09:00", "2030-01-05T14:00:00+09:00")),
        &[("2030-01-05T04:00:00Z", 60)],
    )
    .await;
    let v = view(&pool, u, REASON_DATE, reasons_now()).await;
    let r = no_record_at(&v, "2030-01-05T00:00:00+09:00");
    assert_eq!(
        reasons_of(r),
        vec![
            (day::ReasonKind::Dropped, Some(60)),
            (day::ReasonKind::Archive, Some(3))
        ]
    );
}

#[tokio::test]
async fn st25_reasons_none_means_no_key() {
    // Scenario: 事情の無い記録なしには何も添えない
    let pool = testdb::pool().await;
    let u = testdb::user();
    let v = view(&pool, u, REASON_DATE, reasons_now()).await;
    let r = no_record_at(&v, "2030-01-05T00:00:00+09:00");
    assert!(r.reasons.is_empty());
    let json = serde_json::to_value(r).unwrap();
    assert!(
        json.get("reasons").is_none(),
        "事情が無ければ鍵ごと出さない: {json}"
    );
}

#[tokio::test]
async fn st25_reasons_do_not_change_the_row_range() {
    // Scenario: 事情は記録なしの行の範囲を変えない
    let pool = testdb::pool().await;
    let u = testdb::user();
    gap_day(&pool, u).await;
    let before = view(&pool, u, REASON_DATE, reasons_now()).await;
    testdb::put_drop(
        &pool,
        u,
        LOCATION,
        Some(("2030-01-05T13:00:00+09:00", "2030-01-05T14:00:00+09:00")),
        &[("2030-01-05T04:00:00Z", 60)],
    )
    .await;
    let after = view(&pool, u, REASON_DATE, reasons_now()).await;
    let r = no_record_at(&after, "2030-01-05T08:20:00+09:00");
    assert_eq!(r.end, t("2030-01-05T16:40:00+09:00"));
    assert_eq!(reasons_of(r), vec![(day::ReasonKind::Dropped, Some(60))]);
    let ranges = |v: &DayResponse| -> Vec<(RowKind, DateTime<Utc>, DateTime<Utc>)> {
        v.entries.iter().map(|e| (e.kind, e.start, e.end)).collect()
    };
    assert_eq!(
        ranges(&before),
        ranges(&after),
        "行の並びと範囲は事情で変わらない"
    );
}

/// 同じ地点の点を 2 分おきに 5 時間置いて作り直す（滞在になる材料）。
async fn dwell_points(pool: &PgPool, user: Uuid, source: &str) {
    for i in 0..150 {
        let at = t("2030-01-05T02:10:00+09:00") + chrono::Duration::minutes(i * 2);
        let p = json!({"lat": 35.0, "lon": 139.0});
        put(pool, user, source, "test", &at.to_rfc3339(), p).await;
    }
    crate::stay_store::rebuild_all(pool, user).await.unwrap();
}

#[tokio::test]
async fn st25_nostay_archive_locations_make_no_stay() {
    // Scenario: 書庫の位置からは滞在を作らない
    let pool = testdb::pool().await;
    // 対照: 手元の位置なら同じ点の並びが滞在になる
    let control = testdb::user();
    dwell_points(&pool, control, LOCATION).await;
    let v = view(&pool, control, REASON_DATE, reasons_now()).await;
    assert!(
        v.entries.iter().any(|e| e.kind == RowKind::Stay),
        "対照に滞在が無い: {:#?}",
        v.entries
    );

    let u = testdb::user();
    for source in [
        "c03-timeline-route",
        "c03-timeline-signal",
        "c03-legacy-location",
    ] {
        dwell_points(&pool, u, source).await;
    }
    let v = view(&pool, u, REASON_DATE, reasons_now()).await;
    assert!(
        !v.entries.iter().any(|e| e.kind == RowKind::Stay),
        "書庫の位置から滞在の行ができた: {:#?}",
        v.entries
    );
}

/// 「最後に届いた時刻」の SQL は利用者とソースと `ingest_time` の索引の末尾を読む（R6）。
/// 試験の DB は小さく計画が表の走査に寄るので、トランザクションの中で走査を止めて、索引で引けることだけを見る。
#[tokio::test]
async fn st25_browser_through_sql_uses_ingest_index() {
    let pool = testdb::pool().await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL enable_seqscan = off")
        .execute(&mut *tx)
        .await
        .unwrap();
    let plan: Vec<(String,)> = sqlx::query_as(&format!("EXPLAIN {}", day::BROWSER_THROUGH_SQL))
        .bind(testdb::user())
        .bind("c02-browser-history")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    let plan = plan.into_iter().map(|r| r.0).collect::<Vec<_>>().join("\n");
    assert!(plan.contains("event_by_user_source_ingest"), "{plan}");
    assert!(plan.contains("Index Only Scan Backward"), "{plan}");
}
