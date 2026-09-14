// SPDX-License-Identifier: AGPL-3.0-only
//! 退役と後継の鎖（第 8 回 Q31）。**本物の PostgreSQL に対して**確かめる。
#![allow(clippy::unwrap_used)]

use super::super::*;
use super::*;
use crate::testdb;

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
