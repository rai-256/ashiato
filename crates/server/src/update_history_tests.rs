// SPDX-License-Identifier: AGPL-3.0-only
//! 更新の履歴の置き場・列の形・門・付与を、本物の PostgreSQL に対して確かめる（ST51 / design D4 / D10）。
#![allow(clippy::unwrap_used)]

use crate::testdb;

/// `information_schema.columns` の（名前, 型, NULL 可）を列の並びの順に。
async fn columns(pool: &sqlx::PgPool, table: &str) -> Vec<(String, String, bool)> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT column_name::text, data_type::text, is_nullable::text
           FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = $1
          ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .unwrap();
    rows.into_iter()
        .map(|(n, t, nl)| (n, t, nl == "YES"))
        .collect()
}

fn denied_by_gate(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.message().contains("更新の履歴は書き換えられない"))
}

const ONE_ROW: &str = "INSERT INTO core.update_history (component, via, to_version, outcome)
                       VALUES ('S-01', 'migrate', '0.1.0+st51test', 'applied')";

/// Scenario: 更新の履歴の置き場と列の形は、決めた形のまま変わらない
#[tokio::test]
async fn st51_form_fixed() {
    let pool = testdb::pool().await;
    let want = [
        ("seq", "bigint", false),
        ("at", "timestamp with time zone", false),
        ("component", "text", false),
        ("via", "text", false),
        ("from_version", "text", true),
        ("to_version", "text", false),
        ("outcome", "text", false),
        ("applied_versions", "ARRAY", false),
        ("failed_version", "text", true),
        ("sqlstate", "text", true),
    ];
    let got = columns(&pool, "update_history").await;
    let want: Vec<(String, String, bool)> = want
        .iter()
        .map(|(n, t, nl)| (n.to_string(), t.to_string(), *nl))
        .collect();
    assert_eq!(got, want);
    let elem: String = sqlx::query_scalar(
        "SELECT e.data_type::text FROM information_schema.element_types e
          WHERE e.object_schema = 'core' AND e.object_name = 'update_history'
            AND e.collection_type_identifier = (
              SELECT dtd_identifier FROM information_schema.columns
               WHERE table_schema = 'core' AND table_name = 'update_history'
                 AND column_name = 'applied_versions')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(elem, "text");
    let pk: Vec<String> = sqlx::query_scalar(
        "SELECT a.attname::text FROM pg_index i
           JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY (i.indkey)
          WHERE i.indrelid = 'core.update_history'::regclass AND i.indisprimary",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(pk, ["seq"]);
    let ident: String = sqlx::query_scalar(
        "SELECT is_generated::text || '/' || COALESCE(identity_generation, '') FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'update_history' AND column_name = 'seq'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ident, "NEVER/ALWAYS");
}

/// Scenario: 決めていない値の行は入らない
#[tokio::test]
async fn st51_form_rejects_unknown_via() {
    let pool = testdb::pool().await;
    for (component, via, outcome) in [
        ("S-01", "upgrade", "applied"),
        ("C-01", "migrate", "applied"),
        ("S-01", "migrate", "rolled_back"),
    ] {
        let e = sqlx::query(
            "INSERT INTO core.update_history (component, via, to_version, outcome) VALUES ($1, $2, 'v', $3)",
        )
        .bind(component)
        .bind(via)
        .bind(outcome)
        .execute(&pool)
        .await
        .unwrap_err();
        let code = e
            .as_database_error()
            .and_then(|d| d.code().map(|c| c.to_string()));
        assert_eq!(
            code.as_deref(),
            Some("23514"),
            "{component}/{via}/{outcome}: {e}"
        );
    }
}

/// 生存信号の版の列は text・NULL 可（D4）。
#[tokio::test]
async fn st51_form_heartbeat_column() {
    let pool = testdb::pool().await;
    let got = columns(&pool, "heartbeat").await;
    let col = got.iter().find(|(n, _, _)| n == "collector_version");
    assert_eq!(
        col,
        Some(&("collector_version".to_string(), "text".to_string(), true))
    );
}

/// Scenario: 更新の履歴の行は書き換えられない
#[tokio::test]
async fn st51_rows_cannot_be_rewritten() {
    let owner = testdb::pool().await;
    let app = testdb::app_pool().await;
    sqlx::query(ONE_ROW).execute(&owner).await.unwrap();
    let seq: i64 = sqlx::query_scalar("SELECT max(seq) FROM core.update_history")
        .fetch_one(&owner)
        .await
        .unwrap();
    let before: String =
        sqlx::query_scalar("SELECT to_version FROM core.update_history WHERE seq = $1")
            .bind(seq)
            .fetch_one(&owner)
            .await
            .unwrap();
    for (who, pool) in [("owner", &owner), ("app", &app)] {
        for sql in [
            "UPDATE core.update_history SET to_version = 'x' WHERE seq = $1",
            "DELETE FROM core.update_history WHERE seq = $1",
        ] {
            let e = sqlx::query(sql).bind(seq).execute(pool).await.unwrap_err();
            let denied = denied_by_gate(&e)
                || e.as_database_error()
                    .and_then(|d| d.code())
                    .is_some_and(|c| c == "42501");
            assert!(denied, "{who}: {sql}: {e}");
        }
        // 切り詰め（所有者は門、アプリは権限が断る）
        let e = sqlx::query("TRUNCATE core.update_history")
            .execute(pool)
            .await
            .unwrap_err();
        assert!(e.as_database_error().is_some(), "{who}: TRUNCATE: {e}");
    }
    // 所有者には門が断ったこと自体を見る（権限でなく）
    let e = sqlx::query("DELETE FROM core.update_history WHERE seq = $1")
        .bind(seq)
        .execute(&owner)
        .await
        .unwrap_err();
    assert!(denied_by_gate(&e), "{e}");
    let after: String =
        sqlx::query_scalar("SELECT to_version FROM core.update_history WHERE seq = $1")
            .bind(seq)
            .fetch_one(&owner)
            .await
            .unwrap();
    assert_eq!(before, after);
}

/// Scenario: S-01 の実行時の役割は、更新の履歴を読み、足せる
#[tokio::test]
async fn st51_rows_app_role_reads_and_inserts() {
    let app = testdb::app_pool().await;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM core.update_history")
        .fetch_one(&app)
        .await
        .unwrap();
    sqlx::query(ONE_ROW).execute(&app).await.unwrap();
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM core.update_history")
        .fetch_one(&app)
        .await
        .unwrap();
    assert!(after > before);
}

// ====================================================================== 当てる処理が書く行（Task 2）
// 試験用の並び = `MIGRATIONS` + 試験用の版。印は引数で渡し、同じプロセスの中で返り値と行を見る。

mod migrate_rows {
    use crate::migration_ledger::{apply_migrations_recorded, MigrateError};
    use crate::testdb::TempDb;
    use crate::update_history::{record_migrate, record_startup, unwritten_seen, Attempt, Outcome};
    use sqlx::Connection as _;

    type V = (&'static str, &'static str);

    const A: &str = "0.1.0+aaaaaaaa";
    const B: &str = "0.1.0+bbbbbbbb";
    const C: &str = "0.1.0+cccccccc";
    const MAKE_T: V = (
        "999901010000_st51_t",
        "CREATE TABLE st51_t (c text); INSERT INTO st51_t VALUES ('st51-secret-value');",
    );
    /// text の列を integer に変える。値が入っているので 22P02 で落ち、誤りの本文に値が入る。
    const BREAK_T: V = (
        "999901010001_st51_break",
        "ALTER TABLE st51_t ALTER COLUMN c TYPE integer USING c::integer;",
    );
    const NOOP: V = ("999901010002_st51_noop", "SELECT 1;");

    fn with(extra: &[V]) -> Vec<V> {
        crate::MIGRATIONS
            .iter()
            .copied()
            .chain(extra.iter().copied())
            .collect()
    }

    async fn run(
        db: &TempDb,
        versions: &[V],
        stamp: Option<&str>,
    ) -> Result<Vec<String>, MigrateError> {
        run_with(db, versions, stamp, || Ok(()), crate::GRANTS).await
    }

    async fn run_with(
        db: &TempDb,
        versions: &[V],
        stamp: Option<&str>,
        before: impl FnOnce() -> anyhow::Result<()>,
        grants: &str,
    ) -> Result<Vec<String>, MigrateError> {
        let conn = db.owner().acquire().await.unwrap();
        apply_migrations_recorded(conn, versions, before, grants, stamp).await
    }

    /// 表ができた後の素の DB（行は 0）。
    async fn migrated() -> TempDb {
        let db = TempDb::create().await;
        run(&db, &with(&[]), None).await.unwrap();
        db
    }

    /// 印 A の通った行が 1 行ある DB。
    async fn after_a(extra: &[V]) -> TempDb {
        let db = TempDb::create().await;
        run(&db, &with(extra), Some(A)).await.unwrap();
        db
    }

    #[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
    struct Row {
        via: String,
        from: Option<String>,
        to: String,
        outcome: String,
        applied: Vec<String>,
        failed_version: Option<String>,
        sqlstate: Option<String>,
    }

    fn row(
        via: &str,
        from: Option<&str>,
        to: &str,
        outcome: &str,
        applied: &[&str],
        failed: Option<&str>,
        sqlstate: Option<&str>,
    ) -> Row {
        Row {
            via: via.into(),
            from: from.map(Into::into),
            to: to.into(),
            outcome: outcome.into(),
            applied: applied.iter().map(|s| (*s).to_string()).collect(),
            failed_version: failed.map(Into::into),
            sqlstate: sqlstate.map(Into::into),
        }
    }

    async fn rows(pool: &sqlx::PgPool) -> Vec<Row> {
        sqlx::query_as(
            "SELECT via, from_version AS \"from\", to_version AS \"to\", outcome,
                    applied_versions AS applied, failed_version, sqlstate
               FROM core.update_history ORDER BY seq",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    /// 別の OS スレッド・別の runtime・別の接続で走らせて結果を待つ（`before_apply` は同期の関数なので）。
    fn on_other_thread<T: Send + 'static>(
        opts: sqlx::postgres::PgConnectOptions,
        f: impl FnOnce(
                sqlx::PgConnection,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>
            + Send
            + 'static,
    ) -> T {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let conn = sqlx::PgConnection::connect_with(&opts).await.unwrap();
                f(conn).await
            })
        })
        .join()
        .unwrap()
    }

    // Scenario: 新しい組み立ての migrate が版を当てると、当てた版の名前を持つ 1 行が残る
    #[tokio::test]
    async fn st51_migrate_applied_row() {
        let db = after_a(&[]).await;
        let got = run(&db, &with(&[NOOP]), Some(B)).await.unwrap();
        assert_eq!(got, [NOOP.0]);
        let all = rows(db.owner()).await;
        let n = all.len();
        assert_eq!(
            all[n - 2..],
            [
                row("migrate", Some(A), B, "attempted", &[], None, None),
                row("migrate", Some(A), B, "applied", &[NOOP.0], None, None),
            ]
        );
    }

    // Scenario: 当てる版が無い入れ替えの migrate も 1 行残る
    #[tokio::test]
    async fn st51_migrate_nothing_to_apply_row() {
        let db = after_a(&[]).await;
        let got = run(&db, &with(&[]), Some(B)).await.unwrap();
        assert!(got.is_empty());
        let all = rows(db.owner()).await;
        let n = all.len();
        assert_eq!(
            all[n - 1],
            row("migrate", Some(A), B, "nothing_to_apply", &[], None, None)
        );
    }

    // Scenario: 同じ組み立てで migrate を走らせ直しても、行は増えない
    #[tokio::test]
    async fn st51_migrate_same_stamp_no_row() {
        let db = after_a(&[]).await;
        let before = rows(db.owner()).await.len();
        run(&db, &with(&[]), Some(A)).await.unwrap();
        run(&db, &with(&[]), Some(A)).await.unwrap();
        assert_eq!(rows(db.owner()).await.len(), before);
    }

    // Scenario: 移行が落ちた回は、落ちた版と SQLSTATE を持つ 1 行が残り、誤りの本文は残らない
    #[tokio::test]
    async fn st51_migrate_failed_row_sqlstate() {
        let db = after_a(&[MAKE_T]).await;
        let e = run(&db, &with(&[MAKE_T, BREAK_T]), Some(B))
            .await
            .unwrap_err();
        let line = e.failure_line().unwrap();
        assert!(!line.contains("st51-secret-value"), "{line}");
        assert!(line.contains(BREAK_T.0) && line.contains("22P02"), "{line}");
        let all = rows(db.owner()).await;
        let n = all.len();
        assert_eq!(
            all[n - 2..],
            [
                row("migrate", Some(A), B, "attempted", &[], None, None),
                row(
                    "migrate",
                    Some(A),
                    B,
                    "failed",
                    &[],
                    Some(BREAK_T.0),
                    Some("22P02")
                ),
            ]
        );
        let dump: String =
            sqlx::query_scalar("SELECT string_agg(u::text, ' ') FROM core.update_history u")
                .fetch_one(db.owner())
                .await
                .unwrap();
        assert!(!dump.contains("st51-secret-value"), "{dump}");
    }

    // Scenario: 落ちた回の後の行の前の版は、最後に通った版になる
    #[tokio::test]
    async fn st51_migrate_after_failure_prev_is_last_passed() {
        let db = after_a(&[MAKE_T]).await;
        run(&db, &with(&[MAKE_T, BREAK_T]), Some(B))
            .await
            .unwrap_err();
        run(&db, &with(&[MAKE_T]), Some(C)).await.unwrap();
        let all = rows(db.owner()).await;
        let n = all.len();
        assert_eq!(
            all[n - 1],
            row("migrate", Some(A), C, "nothing_to_apply", &[], None, None),
            "前の版は落ちた回の B でなく、最後に通った A"
        );
    }

    // Scenario: 落ちた回の行は、落ちた版と一緒に戻らない
    #[tokio::test]
    async fn st51_migrate_failed_row_survives_rollback() {
        let db = after_a(&[MAKE_T]).await;
        run(&db, &with(&[MAKE_T, BREAK_T]), Some(B))
            .await
            .unwrap_err();
        let recorded: i64 =
            sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied WHERE name = $1")
                .bind(BREAK_T.0)
                .fetch_one(db.owner())
                .await
                .unwrap();
        assert_eq!(recorded, 0, "落ちた版が版の記録にある");
        let col: String = sqlx::query_scalar(
            "SELECT data_type::text FROM information_schema.columns WHERE table_name = 'st51_t'",
        )
        .fetch_one(db.owner())
        .await
        .unwrap();
        assert_eq!(col, "text", "落ちた版の変更が戻っていない");
        let failed: i64 =
            sqlx::query_scalar("SELECT count(*) FROM core.update_history WHERE outcome = 'failed'")
                .fetch_one(db.owner())
                .await
                .unwrap();
        assert_eq!(failed, 1);
    }

    // Scenario: 更新の履歴の初めの行は、前の版を空にする
    #[tokio::test]
    async fn st51_migrate_first_row_prev_empty() {
        let db = migrated().await;
        assert!(rows(db.owner()).await.is_empty());
        run(&db, &with(&[]), Some(A)).await.unwrap();
        assert_eq!(
            rows(db.owner()).await,
            [
                row("migrate", None, A, "attempted", &[], None, None),
                row("migrate", None, A, "nothing_to_apply", &[], None, None),
            ]
        );
    }

    // Scenario: 更新の履歴の表が無い D-01 で落ちても、当てる処理は落ちた版の失敗を返し、書けなかったことを警告する
    #[tokio::test]
    async fn st51_migrate_no_table_warns() {
        let db = TempDb::create().await;
        let versions: [V; 2] = [
            ("999901010000_ok", "CREATE TABLE st51_ok (x int);"),
            ("999901010001_bad", "SELECT 1/0;"),
        ];
        let e = run(&db, &versions, Some(A)).await.unwrap_err();
        assert!(
            matches!(&e, MigrateError::Version { name, .. } if name == "999901010001_bad"),
            "{e}"
        );
        assert!(unwritten_seen().contains(&"no_table"));
        let exists: bool =
            sqlx::query_scalar("SELECT to_regclass('core.update_history') IS NOT NULL")
                .fetch_one(db.owner())
                .await
                .unwrap();
        assert!(!exists);
    }

    // Scenario: 付与で落ちた migrate は、落ちた版の名前を持たない failed の行を残す
    #[tokio::test]
    async fn st51_migrate_grants_failure_row() {
        let db = after_a(&[]).await;
        let e = run_with(&db, &with(&[NOOP]), Some(B), || Ok(()), "SELECT 1/0;")
            .await
            .unwrap_err();
        assert!(matches!(e, MigrateError::Grants(_)), "{e}");
        let all = rows(db.owner()).await;
        let n = all.len();
        assert_eq!(
            all[n - 1],
            row(
                "migrate",
                Some(A),
                B,
                "failed",
                &[NOOP.0],
                None,
                Some("22012")
            )
        );
    }

    // Scenario: 時計が戻っても、前の版は書いた順で決まる
    #[tokio::test]
    async fn st51_migrate_prev_by_seq_not_time() {
        let db = migrated().await;
        for (to, at) in [(A, "2026-10-30T00:00:00Z"), (B, "2000-01-01T00:00:00Z")] {
            sqlx::query(
                "INSERT INTO core.update_history (at, component, via, to_version, outcome)
                 VALUES ($1::timestamptz, 'S-01', 'migrate', $2, 'applied')",
            )
            .bind(at)
            .bind(to)
            .execute(db.owner())
            .await
            .unwrap();
        }
        run(&db, &with(&[]), Some(C)).await.unwrap();
        let all = rows(db.owner()).await;
        assert_eq!(all[all.len() - 1].from.as_deref(), Some(B));
    }

    // Scenario: 同じ版の変化を 2 つ同時に書いても、行は 1 つ
    #[tokio::test]
    async fn st51_migrate_concurrent_one_row() {
        let db = after_a(&[]).await;
        let opts = (*db.owner().connect_options()).clone();
        let mut c1 = sqlx::PgConnection::connect_with(&opts).await.unwrap();
        let mut c2 = sqlx::PgConnection::connect_with(&opts).await.unwrap();
        let out = Outcome::NothingToApply;
        tokio::time::timeout(std::time::Duration::from_secs(60), async {
            tokio::join!(
                record_migrate(&mut c1, B, Attempt::Unwritten, &out),
                record_migrate(&mut c2, B, Attempt::Unwritten, &out),
            )
        })
        .await
        .unwrap();
        let n: i64 =
            sqlx::query_scalar("SELECT count(*) FROM core.update_history WHERE to_version = $1")
                .bind(B)
                .fetch_one(db.owner())
                .await
                .unwrap();
        assert_eq!(n, 1);
    }

    // Scenario: commit の取れない組み立ての印も、そのまま今の版に書く
    #[tokio::test]
    async fn st51_migrate_unknown_stamp_as_is() {
        let db = after_a(&[]).await;
        run(&db, &with(&[]), Some("0.1.0+unknown")).await.unwrap();
        let all = rows(db.owner()).await;
        assert_eq!(all[all.len() - 1].to, "0.1.0+unknown");
    }

    // ST48 の版の記録は落ちた版を持たない（既存の振る舞いが変わらない）
    #[tokio::test]
    async fn st51_migrate_ledger_untouched() {
        let db = after_a(&[MAKE_T]).await;
        let before: Vec<String> =
            sqlx::query_scalar("SELECT name FROM migration_ledger.applied ORDER BY name")
                .fetch_all(db.owner())
                .await
                .unwrap();
        run(&db, &with(&[MAKE_T, BREAK_T, NOOP]), Some(B))
            .await
            .unwrap_err();
        let after: Vec<String> =
            sqlx::query_scalar("SELECT name FROM migration_ledger.applied ORDER BY name")
                .fetch_all(db.owner())
                .await
                .unwrap();
        assert_eq!(before, after);
    }

    // Scenario: 退避を取るときには、試みの行がもう D-01 に入っている
    #[tokio::test]
    async fn st51_migrate_attempt_committed_before_backup() {
        let db = after_a(&[]).await;
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = seen.clone();
        let opts = (*db.owner().connect_options()).clone();
        run_with(
            &db,
            &with(&[]),
            Some(B),
            move || {
                // 別の接続から読む（commit 済みでなければ見えない）
                let got = on_other_thread(opts, |mut c| {
                    Box::pin(async move {
                        sqlx::query_as::<_, (String, String)>(
                            "SELECT to_version, outcome FROM core.update_history ORDER BY seq DESC LIMIT 1",
                        )
                        .fetch_all(&mut c)
                        .await
                        .unwrap()
                    })
                });
                *sink.lock().unwrap() = got;
                Ok(())
            },
            crate::GRANTS,
        )
        .await
        .unwrap();
        assert_eq!(
            *seen.lock().unwrap(),
            [(B.to_string(), "attempted".to_string())]
        );
    }

    // 試みの行だけが最後にある D-01 では、前の版は最後に通った行の版になる（D7・D16（仮））
    #[tokio::test]
    async fn st51_migrate_attempt_not_counted_as_prev() {
        let db = migrated().await;
        sqlx::query(
            "INSERT INTO core.update_history (component, via, to_version, outcome)
             VALUES ('S-01', 'migrate', $1, 'attempted')",
        )
        .bind(B)
        .execute(db.owner())
        .await
        .unwrap();
        run(&db, &with(&[]), Some(B)).await.unwrap();
        let all = rows(db.owner()).await;
        assert_eq!(
            all[1..],
            [
                row("migrate", None, B, "attempted", &[], None, None),
                row("migrate", None, B, "nothing_to_apply", &[], None, None),
            ],
            "試みの行を前の版に数えていない"
        );
    }

    // Scenario: 当てる処理の途中で同じ組み立ての S-01 が起動しても、試みと結果の組は崩れない
    #[tokio::test]
    async fn st51_migrate_startup_between_attempt_and_result() {
        let db = after_a(&[]).await;
        let opts = (*db.owner().connect_options()).clone();
        run_with(
            &db,
            &with(&[]),
            Some(B),
            move || {
                // 起動の行を書く取引は錠を待たずに戻る（待つと migrate の錠を待って止まる）
                on_other_thread(opts, |mut conn| {
                    Box::pin(async move {
                        tokio::time::timeout(
                            std::time::Duration::from_secs(20),
                            record_startup(&mut conn, B, crate::MIGRATIONS),
                        )
                        .await
                        .expect("起動の行を書く口が錠を待って止まった");
                    })
                });
                Ok(())
            },
            crate::GRANTS,
        )
        .await
        .unwrap();
        let all = rows(db.owner()).await;
        assert_eq!(
            all[1..],
            [
                row("migrate", Some(A), B, "attempted", &[], None, None),
                row("migrate", Some(A), B, "nothing_to_apply", &[], None, None),
            ],
            "起動の行が組に割り込んだ"
        );
    }

    // Scenario: 更新の履歴の表が無い D-01 に初めて当てて通った migrate は、警告を出し、結果の 1 行だけを残す
    #[tokio::test]
    async fn st51_migrate_first_install_no_attempt() {
        let db = TempDb::create().await;
        run(&db, &with(&[]), Some(A)).await.unwrap();
        assert!(unwritten_seen().contains(&"no_table"));
        let got = rows(db.owner()).await;
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].outcome, "applied");
        assert_eq!(got[0].from, None);
        assert_eq!(got[0].to, A);
        assert_eq!(got[0].applied.len(), crate::MIGRATIONS.len());
    }

    // Scenario: 同じ組み立ての当てる処理を 2 つ同時に走らせても、試みと結果は 1 組だけ
    #[tokio::test]
    async fn st51_migrate_two_migrates_pairs_intact() {
        let db = after_a(&[]).await;
        let versions = with(&[]);
        let (r1, r2) = tokio::time::timeout(std::time::Duration::from_secs(120), async {
            tokio::join!(run(&db, &versions, Some(B)), run(&db, &versions, Some(B)))
        })
        .await
        .expect("2 つの当てる処理が止まった");
        r1.unwrap();
        r2.unwrap();
        let all = rows(db.owner()).await;
        assert_eq!(
            all[1..],
            [
                row("migrate", Some(A), B, "attempted", &[], None, None),
                row("migrate", Some(A), B, "nothing_to_apply", &[], None, None),
            ]
        );
    }

    // Scenario: 版の検査の後に知らない版が当たった D-01 には、起動の行を書かない
    #[tokio::test]
    async fn st51_migrate_startup_rechecks_ledger() {
        let db = after_a(&[]).await;
        sqlx::query(
            "INSERT INTO migration_ledger.applied (name, sha256, build_version, build_commit)
             VALUES ('999912312359_from_the_future', 'abc', '9.9.9', 'c')",
        )
        .execute(db.owner())
        .await
        .unwrap();
        let before = rows(db.owner()).await.len();
        let mut conn = sqlx::PgConnection::connect_with(&(*db.owner().connect_options()).clone())
            .await
            .unwrap();
        record_startup(&mut conn, B, crate::MIGRATIONS).await;
        assert_eq!(rows(db.owner()).await.len(), before);
        assert_eq!(unwritten_seen().last(), Some(&"db_ahead"));
        // 知らない版が無ければ、同じ呼び方で起動の行が書かれる
        sqlx::query("DELETE FROM migration_ledger.applied WHERE name LIKE '9999%'")
            .execute(db.owner())
            .await
            .unwrap();
        record_startup(&mut conn, B, crate::MIGRATIONS).await;
        let all = rows(db.owner()).await;
        assert_eq!(
            all[all.len() - 1],
            row("startup", Some(A), B, "nothing_to_apply", &[], None, None)
        );
    }
}
