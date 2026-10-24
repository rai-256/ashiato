// SPDX-License-Identifier: AGPL-3.0-only
//! 版の記録の形・役割・読み方・比べ方（ST48 / design D1・D2・D3・D5）。
//! 知らない行や壊れた形は、使い捨ての database にだけ作る（D16）。
#![allow(clippy::unwrap_used)]

use crate::migration_ledger::{compare, read_ledger, Ledger, Verdict, BOOTSTRAP};
use crate::testdb::TempDb;
use std::collections::BTreeSet;

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

async fn columns(pool: &sqlx::PgPool) -> Vec<(String, String, String, String)> {
    sqlx::query_as(
        "SELECT table_name::text, column_name::text, data_type::text, is_nullable::text
           FROM information_schema.columns
          WHERE table_schema = 'migration_ledger'
          ORDER BY table_name, ordinal_position",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// bootstrap だけを当てた使い捨ての database。
async fn bootstrapped() -> TempDb {
    let db = TempDb::create().await;
    sqlx::raw_sql(BOOTSTRAP).execute(db.owner()).await.unwrap();
    db
}

/// Scenario: 版の記録の置き場と列の形は、決めた形のまま変わらない
#[tokio::test]
async fn st48_ledger_shape_is_frozen() {
    let db = bootstrapped().await;
    let want: Vec<(&str, &str, &str, &str)> = vec![
        ("applied", "name", "text", "NO"),
        ("applied", "sha256", "text", "NO"),
        ("applied", "applied_at", "timestamp with time zone", "NO"),
        ("applied", "build_version", "text", "NO"),
        ("applied", "build_commit", "text", "NO"),
        ("applied", "before_ledger", "boolean", "NO"),
        ("form", "only_row", "boolean", "NO"),
        ("form", "form", "integer", "NO"),
    ];
    let got = columns(db.owner()).await;
    let got: Vec<(&str, &str, &str, &str)> = got
        .iter()
        .map(|c| (c.0.as_str(), c.1.as_str(), c.2.as_str(), c.3.as_str()))
        .collect();
    assert_eq!(got, want);
    let form: i32 = sqlx::query_scalar("SELECT form FROM migration_ledger.form")
        .fetch_one(db.owner())
        .await
        .unwrap();
    assert_eq!(form, 1);
}

/// Scenario: S-01 の実行時の役割は版の記録を読めるが、書き換えられない
#[tokio::test]
async fn st48_ledger_app_role_reads_only() {
    let db = TempDb::create().await;
    crate::migrate(db.owner()).await.unwrap();
    sqlx::raw_sql(BOOTSTRAP).execute(db.owner()).await.unwrap();
    sqlx::query(
        "INSERT INTO migration_ledger.applied (name, sha256, build_version, build_commit)
         VALUES ('t', 'abc', '0', 'c')",
    )
    .execute(db.owner())
    .await
    .unwrap();
    let app = db.app().await;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied")
        .fetch_one(&app)
        .await
        .unwrap();
    assert_eq!(n, 1);
    for sql in [
        "INSERT INTO migration_ledger.applied (name, sha256, build_version, build_commit)
         VALUES ('x', 'abc', '0', 'c')",
        "UPDATE migration_ledger.applied SET sha256 = 'z'",
        "DELETE FROM migration_ledger.applied",
        "UPDATE migration_ledger.form SET form = 2",
    ] {
        let e = sqlx::query(sql).execute(&app).await.unwrap_err();
        let code = e.as_database_error().and_then(|d| d.code());
        assert_eq!(code.as_deref(), Some("42501"), "{sql}: {e}");
    }
    app.close().await;
}

#[test]
fn st48_compare_same_set_is_even() {
    assert_eq!(compare(&set(&["a", "b"]), &set(&["b", "a"])), Verdict::Even);
    assert_eq!(compare(&set(&[]), &set(&[])), Verdict::Even);
}

#[test]
fn st48_compare_db_only_name_is_ahead() {
    assert_eq!(
        compare(&set(&["a"]), &set(&["a", "z"])),
        Verdict::Ahead {
            unknown: vec!["z".into()]
        }
    );
}

#[test]
fn st48_compare_own_only_name_is_behind() {
    assert_eq!(
        compare(&set(&["a", "b"]), &set(&["a"])),
        Verdict::Behind {
            missing: vec!["b".into()]
        }
    );
}

#[test]
fn st48_compare_ahead_wins_over_behind() {
    assert_eq!(
        compare(&set(&["a", "b"]), &set(&["a", "z"])),
        Verdict::Ahead {
            unknown: vec!["z".into()]
        }
    );
}

#[test]
fn st48_compare_ignores_order() {
    let known: BTreeSet<String> = ["c", "a", "b"].iter().map(|s| s.to_string()).collect();
    let ledger: BTreeSet<String> = ["b", "c", "a"].iter().map(|s| s.to_string()).collect();
    assert_eq!(compare(&known, &ledger), Verdict::Even);
}

#[tokio::test]
async fn st48_read_ledger_empty_and_before_ledger_and_recorded() {
    let db = TempDb::create().await;
    assert_eq!(read_ledger(db.owner()).await.unwrap(), Ledger::Empty);
    sqlx::raw_sql("CREATE SCHEMA core")
        .execute(db.owner())
        .await
        .unwrap();
    assert_eq!(read_ledger(db.owner()).await.unwrap(), Ledger::BeforeLedger);
    sqlx::raw_sql(BOOTSTRAP).execute(db.owner()).await.unwrap();
    sqlx::query(
        "INSERT INTO migration_ledger.applied (name, sha256, build_version, build_commit)
         VALUES ('m1', 'abc', '0', 'c')",
    )
    .execute(db.owner())
    .await
    .unwrap();
    assert_eq!(
        read_ledger(db.owner()).await.unwrap(),
        Ledger::Recorded(set(&["m1"]))
    );
}

#[tokio::test]
async fn st48_read_ledger_unreadable_form_is_not_one() {
    let db = bootstrapped().await;
    sqlx::query("UPDATE migration_ledger.form SET form = 2")
        .execute(db.owner())
        .await
        .unwrap();
    assert!(matches!(
        read_ledger(db.owner()).await.unwrap(),
        Ledger::Unreadable(_)
    ));
}

#[tokio::test]
async fn st48_read_ledger_unreadable_form_row_missing() {
    let db = bootstrapped().await;
    sqlx::query("DELETE FROM migration_ledger.form")
        .execute(db.owner())
        .await
        .unwrap();
    assert!(matches!(
        read_ledger(db.owner()).await.unwrap(),
        Ledger::Unreadable(_)
    ));
}

#[tokio::test]
async fn st48_read_ledger_unreadable_applied_column_missing() {
    let db = bootstrapped().await;
    sqlx::query("ALTER TABLE migration_ledger.applied DROP COLUMN build_commit")
        .execute(db.owner())
        .await
        .unwrap();
    assert!(matches!(
        read_ledger(db.owner()).await.unwrap(),
        Ledger::Unreadable(_)
    ));
}
