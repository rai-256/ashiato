// SPDX-License-Identifier: AGPL-3.0-only
//! 移行 `access_control` を当て直せることと、読み出しの記録の錠（ST28 / design D8 / D9）。
#![allow(clippy::unwrap_used)]

use crate::testdb;

#[tokio::test]
async fn access_log_migration_applies_twice() {
    let pool = testdb::pool().await;
    let (name, sql) = crate::MIGRATIONS
        .iter()
        .find(|(n, _)| n.ends_with("_access_control"))
        .unwrap();
    // testdb が 1 回当てている。さらに 2 回当てて壊れない
    for _ in 0..2 {
        sqlx::raw_sql(sql)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }

    sqlx::query(
        "INSERT INTO core.access_log (via, credential, route, method, outcome, status)
         VALUES ('direct', 'none', '/access-migration-test', 'GET', 'ok', 200)",
    )
    .execute(&pool)
    .await
    .unwrap();
    for stmt in [
        "UPDATE core.access_log SET route = 'x'",
        "DELETE FROM core.access_log",
        "TRUNCATE core.access_log",
    ] {
        let err = sqlx::query(stmt)
            .execute(&pool)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("読み出しの記録"), "{stmt} が拒まれない: {err}");
    }
}
