// SPDX-License-Identifier: AGPL-3.0-only
//! 本文ごと消す操作の移行（台帳・範囲・派生の門・ビュー）を、**本物の PostgreSQL に対して**確かめる
//! （ST23 / design D4 / D5 / D7）。門は COMMIT の時に落ちるので、拒否は commit の結果で見る。
//!
//! 追記のみの表に行を置くと消せない。拒否の試験は、行を置いたトランザクションごと巻き戻して試す。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use sqlx::PgPool;
use uuid::Uuid;

const MIGRATION_DOWN: &str = include_str!("../../../migrations/202610240500_record_purge.down.sql");

/// 表の列（名前, 型, NULL 可）を `information_schema` から読む。順は定義の順。
async fn columns(pool: &PgPool, table: &str) -> Vec<(String, String, bool)> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT column_name::text, udt_name::text, is_nullable::text
           FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = $1
          ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .unwrap();
    rows.into_iter()
        .map(|(n, t, nullable)| (n, t, nullable == "YES"))
        .collect()
}

/// 表の CHECK（`NOT NULL` を除く）の式を `information_schema` から読む。
async fn checks(pool: &PgPool, table: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT cc.check_clause::text
           FROM information_schema.check_constraints cc
           JOIN information_schema.table_constraints tc
             ON tc.constraint_schema = cc.constraint_schema AND tc.constraint_name = cc.constraint_name
          WHERE tc.table_schema = 'core' AND tc.table_name = $1
            AND cc.check_clause NOT LIKE '%IS NOT NULL'
          ORDER BY cc.check_clause",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .unwrap()
}

fn shape(spec: &[(&str, &str, bool)]) -> Vec<(String, String, bool)> {
    spec.iter()
        .map(|(n, t, nullable)| ((*n).to_string(), (*t).to_string(), *nullable))
        .collect()
}

// 行の形は固定の値で試験する（R6）。列を足したり型を変えたりしたら、ここが落ちる。
#[tokio::test]
async fn purge_schema_operation_row_shape_is_fixed() {
    let pool = testdb::pool().await;
    assert_eq!(
        columns(&pool, "erasure_operation").await,
        shape(&[
            ("id", "uuid", false),
            ("user_id", "uuid", false),
            ("kind", "text", false),
            ("parent_id", "uuid", true),
            ("stay_id", "uuid", false),
            ("start_at", "timestamptz", false),
            ("end_at", "timestamptz", false),
            ("stays", "int4", false),
            ("locations", "int4", false),
            ("archive_locations", "int4", false),
            ("archive_items", "int4", false),
            ("versions", "int4", false),
            ("at", "timestamptz", false),
            ("txid", "xid8", false),
        ])
    );
    let mut want = vec![
        "(kind = ANY (ARRAY['user'::text, 'late'::text]))",
        "(stays >= 0)",
        "(locations >= 0)",
        "(archive_locations >= 0)",
        "(archive_items >= 0)",
        "(versions >= 0)",
    ];
    want.sort_unstable();
    assert_eq!(checks(&pool, "erasure_operation").await, want);
}

#[tokio::test]
async fn purge_schema_span_row_shape_is_fixed() {
    let pool = testdb::pool().await;
    assert_eq!(
        columns(&pool, "erasure_span").await,
        shape(&[
            ("operation_id", "uuid", false),
            ("event_id", "uuid", false),
            ("scope", "text", false),
            ("version_no", "int4", true),
            ("start_at", "timestamptz", false),
            ("end_at", "timestamptz", false),
        ])
    );
    let mut want = vec![
        "(scope = ANY (ARRAY['event'::text, 'version'::text]))",
        "(end_at >= start_at)",
    ];
    want.sort_unstable();
    assert_eq!(checks(&pool, "erasure_span").await, want);
}

#[tokio::test]
async fn purge_schema_compaction_row_shape_is_fixed() {
    let pool = testdb::pool().await;
    assert_eq!(
        columns(&pool, "purge_compaction").await,
        shape(&[
            ("id", "int8", false),
            ("table_name", "text", false),
            ("started_at", "timestamptz", false),
            ("finished_at", "timestamptz", false),
            ("outcome", "text", false),
            ("sqlstate", "text", true),
            ("snapshot", "pg_snapshot", true),
        ])
    );
    let mut want = vec![
        "(outcome = ANY (ARRAY['rewritten'::text, 'not_rewritten'::text, 'lock_timeout'::text, 'failed'::text]))",
        "(table_name = ANY (ARRAY['core.event'::text, 'core.event_version'::text]))",
    ];
    want.sort_unstable();
    assert_eq!(checks(&pool, "purge_compaction").await, want);
}

/// 記録ごとの台帳の `operation_id` は NULL を許し、既存の行は NULL のまま。
#[tokio::test]
async fn purge_schema_ledger_operation_id_is_nullable() {
    let pool = testdb::pool().await;
    let cols = columns(&pool, "erasure_ledger").await;
    let col = cols.iter().find(|(n, _, _)| n == "operation_id").unwrap();
    assert_eq!((col.1.as_str(), col.2), ("uuid", true));
    let set: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.erasure_ledger
          WHERE operation_id IS NOT NULL AND erased_at < '2026-10-24'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(set, 0, "移行より前の台帳の行に operation_id が入っている");
}

// ------------------------------------------------------------------ 足場

/// 操作の行 1 つ（件数は 0）。
async fn put_operation(tx: &mut sqlx::PgConnection, id: Uuid, user: Uuid) {
    sqlx::query(
        "INSERT INTO core.erasure_operation
           (id, user_id, kind, stay_id, start_at, end_at,
            stays, locations, archive_locations, archive_items, versions)
         VALUES ($1, $2, 'user', $1, '2026-10-01T00:00:00Z', '2026-10-01T01:00:00Z', 0, 0, 0, 0, 0)",
    )
    .bind(id)
    .bind(user)
    .execute(tx)
    .await
    .unwrap();
}

async fn put_span(tx: &mut sqlx::PgConnection, op: Uuid, event: Uuid) {
    sqlx::query(
        "INSERT INTO core.erasure_span (operation_id, event_id, scope, start_at, end_at)
         VALUES ($1, $2, 'event', '2026-10-01T00:00:00Z', '2026-10-01T01:00:00Z')",
    )
    .bind(op)
    .bind(event)
    .execute(tx)
    .await
    .unwrap();
}

/// 失敗は SQL の文字列に「追記のみ」を含む（拒否の理由が台帳であること）。
fn assert_append_only_error(e: &sqlx::Error, what: &str) {
    let msg = e
        .as_database_error()
        .map(|d| d.message().to_string())
        .unwrap_or_default();
    assert!(
        msg.contains("追記のみ"),
        "{what} が追記のみで拒まれない: {e}"
    );
}

// Scenario: 消去の操作の台帳は書き換えられない
#[tokio::test]
async fn purge_schema_ledger_rejects_update() {
    let pool = testdb::pool().await;
    for (table, set, key) in [
        ("erasure_operation", "stays = 1", "id"),
        (
            "erasure_span",
            "end_at = end_at + interval '1 second'",
            "operation_id",
        ),
    ] {
        let (op, event) = (Uuid::new_v4(), Uuid::new_v4());
        let mut tx = pool.begin().await.unwrap();
        put_operation(&mut tx, op, Uuid::new_v4()).await;
        put_span(&mut tx, op, event).await;
        let e = sqlx::query(&format!("UPDATE core.{table} SET {set} WHERE {key} = $1"))
            .bind(op)
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_append_only_error(&e, &format!("{table} の書き換え"));
        tx.rollback().await.unwrap();
    }
}

// Scenario: 消去の操作の台帳は消せず、表も切り詰められない
#[tokio::test]
async fn purge_schema_ledger_rejects_delete_and_truncate() {
    let pool = testdb::pool().await;
    for table in ["erasure_operation", "erasure_span"] {
        let (op, event) = (Uuid::new_v4(), Uuid::new_v4());
        let key = if table == "erasure_operation" {
            "id"
        } else {
            "operation_id"
        };

        let mut tx = pool.begin().await.unwrap();
        put_operation(&mut tx, op, Uuid::new_v4()).await;
        put_span(&mut tx, op, event).await;
        let e = sqlx::query(&format!("DELETE FROM core.{table} WHERE {key} = $1"))
            .bind(op)
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_append_only_error(&e, &format!("{table} の削除"));
        tx.rollback().await.unwrap();

        // TRUNCATE は表の錠を取ってから文トリガが撃たれる。並走する書き出しの読み出しが錠を持っていると
        // 待たされるので、錠の待ちで切れたときだけ撃ち直す（拒否の理由は追記のみであること）
        let mut tries = 0;
        let e = loop {
            let mut tx = pool.begin().await.unwrap();
            sqlx::query("SET LOCAL lock_timeout = '5s'")
                .execute(&mut *tx)
                .await
                .unwrap();
            let e = sqlx::query(&format!("TRUNCATE core.{table}"))
                .execute(&mut *tx)
                .await
                .unwrap_err();
            tx.rollback().await.unwrap();
            let waited = e
                .as_database_error()
                .and_then(|d| d.code())
                .is_some_and(|c| c == "55P03");
            tries += 1;
            if !waited || tries >= 12 {
                break e;
            }
        };
        // 切り詰めを拒む文トリガが有効に置かれ、BEFORE（`tgtype & 2`）で拒む関数を呼ぶこと。錠が取れたときも見る
        // （錠が取れないときの代わりの確かめが、いつも走って壊れていないことを保つ。final review R12）
        let armed: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_trigger
              WHERE tgrelid = format('core.%I', $1::text)::regclass
                AND tgenabled = 'O' AND (tgtype & 32) <> 0 AND (tgtype & 2) <> 0
                AND tgfoid = format('core.reject_%s_change', $1::text)::regproc",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            armed >= 1,
            "{table} に切り詰めを BEFORE で拒む文トリガが無い"
        );
        let still_waiting = e
            .as_database_error()
            .and_then(|d| d.code())
            .is_some_and(|c| c == "55P03");
        if still_waiting {
            // 全体の試験の最中は、書き出しの長い読み出しが錠を数分持つことがある。錠が取れないなら、上の文トリガの確かめで代える
            continue;
        }
        assert_append_only_error(&e, &format!("{table} の切り詰め"));
    }
}

/// 派生の滞在 1 件（本文あり）。ほかの試験と被らない利用者に置く。
async fn put_stay(pool: &PgPool, user: Uuid, raw: &str, payload: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1, $2, 's01-stay', 'test', 'derived', '2026-10-01T00:00:00Z',
                 540, 'Asia/Tokyo', 1, $3, $4, $5::jsonb)",
    )
    .bind(id)
    .bind(user)
    .bind(Uuid::new_v4().to_string())
    .bind(raw)
    .bind(payload)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn put_stay_version(pool: &PgPool, id: Uuid, user: Uuid) {
    sqlx::query(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
         SELECT id, user_id, logical_source, 1, event_time, content_hash, raw, payload
           FROM core.event WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user)
    .execute(pool)
    .await
    .unwrap();
}

/// 消去の台帳の行（滞在 1 件分）。
async fn put_ledger(tx: &mut sqlx::PgConnection, id: Uuid, user: Uuid) {
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, 's01-stay', 'event', 'purge_tests')",
    )
    .bind(id)
    .bind(user)
    .execute(tx)
    .await
    .unwrap();
}

const ERASE: &str = "UPDATE core.event SET raw = '', payload = '{}'::jsonb WHERE id = $1";

/// 1 つのトランザクションで（台帳 →）書き換えを撃って commit する。門は commit で落ちる。
async fn commit_update(
    pool: &PgPool,
    id: Uuid,
    user: Uuid,
    ledger: bool,
    sql: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    if ledger {
        put_ledger(&mut tx, id, user).await;
    }
    sqlx::query(sql).bind(id).execute(&mut *tx).await?;
    tx.commit().await
}

async fn raw_of(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar("SELECT raw FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

// Scenario: 台帳を書かない滞在の本文の消去は拒まれる
#[tokio::test]
async fn purge_schema_gate_rejects_erasure_without_ledger() {
    let pool = testdb::pool().await;
    let user = Uuid::new_v4();
    let id = put_stay(&pool, user, r#"{"k":1}"#, r#"{"k":1}"#).await;
    let e = commit_update(&pool, id, user, false, ERASE)
        .await
        .unwrap_err();
    assert!(
        e.to_string().contains("消去の台帳"),
        "理由が台帳でない: {e}"
    );
    assert_eq!(raw_of(&pool, id).await, r#"{"k":1}"#, "本文が消えている");
}

// Scenario: 前の版を残した滞在の本文の消去は拒まれる
#[tokio::test]
async fn purge_schema_gate_rejects_erasure_leaving_a_version() {
    let pool = testdb::pool().await;
    let user = Uuid::new_v4();
    let id = put_stay(&pool, user, r#"{"k":1}"#, r#"{"k":1}"#).await;
    put_stay_version(&pool, id, user).await;
    let e = commit_update(&pool, id, user, true, ERASE)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("前の版"), "理由が前の版でない: {e}");
    assert_eq!(raw_of(&pool, id).await, r#"{"k":1}"#, "本文が消えている");
}

// Scenario: 本文を消した滞在へ本文を書き戻せない
#[tokio::test]
async fn purge_schema_gate_rejects_write_back() {
    let pool = testdb::pool().await;
    let user = Uuid::new_v4();
    let id = put_stay(&pool, user, r#"{"k":1}"#, r#"{"k":1}"#).await;
    // 台帳つきの消去は通る（前の版が無いので）
    commit_update(&pool, id, user, true, ERASE).await.unwrap();
    assert_eq!(raw_of(&pool, id).await, "");
    let e = commit_update(
        &pool,
        id,
        user,
        false,
        r#"UPDATE core.event SET raw = '{"k":1}' WHERE id = $1"#,
    )
    .await
    .unwrap_err();
    assert!(
        e.to_string().contains("書き戻せない"),
        "理由が書き戻しでない: {e}"
    );
    assert_eq!(raw_of(&pool, id).await, "", "原文が空でなくなった");
    // 台帳の行を添えても通らない
    let e = commit_update(
        &pool,
        id,
        user,
        true,
        r#"UPDATE core.event SET raw = '{"k":1}' WHERE id = $1"#,
    )
    .await
    .unwrap_err();
    assert!(
        e.to_string().contains("書き戻せない"),
        "台帳つきでも通った: {e}"
    );
    assert_eq!(raw_of(&pool, id).await, "");
}

// Scenario: 本文を消した滞在へ本文は書き戻せない（原文を空のまま解析済みだけ差し替える形）
#[tokio::test]
async fn purge_schema_gate_rejects_payload_write_back_on_erased_stay() {
    let pool = testdb::pool().await;
    let user = Uuid::new_v4();
    let id = put_stay(&pool, user, r#"{"k":1}"#, r#"{"k":1}"#).await;
    commit_update(&pool, id, user, true, ERASE).await.unwrap();
    for ledger in [false, true] {
        let e = commit_update(
            &pool,
            id,
            user,
            ledger,
            r#"UPDATE core.event SET payload = '{"k":2}'::jsonb WHERE id = $1"#,
        )
        .await
        .unwrap_err();
        assert!(
            e.to_string().contains("書き戻せない"),
            "理由が書き戻しでない（台帳 {ledger}）: {e}"
        );
    }
    let payload: String = sqlx::query_scalar("SELECT payload::text FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(payload, "{}", "解析済みに本文が戻った");
}

// Scenario: 解析済みを残して原文だけ空にする書き換えは拒まれる
#[tokio::test]
async fn purge_schema_gate_rejects_raw_only_erasure() {
    let pool = testdb::pool().await;
    let user = Uuid::new_v4();
    let id = put_stay(&pool, user, r#"{"k":1}"#, r#"{"k":1}"#).await;
    let e = commit_update(
        &pool,
        id,
        user,
        true,
        "UPDATE core.event SET raw = '' WHERE id = $1",
    )
    .await
    .unwrap_err();
    assert!(
        e.to_string().contains("一緒に空にする"),
        "理由が消去の形でない: {e}"
    );
    assert_eq!(raw_of(&pool, id).await, r#"{"k":1}"#, "本文が消えている");
}

// Scenario: 作り直しの書き換えは今までどおり通る
#[tokio::test]
async fn purge_schema_gate_lets_a_rebuild_through() {
    let pool = testdb::pool().await;
    let user = Uuid::new_v4();
    let id = put_stay(
        &pool,
        user,
        r#"{"end":"2026-10-01T00:20:00Z"}"#,
        r#"{"end":"2026-10-01T00:20:00Z"}"#,
    )
    .await;
    // 作り直しは前の版を積んでから、終わりを延ばして書き換える（台帳は要らない）
    put_stay_version(&pool, id, user).await;
    commit_update(
        &pool,
        id,
        user,
        false,
        r#"UPDATE core.event
              SET raw = '{"end":"2026-10-01T01:00:00Z"}',
                  payload = '{"end":"2026-10-01T01:00:00Z"}'::jsonb
            WHERE id = $1"#,
    )
    .await
    .unwrap();
    assert_eq!(raw_of(&pool, id).await, r#"{"end":"2026-10-01T01:00:00Z"}"#);
    let versions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.event_version WHERE event_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(versions, 1, "前の版が 1 件でない");
}

// ------------------------------------------------------------------ 戻し手順

/// 操作の行があると、戻し手順は先頭で止まる（本文を消した後に戻すと範囲が 1µs に縮む）。
/// 巻き戻すトランザクションの中で撃つ。止まらなければ表が落ちるが、それも巻き戻る。
#[tokio::test]
async fn purge_schema_down_stops_when_operations_exist() {
    let pool = testdb::pool().await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL lock_timeout = '10s'")
        .execute(&mut *tx)
        .await
        .unwrap();
    put_operation(&mut tx, Uuid::new_v4(), Uuid::new_v4()).await;
    let e = sqlx::raw_sql(MIGRATION_DOWN)
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert!(
        e.to_string().contains("戻せない"),
        "戻し手順が行の存在で止まらない: {e}"
    );
    tx.rollback().await.unwrap();
    let still: bool =
        sqlx::query_scalar("SELECT to_regclass('core.erasure_operation') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(still, "戻し手順が表を落とした");
}

// ------------------------------------------------------------------ 消した時間帯のビュー

/// 全部の版を当て直しても、`core.stay_erased` は ST41 までの 6 列のまま範囲の表を読む側（`core.stay_erased_purge` の写し）に戻る。
/// 列を足すと、当て直しで ST41 の `CREATE OR REPLACE VIEW` が「cannot drop columns from view」で落ちた（merge gate）。
#[tokio::test]
async fn purge_schema_stay_erased_survives_reapplying_every_version() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    for (name, sql) in crate::MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("2 回目の {name}: {e}"));
    }
    let names: Vec<String> = columns(&pool, "stay_erased")
        .await
        .into_iter()
        .map(|(n, _, _)| n)
        .collect();
    assert_eq!(
        names,
        [
            "id",
            "user_id",
            "start_at",
            "end_at",
            "deleted_at",
            "deleted_by"
        ],
        "core.stay_erased の列が ST41 の形から変わった"
    );
    let def: String = sqlx::query_scalar("SELECT pg_get_viewdef('core.stay_erased'::regclass)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        def.contains("stay_erased_purge"),
        "当て直しの後の core.stay_erased が範囲の表を読む側でない: {def}"
    );
}

/// ST22 の消した滞在の範囲は、移行の前後で変わらない（範囲の行が無いものは本文の読み）。
#[tokio::test]
async fn purge_schema_stay_erased_range_is_unchanged_for_st22_rows() {
    let pool = testdb::pool().await;
    let user = Uuid::new_v4();
    for (payload, end) in [
        (r#"{"end":"2026-10-01T00:45:00Z"}"#, "2026-10-01T00:45:00Z"),
        (r#"{"end":"broken"}"#, "2026-10-01T00:00:00.000001Z"),
        (r#"{"x":1}"#, "2026-10-01T00:00:00.000001Z"),
    ] {
        let id = put_stay(&pool, user, payload, payload).await;
        sqlx::query(
            "UPDATE core.event SET deleted_at = now(), deleted_by = 'user:erase' WHERE id = $1",
        )
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
        let (start_ok, end_ok, purged, op): (bool, bool, bool, Option<Uuid>) = sqlx::query_as(
            "SELECT start_at = '2026-10-01T00:00:00Z'::timestamptz,
                    end_at = $2::timestamptz, purged, operation_id
               FROM core.stay_erased_purge WHERE id = $1",
        )
        .bind(id)
        .bind(end)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(start_ok && end_ok, "{payload} の範囲が変わった");
        assert!(!purged && op.is_none(), "本文を消していない行が purged");
    }
}

/// 本文を消した滞在は範囲の表の時間を返し、一緒に消した滞在は出さない。
#[tokio::test]
async fn purge_schema_stay_erased_reads_the_span_and_hides_absorbed() {
    let pool = testdb::pool().await;
    let user = Uuid::new_v4();
    let id = put_stay(
        &pool,
        user,
        r#"{"end":"2026-10-01T00:10:00Z"}"#,
        r#"{"end":"2026-10-01T00:10:00Z"}"#,
    )
    .await;
    let op = Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    put_operation(&mut tx, op, user).await;
    put_span(&mut tx, op, id).await;
    put_ledger(&mut tx, id, user).await;
    sqlx::query(
        "UPDATE core.event SET raw = '', payload = '{}'::jsonb,
                deleted_at = now(), deleted_by = 'user:erase' WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let (end_ok, purged, got_op): (bool, bool, Option<Uuid>) = sqlx::query_as(
        "SELECT end_at = '2026-10-01T01:00:00Z'::timestamptz, purged, operation_id
           FROM core.stay_erased_purge WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(end_ok, "範囲の表の終わりを読んでいない");
    assert!(purged);
    assert_eq!(got_op, Some(op));

    // 重なって一緒に本文を消した滞在（user:purged）はビューに出ない
    let absorbed = put_stay(&pool, user, r#"{"k":2}"#, r#"{"k":2}"#).await;
    let mut tx = pool.begin().await.unwrap();
    put_ledger(&mut tx, absorbed, user).await;
    sqlx::query(
        "UPDATE core.event SET raw = '', payload = '{}'::jsonb,
                deleted_at = now(), deleted_by = 'user:purged' WHERE id = $1",
    )
    .bind(absorbed)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let seen: i64 = sqlx::query_scalar("SELECT count(*) FROM core.stay_erased WHERE id = $1")
        .bind(absorbed)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(seen, 0, "user:purged の滞在がビューに出ている");
}

// ------------------------------------------------------------------ 本文ごと消す範囲と消去の核（Task 2 / design D2 / D3 / D6 / D15 / D17）

use crate::deletion::{self, PurgeCounts, PurgeError};
use chrono::{DateTime, Duration, Utc};

pub(crate) fn at(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

/// 派生の滞在 1 件（本文あり。時間は `payload` の `start` / `end`）。
pub(crate) async fn scene_stay(pool: &PgPool, user: Uuid, start: &str, end: &str) -> Uuid {
    let id = Uuid::new_v4();
    let body = serde_json::json!({
        "start": at(start).to_rfc3339(), "end": at(end).to_rfc3339(), "lat": 35.68, "lon": 139.76,
    })
    .to_string();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1, $2, 's01-stay', $3, 'derived', $4, 540, 'Asia/Tokyo', 1, $3, $5, $5::jsonb)",
    )
    .bind(id)
    .bind(user)
    .bind(id.to_string())
    .bind(at(start))
    .bind(body)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// 収集した記録 1 件（論理ソースと原文を指す）。
pub(crate) async fn scene_event(
    pool: &PgPool,
    user: Uuid,
    source: &str,
    when: &str,
    raw: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1, $2, $3, 'test', 'collected', $4, 540, 'Asia/Tokyo', 1, $5, $6, $6::jsonb)",
    )
    .bind(id)
    .bind(user)
    .bind(source)
    .bind(at(when))
    .bind(Uuid::new_v4().to_string())
    .bind(raw)
    .execute(pool)
    .await
    .unwrap();
    id
}

const POINT: &str = r#"{"lat":35.68,"lon":139.76,"acc_m":10}"#;

pub(crate) async fn scene_location(pool: &PgPool, user: Uuid, when: &str) -> Uuid {
    scene_event(pool, user, "c01-location", when, POINT).await
}

/// 前の版 1 件（滞在。時間は `start` / `end`）。
pub(crate) async fn scene_version(
    pool: &PgPool,
    stay: Uuid,
    user: Uuid,
    no: i32,
    start: &str,
    end: &str,
) {
    let body = serde_json::json!({
        "start": at(start).to_rfc3339(), "end": at(end).to_rfc3339(), "lat": 35.68, "lon": 139.76,
    })
    .to_string();
    sqlx::query(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
         VALUES ($1, $2, 's01-stay', $3, $4, $5, $6, $6::jsonb)",
    )
    .bind(stay)
    .bind(user)
    .bind(no)
    .bind(at(start))
    .bind(Uuid::new_v4().to_string())
    .bind(body)
    .execute(pool)
    .await
    .unwrap();
}

/// 作り直しの印で隠れた滞在にする（`rebuild:absorbed` など）。
pub(crate) async fn scene_mark_rebuild(pool: &PgPool, id: Uuid, mark: &str) {
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = $2 WHERE id = $1")
        .bind(id)
        .bind(mark)
        .execute(pool)
        .await
        .unwrap();
}

/// 原文と解析済みの**両方**を DB から読み、どちらも空か。
pub(crate) async fn emptied(pool: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar("SELECT raw = '' AND payload = '{}'::jsonb FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// 原文も解析済みも残っているか。
pub(crate) async fn kept(pool: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar("SELECT raw <> '' AND payload <> '{}'::jsonb FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn versions_emptied(pool: &PgPool, id: Uuid) -> (i64, i64) {
    sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE raw = '' AND payload = '{}'::jsonb)
           FROM core.event_version WHERE event_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// 「消した」にして、確認と同じ数え方で数えた件数を添えて本文ごと消す。
pub(crate) async fn purge_now(pool: &PgPool, stay: Uuid) -> deletion::PurgeOutcome {
    let counts = scope_counts(pool, stay).await;
    deletion::purge(pool, stay, None, counts).await.unwrap()
}

pub(crate) async fn scope_counts(pool: &PgPool, stay: Uuid) -> PurgeCounts {
    let mut tx = pool.begin().await.unwrap();
    let scope = deletion::purge_scope(&mut tx, stay).await.unwrap();
    tx.rollback().await.unwrap();
    scope.counts
}

/// 滞在を ST22 の口で消す（その時間の位置が一緒に消える）。
pub(crate) async fn scene_erase(pool: &PgPool, stay: Uuid) {
    deletion::erase(pool, stay, None).await.unwrap();
}

/// 10:00–11:00 の滞在と、その中の位置 3 件。
pub(crate) async fn scene_basic(pool: &PgPool, user: Uuid) -> (Uuid, Vec<Uuid>) {
    let stay = scene_stay(
        pool,
        user,
        "2026-10-01T10:00:00+09:00",
        "2026-10-01T11:00:00+09:00",
    )
    .await;
    let mut locations = Vec::new();
    for when in ["10:00", "10:30", "11:00"] {
        locations.push(scene_location(pool, user, &format!("2026-10-01T{when}:00+09:00")).await);
    }
    (stay, locations)
}

// Scenario: 一緒に消えた位置の本文も消える
#[tokio::test]
async fn purge_scope_cascaded_locations_are_emptied() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, locations) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;

    purge_now(&pool, stay).await;

    assert!(emptied(&pool, stay).await);
    assert_eq!(locations.len(), 3);
    for id in locations {
        assert!(emptied(&pool, id).await);
    }
}

// Scenario: 位置を持つ書庫の項目は項目ごと本文が消える
#[tokio::test]
async fn purge_scope_archive_item_with_location_is_emptied_whole() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    let item = scene_event(
        &pool,
        user,
        "c03-youtube-watch",
        "2026-10-01T10:20:00+09:00",
        r#"{"title":"視聴","locationInfos":[{"name":"東京"}]}"#,
    )
    .await;
    scene_erase(&pool, stay).await;

    let outcome = purge_now(&pool, stay).await;

    assert_eq!(outcome.counts.archive_items, 1);
    assert!(emptied(&pool, item).await, "位置の欄だけでなく全体が空");
}

/// 書庫の位置は「書庫の位置」の組に数え、端末の位置の組（`locations`）には数えない（final review R4）。
/// `purge_scope` の組の判定の順を入れ替えると、確認と消すが同じ関数なので `CountsChanged` も起きずに件数がずれる。
#[tokio::test]
async fn purge_scope_archive_location_is_counted_in_its_own_group() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let stay = scene_stay(
        &pool,
        user,
        "2026-10-01T10:00:00+09:00",
        "2026-10-01T11:00:00+09:00",
    )
    .await;
    let source = "c03-legacy-location";
    assert!(crate::archive::LOCATION_SOURCES.contains(&source));
    let archived = scene_event(&pool, user, source, "2026-10-01T10:20:00+09:00", POINT).await;
    scene_erase(&pool, stay).await;

    let outcome = purge_now(&pool, stay).await;

    assert_eq!(outcome.counts.archive_locations, 1, "書庫の位置の組");
    assert_eq!(outcome.counts.locations, 0, "端末の位置の組に数えない");
    assert!(emptied(&pool, archived).await);
}

// Scenario: 前の版の本文も消える
#[tokio::test]
async fn purge_scope_versions_are_emptied() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    for no in 1..=20 {
        scene_version(
            &pool,
            stay,
            user,
            no,
            "2026-10-01T10:00:00+09:00",
            "2026-10-01T10:50:00+09:00",
        )
        .await;
    }
    scene_erase(&pool, stay).await;

    let outcome = purge_now(&pool, stay).await;

    assert_eq!(outcome.counts.versions, 20);
    assert_eq!(versions_emptied(&pool, stay).await, (20, 20));
}

// Scenario: 吸収された滞在の本文も消える
#[tokio::test]
async fn purge_scope_absorbed_stay_is_emptied_with_its_versions() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    let absorbed = scene_stay(
        &pool,
        user,
        "2026-10-01T10:10:00+09:00",
        "2026-10-01T10:40:00+09:00",
    )
    .await;
    scene_version(
        &pool,
        absorbed,
        user,
        1,
        "2026-10-01T10:10:00+09:00",
        "2026-10-01T10:30:00+09:00",
    )
    .await;
    scene_erase(&pool, stay).await;
    scene_mark_rebuild(&pool, absorbed, "rebuild:absorbed").await;

    let outcome = purge_now(&pool, stay).await;

    assert_eq!(outcome.counts.stays, 2);
    assert!(emptied(&pool, absorbed).await);
    assert_eq!(versions_emptied(&pool, absorbed).await, (1, 1));
    // 作り直しが印を外さないように、本人の「本文ごと消した」に付け替わり、台帳に原因つきで 1 行残る（D6）
    let mark: Option<String> =
        sqlx::query_scalar("SELECT deleted_by FROM core.event WHERE id = $1")
            .bind(absorbed)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(mark.as_deref(), Some("user:purged"));
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.deletion_ledger
          WHERE event_id = $1 AND action = 'erase' AND cause_event_id = $2 AND mark = 'user:purged'",
    )
    .bind(absorbed)
    .bind(stay)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rows, 1);
    let own: Option<String> = sqlx::query_scalar("SELECT deleted_by FROM core.event WHERE id = $1")
        .bind(stay)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        own.as_deref(),
        Some("user"),
        "指定した滞在の印は本人の印のまま"
    );
}

// Scenario: 同じ時間の写真の本文は残る
#[tokio::test]
async fn purge_scope_keeps_photo() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    let photo = scene_event(
        &pool,
        user,
        "c01-photo",
        "2026-10-01T10:30:00+09:00",
        r#"{"lat":35.68,"lon":139.76,"file":"a.jpg"}"#,
    )
    .await;
    scene_erase(&pool, stay).await;

    purge_now(&pool, stay).await;

    assert!(kept(&pool, photo).await);
}

// Scenario: 同じ時間の PC のウィンドウの本文は残る
#[tokio::test]
async fn purge_scope_keeps_pc_window_and_browser_history() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    let window = scene_event(
        &pool,
        user,
        "c02-window",
        "2026-10-01T10:30:00+09:00",
        r#"{"title":"w"}"#,
    )
    .await;
    let history = scene_event(
        &pool,
        user,
        "c02-browser-history",
        "2026-10-01T10:31:00+09:00",
        r#"{"url":"https://example.test/"}"#,
    )
    .await;
    scene_erase(&pool, stay).await;

    purge_now(&pool, stay).await;

    assert!(kept(&pool, window).await);
    assert!(kept(&pool, history).await);
}

// Scenario: 消した滞在を指して書いた気分は残る
#[tokio::test]
async fn purge_scope_keeps_feeling_pointing_at_the_stay() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    let feeling = scene_event(
        &pool,
        user,
        "s01-feeling",
        "2026-10-01T11:10:00+09:00",
        &format!(r#"{{"mood":"good","stay_id":"{stay}"}}"#),
    )
    .await;
    scene_erase(&pool, stay).await;

    purge_now(&pool, stay).await;

    assert!(kept(&pool, feeling).await);
}

// Scenario: 別の操作で消した位置の本文は消えない
#[tokio::test]
async fn purge_scope_keeps_locations_erased_by_another_stay() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (first, _) = scene_basic(&pool, user).await;
    let second = scene_stay(
        &pool,
        user,
        "2026-10-01T14:00:00+09:00",
        "2026-10-01T15:00:00+09:00",
    )
    .await;
    let mut others = Vec::new();
    for when in ["14:00", "14:30", "15:00"] {
        others.push(scene_location(&pool, user, &format!("2026-10-01T{when}:00+09:00")).await);
    }
    scene_erase(&pool, first).await;
    scene_erase(&pool, second).await;

    purge_now(&pool, first).await;

    assert!(kept(&pool, second).await);
    for id in others {
        assert!(kept(&pool, id).await);
    }
}

// Scenario: 別に消した滞在は重なっていても本文が残る
#[tokio::test]
async fn purge_scope_keeps_separately_erased_overlapping_stay() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let first = scene_stay(
        &pool,
        user,
        "2026-10-01T10:00:00+09:00",
        "2026-10-01T11:00:00+09:00",
    )
    .await;
    let second = scene_stay(
        &pool,
        user,
        "2026-10-01T10:30:00+09:00",
        "2026-10-01T11:30:00+09:00",
    )
    .await;
    scene_erase(&pool, first).await;
    scene_erase(&pool, second).await;

    let outcome = purge_now(&pool, first).await;

    assert_eq!(outcome.counts.stays, 1);
    assert!(emptied(&pool, first).await);
    assert!(kept(&pool, second).await);
}

// Scenario: 一覧に出ている滞在の本文は消えない
#[tokio::test]
async fn purge_scope_keeps_listed_overlapping_stay() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    let listed = scene_stay(
        &pool,
        user,
        "2026-10-01T10:30:00+09:00",
        "2026-10-01T11:30:00+09:00",
    )
    .await;
    scene_erase(&pool, stay).await;

    purge_now(&pool, stay).await;

    assert!(kept(&pool, listed).await);
    let live: i64 = sqlx::query_scalar("SELECT count(*) FROM core.event_live WHERE id = $1")
        .bind(listed)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(live, 1, "一覧（生きた記録）に出ている");
}

#[tokio::test]
async fn purge_scope_edge_touching_rebuild_stay_is_not_included() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    // 端が触れるだけ（11:00 で始まる）。作り直しの印のものでも重なりにしない
    let touching = scene_stay(
        &pool,
        user,
        "2026-10-01T11:00:00+09:00",
        "2026-10-01T12:00:00+09:00",
    )
    .await;
    let before = scene_stay(
        &pool,
        user,
        "2026-10-01T09:00:00+09:00",
        "2026-10-01T10:00:00+09:00",
    )
    .await;
    scene_erase(&pool, stay).await;
    scene_mark_rebuild(&pool, touching, "rebuild:absorbed").await;
    scene_mark_rebuild(&pool, before, "rebuild:erased-range").await;

    let outcome = purge_now(&pool, stay).await;

    assert_eq!(outcome.counts.stays, 1);
    assert!(kept(&pool, touching).await);
    assert!(kept(&pool, before).await);
}

// Scenario: 本文を消した記録は時刻と鍵を残す
#[tokio::test]
async fn purge_scope_emptied_row_keeps_time_and_hash() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, locations) = scene_basic(&pool, user).await;
    let id = locations[1];
    let before: (DateTime<Utc>, String) =
        sqlx::query_as("SELECT event_time, content_hash FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    scene_erase(&pool, stay).await;

    purge_now(&pool, stay).await;

    let row: (String, serde_json::Value, DateTime<Utc>, String) = sqlx::query_as(
        "SELECT raw, payload, event_time, content_hash FROM core.event WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, "");
    assert_eq!(row.1, serde_json::json!({}));
    assert_eq!((row.2, row.3), before);
}

/// 10:05–11:48 の滞在・位置 70 件・前の版 20 件（1 つ目は 10:05–11:30）を本文ごと消して、操作の行を返す。
pub(crate) async fn scene_big_purge(pool: &PgPool, user: Uuid) -> (Uuid, Uuid) {
    let stay = scene_stay(
        pool,
        user,
        "2026-10-01T10:05:00+09:00",
        "2026-10-01T11:48:00+09:00",
    )
    .await;
    for minute in 0..70 {
        let when = at("2026-10-01T10:05:00+09:00") + Duration::minutes(minute);
        scene_location(pool, user, &when.to_rfc3339()).await;
    }
    scene_version(
        pool,
        stay,
        user,
        1,
        "2026-10-01T10:05:00+09:00",
        "2026-10-01T11:30:00+09:00",
    )
    .await;
    for no in 2..=20 {
        scene_version(
            pool,
            stay,
            user,
            no,
            "2026-10-01T10:05:00+09:00",
            "2026-10-01T11:00:00+09:00",
        )
        .await;
    }
    scene_erase(pool, stay).await;
    let outcome = purge_now(pool, stay).await;
    (stay, outcome.operation_id.unwrap())
}

// Scenario: 本文ごと消すと操作の台帳に 1 行残る
#[tokio::test]
async fn purge_scope_writes_one_operation_row() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, op) = scene_big_purge(&pool, user).await;

    type OperationRow = (Uuid, DateTime<Utc>, DateTime<Utc>, i32, i32, i32, i32, i32);
    let rows: Vec<OperationRow> = sqlx::query_as(
        "SELECT stay_id, start_at, end_at, stays, locations, archive_locations, archive_items, versions
           FROM core.erasure_operation WHERE user_id = $1",
    )
    .bind(user)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![(
            stay,
            at("2026-10-01T10:05:00+09:00"),
            at("2026-10-01T11:48:00+09:00"),
            1,
            70,
            0,
            0,
            20
        )]
    );
    let _ = op;
}

// Scenario: 記録ごとの消去の台帳の行が操作に結ばれる
#[tokio::test]
async fn purge_scope_ledger_rows_are_tied_to_the_operation() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, locations) = scene_basic(&pool, user).await;
    scene_version(
        &pool,
        stay,
        user,
        1,
        "2026-10-01T10:00:00+09:00",
        "2026-10-01T10:40:00+09:00",
    )
    .await;
    scene_version(
        &pool,
        stay,
        user,
        2,
        "2026-10-01T10:00:00+09:00",
        "2026-10-01T10:50:00+09:00",
    )
    .await;
    scene_erase(&pool, stay).await;

    let op = purge_now(&pool, stay).await.operation_id.unwrap();

    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT event_id, scope, logical_source FROM core.erasure_ledger
          WHERE operation_id = $1 ORDER BY scope, logical_source, event_id",
    )
    .bind(op)
    .fetch_all(&pool)
    .await
    .unwrap();
    let of = |scope: &str, source: &str| {
        rows.iter()
            .filter(|r| r.1 == scope && r.2 == source)
            .count()
    };
    assert_eq!(of("event", "s01-stay"), 1);
    assert_eq!(of("event", "c01-location"), 3);
    assert_eq!(of("version", "s01-stay"), 2, "前の版ごとに 1 行");
    assert!(
        rows.iter()
            .filter(|r| r.1 == "version")
            .all(|r| r.0 == stay),
        "前の版の event_id は親"
    );
    for id in locations {
        assert!(rows.iter().any(|r| r.0 == id && r.1 == "event"));
    }
    assert_eq!(rows.len(), 6);
}

// Scenario: 本文を消した滞在の時間が範囲の行に残る
#[tokio::test]
async fn purge_scope_span_rows_keep_the_times() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, op) = scene_big_purge(&pool, user).await;

    type SpanRow = (Uuid, String, Option<i32>, DateTime<Utc>, DateTime<Utc>);
    let rows: Vec<SpanRow> = sqlx::query_as(
        "SELECT event_id, scope, version_no, start_at, end_at FROM core.erasure_span
          WHERE operation_id = $1 ORDER BY scope, version_no",
    )
    .bind(op)
    .fetch_all(&pool)
    .await
    .unwrap();
    let start = at("2026-10-01T10:05:00+09:00");
    assert!(rows.contains(&(
        stay,
        "event".into(),
        None,
        start,
        at("2026-10-01T11:48:00+09:00")
    )));
    assert!(rows.contains(&(
        stay,
        "version".into(),
        Some(1),
        start,
        at("2026-10-01T11:30:00+09:00")
    )));
    assert_eq!(rows.len(), 1 + 20);
}

// Scenario: 台帳に書けないと本文も消えない
#[tokio::test]
async fn purge_scope_ledger_failure_leaves_every_body() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, locations) = scene_basic(&pool, user).await;
    scene_version(
        &pool,
        stay,
        user,
        1,
        "2026-10-01T10:00:00+09:00",
        "2026-10-01T10:40:00+09:00",
    )
    .await;
    scene_erase(&pool, stay).await;
    let counts = scope_counts(&pool, stay).await;

    // 本文を空にした後の、台帳の最後の書き込み（範囲の行）が CHECK 違反で落ちる
    let result = deletion::purge_with_span_scope(&pool, stay, None, counts, "not-a-scope").await;

    assert!(matches!(result, Err(PurgeError::Database(_))));
    assert!(kept(&pool, stay).await);
    for id in locations {
        assert!(kept(&pool, id).await);
    }
    let versions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.event_version WHERE event_id = $1 AND raw <> ''",
    )
    .bind(stay)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(versions, 1);
    let operations: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.erasure_operation WHERE stay_id = $1")
            .bind(stay)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(operations, 0);
}

// Scenario: 本文ごと消す操作と戻す操作が同時に走るとどちらか一方だけが効く
#[tokio::test]
async fn purge_scope_purge_and_restore_race_has_one_winner() {
    let pool = testdb::pool().await;
    let holder = testdb::pool().await;
    let user = testdb::user();
    let (stay, locations) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    let counts = scope_counts(&pool, stay).await;

    let mut tx = holder.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text))")
        .bind(crate::stay_store::LOCK_KEY)
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
    let purge_task = tokio::spawn({
        let pool = pool.clone();
        async move { deletion::purge(&pool, stay, None, counts).await }
    });
    let restore_task = tokio::spawn({
        let pool = pool.clone();
        async move { deletion::restore(&pool, &[stay], None).await }
    });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(!purge_task.is_finished(), "消す操作が錠を待っていない");
    assert!(!restore_task.is_finished(), "戻す操作が錠を待っていない");
    tx.rollback().await.unwrap();
    let purged = purge_task.await.unwrap();
    let restored = restore_task.await.unwrap();

    let state: (bool, bool) = sqlx::query_as(
        "SELECT raw = '', deleted_at IS NOT NULL AND deleted_by = 'user' FROM core.event WHERE id = $1",
    )
    .bind(stay)
    .fetch_one(&pool)
    .await
    .unwrap();
    match (purged, restored) {
        (Ok(_), Err(deletion::RestoreError::Purged)) => {
            assert_eq!(state, (true, true), "本文が空で削除の印が付いている");
            assert!(emptied(&pool, locations[0]).await);
        }
        (Err(PurgeError::NotErased), Ok(_)) => {
            assert_eq!(state, (false, false), "本文が残って削除の印が外れている");
            assert!(kept(&pool, locations[0]).await);
        }
        (p, r) => panic!("どちらか一方だけが効くはずが: purge={p:?} restore={r:?}"),
    }
}

#[tokio::test]
async fn purge_scope_unknown_source_is_refused_and_nothing_is_written() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, locations) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    // 連鎖の規則が将来変わって、写真が同じ原因で消えたことにされた場面
    let photo = scene_event(
        &pool,
        user,
        "c01-photo",
        "2026-10-01T10:30:00+09:00",
        r#"{"lat":35.68}"#,
    )
    .await;
    scene_mark_rebuild(&pool, photo, "user:cascade").await;
    sqlx::query(
        "INSERT INTO core.deletion_ledger (event_id, user_id, logical_source, action, cause_event_id, mark)
         VALUES ($1, $2, 'c01-photo', 'erase', $3, 'user:cascade')",
    )
    .bind(photo)
    .bind(user)
    .bind(stay)
    .execute(&pool)
    .await
    .unwrap();

    let mut tx = pool.begin().await.unwrap();
    let scoped = deletion::purge_scope(&mut tx, stay).await;
    tx.rollback().await.unwrap();
    assert!(matches!(scoped, Err(PurgeError::UnknownSource)));
    let result = deletion::purge(&pool, stay, None, PurgeCounts::default()).await;

    assert!(matches!(result, Err(PurgeError::UnknownSource)));
    assert!(kept(&pool, stay).await);
    assert!(kept(&pool, photo).await);
    assert!(kept(&pool, locations[0]).await);
}

#[tokio::test]
async fn purge_scope_counts_changed_writes_nothing() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, locations) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    let mut shown = scope_counts(&pool, stay).await;
    shown.locations -= 1;

    let result = deletion::purge(&pool, stay, None, shown).await;

    match result {
        Err(PurgeError::CountsChanged(now)) => assert_eq!(now.locations, 3),
        other => panic!("件数が違えば断られるはず: {other:?}"),
    }
    assert!(kept(&pool, stay).await);
    assert!(kept(&pool, locations[0]).await);
}

#[tokio::test]
async fn purge_scope_second_press_is_a_success_with_nothing_added() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    purge_now(&pool, stay).await;

    // 件数は比べない（2 度押しは成功として返す）
    let again = deletion::purge(
        &pool,
        stay,
        None,
        PurgeCounts {
            stays: 9,
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(again.counts, PurgeCounts::default());
    assert!(again.operation_id.is_none());
    let operations: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.erasure_operation WHERE stay_id = $1")
            .bind(stay)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(operations, 1);
}

/// すでに本文を消した滞在の確認でも、`range` は滞在の時間（始まり〜終わり）を返す（final review R8 / design D1）。
/// 本文は空なので、時間は `core.stay_erased`（範囲の行）から読む。
#[tokio::test]
async fn purge_scope_preview_of_a_purged_stay_keeps_the_range() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    purge_now(&pool, stay).await;

    let scope = deletion::purge_preview(&pool, stay, None).await.unwrap();

    assert!(scope.already_purged);
    assert_eq!(scope.counts, PurgeCounts::default());
    assert_eq!(scope.start, at("2026-10-01T10:00:00+09:00"));
    assert_eq!(scope.end, at("2026-10-01T11:00:00+09:00"));
}

#[tokio::test]
async fn purge_scope_refuses_a_stay_that_is_not_erased() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    let rebuilt = scene_stay(
        &pool,
        user,
        "2026-10-01T12:00:00+09:00",
        "2026-10-01T13:00:00+09:00",
    )
    .await;
    scene_mark_rebuild(&pool, rebuilt, "rebuild:absorbed").await;

    for id in [stay, rebuilt] {
        let result = deletion::purge(&pool, id, None, PurgeCounts::default()).await;
        assert!(matches!(result, Err(PurgeError::NotErased)), "{result:?}");
        assert!(kept(&pool, id).await);
    }
    let result = deletion::purge(&pool, stay, Some(testdb::user()), PurgeCounts::default()).await;
    assert!(matches!(result, Err(PurgeError::UserMismatch)));
}

#[tokio::test]
async fn purge_scope_restore_of_a_purged_stay_is_refused() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, locations) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    purge_now(&pool, stay).await;

    let result = deletion::restore(&pool, &[stay], None).await;

    assert!(
        matches!(result, Err(deletion::RestoreError::Purged)),
        "{result:?}"
    );
    let marked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.event WHERE id = ANY($1) AND deleted_at IS NOT NULL",
    )
    .bind([vec![stay], locations].concat())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(marked, 4, "何も書かない（削除の印は付いたまま）");
}

// ------------------------------------------------------------------ 消した時間帯の読み手を台帳へ寄せる（Task 3 / design D5 / D6 / D14）

use crate::export_excluded_tests::World;
use crate::stay_store::EntryKind;
use serde_json::json;

fn far_future() -> DateTime<Utc> {
    at("2027-01-01T00:00:00+09:00")
}

async fn day_entries(pool: &PgPool, user: Uuid) -> Vec<crate::stay_store::DayEntry> {
    crate::stay_store::day_view(pool, user, testdb::date("2026-10-01"), far_future())
        .await
        .unwrap()
        .entries
}

fn stays_in(entries: &[crate::stay_store::DayEntry]) -> usize {
    entries.iter().filter(|e| e.kind == EntryKind::Stay).count()
}

/// 10:15〜10:45 に 5 分おきに新しい位置が入った状態にする（本文を消した 10:00〜11:00 の中）。
async fn scene_new_points(pool: &PgPool, user: Uuid) {
    for when in [
        "10:15", "10:20", "10:25", "10:30", "10:35", "10:40", "10:45",
    ] {
        scene_location(pool, user, &format!("2026-10-01T{when}:00+09:00")).await;
    }
}

// Scenario: 本文ごと消した後も消した時間帯の長さは変わらない
#[tokio::test]
async fn purge_range_length_is_unchanged_after_purge() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let stay = scene_stay(
        &pool,
        user,
        "2026-10-01T10:05:00+09:00",
        "2026-10-01T11:48:00+09:00",
    )
    .await;
    scene_erase(&pool, stay).await;
    purge_now(&pool, stay).await;

    let entries = day_entries(&pool, user).await;

    let erased: Vec<_> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Erased)
        .collect();
    assert_eq!(erased.len(), 1);
    assert_eq!(erased[0].start, at("2026-10-01T10:05:00+09:00"));
    assert_eq!(erased[0].end, at("2026-10-01T11:48:00+09:00"));
}

// Scenario: 本文ごと消した時間帯の撮影位置は書き出しから除かれたまま
#[tokio::test]
async fn purge_range_photo_stays_out_of_the_export() {
    let w = World::new().await;
    let stay = w.stay(&w.at("01:00"), &w.at("02:00")).await;
    let photo = w
        .put(
            "c01-photo",
            &w.at("01:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    deletion::erase(&w.pool, stay, None).await.unwrap();
    assert!(
        w.kept(&[photo]).await.is_empty(),
        "本文を消す前から除かれている"
    );

    purge_now(&w.pool, stay).await;

    assert!(emptied(&w.pool, stay).await);
    assert!(
        w.kept(&[photo]).await.is_empty(),
        "本文を消した後に写真が書き出しへ戻った"
    );
}

// Scenario: 本文ごと消した時間帯に作り直しは滞在を作らない
#[tokio::test]
async fn purge_range_rebuild_makes_no_stay_inside() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    purge_now(&pool, stay).await;
    scene_new_points(&pool, user).await;

    crate::stay_store::rebuild_day(&pool, user, testdb::date("2026-10-01"))
        .await
        .unwrap();

    let entries = day_entries(&pool, user).await;
    assert_eq!(stays_in(&entries), 0, "{entries:?}");
}

// Scenario: 作り直しの後も本文を消した滞在の本文は空のまま
#[tokio::test]
async fn purge_range_rebuild_keeps_the_purged_body_empty() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    purge_now(&pool, stay).await;

    crate::stay_store::rebuild_day(&pool, user, testdb::date("2026-10-01"))
        .await
        .unwrap();

    assert!(emptied(&pool, stay).await);
    assert_eq!(versions_emptied(&pool, stay).await, (0, 0));
    assert_eq!(stays_in(&day_entries(&pool, user).await), 0);
}

// Scenario: 吸収されていた滞在の本文は作り直しで戻らない
#[tokio::test]
async fn purge_range_rebuild_does_not_restore_an_absorbed_body() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    let absorbed = scene_stay(
        &pool,
        user,
        "2026-10-01T10:10:00+09:00",
        "2026-10-01T10:40:00+09:00",
    )
    .await;
    scene_erase(&pool, stay).await;
    scene_mark_rebuild(&pool, absorbed, "rebuild:absorbed").await;
    purge_now(&pool, stay).await;
    scene_new_points(&pool, user).await;

    crate::stay_store::rebuild_day(&pool, user, testdb::date("2026-10-01"))
        .await
        .unwrap();

    assert!(emptied(&pool, absorbed).await);
    let mark: (bool, Option<String>) =
        sqlx::query_as("SELECT deleted_at IS NOT NULL, deleted_by FROM core.event WHERE id = $1")
            .bind(absorbed)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(mark, (true, Some("user:purged".into())));
}

// Scenario: 本文を消した記録は 1 日の記録の読み出しに出ない
#[tokio::test]
async fn purge_range_purged_records_are_not_in_the_day_records() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (stay, locations) = scene_basic(&pool, user).await;
    let absorbed = scene_stay(
        &pool,
        user,
        "2026-10-01T10:10:00+09:00",
        "2026-10-01T10:40:00+09:00",
    )
    .await;
    scene_erase(&pool, stay).await;
    scene_mark_rebuild(&pool, absorbed, "rebuild:absorbed").await;
    purge_now(&pool, stay).await;

    let records = crate::day_records::records(
        &pool,
        user,
        testdb::date("2026-10-01"),
        None,
        None,
        far_future(),
    )
    .await
    .unwrap();

    let purged: Vec<Uuid> = [vec![stay, absorbed], locations].concat();
    assert!(
        records.iter().all(|r| !purged.contains(&r.id)),
        "本文を消した記録が出た: {records:?}"
    );
}

/// 起動時に数える「本文を消したのに範囲の行が無い滞在」（design D14 / final review R2）。
/// 範囲の行を持つ本文ごと消した滞在は数えず、範囲の行が無いものだけを数える。サーバと同じ役割の接続で読む。
#[tokio::test]
async fn purge_range_startup_counts_purged_stays_without_a_span_row() {
    let db = testdb::isolated_with_app().await;
    let (owner, app) = (db.owner.clone(), db.app.clone());
    let user = testdb::user();
    assert_eq!(deletion::purged_without_span(&app).await.unwrap(), 0);

    let (stay, _) = scene_basic(&owner, user).await;
    scene_erase(&owner, stay).await;
    purge_now(&owner, stay).await;
    assert_eq!(
        deletion::purged_without_span(&app).await.unwrap(),
        0,
        "範囲の行を持つ滞在は数えない"
    );

    // 範囲の行を持たない、本文を消した滞在（移行の前の手作業・壊れた行）
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload, deleted_at, deleted_by)
         VALUES ($1, $2, 's01-stay', $1::text, 'derived', $3, 540, 'Asia/Tokyo', 1, $1::text,
                 '', '{}'::jsonb, now(), 'user')",
    )
    .bind(Uuid::new_v4())
    .bind(user)
    .bind(at("2026-10-02T10:00:00+09:00"))
    .execute(&owner)
    .await
    .unwrap();
    assert_eq!(deletion::purged_without_span(&app).await.unwrap(), 1);
}

// Scenario: 範囲の行が無い本文を消した滞在はその日全体を書き出しから除く
#[tokio::test]
async fn purge_range_without_a_span_row_excludes_the_whole_day() {
    let w = World::new().await;
    // 範囲の行を積まずに本文だけ消えた滞在（移行の前の手作業・壊れた行）
    let stay = w.stay(&w.at("01:00"), &w.at("02:00")).await;
    let mut tx = w.pool.begin().await.unwrap();
    put_ledger(&mut tx, stay, w.user).await;
    sqlx::query(
        "UPDATE core.event SET raw = '', payload = '{}'::jsonb,
                deleted_at = now(), deleted_by = 'user' WHERE id = $1",
    )
    .bind(stay)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    // 15:00（Asia/Tokyo）= 06:00Z。滞在の始まりの日の中で、滞在の時間の外
    let photo = w
        .put(
            "c01-photo",
            &w.at("06:00"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;

    assert!(w.kept(&[photo]).await.is_empty());
}

// Scenario: 本文ごと消す操作と作り直しが同時に走っても本文は戻らない
#[tokio::test]
async fn purge_range_purge_and_rebuild_race_keeps_the_body_empty() {
    let pool = testdb::pool().await;
    let holder = testdb::pool().await;
    let user = testdb::user();
    let (stay, _) = scene_basic(&pool, user).await;
    scene_erase(&pool, stay).await;
    let counts = scope_counts(&pool, stay).await;

    let mut tx = holder.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text))")
        .bind(crate::stay_store::LOCK_KEY)
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
    let purge_task = tokio::spawn({
        let pool = pool.clone();
        async move { deletion::purge(&pool, stay, None, counts).await }
    });
    let rebuild_task = tokio::spawn({
        let pool = pool.clone();
        async move { crate::stay_store::rebuild_day(&pool, user, testdb::date("2026-10-01")).await }
    });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(!purge_task.is_finished(), "消す操作が錠を待っていない");
    assert!(!rebuild_task.is_finished(), "作り直しが錠を待っていない");
    tx.rollback().await.unwrap();
    purge_task.await.unwrap().unwrap();
    rebuild_task.await.unwrap().unwrap();

    assert!(emptied(&pool, stay).await);
    assert_eq!(stays_in(&day_entries(&pool, user).await), 0);
}

/// 消した時間帯を読む SQL は、滞在の本文の終わり（`payload->>'end'`）を直に読まない（design D5 / spec review R6）。
/// 出現は許可の一覧の `file:関数` と一致する。コメント行と試験の置き場（`_tests.rs`）は数えない。
#[test]
fn purge_range_reader_allowlist() {
    use std::collections::BTreeSet;
    use std::path::Path;

    fn collect(dir: &Path, root: &Path, found: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(&path, root, found);
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if !name.ends_with(".rs") || name.ends_with("_tests.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if line.trim_start().starts_with("//") || !line.contains("payload->>'end'") {
                    continue;
                }
                let func = lines[..=i]
                    .iter()
                    .rev()
                    .find_map(|l| {
                        let l = l.trim_start();
                        let l = l
                            .strip_prefix("pub(crate) ")
                            .or_else(|| l.strip_prefix("pub "))
                            .unwrap_or(l);
                        let l = l.strip_prefix("async ").unwrap_or(l);
                        l.strip_prefix("fn ").map(|r| {
                            r.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                                .next()
                                .unwrap()
                                .to_owned()
                        })
                    })
                    .unwrap_or_else(|| "?".into());
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                found.insert(format!("{rel}:{func}"));
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = BTreeSet::new();
    collect(&root, &root, &mut found);

    let allowed: BTreeSet<String> = [
        // 戻した滞在が一覧に残るかを見る前の、本文ごと消した時間帯との重なり（`core.stay_erased_purge` と同じ終わりの読み方）
        "deletion.rs:purge_scope",
        // 生きた滞在の粗い絞り（消した時間帯の読みではない）
        "stay_store.rs:load_existing",
        "stay_store.rs:day_view",
        // 生きた滞在だけを読む（`event_live` 越し）
        "lib.rs:stays_detail_get",
        // 本文が残る前の版の本文の補い
        "export/excluded.rs:excluded_sql",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_eq!(found, allowed);
}
