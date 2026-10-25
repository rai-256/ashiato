// SPDX-License-Identifier: AGPL-3.0-only
//! 主観の記録の錠と、紐づけ先の種類・通知の時刻の台帳を、**本物の PostgreSQL に対して**確かめる
//! （ST17 / Task 1）。
//!
//! 錠は取り込み口の外から（psql・プラグインが）直に殴る経路を止めるものなので、
//! ここでは取り込み口を通さず SQL で行を置いて撃つ。**利用者で隔離する**（`testdb::user()`）。
//!
//! 試験の名前の接頭辞は `st17_lock_`（この Task だけのもの）。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use uuid::Uuid;

const FEELING_DOWN: &str = include_str!("../../../migrations/202610141000_subjective_log.down.sql");

async fn fresh_db() -> (sqlx::PgPool, impl std::future::Future<Output = ()>) {
    let scratch = testdb::scratch().await;
    let fresh = scratch.pool().await;
    let closer = fresh.clone();
    (fresh, async move {
        closer.close().await;
        // 消すのは TestDb の Drop だけ（拒まれたときの待ちと打ち直し・診断を持つ。review/code.md R14）。
        drop(scratch);
    })
}

async fn apply_all(pool: &sqlx::PgPool) -> Result<(), String> {
    for (label, sql) in crate::MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(pool)
            .await
            .map_err(|e| format!("{label}: {e}"))?;
    }
    Ok(())
}

async fn count(pool: &sqlx::PgPool, sql: &str) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql).fetch_one(pool).await.unwrap();
    n
}

#[tokio::test]
async fn st17_lock_migration_applies_twice() {
    let (fresh, drop) = fresh_db().await;
    let applied = async {
        for round in 1..=2 {
            for (label, sql) in crate::MIGRATIONS {
                sqlx::raw_sql(sql)
                    .execute(&fresh)
                    .await
                    .map_err(|e| format!("{round} 回目の {label}: {e}"))?;
            }
        }
        let (n, kind, gap, name): (i64, String, i32, String) = sqlx::query_as(
            "SELECT count(*) OVER (), external_id_kind, expected_gap_sec, display_name
               FROM core.source WHERE logical_source = 's01-feeling'",
        )
        .fetch_one(&fresh)
        .await
        .map_err(|e| e.to_string())?;
        let kinds: Vec<(String, String)> =
            sqlx::query_as("SELECT kind, label FROM core.feeling_target_kind ORDER BY kind")
                .fetch_all(&fresh)
                .await
                .map_err(|e| e.to_string())?;
        Ok::<_, String>((n, kind, gap, name, kinds))
    }
    .await;
    drop.await;

    let (n, kind, gap, name, kinds) = applied.unwrap();
    assert_eq!(n, 1, "2 回当てると s01-feeling が二重になる");
    assert_eq!(
        (kind.as_str(), gap, name.as_str()),
        ("none", 86_400, "主観")
    );
    assert_eq!(
        kinds,
        vec![
            ("day".to_string(), "日".to_string()),
            ("stay".to_string(), "滞在".to_string())
        ],
        "紐づけ先の種類は日と滞在の 2 行（2 回当てても増えない）"
    );

    // 錠は `core.event` と `core.erasure_ledger` を前提にする。依存する版より後にあること
    let names: Vec<&str> = crate::MIGRATIONS.iter().map(|(n, _)| *n).collect();
    let mine = names
        .iter()
        .position(|n| n.ends_with("_subjective_log"))
        .expect("主観の移行が MIGRATIONS に無い（当て忘れると錠が本番だけ効かない）");
    let gates = names
        .iter()
        .position(|n| n.ends_with("_gates"))
        .expect("ST03 の門の版が MIGRATIONS に無い");
    assert!(
        mine > gates,
        "主観の移行が、前提にしている門の版より前にある"
    );
}

#[tokio::test]
async fn st17_lock_down_keeps_rows() {
    let (fresh, drop) = fresh_db().await;
    let result = async {
        apply_all(&fresh).await?;
        sqlx::query("INSERT INTO core.feeling_reminder (user_id, at) VALUES ($1, '21:00')")
            .bind(testdb::user())
            .execute(&fresh)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::raw_sql(FEELING_DOWN)
            .execute(&fresh)
            .await
            .map_err(|e| format!("戻しが当たらない: {e}"))?;
        let reminders = count(&fresh, "SELECT count(*) FROM core.feeling_reminder").await;
        let kinds = count(&fresh, "SELECT count(*) FROM core.feeling_target_kind").await;
        let reg = count(
            &fresh,
            "SELECT count(*) FROM core.source WHERE logical_source = 's01-feeling'",
        )
        .await;
        let locks = count(
            &fresh,
            "SELECT count(*) FROM pg_trigger WHERE tgname LIKE 'event_feeling_%'",
        )
        .await;
        // 台帳が残るので、台帳の錠も残る
        let ledger_locks = count(
            &fresh,
            "SELECT count(*) FROM pg_trigger WHERE tgname IN
               ('feeling_reminder_append_only','feeling_reminder_no_truncate',
                'feeling_target_kind_append_only','feeling_target_kind_no_truncate')",
        )
        .await;
        // 戻した後に当て直せる
        apply_all(&fresh).await?;
        Ok::<_, String>((reminders, kinds, reg, locks, ledger_locks))
    }
    .await;
    drop.await;

    let (reminders, kinds, reg, locks, ledger_locks) = result.unwrap();
    assert_eq!(reminders, 1, "通知の時刻の行が残っているのに表が消えた");
    assert_eq!(kinds, 2, "行が残っているのに紐づけ先の種類の表が消えた");
    assert_eq!(reg, 1, "行が残っているのに登録簿の行が消えた");
    assert_eq!(locks, 0, "戻しで主観の記録の錠が落ちていない");
    assert_eq!(ledger_locks, 4, "台帳が残るのに台帳の錠が落ちた");
}

#[tokio::test]
async fn st17_lock_down_keeps_rows_when_only_a_feeling_remains() {
    let (fresh, drop) = fresh_db().await;
    let result = async {
        apply_all(&fresh).await?;
        put_feeling_on(&fresh, testdb::user()).await;
        sqlx::raw_sql(FEELING_DOWN)
            .execute(&fresh)
            .await
            .map_err(|e| format!("戻しが当たらない: {e}"))?;
        let events = count(
            &fresh,
            "SELECT count(*) FROM core.event WHERE logical_source = 's01-feeling'",
        )
        .await;
        let reg = count(
            &fresh,
            "SELECT count(*) FROM core.source WHERE logical_source = 's01-feeling'",
        )
        .await;
        let kinds = count(&fresh, "SELECT count(*) FROM core.feeling_target_kind").await;
        Ok::<_, String>((events, reg, kinds))
    }
    .await;
    drop.await;
    let (events, reg, kinds) = result.unwrap();
    assert_eq!(events, 1, "戻しで主観の記録が消えた");
    assert_eq!(reg, 1, "主観の記録が残るのに登録簿の行が消えた");
    assert_eq!(kinds, 2, "主観の記録が残るのに紐づけ先の種類の表が消えた");
}

#[tokio::test]
async fn st17_lock_down_drops_when_unused() {
    let (fresh, drop) = fresh_db().await;
    let result = async {
        apply_all(&fresh).await?;
        sqlx::raw_sql(FEELING_DOWN)
            .execute(&fresh)
            .await
            .map_err(|e| format!("戻しが当たらない: {e}"))?;
        let gone: (bool,) = sqlx::query_as(
            "SELECT to_regclass('core.feeling_reminder') IS NULL
                AND to_regclass('core.feeling_target_kind') IS NULL",
        )
        .fetch_one(&fresh)
        .await
        .map_err(|e| e.to_string())?;
        let reg = count(
            &fresh,
            "SELECT count(*) FROM core.source WHERE logical_source = 's01-feeling'",
        )
        .await;
        // 2 回目の戻しも当たる。戻した後に当て直せる
        sqlx::raw_sql(FEELING_DOWN)
            .execute(&fresh)
            .await
            .map_err(|e| format!("2 回目の戻しが当たらない: {e}"))?;
        apply_all(&fresh).await?;
        Ok::<_, String>((gone.0, reg))
    }
    .await;
    drop.await;
    let (gone, reg) = result.unwrap();
    assert!(gone, "何も使っていないのに表が残っている");
    assert_eq!(reg, 0, "何も使っていないのに登録簿の行が残っている");
}

#[tokio::test]
async fn st17_lock_ledgers_are_append_only() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    sqlx::query("INSERT INTO core.feeling_reminder (user_id, at) VALUES ($1, '20:30')")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    for stmt in [
        format!("UPDATE core.feeling_reminder SET at = '07:00' WHERE user_id = '{user}'"),
        format!("DELETE FROM core.feeling_reminder WHERE user_id = '{user}'"),
        "TRUNCATE core.feeling_reminder".to_string(),
        "UPDATE core.feeling_target_kind SET label = '書き換え' WHERE kind = 'day'".to_string(),
        "DELETE FROM core.feeling_target_kind WHERE kind = 'stay'".to_string(),
        "TRUNCATE core.feeling_target_kind".to_string(),
        "TRUNCATE core.feeling_target_kind CASCADE".to_string(),
    ] {
        assert!(
            sqlx::raw_sql(&stmt).execute(&pool).await.is_err(),
            "追記のみの台帳が変えられた: {stmt}"
        );
    }
    let (at,): (String,) =
        sqlx::query_as("SELECT at::text FROM core.feeling_reminder WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(at, "20:30:00", "拒まれたのに通知の時刻が変わっている");
    assert_eq!(
        count(&pool, "SELECT count(*) FROM core.feeling_target_kind").await,
        2,
        "拒まれたのに紐づけ先の種類が変わっている"
    );
    // 種類の名前の形は DB も見る（アプリを通らない書き込み）
    assert!(
        sqlx::raw_sql("INSERT INTO core.feeling_target_kind (kind, label) VALUES ('Bad Kind','x')")
            .execute(&pool)
            .await
            .is_err(),
        "種類の名前の形が見られていない"
    );
}

// ================================================================ 主観の記録の錠

const RAW: &str = r#"{"valence":2}"#;

async fn put_feeling_on(pool: &sqlx::PgPool, user: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, sensitivity, content_hash, raw, payload)
         VALUES ($1, $2, 's01-feeling', 'authored', '2026-09-15T02:00:00Z', 540, 'Asia/Tokyo', 1, 1,
                 $3, $4, $4::jsonb)",
    )
    .bind(id)
    .bind(user)
    .bind(format!("feeling-hash-{id}"))
    .bind(RAW)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn put_feeling(pool: &sqlx::PgPool) -> Uuid {
    put_feeling_on(pool, testdb::user()).await
}

async fn put_ledger(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, event: Uuid, user: Uuid) {
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, 's01-feeling', 'event', 'check')",
    )
    .bind(event)
    .bind(user)
    .execute(&mut **tx)
    .await
    .unwrap();
}

async fn raw_of(pool: &sqlx::PgPool, id: Uuid) -> String {
    let (raw,): (String,) = sqlx::query_as("SELECT raw FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    raw
}

async fn assert_rejected(pool: &sqlx::PgPool, sql: &str, id: Uuid) {
    let r = sqlx::query(sql).bind(id).execute(pool).await;
    assert!(r.is_err(), "拒まれるはずの文が通った: {sql}");
}

// Scenario: 主観の値を書き換える文は拒まれる
#[tokio::test]
async fn st17_lock_rejects_rewriting_the_value() {
    let pool = testdb::pool().await;
    let id = put_feeling(&pool).await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET payload = jsonb_set(payload,'{valence}','-2') WHERE id = $1",
        id,
    )
    .await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET raw = '{\"valence\":-2}' WHERE id = $1",
        id,
    )
    .await;
    assert_eq!(raw_of(&pool, id).await, RAW, "拒まれたのに値が変わっている");
}

// Scenario: 主観の紐づけ先は書き換えられない
#[tokio::test]
async fn st17_lock_rejects_rewriting_the_target() {
    let pool = testdb::pool().await;
    let id = put_feeling(&pool).await;
    // 紐づけ先は原文と解析済みの中にある。差し替えは消去の形ではない
    assert_rejected(
        &pool,
        "UPDATE core.event
            SET raw = '{\"target\":{\"kind\":\"day\",\"id\":\"2026-09-16\"}}',
                payload = '{\"target\":{\"kind\":\"day\",\"id\":\"2026-09-16\"}}'
          WHERE id = $1",
        id,
    )
    .await;
    assert_eq!(
        raw_of(&pool, id).await,
        RAW,
        "拒まれたのに原文が変わっている"
    );
}

// Scenario: 主観の出来事の時刻は書き換えられない
#[tokio::test]
async fn st17_lock_rejects_rewriting_the_event_time() {
    let pool = testdb::pool().await;
    let id = put_feeling(&pool).await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET event_time = '2000-01-01T00:00:00Z' WHERE id = $1",
        id,
    )
    .await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET tz_offset_min = 0, tz_id = 'UTC' WHERE id = $1",
        id,
    )
    .await;
    let (t,): (String,) = sqlx::query_as("SELECT event_time::text FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        t.starts_with("2026-09-15"),
        "拒まれたのに時刻が変わっている: {t}"
    );
}

#[tokio::test]
async fn st17_lock_rejects_rewriting_the_other_frozen_columns() {
    let pool = testdb::pool().await;
    let id = put_feeling(&pool).await;
    let other = Uuid::new_v4();
    for sql in [
        format!("UPDATE core.event SET user_id = '{other}' WHERE id = $1"),
        format!("UPDATE core.event SET id = '{other}' WHERE id = $1"),
        "UPDATE core.event SET origin = 'collected' WHERE id = $1".to_string(),
        "UPDATE core.event SET external_ref = 'forged' WHERE id = $1".to_string(),
        "UPDATE core.event SET device_id = 'forged' WHERE id = $1".to_string(),
        "UPDATE core.event SET ingest_time = '2000-01-01T00:00:00Z' WHERE id = $1".to_string(),
        "UPDATE core.event SET schema_version = 9 WHERE id = $1".to_string(),
        "UPDATE core.event SET logical_source = 'immutable-check' WHERE id = $1".to_string(),
    ] {
        assert_rejected(&pool, &sql, id).await;
    }
}

// Scenario: 主観の行は削除できない
#[tokio::test]
async fn st17_lock_rejects_deleting_the_row() {
    let pool = testdb::pool().await;
    let id = put_feeling(&pool).await;
    assert_rejected(&pool, "DELETE FROM core.event WHERE id = $1", id).await;
    assert_eq!(
        count(
            &pool,
            &format!("SELECT count(*) FROM core.event WHERE id = '{id}'")
        )
        .await,
        1,
        "拒まれたのに行が消えている"
    );
}

async fn put_other_authored(pool: &sqlx::PgPool, src: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ($1, $2, $3, 'authored', '2026-09-15T02:00:00Z', 540, 'Asia/Tokyo', 1,
                 $4, '{}', '{}')",
    )
    .bind(id)
    .bind(testdb::user())
    .bind(src)
    .bind(format!("other-hash-{id}"))
    .execute(pool)
    .await
    .unwrap();
    id
}

// Scenario: 他の記録を主観へ付け替えられない
#[tokio::test]
async fn st17_lock_rejects_reassigning_another_record() {
    let pool = testdb::pool().await;
    let src = testdb::source(&pool, "st17-lock", 3600).await;
    let id = put_other_authored(&pool, &src).await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET logical_source = 's01-feeling' WHERE id = $1",
        id,
    )
    .await;
    let (kept,): (String,) = sqlx::query_as("SELECT logical_source FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kept, src);
}

// Scenario: 主観に削除の印を付けられる
#[tokio::test]
async fn st17_lock_allows_the_deletion_mark() {
    let pool = testdb::pool().await;
    let id = put_feeling(&pool).await;
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'check' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("削除の印まで止めている（FR-50）");
}

// Scenario: 主観の感度を変えられる
#[tokio::test]
async fn st17_lock_allows_changing_the_sensitivity() {
    let pool = testdb::pool().await;
    let id = put_feeling(&pool).await;
    sqlx::query("UPDATE core.event SET sensitivity = 3 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("感度まで止めている（PERM-2）");
}

// Scenario: 台帳のある主観の消去は通る
#[tokio::test]
async fn st17_lock_allows_erasure_with_its_ledger() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let id = put_feeling_on(&pool, user).await;
    let mut tx = pool.begin().await.unwrap();
    put_ledger(&mut tx, id, user).await;
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit()
        .await
        .expect("台帳のある消去が通らない（FR-51）");
    assert_eq!(raw_of(&pool, id).await, "");
}

// Scenario: 台帳の無い主観の消去は拒まれる
#[tokio::test]
async fn st17_lock_rejects_erasure_without_a_ledger() {
    let pool = testdb::pool().await;
    let id = put_feeling(&pool).await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1",
        id,
    )
    .await;
    assert_eq!(raw_of(&pool, id).await, RAW, "拒まれたのに原文が消えている");
}

// Scenario: 別の記録の台帳の行では主観の消去は通らない
#[tokio::test]
async fn st17_lock_rejects_erasure_with_another_records_ledger() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let id = put_feeling_on(&pool, user).await;
    let mut tx = pool.begin().await.unwrap();
    put_ledger(&mut tx, Uuid::new_v4(), user).await;
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        tx.commit().await.is_err(),
        "別の記録の台帳 1 行で主観の記録が消去できた"
    );
    assert_eq!(raw_of(&pool, id).await, RAW, "拒まれたのに原文が消えている");
}

// Scenario: 台帳の行があっても消去の形でない主観の書き換えは拒まれる
#[tokio::test]
async fn st17_lock_rejects_a_forged_erasure() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let id = put_feeling_on(&pool, user).await;
    let mut tx = pool.begin().await.unwrap();
    put_ledger(&mut tx, id, user).await;
    sqlx::query("UPDATE core.event SET raw = '', payload = '{\"valence\":-2}' WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        tx.commit().await.is_err(),
        "消去の顔で解析済みを差し替えられた"
    );
    assert_eq!(raw_of(&pool, id).await, RAW, "拒まれたのに原文が消えている");
}

// Scenario: 主観以外の本人が書いた記録は従来どおりの規則で扱われる
#[tokio::test]
async fn st17_lock_leaves_other_authored_records_alone() {
    let pool = testdb::pool().await;
    let src = testdb::source(&pool, "st17-lock-other", 3600).await;
    put_feeling(&pool).await; // 主観の行がある DB で撃つ
    let id = put_other_authored(&pool, &src).await;
    sqlx::query("UPDATE core.event SET payload = '{\"edited\":true}' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("主観の錠が主観以外の本人が書いた記録まで止めている");
}
