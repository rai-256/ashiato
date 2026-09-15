// SPDX-License-Identifier: AGPL-3.0-only
//! 破棄の報告の受け口（`/drops`）を、**本物の PostgreSQL に対して**確かめる（ST04 / design D7）。
//!
//! 形の検査そのもの（理由・範囲・時間ごとの件数）は `drops::tests` が見る。
//! ここは**口を通したときにだけ起きること**（部分拒否・冪等・原文の素通し・行の保護）を見る。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::drops::{drops_post, DropError, DropResult};
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

async fn app() -> App {
    App::for_test(testdb::pool().await, TOKEN)
}

fn auth() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

/// 破棄の報告 1 件ぶんの JSON。`hours` は (UTC の時, 件数)。範囲は最初の時の正時から最後の時の終わりまで。
fn report(source: &str, user: uuid::Uuid, day: &str, hours: &[(u32, i64)]) -> serde_json::Value {
    let first = hours.first().map(|h| h.0).unwrap_or(0);
    let last = hours.last().map(|h| h.0).unwrap_or(0);
    let count: i64 = hours.iter().map(|h| h.1).sum();
    let hourly: Vec<serde_json::Value> = hours
        .iter()
        .map(|(h, c)| serde_json::json!({"hour": format!("{day}T{h:02}:00:00Z"), "count": c}))
        .collect();
    let body = serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "user_id": user,
        "logical_source": source,
        "device_id": "test-dev",
        "reason": "age",
        "created_at": "2026-09-14T00:00:00Z",
        "range_start": format!("{day}T{first:02}:00:00Z"),
        "range_end": format!("{day}T{:02}:00:00Z", last + 1),
        "count": count,
        "hourly": hourly,
    });
    let mut out = body.clone();
    // 原文は端末が欄を組んだ文字列そのもの（design D4）
    out["raw"] = serde_json::json!(body.to_string());
    out
}

/// 稼働状況を引くための登録簿 1 行（鎖をたどらない）。
fn source_row(name: &str) -> coverage::SourceRow {
    coverage::SourceRow {
        logical_source: name.into(),
        named_source: name.into(),
        display_name: name.into(),
        expected_gap_sec: 21_600,
        collection_started_on: Some(testdb::date("2026-05-01")),
        retired_on: None,
        chain: vec![name.into()],
    }
}

async fn post(app: &App, body: serde_json::Value) -> (StatusCode, Vec<DropResult>) {
    let (code, Json(res)) = drops_post(State(app.clone()), auth(), Json(body))
        .await
        .expect("破棄の報告の受け口");
    (code, res)
}

async fn count_reports(app: &App, source: &str) -> i64 {
    let n: (i64,) =
        sqlx::query_as("SELECT count(*) FROM core.drop_report WHERE logical_source = $1")
            .bind(source)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    n.0
}

/// 移行は当て直しても壊れない（`run()` は起動のたびに全版を当てる）。
#[tokio::test]
async fn drop_reports_migration_applies_twice() {
    let pool = testdb::pool().await;
    let sql = MIGRATIONS
        .iter()
        .find(|(name, _)| *name == "202609151546_drop_reports")
        .expect("MIGRATIONS に 202609151546_drop_reports が無い")
        .1;
    let mut tx = pool.begin().await.unwrap();
    // testdb と同じ錠で直列にする（並んだテストの当て直しと競合しない）
    sqlx::query("SELECT pg_advisory_xact_lock(4820251)")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::raw_sql(sql).execute(&mut *tx).await.unwrap();
    sqlx::raw_sql(sql).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let t: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT to_regclass('core.drop_report')::text, to_regclass('core.drop_report_hour')::text",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(t.0.as_deref(), Some("core.drop_report"));
    assert_eq!(t.1.as_deref(), Some("core.drop_report_hour"));
}

/// Scenario: 破棄の報告を複数件まとめて受け取る
#[tokio::test]
async fn drops_api_batch() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-batch", 21_600).await;
    let u = testdb::user();
    let (code, res) = post(
        &app,
        serde_json::json!([
            report(&s, u, "2026-05-01", &[(1, 60)]),
            report(&s, u, "2026-05-02", &[(1, 60)]),
        ]),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(res.len(), 2);
    assert!(res.iter().all(|r| r.accepted && r.error.is_none()));
    assert_eq!(count_reports(&app, &s).await, 2);

    // 裸の 1 件も受ける（`/heartbeat` と同じ約束）
    let (code, res) = post(&app, report(&s, u, "2026-05-03", &[(1, 60)])).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(res.len(), 1);
}

/// Scenario: 一部が不正でも正しい破棄の報告は受け付けられる
#[tokio::test]
async fn drops_api_partial_reject() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-partial", 21_600).await;
    let u = testdb::user();
    let mut bad = report(&s, u, "2026-05-01", &[(1, 60)]);
    bad.as_object_mut().unwrap().remove("reason");
    let good = report(&s, u, "2026-05-02", &[(1, 60)]);
    let (code, res) = post(&app, serde_json::json!([bad, good])).await;
    assert_eq!(code, StatusCode::OK, "正しい分があるので要求は通る");
    assert!(!res[0].accepted);
    assert_eq!(res[0].error, Some(DropError::InvalidReason));
    assert!(res[1].accepted);
    assert_eq!(count_reports(&app, &s).await, 1);
}

/// Scenario: 破棄の報告がすべて拒否されたときだけ要求が拒否される
#[tokio::test]
async fn drops_api_400_only_when_none() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-400", 21_600).await;
    let u = testdb::user();
    let mut zero = report(&s, u, "2026-05-01", &[(1, 60)]);
    zero["count"] = serde_json::json!(0);
    let (code, res) = post(
        &app,
        serde_json::json!([zero, report("no-such-source", u, "2026-05-01", &[(1, 60)])]),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert_eq!(res.len(), 2, "本文は同じ形のまま返す（端末が読む）");
    assert!(res.iter().all(|r| !r.accepted));
    assert_eq!(res[1].error, Some(DropError::UnknownSource));
}

/// Scenario: 破棄の報告の拒否の応答に受け取った値が含まれない
#[tokio::test]
async fn drops_api_no_echo() {
    let app = app().await;
    let u = testdb::user();
    let secret = "secret-drop-source-7c1e";
    let (_, res) = post(&app, report(secret, u, "2026-05-01", &[(1, 60)])).await;
    assert_eq!(res[0].error, Some(DropError::UnknownSource));
    let body = serde_json::to_string(&res).unwrap();
    assert!(
        !body.contains(secret),
        "応答にソース名が反射している: {body}"
    );
    assert!(
        !body.contains("test-dev"),
        "応答に端末識別子が反射している: {body}"
    );
    assert!(
        !body.contains("2026-05-01"),
        "応答に範囲が反射している: {body}"
    );
}

/// Scenario: 同じ破棄の報告を 2 回送っても 1 つ
/// Scenario: 同じ報告を 2 回受けても日の件数は 1 回ぶん
#[tokio::test]
async fn drops_api_idempotent() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-idem", 21_600).await;
    testdb::set_started_on(&app.pool, &s, "2026-05-01").await;
    let u = testdb::user();
    // 10:00〜13:00 JST = 01:00〜04:00 UTC に 60 件ずつ
    let one = report(&s, u, "2026-05-01", &[(1, 60), (2, 60), (3, 60)]);
    let (_, first) = post(&app, one.clone()).await;
    assert!(first[0].accepted && !first[0].duplicate);
    // **端末が振った id が変わっても同じ 1 件**（鍵は原文から作る）
    let mut again = one.clone();
    again["id"] = serde_json::json!(uuid::Uuid::new_v4());
    let (code, second) = post(&app, again).await;
    assert_eq!(code, StatusCode::OK);
    assert!(second[0].duplicate);
    assert!(second[0].accepted, "重複でも未送信から取り除いてよい");
    assert_eq!(count_reports(&app, &s).await, 1);

    let hours: (i64, i64) = sqlx::query_as(
        "SELECT count(*), coalesce(sum(h.count), 0) FROM core.drop_report_hour h
           JOIN core.drop_report r ON r.id = h.report_id WHERE r.logical_source = $1",
    )
    .bind(&s)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(hours, (3, 180), "時間ごとの件数が二重に入っている");

    let d = testdb::date("2026-05-01");
    let src = source_row(&s);
    let got = coverage::of_source(&app.pool, Some(u), &src, d, d)
        .await
        .unwrap();
    assert_eq!(got.days[0].dropped_count, 180);
}

/// Scenario: 破棄の報告の原文が 1 バイトも変わらずに残る
#[tokio::test]
async fn drops_api_raw_passthrough() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-raw", 21_600).await;
    let u = testdb::user();
    let original = r#"{"b":1,"a":2,"a":3,"n":1.100,"m":1e2 }"#;
    let mut item = report(&s, u, "2026-05-01", &[(1, 60)]);
    item["raw"] = serde_json::json!(original);
    let (code, _) = post(&app, item).await;
    assert_eq!(code, StatusCode::OK);
    let row: (String,) =
        sqlx::query_as("SELECT raw FROM core.drop_report WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, original, "原文がバイト単位で一致していない");
}

/// Scenario: 破棄の報告の受信時刻が時刻のまま残る
#[tokio::test]
async fn drops_api_received_at_is_timestamptz() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-recv", 21_600).await;
    let u = testdb::user();
    post(&app, report(&s, u, "2026-05-01", &[(1, 60)])).await;
    let ty: (String,) = sqlx::query_as(
        "SELECT data_type FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'drop_report' AND column_name = 'received_at'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(ty.0, "timestamp with time zone");
    let t: (chrono::DateTime<chrono::Utc>,) =
        sqlx::query_as("SELECT received_at FROM core.drop_report WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    // 受信の直後に引いているので、いまとの差は数分に収まる（日に丸めていれば最大 24 時間ずれる）
    let lag = (chrono::Utc::now() - t.0).num_seconds().abs();
    assert!(
        lag < 600,
        "受信時刻がいまから {lag} 秒ずれている —— 日に丸めている疑い"
    );
}

/// Scenario: 1 件だけの破棄も範囲として残る
#[tokio::test]
async fn drops_api_single_record_range() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-one", 21_600).await;
    let u = testdb::user();
    // 端末の規則: 1 件だけ捨てたら終わりは出来事の時刻 + 1 ms（design D4）
    let body = serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": u, "logical_source": s, "device_id": "d1",
        "reason": "age", "created_at": "2026-09-14T00:00:00Z",
        "range_start": "2026-05-01T01:23:45Z", "range_end": "2026-05-01T01:23:45.001Z",
        "count": 1, "hourly": [{"hour": "2026-05-01T01:00:00Z", "count": 1}],
        "raw": "{\"one\":1}",
    });
    let (code, res) = post(&app, body).await;
    assert_eq!(code, StatusCode::OK, "{res:?}");
    let row: (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        "SELECT range_start, range_end FROM core.drop_report WHERE logical_source = $1",
    )
    .bind(&s)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(row.1 > row.0, "終わりが始まりより後でない");
}

/// Scenario: 破棄が端末と理由と時間ごとの件数で残る
#[tokio::test]
async fn drops_api_keeps_device_reason_and_hours() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-hours", 21_600).await;
    let u = testdb::user();
    let (code, _) = post(&app, report(&s, u, "2026-05-01", &[(10, 50), (11, 30)])).await;
    assert_eq!(code, StatusCode::OK);
    let row: (String, String, i32) = sqlx::query_as(
        "SELECT device_id, reason, count FROM core.drop_report WHERE logical_source = $1",
    )
    .bind(&s)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(row, ("test-dev".into(), "age".into(), 80));
    let hours: Vec<(chrono::DateTime<chrono::Utc>, i32)> = sqlx::query_as(
        "SELECT h.hour, h.count FROM core.drop_report_hour h
           JOIN core.drop_report r ON r.id = h.report_id
          WHERE r.logical_source = $1 ORDER BY h.hour",
    )
    .bind(&s)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    let got: Vec<(String, i32)> = hours.iter().map(|(h, c)| (h.to_rfc3339(), *c)).collect();
    assert_eq!(
        got,
        vec![
            ("2026-05-01T10:00:00+00:00".to_string(), 50),
            ("2026-05-01T11:00:00+00:00".to_string(), 30)
        ]
    );
}

/// 口を通して入れた破棄を、期間と件数として読み戻す（ST02 の Scenario を `/drops` 経由で置き直す）。
/// `coverage/tests/spans.rs` の同名の印は `coverage_span` を直接置く読み手の試験として残す。
///
/// Scenario: 破棄が期間と件数で残る
#[tokio::test]
async fn drops_api_stores_period_and_count() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-period", 21_600).await;
    let u = testdb::user();
    post(
        &app,
        report(&s, u, "2026-05-01", &[(1, 60), (2, 60), (3, 60)]),
    )
    .await;
    let row: (
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        i32,
    ) = sqlx::query_as(
        "SELECT range_start, range_end, count FROM core.drop_report WHERE logical_source = $1",
    )
    .bind(&s)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(
        row.0.map(|t| t.to_rfc3339()).as_deref(),
        Some("2026-05-01T01:00:00+00:00")
    );
    assert_eq!(
        row.1.map(|t| t.to_rfc3339()).as_deref(),
        Some("2026-05-01T04:00:00+00:00")
    );
    assert_eq!(row.2, 180);
}

/// Scenario: 読めなかった行の報告は範囲なしで受け付けられる
#[tokio::test]
async fn drops_api_rangeless_unreadable_does_not_change_state() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-unread", 21_600).await;
    testdb::set_started_on(&app.pool, &s, "2026-05-01").await;
    let u = testdb::user();
    testdb::put_heartbeat(&app.pool, u, &s, "2026-05-01T03:00:00Z", true).await;
    let d = testdb::date("2026-05-01");
    let src = source_row(&s);
    let before = coverage::of_source(&app.pool, Some(u), &src, d, d)
        .await
        .unwrap();

    let body = serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": u, "logical_source": s, "device_id": "d1",
        "reason": "unreadable", "created_at": "2026-05-01T05:00:00Z", "count": 2,
        "raw": "{\"unreadable\":2}",
    });
    let (code, res) = post(&app, body).await;
    assert_eq!(code, StatusCode::OK, "{res:?}");
    let after = coverage::of_source(&app.pool, Some(u), &src, d, d)
        .await
        .unwrap();
    assert_eq!(before.days[0].state, after.days[0].state);
    assert_eq!(after.days[0].dropped_count, 0);
}

/// 口を通して入れた報告も、DB の側で書き換えと削除を拒む（`tools/check-immutable.sh` は psql から見る）。
///
/// Scenario: 格納された破棄の報告は書き換えられない
#[tokio::test]
async fn drops_api_rows_are_immutable() {
    let app = app().await;
    let s = testdb::source(&app.pool, "drop-lock", 21_600).await;
    let u = testdb::user();
    post(&app, report(&s, u, "2026-05-01", &[(1, 60)])).await;
    for stmt in [
        "UPDATE core.drop_report SET count = 1 WHERE logical_source = $1",
        "UPDATE core.drop_report SET raw = '{}' WHERE logical_source = $1",
        "DELETE FROM core.drop_report WHERE logical_source = $1",
        "UPDATE core.drop_report_hour SET count = 1
          WHERE report_id IN (SELECT id FROM core.drop_report WHERE logical_source = $1)",
        "DELETE FROM core.drop_report_hour
          WHERE report_id IN (SELECT id FROM core.drop_report WHERE logical_source = $1)",
    ] {
        let r = sqlx::query(stmt).bind(&s).execute(&app.pool).await;
        assert!(r.is_err(), "拒まれなかった: {stmt}");
    }
    let row: (i32,) =
        sqlx::query_as("SELECT count FROM core.drop_report WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, 60);
}
