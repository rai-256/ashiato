// SPDX-License-Identifier: AGPL-3.0-only
//! 端末からの破棄の報告を、稼働状況がどう読むか（ST04 / design D8 / D9）。**本物の PostgreSQL に対して**確かめる。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

/// ある 1 日のセルを引く。
async fn cell_on(pool: &sqlx::PgPool, user: uuid::Uuid, s: &SourceRow, day: &str) -> DayCell {
    let d = testdb::date(day);
    let mut got = of_source(pool, Some(user), s, d, d).await.unwrap();
    got.days.remove(0)
}

fn range(from: &str, to: &str, count: i64) -> DroppedRange {
    DroppedRange {
        from: from.into(),
        to: to.into(),
        count,
    }
}

/// 2 本に割れた報告（1 本目を送った後も押し出しが続いた）を、つないでから丸ごと判定する。
/// 丸ごと覆う破棄は記録より先に見る（ST02 の判定順）ので、記録があっても⑤。
///
/// Scenario: 端が接する 2 本の破棄は合わせて丸ごと判定される
#[tokio::test]
async fn dropped_ranges_merge_touching_reports_cover_the_day() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "mergetouch", SIX_HOURS, Some("2026-05-01")).await;
    // 2026-05-01 18:00 JST 〜 05-02 09:00 JST ／ 05-02 09:00 JST 〜 05-03 06:00 JST
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-01T09:00:00Z", "2026-05-02T00:00:00Z")),
        &[("2026-05-01T09:00:00Z", 1)],
    )
    .await;
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-02T00:00:00Z", "2026-05-02T21:00:00Z")),
        &[("2026-05-02T00:00:00Z", 1)],
    )
    .await;
    testdb::put_event(&pool, u, &s.logical_source, "2026-05-02T03:00:00Z").await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Dropped,
        "端が接する 2 本をつながずに 1 本ずつ見ている"
    );
    // 片方しか覆わない端の日は⑤にならない
    assert_ne!(
        state_on(&pool, u, &s, "2026-05-01").await,
        DayState::Dropped
    );
}

/// Scenario: 離れた 2 本の破棄は合わせない
#[tokio::test]
async fn dropped_ranges_merge_keeps_separate_reports_apart() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "mergegap", SIX_HOURS, Some("2026-05-01")).await;
    // 2 本目は 09:01 JST から —— 1 分の隙間
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-01T09:00:00Z", "2026-05-02T00:00:00Z")),
        &[("2026-05-01T09:00:00Z", 1)],
    )
    .await;
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-02T00:01:00Z", "2026-05-02T21:00:00Z")),
        &[("2026-05-02T00:00:00Z", 1)],
    )
    .await;
    testdb::put_event(&pool, u, &s.logical_source, "2026-05-02T00:00:30Z").await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Recorded,
        "離れた 2 本を近さで合わせている"
    );
}

/// 重なる 2 本・ST02 の `coverage_span` の破棄と端末の報告の組み合わせもつなぐ（design D8 の材料は 2 表）。
/// 3 本が 1 つの島になる途中で、2 本目が 1 本目の中に収まる（終わりの最大を持ち回っていないと途切れる）。
#[tokio::test]
async fn dropped_ranges_merge_overlapping_and_span_rows() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "mergemix", SIX_HOURS, Some("2026-05-01")).await;
    // JST 05-02 は UTC 05-01T15:00 〜 05-02T15:00
    testdb::put_span(
        &pool,
        u,
        &s.logical_source,
        "dropped",
        "2026-05-01T12:00:00Z",
        Some("2026-05-02T06:00:00Z"),
    )
    .await;
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-01T13:00:00Z", "2026-05-01T14:00:00Z")),
        &[("2026-05-01T13:00:00Z", 1)],
    )
    .await;
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-02T06:00:00Z", "2026-05-02T16:00:00Z")),
        &[("2026-05-02T06:00:00Z", 1)],
    )
    .await;
    testdb::put_event(&pool, u, &s.logical_source, "2026-05-02T03:00:00Z").await;
    assert_eq!(
        state_on(&pool, u, &s, "2026-05-02").await,
        DayState::Dropped
    );
}

/// 一部を破棄した日: 状態は記録ありのまま、件数と区間が載る。
///
/// 応答の側（印そのものは web の `drop-mark.test.tsx`）: Scenario「一部を破棄した日のセルに形の印が付く」の材料
#[tokio::test]
async fn day_cell_dropped_partial_day_has_count_and_range() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "cellpart", SIX_HOURS, Some("2026-05-01")).await;
    // 10:00〜13:00 JST = 01:00〜04:00 UTC に 60 件ずつ
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-01T01:00:00Z", "2026-05-01T04:00:00Z")),
        &[
            ("2026-05-01T01:00:00Z", 60),
            ("2026-05-01T02:00:00Z", 60),
            ("2026-05-01T03:00:00Z", 60),
        ],
    )
    .await;
    testdb::put_event(&pool, u, &s.logical_source, "2026-05-01T06:00:00Z").await;
    let c = cell_on(&pool, u, &s, "2026-05-01").await;
    assert_eq!(c.state, DayState::Recorded);
    assert_eq!(c.dropped_count, 180);
    assert_eq!(c.dropped_ranges, vec![range("10:00", "13:00", 180)]);
    // 応答の欄名（web が読む）
    let json = serde_json::to_value(&c).unwrap();
    assert_eq!(json["dropped_count"], 180);
    assert_eq!(json["dropped_ranges"][0]["from"], "10:00");
    assert_eq!(json["dropped_ranges"][0]["to"], "13:00");
    assert_eq!(json["dropped_ranges"][0]["count"], 180);
    // 破棄の無い日は 0 と空
    let none = cell_on(&pool, u, &s, "2026-05-02").await;
    assert_eq!(none.dropped_count, 0);
    assert!(none.dropped_ranges.is_empty());
}

/// 丸ごと覆う日は⑤で、件数と 00:00〜24:00 が載る。
///
/// 応答の側（文字は web の `drop-detail.test.tsx`）: Scenario「丸ごと覆う破棄の日は件数が添えられる」の材料
#[tokio::test]
async fn day_cell_dropped_full_day_has_state_and_count() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "cellfull", SIX_HOURS, Some("2026-05-01")).await;
    let hours: Vec<String> = (0..24)
        .map(|h| {
            (chrono::DateTime::parse_from_rfc3339("2026-05-01T15:00:00Z").unwrap()
                + chrono::Duration::hours(h))
            .to_utc()
            .to_rfc3339()
        })
        .collect();
    let hr: Vec<(&str, i32)> = hours.iter().map(|h| (h.as_str(), 60)).collect();
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-01T15:00:00Z", "2026-05-02T15:00:00Z")),
        &hr,
    )
    .await;
    let c = cell_on(&pool, u, &s, "2026-05-02").await;
    assert_eq!(c.state, DayState::Dropped);
    assert_eq!(c.dropped_count, 1440);
    assert_eq!(c.dropped_ranges, vec![range("00:00", "24:00", 1440)]);
}

/// Scenario: 日をまたぐ破棄は日ごとに切られる
#[tokio::test]
async fn day_cell_dropped_is_cut_per_day() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "cellcross", SIX_HOURS, Some("2026-05-01")).await;
    // 2026-05-01 22:00 JST 〜 05-02 03:00 JST = 13:00Z 〜 18:00Z
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-01T13:00:00Z", "2026-05-01T18:00:00Z")),
        &[
            ("2026-05-01T13:00:00Z", 60),
            ("2026-05-01T14:00:00Z", 60),
            ("2026-05-01T15:00:00Z", 60),
            ("2026-05-01T16:00:00Z", 60),
            ("2026-05-01T17:00:00Z", 60),
        ],
    )
    .await;
    let d1 = testdb::date("2026-05-01");
    let d2 = testdb::date("2026-05-02");
    let got = of_source(&pool, Some(u), &s, d1, d2).await.unwrap();
    assert_eq!(got.days[0].dropped_count, 120);
    assert_eq!(
        got.days[0].dropped_ranges,
        vec![range("22:00", "24:00", 120)]
    );
    assert_eq!(got.days[1].dropped_count, 180);
    assert_eq!(
        got.days[1].dropped_ranges,
        vec![range("00:00", "03:00", 180)]
    );
}

/// Scenario: 範囲を持たない破棄は日の件数に入らない
#[tokio::test]
async fn day_cell_dropped_ignores_rangeless_reports() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "cellunread", SIX_HOURS, Some("2026-05-01")).await;
    testdb::put_drop(&pool, u, &s.logical_source, None, &[]).await;
    let got = of_source(
        &pool,
        Some(u),
        &s,
        testdb::date("2026-05-01"),
        testdb::date("2026-05-07"),
    )
    .await
    .unwrap();
    assert!(got
        .days
        .iter()
        .all(|d| d.dropped_count == 0 && d.dropped_ranges.is_empty()));
}

/// Scenario: 同じ時間に重なる 2 つの区間では前の区間にだけ数える
///
/// 範囲の終わりが分の途中（1 件だけの破棄は出来事の時刻 + 1 ms）なら、`to` は分に切り上げる ——
/// `10:00〜10:00` のような空に見える区間を出さない。時間が 2 つの区間にまたがったら前の区間にだけ数える。
#[tokio::test]
async fn day_cell_dropped_rounds_the_end_up_and_counts_hours_once() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "cellround", SIX_HOURS, Some("2026-05-01")).await;
    // 10:00:00 JST と 10:30:00 JST の 1 件ずつ、別の報告（離れているので別の区間）
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-01T01:00:00Z", "2026-05-01T01:00:00.001Z")),
        &[("2026-05-01T01:00:00Z", 1)],
    )
    .await;
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-01T01:30:00Z", "2026-05-01T01:30:00.001Z")),
        &[("2026-05-01T01:00:00Z", 1)],
    )
    .await;
    let c = cell_on(&pool, u, &s, "2026-05-01").await;
    assert_eq!(c.dropped_count, 2);
    assert_eq!(
        c.dropped_ranges,
        vec![range("10:00", "10:01", 2), range("10:30", "10:31", 0)]
    );
}

/// 23:59 台に終わる区間は、分に切り上げると日の終わりに届くので `24:00`（review R11 / design D19）。
#[tokio::test]
async fn day_cell_dropped_ending_in_the_last_minute_is_24_00() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "celllastmin", SIX_HOURS, Some("2026-05-01")).await;
    // 2026-05-01 22:00 JST 〜 23:59:10.001 JST（翌日に続かない）
    testdb::put_drop(
        &pool,
        u,
        &s.logical_source,
        Some(("2026-05-01T13:00:00Z", "2026-05-01T14:59:10.001Z")),
        &[("2026-05-01T13:00:00Z", 60), ("2026-05-01T14:00:00Z", 60)],
    )
    .await;
    let c = cell_on(&pool, u, &s, "2026-05-01").await;
    assert_eq!(c.dropped_ranges, vec![range("22:00", "24:00", 120)]);
    // 翌日には何も出ない
    assert!(cell_on(&pool, u, &s, "2026-05-02")
        .await
        .dropped_ranges
        .is_empty());
}

/// 破棄は利用者ごとに分かれる（FR-29）。別の利用者の破棄はその日の件数にも区間にも入らない。
#[tokio::test]
async fn day_cell_dropped_is_per_user() {
    let pool = testdb::pool().await;
    let (s, u) = src(&pool, "celluser", SIX_HOURS, Some("2026-05-01")).await;
    let other = testdb::user();
    testdb::put_drop(
        &pool,
        other,
        &s.logical_source,
        Some(("2026-05-01T01:00:00Z", "2026-05-01T02:00:00Z")),
        &[("2026-05-01T01:00:00Z", 60)],
    )
    .await;
    let c = cell_on(&pool, u, &s, "2026-05-01").await;
    assert_eq!(c.dropped_count, 0);
    assert!(c.dropped_ranges.is_empty());
    assert_eq!(
        cell_on(&pool, other, &s, "2026-05-01").await.dropped_count,
        60
    );
}

/// 引き継ぎの鎖（第 8 回 Q31）の古い名前で届いた破棄も、先端のソースの稼働状況に入る。
#[tokio::test]
async fn day_cell_dropped_follows_the_chain() {
    let pool = testdb::pool().await;
    let (mut s, u) = src(&pool, "cellchain", SIX_HOURS, Some("2026-05-01")).await;
    let old = testdb::source(&pool, "cellchain-old", SIX_HOURS).await;
    s.chain = vec![old.clone(), s.logical_source.clone()];
    testdb::put_drop(
        &pool,
        u,
        &old,
        Some(("2026-05-01T01:00:00Z", "2026-05-01T02:00:00Z")),
        &[("2026-05-01T01:00:00Z", 60)],
    )
    .await;
    let c = cell_on(&pool, u, &s, "2026-05-01").await;
    assert_eq!(c.dropped_count, 60);
    assert_eq!(c.dropped_ranges, vec![range("10:00", "11:00", 60)]);
}
