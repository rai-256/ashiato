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
            display_name: name,
            expected_gap_sec: gap,
            collection_started_on: started.map(testdb::date),
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
        display_name: name.clone(),
        expected_gap_sec: SIX_HOURS,
        collection_started_on: Some(testdb::date("2026-05-01")),
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
            decide(&f, start, gap, &active),
            want,
            "順序 {label} が崩れている"
        );
    }

    // (8) 想定間隔以内に活動があれば②
    let near = vec![testdb::date("2026-05-03")];
    assert_eq!(
        decide(
            &facts("2026-05-02", 0, None, false, false),
            start,
            60.0,
            &near
        ),
        DayState::AliveNoRecord
    );
}
