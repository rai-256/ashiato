// SPDX-License-Identifier: AGPL-3.0-only
//! 打ち切られた回の残りを、次の回の片付けが消すことを `tools/check-test-db-killed.sh` が確かめるための探り（CH05 / design D2）。
//!
//! `ch05_killed_probe` は `#[ignore]`。台本が環境変数 `CH05_PROBE_AT` と `CH05_PROBE_MARK` を渡して起こし、
//! 印のファイルで同期してから `kill -KILL` する。環境変数が無ければ何もせずに通る。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use std::io::Write;

/// 体積の試験（`deletion_tests.rs` の `stays_detail_query_uses_user_time_index`）と同じ形の 20,001 件の INSERT。
/// `lead` は FROM の先頭に置く句（眠りを前に置くとき `pg_sleep(600),`）。
fn insert_sql(lead: &str) -> String {
    format!(
        "/* ch05_probe */ INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         SELECT gen_random_uuid(), $1, $2, 'test', 'collected',
                '2026-09-28T00:00:00+00:00'::timestamptz + make_interval(mins => i),
                0, 'UTC', 1, gen_random_uuid()::text, '{{}}', '{{}}'::jsonb
           FROM {lead} generate_series(0, 20_000) AS i"
    )
}

/// 印のファイルへ 1 行足す（台本が読む）。
fn mark(path: &str, line: &str) {
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    writeln!(f, "{line}").unwrap();
    f.sync_all().unwrap();
}

async fn current_database(pool: &sqlx::PgPool) -> String {
    sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn insert_rows(pool: &sqlx::PgPool, lead: &str, prefix: &str) {
    let source = testdb::source(pool, prefix, 21_600).await;
    sqlx::query(&insert_sql(lead))
        .bind(testdb::user())
        .bind(&source)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "tools/check-test-db-killed.sh が起こす。単独では走らせない"]
async fn ch05_killed_probe() {
    let (Ok(at), Ok(path)) = (
        std::env::var("CH05_PROBE_AT"),
        std::env::var("CH05_PROBE_MARK"),
    ) else {
        return;
    };
    // 値の確かめは回の database を作る前に（誤った値で回の database だけが残らないように）
    assert!(
        matches!(at.as_str(), "insert" | "committed"),
        "CH05_PROBE_AT は insert か committed: {at:?}"
    );
    let run = testdb::pool().await;
    let iso = testdb::isolated().await;
    let iso_pool = iso.pool().await;
    mark(&path, &format!("run={}", current_database(&run).await));
    mark(&path, &format!("iso={}", current_database(&iso_pool).await));
    match at.as_str() {
        "insert" => {
            mark(&path, "inserting");
            insert_rows(&run, "pg_sleep(600),", "ch05-probe").await;
        }
        "committed" => {
            insert_rows(&run, "", "ch05-probe").await;
            insert_rows(&iso_pool, "", "ch05-probe").await;
            mark(&path, "committed");
            tokio::time::sleep(std::time::Duration::from_secs(600)).await;
        }
        other => unreachable!("CH05_PROBE_AT は先に確かめた: {other:?}"),
    }
}

/// 次の回。繋ぐだけで、CH04 の片付けが前の回の残りを消す。
#[tokio::test]
async fn ch05_next_run() {
    let pool = testdb::pool().await;
    let one: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(one, 1);
}
