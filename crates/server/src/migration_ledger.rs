// SPDX-License-Identifier: AGPL-3.0-only
//! DB に当てた移行の版の記録を読み、自分の知る版と比べる（ST48 / design D1・D2・D3・D5）。
//!
//! 比べ方は**名前の集合**だけ（並びの位置・名前の時刻では比べない。ORDER は時刻順でない）。
//! 読むだけで、DB には何も書かない。

use std::collections::BTreeSet;

/// 版の記録の置き場を作る DDL（design D1）。`migrations/ORDER` の外。何度流しても同じ。
pub const BOOTSTRAP: &str = include_str!("migration_ledger.sql");

/// 記録の形の版。後の版が形を変えるときは上げる（古い版は知らない形として止まる。D5）。
pub const FORM: i32 = 1;

/// `migration_ledger.form` の列（名前, 型）。
const FORM_COLUMNS: &[(&str, &str)] = &[("only_row", "boolean"), ("form", "integer")];

/// `migration_ledger.applied` の列（名前, 型）。D5 の「読めない」は名前と型だけを見る。
const APPLIED_COLUMNS: &[(&str, &str)] = &[
    ("name", "text"),
    ("sha256", "text"),
    ("applied_at", "timestamp with time zone"),
    ("build_version", "text"),
    ("build_commit", "text"),
    ("before_ledger", "boolean"),
];

/// DB の版の記録の様子（design D3 の 3 分岐と D5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ledger {
    /// `migration_ledger` が無く、`core` も無い。
    Empty,
    /// `migration_ledger` が無く、`core` がある（ST48 より前の DB）。
    BeforeLedger,
    /// 記録のある DB。当たっている版の名前の集合。
    Recorded(BTreeSet<String>),
    /// 記録はあるが読めない（`form` の表か行が無い・`form` が 1 でない・`applied` の列が違う）。理由つき。
    Unreadable(String),
}

/// 知る版と DB の版の比べ結果（design D2）。`Ahead` を先に返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// DB にだけある名前がある（自分より新しい DB）。
    Ahead { unknown: Vec<String> },
    /// 自分にだけある名前がある（まだ当たっていない版）。
    Behind { missing: Vec<String> },
    /// 同じ集合。
    Even,
}

/// `known`（自分の知る版）と `ledger`（DB の版）の名前の集合を比べる。純粋。
pub fn compare(known: &BTreeSet<String>, ledger: &BTreeSet<String>) -> Verdict {
    let unknown: Vec<String> = ledger.difference(known).cloned().collect();
    if !unknown.is_empty() {
        return Verdict::Ahead { unknown };
    }
    let missing: Vec<String> = known.difference(ledger).cloned().collect();
    if !missing.is_empty() {
        return Verdict::Behind { missing };
    }
    Verdict::Even
}

async fn schema_exists(pool: &sqlx::PgPool, name: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)")
        .bind(name)
        .fetch_one(pool)
        .await
}

/// 表の列（名前, 型）を並びの順に引く。
async fn columns(pool: &sqlx::PgPool, table: &str) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT column_name::text, data_type::text FROM information_schema.columns
          WHERE table_schema = 'migration_ledger' AND table_name = $1
          ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(pool)
    .await
}

fn same_columns(found: &[(String, String)], want: &[(&str, &str)]) -> bool {
    found.len() == want.len()
        && found
            .iter()
            .zip(want)
            .all(|(f, w)| f.0 == w.0 && f.1 == w.1)
}

/// DB の版の記録を読む。DB の問い合わせそのものの失敗は `Err`（記録が「読めない」のとは別）。
/// アプリの役割（`SELECT` だけ）でも所有者でも読める。
pub async fn read_ledger(pool: &sqlx::PgPool) -> Result<Ledger, sqlx::Error> {
    if !schema_exists(pool, "migration_ledger").await? {
        return Ok(if schema_exists(pool, "core").await? {
            Ledger::BeforeLedger
        } else {
            Ledger::Empty
        });
    }
    let form_cols = columns(pool, "form").await?;
    if form_cols.is_empty() {
        return Ok(Ledger::Unreadable("form の表が無い".into()));
    }
    if !same_columns(&form_cols, FORM_COLUMNS) {
        return Ok(Ledger::Unreadable("form の列が決めた形と違う".into()));
    }
    let applied_cols = columns(pool, "applied").await?;
    if !same_columns(&applied_cols, APPLIED_COLUMNS) {
        return Ok(Ledger::Unreadable(
            "applied の表が無いか、列が決めた形と違う".into(),
        ));
    }
    let form: Option<i32> = sqlx::query_scalar("SELECT form FROM migration_ledger.form")
        .fetch_optional(pool)
        .await?;
    match form {
        None => return Ok(Ledger::Unreadable("form の行が無い".into())),
        Some(f) if f != FORM => {
            return Ok(Ledger::Unreadable(format!(
                "記録の形の版 {f} を知らない（知っているのは {FORM}）"
            )))
        }
        Some(_) => {}
    }
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM migration_ledger.applied")
        .fetch_all(pool)
        .await?;
    Ok(Ledger::Recorded(names.into_iter().collect()))
}
