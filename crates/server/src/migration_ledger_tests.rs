// SPDX-License-Identifier: AGPL-3.0-only
//! 版の記録の形・役割・読み方・比べ方（ST48 / design D1・D2・D3・D5）。
//! 知らない行や壊れた形は、使い捨ての database にだけ作る（D16）。
#![allow(clippy::unwrap_used)]

use crate::migration_ledger::{
    apply_migrations, compare, read_ledger, Ledger, MigrateError, Verdict, BOOTSTRAP,
};
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
    // 記録は付けずに版と付与だけを当て、知らない行 1 つだけを書く（`migrate()` は記録を付ける）
    apply_without_ledger(db.owner()).await;
    sqlx::raw_sql(crate::GRANTS)
        .execute(db.owner())
        .await
        .unwrap();
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

/// 記録の無い database に `MIGRATIONS` を直に当てる（ST48 より前の DB の再現）。
async fn apply_without_ledger(pool: &sqlx::PgPool) {
    for (_, sql) in crate::MIGRATIONS {
        sqlx::raw_sql(sql).execute(pool).await.unwrap();
    }
}

async fn apply(pool: &sqlx::PgPool, versions: &[(&str, &str)]) -> Result<(), MigrateError> {
    let mut conn = pool.acquire().await.unwrap();
    apply_migrations(&mut conn, versions).await
}

async fn recorded_names(pool: &sqlx::PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT name FROM migration_ledger.applied ORDER BY name")
        .fetch_all(pool)
        .await
        .unwrap()
}

fn sha256_hex(sql: &str) -> String {
    use sha2::{Digest as _, Sha256};
    use std::fmt::Write as _;
    Sha256::digest(sql.as_bytes())
        .iter()
        .fold(String::new(), |mut acc, b| {
            write!(acc, "{b:02x}").unwrap();
            acc
        })
}

/// Scenario: 空の DB に移行を当てると、当てた版がすべて記録される
#[tokio::test]
async fn st48_apply_records_every_version() {
    let db = TempDb::create().await;
    apply(db.owner(), crate::MIGRATIONS).await.unwrap();
    let mut want: Vec<String> = crate::MIGRATIONS.iter().map(|v| v.0.to_string()).collect();
    want.sort();
    assert_eq!(recorded_names(db.owner()).await, want);
    let marked: i64 =
        sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied WHERE before_ledger")
            .fetch_one(db.owner())
            .await
            .unwrap();
    assert_eq!(marked, 0);
}

/// Scenario: 版の記録の 1 行は、名前・指紋・時刻・組み立ての版番号と commit を持つ
#[tokio::test]
async fn st48_apply_row_has_fingerprint_and_build() {
    let db = TempDb::create().await;
    apply(db.owner(), crate::MIGRATIONS).await.unwrap();
    let rows: Vec<(String, String, String, String, bool)> = sqlx::query_as(
        "SELECT name, sha256, build_version, build_commit,
                (applied_at BETWEEN now() - interval '1 hour' AND now()) FROM migration_ledger.applied",
    )
    .fetch_all(db.owner())
    .await
    .unwrap();
    assert_eq!(rows.len(), crate::MIGRATIONS.len());
    for (name, sha, version, commit, recent) in rows {
        let sql = crate::MIGRATIONS.iter().find(|v| v.0 == name).unwrap().1;
        assert_eq!(sha, sha256_hex(sql), "{name}");
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
        assert_eq!(commit, env!("ASHIATO_BUILD_COMMIT"));
        assert!(recent, "{name} の applied_at が今でない");
    }
    // 渡さなければ git の HEAD。差分のビルドでは古いまま残りうる（design D8）ので、HEAD の祖先まで許す
    if std::env::var_os("ASHIATO_BUILD_COMMIT").is_none() {
        let commit = env!("ASHIATO_BUILD_COMMIT");
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .unwrap()
        };
        let head = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout).unwrap();
        assert!(
            commit == head.trim()
                || git(&["merge-base", "--is-ancestor", commit, "HEAD"])
                    .status
                    .success(),
            "commit {commit} が git の HEAD {head} と一致も祖先でもない"
        );
    }
}

/// Scenario: 版の記録の無い古い DB には一度だけ全部を当て、印を付けて記録する
#[tokio::test]
async fn st48_apply_pre_ledger_db_marks_all() {
    let db = TempDb::create().await;
    apply_without_ledger(db.owner()).await;
    let ev = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event (id,user_id,logical_source,origin,event_time,tz_offset_min,tz_id,
           schema_version,content_hash,raw,payload)
         VALUES ($1,$1,'c01-screenshot','collected',now(),0,'UTC',1,$1::text,'{}','{}')",
    )
    .bind(ev)
    .execute(db.owner())
    .await
    .unwrap();
    apply(db.owner(), crate::MIGRATIONS).await.unwrap();
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM core.event")
        .fetch_one(db.owner())
        .await
        .unwrap();
    assert_eq!(kept, 1);
    let unmarked: i64 =
        sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied WHERE NOT before_ledger")
            .fetch_one(db.owner())
            .await
            .unwrap();
    assert_eq!(unmarked, 0);
    assert_eq!(
        recorded_names(db.owner()).await.len(),
        crate::MIGRATIONS.len()
    );
}

/// Scenario: 移行が途中で落ちたら、当て終えた版だけが記録される
#[tokio::test]
async fn st48_apply_partial_failure_records_done_only() {
    let db = TempDb::create().await;
    let versions = [
        ("t1", "CREATE TABLE st48_t1(x int);"),
        ("t2", "SELECT 1/0;"),
        ("t3", "CREATE TABLE st48_t3(x int);"),
    ];
    let e = apply(db.owner(), &versions).await.unwrap_err();
    assert!(
        matches!(&e, MigrateError::Version { name, .. } if name == "t2"),
        "{e}"
    );
    assert_eq!(recorded_names(db.owner()).await, vec!["t1".to_string()]);
}

/// Scenario: 記録にある版は、中身をもう一度走らせない
#[tokio::test]
async fn st48_apply_does_not_rerun_recorded() {
    let db = TempDb::create().await;
    let versions = [(
        "t1",
        "CREATE TABLE IF NOT EXISTS st48_trace(x int); INSERT INTO st48_trace VALUES (1);",
    )];
    apply(db.owner(), &versions).await.unwrap();
    apply(db.owner(), &versions).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM st48_trace")
        .fetch_one(db.owner())
        .await
        .unwrap();
    assert_eq!(n, 1);
}

/// Scenario: 並びの前にある未適用の版も当たる
#[tokio::test]
async fn st48_apply_fills_earlier_gap() {
    let db = TempDb::create().await;
    let all = [
        ("t1", "CREATE TABLE st48_t1(x int);"),
        ("t2", "CREATE TABLE st48_t2(x int);"),
        ("t3", "CREATE TABLE st48_t3(x int);"),
    ];
    apply(db.owner(), &[all[0], all[2]]).await.unwrap();
    let before: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT applied_at FROM migration_ledger.applied WHERE name = 't3'")
            .fetch_one(db.owner())
            .await
            .unwrap();
    apply(db.owner(), &all).await.unwrap();
    assert_eq!(
        recorded_names(db.owner()).await,
        vec!["t1".to_string(), "t2".into(), "t3".into()]
    );
    let after: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT applied_at FROM migration_ledger.applied WHERE name = 't3'")
            .fetch_one(db.owner())
            .await
            .unwrap();
    assert_eq!(before, after);
}

/// Scenario: 2 つの migrate が同時に走っても、版は 1 回ずつ当たる
#[tokio::test]
async fn st48_apply_concurrent_runs_once() {
    let db = TempDb::create().await;
    let versions = [
        ("t1", "CREATE TABLE st48_t1(x int);"),
        ("t2", "CREATE TABLE st48_t2(x int);"),
    ];
    let (a, b) = tokio::join!(apply(db.owner(), &versions), apply(db.owner(), &versions));
    a.unwrap();
    b.unwrap();
    assert_eq!(
        recorded_names(db.owner()).await,
        vec!["t1".to_string(), "t2".into()]
    );
}

#[tokio::test]
async fn st48_apply_legacy_own_transaction_versions_are_recorded() {
    let db = TempDb::create().await;
    apply(db.owner(), crate::MIGRATIONS).await.unwrap();
    let names = recorded_names(db.owner()).await;
    for own_tx in [
        "202609082001_immutable_collected",
        "202609112113_source_lifecycle",
    ] {
        assert!(names.iter().any(|n| n == own_tx), "{own_tx} の記録が無い");
    }
}

/// Scenario: 試験の足場が当てた DB にも、同じ版の記録がある
#[tokio::test]
async fn st48_testdb_has_ledger() {
    let pool = crate::testdb::pool().await;
    let got: Vec<String> = sqlx::query_scalar("SELECT name FROM migration_ledger.applied")
        .fetch_all(&pool)
        .await
        .unwrap();
    let want: Vec<&str> = crate::MIGRATIONS.iter().map(|v| v.0).collect();
    assert_eq!(
        got.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        want.into_iter().collect::<BTreeSet<_>>()
    );
}
