// SPDX-License-Identifier: AGPL-3.0-only
//! アプリの役割（`ashiato_app`）の権限を、**本物の PostgreSQL に対して**確かめる（ST28 / design D4）。
//!
//! 表の名前を試験に書かない —— `core` の全表を `pg_tables` から引く。並走中の change が足す表も自動で入る。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

/// `core` の全表の（名前, 所有者）。
async fn core_tables(pool: &sqlx::PgPool) -> Vec<(String, String)> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT tablename::text, tableowner::text FROM pg_tables
          WHERE schemaname = 'core' ORDER BY tablename",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    assert!(
        !rows.is_empty(),
        "core の表が引けない（移行が当たっていない）"
    );
    rows
}

/// SQLSTATE 42501（権限の不足）で拒まれたか。
fn is_denied(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .and_then(|d| d.code())
        .is_some_and(|c| c == "42501")
}

/// Scenario: 移行が足したどの表にもアプリの役割が届き、切り詰めの権限は無い
#[tokio::test]
async fn app_role_privileges_reach_every_table() {
    let app = testdb::app_pool().await;
    let tables = core_tables(&app).await;
    for (name, owner) in &tables {
        let q = format!("core.\"{name}\"");
        for (privilege, want) in [
            ("SELECT", true),
            ("INSERT", true),
            ("UPDATE", true),
            ("DELETE", true),
            ("TRUNCATE", false),
            ("REFERENCES", false),
            ("TRIGGER", false),
        ] {
            let got: bool = sqlx::query_scalar("SELECT has_table_privilege(current_user, $1, $2)")
                .bind(&q)
                .bind(privilege)
                .fetch_one(&app)
                .await
                .unwrap();
            assert_eq!(got, want, "{name} の {privilege}");
        }
        let me: String = sqlx::query_scalar("SELECT current_user::text")
            .fetch_one(&app)
            .await
            .unwrap();
        assert_ne!(owner, &me, "{name} をアプリの役割が所有している");
    }
}

/// Scenario: アプリの接続からは門を外せない
/// Scenario: アプリの接続からは表の定義を変えられない
/// Scenario: アプリの接続からはどの表も切り詰められない
#[tokio::test]
async fn app_role_cannot_bypass_gate() {
    let owner = testdb::pool().await;
    let app = testdb::app_pool().await;

    // 門を外す設定
    let e = sqlx::query("SET session_replication_role = replica")
        .execute(&app)
        .await
        .unwrap_err();
    assert!(
        is_denied(&e),
        "session_replication_role が権限の不足で拒まれない: {e}"
    );

    // トリガを無効にする定義の変更。拒まれ、トリガは有効のまま
    let e = sqlx::query("ALTER TABLE core.event DISABLE TRIGGER ALL")
        .execute(&app)
        .await
        .unwrap_err();
    let disabled: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_trigger
          WHERE tgrelid = 'core.event'::regclass AND NOT tgisinternal AND tgenabled = 'D'",
    )
    .fetch_one(&owner)
    .await
    .unwrap();
    assert!(
        e.as_database_error().is_some() && disabled == 0,
        "DISABLE TRIGGER が拒まれていない / トリガが無効になった: {e}"
    );

    // どの表も切り詰められない（行を 1 行ずつ置いて、行数が変わらないことも見る）。
    // 開発 DB は並走する試験と共有なので、数える間は所有者の側で表を錠で止め、他の試験の挿入で
    // 行数がずれないようにする。TRUNCATE の権限は錠を取る前に判定されるので、拒否は待たずに返る
    let tables = core_tables(&owner).await;
    for (name, _) in &tables {
        let mut guard = owner.begin().await.unwrap();
        sqlx::query(&format!(
            "LOCK TABLE core.\"{name}\" IN SHARE ROW EXCLUSIVE MODE"
        ))
        .execute(&mut *guard)
        .await
        .unwrap();
        let before: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM core.\"{name}\""))
            .fetch_one(&mut *guard)
            .await
            .unwrap();
        let e = sqlx::query(&format!("TRUNCATE core.\"{name}\" CASCADE"))
            .execute(&app)
            .await
            .unwrap_err();
        assert!(
            is_denied(&e),
            "{name} の TRUNCATE が権限の不足で拒まれない: {e}"
        );
        let after: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM core.\"{name}\""))
            .fetch_one(&mut *guard)
            .await
            .unwrap();
        guard.commit().await.unwrap();
        assert_eq!(before, after, "{name} の行数が変わった");
    }
}

/// Scenario: アプリの接続で取り込みと読み出しが通る
#[tokio::test]
async fn app_role_ingest_and_read() {
    let owner = testdb::pool().await;
    let source = testdb::source(&owner, "approle", 21_600).await;
    let app = App::for_test(testdb::app_pool().await, TOKEN);
    let mut headers = HeaderMap::new();
    headers.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());

    let id = uuid::Uuid::new_v4();
    let body = serde_json::json!({
        "id": id,
        "user_id": testdb::user(),
        "logical_source": source,
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": "2026-03-01T03:00:00Z",
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": r#"{"a":1}"#,
        "payload": {},
    });
    let (code, Json(res)) = ingest(State(app.clone()), headers.clone(), Json(body))
        .await
        .expect("取り込み口");
    assert_eq!(code, StatusCode::OK);
    assert_eq!(res.len(), 1);

    let Json(rows) = events(State(app), headers).await.expect("読み出し口");
    assert!(
        rows.iter().any(|r| r.id == id),
        "取り込んだ記録が読み出せない"
    );
}
