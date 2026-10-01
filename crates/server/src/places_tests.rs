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

// ---------------------------------------------------------------- 場所の記録の取り込み（Task 3 / design D2 / D3 / D4 / D11）

mod ingest_endpoint {
    use crate::places::{self, PlaceCreateRequest};
    use crate::{ingest, places_post, testdb, App, IngestResult};
    use axum::{extract::State, http::HeaderMap, Json};
    use unicode_normalization::UnicodeNormalization as _;
    use uuid::Uuid;

    pub(super) const TOKEN: &str = "test-token-0123456789abcdef";
    /// 128 bit を base64url で書いた 22 文字（design D3）
    pub(super) const NONCE: &str = "Zm9vYmFyYmF6cXV4MTIzNDU2";
    /// 64 bit（11 文字）
    pub(super) const SHORT_NONCE: &str = "Zm9vYmFyYmE";
    pub(super) const AT: &str = "2026-10-01T02:00:00Z";

    pub(super) async fn app() -> App {
        App::for_test(testdb::pool().await, TOKEN)
    }

    pub(super) fn auth() -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            "authorization",
            format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
        );
        h
    }

    /// 器を作る（`POST /places`）
    pub(super) async fn container(app: &App, user: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        let _ = places_post(
            State(app.clone()),
            auth(),
            Json(PlaceCreateRequest {
                id,
                user_id: Some(user),
            }),
        )
        .await
        .expect("器が作れる");
        id
    }

    /// 原文を組む（design D2 の形）。`extra` は項目ごとの欄
    pub(super) fn raw_of(id: Uuid, place: Uuid, nonce: &str, extra: serde_json::Value) -> String {
        let mut v = serde_json::json!({ "record": id, "place": place, "nonce": nonce });
        for (k, val) in extra.as_object().unwrap() {
            v[k] = val.clone();
        }
        v.to_string()
    }

    pub(super) fn name_raw(id: Uuid, place: Uuid, name: &str) -> String {
        raw_of(
            id,
            place,
            NONCE,
            serde_json::json!({ "field": "name", "name": name }),
        )
    }

    pub(super) fn coord_raw(
        id: Uuid,
        place: Uuid,
        lat: f64,
        change: &str,
        valid_from: serde_json::Value,
        supersedes: Option<Uuid>,
    ) -> String {
        raw_of(
            id,
            place,
            NONCE,
            serde_json::json!({
                "field": "coord", "lat": lat, "lon": 139.767125, "change": change,
                "valid_from": valid_from, "supersedes": supersedes,
            }),
        )
    }

    pub(super) fn first_raw(id: Uuid, place: Uuid) -> String {
        coord_raw(id, place, 35.681236, "first", serde_json::Value::Null, None)
    }

    pub(super) fn radius_raw(id: Uuid, place: Uuid, radius: serde_json::Value) -> String {
        raw_of(
            id,
            place,
            NONCE,
            serde_json::json!({ "field": "radius", "radius_m": radius }),
        )
    }

    pub(super) fn item(id: Uuid, user: Uuid, raw: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "user_id": user, "logical_source": places::SOURCE,
            "external_id": null, "device_id": null, "origin": "authored",
            "event_time": AT, "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
            "schema_version": 1, "raw": raw, "payload": {},
        })
    }

    pub(super) async fn send_item(app: &App, item: serde_json::Value) -> IngestResult {
        let (_, Json(mut res)) = crate::ingest(State(app.clone()), auth(), Json(item_array(item)))
            .await
            .expect("取り込み口");
        res.pop().expect("1 件ぶんの結果")
    }

    pub(super) fn item_array(item: serde_json::Value) -> serde_json::Value {
        serde_json::json!([item])
    }

    /// 原文の `record` を記録の識別子にして送る（原文が JSON でないときだけ新しい識別子）
    pub(super) async fn send(app: &App, user: Uuid, raw: &str) -> IngestResult {
        let id = serde_json::from_str::<serde_json::Value>(raw)
            .ok()
            .and_then(|v| v["record"].as_str().and_then(|s| Uuid::parse_str(s).ok()))
            .unwrap_or_else(Uuid::new_v4);
        send_item(app, item(id, user, raw)).await
    }

    /// 結果の理由の種別を文字列で見る
    pub(super) fn error_of(r: &IngestResult) -> String {
        assert!(!r.accepted, "受理されている");
        serde_json::to_value(r).unwrap()["error"]
            .as_str()
            .expect("理由の種別")
            .to_string()
    }

    pub(super) async fn rejects(app: &App, user: Uuid, raw: &str, want: &str) {
        let r = send(app, user, raw).await;
        assert_eq!(error_of(&r), want, "原文: {raw}");
    }

    /// 送った 1 件が受理された
    pub(super) fn assert_accepted(r: &IngestResult, why: &str) {
        assert!(r.accepted, "{why}: 受理されていない {:?}", r.error);
    }

    // Scenario: 無い器を指す場所の記録は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_an_unknown_place() {
        let app = app().await;
        let user = testdb::user();
        let id = Uuid::new_v4();
        rejects(
            &app,
            user,
            &name_raw(id, Uuid::new_v4(), "自宅"),
            "unknown_place",
        )
        .await;
    }

    // Scenario: 別の利用者の器を指す場所の記録は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_another_users_place() {
        let app = app().await;
        let (a, b) = (testdb::user(), testdb::user());
        let place = container(&app, a).await;
        rejects(
            &app,
            b,
            &name_raw(Uuid::new_v4(), place, "自宅"),
            "unknown_place",
        )
        .await;
    }

    // Scenario: 空の名前は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_a_blank_name() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        rejects(
            &app,
            user,
            &name_raw(Uuid::new_v4(), place, " \u{3000} "),
            "invalid_place_name",
        )
        .await;
    }

    // Scenario: 範囲の外の緯度は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_latitude_out_of_range() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let raw = coord_raw(
            Uuid::new_v4(),
            place,
            91.0,
            "first",
            serde_json::Value::Null,
            None,
        );
        rejects(&app, user, &raw, "invalid_coordinate").await;
    }

    // Scenario: WGS84 でない座標は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_a_non_wgs84_crs() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let mut it = item(id, user, &first_raw(id, place));
        it["crs"] = serde_json::json!("EPSG:6668");
        let r = send_item(&app, it).await;
        assert_eq!(error_of(&r), "invalid_coordinate");
    }

    // 座標でない記録は座標系の指定があっても通る（座標系は座標の記録のものだけ）
    #[tokio::test]
    async fn place_ingest_rejects_only_coordinates_for_the_crs() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let mut it = item(id, user, &name_raw(id, place, "自宅"));
        it["crs"] = serde_json::json!("EPSG:6668");
        assert_accepted(&send_item(&app, it).await, "名前の記録");
    }

    // 座標の数でない・欠けの区別と、ほかの値の形
    #[tokio::test]
    async fn place_ingest_rejects_odd_coordinates() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let mk = |lat: serde_json::Value, lon: serde_json::Value| {
            raw_of(
                Uuid::new_v4(),
                place,
                NONCE,
                serde_json::json!({ "field": "coord", "lat": lat, "lon": lon, "change": "first",
                                     "valid_from": null, "supersedes": null }),
            )
        };
        rejects(
            &app,
            user,
            &mk(serde_json::json!("35.6"), serde_json::json!(139.7)),
            "invalid_coordinate",
        )
        .await;
        rejects(
            &app,
            user,
            &mk(serde_json::json!(35.6), serde_json::json!(180.5)),
            "invalid_coordinate",
        )
        .await;
        rejects(
            &app,
            user,
            &mk(serde_json::json!(-90.5), serde_json::json!(0)),
            "invalid_coordinate",
        )
        .await;
    }

    // Scenario: 座標を持つ場所に初めての座標は書けない
    #[tokio::test]
    async fn place_ingest_rejects_first_on_a_place_with_a_coordinate() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        assert_accepted(
            &send(&app, user, &first_raw(Uuid::new_v4(), place)).await,
            "1 件目",
        );
        let id = Uuid::new_v4();
        let again = coord_raw(id, place, 35.7, "first", serde_json::Value::Null, None);
        rejects(&app, user, &again, "invalid_coord_change").await;
    }

    // Scenario: 座標の無い場所は移れない
    #[tokio::test]
    async fn place_ingest_rejects_move_on_a_place_without_a_coordinate() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let raw = coord_raw(
            Uuid::new_v4(),
            place,
            35.7,
            "move",
            serde_json::json!({ "precision": "month", "date": "2026-04" }),
            None,
        );
        rejects(&app, user, &raw, "invalid_coord_change").await;
    }

    // Scenario: 直す先の無い直す記録は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_fix_without_a_target() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        assert_accepted(
            &send(&app, user, &first_raw(Uuid::new_v4(), place)).await,
            "1 件目",
        );
        let raw = coord_raw(
            Uuid::new_v4(),
            place,
            35.7,
            "fix",
            serde_json::Value::Null,
            None,
        );
        rejects(&app, user, &raw, "invalid_coord_change").await;
    }

    // 変え方と欄の組が合わないもの（spec の表）
    #[tokio::test]
    async fn place_ingest_rejects_mismatched_change_fields() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let vf = serde_json::json!({ "precision": "year", "date": "2026" });
        let target = Uuid::new_v4();
        let cases = [
            ("unknown", serde_json::Value::Null, None),
            ("move", serde_json::Value::Null, None),
            ("first", vf.clone(), None),
            ("fix", vf.clone(), Some(target)),
            ("first", serde_json::Value::Null, Some(target)),
            ("move", vf.clone(), Some(target)),
        ];
        for (change, valid_from, supersedes) in cases {
            let raw = coord_raw(Uuid::new_v4(), place, 35.7, change, valid_from, supersedes);
            rejects(&app, user, &raw, "invalid_coord_change").await;
        }
    }

    // Scenario: 座標をすべて消した場所にも初めての座標は書けない
    #[tokio::test]
    async fn place_ingest_rejects_first_after_every_coordinate_is_marked_deleted() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let coord = Uuid::new_v4();
        let it = item(coord, user, &first_raw(coord, place));
        assert_accepted(&send_item(&app, it).await, "1 件目");
        sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
            .bind(coord)
            .execute(&app.pool)
            .await
            .unwrap();
        let again = coord_raw(
            Uuid::new_v4(),
            place,
            35.7,
            "first",
            serde_json::Value::Null,
            None,
        );
        rejects(&app, user, &again, "invalid_coord_change").await;
    }

    // Scenario: 座標をすべて消した場所にも初めての座標は書けない
    #[tokio::test]
    async fn place_ingest_rejects_first_after_every_coordinate_is_erased() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let coord = Uuid::new_v4();
        let it = item(coord, user, &first_raw(coord, place));
        assert_accepted(&send_item(&app, it).await, "1 件目");
        let mut tx = app.pool.begin().await.unwrap();
        sqlx::query(
            "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
             VALUES ($1, $2, 's01-place', 'event', 'test')",
        )
        .bind(coord)
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query("UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1")
            .bind(coord)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let again = coord_raw(
            Uuid::new_v4(),
            place,
            35.7,
            "first",
            serde_json::Value::Null,
            None,
        );
        rejects(&app, user, &again, "invalid_coord_change").await;
    }

    // Scenario: 移ったの精度と日付が合わなければ受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_a_move_with_a_mismatched_valid_from() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        assert_accepted(
            &send(&app, user, &first_raw(Uuid::new_v4(), place)).await,
            "1 件目",
        );
        let raw = coord_raw(
            Uuid::new_v4(),
            place,
            35.7,
            "move",
            serde_json::json!({ "precision": "year", "date": "2026-04-01" }),
            None,
        );
        rejects(&app, user, &raw, "invalid_valid_from").await;
    }

    /// 座標の記録を 1 件入れて、その識別子を返す
    async fn put_first(app: &App, user: Uuid, place: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        let it = item(id, user, &first_raw(id, place));
        assert_accepted(&send_item(app, it).await, "座標の記録");
        id
    }

    // Scenario: 別の場所の座標は直せない
    #[tokio::test]
    async fn place_ingest_rejects_fixing_another_places_coordinate() {
        let app = app().await;
        let user = testdb::user();
        let (a, b) = (container(&app, user).await, container(&app, user).await);
        let in_a = put_first(&app, user, a).await;
        put_first(&app, user, b).await;
        let raw = coord_raw(
            Uuid::new_v4(),
            b,
            35.7,
            "fix",
            serde_json::Value::Null,
            Some(in_a),
        );
        rejects(&app, user, &raw, "invalid_coord_supersedes").await;
    }

    // Scenario: 座標でない記録は直せない
    #[tokio::test]
    async fn place_ingest_rejects_fixing_a_record_that_is_not_a_coordinate() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        put_first(&app, user, place).await;
        let name = Uuid::new_v4();
        let it = item(name, user, &name_raw(name, place, "自宅"));
        assert_accepted(&send_item(&app, it).await, "名前の記録");
        let raw = coord_raw(
            Uuid::new_v4(),
            place,
            35.7,
            "fix",
            serde_json::Value::Null,
            Some(name),
        );
        rejects(&app, user, &raw, "invalid_coord_supersedes").await;
    }

    // 直す先が無い・別の利用者・自分自身
    #[tokio::test]
    async fn place_ingest_rejects_fixing_a_missing_or_foreign_or_own_record() {
        let app = app().await;
        let (user, other) = (testdb::user(), testdb::user());
        let place = container(&app, user).await;
        put_first(&app, user, place).await;
        let other_place = container(&app, other).await;
        let theirs = put_first(&app, other, other_place).await;
        let own = Uuid::new_v4();
        for target in [Uuid::new_v4(), theirs, own] {
            let raw = coord_raw(
                own,
                place,
                35.7,
                "fix",
                serde_json::Value::Null,
                Some(target),
            );
            let r = send_item(&app, item(own, user, &raw)).await;
            assert_eq!(error_of(&r), "invalid_coord_supersedes");
        }
    }

    // Scenario: 消した座標の記録を直す先に指せる
    #[tokio::test]
    async fn place_ingest_accepts_fixing_a_coordinate_marked_deleted() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let coord = put_first(&app, user, place).await;
        sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
            .bind(coord)
            .execute(&app.pool)
            .await
            .unwrap();
        let raw = coord_raw(
            Uuid::new_v4(),
            place,
            35.7,
            "fix",
            serde_json::Value::Null,
            Some(coord),
        );
        assert_accepted(&send(&app, user, &raw).await, "消した座標を直す");
    }

    // 本文を消去した座標の記録も直す先に指せる（spec。消去で場所は読めないので理由では断らない）
    #[tokio::test]
    async fn place_ingest_accepts_fixing_an_erased_coordinate() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let coord = put_first(&app, user, place).await;
        let mut tx = app.pool.begin().await.unwrap();
        sqlx::query(
            "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
             VALUES ($1, $2, 's01-place', 'event', 'test')",
        )
        .bind(coord)
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query("UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1")
            .bind(coord)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let raw = coord_raw(
            Uuid::new_v4(),
            place,
            35.7,
            "fix",
            serde_json::Value::Null,
            Some(coord),
        );
        assert_accepted(&send(&app, user, &raw).await, "消去した座標を直す");
    }

    // 移った・直すが正しく通る
    #[tokio::test]
    async fn place_ingest_rejects_nothing_valid_for_move_and_fix() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let first = put_first(&app, user, place).await;
        let mv = coord_raw(
            Uuid::new_v4(),
            place,
            35.7,
            "move",
            serde_json::json!({ "precision": "month", "date": "2026-04" }),
            None,
        );
        assert_accepted(&send(&app, user, &mv).await, "移った");
        let fix = coord_raw(
            Uuid::new_v4(),
            place,
            35.8,
            "fix",
            serde_json::Value::Null,
            Some(first),
        );
        assert_accepted(&send(&app, user, &fix).await, "直す");
    }

    // Scenario: 広すぎる広さは受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_a_radius_too_wide() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let raw = radius_raw(Uuid::new_v4(), place, serde_json::json!(5001));
        rejects(&app, user, &raw, "invalid_radius").await;
    }

    // 広さが整数でない
    #[tokio::test]
    async fn place_ingest_rejects_a_radius_that_is_not_an_integer() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        for bad in [
            serde_json::json!(100.5),
            serde_json::json!("100"),
            serde_json::json!(null),
        ] {
            let raw = radius_raw(Uuid::new_v4(), place, bad);
            rejects(&app, user, &raw, "invalid_radius").await;
        }
    }

    // Scenario: 何を書くものか分からない場所の記録は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_an_unknown_field() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let raw = raw_of(
            Uuid::new_v4(),
            place,
            NONCE,
            serde_json::json!({ "field": "color", "color": "red" }),
        );
        rejects(&app, user, &raw, "malformed_place_record").await;
    }

    // 原文の形が壊れているもの（JSON でない・欄が欠ける・型が違う・記録の識別子が違う）
    #[tokio::test]
    async fn place_ingest_rejects_malformed_raw() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let good = name_raw(id, place, "自宅");
        let mut no_place: serde_json::Value = serde_json::from_str(&good).unwrap();
        no_place.as_object_mut().unwrap().remove("place");
        let mut name_number: serde_json::Value = serde_json::from_str(&good).unwrap();
        name_number["name"] = serde_json::json!(5);
        let other_record = name_raw(Uuid::new_v4(), place, "自宅");
        for bad in [
            "これは JSON でない".to_string(),
            "[]".to_string(),
            no_place.to_string(),
            name_number.to_string(),
            other_record,
        ] {
            let r = send_item(&app, item(id, user, &bad)).await;
            assert_eq!(error_of(&r), "malformed_place_record", "{bad}");
        }
    }

    // Scenario: 乱数が短い場所の記録は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_a_short_nonce() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let raw = raw_of(
            id,
            place,
            SHORT_NONCE,
            serde_json::json!({ "field": "name", "name": "自宅" }),
        );
        let r = send_item(&app, item(id, user, &raw)).await;
        assert_eq!(error_of(&r), "malformed_place_record");
    }

    // Scenario: 本人が書いたでない場所の記録は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_a_record_that_is_not_authored() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let mut it = item(id, user, &name_raw(id, place, "自宅"));
        it["origin"] = serde_json::json!("derived");
        assert_eq!(
            error_of(&send_item(&app, it).await),
            "place_record_not_authored"
        );
        // 端末識別子を持つものも同じ
        let mut it = item(id, user, &name_raw(id, place, "自宅"));
        it["device_id"] = serde_json::json!("dev-1");
        assert_eq!(
            error_of(&send_item(&app, it).await),
            "place_record_not_authored"
        );
        // 由来が「収集した」でも主張の錠ではなくこの種別（一般の検査より先に当たる）
        let mut it = item(id, user, &name_raw(id, place, "自宅"));
        it["origin"] = serde_json::json!("collected");
        assert_eq!(
            error_of(&send_item(&app, it).await),
            "place_record_not_authored"
        );
    }

    // Scenario: 外部識別子を持つ場所の記録は受け付けない
    #[tokio::test]
    async fn place_ingest_rejects_an_external_id() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        for field in ["external_id", "external_ref"] {
            let mut it = item(id, user, &name_raw(id, place, "自宅"));
            it[field] = serde_json::json!("ext-1");
            assert_eq!(
                error_of(&send_item(&app, it).await),
                "place_record_has_external_id",
                "{field}"
            );
        }
    }

    // Scenario: 場所の記録の拒否の応答に値が含まれない
    #[tokio::test]
    async fn place_ingest_rejects_without_echoing_the_values() {
        let app = app().await;
        let user = testdb::user();
        let id = Uuid::new_v4();
        let raw = raw_of(
            id,
            Uuid::new_v4(),
            NONCE,
            serde_json::json!({ "field": "name", "name": "秘密の名前", "note": "秘密の補足" }),
        );
        let r = send_item(&app, item(id, user, &raw)).await;
        let body = serde_json::to_string(&r).unwrap();
        assert!(!r.accepted, "器が無いのに受理されている");
        assert!(!body.contains("秘密の名前"), "名前が応答に載っている");
        assert!(!body.contains("秘密の補足"), "補足が応答に載っている");
    }

    // 範囲外の地域のずれは格納の前に断る（読み出しから消える値を入れない）
    #[tokio::test]
    async fn place_ingest_rejects_a_timezone_offset_out_of_range() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let mut it = item(id, user, &name_raw(id, place, "自宅"));
        it["tz_offset_min"] = serde_json::json!(1440);
        assert_eq!(
            error_of(&send_item(&app, it).await),
            "malformed_place_record"
        );
    }

    // NUL を含む名前・補足は格納の前に断る（jsonb に入らず 500 になる）
    #[tokio::test]
    async fn place_ingest_rejects_control_characters() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        rejects(
            &app,
            user,
            &name_raw(Uuid::new_v4(), place, "a\u{0}b"),
            "invalid_place_name",
        )
        .await;
        let note = raw_of(
            Uuid::new_v4(),
            place,
            NONCE,
            serde_json::json!({ "field": "note", "note": "a\u{0}b" }),
        );
        rejects(&app, user, &note, "malformed_place_record").await;
    }

    // ---- 3.2 格納の形

    async fn stored(app: &App, id: Uuid) -> (String, serde_json::Value, i32, String) {
        sqlx::query_as(
            "SELECT raw, payload, sensitivity::int4, origin FROM core.event WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap()
    }

    // Scenario: 同じ場所の記録の再送は増えない
    #[tokio::test]
    async fn place_ingest_stores_a_resend_once() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let it = item(id, user, &name_raw(id, place, "自宅"));
        let first = send_item(&app, it.clone()).await;
        let second = send_item(&app, it).await;
        assert_accepted(&first, "1 回目");
        assert_accepted(&second, "2 回目");
        assert!(second.duplicate, "2 回目は重複として返る");
        let n: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = 's01-place'",
        )
        .bind(user)
        .fetch_one(&app.pool)
        .await
        .unwrap();
        assert_eq!(n.0, 1, "記録は 1 件のまま");
    }

    // 座標の「初めての座標」も再送できる（押し直しで断られない）
    #[tokio::test]
    async fn place_ingest_stores_a_resent_first_coordinate() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let it = item(id, user, &first_raw(id, place));
        assert_accepted(&send_item(&app, it.clone()).await, "1 回目");
        let again = send_item(&app, it).await;
        assert_accepted(&again, "再送");
        assert!(again.duplicate);
    }

    // Scenario: 場所の記録の原文が 1 バイトも変わらずに残る
    #[tokio::test]
    async fn place_ingest_stores_the_raw_byte_for_byte() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        // 並び・空白・数の書き方を普通でなくする（正規化されると一致しない）
        let raw = format!(
            r#"{{ "record" : "{id}", "place":"{place}", "nonce":"{NONCE}", "field":"coord",
 "lat": 35.6800, "lon":139.70, "change":"first", "valid_from":null, "supersedes":null }}"#
        );
        assert_accepted(&send_item(&app, item(id, user, &raw)).await, "座標");
        assert_eq!(stored(&app, id).await.0, raw);
    }

    // Scenario: 原文と食い違う解析済みを送っても原文の座標で格納される
    #[tokio::test]
    async fn place_ingest_stores_the_coordinate_of_the_raw() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let raw = coord_raw(id, place, 35.68, "first", serde_json::Value::Null, None);
        let mut it = item(id, user, &raw);
        it["payload"] = serde_json::json!({ "field": "coord", "lat": 34.70, "lon": 135.5 });
        assert_accepted(&send_item(&app, it).await, "座標");
        let (_, payload, _, _) = stored(&app, id).await;
        assert_eq!(payload["lat"], serde_json::json!(35.68));
    }

    // 名前は NFC で解析済みに入り、原文は NFD のまま残る
    #[tokio::test]
    async fn place_ingest_stores_the_name_composed_in_the_parsed_value() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        let nfd: String = "がっこう".nfd().collect();
        let raw = name_raw(id, place, &nfd);
        assert_accepted(&send_item(&app, item(id, user, &raw)).await, "名前");
        let (stored_raw, payload, _, _) = stored(&app, id).await;
        assert_eq!(payload["name"], serde_json::json!("がっこう"));
        assert!(stored_raw.contains(&nfd), "原文は NFD のまま");
    }

    // Scenario: 場所の記録の乱数は解析済みに写らない
    #[tokio::test]
    async fn place_ingest_stores_no_nonce_in_the_parsed_value() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        assert_accepted(
            &send_item(&app, item(id, user, &first_raw(id, place))).await,
            "座標",
        );
        let (_, payload, _, _) = stored(&app, id).await;
        assert!(
            !payload.to_string().contains(NONCE),
            "解析済みに乱数が写っている"
        );
        // 値を読み出す経路（読み出した場所の列）にも写らない
        let row: (String,) = sqlx::query_as(
            "SELECT (e.id, e.user_id, e.payload, e.event_time, e.content_hash)::text
               FROM core.event e WHERE e.id = $1",
        )
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
        assert!(!row.0.contains(NONCE));
        assert!(payload.get("nonce").is_none());
        assert_eq!(payload["place"], serde_json::json!(place));
        assert_eq!(payload["record"], serde_json::json!(id));
        assert_eq!(payload["change"], serde_json::json!("first"));
    }

    // Scenario: 消去後に残る列と正しい座標から場所の記録の鍵を作り直せない
    #[tokio::test]
    async fn place_ingest_stores_a_key_that_cannot_be_rebuilt_without_the_nonce() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let id = Uuid::new_v4();
        assert_accepted(
            &send_item(&app, item(id, user, &first_raw(id, place))).await,
            "座標",
        );
        let mut tx = app.pool.begin().await.unwrap();
        sqlx::query(
            "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
             VALUES ($1, $2, 's01-place', 'event', 'test')",
        )
        .bind(id)
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query("UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let (kept_time, kept_hash): (chrono::DateTime<chrono::Utc>, String) =
            sqlx::query_as("SELECT event_time, content_hash FROM core.event WHERE id = $1")
                .bind(id)
                .fetch_one(&app.pool)
                .await
                .unwrap();
        // 乱数を知らないまま、同じ場所・座標・変え方から原文を組む（乱数だけが違う）
        let guess = raw_of(
            id,
            place,
            "AAAAAAAAAAAAAAAAAAAAAA",
            serde_json::json!({
                "field": "coord", "lat": 35.681236, "lon": 139.767125, "change": "first",
                "valid_from": null, "supersedes": null,
            }),
        );
        assert_ne!(
            ingest::content_hash_of(places::SOURCE, kept_time, &guess),
            kept_hash,
            "乱数を知らなくても鍵が作り直せる"
        );
    }

    // ---- 3.3 同時の `first`

    // 同時に `first` を 2 本送っても座標の記録は 1 本だけ入る（D4 の錠）
    #[tokio::test]
    async fn place_ingest_first_is_serialized() {
        let app = app().await;
        for _ in 0..5 {
            let user = testdb::user();
            let place = container(&app, user).await;
            let ids = [Uuid::new_v4(), Uuid::new_v4()];
            let handles: Vec<_> = ids
                .iter()
                .map(|id| {
                    let (app, id) = (app.clone(), *id);
                    let it = item(id, user, &first_raw(id, place));
                    tokio::spawn(async move { send_item(&app, it).await })
                })
                .collect();
            let mut results = Vec::new();
            for h in handles {
                results.push(h.await.unwrap());
            }
            let accepted = results.iter().filter(|r| r.accepted).count();
            assert_eq!(accepted, 1, "同時の first が 2 本とも通った / 0 本になった");
            let n: (i64,) = sqlx::query_as(
                "SELECT count(*) FROM core.event
                  WHERE user_id = $1 AND payload->>'field' = 'coord' AND payload->>'place' = $2",
            )
            .bind(user)
            .bind(place.to_string())
            .fetch_one(&app.pool)
            .await
            .unwrap();
            assert_eq!(n.0, 1, "座標の記録が 1 本でない");
        }
    }

    // ---- 3.4 既定の感度（D11（仮））

    // 定数を名指しで固定する
    #[tokio::test]
    async fn place_sensitivity_constant_is_pinned() {
        assert_eq!(places::DEFAULT_SENSITIVITY, 1);
        assert_eq!(crate::attributes::DEFAULT_SENSITIVITY, 2);
    }

    // Scenario: 場所の記録は外部 AI に出してよいで格納される
    #[tokio::test]
    async fn place_sensitivity_of_place_records_is_external_ai_ok() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        assert_accepted(
            &send_item(&app, item(a, user, &name_raw(a, place, "自宅"))).await,
            "名前",
        );
        assert_accepted(
            &send_item(&app, item(b, user, &first_raw(b, place))).await,
            "座標",
        );
        for id in [a, b] {
            assert_eq!(stored(&app, id).await.2, places::DEFAULT_SENSITIVITY);
            assert_eq!(stored(&app, id).await.2, 1);
        }
    }

    // Scenario: 場所を足しても主張の既定の感度は変わらない
    #[tokio::test]
    async fn place_sensitivity_leaves_the_claim_default_unchanged() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        let p = Uuid::new_v4();
        assert_accepted(
            &send_item(&app, item(p, user, &name_raw(p, place, "自宅"))).await,
            "名前",
        );
        // 個人属性の主張（種類は取り込み口が見るので、本物の種類を足す）
        let kind = crate::attributes_store::add_kind(&app.pool, user, "趣味")
            .await
            .unwrap()
            .unwrap()
            .id;
        let claim = Uuid::new_v4();
        let raw = serde_json::json!({
            "claim": claim, "nonce": NONCE, "kind": kind, "value": "囲碁",
            "valid_from": { "precision": "unknown", "date": null },
            "supersedes": null, "note": null,
        })
        .to_string();
        let mut it = item(claim, user, &raw);
        it["logical_source"] = serde_json::json!(crate::attributes::SOURCE);
        assert_accepted(&send_item(&app, it).await, "主張");
        assert_eq!(stored(&app, claim).await.2, 2, "主張は「ローカル AI まで」");
    }

    // ---- 3.5 広さの範囲

    #[tokio::test]
    async fn place_ingest_radius_bounds() {
        assert_eq!(places::PLACE_RADIUS_MIN_M, 10);
        assert_eq!(places::PLACE_RADIUS_MAX_M, 5000);
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        for ok in [10, 5000] {
            let raw = radius_raw(Uuid::new_v4(), place, serde_json::json!(ok));
            assert_accepted(&send(&app, user, &raw).await, &format!("{ok} m"));
        }
        for bad in [9, 5001] {
            let raw = radius_raw(Uuid::new_v4(), place, serde_json::json!(bad));
            rejects(&app, user, &raw, "invalid_radius").await;
        }
    }

    // 補足は null（補足なし）も文字列も通る
    #[tokio::test]
    async fn place_ingest_stores_a_note_and_a_cleared_note() {
        let app = app().await;
        let user = testdb::user();
        let place = container(&app, user).await;
        for note in [serde_json::json!("裏口から入る"), serde_json::json!(null)] {
            let raw = raw_of(
                Uuid::new_v4(),
                place,
                NONCE,
                serde_json::json!({ "field": "note", "note": note }),
            );
            assert_accepted(&send(&app, user, &raw).await, "補足");
        }
    }
}

// ---------------------------------------------------------------- いまの値と前の値・座標の版（Task 4 / design D6 / D7 / D15）

/// `view` と `coord_windows` の単体（DB を持たない。D6 / D7 の表を固定する）。
mod view_unit {
    use crate::attributes::{Precision, ValidFrom};
    use crate::places::{
        coord_windows, view, CoordChange, PlaceField, PlaceRecord, StoredPlaceRecord,
        PLACE_DEFAULT_RADIUS_M,
    };
    use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
    use uuid::Uuid;

    fn written(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    fn today(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn utc(s: &str) -> DateTime<Utc> {
        written(s).with_timezone(&Utc)
    }

    /// 1 件の記録。`ingested` は D-01 に入った時刻（2026-10-01 の 00:00:`ingested` 秒）
    fn rec(place: Uuid, field: PlaceField, at: &str, ingested: u32) -> StoredPlaceRecord {
        let id = Uuid::new_v4();
        StoredPlaceRecord {
            record: PlaceRecord {
                id,
                place,
                field,
                payload: serde_json::json!({}),
            },
            written_at: written(at),
            ingested_at: utc("2026-10-01T00:00:00Z") + chrono::Duration::seconds(ingested.into()),
        }
    }

    fn name(place: Uuid, n: &str, at: &str, ingested: u32) -> StoredPlaceRecord {
        rec(place, PlaceField::Name(n.into()), at, ingested)
    }

    fn first(place: Uuid, lat: f64, at: &str) -> StoredPlaceRecord {
        coord(place, lat, CoordChange::First, None, None, at)
    }

    fn coord(
        place: Uuid,
        lat: f64,
        change: CoordChange,
        valid_from: Option<ValidFrom>,
        supersedes: Option<Uuid>,
        at: &str,
    ) -> StoredPlaceRecord {
        rec(
            place,
            PlaceField::Coord {
                lat,
                lon: 139.0,
                change,
                valid_from,
                supersedes,
            },
            at,
            0,
        )
    }

    fn moved(
        place: Uuid,
        lat: f64,
        precision: Precision,
        date: Option<&str>,
        at: &str,
    ) -> StoredPlaceRecord {
        let vf = ValidFrom {
            precision,
            date: date.map(str::to_string),
        };
        coord(place, lat, CoordChange::Move, Some(vf), None, at)
    }

    fn fix(place: Uuid, lat: f64, target: &StoredPlaceRecord, at: &str) -> StoredPlaceRecord {
        coord(
            place,
            lat,
            CoordChange::Fix,
            None,
            Some(target.record.id),
            at,
        )
    }

    // ---------------------------------------------------------------- view（D6）

    #[test]
    fn place_view_unit_default_radius_is_pinned_and_separate_from_the_stay_criteria() {
        assert_eq!(PLACE_DEFAULT_RADIUS_M, 100);
        // 滞在の判定の半径は別の定数（どちらも 100 を名指しで固定する）
        assert_eq!(crate::stay::Criteria::default_values().radius_m, 100);
    }

    #[test]
    fn place_view_unit_latest_written_wins_and_ties_break_by_ingest_time() {
        let p = Uuid::new_v4();
        let recs = vec![
            name(p, "旧", "2026-09-01T09:00:00+09:00", 1),
            // 書いた日時が同じなら D-01 に入った時刻が後のものが勝つ
            name(p, "新A", "2026-09-02T09:00:00+09:00", 2),
            name(p, "新B", "2026-09-02T09:00:00+09:00", 3),
            first(p, 35.0, "2026-09-01T09:00:00+09:00"),
        ];
        let v = view(&[p], &recs, today("2026-10-01"));
        assert_eq!(v.places.len(), 1);
        assert_eq!(v.places[0].name, "新B");
        let prev: Vec<&str> = v.places[0]
            .previous_names
            .iter()
            .map(|n| n.name.as_str())
            .collect();
        // 前の名前は書いた日時の新しい順
        assert_eq!(prev, vec!["新A", "旧"]);
    }

    #[test]
    fn place_view_unit_later_ingest_does_not_beat_a_later_written_time() {
        let p = Uuid::new_v4();
        let recs = vec![
            name(p, "後に書いた", "2026-09-05T09:00:00+09:00", 1),
            // 先に書いたものを後から送っても、いまの名前にならない
            name(p, "先に書いた", "2026-09-01T09:00:00+09:00", 9),
            first(p, 35.0, "2026-09-01T09:00:00+09:00"),
        ];
        let v = view(&[p], &recs, today("2026-10-01"));
        assert_eq!(v.places[0].name, "後に書いた");
    }

    #[test]
    fn place_view_unit_a_place_needs_a_name_and_a_coordinate() {
        let (only_name, only_coord, both) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let recs = vec![
            name(only_name, "名前だけ", "2026-09-01T09:00:00+09:00", 1),
            first(only_coord, 35.0, "2026-09-01T09:00:00+09:00"),
            name(both, "両方", "2026-09-01T09:00:00+09:00", 2),
            first(both, 35.0, "2026-09-01T09:00:00+09:00"),
        ];
        let v = view(&[only_name, only_coord, both], &recs, today("2026-10-01"));
        let ids: Vec<Uuid> = v.places.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![both]);
    }

    #[test]
    fn place_view_unit_places_keep_the_container_order() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let mut recs = vec![];
        for (p, n) in [(a, "A"), (b, "B")] {
            recs.push(name(p, n, "2026-09-01T09:00:00+09:00", 1));
            recs.push(first(p, 35.0, "2026-09-01T09:00:00+09:00"));
        }
        let v = view(&[b, a], &recs, today("2026-10-01"));
        let names: Vec<&str> = v.places.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["B", "A"]);
    }

    #[test]
    fn place_view_unit_radius_defaults_and_the_latest_wins() {
        let p = Uuid::new_v4();
        let mut recs = vec![
            name(p, "n", "2026-09-01T09:00:00+09:00", 1),
            first(p, 35.0, "2026-09-01T09:00:00+09:00"),
        ];
        assert_eq!(
            view(&[p], &recs, today("2026-10-01")).places[0].radius_m,
            100
        );
        recs.push(rec(
            p,
            PlaceField::Radius(50),
            "2026-09-02T09:00:00+09:00",
            2,
        ));
        recs.push(rec(
            p,
            PlaceField::Radius(300),
            "2026-09-03T09:00:00+09:00",
            3,
        ));
        assert_eq!(
            view(&[p], &recs, today("2026-10-01")).places[0].radius_m,
            300
        );
    }

    #[test]
    fn place_view_unit_a_null_note_clears_the_note() {
        let p = Uuid::new_v4();
        let mut recs = vec![
            name(p, "n", "2026-09-01T09:00:00+09:00", 1),
            first(p, 35.0, "2026-09-01T09:00:00+09:00"),
            rec(
                p,
                PlaceField::Note(Some("裏口".into())),
                "2026-09-02T09:00:00+09:00",
                2,
            ),
        ];
        assert_eq!(
            view(&[p], &recs, today("2026-10-01")).places[0]
                .note
                .as_deref(),
            Some("裏口")
        );
        recs.push(rec(
            p,
            PlaceField::Note(None),
            "2026-09-03T09:00:00+09:00",
            3,
        ));
        assert_eq!(view(&[p], &recs, today("2026-10-01")).places[0].note, None);
    }

    #[test]
    fn place_view_unit_other_places_records_do_not_leak() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let recs = vec![
            name(a, "A", "2026-09-01T09:00:00+09:00", 1),
            first(a, 35.0, "2026-09-01T09:00:00+09:00"),
            name(b, "B", "2026-09-02T09:00:00+09:00", 2),
            first(b, 36.0, "2026-09-02T09:00:00+09:00"),
            rec(b, PlaceField::Radius(300), "2026-09-02T09:00:00+09:00", 3),
        ];
        let v = view(&[a, b], &recs, today("2026-10-01"));
        assert_eq!(v.places[0].name, "A");
        assert_eq!(v.places[0].radius_m, 100);
        assert_eq!(v.places[0].coord.lat, 35.0);
        assert_eq!(v.places[1].radius_m, 300);
    }

    #[test]
    fn place_view_unit_previous_coordinates_carry_their_state() {
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let x = fix(p, 35.1, &f, "2026-09-02T09:00:00+09:00");
        let m = moved(
            p,
            35.2,
            Precision::Month,
            Some("2026-04"),
            "2026-09-03T09:00:00+09:00",
        );
        let future = moved(
            p,
            35.3,
            Precision::Year,
            Some("2027"),
            "2026-09-04T09:00:00+09:00",
        );
        let recs = vec![
            name(p, "n", "2026-09-01T09:00:00+09:00", 1),
            f.clone(),
            x.clone(),
            m.clone(),
            future.clone(),
        ];
        let v = view(&[p], &recs, today("2026-10-01"));
        let out = &v.places[0];
        assert_eq!(out.coord.record_id, m.record.id, "いまは 2026-04 の移転");
        let state = |id: Uuid| {
            let c = out
                .previous_coords
                .iter()
                .find(|c| c.record_id == id)
                .unwrap();
            (c.state.as_str(), c.fixed_by)
        };
        assert_eq!(state(f.record.id), ("fixed", Some(x.record.id)));
        assert_eq!(state(x.record.id), ("before_move", None));
        assert_eq!(state(future.record.id), ("upcoming", None));
        assert_eq!(
            out.previous_coords.len(),
            3,
            "いまの座標は前の座標に入らない"
        );
    }

    #[test]
    fn place_view_unit_with_no_version_containing_today_the_last_written_is_current() {
        // 最初の座標を消して、未来の移転だけが残った
        let p = Uuid::new_v4();
        let m = moved(
            p,
            35.2,
            Precision::Year,
            Some("2027"),
            "2026-09-03T09:00:00+09:00",
        );
        let recs = vec![name(p, "n", "2026-09-01T09:00:00+09:00", 1), m.clone()];
        let v = view(&[p], &recs, today("2026-10-01"));
        assert_eq!(v.places[0].coord.record_id, m.record.id);
        assert!(v.places[0].previous_coords.is_empty());
    }

    // ---------------------------------------------------------------- coord_windows（D7）

    fn jst(date: &str) -> Option<DateTime<Utc>> {
        Some(utc(&format!("{date}T00:00:00+09:00")))
    }

    #[test]
    fn place_window_unit_a_first_coordinate_covers_all_time() {
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let w = coord_windows(std::slice::from_ref(&f));
        assert_eq!(w.len(), 1);
        assert_eq!((w[0].id, w[0].start, w[0].end), (f.record.id, None, None));
    }

    #[test]
    fn place_window_unit_a_dated_move_cuts_the_previous_version_at_its_start() {
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let cases = [
            (Precision::Year, "2026", "2026-01-01"),
            (Precision::Month, "2026-04", "2026-04-01"),
            (Precision::Day, "2026-04-15", "2026-04-15"),
        ];
        for (precision, date, from) in cases {
            let m = moved(p, 35.1, precision, Some(date), "2026-09-02T09:00:00+09:00");
            let w = coord_windows(&[f.clone(), m.clone()]);
            assert_eq!(w.len(), 2, "{date}");
            // 日の境は Asia/Tokyo の 0 時
            assert_eq!((w[0].start, w[0].end), (None, jst(from)), "{date}");
            assert_eq!((w[1].start, w[1].end), (jst(from), None), "{date}");
        }
    }

    #[test]
    fn place_window_unit_an_unknown_move_overlaps_until_it_was_written() {
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let m = moved(
            p,
            35.1,
            Precision::Unknown,
            None,
            "2026-09-05T09:00:00+09:00",
        );
        let w = coord_windows(&[f, m]);
        // 移ったと書いた日まで、前の版と新しい版の両方が当たる
        assert_eq!(
            (w[0].start, w[0].end),
            (None, Some(utc("2026-09-05T09:00:00+09:00")))
        );
        assert_eq!((w[1].start, w[1].end), (None, None));
    }

    #[test]
    fn place_window_unit_the_later_written_version_wins_an_overlap() {
        // 2026-04 の後に、より古い 2025-06 の移転を書いた（spec-review R15）
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let a = moved(
            p,
            35.1,
            Precision::Month,
            Some("2026-04"),
            "2026-09-02T09:00:00+09:00",
        );
        let b = moved(
            p,
            35.2,
            Precision::Month,
            Some("2025-06"),
            "2026-09-03T09:00:00+09:00",
        );
        let w = coord_windows(&[f, a, b]);
        assert_eq!(w[0].end, jst("2025-06-01"));
        assert_eq!(w[1].end, jst("2025-06-01"), "間の版は空の期間を持つ");
        assert!(w[1].start > w[1].end);
        assert_eq!((w[2].start, w[2].end), (jst("2025-06-01"), None));
    }

    #[test]
    fn place_window_unit_a_fix_inherits_the_position_and_start_of_the_fixed_record() {
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let m = moved(
            p,
            35.1,
            Precision::Month,
            Some("2026-04"),
            "2026-09-02T09:00:00+09:00",
        );
        // 最初の座標を、移った後に直す
        let x = fix(p, 35.05, &f, "2026-09-03T09:00:00+09:00");
        let w = coord_windows(&[f.clone(), m.clone(), x.clone()]);
        assert_eq!(w.len(), 2, "直された座標はどの期間にも当たらない");
        assert_eq!(
            (w[0].id, w[0].start, w[0].end),
            (x.record.id, None, jst("2026-04-01"))
        );
        assert_eq!(
            (w[1].id, w[1].start, w[1].end),
            (m.record.id, jst("2026-04-01"), None)
        );
    }

    #[test]
    fn place_window_unit_a_fix_of_a_move_keeps_the_moves_start() {
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let m = moved(
            p,
            35.1,
            Precision::Day,
            Some("2026-04-10"),
            "2026-09-02T09:00:00+09:00",
        );
        let x = fix(p, 35.15, &m, "2026-09-03T09:00:00+09:00");
        let w = coord_windows(&[f, m, x.clone()]);
        assert_eq!(w.len(), 2);
        assert_eq!((w[1].id, w[1].start), (x.record.id, jst("2026-04-10")));
    }

    #[test]
    fn place_window_unit_a_fix_of_a_fix_resolves_to_the_root() {
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let x1 = fix(p, 35.1, &f, "2026-09-02T09:00:00+09:00");
        let x2 = fix(p, 35.2, &x1, "2026-09-03T09:00:00+09:00");
        let w = coord_windows(&[f, x1, x2.clone()]);
        assert_eq!(w.len(), 1);
        assert_eq!((w[0].id, w[0].start, w[0].end), (x2.record.id, None, None));
    }

    #[test]
    fn place_window_unit_a_fix_whose_target_is_unusable_stands_on_its_own_position() {
        let p = Uuid::new_v4();
        let gone = first(p, 35.0, "2026-09-01T09:00:00+09:00"); // 渡さない（消えた記録）
        let m = moved(
            p,
            35.1,
            Precision::Month,
            Some("2026-04"),
            "2026-09-02T09:00:00+09:00",
        );
        let x = fix(p, 35.2, &gone, "2026-09-03T09:00:00+09:00");
        let w = coord_windows(&[m.clone(), x.clone()]);
        // 並びは自分の書いた順（移った → 直す）。直す先が無いので最も古い時刻から、書いた日時で区切る
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].id, m.record.id);
        assert_eq!(
            (w[0].start, w[0].end),
            (jst("2026-04-01"), Some(utc("2026-09-03T09:00:00+09:00")))
        );
        assert_eq!((w[1].id, w[1].start, w[1].end), (x.record.id, None, None));
    }

    #[test]
    fn place_window_unit_records_that_are_not_coordinates_are_ignored() {
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let w = coord_windows(&[name(p, "n", "2026-09-01T09:00:00+09:00", 1), f.clone()]);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].id, f.record.id);
        assert!(coord_windows(&[]).is_empty());
    }

    #[test]
    fn place_window_unit_the_order_of_the_input_does_not_matter() {
        let p = Uuid::new_v4();
        let f = first(p, 35.0, "2026-09-01T09:00:00+09:00");
        let m = moved(
            p,
            35.1,
            Precision::Month,
            Some("2026-04"),
            "2026-09-02T09:00:00+09:00",
        );
        let a = coord_windows(&[f.clone(), m.clone()]);
        let b = coord_windows(&[m, f]);
        assert_eq!(a, b);
    }
}

/// `GET /places` の口（Task 4 / design D15）。
mod view_endpoint {
    use super::ingest_endpoint::{
        app, assert_accepted, auth, container, coord_raw, first_raw, item, name_raw, radius_raw,
        raw_of, send_item, NONCE,
    };
    use crate::{places_get, testdb, App, PlacesQuery};
    use axum::{extract::Query, extract::State, http::HeaderMap, http::StatusCode};
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    /// 2026-10-01 の昼（JST）に読み出す
    const NOW: &str = "2026-10-01T03:00:00Z";

    fn at_now(app: App, now: &str) -> App {
        app.at(DateTime::parse_from_rfc3339(now)
            .unwrap()
            .with_timezone(&Utc))
    }

    /// 書いた日時を決めて送り、記録の識別子を返す
    async fn put(app: &App, user: Uuid, raw: String, written: &str) -> Uuid {
        let id = serde_json::from_str::<serde_json::Value>(&raw).unwrap()["record"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap();
        let mut it = item(id, user, &raw);
        it["event_time"] = serde_json::json!(written);
        assert_accepted(&send_item(app, it).await, "場所の記録");
        id
    }

    async fn put_name(app: &App, user: Uuid, place: Uuid, name: &str, written: &str) -> Uuid {
        put(app, user, name_raw(Uuid::new_v4(), place, name), written).await
    }

    /// 名前と座標を持つ場所を作る。返すのは (器, 名前の記録, 座標の記録)
    async fn registered(app: &App, user: Uuid, name: &str) -> (Uuid, Uuid, Uuid) {
        let place = container(app, user).await;
        let n = put_name(app, user, place, name, "2026-09-01T09:00:00+09:00").await;
        let c = put_coord(
            app,
            user,
            place,
            35.681236,
            "first",
            serde_json::Value::Null,
            None,
            "2026-09-01T09:00:00+09:00",
        )
        .await;
        (place, n, c)
    }

    #[allow(clippy::too_many_arguments)]
    async fn put_coord(
        app: &App,
        user: Uuid,
        place: Uuid,
        lat: f64,
        change: &str,
        valid_from: serde_json::Value,
        supersedes: Option<Uuid>,
        written: &str,
    ) -> Uuid {
        let raw = coord_raw(Uuid::new_v4(), place, lat, change, valid_from, supersedes);
        put(app, user, raw, written).await
    }

    async fn get(app: &App, user: Uuid) -> serde_json::Value {
        let out = places_get(
            State(app.clone()),
            auth(),
            Query(PlacesQuery {
                user_id: Some(user),
            }),
        )
        .await
        .expect("読み出せる");
        serde_json::to_value(out.0).unwrap()
    }

    fn place_of(v: &serde_json::Value, id: Uuid) -> Option<&serde_json::Value> {
        v["places"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == serde_json::json!(id))
    }

    async fn mark_deleted(app: &App, id: Uuid) {
        sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
            .bind(id)
            .execute(&app.pool)
            .await
            .unwrap();
    }

    async fn erase(app: &App, user: Uuid, id: Uuid) {
        let mut tx = app.pool.begin().await.unwrap();
        sqlx::query(
            "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
             VALUES ($1, $2, 's01-place', 'event', 'test')",
        )
        .bind(id)
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query("UPDATE core.event SET raw = '', payload = '{}' WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }

    fn month(date: &str) -> serde_json::Value {
        serde_json::json!({ "precision": "month", "date": date })
    }

    fn names_of(p: &serde_json::Value, key: &str) -> Vec<String> {
        p[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["name"].as_str().unwrap().to_string())
            .collect()
    }

    #[tokio::test]
    async fn place_view_endpoint_requires_the_token() {
        let app = app().await;
        let (code, _) = places_get(
            State(app),
            HeaderMap::new(),
            Query(PlacesQuery {
                user_id: Some(testdb::user()),
            }),
        )
        .await
        .expect_err("資格情報なしで読み出せた");
        assert_eq!(code, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn place_view_endpoint_returns_today_and_the_zero_stays() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        let v = get(&app, user).await;
        assert_eq!(v["today"], "2026-10-01");
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["stays"]["count"], 0, "滞在の項は Task 5 で埋める");
        assert_eq!(p["stays"]["hours"].as_array().unwrap().len(), 24);
    }

    // Scenario: 名前を 2 回変えると 3 つの名前の記録が残る
    #[tokio::test]
    async fn place_view_endpoint_three_name_records_after_two_renames() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        put_name(&app, user, place, "本社", "2026-09-02T09:00:00+09:00").await;
        let last = put_name(&app, user, place, "職場", "2026-09-03T09:00:00+09:00").await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["name"], "職場");
        assert_eq!(p["name_record"]["record_id"], serde_json::json!(last));
        // いまの名前の記録 1 件 + 前の名前の記録 2 件 = 3 件
        assert_eq!(p["previous_names"].as_array().unwrap().len(), 2);
    }

    // Scenario: 座標の記録は変え方を持つ
    #[tokio::test]
    async fn place_view_endpoint_coordinate_records_carry_the_change() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        put_coord(
            &app,
            user,
            place,
            35.7,
            "fix",
            serde_json::Value::Null,
            Some(first),
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        put_coord(
            &app,
            user,
            place,
            35.8,
            "move",
            month("2026-04"),
            None,
            "2026-09-03T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        let mut changes = vec![p["coord"]["change"].as_str().unwrap().to_string()];
        for c in p["previous_coords"].as_array().unwrap() {
            changes.push(c["change"].as_str().unwrap().to_string());
        }
        changes.sort();
        assert_eq!(changes, vec!["first", "fix", "move"]);
    }

    // Scenario: 直す記録は直した座標の記録を指す
    #[tokio::test]
    async fn place_view_endpoint_a_fix_points_at_the_fixed_record() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        put_coord(
            &app,
            user,
            place,
            35.7,
            "fix",
            serde_json::Value::Null,
            Some(first),
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["coord"]["change"], "fix");
        assert_eq!(p["coord"]["supersedes"], serde_json::json!(first));
    }

    // Scenario: 移ったのいつからは精度のまま残る
    #[tokio::test]
    async fn place_view_endpoint_valid_from_keeps_its_precision() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        put_coord(
            &app,
            user,
            place,
            35.8,
            "move",
            month("2026-04"),
            None,
            "2026-09-03T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["coord"]["valid_from"], month("2026-04"));
        assert!(
            p["coord"]["valid_from"]["date"].as_str().unwrap().len() == 7,
            "日を持たない"
        );
    }

    // Scenario: 座標は丸めずに残る
    #[tokio::test]
    async fn place_view_endpoint_coordinates_are_not_rounded() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let place = container(&app, user).await;
        put_name(&app, user, place, "駅", "2026-09-01T09:00:00+09:00").await;
        put(
            &app,
            user,
            first_raw(Uuid::new_v4(), place),
            "2026-09-01T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["coord"]["lat"].as_f64(), Some(35.681236));
        assert_eq!(p["coord"]["lon"].as_f64(), Some(139.767125));
    }

    // Scenario: 場所の記録の書いた日時と D-01 に入った時刻が別々に入る
    #[tokio::test]
    async fn place_view_endpoint_written_and_ingested_times_are_separate() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let place = container(&app, user).await;
        put_name(&app, user, place, "職場", "2026-09-01T09:00:00+09:00").await;
        put_coord(
            &app,
            user,
            place,
            35.0,
            "first",
            serde_json::Value::Null,
            None,
            "2026-09-01T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        let rec = &place_of(&v, place).unwrap()["name_record"];
        assert_eq!(rec["written_at"], "2026-09-01T09:00:00+09:00");
        let ingested = DateTime::parse_from_rfc3339(rec["ingested_at"].as_str().unwrap()).unwrap();
        let gap = (Utc::now() - ingested.with_timezone(&Utc))
            .num_seconds()
            .abs();
        assert!(
            gap < 300,
            "D-01 に入った時刻は送った時刻（いま）: {gap} 秒のずれ"
        );
    }

    // Scenario: 同じ名前に変え直しても 1 件増える
    #[tokio::test]
    async fn place_view_endpoint_renaming_to_the_same_name_adds_a_record() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        put_name(&app, user, place, "職場", "2026-09-02T09:00:00+09:00").await;
        let v = get(&app, user).await;
        assert_eq!(
            place_of(&v, place).unwrap()["previous_names"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    // Scenario: 場所の名前は合成済みで読み出される
    #[tokio::test]
    async fn place_view_endpoint_names_are_read_composed() {
        use unicode_normalization::UnicodeNormalization as _;
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let decomposed: String = "ガ".nfd().collect();
        assert_ne!(decomposed, "ガ", "分解された形で送る");
        let (place, _, _) = registered(&app, user, &decomposed).await;
        let v = get(&app, user).await;
        assert_eq!(place_of(&v, place).unwrap()["name"], "ガ");
    }

    // Scenario: 同じ名前の場所を 2 つ持てる
    #[tokio::test]
    async fn place_view_endpoint_two_places_can_share_a_name() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (a, _, _) = registered(&app, user, "支店").await;
        let (b, _, _) = registered(&app, user, "支店").await;
        let v = get(&app, user).await;
        let shop: Vec<&serde_json::Value> = v["places"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["name"] == "支店")
            .collect();
        assert_eq!(shop.len(), 2);
        assert_ne!(shop[0]["id"], shop[1]["id"]);
        assert!(place_of(&v, a).is_some() && place_of(&v, b).is_some());
    }

    // Scenario: 名前と座標と広さを変えても識別子が変わらない
    #[tokio::test]
    async fn place_view_endpoint_the_id_survives_every_change() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        put_name(&app, user, place, "本社", "2026-09-02T09:00:00+09:00").await;
        put_coord(
            &app,
            user,
            place,
            35.7,
            "fix",
            serde_json::Value::Null,
            Some(first),
            "2026-09-03T09:00:00+09:00",
        )
        .await;
        put(
            &app,
            user,
            radius_raw(Uuid::new_v4(), place, serde_json::json!(200)),
            "2026-09-04T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        assert_eq!(v["places"].as_array().unwrap().len(), 1);
        assert_eq!(v["places"][0]["id"], serde_json::json!(place));
    }

    // Scenario: 名前を変えるといまの名前が変わり前の名前が残る
    #[tokio::test]
    async fn place_view_endpoint_a_rename_keeps_the_previous_name() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        put_name(&app, user, place, "本社", "2026-09-02T09:00:00+09:00").await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["name"], "本社");
        assert_eq!(names_of(p, "previous_names"), vec!["職場"]);
    }

    // Scenario: 広さの記録の無い場所は 100 m
    #[tokio::test]
    async fn place_view_endpoint_a_place_without_a_radius_record_is_100_m() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        let v = get(&app, user).await;
        assert_eq!(place_of(&v, place).unwrap()["radius_m"], 100);
    }

    // Scenario: 広さを変えるといまの広さが変わる
    #[tokio::test]
    async fn place_view_endpoint_a_new_radius_record_changes_the_radius() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        put(
            &app,
            user,
            radius_raw(Uuid::new_v4(), place, serde_json::json!(100)),
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        put(
            &app,
            user,
            radius_raw(Uuid::new_v4(), place, serde_json::json!(300)),
            "2026-09-03T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        assert_eq!(place_of(&v, place).unwrap()["radius_m"], 300);
    }

    // Scenario: 補足なしを書くと補足が消える
    #[tokio::test]
    async fn place_view_endpoint_a_null_note_clears_the_note() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        let note = |v: serde_json::Value| {
            raw_of(
                Uuid::new_v4(),
                place,
                NONCE,
                serde_json::json!({ "field": "note", "note": v }),
            )
        };
        put(
            &app,
            user,
            note(serde_json::json!("裏口から")),
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        assert_eq!(place_of(&v, place).unwrap()["note"], "裏口から");
        put(
            &app,
            user,
            note(serde_json::Value::Null),
            "2026-09-03T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        assert!(place_of(&v, place).unwrap()["note"].is_null());
    }

    // Scenario: 消したことにした名前の記録の前の名前がいまの名前に戻る
    #[tokio::test]
    async fn place_view_endpoint_a_deleted_name_record_is_ignored() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        let renamed = put_name(&app, user, place, "本社", "2026-09-02T09:00:00+09:00").await;
        mark_deleted(&app, renamed).await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["name"], "職場");
        assert!(!names_of(p, "previous_names").contains(&"本社".to_string()));
    }

    // Scenario: 本文を消去した名前の記録は使わない
    #[tokio::test]
    async fn place_view_endpoint_an_erased_name_record_is_not_used() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        let renamed = put_name(&app, user, place, "本社", "2026-09-02T09:00:00+09:00").await;
        erase(&app, user, renamed).await;
        let v = get(&app, user).await;
        assert_eq!(place_of(&v, place).unwrap()["name"], "職場");
    }

    // Scenario: 名前の記録が全部消えた場所は返らない
    #[tokio::test]
    async fn place_view_endpoint_a_place_without_any_usable_name_is_not_returned() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, name, _) = registered(&app, user, "職場").await;
        mark_deleted(&app, name).await;
        let v = get(&app, user).await;
        assert!(place_of(&v, place).is_none());
    }

    // Scenario: 座標の記録の無い器は返らない
    #[tokio::test]
    async fn place_view_endpoint_a_container_without_a_coordinate_is_not_returned() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let place = container(&app, user).await;
        put_name(&app, user, place, "名前だけ", "2026-09-01T09:00:00+09:00").await;
        let v = get(&app, user).await;
        assert!(place_of(&v, place).is_none());
    }

    // Scenario: 直した前の座標は直したものとして返る
    #[tokio::test]
    async fn place_view_endpoint_a_fixed_coordinate_is_returned_as_fixed() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        let fix = put_coord(
            &app,
            user,
            place,
            35.7,
            "fix",
            serde_json::Value::Null,
            Some(first),
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["coord"]["lat"].as_f64(), Some(35.7));
        assert_eq!(p["coord"]["record_id"], serde_json::json!(fix));
        let prev = &p["previous_coords"][0];
        assert_eq!(p["previous_coords"].as_array().unwrap().len(), 1);
        assert_eq!(prev["record_id"], serde_json::json!(first));
        assert_eq!(prev["state"], "fixed");
        assert_eq!(prev["fixed_by"], serde_json::json!(fix));
        assert_eq!(prev["lat"].as_f64(), Some(35.681236));
    }

    // Scenario: 移る前の座標は移る前のものとして返る
    #[tokio::test]
    async fn place_view_endpoint_the_coordinate_before_a_move_is_returned_as_before_the_move() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        let moved = put_coord(
            &app,
            user,
            place,
            35.8,
            "move",
            month("2026-04"),
            None,
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["coord"]["record_id"], serde_json::json!(moved));
        // 移った日はいまの座標の「いつから」が持つ
        assert_eq!(p["coord"]["valid_from"], month("2026-04"));
        assert_eq!(p["previous_coords"].as_array().unwrap().len(), 1);
        assert_eq!(
            p["previous_coords"][0]["record_id"],
            serde_json::json!(first)
        );
        assert_eq!(p["previous_coords"][0]["state"], "before_move");
    }

    // Scenario: 未来のいつからの移転はいまの座標を変えない
    #[tokio::test]
    async fn place_view_endpoint_a_future_move_does_not_change_the_current_coordinate() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        let future = put_coord(
            &app,
            user,
            place,
            35.8,
            "move",
            month("2026-12"),
            None,
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["coord"]["record_id"], serde_json::json!(first));
        let prev = &p["previous_coords"][0];
        assert_eq!(prev["record_id"], serde_json::json!(future));
        assert_eq!(prev["state"], "upcoming");
        assert_eq!(prev["valid_from"], month("2026-12"));
    }

    // Scenario: いまの座標の記録の識別子が返る
    #[tokio::test]
    async fn place_view_endpoint_the_current_coordinate_record_id_is_returned() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        let v = get(&app, user).await;
        assert_eq!(
            place_of(&v, place).unwrap()["coord"]["record_id"],
            serde_json::json!(first)
        );
    }

    // Scenario: 感度で場所の記録を絞らない
    #[tokio::test]
    async fn place_view_endpoint_does_not_filter_by_sensitivity() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, name, _) = registered(&app, user, "職場").await;
        sqlx::query("UPDATE core.event SET sensitivity = 3 WHERE id = $1")
            .bind(name)
            .execute(&app.pool)
            .await
            .unwrap();
        let v = get(&app, user).await;
        assert_eq!(place_of(&v, place).unwrap()["name"], "職場");
    }

    // Scenario: 別の利用者の場所は読み出せない
    #[tokio::test]
    async fn place_view_endpoint_does_not_read_another_users_places() {
        let app = at_now(app().await, NOW);
        let (a, b) = (testdb::user(), testdb::user());
        let (pa, _, _) = registered(&app, a, "A の場所").await;
        let (pb, _, _) = registered(&app, b, "B の場所").await;
        let v = get(&app, a).await;
        assert!(place_of(&v, pa).is_some());
        assert!(place_of(&v, pb).is_none());
    }

    // 4.3: 日は Asia/Tokyo で切る（UTC では 2026-03-31 15:00 は 3/31）
    #[tokio::test]
    async fn place_view_today_is_tokyo() {
        let base = app().await;
        let user = testdb::user();
        let (place, _, first) = registered(&base, user, "職場").await;
        let moved = put_coord(
            &base,
            user,
            place,
            35.8,
            "move",
            serde_json::json!({"precision":"day","date":"2026-04-01"}),
            None,
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        // 東京ではまだ 3/31（UTC の 3/31 14:59:59）→ 移転は予定
        let before = get(&at_now(base.clone(), "2026-03-31T14:59:59Z"), user).await;
        assert_eq!(before["today"], "2026-03-31");
        let p = place_of(&before, place).unwrap();
        assert_eq!(p["coord"]["record_id"], serde_json::json!(first));
        assert_eq!(p["previous_coords"][0]["state"], "upcoming");
        // 東京では 4/1 0 時（UTC の 3/31 15:00）→ 移転がいまの座標
        let after = get(&at_now(base, "2026-03-31T15:00:00Z"), user).await;
        assert_eq!(after["today"], "2026-04-01");
        let p = place_of(&after, place).unwrap();
        assert_eq!(p["coord"]["record_id"], serde_json::json!(moved));
        assert_eq!(p["previous_coords"][0]["state"], "before_move");
    }

    // ---------------------------------------------------------------- 照合と合計（Task 5 / D7 / D8 / D9）

    /// 場所の座標の経度（`coord_raw` と同じ）
    const LON: f64 = 139.767125;
    /// 登録の座標の緯度（`registered` と同じ）
    const LAT: f64 = 35.681236;
    /// 緯度 1 度が約 111.32 km（`stay::distance_m` と同じ近似）。北へ `m` メートル離れた緯度
    fn north(lat: f64, m: f64) -> f64 {
        lat + m / 111_320.0
    }

    /// 派生の滞在を直に置く（`stay_tests` と同じ形。代表点と時刻を試験が決める）。返すのは行の識別子
    async fn stay(app: &App, user: Uuid, start: &str, end: &str, lat: f64) -> Uuid {
        let id = Uuid::new_v4();
        let raw = serde_json::json!({
            "start": DateTime::parse_from_rfc3339(start).unwrap().with_timezone(&Utc).to_rfc3339(),
            "end": DateTime::parse_from_rfc3339(end).unwrap().with_timezone(&Utc).to_rfc3339(),
            "lat": lat, "lon": LON,
        })
        .to_string();
        sqlx::query(
            "INSERT INTO core.event
               (id, user_id, logical_source, external_id, origin, event_time,
                tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
             VALUES ($1,$2,'s01-stay',$3,'derived',$4,540,'Asia/Tokyo',1,$3,$5,$5::jsonb)",
        )
        .bind(id)
        .bind(user)
        .bind(id.to_string())
        .bind(
            DateTime::parse_from_rfc3339(start)
                .unwrap()
                .with_timezone(&Utc),
        )
        .bind(&raw)
        .execute(&app.pool)
        .await
        .unwrap();
        id
    }

    /// 滞在を時刻（JST）と長さで置く
    async fn stay_min(app: &App, user: Uuid, start: &str, minutes: i64, lat: f64) -> Uuid {
        let s = DateTime::parse_from_rfc3339(start).unwrap();
        let e = s + chrono::Duration::minutes(minutes);
        stay(app, user, start, &e.to_rfc3339(), lat).await
    }

    /// (件数, 合計分)
    fn count_minutes(v: &serde_json::Value, place: Uuid) -> (i64, i64) {
        let s = &place_of(v, place).unwrap()["stays"];
        (s["count"].as_i64().unwrap(), s["minutes"].as_i64().unwrap())
    }

    async fn radius(app: &App, user: Uuid, place: Uuid, m: i64, written: &str) -> Uuid {
        put(
            app,
            user,
            radius_raw(Uuid::new_v4(), place, serde_json::json!(m)),
            written,
        )
        .await
    }

    async fn place_at(app: &App, user: Uuid, name: &str, lat: f64, radius_m: i64) -> Uuid {
        let place = container(app, user).await;
        put_name(app, user, place, name, "2026-09-01T09:00:00+09:00").await;
        put_coord(
            app,
            user,
            place,
            lat,
            "first",
            serde_json::Value::Null,
            None,
            "2026-09-01T09:00:00+09:00",
        )
        .await;
        radius(app, user, place, radius_m, "2026-09-01T09:00:00+09:00").await;
        place
    }

    // Scenario: 広さの中の滞在はその場所に当たる
    #[tokio::test]
    async fn place_match_a_stay_inside_the_radius_is_assigned() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        stay_min(
            &app,
            user,
            "2026-09-10T10:00:00+09:00",
            60,
            north(LAT, 80.0),
        )
        .await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (1, 60));
    }

    // Scenario: 広さの外の滞在は当たらない
    #[tokio::test]
    async fn place_match_a_stay_outside_the_radius_is_not_assigned() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        stay_min(
            &app,
            user,
            "2026-09-10T10:00:00+09:00",
            60,
            north(LAT, 120.0),
        )
        .await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (0, 0));
    }

    // Scenario: 広さを広げると外にあった滞在も当たる
    #[tokio::test]
    async fn place_match_widening_the_radius_assigns_the_outside_stay() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        stay_min(
            &app,
            user,
            "2026-09-10T10:00:00+09:00",
            60,
            north(LAT, 120.0),
        )
        .await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (0, 0));
        radius(&app, user, place, 200, "2026-09-02T09:00:00+09:00").await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (1, 60));
    }

    // Scenario: 2 つの場所に入る滞在は近いほうに当たる
    #[tokio::test]
    async fn place_match_the_nearer_place_wins() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        // 滞在は LAT。A は 150 m 南、B は 90 m 北
        let a = place_at(&app, user, "A", north(LAT, -150.0), 300).await;
        let b = place_at(&app, user, "B", north(LAT, 90.0), 300).await;
        stay_min(&app, user, "2026-09-10T10:00:00+09:00", 60, LAT).await;
        let v = get(&app, user).await;
        assert_eq!(count_minutes(&v, b), (1, 60));
        assert_eq!(count_minutes(&v, a), (0, 0));
    }

    // Scenario: 同じ距離なら先に作った場所に当たる
    #[tokio::test]
    async fn place_match_a_tie_goes_to_the_place_made_first() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let a = place_at(&app, user, "A", north(LAT, 50.0), 100).await;
        let b = place_at(&app, user, "B", north(LAT, 50.0), 100).await;
        stay_min(&app, user, "2026-09-10T10:00:00+09:00", 60, LAT).await;
        let v = get(&app, user).await;
        assert_eq!(count_minutes(&v, a), (1, 60));
        assert_eq!(count_minutes(&v, b), (0, 0));
    }

    // Scenario: 照合は滞在に書き込まない
    #[tokio::test]
    async fn place_match_does_not_write_to_the_stays() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let id = stay_min(
            &app,
            user,
            "2026-09-10T10:00:00+09:00",
            60,
            north(LAT, 10.0),
        )
        .await;
        let snapshot = || async {
            sqlx::query_as::<_, (String, serde_json::Value, String)>(
                "SELECT raw, payload, content_hash FROM core.event WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap()
        };
        let before = snapshot().await;
        let (place, _, _) = registered(&app, user, "職場").await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (1, 60));
        assert_eq!(snapshot().await, before);
    }

    // Scenario: 消した滞在は場所に当たらない
    #[tokio::test]
    async fn place_match_a_deleted_stay_is_not_counted() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        let id = stay_min(&app, user, "2026-09-10T10:00:00+09:00", 60, LAT).await;
        stay_min(&app, user, "2026-09-11T10:00:00+09:00", 120, LAT).await;
        mark_deleted(&app, id).await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (1, 120));
    }

    /// 代表点を持たない滞在は当てない（D8）
    #[tokio::test]
    async fn place_match_a_stay_without_a_representative_point_is_not_assigned() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO core.event
               (id, user_id, logical_source, external_id, origin, event_time,
                tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
             VALUES ($1,$2,'s01-stay',$3,'derived','2026-09-10T01:00:00Z',540,'Asia/Tokyo',1,$3,'{}','{}')",
        )
        .bind(id)
        .bind(user)
        .bind(id.to_string())
        .execute(&app.pool)
        .await
        .unwrap();
        assert_eq!(count_minutes(&get(&app, user).await, place), (0, 0));
    }

    // Scenario: 直すと全期間が新しい座標で照らされる
    #[tokio::test]
    async fn place_window_a_fix_lights_the_whole_period_with_the_new_coordinate() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        let fixed = north(LAT, 1000.0);
        stay_min(
            &app,
            user,
            "2025-03-10T10:00:00+09:00",
            60,
            north(fixed, 50.0),
        )
        .await;
        put_coord(
            &app,
            user,
            place,
            fixed,
            "fix",
            serde_json::Value::Null,
            Some(first),
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (1, 60));
    }

    // Scenario: 直すと前の座標の近くの滞在は外れる
    #[tokio::test]
    async fn place_window_a_fix_drops_the_stay_near_the_old_coordinate() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        stay_min(&app, user, "2026-09-10T10:00:00+09:00", 60, LAT).await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (1, 60));
        put_coord(
            &app,
            user,
            place,
            north(LAT, 1000.0),
            "fix",
            serde_json::Value::Null,
            Some(first),
            "2026-09-11T09:00:00+09:00",
        )
        .await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (0, 0));
    }

    /// 座標 A（登録）の場所を 2026-04 に座標 B（A の 1 km 北）へ移した場所を作る
    async fn moved_place(
        app: &App,
        user: Uuid,
        valid_from: serde_json::Value,
        written: &str,
    ) -> Uuid {
        let (place, _, _) = registered(app, user, "職場").await;
        put_coord(
            app,
            user,
            place,
            north(LAT, 1000.0),
            "move",
            valid_from,
            None,
            written,
        )
        .await;
        place
    }

    // Scenario: 移ったなら前の座標で居た時間もこの場所
    #[tokio::test]
    async fn place_window_a_move_keeps_the_time_spent_at_the_old_coordinate() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let place = moved_place(&app, user, month("2026-04"), "2026-09-02T09:00:00+09:00").await;
        stay_min(&app, user, "2025-06-10T10:00:00+09:00", 60, LAT).await;
        stay_min(
            &app,
            user,
            "2026-05-10T10:00:00+09:00",
            120,
            north(LAT, 1000.0),
        )
        .await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (2, 180));
    }

    // Scenario: 移ったより前の新しい座標の滞在は当たらない
    #[tokio::test]
    async fn place_window_the_new_coordinate_does_not_light_the_time_before_the_move() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let place = moved_place(&app, user, month("2026-04"), "2026-09-02T09:00:00+09:00").await;
        stay_min(
            &app,
            user,
            "2025-06-10T10:00:00+09:00",
            60,
            north(LAT, 1000.0),
        )
        .await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (0, 0));
    }

    // Scenario: 移ったより後の前の座標の滞在は当たらない
    #[tokio::test]
    async fn place_window_the_old_coordinate_does_not_light_the_time_after_the_move() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let place = moved_place(&app, user, month("2026-04"), "2026-09-02T09:00:00+09:00").await;
        stay_min(&app, user, "2026-05-10T10:00:00+09:00", 60, LAT).await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (0, 0));
    }

    // Scenario: 年だけの移ったはその年の初めから当てる
    #[tokio::test]
    async fn place_window_a_year_only_move_starts_at_the_beginning_of_the_year() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let place = moved_place(
            &app,
            user,
            serde_json::json!({ "precision": "year", "date": "2026" }),
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        stay_min(
            &app,
            user,
            "2026-01-02T10:00:00+09:00",
            60,
            north(LAT, 1000.0),
        )
        .await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (1, 60));
    }

    // Scenario: いつから分からない移転は書いた日まで両方の座標で当てる
    #[tokio::test]
    async fn place_window_an_unknown_start_move_lights_both_coordinates_until_it_was_written() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let place = moved_place(
            &app,
            user,
            serde_json::json!({ "precision": "unknown", "date": null }),
            "2026-10-01T09:00:00+09:00",
        )
        .await;
        stay_min(&app, user, "2025-06-10T10:00:00+09:00", 60, LAT).await;
        stay_min(
            &app,
            user,
            "2025-07-10T10:00:00+09:00",
            120,
            north(LAT, 1000.0),
        )
        .await;
        stay_min(&app, user, "2026-10-05T10:00:00+09:00", 30, LAT).await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (2, 180));
    }

    // Scenario: 後から書いた古いいつからの移転が後を占める
    #[tokio::test]
    async fn place_window_a_later_written_older_move_takes_over_the_rest() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        let b = north(LAT, 1000.0);
        let c = north(LAT, 2000.0);
        put_coord(
            &app,
            user,
            place,
            b,
            "move",
            month("2026-04"),
            None,
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        put_coord(
            &app,
            user,
            place,
            c,
            "move",
            month("2025-06"),
            None,
            "2026-09-03T09:00:00+09:00",
        )
        .await;
        stay_min(&app, user, "2025-08-10T10:00:00+09:00", 60, c).await;
        stay_min(&app, user, "2026-05-10T10:00:00+09:00", 120, c).await;
        stay_min(&app, user, "2026-05-11T10:00:00+09:00", 30, b).await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (2, 180));
    }

    // Scenario: 直す記録を消すと直す前の座標に戻る
    #[tokio::test]
    async fn place_window_deleting_the_fix_restores_the_old_coordinate() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, first) = registered(&app, user, "職場").await;
        let fix = put_coord(
            &app,
            user,
            place,
            north(LAT, 1000.0),
            "fix",
            serde_json::Value::Null,
            Some(first),
            "2026-09-02T09:00:00+09:00",
        )
        .await;
        stay_min(&app, user, "2026-09-10T10:00:00+09:00", 60, LAT).await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (0, 0));
        mark_deleted(&app, fix).await;
        let v = get(&app, user).await;
        let p = place_of(&v, place).unwrap();
        assert_eq!(p["coord"]["record_id"], serde_json::json!(first));
        assert_eq!(count_minutes(&v, place), (1, 60));
    }

    // Scenario: 場所の滞在の件数と合計が返る
    #[tokio::test]
    async fn place_view_stays_count_and_total() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        stay_min(&app, user, "2026-09-10T10:00:00+09:00", 120, LAT).await;
        stay_min(&app, user, "2026-09-11T10:00:00+09:00", 180, LAT).await;
        assert_eq!(count_minutes(&get(&app, user).await, place), (2, 300));
    }

    // Scenario: 最後に居た日が返る
    #[tokio::test]
    async fn place_view_stays_last_day() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        assert!(place_of(&get(&app, user).await, place).unwrap()["stays"]["last_day"].is_null());
        stay(
            &app,
            user,
            "2026-09-20T09:00:00+09:00",
            "2026-09-20T10:00:00+09:00",
            LAT,
        )
        .await;
        stay(
            &app,
            user,
            "2026-09-28T17:00:00+09:00",
            "2026-09-28T18:00:00+09:00",
            LAT,
        )
        .await;
        let v = get(&app, user).await;
        assert_eq!(
            place_of(&v, place).unwrap()["stays"]["last_day"],
            "2026-09-28"
        );
    }

    fn hours_of(v: &serde_json::Value, place: Uuid) -> Vec<i64> {
        place_of(v, place).unwrap()["stays"]["hours"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h.as_i64().unwrap())
            .collect()
    }

    // Scenario: 24 時間の帯は時刻ごとの居た分を持つ
    #[tokio::test]
    async fn place_view_stays_hours_hold_the_minutes_per_hour() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        stay(
            &app,
            user,
            "2026-09-10T08:30:00+09:00",
            "2026-09-10T10:00:00+09:00",
            LAT,
        )
        .await;
        let mut want = vec![0; 24];
        want[8] = 30;
        want[9] = 60;
        assert_eq!(hours_of(&get(&app, user).await, place), want);
    }

    // Scenario: 日をまたぐ滞在は両方の日の時刻に分かれる
    #[tokio::test]
    async fn place_view_stays_hours_split_a_stay_across_midnight() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let (place, _, _) = registered(&app, user, "職場").await;
        stay(
            &app,
            user,
            "2026-09-10T23:00:00+09:00",
            "2026-09-11T01:00:00+09:00",
            LAT,
        )
        .await;
        let mut want = vec![0; 24];
        want[23] = 60;
        want[0] = 60;
        assert_eq!(hours_of(&get(&app, user).await, place), want);
    }

    // Scenario: 場所は最近居た順に返る
    #[tokio::test]
    async fn place_view_stays_places_come_back_most_recent_first() {
        let app = at_now(app().await, NOW);
        let user = testdb::user();
        let a = place_at(&app, user, "A", north(LAT, 10_000.0), 100).await;
        let b = place_at(&app, user, "B", north(LAT, 20_000.0), 100).await;
        let c = place_at(&app, user, "C", north(LAT, 30_000.0), 100).await;
        stay_min(
            &app,
            user,
            "2026-09-28T10:00:00+09:00",
            60,
            north(LAT, 10_000.0),
        )
        .await;
        stay_min(
            &app,
            user,
            "2026-09-30T10:00:00+09:00",
            60,
            north(LAT, 20_000.0),
        )
        .await;
        let v = get(&app, user).await;
        let ids: Vec<&serde_json::Value> = v["places"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| &p["id"])
            .collect();
        assert_eq!(
            ids,
            vec![
                &serde_json::json!(b),
                &serde_json::json!(a),
                &serde_json::json!(c)
            ]
        );
    }
}
