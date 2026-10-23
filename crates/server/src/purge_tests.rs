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
    let payload: String =
        sqlx::query_scalar("SELECT payload::text FROM core.event WHERE id = $1")
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
               FROM core.stay_erased WHERE id = $1",
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
           FROM core.stay_erased WHERE id = $1",
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
