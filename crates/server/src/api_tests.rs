// SPDX-License-Identifier: AGPL-3.0-only
//! 受け口（`/ingest` / `/heartbeat`）と DB の不変条件を、**本物の PostgreSQL に対して**確かめる。
//!
//! 取り込み口を通さずに行を置く検査は `coverage::tests` にある。ここは
//! **口を通したときにだけ起きること**（冪等・部分拒否・原文の素通し・収集開始日の更新）を見る。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

async fn app() -> App {
    App {
        pool: testdb::pool().await,
        token: TOKEN.into(),
    }
}

fn auth() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

/// 生存信号 1 件ぶんの JSON。
fn hb(source: &str, user: uuid::Uuid, emitted: &str) -> serde_json::Value {
    serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "user_id": user,
        "logical_source": source,
        "device_id": "test-dev",
        "emitted_at": emitted,
        "capturable": true,
        "blockers": [],
        "attempts": 360,
        "successes": 230,
        "raw": r#"{"alive":true}"#,
    })
}

/// 記録 1 件ぶんの JSON。
fn ev(source: &str, user: uuid::Uuid, event_time: &str, tz: &str, raw: &str) -> serde_json::Value {
    serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "user_id": user,
        "logical_source": source,
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": event_time,
        "tz_offset_min": 540,
        "tz_id": tz,
        "schema_version": 1,
        "raw": raw,
        "payload": {},
    })
}

async fn post_hb(app: &App, body: serde_json::Value) -> (StatusCode, Vec<HeartbeatResult>) {
    let (code, Json(res)) = heartbeat_post(State(app.clone()), auth(), Json(body))
        .await
        .expect("生存信号の受け口");
    (code, res)
}

async fn post_ingest(app: &App, body: serde_json::Value) -> (StatusCode, Vec<IngestResult>) {
    let (code, Json(res)) = ingest(State(app.clone()), auth(), Json(body))
        .await
        .expect("取り込み口");
    (code, res)
}

// ------------------------------------------------------------------ 日境界

/// 日本時間の 0 時で日が変わる（深掘り Q2 / tasks 2.2）。
///
/// Scenario: 日本時間の 0 時を境に別の日になる
#[tokio::test]
async fn day_boundary_jst() {
    let app = app().await;
    let s = testdb::source(&app.pool, "dayjst", 21_600).await;
    let u = testdb::user();
    post_ingest(
        &app,
        serde_json::json!([
            ev(&s, u, "2026-03-01T14:59:59Z", "Asia/Tokyo", r#"{"a":1}"#),
            ev(&s, u, "2026-03-01T15:00:01Z", "Asia/Tokyo", r#"{"a":2}"#),
        ]),
    )
    .await;

    let rows: Vec<(chrono::NaiveDate, i32)> = sqlx::query_as(
        "SELECT day, event_count FROM core.coverage
          WHERE user_id = $1 AND logical_source = $2 ORDER BY day",
    )
    .bind(u)
    .bind(&s)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            (testdb::date("2026-03-01"), 1),
            (testdb::date("2026-03-02"), 1)
        ],
        "UTC で切ると両方 03-01 に入る"
    );
}

/// 記録に付いたタイムゾーンは日境界に効かない（tasks 2.3）。
/// **これが無いと「記録のタイムゾーンで切るほうが自然」と後から戻され、NFR-13 の分母が壊れる。**
///
/// Scenario: 記録のタイムゾーンが違っても日の区切りは動かない
#[tokio::test]
async fn day_boundary_ignores_record_tz() {
    let app = app().await;
    let s = testdb::source(&app.pool, "daytz", 21_600).await;
    let u = testdb::user();
    post_ingest(
        &app,
        serde_json::json!([ev(
            &s,
            u,
            "2026-03-01T15:00:01Z",
            "America/New_York",
            r#"{"a":1}"#
        )]),
    )
    .await;
    let days: Vec<(chrono::NaiveDate,)> =
        sqlx::query_as("SELECT day FROM core.coverage WHERE user_id = $1 AND logical_source = $2")
            .bind(u)
            .bind(&s)
            .fetch_all(&app.pool)
            .await
            .unwrap();
    assert_eq!(days, vec![(testdb::date("2026-03-02"),)]);
}

// ------------------------------------------------------------------ 収集開始日

/// 最初に届いたものの**出来事時刻**が収集開始日になる（第 6 回 Q24。tasks 1.4b）。
///
/// Scenario: 最初の信号が発信された日が開始日になる
/// Scenario: 溜めてから送っても開始日は取得した日になる
#[tokio::test]
async fn sets_started_on_first_arrival() {
    let app = app().await;
    // 生存信号なら発信時刻の日
    let s1 = testdb::source(&app.pool, "start-hb", 21_600).await;
    let u = testdb::user();
    post_hb(
        &app,
        serde_json::json!([hb(&s1, u, "2026-03-31T16:00:00Z")]), // = 04-01 01:00 JST
    )
    .await;
    assert_eq!(
        started_on(&app.pool, &s1).await,
        Some(testdb::date("2026-04-01"))
    );

    // 記録なら出来事時刻の日。**受信時刻ではない** ——
    // 04-01〜04-03 に取った記録を 04-03 にまとめて送っても開始日は 04-01
    let s2 = testdb::source(&app.pool, "start-ev", 21_600).await;
    post_ingest(
        &app,
        serde_json::json!([
            ev(&s2, u, "2026-04-03T01:00:00Z", "Asia/Tokyo", r#"{"a":3}"#),
            ev(&s2, u, "2026-04-02T01:00:00Z", "Asia/Tokyo", r#"{"a":2}"#),
            ev(&s2, u, "2026-04-01T01:00:00Z", "Asia/Tokyo", r#"{"a":1}"#),
        ]),
    )
    .await;
    assert_eq!(
        started_on(&app.pool, &s2).await,
        Some(testdb::date("2026-04-01"))
    );
}

/// 後から古い記録が届くと開始日は遡る（第 7 回 Q26。tasks 1.4b）。
/// **圏外の保持がこれを起こす** —— いちばん古い記録が最初に届くとは限らない。
///
/// Scenario: 後から古い記録が届くと開始日が遡る
#[tokio::test]
async fn started_on_moves_back() {
    let app = app().await;
    let s = testdb::source(&app.pool, "backdate", 21_600).await;
    let u = testdb::user();
    post_ingest(
        &app,
        serde_json::json!([ev(
            &s,
            u,
            "2026-04-05T01:00:00Z",
            "Asia/Tokyo",
            r#"{"a":5}"#
        )]),
    )
    .await;
    assert_eq!(
        started_on(&app.pool, &s).await,
        Some(testdb::date("2026-04-05"))
    );

    post_ingest(
        &app,
        serde_json::json!([ev(
            &s,
            u,
            "2026-04-01T01:00:00Z",
            "Asia/Tokyo",
            r#"{"a":1}"#
        )]),
    )
    .await;
    assert_eq!(
        started_on(&app.pool, &s).await,
        Some(testdb::date("2026-04-01")),
        "古い記録が後から届いたら開始日は前へ動く"
    );

    // **前へだけ動く。** 新しい記録が届いても戻らない
    post_ingest(
        &app,
        serde_json::json!([ev(
            &s,
            u,
            "2026-04-09T01:00:00Z",
            "Asia/Tokyo",
            r#"{"a":9}"#
        )]),
    )
    .await;
    assert_eq!(
        started_on(&app.pool, &s).await,
        Some(testdb::date("2026-04-01"))
    );

    // そして 04-01 は「導入前」ではなくなっている
    let src = coverage::sources(&app.pool, std::slice::from_ref(&s))
        .await
        .unwrap();
    let got = coverage::of_source(
        &app.pool,
        Some(u),
        &src[0],
        testdb::date("2026-04-01"),
        testdb::date("2026-04-01"),
    )
    .await
    .unwrap();
    assert_ne!(got.days[0].state, coverage::DayState::BeforeStart);
}

async fn started_on(pool: &sqlx::PgPool, source: &str) -> Option<chrono::NaiveDate> {
    let row: (Option<chrono::NaiveDate>,) =
        sqlx::query_as("SELECT collection_started_on FROM core.source WHERE logical_source = $1")
            .bind(source)
            .fetch_one(pool)
            .await
            .unwrap();
    row.0
}

// ------------------------------------------------------------------ 生存信号の受け口

/// 複数件をまとめて受け、1 件ごとの結果を返す（tasks 3.1）。1 件だけの裸の要求も受ける。
///
/// Scenario: 複数件を 1 回で受け取る
#[tokio::test]
async fn heartbeat_batch() {
    let app = app().await;
    let s = testdb::source(&app.pool, "hb-batch", 21_600).await;
    let u = testdb::user();
    let (code, res) = post_hb(
        &app,
        serde_json::json!([
            hb(&s, u, "2026-05-01T00:00:00Z"),
            hb(&s, u, "2026-05-01T06:00:00Z"),
            hb(&s, u, "2026-05-01T12:00:00Z"),
        ]),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(res.len(), 3);
    assert!(res.iter().all(|r| r.accepted));

    // 裸の 1 件も受ける（`/ingest` と同じ約束）
    let (code, res) = post_hb(&app, hb(&s, u, "2026-05-01T18:00:00Z")).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(res.len(), 1);
}

/// 一部が不正でも正しい分は受け付ける（tasks 3.2）。
///
/// Scenario: 一部が不正でも正しい分は受け付けられる
/// Scenario: 登録簿に無いソースの生存信号は拒否される
#[tokio::test]
async fn heartbeat_partial_reject() {
    let app = app().await;
    let s = testdb::source(&app.pool, "hb-partial", 21_600).await;
    let u = testdb::user();
    let (code, res) = post_hb(
        &app,
        serde_json::json!([
            hb(&s, u, "2026-05-01T00:00:00Z"),
            hb("no-such-source", u, "2026-05-01T06:00:00Z"),
            hb(&s, u, "2026-05-01T12:00:00Z"),
        ]),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "正しい分があるので要求は通る");
    assert!(res[0].accepted);
    assert!(!res[1].accepted);
    assert!(matches!(res[1].error, Some(HeartbeatError::UnknownSource)));
    assert!(res[2].accepted);
    // 拒否された 1 件は残っていない
    let n: (i64,) = sqlx::query_as("SELECT count(*) FROM core.heartbeat WHERE logical_source = $1")
        .bind("no-such-source")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(n.0, 0);
}

/// 1 件も受け付けなかったときだけ 400（tasks 3.3）。
///
/// Scenario: 1 件も受け付けなかったときだけ要求が拒否される
#[tokio::test]
async fn heartbeat_400_only_when_none() {
    let app = app().await;
    let u = testdb::user();
    let (code, res) = post_hb(
        &app,
        serde_json::json!([
            hb("no-such-source", u, "2026-05-01T00:00:00Z"),
            hb("no-such-source-2", u, "2026-05-01T06:00:00Z"),
        ]),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert_eq!(res.len(), 2, "**本文は同じ形のまま返す**（収集側が読む）");
    assert!(res.iter().all(|r| !r.accepted));
}

/// 拒否の応答に受け取った値を含めない（ST01 の 2.5 と同じ向き。tasks 3.3）。
///
/// Scenario: 拒否の応答に受け取った値が含まれない
#[tokio::test]
async fn heartbeat_no_echo() {
    let app = app().await;
    let u = testdb::user();
    let secret = "secret-source-name-9f3a";
    let (_, res) = post_hb(
        &app,
        serde_json::json!([hb(secret, u, "2026-05-01T00:00:00Z")]),
    )
    .await;
    let body = serde_json::to_string(&res).unwrap();
    assert!(
        !body.contains(secret),
        "応答に送ったソース名が反射している: {body}"
    );
    assert!(!body.contains("alive"), "原文が反射している: {body}");
}

/// 取得の試行回数と成功回数を受け取って保存する（第 5 回 Q17。tasks 3.3b）。
#[tokio::test]
async fn heartbeat_attempt_counts() {
    let app = app().await;
    let s = testdb::source(&app.pool, "hb-counts", 21_600).await;
    let u = testdb::user();
    post_hb(&app, serde_json::json!([hb(&s, u, "2026-05-01T00:00:00Z")])).await;
    let row: (i32, i32) =
        sqlx::query_as("SELECT attempts, successes FROM core.heartbeat WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row, (360, 230));
}

/// 回数を持たない信号と、成功が試行を超える信号は受け付けない（2 巡目 R7。tasks 3.3b）。
///
/// Scenario: 取得の回数を持たない信号は受け付けられない
/// Scenario: 成功が試行を超える信号は受け付けられない
#[tokio::test]
async fn heartbeat_rejects_bad_counts() {
    let app = app().await;
    let s = testdb::source(&app.pool, "hb-badcounts", 21_600).await;
    let u = testdb::user();

    let mut no_counts = hb(&s, u, "2026-05-01T00:00:00Z");
    no_counts.as_object_mut().unwrap().remove("attempts");
    let (code, res) = post_hb(&app, serde_json::json!([no_counts])).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(matches!(res[0].error, Some(HeartbeatError::Malformed)));

    let mut over = hb(&s, u, "2026-05-01T06:00:00Z");
    over["attempts"] = serde_json::json!(10);
    over["successes"] = serde_json::json!(11);
    let (code, res) = post_hb(&app, serde_json::json!([over])).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(matches!(res[0].error, Some(HeartbeatError::InvalidCounts)));

    let n: (i64,) = sqlx::query_as("SELECT count(*) FROM core.heartbeat WHERE logical_source = $1")
        .bind(&s)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(n.0, 0, "断った信号が残っている");
}

/// 理由の無い「取れない」は受け付けない（FR-78。tasks 3.4）。
#[tokio::test]
async fn heartbeat_rejects_blockerless() {
    let app = app().await;
    let s = testdb::source(&app.pool, "hb-blockerless", 21_600).await;
    let u = testdb::user();
    let mut bad = hb(&s, u, "2026-05-01T00:00:00Z");
    bad["capturable"] = serde_json::json!(false);
    let (code, res) = post_hb(&app, serde_json::json!([bad])).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(matches!(
        res[0].error,
        Some(HeartbeatError::MissingBlockers)
    ));

    // 理由があれば通り、理由が残る
    let mut ok = hb(&s, u, "2026-05-01T06:00:00Z");
    ok["capturable"] = serde_json::json!(false);
    ok["blockers"] = serde_json::json!(["permission"]);
    let (code, _) = post_hb(&app, serde_json::json!([ok])).await;
    assert_eq!(code, StatusCode::OK);
    let row: (Vec<String>,) =
        sqlx::query_as("SELECT blockers FROM core.heartbeat WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, vec!["permission".to_string()]);
}

/// 同じ内容を 2 回送っても行は 1 つ（第 4 回 Q13。tasks 3.5）。
/// **再送は常態** —— ST01 の Outbox は部分失敗の後で送り直す。
///
/// Scenario: 同じ生存信号を 2 回送っても 1 行
#[tokio::test]
async fn heartbeat_idempotent() {
    let app = app().await;
    let s = testdb::source(&app.pool, "hb-idem", 21_600).await;
    let u = testdb::user();
    let one = hb(&s, u, "2026-05-01T00:00:00Z");
    let (_, first) = post_hb(&app, serde_json::json!([one.clone()])).await;
    assert!(!first[0].duplicate);
    // **収集側が採番した id が変わっても同じ 1 件**（鍵は内容から作る）
    let mut again = one.clone();
    again["id"] = serde_json::json!(uuid::Uuid::new_v4());
    let (code, second) = post_hb(&app, serde_json::json!([again])).await;
    assert_eq!(code, StatusCode::OK);
    assert!(second[0].duplicate);
    assert!(second[0].accepted, "重複でも未送信から取り除いてよい");

    let n: (i64,) = sqlx::query_as("SELECT count(*) FROM core.heartbeat WHERE logical_source = $1")
        .bind(&s)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(n.0, 1);
}

/// 原文はバイト単位で素通し（0003 と同じ理由。tasks 3.6）。
/// **`jsonb` にするとキー順が変わり、重複キーが消え、数値の表記が展開される。**
#[tokio::test]
async fn heartbeat_raw_passthrough() {
    let app = app().await;
    let s = testdb::source(&app.pool, "hb-raw", 21_600).await;
    let u = testdb::user();
    let original = r#"{"b":1,"a":2,"a":3,"n":1.100,"m":1e2}"#;
    let mut item = hb(&s, u, "2026-05-01T00:00:00Z");
    item["raw"] = serde_json::json!(original);
    post_hb(&app, serde_json::json!([item])).await;

    let row: (String,) = sqlx::query_as("SELECT raw FROM core.heartbeat WHERE logical_source = $1")
        .bind(&s)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(row.0, original, "原文がバイト単位で一致していない");
}

/// 受信時刻は日に丸めず `timestamptz` のまま残る（第 4 回 Q14。tasks 3.7）。
///
/// Scenario: 受信時刻が時刻のまま残る
#[tokio::test]
async fn heartbeat_received_at_is_timestamptz() {
    let app = app().await;
    let s = testdb::source(&app.pool, "hb-recv", 21_600).await;
    let u = testdb::user();
    post_hb(&app, serde_json::json!([hb(&s, u, "2026-05-01T00:00:00Z")])).await;

    let ty: (String,) = sqlx::query_as(
        "SELECT data_type FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'heartbeat' AND column_name = 'received_at'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(ty.0, "timestamp with time zone");
    // 時刻の成分が残っている（日に丸めていない）
    let t: (chrono::DateTime<chrono::Utc>,) =
        sqlx::query_as("SELECT received_at FROM core.heartbeat WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    use chrono::Timelike as _;
    assert!(
        t.0.hour() != 0 || t.0.minute() != 0 || t.0.second() != 0 || t.0.nanosecond() != 0,
        "受信時刻が 00:00:00 ちょうど —— 日に丸めている疑い"
    );
}

/// 格納済みの生存信号は書き換えられない（第 4 回 Q13。tasks 4.1）。
/// **列ごとに確かめる** —— 1 つだけ効いていて他が素通しでも気付くように。
///
/// Scenario: 格納された生存信号は書き換えられない
#[tokio::test]
async fn heartbeat_is_immutable() {
    let app = app().await;
    let s = testdb::source(&app.pool, "hb-immutable", 21_600).await;
    let u = testdb::user();
    post_hb(&app, serde_json::json!([hb(&s, u, "2026-05-01T00:00:00Z")])).await;

    for (col, val) in [
        ("raw", "'{\"tampered\":1}'"),
        ("content_hash", "'rewritten'"),
        ("received_at", "'2000-01-01T00:00:00Z'"),
        ("emitted_at", "'2000-01-01T00:00:00Z'"),
        ("capturable", "false"),
        ("blockers", "ARRAY['forged']"),
        ("attempts", "0"),
        ("successes", "0"),
        ("device_id", "'forged'"),
        ("user_id", "'00000000-0000-0000-0000-000000000000'"),
    ] {
        let r = sqlx::query(&format!(
            "UPDATE core.heartbeat SET {col} = {val} WHERE logical_source = $1"
        ))
        .bind(&s)
        .execute(&app.pool)
        .await;
        assert!(r.is_err(), "{col} の書き換えが通った（FR-78 違反）");
    }
    // 中身が本当に変わっていない
    let row: (String, bool) =
        sqlx::query_as("SELECT raw, capturable FROM core.heartbeat WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, r#"{"alive":true}"#);
    assert!(row.1);
}

// ------------------------------------------------------------------ 表の形

/// 稼働記録系の全表に利用者識別子がある（FR-29 / 扉 #9。tasks 1.5）。
///
/// Scenario: すべての稼働記録系の表に利用者識別子がある
#[tokio::test]
async fn user_id_on_all_coverage_tables() {
    let pool = testdb::pool().await;
    for table in ["coverage", "heartbeat", "coverage_span", "source"] {
        let got: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM information_schema.columns
              WHERE table_schema = 'core' AND table_name = $1 AND column_name = 'user_id'",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(got.0, 1, "core.{table} に user_id が無い（FR-29 違反）");
    }
}

/// Must の 5 ソースが想定間隔つきで登録簿にある（FR-35 の改訂。tasks 1.6）。
/// **これが無いと途絶の判定も、利用が主語の 3 ソースの達成日も成立しない。**
#[tokio::test]
async fn expected_gap_seeded() {
    let pool = testdb::pool().await;
    for (name, gap) in [
        ("c01-location", 21_600),
        ("c01-app-usage", 21_600),
        ("c01-photo", 21_600),
        ("c02-window", 21_600),
        ("c02-browser-history", 86_400),
    ] {
        let got: Option<(i32,)> =
            sqlx::query_as("SELECT expected_gap_sec FROM core.source WHERE logical_source = $1")
                .bind(name)
                .fetch_optional(&pool)
                .await
                .unwrap();
        let got = got.unwrap_or_else(|| panic!("{name} が登録簿に無い"));
        assert_eq!(got.0, gap, "{name} の想定間隔");
    }
    // 定数と登録簿の名前がずれていないこと（ずれると達成日が 0 のまま緑になる）
    for (name, _) in coverage::must_sources() {
        let n: (i64,) =
            sqlx::query_as("SELECT count(*) FROM core.source WHERE logical_source = $1")
                .bind(&name)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(n.0, 1, "定数 {name} に対応する登録簿の行が無い");
    }
}

/// 稼働記録は重複を数えない（ST01 の design D13。ST02 は日の切り方だけを変えた）。
///
/// Scenario: 重複は件数に加えない
#[tokio::test]
async fn duplicate_records_are_not_counted() {
    let app = app().await;
    let s = testdb::source(&app.pool, "dup", 21_600).await;
    let u = testdb::user();
    let one = ev(&s, u, "2026-05-01T01:00:00Z", "Asia/Tokyo", r#"{"a":1}"#);
    post_ingest(&app, serde_json::json!([one.clone()])).await;
    post_ingest(&app, serde_json::json!([one])).await;

    let row: (i32,) = sqlx::query_as(
        "SELECT event_count FROM core.coverage WHERE user_id = $1 AND logical_source = $2",
    )
    .bind(u)
    .bind(&s)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(row.0, 1, "重複まで数えている");
}

/// `GET /coverage/achievement` が**期間を取らず**、5 本ぶんを返す（第 6 回 Q23。tasks 6.1）。
/// 窓を呼び出し側に委ねる余地を残すと、窓の取り方で合否が動く。
///
/// Scenario: 5 本の達成日数と合否が出る
#[tokio::test]
async fn achievement_endpoint() {
    let app = app().await;
    let Json(got) = achievement_get(
        State(app.clone()),
        auth(),
        Query(AchievementQuery { user_id: None }),
    )
    .await
    .expect("達成の読み出し口");
    assert_eq!(got.sources.len(), 5, "NFR-13 の 5 本が返る");
    assert!(
        got.sources
            .iter()
            .all(|s| s.denominator >= 0 && s.achieved_days <= s.denominator),
        "達成日数が分母を超えている"
    );
    // 合言葉が無ければ 401（PERM-10: すべての API 要求）
    let denied = achievement_get(
        State(app.clone()),
        HeaderMap::new(),
        Query(AchievementQuery { user_id: None }),
    )
    .await;
    assert!(matches!(denied, Err((StatusCode::UNAUTHORIZED, _))));
}

/// `GET /coverage` が 5 ソースぶんを、**NFR-13 の順**で返す（画面の縦の並びがこれになる）。
///
/// Scenario: ソースごとに格子が分かれる
#[tokio::test]
async fn coverage_endpoint_returns_five_sources() {
    let app = app().await;
    let Json(got) = coverage_get(
        State(app.clone()),
        auth(),
        Query(CoverageQuery {
            from: testdb::date("2026-03-01"),
            to: testdb::date("2026-03-07"),
            user_id: None,
        }),
    )
    .await
    .expect("稼働状況の読み出し口");
    let names: Vec<String> = got.iter().map(|s| s.logical_source.clone()).collect();
    assert_eq!(
        names,
        coverage::must_sources()
            .into_iter()
            .map(|(n, _)| n)
            .collect::<Vec<_>>()
    );
    assert!(got.iter().all(|s| s.days.len() == 7), "7 日ぶん返る");
    // 各格子にソース名の文字を添えられるだけの材料が返っている（第 4 回 Q15）
    assert!(got.iter().all(|s| !s.display_name.is_empty()));
}
