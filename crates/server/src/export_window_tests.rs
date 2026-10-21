// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの「次の記録」の引きを窓関数の一度読みにする change（CH02）の試験。
//!
//! **まっさらな DB**（全移行だけを当てたもの）で走らせる。共有の DB には他の試験の消した滞在が混ざり、
//! いまの SQL がそこで遅い（design D6）。試験の終わりに `drop` を待つ。
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use serde_json::{json, Value};
use uuid::Uuid;

use crate::export::{self, legacy, ExcludedSql};

/// 全移行を当てたまっさらな DB。**`drop` を必ず待つ**（接続を閉じて DB を消す）。
async fn fresh() -> (sqlx::PgPool, impl std::future::Future<Output = ()>) {
    let (pool, drop) = crate::stay_tests::fresh_db().await;
    for (label, sql) in crate::MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("{label}: {e}"));
    }
    (pool, drop)
}

/// 記録を 1 件置く（`export_excluded_tests::World::put_as` と同じ列）。
/// `device` は `NULL` を渡せる。`at` はマイクロ秒まで書ける（例 `2000-01-01T00:00:00.000001Z`）。
async fn put_as(
    pool: &sqlx::PgPool,
    user: Uuid,
    source: &str,
    device: Option<&str>,
    at: &str,
    payload: Value,
    raw: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,$4,$9,$5::timestamptz,540,'Asia/Tokyo',1,$6,$7,$8)",
    )
    .bind(id)
    .bind(user)
    .bind(source)
    .bind(device)
    .bind(at)
    .bind(Uuid::new_v4().to_string())
    .bind(raw)
    .bind(payload)
    .bind(if source == "s01-stay" {
        "derived"
    } else {
        "collected"
    })
    .execute(pool)
    .await
    .unwrap();
    id
}

/// 前の版を 1 つ足す。`user` は親と違う値にもできる。
async fn put_version(
    pool: &sqlx::PgPool,
    event: Uuid,
    user: Uuid,
    no: i32,
    at: &str,
    payload: Value,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event_version
           (id, event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
         SELECT $1, e.id, $7, e.logical_source, $2, $3::timestamptz, $4, '{}', $5
           FROM core.event e WHERE e.id = $6",
    )
    .bind(id)
    .bind(no)
    .bind(at)
    .bind(Uuid::new_v4().to_string())
    .bind(payload)
    .bind(event)
    .bind(user)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn mark_deleted(pool: &sqlx::PgPool, id: Uuid, by: &str) {
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = $2 WHERE id = $1")
        .bind(id)
        .bind(by)
        .execute(pool)
        .await
        .unwrap();
}

/// 本人が消した滞在を置く（`end` と緯度経度を持つ）。`versions` は前の版の `(始まり, 終わり)`。
async fn erased_stay(
    pool: &sqlx::PgPool,
    user: Uuid,
    start: &str,
    end: &str,
    versions: &[(&str, &str)],
) -> Uuid {
    let id = put_as(
        pool,
        user,
        "s01-stay",
        Some("server"),
        start,
        json!({"end": end, "lat": 35.0, "lon": 139.0}),
        "{}",
    )
    .await;
    for (no, (s, e)) in versions.iter().enumerate() {
        put_version(
            pool,
            id,
            user,
            no as i32 + 1,
            s,
            json!({"end": e, "lat": 35.0, "lon": 139.0}),
        )
        .await;
    }
    mark_deleted(pool, id, "user").await;
    id
}

/// 断片を通して**残る**記録の id と前の版の id（`ids` のうち）。
/// 前の版がファイルに出るかは親が除かれるかにも依るので、`excluded_version` の id だけでは見ず `version_kept` を通す。
async fn kept_ids(
    pool: &sqlx::PgPool,
    sql: &ExcludedSql,
    ids: &[Uuid],
) -> (BTreeSet<Uuid>, BTreeSet<Uuid>) {
    let events = format!(
        "WITH {} SELECT e.id FROM core.event_live e WHERE e.id = ANY($1) AND {}",
        sql.ctes(),
        sql.event_kept("e")
    );
    let versions = format!(
        "WITH {} SELECT v.id FROM core.event_version_live v WHERE v.id = ANY($1) AND {}",
        sql.ctes(),
        sql.version_kept("v")
    );
    let e: Vec<(Uuid,)> = sqlx::query_as(&events)
        .bind(ids)
        .fetch_all(pool)
        .await
        .unwrap();
    let v: Vec<(Uuid,)> = sqlx::query_as(&versions)
        .bind(ids)
        .fetch_all(pool)
        .await
        .unwrap();
    (
        e.into_iter().map(|(i,)| i).collect(),
        v.into_iter().map(|(i,)| i).collect(),
    )
}

/// 断片が除く記録の id の集合（`excluded_event`）。
async fn excluded_ids(pool: &sqlx::PgPool, sql: &ExcludedSql) -> BTreeSet<Uuid> {
    let q = format!(
        "WITH {} SELECT id FROM {}",
        sql.ctes(),
        export::EXCLUDED_EVENT
    );
    let got: Vec<(Uuid,)> = sqlx::query_as(&q).fetch_all(pool).await.unwrap();
    got.into_iter().map(|(i,)| i).collect()
}

const MAP: &str = "https://www.google.com/maps/@35.6812,139.7671,15z";

#[tokio::test]
async fn ch02_span_legacy_matches_current_before_the_change() {
    let (pool, drop) = fresh().await;
    let user = Uuid::new_v4();
    let day = "2000-01-01";
    let at = |hhmm: &str| format!("{day}T{hhmm}:00Z");
    // `export_excluded_sql_record_without_end_runs_to_the_next_record` と同じ配置
    erased_stay(&pool, user, &at("01:00"), &at("02:00"), &[]).await;
    let a = put_as(
        &pool,
        user,
        "c02-window",
        Some("pc"),
        &at("00:10"),
        json!({}),
        MAP,
    )
    .await;
    let b = put_as(
        &pool,
        user,
        "c02-window",
        Some("pc"),
        &at("00:40"),
        json!({}),
        MAP,
    )
    .await;
    let c = put_as(
        &pool,
        user,
        "c02-window",
        Some("pc"),
        &at("02:30"),
        json!({}),
        "plain",
    )
    .await;

    let now = export::excluded_sql("now()");
    let old = legacy::excluded_sql("now()");
    let (now_ids, old_ids) = (
        excluded_ids(&pool, &now).await,
        excluded_ids(&pool, &old).await,
    );
    let kept = kept_ids(&pool, &old, &[a, b, c]).await.0;
    drop.await;

    assert_eq!(now_ids, old_ids, "いまの断片と写しの除く集合が違う");
    // 00:40 の記録は次の 02:30 まで続くので消した時間に重なる。00:10 は 00:40 まで
    assert!(
        old_ids.contains(&b) && !old_ids.contains(&c),
        "配置が効いていない"
    );
    assert_eq!(kept, BTreeSet::from([a, c]));
}
