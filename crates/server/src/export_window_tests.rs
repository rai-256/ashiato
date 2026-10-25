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
pub(crate) async fn put_as(
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
pub(crate) async fn put_version(
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
pub(crate) async fn erased_stay(
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

pub(crate) const MAP: &str = "https://www.google.com/maps/@35.6812,139.7671,15z";

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

// ---- Task 3: 新旧の突き合わせ（変わらないことの固定。design D6）----

/// 時刻（`HH:MM:SS` かマイクロ秒つき）を、試験の日の ISO の文字列にする。
fn ts(time: &str) -> String {
    format!("2000-01-01T{time}Z")
}

/// ウィンドウ系の記録を 1 件置く。`coords` は原文に地図の座標を持たせる（持たなければ緯度経度を持たない記録）。
async fn rec(
    pool: &sqlx::PgPool,
    user: Uuid,
    source: &str,
    device: Option<&str>,
    time: &str,
    coords: bool,
) -> Uuid {
    put_as(
        pool,
        user,
        source,
        device,
        &ts(time),
        json!({}),
        if coords { MAP } else { "plain" },
    )
    .await
}

/// 族ごとの配置。`excluded` は `now()` で除かれる記録と前の版の id、`kept` は残る id。
struct Family {
    name: &'static str,
    excluded: Vec<Uuid>,
    kept: Vec<Uuid>,
}

struct World {
    families: Vec<Family>,
    /// 同じ時刻の組のうち、`(event_time, id)` の順で先の記録（いまの SQL では隣の同じ時刻の記録までの幅 0 で残る）。
    same_time_first: Uuid,
}

impl World {
    fn all_ids(&self) -> Vec<Uuid> {
        self.families
            .iter()
            .flat_map(|f| f.excluded.iter().chain(&f.kept))
            .copied()
            .collect()
    }
}

/// design D6 の偽データの族を置く。`erase` が偽なら消した滞在だけを置かない。
/// 消した滞在は 01:00〜02:00 と、前の版の広い時間 03:30〜04:30（消した時間の終わりは 04:30）。
async fn seed(pool: &sqlx::PgPool, erase: bool) -> World {
    if erase {
        erased_stay(
            pool,
            Uuid::new_v4(),
            &at("01:00"),
            &at("02:00"),
            &[(&at("03:30"), &at("04:30"))],
        )
        .await;
    }
    let pc = Some("pc");
    let mut families = Vec::new();

    // 同じ時刻の組: 先の記録は新旧で幅が違う（Q1）
    let u = Uuid::new_v4();
    let a = rec(pool, u, "c02-window", pc, "00:10:00", true).await;
    let b = rec(pool, u, "c02-window", pc, "00:10:00", true).await;
    let c = rec(pool, u, "c02-window", pc, "02:30:00", false).await;
    families.push(Family {
        name: "同じ時刻の組",
        excluded: vec![a, b],
        kept: vec![c],
    });
    let same_time_first = a.min(b);

    // 端末 NULL は端末 NULL どうしで組む
    let u = Uuid::new_v4();
    let a = rec(pool, u, "c02-window", None, "00:10:00", true).await;
    let m = rec(pool, u, "c02-window", None, "00:40:00", false).await;
    let x = rec(pool, u, "c02-window", None, "00:50:00", true).await;
    let c = rec(pool, u, "c02-window", None, "02:30:00", false).await;
    let p = rec(pool, u, "c02-window", pc, "00:20:00", false).await;
    families.push(Family {
        name: "端末 NULL",
        excluded: vec![x],
        kept: vec![a, m, c, p],
    });

    // 前の版の時刻のずれ 0 と 5 秒（次の記録は厳密に後）
    let u = Uuid::new_v4();
    let parent = rec(pool, u, "c02-window", pc, "05:30:00", false).await;
    let r1 = rec(pool, u, "c02-window", pc, "00:40:00", false).await;
    let r2 = rec(pool, u, "c02-window", pc, "02:30:00", false).await;
    let r3 = rec(pool, u, "c02-window", pc, "00:20:05", false).await;
    let va = put_version(pool, parent, u, 1, &ts("00:40:00"), with_coords()).await;
    let vb = put_version(pool, parent, u, 2, &ts("00:40:05"), with_coords()).await;
    let vd = put_version(pool, parent, u, 3, &ts("00:20:00"), with_coords()).await;
    families.push(Family {
        name: "前の版の時刻のずれ 0 と 5 秒",
        excluded: vec![va, vb],
        kept: vec![vd, parent, r1, r2, r3],
    });

    // 前の版の user_id が親と違う（組は親の利用者で読む）
    let (ua, ub) = (Uuid::new_v4(), Uuid::new_v4());
    let parent = rec(pool, ua, "c02-window", pc, "05:30:00", false).await;
    let v = put_version(pool, parent, ub, 1, &ts("00:40:00"), with_coords()).await;
    let v2 = put_version(pool, parent, ub, 2, &ts("00:20:00"), with_coords()).await;
    let ra = rec(pool, ua, "c02-window", pc, "02:30:00", false).await;
    let rb = rec(pool, ub, "c02-window", pc, "00:50:00", false).await;
    let ra2 = rec(pool, ua, "c02-window", pc, "00:25:00", false).await;
    families.push(Family {
        name: "前の版の user_id が親と違う",
        excluded: vec![v],
        kept: vec![v2, parent, ra, rb, ra2],
    });

    // 削除の印を間に挟む（印の付いた記録は次の記録にならない）
    let u = Uuid::new_v4();
    let a2 = rec(pool, u, "c02-window", pc, "00:05:00", true).await;
    let b2 = rec(pool, u, "c02-window", pc, "00:08:00", false).await;
    let a = rec(pool, u, "c02-window", pc, "00:10:00", true).await;
    let marked = rec(pool, u, "c02-window", pc, "00:40:00", false).await;
    let c = rec(pool, u, "c02-window", pc, "02:30:00", false).await;
    mark_deleted(pool, marked, "user").await;
    families.push(Family {
        name: "削除の印を間に挟む",
        excluded: vec![a],
        kept: vec![a2, b2, c],
    });

    // 緯度経度を持たない行が次の記録になる
    let u = Uuid::new_v4();
    let a = rec(pool, u, "c02-window", Some("nc"), "00:10:00", true).await;
    let b = rec(pool, u, "c02-window", Some("nc"), "00:40:00", false).await;
    let a2 = rec(pool, u, "c02-window", Some("nc"), "00:50:00", true).await;
    let c = rec(pool, u, "c02-window", Some("nc"), "02:30:00", false).await;
    families.push(Family {
        name: "緯度経度を持たない行が次の記録",
        excluded: vec![a2],
        kept: vec![a, b, c],
    });

    // 消した時間より後の行だけが次の記録になる（組ごとの後の最初の 1 行。t が消した滞在より前でも効く）
    let u = Uuid::new_v4();
    let g = rec(pool, u, "c02-window", Some("far"), "00:10:00", true).await;
    let h = rec(pool, u, "c02-window", Some("far"), "06:00:00", false).await;
    families.push(Family {
        name: "消した時間より後の行が次の記録",
        excluded: vec![g],
        kept: vec![h],
    });

    // 次の記録が無い（写しの時点まで）
    let u = Uuid::new_v4();
    let z = rec(pool, u, "c02-window", Some("alone"), "00:15:00", true).await;
    let zp = rec(pool, u, "c02-window", Some("alone2"), "00:16:00", false).await;
    families.push(Family {
        name: "次の記録が無い",
        excluded: vec![z],
        kept: vec![zp],
    });

    // ほかの前の版は次の記録の候補にならない
    let u = Uuid::new_v4();
    let parent = rec(pool, u, "c02-window", pc, "05:30:00", false).await;
    let v1 = put_version(pool, parent, u, 1, &ts("00:30:00"), with_coords()).await;
    let v2 = put_version(pool, parent, u, 2, &ts("00:40:00"), with_coords()).await;
    let v3 = put_version(pool, parent, u, 3, &ts("00:20:00"), with_coords()).await;
    let r = rec(pool, u, "c02-window", pc, "02:30:00", false).await;
    let r3 = rec(pool, u, "c02-window", pc, "00:25:00", false).await;
    families.push(Family {
        name: "ほかの前の版は候補にならない",
        excluded: vec![v1, v2],
        kept: vec![v3, parent, r, r3],
    });

    // 1 µs の差（消した滞在の始まり 01:00:00 の 1 µs 前）
    let u = Uuid::new_v4();
    let x = rec(pool, u, "c02-window", Some("us1"), "00:59:59.999999", true).await;
    let y = rec(pool, u, "c02-window", Some("us1"), "01:00:00.000000", false).await;
    let x2 = rec(pool, u, "c02-window", Some("us2"), "00:59:59.999998", true).await;
    let y2 = rec(pool, u, "c02-window", Some("us2"), "00:59:59.999999", false).await;
    let z2 = rec(pool, u, "c02-window", Some("us2"), "02:30:00", false).await;
    families.push(Family {
        name: "1 µs の差",
        excluded: vec![x],
        kept: vec![y, x2, y2, z2],
    });

    // (a) の読めた終わりが記録の時刻より前: 読めないものとして次の記録まで
    let u = Uuid::new_v4();
    let e2 = put_as(
        pool,
        u,
        "c02-window",
        pc,
        &ts("00:20:00"),
        json!({"end": ts("00:25:00")}),
        MAP,
    )
    .await;
    let e1 = put_as(
        pool,
        u,
        "c02-window",
        pc,
        &ts("00:40:00"),
        json!({"end": ts("00:10:00")}),
        MAP,
    )
    .await;
    let f = rec(pool, u, "c02-window", pc, "02:30:00", false).await;
    families.push(Family {
        name: "終わりが記録の時刻より前",
        excluded: vec![e1],
        kept: vec![e2, f],
    });

    // ソースの種類: 点・end の終わり・visit_duration_us の長さ・次の記録まで
    let u = Uuid::new_v4();
    let loc = |end: Option<String>| match end {
        Some(end) => json!({"lat": 35.0, "lon": 139.0, "end": end}),
        None => with_coords(),
    };
    let l1 = put_as(
        pool,
        u,
        "c01-location",
        None,
        &ts("00:30:00"),
        loc(None),
        "plain",
    )
    .await;
    let l2 = put_as(
        pool,
        u,
        "c01-location",
        None,
        &ts("01:30:00"),
        loc(None),
        "plain",
    )
    .await;
    let l3 = put_as(
        pool,
        u,
        "c01-location",
        None,
        &ts("00:31:00"),
        loc(Some(ts("01:10:00"))),
        "plain",
    )
    .await;
    let l4 = put_as(
        pool,
        u,
        "c01-location",
        None,
        &ts("00:20:00"),
        loc(Some(ts("00:25:00"))),
        "plain",
    )
    .await;
    let d1 = put_as(
        pool,
        u,
        "c02-browser-history",
        Some("b"),
        &ts("00:40:00"),
        json!({"visit_duration_us": 1_800_000_000_i64}),
        MAP,
    )
    .await;
    let d2 = put_as(
        pool,
        u,
        "c02-browser-history",
        Some("b"),
        &ts("00:41:00"),
        json!({"visit_duration_us": 600_000_000_i64}),
        MAP,
    )
    .await;
    let w1 = rec(pool, u, "c02-window", Some("k"), "00:10:00", true).await;
    let w2 = rec(pool, u, "c02-window", Some("k"), "02:30:00", false).await;
    families.push(Family {
        name: "ソースの種類",
        excluded: vec![l2, l3, d1, w1],
        kept: vec![l1, l4, d2, w2],
    });

    // 消した滞在の前の版の広い時間（03:30〜04:30）に届く幅
    let u = Uuid::new_v4();
    let wp = rec(pool, u, "c02-window", Some("wide"), "02:40:00", true).await;
    let q = rec(pool, u, "c02-window", Some("wide"), "03:20:00", false).await;
    let w = rec(pool, u, "c02-window", Some("wide"), "03:25:00", true).await;
    let n = rec(pool, u, "c02-window", Some("wide"), "03:35:00", false).await;
    families.push(Family {
        name: "消した滞在の前の版の広い時間",
        excluded: vec![w],
        kept: vec![wp, q, n],
    });

    World {
        families,
        same_time_first,
    }
}

/// 断片が除く記録・前の版の id と、`ids` のうち残る記録・前の版の id。
#[derive(Debug, PartialEq)]
struct Outcome {
    excluded_events: BTreeSet<Uuid>,
    excluded_versions: BTreeSet<Uuid>,
    kept_events: BTreeSet<Uuid>,
    kept_versions: BTreeSet<Uuid>,
}

async fn outcome(pool: &sqlx::PgPool, sql: &ExcludedSql, ids: &[Uuid]) -> Outcome {
    let q = format!(
        "WITH {} SELECT id FROM {}",
        sql.ctes(),
        export::EXCLUDED_VERSION
    );
    let versions: Vec<(Uuid,)> = sqlx::query_as(&q).fetch_all(pool).await.unwrap();
    let (kept_events, kept_versions) = kept_ids(pool, sql, ids).await;
    Outcome {
        excluded_events: excluded_ids(pool, sql).await,
        excluded_versions: versions.into_iter().map(|(i,)| i).collect(),
        kept_events,
        kept_versions,
    }
}

/// 固定の時刻の式 3 つ（消した滞在の始まりより前・消した時間の中・後）。
const FIXED_SNAPSHOTS: [&str; 3] = [
    "'2000-01-01T00:30:00Z'::timestamptz",
    "'2000-01-01T01:30:00Z'::timestamptz",
    "'2000-01-01T05:00:00Z'::timestamptz",
];

#[tokio::test]
async fn ch02_same_matches_the_strict_legacy_on_every_family() {
    let (pool, drop) = fresh().await;
    let world = seed(&pool, true).await;
    let ids = world.all_ids();
    let mut results = Vec::new();
    for t in ["now()"].into_iter().chain(FIXED_SNAPSHOTS) {
        let new = outcome(&pool, &export::excluded_sql(t), &ids).await;
        let old = outcome(&pool, &legacy::excluded_sql_strict(t), &ids).await;
        results.push((t, new, old));
    }
    drop.await;

    for (t, new, old) in &results {
        assert_eq!(new, old, "t = {t}: 新旧の除く集合か残る集合が違う");
    }
    // 配置が効いていること: 族ごとに除かれるものと残るものの両方があり、`now()` でその通りになる
    let (_, now, _) = &results[0];
    let excluded: BTreeSet<Uuid> = now
        .excluded_events
        .union(&now.excluded_versions)
        .copied()
        .collect();
    let kept: BTreeSet<Uuid> = now.kept_events.union(&now.kept_versions).copied().collect();
    for f in &world.families {
        assert!(
            !f.excluded.is_empty() && !f.kept.is_empty(),
            "{}: 除かれる側か残る側が空",
            f.name
        );
        for id in &f.excluded {
            assert!(
                excluded.contains(id) && !kept.contains(id),
                "{}: 除かれるはずが残っている",
                f.name
            );
        }
        for id in &f.kept {
            assert!(
                !excluded.contains(id) && kept.contains(id),
                "{}: 残るはずが除かれている",
                f.name
            );
        }
    }
}

#[tokio::test]
async fn ch02_same_differs_from_the_legacy_only_on_same_time_pairs() {
    let (pool, drop) = fresh().await;
    let world = seed(&pool, true).await;
    let ids = world.all_ids();
    let new = outcome(&pool, &export::excluded_sql("now()"), &ids).await;
    let old = outcome(&pool, &legacy::excluded_sql("now()"), &ids).await;
    drop.await;

    // 同じ時刻の組の、先の記録だけが、新しい側にだけ除かれる（Q1。除く側にだけ変わる）
    assert!(old.excluded_events.is_subset(&new.excluded_events));
    let only_new: BTreeSet<Uuid> = new
        .excluded_events
        .difference(&old.excluded_events)
        .copied()
        .collect();
    assert_eq!(only_new, BTreeSet::from([world.same_time_first]));
    assert_eq!(new.excluded_versions, old.excluded_versions);
}

#[tokio::test]
async fn ch02_same_nothing_erased_matches_the_legacy() {
    let (pool, drop) = fresh().await;
    let world = seed(&pool, false).await;
    let ids = world.all_ids();
    let new = outcome(&pool, &export::excluded_sql("now()"), &ids).await;
    let old = outcome(&pool, &legacy::excluded_sql("now()"), &ids).await;
    let strict = outcome(&pool, &legacy::excluded_sql_strict("now()"), &ids).await;
    drop.await;

    for (name, o) in [("新", &new), ("旧", &old), ("旧の厳密版", &strict)] {
        assert!(
            o.excluded_events.is_empty() && o.excluded_versions.is_empty(),
            "{name}: 消した滞在が無いのに除く記録がある"
        );
    }
    // 記録は置いてあり、全部残る
    assert!(!new.kept_events.is_empty() && !new.kept_versions.is_empty());
    assert_eq!(new, old);
}

#[tokio::test]
async fn ch02_same_runs_in_a_read_only_transaction() {
    let (pool, drop) = fresh().await;
    let world = seed(&pool, true).await;
    let ids = world.all_ids();
    let sql = export::excluded_sql("now()");
    let expected = outcome(&pool, &sql, &ids).await;

    let count = |table: &str| format!("WITH {} SELECT count(*) FROM {table}", sql.ctes());
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("BEGIN READ ONLY")
        .execute(&mut *conn)
        .await
        .unwrap();
    let events: i64 = sqlx::query_scalar(&count(export::EXCLUDED_EVENT))
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    let versions: i64 = sqlx::query_scalar(&count(export::EXCLUDED_VERSION))
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    // 読み取り専用が本当に効いている（一時表を作る実装はここで落ちる）
    let probe = sqlx::query("CREATE TEMP TABLE ch02_probe (a int)")
        .execute(&mut *conn)
        .await;
    sqlx::query("ROLLBACK").execute(&mut *conn).await.unwrap();
    std::mem::drop(conn);
    drop.await;

    assert_eq!(events as usize, expected.excluded_events.len());
    assert_eq!(versions as usize, expected.excluded_versions.len());
    assert!(events > 0 && versions > 0);
    assert!(probe.is_err(), "READ ONLY になっていない");
}

// ---- Task 4: 読んだ行の数が線形に近いことと、窓の枠（design D7）----

/// 仮決めの値（design D7。`docs/testing.md` §3）。4N の量が N の量の `SCALE_LIMIT` 倍未満であることを見る。
const SCALE_N: usize = 1_000;
const SCALE_FACTOR: usize = 4;
const SCALE_LIMIT: u64 = 8;

/// 計画の JSON（`EXPLAIN (ANALYZE, FORMAT JSON)`）の `Relation Name` が `event` / `event_version` の節が読んだ行の数の和。
/// 節ごとに `(Actual Rows + Rows Removed by Filter + Rows Removed by Index Recheck) × Actual Loops`。
/// `skip_cte` に名前を渡すと、その CTE（`Subplan Name` が `CTE <名前>`）の下の節を数えない。
fn rows_read(plan: &Value, skip_cte: Option<&str>) -> u64 {
    let node = match plan {
        Value::Array(items) => {
            return items.iter().map(|p| rows_read(p, skip_cte)).sum();
        }
        Value::Object(o) => match o.get("Plan") {
            Some(root) => return rows_read(root, skip_cte),
            None => o,
        },
        _ => return 0,
    };
    if let Some(name) = skip_cte {
        if node.get("Subplan Name").and_then(Value::as_str) == Some(&format!("CTE {name}")) {
            return 0;
        }
    }
    let count = |key: &str| node.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    let own = match node.get("Relation Name").and_then(Value::as_str) {
        Some("event" | "event_version") => {
            ((count("Actual Rows")
                + count("Rows Removed by Filter")
                + count("Rows Removed by Index Recheck"))
                * count("Actual Loops")) as u64
        }
        _ => 0,
    };
    let children: u64 = node
        .get("Plans")
        .and_then(Value::as_array)
        .map(|ps| ps.iter().map(|p| rows_read(p, skip_cte)).sum())
        .unwrap_or(0);
    own + children
}

/// `rows_read` と同じ範囲の節（`event` / `event_version`）の `Actual Loops` の和。1 度も実行されない節は 0。
fn loops_run(plan: &Value, skip_cte: Option<&str>) -> u64 {
    let node = match plan {
        Value::Array(items) => return items.iter().map(|p| loops_run(p, skip_cte)).sum(),
        Value::Object(o) => match o.get("Plan") {
            Some(root) => return loops_run(root, skip_cte),
            None => o,
        },
        _ => return 0,
    };
    if let Some(name) = skip_cte {
        if node.get("Subplan Name").and_then(Value::as_str) == Some(&format!("CTE {name}")) {
            return 0;
        }
    }
    let own = match node.get("Relation Name").and_then(Value::as_str) {
        Some("event" | "event_version") => node
            .get("Actual Loops")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as u64,
        _ => 0,
    };
    let children: u64 = node
        .get("Plans")
        .and_then(Value::as_array)
        .map(|ps| ps.iter().map(|p| loops_run(p, skip_cte)).sum())
        .unwrap_or(0);
    own + children
}

#[test]
fn ch02_scale_loops_run_sums_the_event_scan_loops() {
    let plan = json!([{"Plan": {
        "Node Type": "Result", "Actual Loops": 1,
        "Plans": [
            {"Node Type": "Seq Scan", "Relation Name": "event", "Actual Loops": 2},
            {"Node Type": "Index Scan", "Relation Name": "event_version", "Actual Loops": 0},
            {"Node Type": "Seq Scan", "Relation Name": "stay_note", "Actual Loops": 9},
            {"Node Type": "Aggregate", "Subplan Name": "CTE erased_spans", "Actual Loops": 1,
             "Plans": [{"Node Type": "Seq Scan", "Relation Name": "event", "Actual Loops": 5}]}
        ]
    }}]);
    assert_eq!(loops_run(&plan, None), 7);
    assert_eq!(loops_run(&plan, Some("erased_spans")), 2);
}

#[test]
fn ch02_scale_rows_read_sums_the_event_scans() {
    let plan = json!([{"Plan": {
        "Node Type": "Result", "Actual Rows": 1, "Actual Loops": 1,
        "Plans": [
            {"Node Type": "Seq Scan", "Relation Name": "event", "Actual Rows": 10,
             "Rows Removed by Filter": 5, "Actual Loops": 2},
            {"Node Type": "Bitmap Heap Scan", "Relation Name": "event_version", "Actual Rows": 3,
             "Rows Removed by Index Recheck": 1, "Actual Loops": 1,
             "Plans": [
                {"Node Type": "Bitmap Index Scan", "Index Name": "event_version_pkey",
                 "Actual Rows": 100, "Actual Loops": 1}
             ]},
            {"Node Type": "Seq Scan", "Relation Name": "stay_note", "Actual Rows": 999, "Actual Loops": 1},
            {"Node Type": "Seq Scan", "Relation Name": "event", "Actual Rows": 0, "Actual Loops": 0},
            {"Node Type": "Aggregate", "Subplan Name": "CTE erased_spans", "Actual Rows": 1, "Actual Loops": 1,
             "Plans": [
                {"Node Type": "Seq Scan", "Relation Name": "event", "Actual Rows": 7, "Actual Loops": 1}
             ]}
        ]
    }}]);
    // (10 + 5) × 2 + (3 + 1) × 1 + 0 + 7 = 41 / CTE の下を除くと 34
    assert_eq!(rows_read(&plan, None), 41);
    assert_eq!(rows_read(&plan, Some("erased_spans")), 34);
}

/// 記録を `count` 件、`user` に置く。端末は記録ごとに違う値、時刻は `from` から 1 秒ずつ（`span` 秒で巻き戻す）、原文に地図の座標。
async fn bulk_windows(pool: &sqlx::PgPool, user: Uuid, count: usize, from: &str, span: i64) {
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         SELECT gen_random_uuid(), $1, 'c02-window', 'dev-' || i, 'collected',
                $3::timestamptz + (i % $4) * interval '1 second',
                540, 'Asia/Tokyo', 1, gen_random_uuid()::text, $5, '{}'::jsonb
           FROM generate_series(1, $2::int) i",
    )
    .bind(user)
    .bind(count as i64)
    .bind(from)
    .bind(span)
    .bind(MAP)
    .execute(pool)
    .await
    .unwrap();
}

/// 除く範囲の判定の `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)`。
async fn explain_excluded(pool: &sqlx::PgPool, sql: &ExcludedSql) -> Value {
    let q = format!(
        "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
         WITH {} SELECT (SELECT count(*) FROM {}), (SELECT count(*) FROM {})",
        sql.ctes(),
        export::EXCLUDED_EVENT,
        export::EXCLUDED_VERSION
    );
    sqlx::query_scalar(&q).fetch_one(pool).await.unwrap()
}

/// 消した滞在 `erased`（`(始まり, 終わり)`。`None` なら置かない）と、利用者 A の 00:00〜00:59 の記録 `n` 件、
/// 別の利用者の 03:00 以降の記録 `n` 件を置いた DB で、`(新しい断片, 写しの断片)` の読んだ行の数を返す。
/// `legacy_too` が偽なら写しは取らない（0）。
async fn scale_reads(n: usize, erased: Option<(&str, &str)>, legacy_too: bool) -> (u64, u64) {
    let (pool, drop) = fresh().await;
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    if let Some((start, end)) = erased {
        erased_stay(&pool, a, start, end, &[]).await;
    }
    bulk_windows(&pool, a, n, &ts("00:00:00"), 3540).await;
    bulk_windows(&pool, b, n, &ts("03:00:00"), 3540).await;
    sqlx::query("ANALYZE").execute(&pool).await.unwrap();
    let new = rows_read(
        &explain_excluded(&pool, &export::excluded_sql("now()")).await,
        None,
    );
    let old = if legacy_too {
        rows_read(
            &explain_excluded(&pool, &legacy::excluded_sql("now()")).await,
            None,
        )
    } else {
        0
    };
    drop.await;
    (new, old)
}

#[tokio::test]
async fn ch02_scale_four_times_the_rows_reads_less_than_eight_times() {
    // Scenario: 記録を 4 倍にしても、除く範囲の判定が読む量は 4 倍の桁に収まる
    let erased = Some((ts("01:00:00"), ts("02:00:00")));
    let erased = erased.as_ref().map(|(s, e)| (s.as_str(), e.as_str()));
    let (new_n, old_n) = scale_reads(SCALE_N, erased, true).await;
    let (new_4n, old_4n) = scale_reads(SCALE_N * SCALE_FACTOR, erased, true).await;
    eprintln!("近い: N={new_n} 4N={new_4n} / 写し: N={old_n} 4N={old_4n}");
    assert!(new_n > 0 && old_n > 0, "読んだ行の数が取れていない");
    assert!(
        new_4n < SCALE_LIMIT * new_n,
        "新しい断片が 4 倍で {SCALE_LIMIT} 倍以上: {new_n} → {new_4n}"
    );
    assert!(
        old_4n >= SCALE_LIMIT * old_n,
        "写しが 4 倍で {SCALE_LIMIT} 倍未満（配置が効いていない）: {old_n} → {old_4n}"
    );

    // 消した滞在が古い配置（行の大半が `max(end_at)` より後）。組ごとの最初の 1 行の GROUP BY も線形
    let old_spot = Some(("1999-12-31T22:00:00Z", "1999-12-31T23:00:00Z"));
    let (late_n, _) = scale_reads(SCALE_N, old_spot, false).await;
    let (late_4n, _) = scale_reads(SCALE_N * SCALE_FACTOR, old_spot, false).await;
    eprintln!("古い配置: N={late_n} 4N={late_4n}");
    assert!(late_n > 0);
    assert!(
        late_4n < SCALE_LIMIT * late_n,
        "消した滞在が古い配置で 4 倍が {SCALE_LIMIT} 倍以上: {late_n} → {late_4n}"
    );
}

#[tokio::test]
async fn ch02_scale_nothing_erased_reads_no_records() {
    // Scenario: 消した滞在が無ければ、除く範囲の判定は記録の表を読まない
    let (new, old) = {
        let (pool, drop) = fresh().await;
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        bulk_windows(&pool, a, SCALE_N, &ts("00:00:00"), 3540).await;
        bulk_windows(&pool, b, SCALE_N, &ts("03:00:00"), 3540).await;
        // 前の版も置く（写しは前の版の表を全走査する。記録の表は索引条件が NULL なら読まないことがあるので、前の版で差を見る）
        sqlx::query(
            "INSERT INTO core.event_version
               (id, event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
             SELECT gen_random_uuid(), e.id, e.user_id, e.logical_source, 1, e.event_time,
                    gen_random_uuid()::text, '{}', '{}'::jsonb
               FROM core.event e",
        )
        .execute(&pool)
        .await
        .unwrap();
        let new = explain_excluded(&pool, &export::excluded_sql("now()")).await;
        let old = explain_excluded(&pool, &legacy::excluded_sql("now()")).await;
        drop.await;
        (
            (
                rows_read(&new, Some("erased_spans")),
                loops_run(&new, Some("erased_spans")),
            ),
            rows_read(&old, Some("erased_spans")),
        )
    };
    let (new, new_loops) = new;
    eprintln!("消した滞在 0 件: 新 {new}（実行 {new_loops} 回）/ 写し {old}");
    assert_eq!(new, 0, "消した滞在が無いのに記録の表を読んでいる");
    assert_eq!(
        new_loops, 0,
        "記録の表の節が実行されている（Actual Loops 0 でない）"
    );
    assert!(
        old > 0,
        "写しは表を読むはず（門がいまより強い保証であることの固定）"
    );
}

#[test]
fn ch02_scale_window_frame_is_head_fixed() {
    let ctes = export::excluded_sql("now()").ctes().to_string();
    let squeeze = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let ctes = squeeze(&ctes);
    assert!(ctes.contains("ORDER BY q.event_time DESC RANGE BETWEEN UNBOUNDED PRECEDING AND '1 microsecond' PRECEDING"));
    assert!(!ctes.contains("FOLLOWING AND UNBOUNDED FOLLOWING"));
}
