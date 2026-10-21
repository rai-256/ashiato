// SPDX-License-Identifier: AGPL-3.0-only
//! 移行 `export_ledger` を当て直せることと、書き出しの台帳の錠（ST33 / design D10）。
#![allow(clippy::unwrap_used)]

use crate::testdb;

/// 全版をまっさらな DB に 2 回当てても壊れない（ST33 tasks 1.1）。
#[tokio::test]
async fn export_ledger_migration_applies_twice() {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    let applied = async {
        for round in 1..=2 {
            for (label, sql) in crate::MIGRATIONS {
                sqlx::raw_sql(sql)
                    .execute(&fresh)
                    .await
                    .map_err(|e| format!("{round} 回目の {label}: {e}"))?;
            }
        }
        sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass('core.export_ledger')::text")
            .fetch_one(&fresh)
            .await
            .map_err(|e| e.to_string())
    }
    .await;
    drop.await;

    assert_eq!(applied.unwrap().as_deref(), Some("core.export_ledger"));
    assert!(
        crate::MIGRATIONS
            .iter()
            .any(|(name, _)| name.ends_with("_export_ledger")),
        "書き出しの台帳の移行が MIGRATIONS に無い"
    );
}

/// 合言葉とそのハッシュの列を持たない（Q9）。
#[tokio::test]
async fn export_ledger_has_no_passphrase_column() {
    let pool = testdb::pool().await;
    let cols: Vec<(String,)> = sqlx::query_as(
        "SELECT column_name FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'export_ledger'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let cols: Vec<String> = cols.into_iter().map(|(c,)| c).collect();
    for want in ["file", "digest", "written", "rows", "bytes", "reason"] {
        assert!(cols.iter().any(|c| c == want), "{want} 列が無い");
    }
    assert!(
        !cols.iter().any(|c| {
            let c = c.to_lowercase();
            c.contains("pass") || c.contains("hash") || c.contains("key") || c.contains("secret")
        }),
        "合言葉・ハッシュ・鍵の列を持ってはいけない: {cols:?}"
    );
}

/// Scenario: 書き出しの台帳の行は書き換えも削除もできない
#[tokio::test]
async fn export_ledger_is_append_only() {
    let pool = testdb::pool().await;
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.export_ledger (export_id, kind, route, file)
         VALUES ($1, 'start', 'pc-encrypted-zip', '/tmp/append-only-test.zip')",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    // 所有者の接続でも拒まれる（トリガ。TRUNCATE は文トリガ）
    for stmt in [
        "UPDATE core.export_ledger SET file = 'x'",
        "DELETE FROM core.export_ledger",
        "TRUNCATE core.export_ledger",
    ] {
        let err = sqlx::query(stmt)
            .execute(&pool)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("書き出しの台帳"), "{stmt} が拒まれない: {err}");
    }
    let kept: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.export_ledger WHERE export_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(kept, 1, "拒まれた後も行が残っている");

    // 経路は pc-encrypted-zip だけ、start と result の対応が崩れる行は入らない
    for bad in [
        "INSERT INTO core.export_ledger (export_id, kind, route, file) VALUES (gen_random_uuid(), 'start', 'cloud', 'f')",
        "INSERT INTO core.export_ledger (export_id, kind, route, file, result) VALUES (gen_random_uuid(), 'start', 'pc-encrypted-zip', 'f', 'done')",
        "INSERT INTO core.export_ledger (export_id, kind, route, file) VALUES (gen_random_uuid(), 'finish', 'pc-encrypted-zip', 'f')",
    ] {
        assert!(sqlx::query(bad).execute(&pool).await.is_err(), "{bad}");
    }
}
