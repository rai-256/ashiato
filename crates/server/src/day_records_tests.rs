// SPDX-License-Identifier: AGPL-3.0-only
//! 1 日の記録の読み出し（ST25 / design D2〜D5）を、**本物の PostgreSQL に対して**確かめる。
//!
//! 許可リストの行ごとの試験は DB に触らない（`st25_allow_row_*`）。畳み・日の範囲は DB の側で効く
//! ので本物を叩く。**利用者は試験ごとに新しくする**（1 つの DB を共有したまま分ける）。
#![allow(clippy::unwrap_used)]

use crate::day_records::*;
use crate::day_records::{normalize, normalize_all, StoredEvent};
use crate::testdb;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// 試験の「いま」。読む日（2026-09-07）より後なので、離席中の扱いは掛からない。
fn now() -> DateTime<Utc> {
    "2026-09-20T00:00:00Z".parse().unwrap()
}

fn day() -> NaiveDate {
    testdb::date("2026-09-07")
}

/// 試験が使う論理ソースを登録簿に無ければ置く（書庫は本物の移行がまだ置かない）。
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

/// 記録 1 件の置き方。既定は内容ごとに別のハッシュ・`Asia/Tokyo`・感度 1・外部識別子なし。
struct Put<'a> {
    user: Uuid,
    source: &'a str,
    device: &'a str,
    at: &'a str,
    tz: &'a str,
    payload: Value,
    hash: Option<String>,
    external_id: Option<String>,
    sensitivity: i16,
    ingest: Option<&'a str>,
}

impl<'a> Put<'a> {
    fn new(user: Uuid, source: &'a str, at: &'a str, payload: Value) -> Self {
        Self {
            user,
            source,
            device: "pc-1",
            at,
            tz: "Asia/Tokyo",
            payload,
            hash: None,
            external_id: None,
            sensitivity: 1,
            ingest: None,
        }
    }

    async fn run(self, pool: &PgPool) -> Uuid {
        ensure_source(pool, self.source).await;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO core.event
               (id, user_id, logical_source, external_id, device_id, origin, event_time, ingest_time,
                tz_offset_min, tz_id, schema_version, sensitivity, content_hash, raw, payload)
             VALUES ($1,$2,$3,$4,$5,'collected',$6::timestamptz, COALESCE($7::timestamptz, now()),
                     540,$8,1,$9,$10,'{}',$11)",
        )
        .bind(id)
        .bind(self.user)
        .bind(self.source)
        .bind(&self.external_id)
        .bind(self.device)
        .bind(self.at)
        .bind(self.ingest)
        .bind(self.tz)
        .bind(self.sensitivity)
        .bind(self.hash.unwrap_or_else(|| Uuid::new_v4().to_string()))
        .bind(&self.payload)
        .execute(pool)
        .await
        .unwrap();
        id
    }
}

fn window(kind: &str, at: &str) -> Value {
    json!({"kind": kind, "at": at})
}

fn foreground(at: &str, app: &str) -> Value {
    json!({"kind": "foreground", "at": at, "app_name": app, "process_name": "x.exe", "title": "題名"})
}

async fn listed(pool: &PgPool, user: Uuid) -> Vec<Norm> {
    read_day(pool, user, day(), now())
        .await
        .unwrap()
        .into_iter()
        .filter(Norm::is_listed)
        .collect()
}

fn row(source: &str, at: &str, payload: Value) -> StoredEvent {
    StoredEvent {
        id: Uuid::new_v4(),
        logical_source: source.into(),
        display_name: format!("表示名 {source}"),
        event_time: at.parse().unwrap(),
        ingest_time: at.parse().unwrap(),
        tz_id: "Asia/Tokyo".into(),
        device_id: Some("d".into()),
        payload,
    }
}

// ------------------------------------------------------------------ 1.1 登録簿と表

/// 登録簿に実在する書庫（`c03-%`）の論理ソースは、どれも D2（活動）か D11（位置の点）の表にある。
/// **表に無い名前が増えたら、行に出すか位置の点にするかを決めさせる。**
#[tokio::test]
async fn st25_allow_registry_covers_archive() {
    let pool = testdb::pool().await;
    let names: Vec<(String,)> =
        sqlx::query_as("SELECT logical_source FROM core.source WHERE logical_source LIKE 'c03-%'")
            .fetch_all(&pool)
            .await
            .unwrap();
    let unknown: Vec<&str> = names
        .iter()
        .map(|n| n.0.as_str())
        .filter(|n| {
            !ARCHIVE_SOURCES
                .iter()
                .chain(ARCHIVE_LOCATION_SOURCES)
                .any(|p| source_matches(p, n))
        })
        .collect();
    assert!(
        unknown.is_empty(),
        "D2 / D11 の表に無い書庫のソース: {unknown:?}"
    );
}

// ------------------------------------------------------------------ 1.2 許可リストの行ごと（偽の記録を 1 件ずつ）

fn norm_of(source: &str, payload: Value) -> Option<Norm> {
    normalize(&row(source, "2026-09-07T01:00:00Z", payload))
}

#[test]
fn st25_allow_row_pc_foreground() {
    let n = norm_of(
        "c02-window",
        json!({"kind":"foreground","app_name":"Code","title":"t","url":"https://Example.com:8080/p?q=1"}),
    )
    .unwrap();
    assert_eq!((n.kind, n.origin), (Kind::Foreground, Origin::Pc));
    assert_eq!(n.app.as_deref(), Some("Code"));
    assert_eq!(n.title.as_deref(), Some("t"));
    assert_eq!(n.domain.as_deref(), Some("example.com"));
    let n = norm_of(
        "c02-window",
        json!({"kind":"foreground","process_name":"a.exe"}),
    )
    .unwrap();
    assert_eq!(
        n.app.as_deref(),
        Some("a.exe"),
        "app_name が無ければ process_name"
    );
}

#[test]
fn st25_allow_row_pc_away_is_leave_only() {
    let leave = json!({"kind":"idle","transition":"leave","range_end":"2026-09-07T01:30:00Z"});
    let n = norm_of("c02-window", leave).unwrap();
    assert_eq!(n.kind, Kind::Away);
    assert_eq!(n.end, Some("2026-09-07T01:30:00Z".parse().unwrap()));
    assert!(norm_of("c02-window", json!({"kind":"idle","transition":"enter"})).is_none());
}

#[test]
fn st25_allow_row_pc_stopped() {
    let n = norm_of(
        "c02-window",
        json!({"kind":"powered-off","range_end":"2026-09-07T02:00:00Z"}),
    )
    .unwrap();
    assert_eq!((n.kind, n.origin), (Kind::Stopped, Origin::Pc));
    assert_eq!(n.end, Some("2026-09-07T02:00:00Z".parse().unwrap()));
}

#[test]
fn st25_allow_row_pc_excluded() {
    let n = norm_of(
        "c02-window",
        json!({"kind":"excluded","range_end":"2026-09-07T01:10:00Z","excluded_count":3,
               "app_name":"秘密","title":"秘密","url":"https://secret.example/"}),
    )
    .unwrap();
    assert_eq!(n.kind, Kind::Excluded);
    assert_eq!(n.excluded_count, Some(3));
    assert_eq!((&n.app, &n.title, &n.domain), (&None, &None, &None));
}

#[test]
fn st25_allow_row_browser_visit() {
    let n = norm_of(
        "c02-browser-history",
        json!({"kind":"visit","browser":"chrome","title":"t","url":"https://user@example.org/a"}),
    )
    .unwrap();
    assert_eq!((n.kind, n.origin), (Kind::Visit, Origin::Browser));
    assert_eq!(n.app.as_deref(), Some("chrome"));
    assert_eq!(n.domain.as_deref(), Some("example.org"));
    assert!(norm_of("c02-browser-history", json!({"kind":"vanished"})).is_none());
    assert!(norm_of("c02-browser-history", json!({"kind":"excluded"})).is_none());
    assert!(norm_of("c02-browser-history", json!({"kind":"profiles"})).is_none());
}

#[test]
fn st25_allow_row_phone_foreground_and_background() {
    let n = norm_of(
        "c01-app-usage",
        json!({"event_type":1,"package":"p.q","app_label":"ラベル"}),
    )
    .unwrap();
    assert_eq!((n.kind, n.origin), (Kind::Foreground, Origin::Phone));
    assert_eq!(n.app.as_deref(), Some("ラベル"));
    let n = norm_of("c01-app-usage", json!({"event_type":1,"package":"p.q"})).unwrap();
    assert_eq!(
        n.app.as_deref(),
        Some("p.q"),
        "app_label が無ければ package"
    );
    for t in [2, 23] {
        let n = norm_of("c01-app-usage", json!({"event_type":t,"package":"p.q"})).unwrap();
        assert_eq!(n.kind, Kind::Background);
        assert!(!n.is_listed(), "後ろに下がった記録は内部だけ");
    }
    assert!(norm_of("c01-app-usage", json!({"event_type":5,"package":"p.q"})).is_none());
}

#[test]
fn st25_allow_row_phone_gap() {
    let n = norm_of(
        "c01-app-usage",
        json!({"kind":"gap","begin":"2026-09-07T00:00:00Z","end":"2026-09-07T03:00:00Z","reason":"retention"}),
    )
    .unwrap();
    assert_eq!(n.kind, Kind::Unavailable);
    assert_eq!(n.reason.as_deref(), Some("retention"));
    assert_eq!(
        n.start,
        "2026-09-07T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
    );
    assert_eq!(n.end, Some("2026-09-07T03:00:00Z".parse().unwrap()));
}

#[test]
fn st25_allow_row_archive() {
    for src in [
        "c03-chrome-history",
        "c03-youtube-watch",
        "c03-youtube-search",
        "c03-myactivity-youtube",
        "c03-timeline-visit",
        "c03-timeline-move",
        "c03-legacy-visit",
        "c03-legacy-activity",
    ] {
        let n = norm_of(src, json!({"title":"t","url":"https://a.example/x"})).unwrap();
        assert_eq!(
            (n.kind, n.origin),
            (Kind::Archive, Origin::Archive),
            "{src}"
        );
        assert_eq!(
            n.source_label.as_deref(),
            Some(format!("表示名 {src}").as_str())
        );
        assert_eq!(n.domain.as_deref(), Some("a.example"));
    }
    let n = norm_of(
        "c03-myactivity-search",
        json!({"product":"検索","query":"q"}),
    )
    .unwrap();
    assert_eq!(
        (n.product.as_deref(), n.title.as_deref()),
        (Some("検索"), Some("q"))
    );
    let n = norm_of("c03-timeline-visit", json!({"semantic_type":"HOME"})).unwrap();
    assert_eq!(n.title.as_deref(), Some("HOME"));
    let n = norm_of("c03-legacy-activity", json!({"activity_type":"WALKING"})).unwrap();
    assert_eq!(n.title.as_deref(), Some("WALKING"));
}

#[test]
fn st25_allow_row_unlisted_sources_are_not_normalized() {
    for src in [
        "c01-location",
        "s01-stay",
        "s01-attribute",
        "c01-app-usage-rollup",
        "c01-clock",
        "clock-skew",
        "c03-timeline-route",
        "c03-timeline-signal",
        "c03-legacy-location",
    ] {
        assert!(
            norm_of(src, json!({"kind":"foreground","event_type":1})).is_none(),
            "{src}"
        );
    }
    assert!(norm_of("c02-window", json!({"kind":"clock-skew"})).is_none());
}

#[test]
fn st25_allow_row_open_enter_is_away_until_now_only_today() {
    let now: DateTime<Utc> = "2026-09-07T05:00:00Z".parse().unwrap();
    let enter = |at: &str| {
        row(
            "c02-window",
            at,
            json!({"kind":"idle","transition":"enter"}),
        )
    };
    // leave の無い enter（今日）→ いままでの離席
    let out = normalize_all(&[enter("2026-09-07T04:00:00Z")], now);
    assert_eq!(out.len(), 1);
    assert_eq!((out[0].kind, out[0].end), (Kind::Away, Some(now)));
    // leave が同じ時刻・同じ端末にあれば、leave だけを使う
    let leave = row(
        "c02-window",
        "2026-09-07T04:00:00Z",
        json!({"kind":"idle","transition":"leave","range_end":"2026-09-07T04:20:00Z"}),
    );
    let out = normalize_all(&[enter("2026-09-07T04:00:00Z"), leave], now);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].end, Some("2026-09-07T04:20:00Z".parse().unwrap()));
    // 今日でない enter はいままでにしない
    assert!(normalize_all(&[enter("2026-09-05T04:00:00Z")], now).is_empty());
}

// Scenario: 許可リストに無い種別の記録は返らない
#[tokio::test]
async fn st25_allow_unlisted_kinds_are_not_returned() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-07T01:00:00Z";
    let unlisted: [(&str, Value); 7] = [
        ("c02-window", window("clock-skew", at)),
        (
            "c01-app-usage-rollup",
            json!({"granularity":"daily","package":"p"}),
        ),
        ("s01-attribute", json!({"claim":"x"})),
        ("s01-stay", json!({"lat":35.0})),
        (
            "c02-browser-history",
            json!({"kind":"vanished","url":"https://a.example/"}),
        ),
        ("c01-app-usage", json!({"event_type":23,"package":"p.q"})),
        ("c01-location", json!({"lat":35.0,"lon":139.0})),
    ];
    for (src, payload) in unlisted {
        Put::new(u, src, at, payload).run(&pool).await;
    }
    let shown = Put::new(u, "c02-window", at, foreground(at, "Code"))
        .run(&pool)
        .await;
    let got = listed(&pool, u).await;
    assert_eq!(got.iter().map(|n| n.id).collect::<Vec<_>>(), vec![shown]);
}

// Scenario: 除外の記録は件数を持つ
#[tokio::test]
async fn st25_allow_excluded_has_count() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let p = json!({"kind":"excluded","at":"2026-09-07T01:00:00Z","range_end":"2026-09-07T01:20:00Z","excluded_count":3});
    Put::new(u, "c02-window", "2026-09-07T01:00:00Z", p)
        .run(&pool)
        .await;
    let got = listed(&pool, u).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].kind, Kind::Excluded);
    assert_eq!(
        got[0].start,
        "2026-09-07T01:00:00Z".parse::<DateTime<Utc>>().unwrap()
    );
    assert_eq!(got[0].end, Some("2026-09-07T01:20:00Z".parse().unwrap()));
    assert_eq!(got[0].excluded_count, Some(3));
}

// Scenario: 除外の記録は本文を持たない
#[tokio::test]
async fn st25_allow_excluded_has_no_body() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 本文を持たないのが契約だが、万一入っていても写さないことまで固定する
    let p = json!({"kind":"excluded","range_end":"2026-09-07T01:20:00Z","excluded_count":1,
                   "app_name":"秘密のアプリ","title":"秘密の題名","url":"https://secret.example/"});
    Put::new(u, "c02-window", "2026-09-07T01:00:00Z", p)
        .run(&pool)
        .await;
    let got = listed(&pool, u).await;
    assert_eq!(got.len(), 1);
    assert_eq!(
        (&got[0].app, &got[0].title, &got[0].domain),
        (&None, &None, &None)
    );
    assert_eq!(got[0].product, None);
}

// Scenario: 地域の違う記録はその地域を持って返る
#[tokio::test]
async fn st25_allow_keeps_region() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-07T01:00:00Z";
    let mut p = Put::new(u, "c02-window", at, foreground(at, "Code"));
    p.tz = "Europe/London";
    p.run(&pool).await;
    let got = listed(&pool, u).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].tz_id, "Europe/London");
}

// Scenario: 感度の高い記録も返る
#[tokio::test]
async fn st25_allow_sensitivity_does_not_filter() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 感度は 0〜3 の 4 段階。どちらの端の値でも外さない（C5）
    for (i, sens) in [0_i16, 3].into_iter().enumerate() {
        let at = format!("2026-09-07T0{}:00:00Z", i + 1);
        let mut p = Put::new(u, "c02-window", &at, foreground(&at, "Code"));
        p.sensitivity = sens;
        p.run(&pool).await;
    }
    assert_eq!(listed(&pool, u).await.len(), 2);
}

/// 今日に限り、leave の無い enter は読み出しでもいままでの離席になる。
#[tokio::test]
async fn st25_allow_open_enter_reads_as_away_today() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-07T03:00:00Z";
    Put::new(
        u,
        "c02-window",
        at,
        json!({"kind":"idle","transition":"enter","at":at}),
    )
    .run(&pool)
    .await;
    let now: DateTime<Utc> = "2026-09-07T05:00:00Z".parse().unwrap();
    let got = read_day(&pool, u, day(), now).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!((got[0].kind, got[0].end), (Kind::Away, Some(now)));
}

// ------------------------------------------------------------------ 1.3 畳み

// Scenario: 削除済みの記録は返らない
#[tokio::test]
async fn st25_fold_deleted_is_not_returned() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let mut ids = vec![];
    for h in 1..=3 {
        let at = format!("2026-09-07T0{h}:00:00Z");
        ids.push(
            Put::new(u, "c02-window", &at, foreground(&at, "Code"))
                .run(&pool)
                .await,
        );
    }
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
        .bind(ids[1])
        .execute(&pool)
        .await
        .unwrap();
    let got = listed(&pool, u).await;
    assert_eq!(got.len(), 2);
    assert!(got.iter().all(|n| n.id != ids[1]));
}

// Scenario: 内容が同じ記録は 1 件として返る
#[tokio::test]
async fn st25_fold_same_content_is_one() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-07T01:00:00Z";
    let mut first = Uuid::nil();
    for (i, ingest) in ["2026-09-07T02:00:00Z", "2026-09-07T01:30:00Z"]
        .into_iter()
        .enumerate()
    {
        let mut p = Put::new(u, "c02-window", at, foreground(at, "Code"));
        p.hash = Some("same-content".into());
        p.external_id = Some(format!("ext-{i}"));
        p.ingest = Some(ingest);
        let id = p.run(&pool).await;
        if i == 1 {
            first = id; // ingest_time の最も古い行が代表
        }
    }
    let got = listed(&pool, u).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, first);
}

// Scenario: 2 台の PC から届いた同じ訪問は 1 件になる
#[tokio::test]
async fn st25_fold_same_visit_from_two_pcs_is_one() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-07T01:00:00.123456Z";
    let visit = json!({"kind":"visit","browser":"chrome","title":"t","url":"https://a.example/x"});
    let mut older = Uuid::nil();
    for (device, ingest) in [
        ("pc-1", "2026-09-07T02:00:00Z"),
        ("pc-2", "2026-09-07T01:30:00Z"),
    ] {
        let mut p = Put::new(u, "c02-browser-history", at, visit.clone());
        p.device = device;
        p.ingest = Some(ingest);
        p.external_id = Some(format!("ext-{device}"));
        let id = p.run(&pool).await;
        if device == "pc-2" {
            older = id;
        }
    }
    // 別の URL の訪問は畳まれない
    let other = json!({"kind":"visit","browser":"chrome","title":"t","url":"https://a.example/y"});
    Put::new(u, "c02-browser-history", at, other)
        .run(&pool)
        .await;
    let got = listed(&pool, u).await;
    assert_eq!(got.len(), 2);
    assert!(got.iter().any(|n| n.id == older));
}

// Scenario: 書庫と手元の同じ出来事は両方返る
#[tokio::test]
async fn st25_fold_archive_and_local_are_both_returned() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-07T01:00:00Z";
    let visit = json!({"kind":"visit","browser":"chrome","title":"t","url":"https://a.example/x"});
    Put::new(u, "c02-browser-history", at, visit)
        .run(&pool)
        .await;
    let arch = json!({"title":"t","url":"https://a.example/x"});
    Put::new(u, "c03-chrome-history", at, arch).run(&pool).await;
    let got = listed(&pool, u).await;
    assert_eq!(got.len(), 2);
    let a = got.iter().find(|n| n.origin == Origin::Archive).unwrap();
    assert_eq!(a.source_label.as_deref(), Some("表示名 c03-chrome-history"));
    assert!(got.iter().any(|n| n.origin == Origin::Browser));
}

// Scenario: 書庫どうしの同じ出来事も両方返る
#[tokio::test]
async fn st25_fold_archives_with_same_event_are_both_returned() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-07T01:00:00Z";
    let video = json!({"title":"動画","url":"https://www.youtube.com/watch?v=abc"});
    Put::new(u, "c03-youtube-watch", at, video.clone())
        .run(&pool)
        .await;
    Put::new(u, "c03-myactivity-youtube", at, video)
        .run(&pool)
        .await;
    let got = listed(&pool, u).await;
    assert_eq!(got.len(), 2);
    let mut labels: Vec<_> = got.iter().filter_map(|n| n.source_label.clone()).collect();
    labels.sort();
    assert_eq!(
        labels,
        ["表示名 c03-myactivity-youtube", "表示名 c03-youtube-watch"]
    );
}

// ------------------------------------------------------------------ 1.4 日の範囲

// Scenario: 日の区切りは Asia/Tokyo で決まる
#[tokio::test]
async fn st25_range_day_boundary_is_asia_tokyo() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-06T15:30:00Z"; // Asia/Tokyo で 9 月 7 日 0:30
    Put::new(u, "c02-window", at, foreground(at, "Code"))
        .run(&pool)
        .await;
    let on = |d: &str| read_day(&pool, u, testdb::date(d), now());
    assert_eq!(on("2026-09-07").await.unwrap().len(), 1);
    assert!(on("2026-09-06").await.unwrap().is_empty());
}

// Scenario: 前の日から続く区間はその日にも返る
#[tokio::test]
async fn st25_range_interval_from_previous_day_is_returned() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 前の日の 23:40（JST）から 9 月 7 日の 8:10（JST）まで止まっていた
    let start = "2026-09-06T14:40:00Z";
    let end = "2026-09-06T23:10:00Z";
    let p = json!({"kind":"powered-off","at":start,"range_end":end});
    Put::new(u, "c02-window", start, p).run(&pool).await;
    let got = listed(&pool, u).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].kind, Kind::Stopped);
    assert_eq!(got[0].start, start.parse::<DateTime<Utc>>().unwrap());
    assert_eq!(got[0].end, Some(end.parse().unwrap()));
    // 次の日は始まりが前の日で終わりが今日なので、前の日の側にも返る
    let prev = read_day(&pool, u, testdb::date("2026-09-06"), now())
        .await
        .unwrap();
    assert_eq!(prev.len(), 1);
}

// Scenario: 他の日の記録は返らない
#[tokio::test]
async fn st25_range_other_day_is_not_returned() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-08T03:00:00Z";
    Put::new(u, "c02-window", at, foreground(at, "Code"))
        .run(&pool)
        .await;
    assert!(listed(&pool, u).await.is_empty());
    // 区間は終わりが日の始まりちょうどなら重ならない
    let start = "2026-09-06T10:00:00Z";
    let end = "2026-09-06T15:00:00Z"; // = 9 月 7 日 0:00（JST）
    let p = json!({"kind":"powered-off","at":start,"range_end":end});
    Put::new(u, "c02-window", start, p).run(&pool).await;
    assert!(listed(&pool, u).await.is_empty());
}

/// `from` / `to` を指定しても同じ判定（始まりを含み、終わりを含まない）。
#[tokio::test]
async fn st25_range_from_to_uses_same_overlap_rule() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let at = "2026-09-07T03:00:00Z";
    Put::new(u, "c02-window", at, foreground(at, "Code"))
        .run(&pool)
        .await;
    let from: DateTime<Utc> = "2026-09-07T03:00:00Z".parse().unwrap();
    let to = from + Duration::hours(1);
    let got = read(&pool, u, from, to, now()).await.unwrap();
    assert_eq!(got.len(), 1, "from は含む");
    let got = read(&pool, u, from + Duration::seconds(1), to, now())
        .await
        .unwrap();
    assert!(got.is_empty());
    let got = read(&pool, u, from - Duration::hours(1), from, now())
        .await
        .unwrap();
    assert!(got.is_empty(), "to は含まない");
}

/// 読みの SQL は論理ソースで索引を引く（`LIKE ANY` に戻すと全ソースを走る）。
/// 試験の DB は小さく、計画が時間の索引に寄るので、トランザクションの中でそれを落として
/// （ロールバックで戻る）、この SQL が論理ソースを索引条件にできることを見る。
#[tokio::test]
async fn st25_read_sql_uses_source_time_index() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let from: DateTime<Utc> = "2026-09-07T00:00:00Z".parse().unwrap();
    let sources: Vec<String> = sqlx::query_scalar(SOURCES_SQL)
        .bind(like_patterns())
        .fetch_all(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("DROP INDEX core.event_by_user_time_live")
        .execute(&mut *tx)
        .await
        .unwrap();
    let plan: Vec<(String,)> = sqlx::query_as(&format!("EXPLAIN {FETCH_SQL}"))
        .bind(u)
        .bind(sources)
        .bind(from)
        .bind(from + Duration::days(1))
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    let plan = plan.into_iter().map(|r| r.0).collect::<Vec<_>>().join("\n");
    // 論理ソースが索引条件に乗る（`LIKE ANY` だと Filter に落ちて全ソースを走る）。
    assert!(plan.contains("logical_source = ANY"), "{plan}");
    assert!(!plan.contains("Filter: ((logical_source"), "{plan}");
}

// ------------------------------------------------------------------ 2 `GET /day/records`

const TOKEN: &str = "test-token-0123456789abcdef";

async fn app() -> crate::App {
    crate::App::for_test(testdb::pool().await, TOKEN)
}

fn auth() -> axum::http::HeaderMap {
    let mut h = axum::http::HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

async fn call(
    app: &crate::App,
    headers: axum::http::HeaderMap,
    user: Uuid,
    date: &str,
    range: Option<(&str, &str)>,
) -> Result<Value, (axum::http::StatusCode, String)> {
    let q = crate::DayRecordsQuery {
        date: date.into(),
        from: range.map(|r| r.0.into()),
        to: range.map(|r| r.1.into()),
        user_id: Some(user),
    };
    crate::day_records_get(
        axum::extract::State(app.clone()),
        headers,
        axum::extract::Query(q),
    )
    .await
    .map(|axum::Json(v)| serde_json::to_value(v).unwrap())
}

fn visit(at: &str) -> Value {
    json!({"kind": "visit", "at": at, "browser": "Chrome", "title": "題", "url": format!("https://example.com/{at}")})
}

// Scenario: その日の記録が出来事の時刻順に 1 件ずつ返る
#[tokio::test]
async fn st25_api_records_are_in_event_order_one_by_one() {
    let app = app().await;
    let pool = &app.pool;
    let u = testdb::user();
    // 挿入順は時刻順にしない
    for at in [
        "2026-09-07T05:00:00Z",
        "2026-09-07T01:00:00Z",
        "2026-09-07T03:00:00Z",
    ] {
        Put::new(u, "c02-window", at, foreground(at, "Code"))
            .run(pool)
            .await;
    }
    for at in ["2026-09-07T04:00:00Z", "2026-09-07T02:00:00Z"] {
        Put::new(u, "c02-browser-history", at, visit(at))
            .run(pool)
            .await;
    }
    let at = "2026-09-07T06:00:00Z";
    let p = json!({"event_type": 1, "package": "com.x", "app_label": "メモ"});
    let mut phone = Put::new(u, "c01-app-usage", at, p);
    phone.device = "phone-1";
    phone.run(pool).await;

    let got = call(&app, auth(), u, "2026-09-07", None).await.unwrap();
    let rows = got.as_array().unwrap();
    assert_eq!(rows.len(), 6);
    let starts: Vec<&str> = rows.iter().map(|r| r["start"].as_str().unwrap()).collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted, "出来事の時刻順");
    let kinds: Vec<(&str, &str)> = rows
        .iter()
        .map(|r| (r["kind"].as_str().unwrap(), r["source"].as_str().unwrap()))
        .collect();
    assert_eq!(
        kinds,
        [
            ("foreground", "pc"),
            ("visit", "browser"),
            ("foreground", "pc"),
            ("visit", "browser"),
            ("foreground", "pc"),
            ("foreground", "phone"),
        ]
    );
    assert!(rows
        .iter()
        .all(|r| r["id"].is_string() && r["tz_id"] == "Asia/Tokyo"));
}

// Scenario: 時刻の範囲を指定するとその範囲の記録だけが返る
#[tokio::test]
async fn st25_api_records_range_returns_overlapping_only() {
    let app = app().await;
    let pool = &app.pool;
    let u = testdb::user();
    // 2026-09-07 の 10:00 – 11:00（JST）= 01:00 – 02:00Z
    for at in ["2026-09-07T00:30:00Z", "2026-09-07T01:30:00Z"] {
        Put::new(u, "c02-window", at, foreground(at, "Code"))
            .run(pool)
            .await;
    }
    // 9:50 – 10:20（JST）の離席: 範囲の外から始まって中で終わる
    let (b, e) = ("2026-09-07T00:50:00Z", "2026-09-07T01:20:00Z");
    let away = json!({"kind": "idle", "transition": "leave", "at": b, "range_end": e});
    Put::new(u, "c02-window", b, away).run(pool).await;

    let got = call(
        &app,
        auth(),
        u,
        "2026-09-07",
        Some(("2026-09-07T01:00:00Z", "2026-09-07T02:00:00Z")),
    )
    .await
    .unwrap();
    let rows = got.as_array().unwrap();
    assert_eq!(rows.len(), 2, "{got}");
    assert_eq!(rows[0]["kind"], "away");
    assert_eq!(rows[0]["start"], "2026-09-07T00:50:00Z");
    assert_eq!(rows[0]["end"], "2026-09-07T01:20:00Z");
    assert_eq!(rows[1]["kind"], "foreground");
    assert_eq!(rows[1]["start"], "2026-09-07T01:30:00Z");
}

// 日をまたぐ行（09-07 23:00 – 09-08 02:00 JST の滞在）を 09-08 の画面から開くと、前の日の側の記録も出る。
// **明示の `from` / `to` は日の端で切らない**（design D7。行の範囲は行の実際の始まりから終わりまで）
#[tokio::test]
async fn st25_api_records_explicit_range_is_not_clamped_to_the_date() {
    let app = app().await;
    let pool = &app.pool;
    let u = testdb::user();
    // 09-07 23:30 JST = 14:30Z（前の日の側）と 09-08 01:00 JST = 16:00Z（その日の側）
    for at in ["2026-09-07T14:30:00Z", "2026-09-07T16:00:00Z"] {
        Put::new(u, "c02-window", at, foreground(at, "Code"))
            .run(pool)
            .await;
    }
    let got = call(
        &app,
        auth(),
        u,
        "2026-09-08",
        Some(("2026-09-07T14:00:00Z", "2026-09-07T17:00:00Z")),
    )
    .await
    .unwrap();
    let starts: Vec<&str> = got
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["start"].as_str().unwrap())
        .collect();
    assert_eq!(
        starts,
        ["2026-09-07T14:30:00Z", "2026-09-07T16:00:00Z"],
        "{got}"
    );
    // 範囲を省けば日の端（09-08 の 0:00 から）のまま
    let got = call(&app, auth(), u, "2026-09-08", None).await.unwrap();
    let starts: Vec<&str> = got
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["start"].as_str().unwrap())
        .collect();
    assert_eq!(starts, ["2026-09-07T16:00:00Z"], "{got}");
}

// 始まりが終わり以後の範囲は、黙って空を返さず断る
#[tokio::test]
async fn st25_api_records_empty_range_is_400() {
    let app = app().await;
    for range in [
        ("2026-09-07T02:00:00Z", "2026-09-07T01:00:00Z"),
        ("2026-09-07T01:00:00Z", "2026-09-07T01:00:00Z"),
    ] {
        let err = call(&app, auth(), testdb::user(), "2026-09-07", Some(range))
            .await
            .unwrap_err();
        assert_eq!(err.0, axum::http::StatusCode::BAD_REQUEST, "{range:?}");
    }
}

// Scenario: 日付として読めない求めは断られる
#[tokio::test]
async fn st25_api_records_invalid_date_is_400() {
    let app = app().await;
    let err = call(&app, auth(), testdb::user(), "2026-13-40", None)
        .await
        .unwrap_err();
    assert_eq!(err.0, axum::http::StatusCode::BAD_REQUEST);
    let err = call(
        &app,
        auth(),
        testdb::user(),
        "2026-09-07",
        Some(("きのう", "2026-09-07T02:00:00Z")),
    )
    .await
    .unwrap_err();
    assert_eq!(err.0, axum::http::StatusCode::BAD_REQUEST);
}

// Scenario: 資格情報の無い 1 日の記録の求めは断られる
#[tokio::test]
async fn st25_api_records_without_credentials_is_401() {
    let app = app().await;
    let err = call(
        &app,
        axum::http::HeaderMap::new(),
        testdb::user(),
        "2026-09-07",
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(err.0, axum::http::StatusCode::UNAUTHORIZED);
}

// ------------------------------------------------------------------ 2.2 既存の口の形

fn keys(v: &Value) -> Vec<String> {
    let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
    k.sort();
    k
}

// Scenario: 記録の読み出しの形は変わらない
#[tokio::test]
async fn st25_api_unchanged_events_and_stays_shapes() {
    use axum::extract::{Query, State};
    let app = app().await;
    let u = testdb::user();
    let at = "2026-09-07T03:00:00Z";
    Put::new(u, "c02-window", at, foreground(at, "Code"))
        .run(&app.pool)
        .await;
    // 同じ地点に 30 分いた位置の記録で、`/stays` の行を stay と no-record の 2 つ以上にする
    for i in 0..=30 {
        let at = (DateTime::parse_from_rfc3339("2026-09-07T01:00:00Z").unwrap()
            + chrono::Duration::minutes(i))
        .to_rfc3339();
        Put::new(
            u,
            "c01-location",
            &at,
            json!({"lat": 35.0, "lon": 139.0, "acc_m": 12}),
        )
        .run(&app.pool)
        .await;
    }
    crate::stay_store::rebuild_all(&app.pool, u).await.unwrap();

    // `/events` は 6 つの鍵のまま（`payload` を返さない）、時刻順
    let axum::Json(events) = crate::events(State(app.clone()), auth()).await.unwrap();
    let events = serde_json::to_value(events).unwrap();
    let rows = events.as_array().unwrap();
    assert!(!rows.is_empty());
    for r in rows {
        assert_eq!(
            keys(r),
            [
                "event_time",
                "id",
                "logical_source",
                "origin",
                "raw",
                "tz_id"
            ]
        );
    }
    let times: Vec<DateTime<Utc>> = rows
        .iter()
        .map(|r| r["event_time"].as_str().unwrap().parse().unwrap())
        .collect();
    let mut sorted = times.clone();
    sorted.sort();
    assert_eq!(times, sorted);

    // `/stays` の応答の鍵の集合
    let axum::Json(stays) = crate::stays_get(
        State(app.clone()),
        auth(),
        Query(crate::StaysQuery {
            date: "2026-09-07".into(),
            user_id: Some(u),
        }),
    )
    .await
    .unwrap();
    let stays = serde_json::to_value(stays).unwrap();
    assert_eq!(keys(&stays), ["criteria", "date", "entries"], "{stays}");
    let entries = stays["entries"].as_array().unwrap();
    assert!(entries.len() >= 2, "{stays}");
    // 行の鍵の集合は種類ごとに固定（滞在だけが識別子と基準を持つ）
    for e in entries {
        let expected: &[&str] = if e["kind"] == "stay" {
            &["criteria_id", "end", "id", "kind", "start"]
        } else {
            &["end", "kind", "start"]
        };
        assert_eq!(keys(e), expected, "{e}");
        assert!(e["kind"].is_string() && e["start"].is_string() && e["end"].is_string());
    }
    assert!(entries.iter().any(|e| e["kind"] == "stay"), "{stays}");
    let starts: Vec<DateTime<Utc>> = entries
        .iter()
        .map(|e| e["start"].as_str().unwrap().parse().unwrap())
        .collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted);
}
