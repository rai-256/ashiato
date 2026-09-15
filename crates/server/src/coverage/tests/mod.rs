//! 稼働状況の導出と、成功条件 1 の達成日数を、**本物の PostgreSQL に対して**確かめる。
//!
//! 日境界（`AT TIME ZONE`）も範囲の重なりも DB の側にあるので、模擬すると意味が消える。
#![allow(clippy::unwrap_used)]
//!
//! 2026-09-14 に 1 ファイル（2,217 行・56 本）から話題ごとに割った（テスト監査の推奨 7）。
//! 相手役（登録簿の行・状態の読み出し・5 ソースの埋め方）はここに置き、各モジュールは `use super::*` で引く。
use super::*;
use crate::testdb;

mod achievement;
mod drops;
mod retired;
mod spans;
mod started_on;
mod states;

// SPDX-License-Identifier: AGPL-3.0-only
/// テスト用の登録簿 1 行。`gap` は想定間隔（秒）、`started` は収集開始日。
pub(super) async fn src(
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
pub(super) async fn state_on(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    s: &SourceRow,
    day: &str,
) -> DayState {
    let d = testdb::date(day);
    let got = of_source(pool, Some(user), s, d, d).await.unwrap();
    got.days[0].state
}

pub(super) const SIX_HOURS: i32 = 21_600;

pub(super) const SIXTY_DAYS: i32 = 60 * 86_400;

/// 5 ソースぶんの登録簿を用意し、それぞれ `days` 日ぶんの達成を置く。
/// 返るのは `(targets, user, 開始日)`。
pub(super) async fn five_sources(
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
pub(super) async fn fill_achieved(
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

/// 移行 0007 の**引き直しの節だけ**を抜き出す。
///
/// ファイル全体を当てると `ALTER TABLE core.source` と `CREATE INDEX ... ON core.event` が
/// 2 つの表に強い錠を掛け、並んで走っている記録の挿入と deadlock する（実測 40P01）。
/// **逐語は本物のまま**なので、閾値をファイルから消せばこの検査が落ちる。
pub(super) fn repair_sql() -> &'static str {
    const SRC: &str = include_str!("../../../../../migrations/202609112113_source_lifecycle.sql");
    let from = SRC.find("-- REPAIR-BEGIN").expect("印 REPAIR-BEGIN が無い");
    let to = SRC.find("-- REPAIR-END").expect("印 REPAIR-END が無い");
    &SRC[from..to]
}
