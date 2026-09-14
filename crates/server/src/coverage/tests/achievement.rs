// SPDX-License-Identifier: AGPL-3.0-only
//! 達成日数と合否（NFR-13）。**本物の PostgreSQL に対して**確かめる。
#![allow(clippy::unwrap_used)]

use super::super::*;
use super::*;
use crate::testdb;

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
