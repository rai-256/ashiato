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

fn at(hhmm: &str) -> String {
    format!("2000-01-01T{hhmm}:00Z")
}

/// PC のウィンドウの記録を置く（`device` は NULL 可）。`raw` は地図の座標（`true`）か座標なし。
async fn window(
    pool: &sqlx::PgPool,
    user: Uuid,
    device: Option<&str>,
    hhmm: &str,
    coords: bool,
) -> Uuid {
    put_as(
        pool,
        user,
        "c02-window",
        device,
        &at(hhmm),
        json!({}),
        if coords { MAP } else { "plain" },
    )
    .await
}

fn with_coords() -> Value {
    json!({"lat": 35.0, "lon": 139.0})
}

#[tokio::test]
async fn ch02_span_same_time_pair_runs_to_the_later_record() {
    // Scenario: 同じ時刻に並んだ終わりを持たない記録は、どれも時刻が後の次の記録までの幅で読む
    let (pool, drop) = fresh().await;
    let user = Uuid::new_v4();
    erased_stay(&pool, user, &at("01:30"), &at("02:00"), &[]).await;
    let a = window(&pool, user, Some("pc"), "01:00", true).await;
    let b = window(&pool, user, Some("pc"), "01:00", true).await;
    let c = window(&pool, user, Some("pc"), "03:00", false).await;
    let kept = kept_ids(&pool, &export::excluded_sql("now()"), &[a, b, c])
        .await
        .0;
    drop.await;
    assert_eq!(kept, BTreeSet::from([c]));
}

#[tokio::test]
async fn ch02_span_null_device_runs_within_null_devices() {
    // Scenario: 端末を持たない記録は、端末を持たない同じソースの次の記録までの幅で読む
    let (pool, drop) = fresh().await;
    let user = Uuid::new_v4();
    erased_stay(&pool, user, &at("01:00"), &at("02:00"), &[]).await;
    let a = window(&pool, user, None, "00:10", true).await;
    let b = window(&pool, user, Some("pc"), "00:40", false).await;
    let c = window(&pool, user, None, "02:30", false).await;
    let kept = kept_ids(&pool, &export::excluded_sql("now()"), &[a, b, c])
        .await
        .0;
    drop.await;
    assert_eq!(kept, BTreeSet::from([b, c]));
}

#[tokio::test]
async fn ch02_span_record_without_coordinates_ends_the_width() {
    // Scenario: 緯度経度を持たない記録も、次の記録として幅の終わりになる
    let (pool, drop) = fresh().await;
    let user = Uuid::new_v4();
    erased_stay(&pool, user, &at("01:00"), &at("02:00"), &[]).await;
    let a = window(&pool, user, Some("pc"), "00:10", true).await;
    let b = window(&pool, user, Some("pc"), "00:40", false).await;
    let c = window(&pool, user, Some("pc"), "02:30", false).await;
    let kept = kept_ids(&pool, &export::excluded_sql("now()"), &[a, b, c])
        .await
        .0;
    drop.await;
    assert!(kept.contains(&a), "00:10 は 00:40 で終わるので出る");
}

#[tokio::test]
async fn ch02_span_version_runs_to_the_next_record_of_the_parent_group() {
    // Scenario: 前の版の幅は、親と同じ組の記録のうち前の版の時刻より後の最初の記録までで読む
    let (pool, drop) = fresh().await;
    let user = Uuid::new_v4();
    erased_stay(&pool, user, &at("01:00"), &at("02:00"), &[]).await;
    let parent = window(&pool, user, Some("pc"), "03:00", false).await;
    let v1 = put_version(&pool, parent, user, 1, &at("00:10"), with_coords()).await;
    let v2 = put_version(&pool, parent, user, 2, &at("00:40"), with_coords()).await;
    window(&pool, user, Some("pc"), "00:20", false).await;
    window(&pool, user, Some("pc"), "02:30", false).await;
    let kept = kept_ids(&pool, &export::excluded_sql("now()"), &[v1, v2])
        .await
        .1;
    drop.await;
    assert_eq!(kept, BTreeSet::from([v1]), "00:40 の前の版は出ない");
}

#[tokio::test]
async fn ch02_span_version_stops_before_the_erased_time() {
    // Scenario: 前の版の幅は、次の記録が消した時間の前にあれば消した時間に届かない
    let (pool, drop) = fresh().await;
    let user = Uuid::new_v4();
    erased_stay(&pool, user, &at("01:00"), &at("02:00"), &[]).await;
    let parent = window(&pool, user, Some("pc"), "03:00", false).await;
    let v1 = put_version(&pool, parent, user, 1, &at("00:10"), with_coords()).await;
    window(&pool, user, Some("pc"), "00:20", false).await;
    window(&pool, user, Some("pc"), "02:30", false).await;
    let kept = kept_ids(&pool, &export::excluded_sql("now()"), &[v1])
        .await
        .1;
    drop.await;
    assert_eq!(kept, BTreeSet::from([v1]));
}

#[tokio::test]
async fn ch02_span_version_is_not_the_next_of_another_version() {
    // Scenario: 前の版は、ほかの前の版を次の記録にしない
    let (pool, drop) = fresh().await;
    let user = Uuid::new_v4();
    erased_stay(&pool, user, &at("01:00"), &at("02:00"), &[]).await;
    let parent = window(&pool, user, Some("pc"), "03:00", false).await;
    let v1 = put_version(&pool, parent, user, 1, &at("00:30"), with_coords()).await;
    let v2 = put_version(&pool, parent, user, 2, &at("00:40"), with_coords()).await;
    window(&pool, user, Some("pc"), "02:30", false).await;
    let kept = kept_ids(&pool, &export::excluded_sql("now()"), &[v1, v2])
        .await
        .1;
    drop.await;
    assert!(
        !kept.contains(&v1),
        "00:30 の前の版は 02:30 まで続くので出ない"
    );
}

#[tokio::test]
async fn ch02_span_version_group_uses_the_parent_user() {
    // Scenario: 前の版の組の利用者は親の利用者で読む
    let (pool, drop) = fresh().await;
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    erased_stay(&pool, a, &at("01:00"), &at("02:00"), &[]).await;
    let parent = window(&pool, a, Some("pc"), "03:00", false).await;
    let v = put_version(&pool, parent, b, 1, &at("00:40"), with_coords()).await;
    window(&pool, a, Some("pc"), "02:30", false).await;
    window(&pool, b, Some("pc"), "00:50", false).await;
    let kept = kept_ids(&pool, &export::excluded_sql("now()"), &[v])
        .await
        .1;
    drop.await;
    assert!(kept.is_empty(), "親の利用者の 02:30 まで続くので出ない");
}

#[tokio::test]
async fn ch02_span_marked_record_is_not_the_next() {
    // Scenario: 削除の印の付いた記録は、次の記録にならない
    let (pool, drop) = fresh().await;
    let user = Uuid::new_v4();
    erased_stay(&pool, user, &at("01:00"), &at("02:00"), &[]).await;
    let a = window(&pool, user, Some("pc"), "00:10", true).await;
    let b = window(&pool, user, Some("pc"), "00:40", false).await;
    window(&pool, user, Some("pc"), "02:30", false).await;
    mark_deleted(&pool, b, "user").await;
    let kept = kept_ids(&pool, &export::excluded_sql("now()"), &[a])
        .await
        .0;
    drop.await;
    assert!(kept.is_empty(), "印の付いた 00:40 は飛ばし 02:30 まで続く");
}

#[tokio::test]
async fn ch02_span_nothing_erased_keeps_everything() {
    // Scenario: 消した滞在が無ければ、緯度経度を持つ記録も幅を問わず出る
    let (pool, drop) = fresh().await;
    let user = Uuid::new_v4();
    let e = window(&pool, user, Some("pc"), "00:10", true).await;
    let v = put_version(&pool, e, user, 1, &at("00:05"), with_coords()).await;
    let (ke, kv) = kept_ids(&pool, &export::excluded_sql("now()"), &[e, v]).await;
    drop.await;
    assert_eq!((ke, kv), (BTreeSet::from([e]), BTreeSet::from([v])));
}
