// SPDX-License-Identifier: AGPL-3.0-only
//! 形の正本（`satellite.rs` の定数）と、移行・登録簿の突き合わせ（ST29 / design D5）。
#![allow(clippy::unwrap_used)]

use crate::satellite;

/// 移行の `identifier` の CHECK から、正規表現と長さの上限を字面で取り出す。
#[test]
fn migration_check_matches_satellite_constants() {
    let (_, sql) = crate::MIGRATIONS
        .iter()
        .find(|(name, _)| name.contains("satellite_credentials"))
        .expect("衛星の移行が MIGRATIONS に無い");
    let line = sql
        .lines()
        .find(|l| l.trim_start().starts_with("identifier "))
        .expect("移行に identifier の列が無い");
    let pattern = line
        .split("identifier ~ '")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .expect("identifier の CHECK に正規表現が無い");
    assert_eq!(pattern, satellite::IDENTIFIER_PATTERN);
    let limit: usize = line
        .split("length(identifier) <= ")
        .nth(1)
        .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|n| n.parse().ok())
        .expect("identifier の CHECK に長さの上限が無い");
    assert_eq!(limit, satellite::IDENTIFIER_MAX_LEN);
}

// Scenario: 本体の論理ソースの名前は区切りを含まない
#[tokio::test]
async fn builtin_logical_source_names_have_no_separator() {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    for (label, sql) in crate::MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(&fresh)
            .await
            .unwrap_or_else(|e| panic!("{label}: {e}"));
    }
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM core.source")
        .fetch_one(&fresh)
        .await
        .unwrap();
    let with_separator: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.source WHERE position($1 in logical_source) > 0",
    )
    .bind(satellite::NAME_SEPARATOR.to_string())
    .fetch_one(&fresh)
    .await
    .unwrap();
    drop.await;
    assert!(total > 0, "登録簿が空では確かめたことにならない");
    assert_eq!(with_separator, 0);
}
