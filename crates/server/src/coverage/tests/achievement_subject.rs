// SPDX-License-Identifier: AGPL-3.0-only
//! 達成日の主語と PC の分母（ST14 / design D17）。**本物の PostgreSQL に対して**確かめる。
//!
//! 分母の絞りは定数が名指しする名前（`c02-window` など）で決まるので、登録簿の行を
//! 手で組んで `achieve_one` に渡す（共有の登録簿の開始日を書き換えない）。利用者は試験ごとに固有。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

fn row(name: &str, started: &str) -> SourceRow {
    SourceRow {
        logical_source: name.to_string(),
        named_source: name.to_string(),
        display_name: name.to_string(),
        expected_gap_sec: SIX_HOURS,
        collection_started_on: Some(testdb::date(started)),
        retired_on: None,
        chain: vec![name.to_string()],
    }
}

/// 開始日 2026-01-01 から 10 日のうち、生存信号が届いたのは先頭の 7 日（すべて取得できる状態）。
async fn seven_alive_days(name: &str, subject: Subject) -> SourceAchievement {
    let pool = testdb::pool().await;
    let u = testdb::user();
    for k in 0..7 {
        let day = testdb::date("2026-01-01") + chrono::Duration::days(k);
        testdb::put_heartbeat(&pool, u, name, &format!("{day}T03:00:00Z"), true).await;
    }
    achieve_one(
        &pool,
        Some(u),
        testdb::date("2026-01-11"),
        name,
        subject,
        &row(name, "2026-01-01"),
    )
    .await
    .unwrap()
}

/// 主語と並びと PC の分母の定数を名指しで固定する（docs/testing.md §3）。
#[test]
fn st14_ach_constants_are_pinned() {
    assert_eq!(DEVICE_SUBJECT, ["c01-location"]);
    assert_eq!(
        USAGE_SUBJECT,
        [
            "c01-app-usage",
            "c01-photo",
            "c02-window",
            "c02-browser-history"
        ]
    );
    assert_eq!(PC_SOURCES, ["c02-window", "c02-browser-history"]);
    // 並びは主語の定数とは別。アプリ利用が利用の主語になっても 2 番目のまま
    assert_eq!(
        must_sources(),
        vec![
            ("c01-location".to_string(), Subject::Device),
            ("c01-app-usage".to_string(), Subject::Usage),
            ("c01-photo".to_string(), Subject::Usage),
            ("c02-window".to_string(), Subject::Usage),
            ("c02-browser-history".to_string(), Subject::Usage),
        ]
    );
}

/// アプリ利用を、与えた日々の入力で 2026-01-01〜02 の 2 日を窓に数える（利用者は固有）。
async fn app_usage_two_days(
    day1_capturable_signal: bool,
    day2_event_with_uncapturable_signal: bool,
) -> SourceAchievement {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let name = "c01-app-usage";
    if day1_capturable_signal {
        testdb::put_heartbeat(&pool, u, name, "2026-01-01T03:00:00Z", true).await;
    }
    if day2_event_with_uncapturable_signal {
        testdb::put_coverage(&pool, u, name, "2026-01-02", 1).await;
        testdb::put_heartbeat(&pool, u, name, "2026-01-02T03:00:00Z", false).await;
    }
    let subject = must_sources()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
        .1;
    achieve_one(
        &pool,
        Some(u),
        testdb::date("2026-01-03"),
        name,
        subject,
        &row(name, "2026-01-01"),
    )
    .await
    .unwrap()
}

/// Scenario: アプリ利用は取得できる状態の信号で数える
#[tokio::test]
async fn st14_ach_app_usage_counts_capturable_signal() {
    // 両方ある日々: 前者だけが達成日。後者は数えない
    let both = app_usage_two_days(true, true).await;
    assert_eq!(both.achieved_days, 1);
    assert_eq!(both.denominator, 2);
    // 主張ごとに別々に: 取得できる状態の信号だけの日は達成日に数える
    let capturable_only = app_usage_two_days(true, false).await;
    assert_eq!(
        capturable_only.achieved_days, 1,
        "取得できる状態の信号は数える"
    );
    // イベントがあっても取得できない状態の信号のみの日は数えない（旧い主語 event_count>0 なら 1 になる）
    let uncapturable_only = app_usage_two_days(false, true).await;
    assert_eq!(
        uncapturable_only.achieved_days, 0,
        "イベントがあっても取得できない状態の信号のみの日は数えない"
    );
}

/// Scenario: PC のソースは生存信号の届いた日だけを分母に数える
#[tokio::test]
async fn st14_ach_pc_denominator_counts_only_signal_days() {
    for name in PC_SOURCES {
        let got = seven_alive_days(name, Subject::Usage).await;
        assert_eq!(got.denominator, 7, "{name}: 信号の届いた 7 日だけが分母");
        assert_eq!(got.achieved_days, 7, "{name}");
    }
}

/// 取得できない状態の信号も「届いた」に数える（D17）。分母には入り、達成日には入らない。
#[tokio::test]
async fn st14_ach_pc_uncapturable_signal_is_in_denominator() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let name = "c02-window";
    testdb::put_heartbeat(&pool, u, name, "2026-01-01T03:00:00Z", true).await;
    testdb::put_heartbeat(&pool, u, name, "2026-01-02T03:00:00Z", false).await;
    let got = achieve_one(
        &pool,
        Some(u),
        testdb::date("2026-01-05"),
        name,
        Subject::Usage,
        &row(name, "2026-01-01"),
    )
    .await
    .unwrap();
    assert_eq!(got.denominator, 2);
    assert_eq!(got.achieved_days, 1);
}

/// Scenario: 携帯のソースは生存信号が届かない日も分母に数える
#[tokio::test]
async fn st14_ach_phone_denominator_counts_silent_days() {
    let got = seven_alive_days("c01-app-usage", Subject::Usage).await;
    assert_eq!(got.denominator, 10, "届かなかった 3 日も分母");
    assert_eq!(got.achieved_days, 7);
}
