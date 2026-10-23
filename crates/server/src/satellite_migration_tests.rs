// SPDX-License-Identifier: AGPL-3.0-only
//! 移行 `satellite_credentials`：衛星の登録簿・資格情報の表と、読み出しの記録の欄（ST29 / design D7 / D9 / D14）。
//! 全版は、試験の中で作った空の DB に当てる（共有の試験の DB には他の試験の行が残るため）。
#![allow(clippy::unwrap_used)]

use crate::testdb;

/// 空の DB に全版を `rounds` 回当てる。
async fn apply_all(pool: &sqlx::PgPool, rounds: usize) {
    for round in 1..=rounds {
        for (label, sql) in crate::MIGRATIONS {
            sqlx::raw_sql(sql)
                .execute(pool)
                .await
                .unwrap_or_else(|e| panic!("{round} 回目の {label}: {e}"));
        }
    }
}

// Scenario: 移行を当て直しても最初の 2 つは増えない
#[tokio::test]
async fn satellite_migration_applied_twice_keeps_two_builtins() {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    apply_all(&fresh, 2).await;
    let rows: Vec<(String, String, bool, bool)> = sqlx::query_as(
        "SELECT identifier, name, builtin, removed_at IS NOT NULL
           FROM core.satellite ORDER BY identifier",
    )
    .fetch_all(&fresh)
    .await
    .unwrap();
    drop.await;
    assert_eq!(
        rows,
        vec![
            ("ashiato.ai-external".into(), "外部 AI".into(), true, false),
            ("ashiato.ai-local".into(), "ローカル AI".into(), true, false),
        ]
    );
}

// Scenario: 移行は最初の 2 つに資格情報を入れない
#[tokio::test]
async fn satellite_migration_seeds_no_credentials() {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    apply_all(&fresh, 1).await;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM core.satellite_credential")
        .fetch_one(&fresh)
        .await
        .unwrap();
    drop.await;
    assert_eq!(n, 0);
}

/// 登録簿の列の約束（D7）と、読み出しの記録の新しい CHECK（D9）を DB が守る。
#[tokio::test]
async fn satellite_migration_constraints_hold() {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    apply_all(&fresh, 1).await;

    let bad_satellite = [
        // 識別子の形・長さ・名前の前後の空白と長さ・最初の 2 つを外す
        "INSERT INTO core.satellite (id, identifier, name, registered_at) VALUES (gen_random_uuid(), 'nodots', 'a', now())",
        "INSERT INTO core.satellite (id, identifier, name, registered_at) VALUES (gen_random_uuid(), 'Dev.Upper', 'a', now())",
        "INSERT INTO core.satellite (id, identifier, name, registered_at) VALUES (gen_random_uuid(), 'dev.x', ' a', now())",
        "INSERT INTO core.satellite (id, identifier, name, registered_at) VALUES (gen_random_uuid(), 'dev.x', '', now())",
        "UPDATE core.satellite SET removed_at = now() WHERE identifier = 'ashiato.ai-local'",
        // 外していない衛星どうしの名前の重なり
        "INSERT INTO core.satellite (id, identifier, name, registered_at) VALUES (gen_random_uuid(), 'dev.x', '外部 AI', now())",
    ];
    for stmt in bad_satellite {
        assert!(sqlx::query(stmt).execute(&fresh).await.is_err(), "{stmt}");
    }

    let sat: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM core.satellite WHERE identifier = 'ashiato.ai-local'")
            .fetch_one(&fresh)
            .await
            .unwrap();
    let cred = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.satellite_credential (id, satellite_id, token_sha256, issued_at)
         VALUES ($1, $2, '\\x00', now())",
    )
    .bind(cred)
    .bind(sat)
    .execute(&fresh)
    .await
    .unwrap();

    let insert = "INSERT INTO core.access_log
        (via, credential, route, method, outcome, status, satellite_id, satellite_credential_id)
        VALUES ('direct', $1, '/x', 'GET', $2, 403, $3, $4)";
    let try_insert = |credential: &'static str,
                      outcome: &'static str,
                      s: Option<uuid::Uuid>,
                      c: Option<uuid::Uuid>| {
        let fresh = fresh.clone();
        async move {
            sqlx::query(insert)
                .bind(credential)
                .bind(outcome)
                .bind(s)
                .bind(c)
                .execute(&fresh)
                .await
        }
    };
    try_insert("satellite", "forbidden", Some(sat), Some(cred))
        .await
        .unwrap();
    try_insert("none", "ok", None, None).await.unwrap();
    // 衛星の番号は衛星の行にだけ、2 つはそろって
    assert!(try_insert("satellite", "forbidden", None, None)
        .await
        .is_err());
    assert!(try_insert("satellite", "forbidden", Some(sat), None)
        .await
        .is_err());
    assert!(try_insert("api_token", "ok", Some(sat), Some(cred))
        .await
        .is_err());
    assert!(try_insert("satellite", "teapot", Some(sat), Some(cred))
        .await
        .is_err());

    // 衛星の値を持つ行も書き換えも削除もできない
    for stmt in [
        "UPDATE core.access_log SET status = 200 WHERE credential = 'satellite'",
        "DELETE FROM core.access_log WHERE credential = 'satellite'",
    ] {
        let err = sqlx::query(stmt)
            .execute(&fresh)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("読み出しの記録"), "{stmt}: {err}");
    }
    drop.await;
}

/// 行は消せない（DELETE と TRUNCATE）。UPDATE（外す・取り消す）は通る。
#[tokio::test]
async fn satellite_migration_rows_cannot_be_deleted() {
    let pool = testdb::pool().await;
    // 行が無いと行トリガは撃たれないので、衛星と資格情報を 1 つずつ置く（試験ごとの識別子。行は残る）
    let sat = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.satellite (id, identifier, name, registered_at) VALUES ($1, $2, $3, now())",
    )
    .bind(sat)
    .bind(format!("dev.t{}.booklog", sat.simple()))
    .bind(format!("t{}", sat.simple()))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.satellite_credential (id, satellite_id, token_sha256, issued_at)
         VALUES ($1, $2, $3, now())",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(sat)
    .bind(sat.as_bytes().to_vec())
    .execute(&pool)
    .await
    .unwrap();
    for stmt in [
        "DELETE FROM core.satellite",
        "TRUNCATE core.satellite CASCADE",
        "DELETE FROM core.satellite_credential",
        "TRUNCATE core.satellite_credential",
    ] {
        let err = sqlx::query(stmt)
            .execute(&pool)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("消せない"), "{stmt} が拒まれない: {err}");
    }
    // 外す・取り消すは UPDATE で通る（この試験の行だけ。共有の DB の他の行は触らない）
    sqlx::query("UPDATE core.satellite SET removed_at = now() WHERE id = $1")
        .bind(sat)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE core.satellite_credential SET revoked_at = now() WHERE satellite_id = $1")
        .bind(sat)
        .execute(&pool)
        .await
        .unwrap();
}
