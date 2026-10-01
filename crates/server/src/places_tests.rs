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

    const TOKEN: &str = "test-token-0123456789abcdef";
    /// 128 bit を base64url で書いた 22 文字（design D3）
    const NONCE: &str = "Zm9vYmFyYmF6cXV4MTIzNDU2";
    /// 64 bit（11 文字）
    const SHORT_NONCE: &str = "Zm9vYmFyYmE";
    const AT: &str = "2026-10-01T02:00:00Z";

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

    /// 器を作る（`POST /places`）
    async fn container(app: &App, user: Uuid) -> Uuid {
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
    fn raw_of(id: Uuid, place: Uuid, nonce: &str, extra: serde_json::Value) -> String {
        let mut v = serde_json::json!({ "record": id, "place": place, "nonce": nonce });
        for (k, val) in extra.as_object().unwrap() {
            v[k] = val.clone();
        }
        v.to_string()
    }

    fn name_raw(id: Uuid, place: Uuid, name: &str) -> String {
        raw_of(
            id,
            place,
            NONCE,
            serde_json::json!({ "field": "name", "name": name }),
        )
    }

    fn coord_raw(
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

    fn first_raw(id: Uuid, place: Uuid) -> String {
        coord_raw(id, place, 35.681236, "first", serde_json::Value::Null, None)
    }

    fn radius_raw(id: Uuid, place: Uuid, radius: serde_json::Value) -> String {
        raw_of(
            id,
            place,
            NONCE,
            serde_json::json!({ "field": "radius", "radius_m": radius }),
        )
    }

    fn item(id: Uuid, user: Uuid, raw: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "user_id": user, "logical_source": places::SOURCE,
            "external_id": null, "device_id": null, "origin": "authored",
            "event_time": AT, "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
            "schema_version": 1, "raw": raw, "payload": {},
        })
    }

    async fn send_item(app: &App, item: serde_json::Value) -> IngestResult {
        let (_, Json(mut res)) = crate::ingest(State(app.clone()), auth(), Json(item_array(item)))
            .await
            .expect("取り込み口");
        res.pop().expect("1 件ぶんの結果")
    }

    fn item_array(item: serde_json::Value) -> serde_json::Value {
        serde_json::json!([item])
    }

    /// 原文の `record` を記録の識別子にして送る（原文が JSON でないときだけ新しい識別子）
    async fn send(app: &App, user: Uuid, raw: &str) -> IngestResult {
        let id = serde_json::from_str::<serde_json::Value>(raw)
            .ok()
            .and_then(|v| v["record"].as_str().and_then(|s| Uuid::parse_str(s).ok()))
            .unwrap_or_else(Uuid::new_v4);
        send_item(app, item(id, user, raw)).await
    }

    /// 結果の理由の種別を文字列で見る
    fn error_of(r: &IngestResult) -> String {
        assert!(!r.accepted, "受理されている");
        serde_json::to_value(r).unwrap()["error"]
            .as_str()
            .expect("理由の種別")
            .to_string()
    }

    async fn rejects(app: &App, user: Uuid, raw: &str, want: &str) {
        let r = send(app, user, raw).await;
        assert_eq!(error_of(&r), want, "原文: {raw}");
    }

    /// 送った 1 件が受理された
    fn assert_accepted(r: &IngestResult, why: &str) {
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
