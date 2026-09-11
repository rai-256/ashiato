// SPDX-License-Identifier: AGPL-3.0-only
//! 稼働状況の導出と、成功条件 1 の達成日数を、**本物の PostgreSQL に対して**確かめる。
//!
//! 日境界（`AT TIME ZONE`）も範囲の重なりも DB の側にあるので、模擬すると意味が消える。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

/// テスト用の登録簿 1 行。`gap` は想定間隔（秒）、`started` は収集開始日。
async fn src(
    pool: &sqlx::PgPool,
    prefix: &str,
    gap: i32,
    started: Option<&str>,
) -> (SourceRow, uuid::Uuid) {
    let name = testdb::source(pool, prefix, gap).await;
    if let Some(d) = started {
        testdb::set_started_on(pool, &name, d).await;
    }
    (
        SourceRow {
            logical_source: name.clone(),
            named_source: name.clone(),
            display_name: name.clone(),
            expected_gap_sec: gap,
            collection_started_on: started.map(testdb::date),
            retired_on: None,
            chain: vec![name],
        },
        testdb::user(),
    )
}

/// ある 1 日の状態を引く。
async fn state_on(pool: &sqlx::PgPool, user: uuid::Uuid, s: &SourceRow, day: &str) -> DayState {
    let d = testdb::date(day);
    let got = of_source(pool, Some(user), s, d, d).await.unwrap();
    got.days[0].state
}

const SIX_HOURS: i32 = 21_600;
const SIXTY_DAYS: i32 = 60 * 86_400;

// ------------------------------------------------------------------ 7 状態

/// 7 状態それぞれが出る入力を 1 本で通す（tasks 5.1）。
///
/// Scenario: 丸ごと覆う停止も破棄も無い日に記録があれば記録あり
/// Scenario: 生存信号だけなら動いていた・記録なし
/// Scenario: 取得できない状態の信号なら固有の状態になる
/// Scenario: 収集開始日より前は導入前になる
/// Scenario: 想定間隔を超えて何も来ない日は途絶になる
/// Scenario: 記録が 0 件でも生存信号があれば稼働が残る
#[tokio::test]
async fn coverage_states() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "states", SIX_HOURS, Some("2026-05-01")).await;

    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-01", 3).await;
    testdb::put_heartbeat(&pool, u, &s.logical_source, "2026-05-02T03:00:00Z", true).await;
    testdb::put_heartbeat(&pool, u, &s.logical_source, "2026-05-03T03:00:00Z", false).await;
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "stopped",
        "2026-05-03T15:00:00Z", // = 05-04 00:00 JST
        Some("2026-05-04T15:00:00Z"),
    )
    .await;
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "dropped",
        "2026-05-04T15:00:00Z", // = 05-05 00:00 JST
        Some("2026-05-05T15:00:00Z"),
    )
    .await;

    let got = of_source(
        &pool,
        Some(u),
        &s,
        testdb::date("2026-04-30"),
        testdb::date("2026-05-06"),
    )
    .await
    .unwrap();
    let states: Vec<DayState> = got.days.iter().map(|d| d.state).collect();
    assert_eq!(
        states,
        vec![
            DayState::BeforeStart,        // 04-30 ⑦ 導入前
            DayState::Recorded,           // 05-01 ① 記録あり
            DayState::AliveNoRecord,      // 05-02 ② 動いていた・記録なし
            DayState::AliveNotCapturable, // 05-03 ③ 取れない状態だった
            DayState::Stopped,            // 05-04 ④ 意図的な停止
            DayState::Dropped,            // 05-05 ⑤ 破棄された期間
            DayState::Outage,             // 05-06 ⑥ 途絶
        ],
        "7 状態が出そろっていない: {states:?}"
    );
    // 記録が 0 件の日でも稼働が残る（FR-78 の目的そのもの）
    assert_eq!(got.days[2].event_count, 0);
}

/// 丸ごと覆う停止・破棄は記録より優先される（第 5 回 Q19。tasks 5.1b）。
///
/// Scenario: 丸ごと覆う停止は記録より優先される
/// Scenario: 破棄は停止より優先される
#[tokio::test]
async fn span_outranks_records() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "outrank", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-02", 5).await;
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "stopped",
        "2026-05-01T15:00:00Z",
        Some("2026-05-02T15:00:00Z"),
    )
    .await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Stopped,
        "記録があっても丸ごと覆う停止が先に立つ"
    );

    // 破棄が重なれば破棄
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "dropped",
        "2026-05-01T15:00:00Z",
        Some("2026-05-02T15:00:00Z"),
    )
    .await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Dropped
    );
}

/// 条件が重なっても状態は 1 つに決まり、2 回評価しても同じ（tasks 5.2）。
///
/// Scenario: 同じ入力から同じ状態が決まる
#[tokio::test]
async fn coverage_state_is_deterministic() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "determ", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-02", 2).await;
    testdb::put_heartbeat(&pool, u, &s.logical_source, "2026-05-02T03:00:00Z", true).await;
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "stopped",
        "2026-05-01T15:00:00Z",
        Some("2026-05-02T15:00:00Z"),
    )
    .await;
    let a = state_on(&pool, u, &s, "2026-05-02").await;
    let b = state_on(&pool, u, &s, "2026-05-02").await;
    assert_eq!(a, b);
    assert_eq!(a, DayState::Stopped);
}

/// 想定間隔を条件に持つ（`review/spec.md` の R4。tasks 5.3）。
///
/// Scenario: 想定間隔を超えない空白の日は途絶にならない
/// Scenario: 前後に何も無い空白の日は途絶になる
#[tokio::test]
async fn outage_respects_expected_gap() {
    let pool = testdb::pool().await;
    // 想定間隔 60 日。前後に記録がある 1 日の空白は途絶にならない
    let (s, u) = src(&pool, "gap60", SIXTY_DAYS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-01", 1).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-03", 1).await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::AliveNoRecord
    );
    // 最後の記録から 60 日を超えた日は途絶
    assert_eq!(
        state_on(&pool, u, &s, "2026-07-15").await,
        DayState::Outage,
        "05-03 から 73 日後は 60 日を超えている"
    );
    // 60 日以内は途絶にならない
    assert_eq!(
        state_on(&pool, u, &s, "2026-06-01").await,
        DayState::AliveNoRecord
    );
}

/// 前後を想定間隔以内に挟まれた空白の日は②（2 巡目 R6。tasks 5.3）。
///
/// **名前と中身を揃え直した**（review/code.md の R50）。以前ここは
/// 「6 時間間隔なら 1 日の空白は⑥」を確かめており、**名前と逆のことを主張していた**。
/// 挟まれて②になる側は名前どおりここが持ち、⑥の側は `outage_respects_expected_gap` が持つ。
///
/// Scenario: 想定間隔を超えない空白の日は途絶にならない
#[tokio::test]
async fn sandwiched_gap_is_alive() {
    let pool = testdb::pool().await;
    // 想定間隔 60 日。前後に記録があるので、間の空白は「収集は生きていた」と読める
    let (s, u) = src(&pool, "sandwich", SIXTY_DAYS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-01", 1).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-03", 1).await;
    for day in ["2026-05-02"] {
        assert_eq!(
            state_on(&pool, u, &s, day).await,
            DayState::AliveNoRecord,
            "{day} が②でない"
        );
    }
    // **生存信号は 1 件も無い。** ②の根拠は前後の記録だけ
    let hb: (i64,) =
        sqlx::query_as("SELECT count(*) FROM core.heartbeat WHERE logical_source = $1")
            .bind(&s.logical_source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(hb.0, 0);
}

/// 途絶は**受け手の側だけ**で立つ（FR-80）。収集側は何も送っていない。
///
/// Scenario: 途絶は収集側の報告なしに立つ
#[tokio::test]
async fn outage_stands_without_any_report() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "noreport", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-01", 1).await;
    // 想定間隔 6 時間なら、翌日以降は何も来ていない時点で⑥
    assert_eq!(state_on(&pool, u, &s, "2026-05-02").await, DayState::Outage);
    let hb: (i64,) =
        sqlx::query_as("SELECT count(*) FROM core.heartbeat WHERE logical_source = $1")
            .bind(&s.logical_source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(hb.0, 0, "収集側から何も送られていないのに立つのが FR-80");
}

/// 想定間隔を変えると過去の判定も変わる（design D6。tasks 5.4）。
/// **バッチで行に焼いていたら変わらない** —— 導出であることの検査になる。
///
/// Scenario: 想定間隔を変えると過去の日の判定も変わる
#[tokio::test]
async fn outage_reevaluates_on_gap_change() {
    let pool = testdb::pool().await;
    let (mut s, u) = src(&pool, "regap", SIXTY_DAYS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-01", 1).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-03", 1).await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::AliveNoRecord
    );

    sqlx::query("UPDATE core.source SET expected_gap_sec = $2 WHERE logical_source = $1")
        .bind(&s.logical_source)
        .bind(SIX_HOURS)
        .execute(&pool)
        .await
        .unwrap();
    s.expected_gap_sec = SIX_HOURS;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Outage,
        "想定間隔を縮めたら過去の同じ日が途絶に変わる"
    );
}

/// 導入前と、停止中が途絶にならないこと（tasks 5.5）。
///
/// Scenario: 停止中は途絶にならない
#[tokio::test]
async fn coverage_before_start() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "before", SIX_HOURS, Some("2026-04-01")).await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-03-20").await,
        DayState::BeforeStart
    );
}

/// 停止中の日は⑥にならない（tasks 5.5）。
#[tokio::test]
async fn coverage_stopped_not_outage() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "stopnotout", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "stopped",
        "2026-05-09T15:00:00Z",
        Some("2026-05-10T15:00:00Z"),
    )
    .await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-10").await,
        DayState::Stopped,
        "記録も生存信号も無いが、停止が先に立つ"
    );
}

/// 登録簿にあるだけで 1 件も届いていないソースは⑦のままで⑥にならない
/// （第 5 回 Q22。tasks 1.4c）。
///
/// Scenario: 1 件も届いていないソースは開始していない
#[tokio::test]
async fn never_started_is_not_outage() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "never", SIX_HOURS, None).await;
    let got = of_source(
        &pool,
        Some(u),
        &s,
        testdb::date("2026-05-01"),
        testdb::date("2026-05-10"),
    )
    .await
    .unwrap();
    assert!(
        got.days.iter().all(|d| d.state == DayState::BeforeStart),
        "1 件も届いていないソースに途絶が立っている: {:?}",
        got.days.iter().map(|d| d.state).collect::<Vec<_>>()
    );
}

/// 丸ごと覆わない停止は状態を決めない（design D7 / 深掘り Q3。tasks 5.6）。
///
/// Scenario: 1 日を丸ごと覆う停止だけが分母から抜ける
#[tokio::test]
async fn partial_stop_does_not_decide_state() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "partial", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-02", 1).await;
    // 09:00〜18:00 JST の半日だけ止める
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "stopped",
        "2026-05-02T00:00:00Z",
        Some("2026-05-02T09:00:00Z"),
    )
    .await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Recorded,
        "半日の停止は状態を決めない"
    );
}

/// 取得率は生存信号が運ぶ（第 5 回 Q17）。**信号の無い区間は埋まらない。**
///
/// Scenario: 想定間隔より細かい空きが取得率として残る
/// Scenario: 信号が来ない区間は取得率では埋まらない
#[tokio::test]
async fn attempt_ratio_is_carried_by_heartbeat() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "ratio", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_heartbeat_counts(
        &pool,
        u,
        &s.logical_source,
        "2026-05-01T03:00:00Z",
        true,
        360,
        230,
    )
    .await;
    let got = of_source(
        &pool,
        Some(u),
        &s,
        testdb::date("2026-05-01"),
        testdb::date("2026-05-02"),
    )
    .await
    .unwrap();
    assert_eq!(got.days[0].attempts, Some(360));
    assert_eq!(got.days[0].successes, Some(230));
    // 信号が 1 件も無い日は取得率を返さない（0 ではなく「無い」）
    assert_eq!(got.days[1].attempts, None);
    assert_eq!(got.days[1].successes, None);
}

/// 取得できない状態が理由とともに残る（FR-78）。
///
/// Scenario: 取得できない状態が理由とともに残る
#[tokio::test]
async fn uncapturable_keeps_its_reason() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "blocked", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_heartbeat(&pool, u, &s.logical_source, "2026-05-01T03:00:00Z", false).await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-01").await,
        DayState::AliveNotCapturable
    );
    let blockers: (Vec<String>,) =
        sqlx::query_as("SELECT blockers FROM core.heartbeat WHERE logical_source = $1")
            .bind(&s.logical_source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(blockers.0, vec!["permission".to_string()]);
}

// ------------------------------------------------------------------ 停止と破棄

/// 停止は時刻の範囲として残る（FR-34 / 深掘り Q3）。
///
/// Scenario: 半日の停止が時刻の範囲で残る
#[tokio::test]
async fn stop_is_stored_as_time_range() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "stopspan", SIX_HOURS, Some("2026-05-01")).await;
    // 2026-05-01 09:00〜18:00 JST = 00:00〜09:00 UTC
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "stopped",
        "2026-05-01T00:00:00Z",
        Some("2026-05-01T09:00:00Z"),
    )
    .await;
    let row: (
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT started_at, ended_at FROM core.coverage_span
          WHERE logical_source = $1 AND kind = 'stopped'",
    )
    .bind(&s.logical_source)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0.to_rfc3339(), "2026-05-01T00:00:00+00:00");
    assert_eq!(
        row.1.map(|t| t.to_rfc3339()),
        Some("2026-05-01T09:00:00+00:00".to_string())
    );
}

/// 破棄は期間と件数で残る（FR-9）。
///
/// Scenario: 破棄が期間と件数で残る
#[tokio::test]
async fn drop_is_stored_with_count() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "dropspan", SIX_HOURS, Some("2026-05-01")).await;
    sqlx::query(
        "INSERT INTO core.coverage_span
           (id, user_id, logical_source, kind, started_at, ended_at, event_count)
         VALUES ($1,$2,$3,'dropped','2026-05-01T00:00:00Z','2026-05-02T00:00:00Z',1234)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(u)
    .bind(&s.logical_source)
    .execute(&pool)
    .await
    .unwrap();
    let row: (
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<i32>,
    ) = sqlx::query_as(
        "SELECT started_at, ended_at, event_count FROM core.coverage_span
          WHERE logical_source = $1 AND kind = 'dropped'",
    )
    .bind(&s.logical_source)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.1.is_some());
    assert_eq!(row.2, Some(1234));
}

/// 逆順の範囲は DB が拒む（tasks 1.3）。**アプリ層だけに置かない** ——
/// psql を直に叩く経路（PERM-8）が素通りする。
#[tokio::test]
async fn span_rejects_reversed_range() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "revspan", SIX_HOURS, None).await;
    let bad = sqlx::query(
        "INSERT INTO core.coverage_span
           (id, user_id, logical_source, kind, started_at, ended_at)
         VALUES ($1,$2,$3,'stopped','2026-05-02T00:00:00Z','2026-05-01T00:00:00Z')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(u)
    .bind(&s.logical_source)
    .execute(&pool)
    .await;
    assert!(bad.is_err(), "終わりが始まりより前の範囲が入った");
}

// ------------------------------------------------------------------ 達成日数と合否

/// 5 ソースぶんの登録簿を用意し、それぞれ `days` 日ぶんの達成を置く。
/// 返るのは `(targets, user, 開始日)`。
async fn five_sources(
    pool: &sqlx::PgPool,
    prefix: &str,
    started: &str,
) -> (Vec<(String, Subject)>, uuid::Uuid) {
    let user = testdb::user();
    let mut targets = Vec::new();
    for (i, subject) in [
        Subject::Device,
        Subject::Device,
        Subject::Usage,
        Subject::Usage,
        Subject::Usage,
    ]
    .into_iter()
    .enumerate()
    {
        let name = testdb::source(pool, &format!("{prefix}{i}"), SIX_HOURS).await;
        testdb::set_started_on(pool, &name, started).await;
        targets.push((name, subject));
    }
    (targets, user)
}

/// 開始日から `n` 日ぶん、達成する材料を置く。
async fn fill_achieved(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    name: &str,
    subject: Subject,
    start: chrono::NaiveDate,
    n: i64,
) {
    for k in 0..n {
        let day = start + chrono::Duration::days(k);
        match subject {
            Subject::Device => {
                testdb::put_coverage(pool, user, name, &day.to_string(), 1).await;
            }
            Subject::Usage => {
                testdb::put_heartbeat(pool, user, name, &format!("{day}T03:00:00Z"), true).await;
            }
        }
    }
}

/// 端末が主語のソースは記録の有無で数える（tasks 6.2）。
///
/// Scenario: 端末が主語のソースは記録の有無で数える
#[tokio::test]
async fn achievement_device_subject() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "dev-subj", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-01-01").await;
    let u = testdb::user();
    let start = testdb::date("2026-01-01");
    fill_achieved(&pool, u, &name, Subject::Device, start, 7).await;

    let got = achievement(
        &pool,
        Some(u),
        testdb::date("2026-01-11"),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].achieved_days, 7);
    assert_eq!(got.sources[0].denominator, 10, "01-01〜01-10 の 10 日");
}

/// 利用が主語のソースは取得できる状態の生存信号で数える（tasks 6.3）。
///
/// Scenario: 利用が主語のソースは取得できる状態の信号で数える
#[tokio::test]
async fn achievement_usage_subject() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "use-subj", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-01-01").await;
    let u = testdb::user();
    fill_achieved(
        &pool,
        u,
        &name,
        Subject::Usage,
        testdb::date("2026-01-01"),
        5,
    )
    .await;
    let got = achievement(
        &pool,
        Some(u),
        testdb::date("2026-01-06"),
        &[(name.clone(), Subject::Usage)],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].achieved_days, 5);
    assert_eq!(got.sources[0].denominator, 5);
    // **記録が 1 件も無くても達成になる** —— 利用が主語のソースは信号で数える
    assert!(got.sources[0].met);
}

/// 取得できない状態しか無い日は達成に入らない（第 4 回 Q7。tasks 6.3）。
///
/// Scenario: 取れない状態しか無い日は達成にならない
#[tokio::test]
async fn achievement_excludes_uncapturable() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "uncap", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-01-01").await;
    let u = testdb::user();
    // 3 日とも「取れない状態」の信号だけ
    for k in 0..3 {
        let day = testdb::date("2026-01-01") + chrono::Duration::days(k);
        testdb::put_heartbeat(&pool, u, &name, &format!("{day}T03:00:00Z"), false).await;
    }
    let got = achievement(
        &pool,
        Some(u),
        testdb::date("2026-01-04"),
        &[(name.clone(), Subject::Usage)],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].denominator, 3);
    assert_eq!(
        got.sources[0].achieved_days, 0,
        "権限が剥がれたまま 1 年放置して 365/365 になる経路"
    );
    assert!(!got.sources[0].met);
}

/// 1 日を丸ごと覆う停止だけが分母から抜ける（深掘り Q3。tasks 6.4）。
#[tokio::test]
async fn achievement_denominator_full_day_stop_only() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "denom", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-01-01").await;
    let u = testdb::user();
    fill_achieved(
        &pool,
        u,
        &name,
        Subject::Device,
        testdb::date("2026-01-01"),
        10,
    )
    .await;
    // 01-02 を丸ごと（JST の 0 時から翌 0 時まで）止める
    testdb::put_span(
        &pool,
        u,
        &name,
        "stopped",
        "2026-01-01T15:00:00Z",
        Some("2026-01-02T15:00:00Z"),
    )
    .await;
    // 01-04 は半日だけ
    testdb::put_span(
        &pool,
        u,
        &name,
        "stopped",
        "2026-01-03T15:00:00Z",
        Some("2026-01-04T03:00:00Z"),
    )
    .await;
    // **丸ごと覆う「破棄」は分母に残る**（review/code.md の R7）。
    // specs は「FR-34 で記録された意図的な停止のうち 1 日を丸ごと覆うもの**だけ**を除く」。
    // ここを `!stopped_full && !dropped_full` に変えても 71 件緑だった（独立検証の実測）——
    // 破棄まで除くと分母が縮んで達成が近づき、第 5 回 Q18 が割合にした理由に直接触る。
    testdb::put_span(
        &pool,
        u,
        &name,
        "dropped",
        "2026-01-05T15:00:00Z",
        Some("2026-01-06T15:00:00Z"),
    )
    .await;
    let got = achievement(
        &pool,
        Some(u),
        testdb::date("2026-01-11"),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(
        got.sources[0].denominator, 9,
        "丸ごとの停止 1 日だけが抜ける（破棄と半日の停止は残る）"
    );
    assert_eq!(got.sources[0].achieved_days, 9);
}

/// 分母から抜けた日は達成日にも数えない（2 巡目 R4。tasks 6.4b）。
///
/// Scenario: 分母から抜けた日は達成日にも数えられない
#[tokio::test]
async fn achievement_numerator_never_exceeds() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "nexceed", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-01-01").await;
    let u = testdb::user();
    fill_achieved(
        &pool,
        u,
        &name,
        Subject::Device,
        testdb::date("2026-01-01"),
        5,
    )
    .await;
    // 記録がある 01-02 を丸ごと止める
    testdb::put_span(
        &pool,
        u,
        &name,
        "stopped",
        "2026-01-01T15:00:00Z",
        Some("2026-01-02T15:00:00Z"),
    )
    .await;
    let got = achievement(
        &pool,
        Some(u),
        testdb::date("2026-01-06"),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].denominator, 4);
    assert_eq!(got.sources[0].achieved_days, 4);
    assert!(got.sources[0].achieved_days <= got.sources[0].denominator);
}

/// 合否は 5 本すべてが分母の 95 % 以上（**固定値**で確かめる。tasks 6.5）。
///
/// Scenario: 1 ソースでも分母の 95 % に届かなければ未達
/// Scenario: 達成日数と分母の両方が返る
#[tokio::test]
async fn achievement_verdict_all_five() {
    let pool = testdb::pool().await;
    let start = "2026-01-01";
    let (targets, u) = five_sources(&pool, "five", start).await;
    let start_d = testdb::date(start);
    // 分母 365 で 360, 355, 352, 351, 340。線は 346.75 なので 340 だけが落ちる
    for ((name, subject), n) in targets.iter().zip([360, 355, 352, 351, 340]) {
        fill_achieved(&pool, u, name, *subject, start_d, n).await;
    }
    // 窓が閉じきった翌日に引く（開始日 + 365 日）
    let today = start_d + chrono::Duration::days(365);
    let got = achievement(&pool, Some(u), today, &targets).await.unwrap();

    for s in &got.sources {
        assert_eq!(s.denominator, 365, "{}", s.logical_source);
    }
    assert_eq!(
        got.sources
            .iter()
            .map(|s| s.achieved_days)
            .collect::<Vec<_>>(),
        vec![360, 355, 352, 351, 340]
    );
    assert!(
        (got.sources[0].threshold - 346.75).abs() < 1e-9,
        "線は 346.75"
    );
    assert!(!got.verdict, "1 本でも落ちれば未達");
    assert_eq!(
        got.failing,
        vec![targets[4].0.clone()],
        "落ちたソースが分かる"
    );
    assert!(got.confirmed, "5 本すべての窓が閉じている");
}

/// 分母が短いソースは線も下がる（第 5 回 Q18。tasks 6.5b）。
/// **絶対値の 350 では、導入 1 年未満が原理的に到達不能だった。**
///
/// Scenario: 分母が短いソースは線も下がる
#[tokio::test]
async fn achievement_threshold_scales() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "scale", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-01-01").await;
    let u = testdb::user();
    let start = testdb::date("2026-01-01");
    // 分母 200 日・達成 195 日 → 線は 190 日なので達成
    fill_achieved(&pool, u, &name, Subject::Device, start, 195).await;
    let got = achievement(
        &pool,
        Some(u),
        start + chrono::Duration::days(200),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].denominator, 200);
    assert_eq!(got.sources[0].achieved_days, 195);
    assert!((got.sources[0].threshold - 190.0).abs() < 1e-9);
    assert!(got.sources[0].met, "195 >= 190");
}

/// 導入前の日は分母に入らない（FR-79。tasks 6.6）。
///
/// Scenario: 導入前の日は分母に入らない
#[tokio::test]
async fn achievement_excludes_before_start() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "beforestart", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-03-01").await;
    let u = testdb::user();
    // 開始日より前の日にも稼働記録を置いてみる（分母に入ってはいけない）
    testdb::put_coverage(&pool, u, &name, "2026-02-01", 5).await;
    fill_achieved(
        &pool,
        u,
        &name,
        Subject::Device,
        testdb::date("2026-03-01"),
        3,
    )
    .await;
    let got = achievement(
        &pool,
        Some(u),
        testdb::date("2026-03-05"),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].denominator, 4, "03-01〜03-04 の 4 日だけ");
}

/// 窓はソースごとに別の日に始まる（第 6 回 Q23。tasks 6.7）。
///
/// Scenario: 窓はソースごとに別の日に始まる
#[tokio::test]
async fn achievement_window_per_source() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let loc = testdb::source(&pool, "win-loc", SIX_HOURS).await;
    let hist = testdb::source(&pool, "win-hist", SIX_HOURS).await;
    testdb::set_started_on(&pool, &loc, "2026-04-01").await;
    testdb::set_started_on(&pool, &hist, "2026-07-01").await;

    let got = achievement(
        &pool,
        Some(u),
        testdb::date("2026-06-30"),
        &[
            (loc.clone(), Subject::Device),
            (hist.clone(), Subject::Usage),
        ],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].denominator, 90, "04-01 から 06-29 まで");
    assert_eq!(got.sources[1].denominator, 0, "まだ始まっていない");
    assert_eq!(
        got.sources[0].window_closes_on,
        Some(testdb::date("2027-04-01"))
    );
}

/// 365 日が経つ前でも途中経過が出て、その合否は暫定（第 6 回 Q23 / 第 7 回 Q27。tasks 6.8）。
///
/// Scenario: 365 日が経つ前でも途中経過が出る
#[tokio::test]
async fn achievement_partial_window() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "partialwin", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-01-01").await;
    let u = testdb::user();
    let start = testdb::date("2026-01-01");
    fill_achieved(&pool, u, &name, Subject::Device, start, 200).await;
    let got = achievement(
        &pool,
        Some(u),
        start + chrono::Duration::days(200),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].denominator, 200);
    assert!(got.sources[0].met);
    assert!(!got.confirmed, "窓が閉じていないので暫定");
}

/// 確定と暫定を区別する（第 7 回 Q27。tasks 6.9）。
///
/// Scenario: 窓が閉じきるまで合否は暫定のまま
/// Scenario: 始まっていないソースがあると確定日が定まらない
/// Scenario: 全部が始まっていれば確定日が返る
/// Scenario: 全部の窓が閉じた日に確定する
#[tokio::test]
async fn achievement_provisional_until_all_windows_close() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let start = testdb::date("2026-01-01");

    // 4 本は閉じ、1 本は 300 日目 → 暫定 + 残り 65 日
    let mut targets = Vec::new();
    for i in 0..4 {
        let n = testdb::source(&pool, &format!("prov-old{i}"), SIX_HOURS).await;
        testdb::set_started_on(&pool, &n, "2025-01-01").await;
        targets.push((n, Subject::Device));
    }
    let young = testdb::source(&pool, "prov-young", SIX_HOURS).await;
    testdb::set_started_on(&pool, &young, &start.to_string()).await;
    targets.push((young.clone(), Subject::Device));

    let today = start + chrono::Duration::days(300);
    let got = achievement(&pool, Some(u), today, &targets).await.unwrap();
    assert!(!got.confirmed);
    assert_eq!(got.days_until_confirmed, Some(65), "365 - 300");
    assert_eq!(
        got.confirms_on,
        Some(start + chrono::Duration::days(365)),
        "いちばん遅く始まったソースが判定日を決める"
    );

    // 1 本がまだ開始していなければ、確定日も残り日数も返らない
    let never = testdb::source(&pool, "prov-never", SIX_HOURS).await;
    let mut with_never = targets.clone();
    with_never.push((never.clone(), Subject::Usage));
    let got = achievement(&pool, Some(u), today, &with_never)
        .await
        .unwrap();
    assert_eq!(got.confirms_on, None);
    assert_eq!(got.days_until_confirmed, None);
    assert_eq!(got.not_started, vec![never]);
    assert!(!got.confirmed);

    // 5 本すべての窓が閉じたら確定
    let closed = achievement(
        &pool,
        Some(u),
        start + chrono::Duration::days(365),
        &targets,
    )
    .await
    .unwrap();
    assert!(closed.confirmed);
    assert_eq!(closed.days_until_confirmed, Some(0));
}

/// 開始日が遡ると達成の判定も引き直される（第 7 回 Q26）。
/// **行に焼いていないので自然に満たす** —— その「自然に」を検査で固定する。
///
/// Scenario: 開始日が遡ると達成の判定も引き直される
#[tokio::test]
async fn achievement_reevaluates_when_start_moves_back() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "backdate", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-03-01").await;
    let u = testdb::user();
    let before = achievement(
        &pool,
        Some(u),
        testdb::date("2026-03-11"),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(before.sources[0].denominator, 10);

    testdb::set_started_on(&pool, &name, "2026-02-01").await;
    let after = achievement(
        &pool,
        Some(u),
        testdb::date("2026-03-11"),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(after.sources[0].denominator, 38, "02-01 から数え直される");
}

/// 主語の割り当ては**サーバ側の定数**（design D11）。DB の列にしない ——
/// 行を 1 つ更新するだけで判定式が動いてしまう。
#[test]
fn must_sources_are_the_five_of_nfr13() {
    // **(名前, 主語) の組を丸ごとリテラルで固定する**（review/code.md の R6）。
    // 個数（Device 2 本 / Usage 3 本）だけを見ていたときは、
    // **位置と写真を入れ替えても 71 件すべて緑**だった（独立検証の実測）。
    // 入れ替えは第 4 回 Q8（本人が `conflict / irreversible` として答えた問い）を裏返し、
    // specs が「写真を記録の有無で数えると**正常動作時から未達で固定される**」と
    // 書いた当の状態を作る。
    //
    // **並びもここで固定する**（design D19）—— 画面の縦の並びがこの順になる。
    assert_eq!(
        must_sources(),
        vec![
            ("c01-location".to_string(), Subject::Device),
            ("c01-app-usage".to_string(), Subject::Device),
            ("c01-photo".to_string(), Subject::Usage),
            ("c02-window".to_string(), Subject::Usage),
            ("c02-browser-history".to_string(), Subject::Usage),
        ]
    );
}

/// 格子のセルが担うのは 3 段だけ（design D10 / 第 5 回 Q21）。
/// **7 段にしない** —— 隣接 3:1 を 6 区間積むと 729:1 が要り、sRGB の 21:1 では成り立たない。
#[test]
fn seven_states_fold_into_three_bands() {
    assert_eq!(DayState::Recorded.band(), Band::Recorded);
    assert_eq!(DayState::AliveNoRecord.band(), Band::AliveNoRecord);
    for s in [
        DayState::AliveNotCapturable,
        DayState::Stopped,
        DayState::Dropped,
        DayState::Outage,
        DayState::BeforeStart,
    ] {
        assert_eq!(
            s.band(),
            Band::Other,
            "{s:?} が「それ以外」に畳まれていない"
        );
    }
}

// ------------------------------------------------------- 独立レビューで足したもの

/// 何が満たされていないかが**読み出し口から返る**（review/code.md の R39 / F1）。
///
/// spec の Scenario は 2 行あり、後半「**何が満たされていないか（権限）が返る**」が
/// 未実装だった。前の検査は `testdb` が入れた行を読み直していただけで、
/// **`blockers` を返さない実装のまま緑**だった。
///
/// Scenario: 取得できない状態が理由とともに残る
#[tokio::test]
async fn blockers_are_returned_from_coverage() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "blockers-out", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_heartbeat(&pool, u, &s.logical_source, "2026-05-01T03:00:00Z", false).await;

    let got = of_source(
        &pool,
        Some(u),
        &s,
        testdb::date("2026-05-01"),
        testdb::date("2026-05-02"),
    )
    .await
    .unwrap();
    assert_eq!(got.days[0].state, DayState::AliveNotCapturable);
    assert_eq!(
        got.days[0].blockers,
        vec!["permission".to_string()],
        "何が満たされていないかが返っていない"
    );
    // 取れている日・信号の無い日には理由が付かない
    assert!(got.days[1].blockers.is_empty());
}

/// **区間ごと**の取得率が返る（review/code.md の R9）。
///
/// spec は「**前回の信号からの間に** 360 回試みて 230 回成功 → **その区間の**回数が返る」。
/// 日の合計だけを返すと、想定間隔 6 時間のソースでは 1 日 4 区間が 1 つに混ざり、
/// **眠っていた区間が薄まって見えなくなる** —— 第 5 回 Q17 を入れた理由そのものが消える。
///
/// Scenario: 想定間隔より細かい空きが取得率として残る
#[tokio::test]
async fn intervals_are_returned_per_signal() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "intervals", SIX_HOURS, Some("2026-05-01")).await;
    // 同じ日に 2 区間。前半は眠っていて（10 / 3）、後半は健全（360 / 355）
    testdb::put_heartbeat_counts(
        &pool,
        u,
        &s.logical_source,
        "2026-05-01T03:00:00Z",
        true,
        10,
        3,
    )
    .await;
    testdb::put_heartbeat_counts(
        &pool,
        u,
        &s.logical_source,
        "2026-05-01T09:00:00Z",
        true,
        360,
        355,
    )
    .await;

    let d = testdb::date("2026-05-01");
    let got = of_source(&pool, Some(u), &s, d, d).await.unwrap();
    // 日の合計も返る（画面の「その日どれくらい取れたか」に使う）
    assert_eq!(got.days[0].attempts, Some(370));
    assert_eq!(got.days[0].successes, Some(358));
    // **区間が畳まれずに残る** —— 前半の 3/10 が後半に薄められていない
    let got_intervals: Vec<(i32, i32)> = got.days[0]
        .intervals
        .iter()
        .map(|i| (i.attempts, i.successes))
        .collect();
    assert_eq!(
        got_intervals,
        vec![(10, 3), (360, 355)],
        "区間が日に畳まれている（眠っていた区間が見分けられない）"
    );
}

/// 1 日に取得可否が**混在**したら②（design D7 の (5)）。
/// **`bool_and` に変えても緑だった**（独立レビューの実測。review/code.md の R32）。
///
/// Scenario: 生存信号だけなら動いていた・記録なし
#[tokio::test]
async fn mixed_capturable_day_is_alive() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "mixed", SIX_HOURS, Some("2026-05-01")).await;
    // 日の途中で権限が剥がれた（現実にいちばん起きる形）
    testdb::put_heartbeat(&pool, u, &s.logical_source, "2026-05-01T03:00:00Z", true).await;
    testdb::put_heartbeat(&pool, u, &s.logical_source, "2026-05-01T09:00:00Z", false).await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-01").await,
        DayState::AliveNoRecord,
        "1 件でも取得できる状態の信号があれば②（すべて取れないときだけ③）"
    );

    // 達成日にも数えられる（利用が主語のソース）
    let got = achievement(
        &pool,
        Some(u),
        testdb::date("2026-05-02"),
        &[(s.logical_source.clone(), Subject::Usage)],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].achieved_days, 1);
}

/// 丸ごと覆わない**破棄**も状態を決めない（review/code.md の R33 / I2）。
/// 停止しか試していなかったので、`dropped_full` を「一部でも重なれば真」に緩めても緑だった。
#[tokio::test]
async fn partial_drop_does_not_decide_state() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "partialdrop", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-02", 1).await;
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "dropped",
        "2026-05-02T00:00:00Z",
        Some("2026-05-02T09:00:00Z"),
    )
    .await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Recorded,
        "半日の破棄は状態を決めない"
    );
}

/// **まだ終わっていない停止**（`ended_at IS NULL`）が、その日以降を④にする。
/// FR-34 の最も普通の状態（いま止めていて、まだ再開していない）が一度も試されていなかった
/// （review/code.md の R43 / F10）。
#[tokio::test]
async fn open_ended_stop_covers_every_later_day() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "openstop", SIX_HOURS, Some("2026-05-01")).await;
    // 2026-05-02 00:00 JST から、終わりを入れずに止める
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "stopped",
        "2026-05-01T15:00:00Z",
        None,
    )
    .await;
    for day in ["2026-05-02", "2026-05-03", "2026-06-01"] {
        assert_eq!(
            state_on(&pool, u, &s, day).await,
            DayState::Stopped,
            "{day} が④でない"
        );
    }
    // 始まる前の日は掛からない
    assert_ne!(
        state_on(&pool, u, &s, "2026-05-01").await,
        DayState::Stopped
    );
}

/// 「丸ごと覆う」の**端**（review/code.md の R43 / F10）。
/// `<=` / `>=` を `<` / `>` に変えても落ちない入力しか無かった。
#[tokio::test]
async fn span_edges_decide_whether_a_day_is_covered() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "spanedge", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-02", 1).await;

    // ちょうど 05-02 00:00 JST 〜 05-03 00:00 JST（丸ごと覆う）
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "stopped",
        "2026-05-01T15:00:00Z",
        Some("2026-05-02T15:00:00Z"),
    )
    .await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Stopped
    );

    // 1 秒足りない範囲は「丸ごと」ではない
    let (s2, u2) = src(&pool, "spanedge2", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u2, &s2.logical_source, "2026-05-02", 1).await;
    testdb::put_span(
        &pool,
        u2,
        &s2.logical_source,
        "stopped",
        "2026-05-01T15:00:00Z",
        Some("2026-05-02T14:59:59Z"),
    )
    .await;
    assert_eq!(
        state_on(&pool, u2, &s2, "2026-05-02").await,
        DayState::Recorded,
        "1 秒足りない範囲が「丸ごと」と判定されている"
    );
}

/// 想定間隔の**ちょうど**の境界（review/code.md の R45 / F13）。
/// `diff <= gap_days` の等号がどちらに倒れるかは、ここでしか決まらない。
#[tokio::test]
async fn outage_boundary_is_inclusive() {
    let pool = testdb::pool().await;
    // 想定間隔 24 時間（ブラウザ履歴）。1 日ちょうどの空白は②、2 日は⑥
    const ONE_DAY: i32 = 86_400;
    let (s, u) = src(&pool, "gapedge", ONE_DAY, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-01", 1).await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::AliveNoRecord,
        "1 日ちょうどは「想定間隔を超えて」いない"
    );
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-03").await,
        DayState::Outage,
        "2 日は超えている"
    );

    // **想定間隔が 1 日未満のソースでは②の分岐に届かない**（design D17 の帰結）。
    // 6 時間 = 0.25 日なので、日の粒度で測る限り隣の日でも「超えて」いる。
    // 設定を 6 時間から 12 時間へ変えても判定が動かないのはこのため。
    let (s6, u6) = src(&pool, "gapsub", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u6, &s6.logical_source, "2026-05-01", 1).await;
    assert_eq!(
        state_on(&pool, u6, &s6, "2026-05-02").await,
        DayState::Outage
    );
}

/// 利用者で**実際に分かれる**（review/code.md の R46 / F15）。
/// `information_schema` を引くだけの検査は、列が使われているかを見ていない ——
/// `user_id = $1` の条件を全部消しても緑だった。
///
/// Scenario: すべての稼働記録系の表に利用者識別子がある
#[tokio::test]
async fn coverage_is_separated_by_user() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "twousers", SIX_HOURS).await;
    testdb::set_started_on(&pool, &name, "2026-05-01").await;
    let a = testdb::user();
    let b = testdb::user();
    testdb::put_coverage(&pool, a, &name, "2026-05-01", 3).await;
    testdb::put_heartbeat(&pool, b, &name, "2026-05-01T03:00:00Z", true).await;

    let s = SourceRow {
        logical_source: name.clone(),
        named_source: name.clone(),
        display_name: name.clone(),
        expected_gap_sec: SIX_HOURS,
        collection_started_on: Some(testdb::date("2026-05-01")),
        retired_on: None,
        chain: vec![name.clone()],
    };
    let d = testdb::date("2026-05-01");
    let for_a = of_source(&pool, Some(a), &s, d, d).await.unwrap();
    let for_b = of_source(&pool, Some(b), &s, d, d).await.unwrap();
    assert_eq!(
        for_a.days[0].state,
        DayState::Recorded,
        "a の記録が見えない"
    );
    assert_eq!(for_a.days[0].event_count, 3);
    assert_eq!(
        for_b.days[0].state,
        DayState::AliveNoRecord,
        "b に a の記録が見えている"
    );
    assert_eq!(for_b.days[0].event_count, 0);

    // **同じ日が 2 行に膨らまない**（review/code.md の R21）。絞らずに引いても 1 日 1 行
    let both = of_source(&pool, None, &s, d, d).await.unwrap();
    assert_eq!(both.days.len(), 1, "利用者ごとに日が複製されている");
    assert_eq!(both.days[0].event_count, 3);

    // 分母も日数で数えられる（行数ではない）
    let got = achievement(
        &pool,
        None,
        testdb::date("2026-05-02"),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(got.sources[0].denominator, 1);
}

/// `decide()` の優先順位を**表で全部**固定する（review/code.md の R49 / F18）。
/// DB 越しの検査は 3 対しか組み合わせておらず、(1)>(2)・(3)>(5)・(4)>(5) が未観測だった。
#[test]
fn decide_follows_the_order_of_d7() {
    fn facts(
        day: &str,
        event_count: i32,
        capturable: Option<bool>,
        dropped_full: bool,
        stopped_full: bool,
    ) -> DayFacts {
        DayFacts {
            day: testdb::date(day),
            event_count,
            capturable,
            attempts: None,
            successes: None,
            blockers: vec![],
            dropped_full,
            stopped_full,
        }
    }
    let start = Some(testdb::date("2026-05-01"));
    let gap = 0.25; // 6 時間
    let active: Vec<chrono::NaiveDate> = vec![];

    // (順, 入力, 期待する状態)
    let table: Vec<(&str, DayFacts, DayState)> = vec![
        // (1) 導入前は破棄より先
        (
            "1>2",
            facts("2026-04-30", 9, Some(true), true, true),
            DayState::BeforeStart,
        ),
        // (2) 破棄は停止より先
        (
            "2>3",
            facts("2026-05-02", 9, Some(true), true, true),
            DayState::Dropped,
        ),
        // (3) 停止は記録より先
        (
            "3>4",
            facts("2026-05-02", 9, Some(true), false, true),
            DayState::Stopped,
        ),
        // (3) 停止は取得可の信号より先
        (
            "3>5",
            facts("2026-05-02", 0, Some(true), false, true),
            DayState::Stopped,
        ),
        // (4) 記録は信号より先
        (
            "4>5",
            facts("2026-05-02", 1, Some(true), false, false),
            DayState::Recorded,
        ),
        (
            "4>6",
            facts("2026-05-02", 1, Some(false), false, false),
            DayState::Recorded,
        ),
        // (5) 取得可の信号は取得不可より先
        (
            "5>6",
            facts("2026-05-02", 0, Some(true), false, false),
            DayState::AliveNoRecord,
        ),
        // (6) 取得不可だけなら③
        (
            "6",
            facts("2026-05-02", 0, Some(false), false, false),
            DayState::AliveNotCapturable,
        ),
        // (7) 何も無ければ⑥（前後の活動が無い）
        (
            "7",
            facts("2026-05-02", 0, None, false, false),
            DayState::Outage,
        ),
    ];
    for (label, f, want) in table {
        assert_eq!(
            decide(&f, start, None, gap, &active),
            want,
            "順序 {label} が崩れている"
        );
    }

    // **⑧「退役」の位置**（design D33 / review/code-r2.md の G3）。⑦の直後なので、
    // 丸ごと覆う破棄・停止・記録・生存信号のどれが重なっていても⑧が勝つ。
    // この行が無かったときは、⑧の判定を破棄・停止の**下**へ動かしても全部緑だった。
    let retired = Some(testdb::date("2026-05-10"));
    let retired_table = [
        ("⑧>⑤", facts("2026-05-11", 0, None, true, false)),
        ("⑧>④", facts("2026-05-11", 0, None, false, true)),
        ("⑧>①", facts("2026-05-11", 3, None, false, false)),
        ("⑧>②", facts("2026-05-11", 0, Some(true), false, false)),
    ];
    for (label, f) in retired_table {
        assert_eq!(
            decide(&f, start, retired, gap, &active),
            DayState::Retired,
            "順序 {label} が崩れている（⑧は⑦の直後）"
        );
    }
    // **⑦は⑧より先**（収集開始日より前なら、退役していても「導入前」）
    assert_eq!(
        decide(
            &facts("2026-04-30", 0, None, false, false),
            start,
            retired,
            gap,
            &active
        ),
        DayState::BeforeStart,
        "⑦より⑧が先に当たっている"
    );

    // (8) 想定間隔以内に活動があれば②
    let near = vec![testdb::date("2026-05-03")];
    assert_eq!(
        decide(
            &facts("2026-05-02", 0, None, false, false),
            start,
            None,
            60.0,
            &near
        ),
        DayState::AliveNoRecord
    );
}

// ------------------------------------------------------------------ 第 8 回 Q29
//                                              端末の時計が狂った信号と収集開始日

/// 登録簿に行ができた日より前の時刻は、収集開始日の計算から外れる（tasks 12.2）。
///
/// **記録も信号も捨てない。** 外すのは収集開始日への寄与だけ —— 本人の答えは
/// 「受けるが、収集開始日の計算から外す」（第 8 回 Q29）。
///
/// Scenario: 登録より前の時刻の信号は開始日を動かさない
#[tokio::test]
async fn clock_skew_does_not_move_started_on() {
    let pool = testdb::pool().await;
    let name = testdb::source_registered_on(&pool, "skew", SIX_HOURS, "2026-04-01").await;
    let u = testdb::user();

    // 端末の時計が 27 年戻った信号。**受け取って保存する**
    testdb::put_heartbeat(&pool, u, &name, "1999-01-01T03:00:00Z", true).await;
    touch_started_on(&pool, &name, "1999-01-01T03:00:00Z".parse().unwrap())
        .await
        .unwrap();

    let got = sources(&pool, std::slice::from_ref(&name)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on, None,
        "登録より前の信号が収集開始日を動かした（1999 年に落ちると前にしか動かないので戻せない）"
    );

    // **信号そのものは残っている**（捨てていないことを、読み出し口から確かめる）
    let d = testdb::date("1999-01-01");
    let cov = of_source(&pool, Some(u), &got[0], d, d).await.unwrap();
    assert_eq!(
        cov.days[0].intervals.len(),
        1,
        "保存したはずの生存信号が読み出せない（外すのは開始日の計算だけ）"
    );

    // 登録より後の信号は、これまでどおり開始日を動かす
    testdb::put_heartbeat(&pool, u, &name, "2026-04-05T03:00:00Z", true).await;
    touch_started_on(&pool, &name, "2026-04-05T03:00:00Z".parse().unwrap())
        .await
        .unwrap();
    let got = sources(&pool, std::slice::from_ref(&name)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on,
        Some(testdb::date("2026-04-05")),
        "登録より後の信号まで外れている"
    );
}

/// 移行 0007 の**引き直しの節だけ**を抜き出す。
///
/// ファイル全体を当てると `ALTER TABLE core.source` と `CREATE INDEX ... ON core.event` が
/// 2 つの表に強い錠を掛け、並んで走っている記録の挿入と deadlock する（実測 40P01）。
/// **逐語は本物のまま**なので、閾値をファイルから消せばこの検査が落ちる。
fn repair_sql() -> &'static str {
    const SRC: &str = include_str!("../../../../migrations/0007_source_lifecycle.sql");
    let from = SRC.find("-- REPAIR-BEGIN").expect("印 REPAIR-BEGIN が無い");
    let to = SRC.find("-- REPAIR-END").expect("印 REPAIR-END が無い");
    &SRC[from..to]
}

/// 既に汚れている収集開始日を、移行 0007 が引き直す（tasks 12.1）。
///
/// **受け口に条件を足すだけでは足りない。** 収集開始日は前にしか動かないので、
/// 一度 1999 年に落ちた行は正しい日を送り直しても戻らない（第 8 回 Q29 の論点そのもの）。
///
/// Scenario: 汚れた収集開始日は引き直せる
#[tokio::test]
async fn migration_repairs_polluted_started_on() {
    let pool = testdb::pool().await;
    let u = testdb::user();

    // (a) 登録より後の記録があるソース —— 引き直すとその日に戻る
    let good = testdb::source_registered_on(&pool, "repair-a", SIX_HOURS, "2026-04-01").await;
    testdb::put_event(&pool, u, &good, "2026-04-07T12:00:00+09:00").await;
    testdb::set_started_on(&pool, &good, "1999-01-01").await;

    // (b) 登録より前の信号しか無いソース —— 引き直すと「まだ開始していない」に戻る
    let bad = testdb::source_registered_on(&pool, "repair-b", SIX_HOURS, "2026-04-01").await;
    testdb::put_heartbeat(&pool, u, &bad, "1999-01-01T03:00:00Z", true).await;
    testdb::set_started_on(&pool, &bad, "1999-01-01").await;

    // 移行を当て直す（`run()` は起動のたびに全部の版を当てる）。
    //
    // **トランザクションの中で当てて、読んでから巻き戻す。** 0007 は登録簿の全行を
    // 引き直すので、そのまま流すと**並んで走っている他のテストの収集開始日まで書き換える**
    // （実測: 達成日数の検査が分母 0 で落ちた）。1 つの DB を共有する足場なので、
    // 「全行に効く移行」を試すテストだけは自分の外へ出さない。
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(repair_sql()).execute(&mut *tx).await.unwrap();
    let got: Vec<(String, Option<chrono::NaiveDate>)> = sqlx::query_as(
        "SELECT logical_source, collection_started_on FROM core.source
          WHERE logical_source = ANY($1)",
    )
    .bind(vec![good.clone(), bad.clone()])
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();

    let by = |n: &str| got.iter().find(|r| r.0 == n).unwrap().1;
    assert_eq!(
        by(&good),
        Some(testdb::date("2026-04-07")),
        "登録以降のいちばん古い記録の日に引き直されていない"
    );
    assert_eq!(
        by(&bad),
        None,
        "登録より前の信号しか無いソースが「開始済み」のまま残っている"
    );
}

// ------------------------------------------------------------------ 第 8 回 Q31
//                                                        引き継ぎと退役

/// 引き継いだソースは、引き継ぎ元の収集開始日を継ぐ（tasks 15.3）。
///
/// Scenario: 引き継いだソースは引き継ぎ元の開始日を継ぐ
#[tokio::test]
async fn succession_inherits_started_on() {
    let pool = testdb::pool().await;
    let old = testdb::source(&pool, "succ-old", SIX_HOURS).await;
    let new = testdb::source(&pool, "succ-new", SIX_HOURS).await;
    testdb::set_started_on(&pool, &old, "2026-04-01").await;
    testdb::set_started_on(&pool, &new, "2026-09-01").await;
    testdb::set_succeeds(&pool, &new, &old).await;

    let got = sources(&pool, std::slice::from_ref(&new)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on,
        Some(testdb::date("2026-04-01")),
        "引き継ぎ元の開始日を継いでいない（名前を分けた日に窓が振り出しに戻る）"
    );

    // **窓の起点も一緒に動く** —— 窓は収集開始日から 365 日
    let ach = achievement(
        &pool,
        None,
        testdb::date("2026-09-10"),
        &[(new.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(
        ach.sources[0].window_closes_on,
        Some(testdb::date("2026-04-01") + chrono::Duration::days(WINDOW_DAYS)),
        "窓の起点が鎖の根になっていない"
    );
}

/// 退役した日より後は⑧、退役した日そのものは本来の状態（tasks 15.1 / 15.2）。
///
/// Scenario: 退役した日より後は退役になる
/// Scenario: 退役した日そのものは本来の状態のまま
#[tokio::test]
async fn retired_days() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "retired", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-10", 2).await;
    testdb::retire(&pool, &s.logical_source, "2026-05-10").await;

    let s = sources(&pool, std::slice::from_ref(&s.logical_source))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(s.retired_on, Some(testdb::date("2026-05-10")));

    // **退役した日そのものから⑧**（正典の逐語は「退役した日**以降**」——
    // `docs/requirements.md` の FR-54 / FR-80。当初 `>` で実装していたのを戻した）
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-10").await,
        DayState::Retired,
        "退役した日そのものが⑧になっていない（正典は「以降」）"
    );
    // **前日はまだ本来の状態**（境界が 1 日ずれていないこと）
    testdb::put_coverage(&pool, u, &s.logical_source, "2026-05-09", 1).await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-09").await,
        DayState::Recorded,
        "退役の前日まで⑧が食い込んでいる"
    );
    // **想定間隔を超えて何も来ていなくても⑥にならない**
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-11").await,
        DayState::Retired
    );
    assert_eq!(
        state_on(&pool, u, &s, "2026-06-20").await,
        DayState::Retired,
        "退役した後の空白日が「途絶」になっている（毎日壊れて見える）"
    );
}

/// 退役した日より後は分母にも達成日にも入らない（tasks 15.2）。
///
/// Scenario: 退役した日より後は分母に入らない
#[tokio::test]
async fn retired_days_out_of_denominator() {
    let pool = testdb::pool().await;
    let name = testdb::source(&pool, "retired-denom", SIX_HOURS).await;
    let u = testdb::user();
    testdb::set_started_on(&pool, &name, "2026-05-01").await;
    for d in ["2026-05-01", "2026-05-02", "2026-05-03"] {
        testdb::put_coverage(&pool, u, &name, d, 1).await;
    }
    testdb::retire(&pool, &name, "2026-05-03").await;

    // 今日は 2026-06-01。退役していなければ分母は 31 日（05-01〜05-31）
    let ach = achievement(
        &pool,
        Some(u),
        testdb::date("2026-06-01"),
        &[(name.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    // **退役した日「以降」を外す**（正典の FR-80）ので、分母は 05-01・05-02 の 2 日
    assert_eq!(
        ach.sources[0].denominator, 2,
        "退役した日以降が分母に残っている（名前を分けただけで未達が積まれる）"
    );
    assert_eq!(ach.sources[0].achieved_days, 2);
    assert!(
        ach.sources[0].met,
        "退役までは全日達成なのに未達になっている"
    );
}

/// Must の 5 本のうち退役したものは、引き継いだ後継で数える（tasks 15.4）。
///
/// Scenario: 退役した名前の代わりに後継を数える
#[tokio::test]
async fn must_source_resolves_to_successor() {
    let pool = testdb::pool().await;
    let old = testdb::source(&pool, "tip-old", SIX_HOURS).await;
    let new = testdb::source(&pool, "tip-new", SIX_HOURS).await;
    let u = testdb::user();
    testdb::set_started_on(&pool, &old, "2026-05-01").await;
    testdb::set_succeeds(&pool, &new, &old).await;
    testdb::retire(&pool, &old, "2026-05-02").await;
    testdb::put_coverage(&pool, u, &new, "2026-05-03", 1).await;

    // **定数が指しているのは古い名前**。達成は後継について数えられる
    let ach = achievement(
        &pool,
        Some(u),
        testdb::date("2026-05-04"),
        &[(old.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(
        ach.sources[0].logical_source, new,
        "退役した名前をそのまま数えている（永久に未達で固まる）"
    );
    assert_eq!(
        ach.sources[0].named_source, old,
        "定数が名指しした名前が残っていない"
    );
    assert!(
        !ach.failing.contains(&old) && !ach.not_started.contains(&old),
        "退役した名前が落ちたソース／未開始のソースとして出ている"
    );
    // 引き継ぎ元の収集開始日を継ぐので、分母は 05-01 から数える
    assert_eq!(
        ach.sources[0].collection_started_on,
        Some(testdb::date("2026-05-01"))
    );

    // **引き継ぎ先が無ければ、退役した名前のまま 1 本として残る**（黙って 4 本にしない）
    let alone = testdb::source(&pool, "tip-alone", SIX_HOURS).await;
    testdb::retire(&pool, &alone, "2026-05-02").await;
    let ach = achievement(
        &pool,
        Some(u),
        testdb::date("2026-05-04"),
        &[(alone.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(ach.sources.len(), 1);
    assert_eq!(ach.sources[0].logical_source, alone);
}

/// 「記録あり」は稼働記録の件数ではなく**記録そのもの**から引く（tasks 15.5 / ST03 の R57）。
///
/// ST03 が外部サービスからの更新経路を開けると、出来事の時刻が別の日へ動く。
/// 稼働記録の件数で決めていると、**記録の無い日が「記録あり」・記録のある日が「途絶」**になる。
///
/// Scenario: 記録の時刻が別の日へ動くと状態も動く
#[tokio::test]
async fn recorded_follows_event_time() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "from-event", SIX_HOURS, Some("2026-05-01")).await;

    // 稼働記録の行だけがある日（更新で記録が別の日へ移った後の形）
    sqlx::query(
        "INSERT INTO core.coverage (user_id, logical_source, day, event_count)
         VALUES ($1,$2,'2026-05-02'::date,5)",
    )
    .bind(u)
    .bind(&s.logical_source)
    .execute(&pool)
    .await
    .unwrap();
    // 記録そのものは別の日にある
    testdb::put_event(&pool, u, &s.logical_source, "2026-05-04T12:00:00+09:00").await;

    // **⑥ まで言い切る**（`assert_ne!` だと②〜⑧のどれでも通ってしまう）——
    // この日は記録も生存信号も無く、前後の活動も想定間隔（6 時間）の外
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Outage,
        "記録の無い日が「記録あり」になっている（稼働記録の件数で決めている）"
    );
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-04").await,
        DayState::Recorded,
        "記録のある日が「記録あり」になっていない"
    );

    // **件数も記録そのものから引く**（稼働記録の側には 5 を入れてある）
    let d = testdb::date("2026-05-02");
    let got = of_source(&pool, Some(u), &s, d, d).await.unwrap();
    assert_eq!(
        got.days[0].event_count, 0,
        "件数が稼働記録の行から来ている（記録は 1 件も無い日）"
    );
    let d = testdb::date("2026-05-04");
    let got = of_source(&pool, Some(u), &s, d, d).await.unwrap();
    assert_eq!(got.days[0].event_count, 1);
}

/// **論理削除された記録は稼働記録を書き換えない**（design D34 / FR-50）。
///
/// `core.event_live`（削除を除くビュー）から引くと、**過去の稼働状況が遡って⑥へ変わり**、
/// 成功条件 1 の達成日数が落ちる。稼働記録が答えるのは「その日に収集が動いていたか」で、
/// 後からの削除はその事実を書き換えない（丸ごと消した期間は⑤が担う）。
///
/// Scenario: 丸ごと覆う停止も破棄も無い日に記録があれば記録あり
#[tokio::test]
async fn deleted_events_still_count_as_recorded() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "deleted", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_deleted_event(&pool, u, &s.logical_source, "2026-05-02T12:00:00+09:00").await;

    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Recorded,
        "論理削除した記録の日が「記録あり」でなくなっている（core.event_live から引いている）"
    );
}

// ------------------------------- 第 2 巡レビュー（review/code-r2.md）で塞いだ穴

/// **引き継ぎ元の時代の達成日も数える**（review/code-r2.md の R1）。
///
/// 先端の名前だけで数えていたときは、**分母は鎖の根から数えるのに分子は切り替え後の
/// 記録しか拾わず**、名前を分けた翌日に成功条件 1 が 0 % へ落ちた ——
/// spec が「名前を分けただけで落ちる」のを防ぐと書いている当のもの。
///
/// Scenario: 退役した名前の代わりに後継を数える
#[tokio::test]
async fn chain_counts_the_predecessor_days() {
    let pool = testdb::pool().await;
    let old = testdb::source(&pool, "chain-old", SIX_HOURS).await;
    let new = testdb::source(&pool, "chain-new", SIX_HOURS).await;
    let u = testdb::user();
    testdb::set_started_on(&pool, &old, "2026-05-01").await;
    testdb::set_succeeds(&pool, &new, &old).await;

    // 旧名で 05-01〜05-03 の 3 日、切り替え後に新名で 05-05・05-06 の 2 日
    for d in ["2026-05-01", "2026-05-02", "2026-05-03"] {
        testdb::put_coverage(&pool, u, &old, d, 1).await;
    }
    testdb::retire(&pool, &old, "2026-05-04").await;
    for d in ["2026-05-05", "2026-05-06"] {
        testdb::put_coverage(&pool, u, &new, d, 1).await;
    }

    // 今日は 05-07。分母は 05-01〜05-06 の 6 日、達成は 5 日（05-04 だけ記録が無い）
    let ach = achievement(
        &pool,
        Some(u),
        testdb::date("2026-05-07"),
        &[(old.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(
        ach.sources[0].denominator, 6,
        "分母が鎖の全期間になっていない"
    );
    assert_eq!(
        ach.sources[0].achieved_days, 5,
        "引き継ぎ元の時代の達成日が消えている（先端の名前だけで数えている）"
    );

    // **格子の側も鎖で引く**（読み出し口が達成と別のものを見ていないこと）
    let rows = sources(&pool, std::slice::from_ref(&old)).await.unwrap();
    let cov = of_source(
        &pool,
        Some(u),
        &rows[0],
        testdb::date("2026-05-01"),
        testdb::date("2026-05-06"),
    )
    .await
    .unwrap();
    assert_eq!(cov.logical_source, new, "格子が先端を指していない");
    assert_eq!(cov.named_source, old, "名指しされた名前が返っていない");
    assert_eq!(
        cov.days[0].state,
        DayState::Recorded,
        "引き継ぎ元の時代の記録が格子から消えている"
    );
}

/// **退役していない名前からは乗り換えない**（同 R2）。
///
/// 後継の行を先に作ってから退役させる運用（登録簿としてごく自然な順）で、
/// まだ動いているソースの達成日が 0 になっていた。
#[tokio::test]
async fn a_live_source_is_not_swapped_for_its_successor() {
    let pool = testdb::pool().await;
    let old = testdb::source(&pool, "live-old", SIX_HOURS).await;
    let new = testdb::source(&pool, "live-new", SIX_HOURS).await;
    let u = testdb::user();
    testdb::set_started_on(&pool, &old, "2026-05-01").await;
    // 後継の行だけ先に作る。**退役はまだしていない**
    testdb::set_succeeds(&pool, &new, &old).await;
    testdb::put_coverage(&pool, u, &old, "2026-05-01", 1).await;

    let ach = achievement(
        &pool,
        Some(u),
        testdb::date("2026-05-02"),
        &[(old.clone(), Subject::Device)],
    )
    .await
    .unwrap();
    assert_eq!(
        ach.sources[0].logical_source, old,
        "まだ退役していないソースから後継へ乗り換えている"
    );
    assert_eq!(ach.sources[0].achieved_days, 1);
    assert!(ach.sources[0].met);
}

/// 収集開始日は**鎖全体でいちばん古い日**（同 G8）。後継が先に始まっていても継ぐ。
#[tokio::test]
async fn chain_started_on_is_the_oldest_in_the_chain() {
    let pool = testdb::pool().await;
    let root = testdb::source(&pool, "old-root", SIX_HOURS).await;
    let mid = testdb::source(&pool, "old-mid", SIX_HOURS).await;
    let tip = testdb::source(&pool, "old-tip", SIX_HOURS).await;
    testdb::set_started_on(&pool, &root, "2026-09-01").await;
    // **後継のほうが古い**（重なって走っていた運用）
    testdb::set_started_on(&pool, &mid, "2026-04-01").await;
    testdb::set_succeeds(&pool, &mid, &root).await;
    testdb::set_succeeds(&pool, &tip, &mid).await;
    testdb::retire(&pool, &root, "2026-09-02").await;
    testdb::retire(&pool, &mid, "2026-09-03").await;

    // 3 本の鎖をたどり切って、いちばん古い日を継ぐ（`tip` 自身は NULL のまま）
    let rows = sources(&pool, std::slice::from_ref(&root)).await.unwrap();
    assert_eq!(
        rows[0].logical_source, tip,
        "3 本の鎖の先端まで届いていない"
    );
    assert_eq!(
        rows[0].collection_started_on,
        Some(testdb::date("2026-04-01")),
        "鎖全体でいちばん古い日を継いでいない"
    );
    assert_eq!(rows[0].chain.len(), 3, "鎖の名前が 3 本そろっていない");
}

/// 引き継ぎの鎖が**輪**になっても止まる（同 R12 / G10）。
///
/// `A → B → A` は FK も CHECK も素通りする（どちらの行を入れた時点でも輪は閉じていない）。
#[tokio::test]
async fn a_cycle_in_the_chain_terminates() {
    let pool = testdb::pool().await;
    let a = testdb::source(&pool, "cyc-a", SIX_HOURS).await;
    let b = testdb::source(&pool, "cyc-b", SIX_HOURS).await;
    testdb::set_started_on(&pool, &a, "2026-05-01").await;
    testdb::set_succeeds(&pool, &b, &a).await;
    testdb::set_succeeds(&pool, &a, &b).await;
    testdb::retire(&pool, &a, "2026-05-02").await;
    testdb::retire(&pool, &b, "2026-05-03").await;

    // 回り続けない。名前は重複せず、深さの上限も超えない
    let rows = sources(&pool, std::slice::from_ref(&a)).await.unwrap();
    assert!(rows[0].chain.len() <= 2, "輪を回って鎖が伸びている");
    let mut uniq = rows[0].chain.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(
        uniq.len(),
        rows[0].chain.len(),
        "鎖に同じ名前が 2 度入っている"
    );
}

/// **登録日ちょうど**の記録は収集開始日を作る（同 R7 / G2）。
///
/// 閾値が `>=` から `>` に倒れると、**登録したその日に初回の記録が来るソース
/// （＝通常の導入手順そのもの）が永久に「未開始」で固まる** —— Q29 が直そうとした型と同じ。
///
/// Scenario: 登録より前の時刻の信号は開始日を動かさない
#[tokio::test]
async fn a_record_on_the_registration_day_starts_collection() {
    let pool = testdb::pool().await;
    let name = testdb::source_registered_on(&pool, "edge", SIX_HOURS, "2026-04-01").await;

    // 登録日の JST 0 時ちょうど（その日でいちばん早い時刻）
    touch_started_on(&pool, &name, "2026-03-31T15:00:00Z".parse().unwrap())
        .await
        .unwrap();
    let got = sources(&pool, std::slice::from_ref(&name)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on,
        Some(testdb::date("2026-04-01")),
        "登録日ちょうどの記録が弾かれている（閾値が > に倒れている）"
    );

    // その 1 秒前（前日の 23:59:59 JST）は弾かれる
    let other = testdb::source_registered_on(&pool, "edge2", SIX_HOURS, "2026-04-01").await;
    touch_started_on(&pool, &other, "2026-03-31T14:59:59Z".parse().unwrap())
        .await
        .unwrap();
    let got = sources(&pool, std::slice::from_ref(&other)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on, None,
        "登録日の前日が通っている"
    );
}

/// 引き直しは**何度当てても同じ**で、**正規の収集開始日を後ろへ動かさない**（同 C-3 / G4 / M-3）。
///
/// `migrate()` は版管理表を持たず**起動のたびに当て直す**ので、条件を付けないと
/// 「記録を破棄した後に再起動する」だけで⑤「破棄された期間」が⑦「導入前」に化ける。
///
/// Scenario: 汚れた収集開始日は引き直せる
#[tokio::test]
async fn repair_is_idempotent_and_never_moves_a_clean_date_forward() {
    let pool = testdb::pool().await;
    let name = testdb::source_registered_on(&pool, "idem", SIX_HOURS, "2026-04-01").await;
    let u = testdb::user();
    // 正規の収集開始日。**記録はもう残っていない**（破棄・保持期間の刈り取りの後の形）
    testdb::set_started_on(&pool, &name, "2026-04-05").await;
    testdb::put_event(&pool, u, &name, "2026-04-20T12:00:00+09:00").await;

    const READ: &str = "SELECT collection_started_on FROM core.source WHERE logical_source = $1";
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(repair_sql()).execute(&mut *tx).await.unwrap();
    let once: (Option<chrono::NaiveDate>,) = sqlx::query_as(READ)
        .bind(&name)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    sqlx::raw_sql(repair_sql()).execute(&mut *tx).await.unwrap();
    let twice: (Option<chrono::NaiveDate>,) = sqlx::query_as(READ)
        .bind(&name)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let (once, twice) = (once.0, twice.0);

    assert_eq!(
        once,
        Some(testdb::date("2026-04-05")),
        "汚れていない収集開始日が後ろへ動いた（窓が縮み、破棄した期間が⑦に化ける）"
    );
    assert_eq!(twice, once, "2 度当てると結果が変わる");
}
