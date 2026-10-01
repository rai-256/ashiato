// SPDX-License-Identifier: AGPL-3.0-only
//! 場所の器と、場所の記録の錠を、**本物の PostgreSQL に対して**確かめる（ST21 / Task 1）。
//!
//! 錠は取り込み口の外から（psql・プラグインが）直に殴る経路を止めるものなので、
//! ここでは取り込み口を通さず SQL で行を置いて撃つ。
//! **利用者で隔離する**（`testdb::user()`）。器の表は追記のみで消せない。
//!
//! 試験の名前の接頭辞は `place_`（この change だけのもの）。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use uuid::Uuid;

async fn fresh_db() -> (sqlx::PgPool, impl std::future::Future<Output = ()>) {
    let admin = testdb::pool().await;
    let name = format!("st21_tmp_{}", Uuid::new_v4().simple());
    sqlx::raw_sql(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    let base = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ashiato:ashiato@127.0.0.1:55432/ashiato".into());
    let url = format!("{}/{name}", &base[..base.rfind('/').unwrap()]);
    let fresh = sqlx::PgPool::connect(&url).await.unwrap();
    let closer = fresh.clone();
    (fresh, async move {
        closer.close().await;
        sqlx::raw_sql(&format!("DROP DATABASE {name} WITH (FORCE)"))
            .execute(&admin)
            .await
            .unwrap();
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

const PLACES_DOWN: &str = include_str!("../../../migrations/202610020030_places.down.sql");

async fn count(pool: &sqlx::PgPool, sql: &str) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql).fetch_one(pool).await.unwrap();
    n
}

#[tokio::test]
async fn place_lock_migration_applies_twice() {
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
        let (n, kind, gap): (i64, String, i32) = sqlx::query_as(
            "SELECT count(*) OVER (), external_id_kind, expected_gap_sec
               FROM core.source WHERE logical_source = 's01-place'",
        )
        .fetch_one(&fresh)
        .await
        .map_err(|e| e.to_string())?;
        Ok::<(i64, String, i32), String>((n, kind, gap))
    }
    .await;
    drop.await;

    let (n, kind, gap) = applied.unwrap();
    assert_eq!(n, 1, "2 回当てると s01-place が二重になる");
    assert_eq!(kind, "none");
    assert_eq!(gap, 86_400);

    // 見たいのは登録し忘れていないことと、依存する版より後にあること
    // （場所の錠は `core.event` と `core.erasure_ledger` を前提にする）。
    let names: Vec<&str> = crate::MIGRATIONS.iter().map(|(n, _)| *n).collect();
    let mine = names
        .iter()
        .position(|n| n.ends_with("_places"))
        .expect("場所の移行が MIGRATIONS に無い（当て忘れると錠が本番だけ効かない）");
    let gates = names
        .iter()
        .position(|n| n.ends_with("_gates"))
        .expect("ST03 の門の版が MIGRATIONS に無い");
    assert!(
        mine > gates,
        "場所の移行が、前提にしている門の版より前にある"
    );
}

#[tokio::test]
async fn place_lock_down_keeps_rows() {
    let (fresh, drop) = fresh_db().await;
    let result = async {
        apply_all(&fresh).await?;
        let user = testdb::user();
        sqlx::query("INSERT INTO core.place (id, user_id) VALUES ($1, $2)")
            .bind(Uuid::new_v4())
            .bind(user)
            .execute(&fresh)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::raw_sql(PLACES_DOWN)
            .execute(&fresh)
            .await
            .map_err(|e| format!("戻しが当たらない: {e}"))?;
        let places = count(&fresh, "SELECT count(*) FROM core.place").await;
        let reg = count(
            &fresh,
            "SELECT count(*) FROM core.source WHERE logical_source = 's01-place'",
        )
        .await;
        // 錠の関数は落ちている（場所の記録の錠）
        let locks = count(
            &fresh,
            "SELECT count(*) FROM pg_trigger WHERE tgname LIKE 'event_place_%'",
        )
        .await;
        // 器の錠は残る（器の行が残るので、追記のみのまま）
        let container_locks = count(
            &fresh,
            "SELECT count(*) FROM pg_trigger WHERE tgname IN ('place_append_only','place_no_truncate')",
        )
        .await;
        // 戻した後に当て直せる
        apply_all(&fresh).await?;
        Ok::<_, String>((places, reg, locks, container_locks))
    }
    .await;
    drop.await;

    let (places, reg, locks, container_locks) = result.unwrap();
    assert_eq!(places, 1, "器の行が残っているのに表が消えた");
    assert_eq!(reg, 1, "器の行が残っているのに登録簿の行が消えた");
    assert_eq!(locks, 0, "戻しで場所の記録の錠が落ちていない");
    assert_eq!(container_locks, 2, "器が残るのに器の錠が落ちた");
}

#[tokio::test]
async fn place_lock_down_drops_when_unused() {
    let (fresh, drop) = fresh_db().await;
    let result = async {
        apply_all(&fresh).await?;
        sqlx::raw_sql(PLACES_DOWN)
            .execute(&fresh)
            .await
            .map_err(|e| format!("戻しが当たらない: {e}"))?;
        let table: (bool,) = sqlx::query_as("SELECT to_regclass('core.place') IS NULL")
            .fetch_one(&fresh)
            .await
            .map_err(|e| e.to_string())?;
        let reg = count(
            &fresh,
            "SELECT count(*) FROM core.source WHERE logical_source = 's01-place'",
        )
        .await;
        // 2 回目の戻しも当たる
        sqlx::raw_sql(PLACES_DOWN)
            .execute(&fresh)
            .await
            .map_err(|e| format!("2 回目の戻しが当たらない: {e}"))?;
        Ok::<_, String>((table.0, reg))
    }
    .await;
    drop.await;
    let (table_gone, reg) = result.unwrap();
    assert!(table_gone, "何も使っていないのに器の表が残っている");
    assert_eq!(reg, 0, "何も使っていないのに登録簿の行が残っている");
}

// Scenario: 器の表は書き換えも削除も切り詰めもできない
#[tokio::test]
async fn place_container_append_only() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO core.place (id, user_id) VALUES ($1, $2)")
        .bind(id)
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();

    let other = Uuid::new_v4();
    for stmt in [
        format!("UPDATE core.place SET user_id = '{other}' WHERE id = '{id}'"),
        format!("UPDATE core.place SET id = '{other}' WHERE id = '{id}'"),
        format!("DELETE FROM core.place WHERE id = '{id}'"),
        "TRUNCATE core.place".to_string(),
        "TRUNCATE core.place CASCADE".to_string(),
    ] {
        assert!(
            sqlx::raw_sql(&stmt).execute(&pool).await.is_err(),
            "器の表が変えられた: {stmt}"
        );
    }
    let (kept,): (Uuid,) = sqlx::query_as("SELECT user_id FROM core.place WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kept, user, "拒まれたのに器の行が変わっている");
}

// ================================================================ 場所の記録の錠

/// 場所の記録を SQL で直接置く。座標は原文と解析済みの両方に入れる。
async fn put_place_record(pool: &sqlx::PgPool, user: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, sensitivity, content_hash, raw, payload)
         VALUES ($1, $2, 's01-place', 'authored', '2026-09-15T02:00:00Z', 540, 'Asia/Tokyo', 1, 1,
                 $3, '{\"lat\":35.0,\"lon\":139.0}', '{\"lat\":35.0,\"lon\":139.0}')",
    )
    .bind(id)
    .bind(user)
    .bind(format!("place-hash-{id}"))
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn put_ledger(
    pool: &sqlx::PgPool,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: Uuid,
    user: Uuid,
) {
    let _ = pool;
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, 's01-place', 'event', 'check')",
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

/// 文を実行して拒まれることを見る。
async fn assert_rejected(pool: &sqlx::PgPool, sql: &str, id: Uuid) {
    let r = sqlx::query(sql).bind(id).execute(pool).await;
    assert!(r.is_err(), "拒まれるはずの文が通った: {sql}");
}

// Scenario: 場所の記録の座標を書き換える文は拒まれる
#[tokio::test]
async fn place_lock_rejects_rewriting_the_coordinates() {
    let pool = testdb::pool().await;
    let id = put_place_record(&pool, testdb::user()).await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET payload = jsonb_set(payload, '{lat}', '36.0') WHERE id = $1",
        id,
    )
    .await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET raw = '{\"lat\":36.0,\"lon\":139.0}' WHERE id = $1",
        id,
    )
    .await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET content_hash = 'forged' WHERE id = $1",
        id,
    )
    .await;
    let (lat,): (String,) = sqlx::query_as("SELECT payload->>'lat' FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(lat, "35.0", "拒まれたのに緯度が変わっている");
}

// Scenario: 場所の記録の書いた日時は書き換えられない
#[tokio::test]
async fn place_lock_rejects_rewriting_the_written_time() {
    let pool = testdb::pool().await;
    let id = put_place_record(&pool, testdb::user()).await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET event_time = '2000-01-01T00:00:00Z' WHERE id = $1",
        id,
    )
    .await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET ingest_time = '2000-01-01T00:00:00Z' WHERE id = $1",
        id,
    )
    .await;
}

// Scenario: 場所の記録の利用者は書き換えられない
#[tokio::test]
async fn place_lock_rejects_rewriting_the_user() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let id = put_place_record(&pool, user).await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET user_id = gen_random_uuid() WHERE id = $1",
        id,
    )
    .await;
    let (kept,): (Uuid,) = sqlx::query_as("SELECT user_id FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kept, user);
}

// Scenario: 場所の記録の行は削除できない
#[tokio::test]
async fn place_lock_rejects_deleting_the_row() {
    let pool = testdb::pool().await;
    let id = put_place_record(&pool, testdb::user()).await;
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

// Scenario: 他の記録を場所の記録へ付け替えられない
#[tokio::test]
async fn place_lock_rejects_reassigning_another_record() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let src = testdb::source(&pool, "place-lock", 3600).await;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ($1, $2, $3, 'authored', '2026-09-15T02:00:00Z', 540, 'Asia/Tokyo', 1,
                 $4, '{}', '{}')",
    )
    .bind(id)
    .bind(user)
    .bind(&src)
    .bind(format!("other-hash-{id}"))
    .execute(&pool)
    .await
    .unwrap();
    assert_rejected(
        &pool,
        "UPDATE core.event SET logical_source = 's01-place' WHERE id = $1",
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

// Scenario: 場所の記録に削除の印を付けられる
#[tokio::test]
async fn place_lock_allows_the_deletion_mark() {
    let pool = testdb::pool().await;
    let id = put_place_record(&pool, testdb::user()).await;
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'check' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("削除の印まで止めている（FR-50）");
}

// Scenario: 場所の記録の感度を変えられる
#[tokio::test]
async fn place_lock_allows_changing_the_sensitivity() {
    let pool = testdb::pool().await;
    let id = put_place_record(&pool, testdb::user()).await;
    sqlx::query("UPDATE core.event SET sensitivity = 3 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("感度まで止めている（PERM-2）");
}

// Scenario: 台帳のある場所の記録の消去は通る
#[tokio::test]
async fn place_lock_allows_erasure_with_its_ledger() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let id = put_place_record(&pool, user).await;
    let mut tx = pool.begin().await.unwrap();
    put_ledger(&pool, &mut tx, id, user).await;
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

// Scenario: 台帳の無い場所の記録の消去は拒まれる
#[tokio::test]
async fn place_lock_rejects_erasure_without_a_ledger() {
    let pool = testdb::pool().await;
    let id = put_place_record(&pool, testdb::user()).await;
    assert_rejected(
        &pool,
        "UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1",
        id,
    )
    .await;
    assert_ne!(raw_of(&pool, id).await, "", "拒まれたのに原文が消えている");
}

// Scenario: 別の記録の台帳の行では場所の記録の消去は通らない
#[tokio::test]
async fn place_lock_rejects_erasure_with_another_records_ledger() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let id = put_place_record(&pool, user).await;
    let mut tx = pool.begin().await.unwrap();
    put_ledger(&pool, &mut tx, Uuid::new_v4(), user).await;
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        tx.commit().await.is_err(),
        "別の記録の台帳 1 行で場所の記録が消去できた"
    );
    assert_ne!(raw_of(&pool, id).await, "", "拒まれたのに原文が消えている");
}

// Scenario: 台帳の行があっても消去の形でない場所の記録の書き換えは拒まれる
#[tokio::test]
async fn place_lock_rejects_a_forged_erasure() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let id = put_place_record(&pool, user).await;
    let mut tx = pool.begin().await.unwrap();
    put_ledger(&pool, &mut tx, id, user).await;
    sqlx::query(
        "UPDATE core.event SET raw = '', payload = '{\"lat\":1.0,\"lon\":2.0}' WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .unwrap();
    assert!(
        tx.commit().await.is_err(),
        "消去の顔で解析済みを差し替えられた"
    );
    assert_ne!(raw_of(&pool, id).await, "", "拒まれたのに原文が消えている");
}

// Scenario: 場所の錠を足しても主張の錠は変わらない
#[tokio::test]
async fn place_lock_leaves_the_claim_lock_unchanged() {
    let pool = testdb::pool().await;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, sensitivity, content_hash, raw, payload)
         VALUES ($1, $2, 's01-attribute', 'authored', '2026-09-15T02:00:00Z', 540, 'Asia/Tokyo',
                 1, 2, $3, '{\"value\":\"東京\"}', '{\"value\":\"東京\"}')",
    )
    .bind(id)
    .bind(testdb::user())
    .bind(format!("claim-hash-{id}"))
    .execute(&pool)
    .await
    .unwrap();
    assert_rejected(
        &pool,
        "UPDATE core.event SET payload = '{\"value\":\"京都\"}' WHERE id = $1",
        id,
    )
    .await;
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'check' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("主張の削除の印まで止めている");
}

// ---------------------------------------------------------------- 器の口（Task 2 / design D1 / D15）

mod container_endpoint {
    use crate::places::{PlaceCreateRequest, PlaceError};
    use crate::{places_post, testdb, App};
    use axum::{extract::State, http::HeaderMap, http::StatusCode, Json};
    use uuid::Uuid;

    const TOKEN: &str = "test-token-0123456789abcdef";

    async fn app() -> App {
        App::for_test(testdb::pool().await, TOKEN)
    }

    fn auth() -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            "authorization",
            format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
        );
        h
    }

    fn req(id: Uuid, user: Uuid) -> Json<PlaceCreateRequest> {
        Json(PlaceCreateRequest {
            id,
            user_id: Some(user),
        })
    }

    async fn rows_of(app: &App, id: Uuid) -> Vec<Uuid> {
        let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT user_id FROM core.place WHERE id = $1")
            .bind(id)
            .fetch_all(&app.pool)
            .await
            .unwrap();
        rows.into_iter().map(|r| r.0).collect()
    }

    #[tokio::test]
    // Scenario: 渡した識別子で場所の器ができる
    async fn place_container_endpoint_creates_the_given_id() {
        let app = app().await;
        let (user, id) = (testdb::user(), Uuid::new_v4());
        let Json(out) = places_post(State(app.clone()), auth(), req(id, user))
            .await
            .expect("器が作れる");
        assert_eq!(out.id, id, "渡した識別子がそのまま返る");
        assert_eq!(rows_of(&app, id).await, vec![user]);
    }

    #[tokio::test]
    // Scenario: 同じ識別子で器を 2 回作っても 1 つ
    async fn place_container_endpoint_is_idempotent() {
        let app = app().await;
        let (user, id) = (testdb::user(), Uuid::new_v4());
        for _ in 0..2 {
            let Json(out) = places_post(State(app.clone()), auth(), req(id, user))
                .await
                .expect("2 回とも受け付ける");
            assert_eq!(out.id, id);
        }
        assert_eq!(rows_of(&app, id).await.len(), 1);
    }

    #[tokio::test]
    // Scenario: 別の利用者の器の識別子では作れない
    async fn place_container_endpoint_rejects_another_users_id() {
        let app = app().await;
        let (a, b, id) = (testdb::user(), testdb::user(), Uuid::new_v4());
        let _ = places_post(State(app.clone()), auth(), req(id, a))
            .await
            .expect("A の器");
        let (code, Json(body)) = places_post(State(app.clone()), auth(), req(id, b))
            .await
            .expect_err("別の利用者の識別子で作れた");
        assert_eq!(code, StatusCode::BAD_REQUEST);
        assert_eq!(body.error, PlaceError::PlaceIdTaken);
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            serde_json::json!({ "error": "place_id_taken" })
        );
        assert_eq!(rows_of(&app, id).await, vec![a], "行は A の 1 つのまま");
    }

    #[tokio::test]
    async fn place_container_endpoint_requires_the_token() {
        let app = app().await;
        let (code, _) = places_post(
            State(app),
            HeaderMap::new(),
            req(Uuid::new_v4(), testdb::user()),
        )
        .await
        .expect_err("資格情報なしで器が作れた");
        assert_eq!(code, StatusCode::UNAUTHORIZED);
    }
}
