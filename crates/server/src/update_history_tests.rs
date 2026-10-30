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
