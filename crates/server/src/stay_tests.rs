// SPDX-License-Identifier: AGPL-3.0-only
//! 滞在の移行・作り直し・読み出しを、**本物の PostgreSQL に対して**確かめる（ST16）。
//!
//! **利用者で隔離する**（design D10 / R13）。基準の台帳は追記のみで消せないので、
//! テストは `testdb::user()` で利用者を毎回新しく作り、その利用者の基準だけを変える。
//! 既定の利用者（`00000000-…`）の基準には触らない。
#![allow(clippy::unwrap_used)]

use crate::testdb;

// ------------------------------------------------------------------ 1. 移行

/// 全版を**まっさらな DB に 2 回**当てて落ちない（tasks 1.1）。
///
/// 共有の開発 DB で当て直すと、並んで走る他のテストの挿入と `ALTER TABLE` の錠が
/// deadlock する（`202609112113_source_lifecycle.sql` の REPAIR の注記と同じ実測）。
/// 使い捨ての DB を作って当てる。
#[tokio::test]
async fn stays_migration_applies_twice() {
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
        let (n,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM core.source WHERE logical_source = 's01-stay'")
                .fetch_one(&fresh)
                .await
                .map_err(|e| e.to_string())?;
        Ok::<i64, String>(n)
    }
    .await;
    drop.await;

    assert_eq!(applied.unwrap(), 1, "2 回当てると s01-stay が二重になる");
    // 末尾ではなく「含まれている」を見る —— 後続の Story（ST04 の `_drop_reports`）が末尾に足す
    assert!(
        crate::MIGRATIONS.iter().any(|(n, _)| n.ends_with("_stays")),
        "滞在の移行が MIGRATIONS に無い"
    );
}

/// 基準の台帳と吸収の台帳は**追記のみ**（tasks 1.2 / design D10）。
#[tokio::test]
async fn stay_ledgers_are_append_only() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let (cid,): (i64,) = sqlx::query_as(
        "INSERT INTO core.stay_criteria (user_id, radius_m, min_minutes, gap_minutes, sources)
         VALUES ($1, 100, 10, 10, '{c01-location}') RETURNING id",
    )
    .bind(u)
    .fetch_one(&pool)
    .await
    .unwrap();

    // 吸収の台帳が指す滞在を 1 行置く
    let stay = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,'s01-stay',$3,'derived','2026-08-01T00:00:00Z',540,'Asia/Tokyo',1,$3,'{}','{}')",
    )
    .bind(stay)
    .bind(u)
    .bind(stay.to_string())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.stay_absorbed (event_id, user_id, into_event_id, criteria_id)
         VALUES ($1, $2, NULL, $3)",
    )
    .bind(stay)
    .bind(u)
    .bind(cid)
    .execute(&pool)
    .await
    .unwrap();

    for (what, sql) in [
        (
            "基準の書き換え",
            "UPDATE core.stay_criteria SET radius_m = 50 WHERE user_id = $1",
        ),
        (
            "基準の削除",
            "DELETE FROM core.stay_criteria WHERE user_id = $1",
        ),
        (
            "吸収の書き換え",
            "UPDATE core.stay_absorbed SET into_event_id = event_id WHERE user_id = $1",
        ),
        (
            "吸収の削除",
            "DELETE FROM core.stay_absorbed WHERE user_id = $1",
        ),
    ] {
        let got = sqlx::query(sql).bind(u).execute(&pool).await;
        assert!(got.is_err(), "{what} が通っている");
    }
    for table in ["core.stay_criteria", "core.stay_absorbed"] {
        // **切り詰めは試すだけで、成功したら取り返しがつかない** —— トランザクションの中で撃って巻き戻す
        let mut tx = pool.begin().await.unwrap();
        let got = sqlx::raw_sql(&format!("TRUNCATE {table} CASCADE"))
            .execute(&mut *tx)
            .await;
        tx.rollback().await.unwrap();
        assert!(got.is_err(), "{table} の切り詰めが通っている");
    }
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM core.stay_criteria WHERE user_id = $1")
        .bind(u)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
}

/// `s01-stay` が登録簿にあり、**「記録ごと」**と宣言されている（tasks 1.3 / design D2 / D3）。
///
/// `'none'` だと滞在の `external_id` が `external_ref` へ回され、`event_dedup_hash` に当たる
/// （同じ内容の滞在を 2 回入れると一意違反。deep-review R1 の実測）。
#[tokio::test]
async fn stay_source_is_registered() {
    let pool = testdb::pool().await;
    let (kind,): (String,) =
        sqlx::query_as("SELECT external_id_kind FROM core.source WHERE logical_source = $1")
            .bind(crate::stay::SOURCE)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(crate::stay::SOURCE, "s01-stay");
    assert_eq!(kind, "record");
}

// ------------------------------------------------------------------ 足場

use crate::stay::fixture::offset;
use crate::stay_store;
use chrono::{DateTime, Duration, Utc};

fn t(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

/// `at`（RFC3339）から 60 秒ごとに `minutes + 1` 件、基準点から北へ `north_m` の位置を置く。
/// 位置の記録は**取り込み口を通さず**に置く（作り直しの規則だけを見るため）。`acc` は水平精度。
async fn put_dwell(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    at: &str,
    minutes: i64,
    north_m: f64,
    acc: Option<f64>,
) {
    let t0 = t(at);
    let (lat, lon) = offset(north_m, 0.0);
    let mut times = Vec::new();
    let mut raws = Vec::new();
    for i in 0..=minutes {
        times.push(t0 + Duration::minutes(i));
        raws.push(match acc {
            Some(a) => format!(r#"{{"lat":{lat},"lon":{lon},"acc_m":{a}}}"#),
            None => format!(r#"{{"lat":{lat},"lon":{lon}}}"#),
        });
    }
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         SELECT gen_random_uuid(), $1, 'c01-location', 'test', 'collected', t,
                540, 'Asia/Tokyo', 1, gen_random_uuid()::text, r, r::jsonb
           FROM unnest($2::timestamptz[], $3::text[]) AS u(t, r)",
    )
    .bind(user)
    .bind(&times)
    .bind(&raws)
    .execute(pool)
    .await
    .unwrap();
}

/// 滞在の行（削除済みを含む）。
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct StayRow {
    id: uuid::Uuid,
    event_time: DateTime<Utc>,
    raw: String,
    payload: serde_json::Value,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<String>,
}

impl StayRow {
    fn span(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        stay_store::span_of(self.event_time, &self.payload)
    }
}

async fn stays(pool: &sqlx::PgPool, user: uuid::Uuid) -> Vec<StayRow> {
    sqlx::query_as(
        "SELECT id, event_time, raw, payload, deleted_at, deleted_by FROM core.event
          WHERE user_id = $1 AND logical_source = 's01-stay' AND origin = 'derived'
          ORDER BY event_time, id",
    )
    .bind(user)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// 読み出しに出ている滞在の (識別子, 始まり, 終わり)。**`core.event_live` 越しに引く。**
async fn live(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
) -> Vec<(uuid::Uuid, DateTime<Utc>, DateTime<Utc>)> {
    let rows: Vec<StayRow> = sqlx::query_as(
        "SELECT id, event_time, raw, payload, deleted_at, deleted_by FROM core.event_live
          WHERE user_id = $1 AND logical_source = 's01-stay' AND origin = 'derived'
          ORDER BY event_time, id",
    )
    .bind(user)
    .fetch_all(pool)
    .await
    .unwrap();
    rows.iter()
        .map(|r| {
            let (s, e) = r.span();
            (r.id, s, e)
        })
        .collect()
}

async fn rebuild(pool: &sqlx::PgPool, user: uuid::Uuid, day: &str) {
    stay_store::rebuild_day(pool, user, testdb::date(day))
        .await
        .unwrap();
}

async fn set(pool: &sqlx::PgPool, user: uuid::Uuid, radius: Option<i32>, gap: Option<i32>) {
    stay_store::set_criteria(pool, user, radius, None, gap)
        .await
        .unwrap();
}

async fn versions(pool: &sqlx::PgPool, id: uuid::Uuid) -> Vec<String> {
    sqlx::query_as::<_, (String,)>(
        "SELECT raw FROM core.event_version_live WHERE event_id = $1 ORDER BY version_no",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|(r,)| r)
    .collect()
}

async fn version_count(pool: &sqlx::PgPool, user: uuid::Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM core.event_version WHERE user_id = $1 AND logical_source = 's01-stay'",
    )
    .bind(user)
    .fetch_one(pool)
    .await
    .unwrap();
    n
}

/// 本人が消す（消す操作は ST22。ここでは行を直に書き換える。tasks 3.4）。
async fn user_deletes(pool: &sqlx::PgPool, id: uuid::Uuid, by: Option<&str>) {
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = $2 WHERE id = $1")
        .bind(id)
        .bind(by)
        .execute(pool)
        .await
        .unwrap();
}

/// 始まりが `at` の滞在の識別子（削除済みを含む）。
fn id_starting(rows: &[StayRow], at: &str) -> uuid::Uuid {
    rows.iter()
        .find(|r| r.span().0 == t(at))
        .unwrap_or_else(|| panic!("{at} に始まる滞在が無い: {rows:#?}"))
        .id
}

// ------------------------------------------------------------------ 3.1 行の形

// Scenario: 作った滞在は派生させたに分類される
// Scenario: 滞在に作った基準と使った件数が載る
// Scenario: 作った滞在の感度は外部 AI に出してよい
#[tokio::test]
async fn stay_row_shape_is_derived_with_criteria() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-03T09:00:00+09:00", 20, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-03").await;

    #[derive(sqlx::FromRow)]
    struct Row {
        id: uuid::Uuid,
        external_id: Option<String>,
        device_id: Option<String>,
        origin: String,
        sensitivity: i16,
        event_time: DateTime<Utc>,
        content_hash: String,
        raw: String,
        tz_id: String,
        schema_version: i32,
    }
    let Row {
        id,
        external_id,
        device_id,
        origin,
        sensitivity,
        event_time,
        content_hash: hash,
        raw,
        tz_id,
        schema_version,
    } = sqlx::query_as(
        "SELECT id, external_id, device_id, origin, sensitivity, event_time, content_hash, raw, tz_id, schema_version
           FROM core.event WHERE user_id = $1 AND logical_source = 's01-stay'",
    )
    .bind(u)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(origin, "derived", "「派生させた」に分類されていない");
    // **感度には何も書かない**（D9）。1 = 外部 AI に出してよい（PERM-2）
    assert_eq!(
        sensitivity, 1,
        "位置由来だからと厳しい側に倒している（Q3 は本人が緩い側を選んだ）"
    );
    assert_eq!(
        external_id.as_deref(),
        Some(id.to_string().as_str()),
        "識別子を external_id に置いていない（D3）"
    );
    assert_eq!(device_id, None);
    assert_eq!(
        event_time,
        t("2026-08-03T09:00:00+09:00"),
        "出来事の時刻が滞在の始まりでない（C7）"
    );
    assert_eq!(
        hash,
        crate::ingest::content_hash_of("s01-stay", event_time, &raw)
    );
    assert_eq!((tz_id.as_str(), schema_version), ("Asia/Tokyo", 1));

    let (cid,): (i64,) = sqlx::query_as("SELECT id FROM core.stay_criteria WHERE user_id = $1")
        .bind(u)
        .fetch_one(&pool)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        v["criteria"],
        serde_json::json!({"id": cid, "radius_m": 100, "min_minutes": 10, "gap_minutes": 10, "sources": ["c01-location"]})
    );
    assert_eq!(v["points_used"], 21, "判定に使った件数");
    assert_eq!(v["start"], "2026-08-03T00:00:00Z");
    assert_eq!(v["end"], "2026-08-03T00:20:00Z");
}

// Scenario: 基準は利用者ごとに分かれる
#[tokio::test]
async fn stay_row_shape_criteria_per_user() {
    let pool = testdb::pool().await;
    let (a, b) = (testdb::user(), testdb::user());
    stay_store::ensure_criteria_committed(&pool, b)
        .await
        .unwrap();
    set(&pool, a, Some(50), None).await;
    let now_b = stay_store::current_criteria(&pool, b)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(now_b.radius_m, 100, "利用者 A の変更が B に効いている");
    let now_a = stay_store::current_criteria(&pool, a)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(now_a.radius_m, 50);
    // 既定の利用者の基準には触っていない（テストの規律）
    assert_ne!(a, uuid::Uuid::nil());
}

// Scenario: 判定に使わなかった位置の記録は残る
#[tokio::test]
async fn stay_row_shape_keeps_unused_points() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-04T09:00:00+09:00", 20, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-04T09:30:30+09:00", 0, 0.0, Some(150.0)).await;
    let before: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, raw FROM core.event WHERE user_id = $1 AND logical_source = 'c01-location' ORDER BY id",
    )
    .bind(u)
    .fetch_all(&pool)
    .await
    .unwrap();
    rebuild(&pool, u, "2026-08-04").await;
    let after: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, raw FROM core.event_live WHERE user_id = $1 AND logical_source = 'c01-location' ORDER BY id",
    )
    .bind(u)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(before.len(), 22);
    assert_eq!(before, after, "判定に使わなかった位置の記録が変わった");
    assert!(after.iter().any(|(_, r)| r.contains("150")));
}

// ------------------------------------------------------------------ 3.2 識別子の引き継ぎ

// Scenario: 区切りが伸びても識別子は変わらない
// Scenario: 区切りが伸びると前の版が残る
#[tokio::test]
async fn stay_identity_extends() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-05T09:00:00+09:00", 20, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-05").await;
    let first = live(&pool, u).await;
    assert_eq!(first.len(), 1);

    put_dwell(&pool, u, "2026-08-05T09:21:00+09:00", 39, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-05").await;
    let now = live(&pool, u).await;
    assert_eq!(
        now,
        vec![(
            first[0].0,
            t("2026-08-05T09:00:00+09:00"),
            t("2026-08-05T10:00:00+09:00")
        )]
    );
    assert_eq!(stays(&pool, u).await.len(), 1, "行が増えている");

    let old = versions(&pool, first[0].0).await;
    assert_eq!(old.len(), 1, "前の版が残っていない");
    let v: serde_json::Value = serde_json::from_str(&old[0]).unwrap();
    assert_eq!(
        v["end"], "2026-08-05T00:20:00Z",
        "前の版が 09:00〜09:20 でない"
    );
}

// Scenario: 割れた滞在は重なりのいちばん大きい 1 件が識別子を継ぐ
#[tokio::test]
async fn stay_identity_split_keeps_largest() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 同じ地点に 09:00〜09:30 / 09:40〜11:30 / 11:40〜12:00。欠けは 10 分ずつ
    put_dwell(&pool, u, "2026-08-06T09:00:00+09:00", 30, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-06T09:40:00+09:00", 110, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-06T11:40:00+09:00", 20, 0.0, Some(10.0)).await;
    set(&pool, u, None, Some(15)).await;
    rebuild(&pool, u, "2026-08-06").await;
    let one = live(&pool, u).await;
    assert_eq!(one.len(), 1, "記録なしとみなす間隔 15 分では 1 件のはず");
    let original = one[0].0;

    set(&pool, u, None, Some(10)).await;
    rebuild(&pool, u, "2026-08-06").await;
    let three = live(&pool, u).await;
    assert_eq!(
        three.iter().map(|(_, s, e)| (*s, *e)).collect::<Vec<_>>(),
        vec![
            (
                t("2026-08-06T09:00:00+09:00"),
                t("2026-08-06T09:30:00+09:00")
            ),
            (
                t("2026-08-06T09:40:00+09:00"),
                t("2026-08-06T11:30:00+09:00")
            ),
            (
                t("2026-08-06T11:40:00+09:00"),
                t("2026-08-06T12:00:00+09:00")
            ),
        ]
    );
    assert_eq!(
        three[1].0, original,
        "重なり最大の 09:40〜11:30 が元の識別子を継いでいない（Q10）"
    );
    assert_ne!(three[0].0, original);
    assert_ne!(three[2].0, original);
    assert_ne!(three[0].0, three[2].0);
}

// Scenario: 離れた滞在には識別子を継がない
#[tokio::test]
async fn stay_identity_far_does_not_inherit() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-07T09:00:00+09:00", 60, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-07").await;
    let before = live(&pool, u).await;
    assert_eq!(before.len(), 1);

    // 同じ時間帯の代表点を 250 m 先へ動かす。**位置の記録は書き換えられない**（FR-30）ので、
    // 地点 A の記録を論理削除して地点 B の記録を置き、基準も変える（半径 100 → 120 m。2 倍 = 240 m < 250 m）
    sqlx::query(
        "UPDATE core.event SET deleted_at = now(), deleted_by = 'test'
          WHERE user_id = $1 AND logical_source = 'c01-location'",
    )
    .bind(u)
    .execute(&pool)
    .await
    .unwrap();
    put_dwell(&pool, u, "2026-08-07T09:00:00+09:00", 60, 250.0, Some(10.0)).await;
    set(&pool, u, Some(120), None).await;
    rebuild(&pool, u, "2026-08-07").await;

    let after = live(&pool, u).await;
    assert_eq!(after.len(), 1);
    assert_ne!(
        after[0].0, before[0].0,
        "250 m 離れた滞在が識別子を継いでいる（Q11）"
    );
    let old = stays(&pool, u)
        .await
        .into_iter()
        .find(|r| r.id == before[0].0)
        .unwrap();
    assert_eq!(
        old.deleted_by.as_deref(),
        Some("rebuild:absorbed"),
        "元の滞在が吸収されていない"
    );
}

/// 距離の境目: 2 倍ちょうどまでは継ぐ（`>` で捨てる）。
#[tokio::test]
async fn stay_identity_near_inherits() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-08T09:00:00+09:00", 60, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-08").await;
    let before = live(&pool, u).await;
    sqlx::query(
        "UPDATE core.event SET deleted_at = now(), deleted_by = 'test'
          WHERE user_id = $1 AND logical_source = 'c01-location'",
    )
    .bind(u)
    .execute(&pool)
    .await
    .unwrap();
    put_dwell(&pool, u, "2026-08-08T09:00:00+09:00", 60, 230.0, Some(10.0)).await;
    set(&pool, u, Some(120), None).await;
    rebuild(&pool, u, "2026-08-08").await;
    assert_eq!(
        live(&pool, u).await[0].0,
        before[0].0,
        "230 m（2 倍の 240 m 以内）で継いでいない"
    );
}

/// 既存の滞在を直に置く（取り込み口や前の作り直しで入った状態を作る）。
async fn put_stay_row(pool: &sqlx::PgPool, user: uuid::Uuid, start: &str, end: &str) -> uuid::Uuid {
    put_stay_row_with(pool, user, uuid::Uuid::new_v4(), start, end, None).await
}

/// 識別子と基準（`(id, 半径, 最短の分)`）を指定して既存の滞在を置く。
async fn put_stay_row_with(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    id: uuid::Uuid,
    start: &str,
    end: &str,
    criteria: Option<(i64, i32, i32)>,
) -> uuid::Uuid {
    let (lat, lon) = offset(0.0, 0.0);
    let tag = criteria.map_or(String::new(), |(cid, r, m)| {
        format!(r#","criteria":{{"id":{cid},"radius_m":{r},"min_minutes":{m},"gap_minutes":10,"sources":["c01-location"]}}"#)
    });
    let raw = format!(
        r#"{{"start":"{}","end":"{}","lat":{lat},"lon":{lon}{tag}}}"#,
        t(start).to_rfc3339(),
        t(end).to_rfc3339()
    );
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,'s01-stay',$3,'derived',$4,540,'Asia/Tokyo',1,$3,$5,$5::jsonb)",
    )
    .bind(id)
    .bind(user)
    .bind(id.to_string())
    .bind(t(start))
    .bind(&raw)
    .execute(pool)
    .await
    .unwrap();
    id
}

// Scenario: 重なりが同じなら始まりの早い既存の滞在から割り当てる
#[tokio::test]
async fn stay_identity_tie_goes_to_earlier() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // **識別子を固定する**（R27）。遅い方の識別子を辞書順で先に置き、始まりで同点を解く処理を消すと必ず落ちるようにする
    let late = put_stay_row_with(
        &pool,
        u,
        uuid::Uuid::from_u128(u.as_u128() & 0x0000_0000_0000_0000_0000_ffff_ffff_ffff),
        "2026-08-09T09:30:00+09:00",
        "2026-08-09T10:00:00+09:00",
        None,
    )
    .await;
    let early = put_stay_row_with(
        &pool,
        u,
        uuid::Uuid::from_u128(u128::MAX - (u.as_u128() & 0xffff_ffff_ffff)),
        "2026-08-09T09:00:00+09:00",
        "2026-08-09T09:30:00+09:00",
        None,
    )
    .await;
    assert!(late < early, "識別子の順が前提と違う");
    put_dwell(&pool, u, "2026-08-09T09:15:00+09:00", 30, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-09").await;
    let now = live(&pool, u).await;
    assert_eq!(now.len(), 1);
    assert_eq!(
        (now[0].1, now[0].2),
        (
            t("2026-08-09T09:15:00+09:00"),
            t("2026-08-09T09:45:00+09:00")
        )
    );
    assert_eq!(
        now[0].0, early,
        "始まりの早い 09:00〜09:30 の識別子を持っていない"
    );
    assert_ne!(now[0].0, late);
}

// Scenario: 吸収された滞在は読み出しから外れる
// Scenario: 吸収された滞在の行と吸収先が残る
// Scenario: 基準を戻すと吸収された滞在が同じ識別子で戻る
#[tokio::test]
async fn stay_identity_absorb_and_restore() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-10T09:00:00+09:00", 30, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-10T09:35:00+09:00", 55, 0.0, Some(10.0)).await;
    set(&pool, u, None, Some(5)).await;
    rebuild(&pool, u, "2026-08-10").await;
    let two = live(&pool, u).await;
    assert_eq!(two.len(), 2, "欠け 5 分を記録なしとみなすと 2 件");
    let (a, b) = (two[0].0, two[1].0);

    set(&pool, u, None, Some(10)).await;
    rebuild(&pool, u, "2026-08-10").await;
    let one = live(&pool, u).await;
    assert_eq!(
        one,
        vec![(
            b,
            t("2026-08-10T09:00:00+09:00"),
            t("2026-08-10T10:30:00+09:00")
        )],
        "09:00〜10:30 が重なりの大きい 09:35〜10:30 の識別子を持っていない"
    );
    // 吸収された行は消えずに残り、吸収先が引ける
    let rows = stays(&pool, u).await;
    let absorbed = rows
        .iter()
        .find(|r| r.id == a)
        .expect("吸収された滞在の行が消えている");
    assert_eq!(absorbed.deleted_by.as_deref(), Some("rebuild:absorbed"));
    let (into,): (Option<uuid::Uuid>,) =
        sqlx::query_as("SELECT into_event_id FROM core.stay_absorbed WHERE event_id = $1")
            .bind(a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(into, Some(b), "吸収先が 09:00〜10:30 の滞在でない");

    set(&pool, u, None, Some(5)).await;
    rebuild(&pool, u, "2026-08-10").await;
    let back = live(&pool, u).await;
    assert_eq!(back.len(), 2);
    assert_eq!(
        back[0],
        (
            a,
            t("2026-08-10T09:00:00+09:00"),
            t("2026-08-10T09:30:00+09:00")
        ),
        "09:00〜09:30 が統合される前の識別子で戻っていない"
    );
    assert_eq!(back[1].0, b);
}

// Scenario: 取り込みの口から送った滞在は作り直しで置き換わる
#[tokio::test]
async fn stay_identity_replaces_ingested_stay() {
    use axum::{extract::State, Json};
    let app = app().await;
    let u = testdb::user();
    let (lat, lon) = offset(0.0, 0.0);
    let item = serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": u, "logical_source": "s01-stay",
        "external_id": "sent-by-someone", "device_id": null, "origin": "derived",
        "event_time": "2026-08-11T01:00:00Z", "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": format!(r#"{{"start":"2026-08-11T01:00:00Z","end":"2026-08-11T09:00:00Z","lat":{lat},"lon":{lon}}}"#),
        "payload": {"start": "2026-08-11T01:00:00Z", "end": "2026-08-11T09:00:00Z", "lat": lat, "lon": lon},
    });
    let (_, Json(res)) = crate::ingest(State(app.clone()), auth(), Json(serde_json::json!([item])))
        .await
        .unwrap();
    assert!(
        serde_json::to_value(&res).unwrap()[0]["accepted"]
            .as_bool()
            .unwrap(),
        "取り込みの口が滞在用のソースを断っている（D2）"
    );
    assert_eq!(live(&app.pool, u).await.len(), 1);

    put_dwell(
        &app.pool,
        u,
        "2026-08-11T10:00:00+09:00",
        40,
        0.0,
        Some(10.0),
    )
    .await;
    stay_store::rebuild_all(&app.pool, u).await.unwrap();
    let now = live(&app.pool, u).await;
    assert_eq!(
        now.iter().map(|(_, s, e)| (*s, *e)).collect::<Vec<_>>(),
        vec![(
            t("2026-08-11T10:00:00+09:00"),
            t("2026-08-11T10:40:00+09:00")
        )],
        "取り込みの口から送った 10:00〜18:00 が残っている"
    );
}

// ------------------------------------------------------------------ 3.3 冪等

// Scenario: 基準を添えずに作り直しても滞在は増えも消えもしない
// Scenario: 基準を添えずに作り直しても前の版は増えない
#[tokio::test]
async fn stay_rebuild_is_idempotent() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-12T08:00:00+09:00", 40, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-12T09:00:00+09:00", 30, 800.0, Some(10.0)).await;
    // 日付をまたぐ滞在も入れる（範囲を広げる経路を 2 回踏む）
    put_dwell(&pool, u, "2026-08-12T23:00:00+09:00", 120, 0.0, None).await;

    stay_store::rebuild_all(&pool, u).await.unwrap();
    let once = live(&pool, u).await;
    let rows_once = stays(&pool, u).await.len();
    let versions_once = version_count(&pool, u).await;
    assert_eq!(once.len(), 3);

    stay_store::rebuild_all(&pool, u).await.unwrap();
    assert_eq!(
        live(&pool, u).await,
        once,
        "2 回目で件数・識別子・始まりと終わりが変わった"
    );
    assert_eq!(stays(&pool, u).await.len(), rows_once, "2 回目で行が増えた");
    assert_eq!(
        version_count(&pool, u).await,
        versions_once,
        "内容の同じ作り直しで前の版が増えた"
    );
}

// ------------------------------------------------------------------ 3.4 本人が消した時間帯

// Scenario: 同じ基準で作り直しても消した滞在は戻らない
#[tokio::test]
async fn stay_erased_range_same_criteria() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-13T10:00:00+09:00", 60, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-13").await;
    let id = live(&pool, u).await[0].0;
    user_deletes(&pool, id, Some("user")).await;

    for _ in 0..2 {
        stay_store::rebuild_all(&pool, u).await.unwrap();
        assert!(
            live(&pool, u).await.is_empty(),
            "本人が消した 10:00〜11:00 に滞在が戻った"
        );
    }
    // 2 回目で行が増えない（隠した滞在を毎回足していない）
    assert_eq!(stays(&pool, u).await.len(), 2);
}

// Scenario: 本人が消した滞在の行は作り直しで変わらない
#[tokio::test]
async fn stay_erased_range_row_untouched() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-14T10:00:00+09:00", 30, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-14T10:31:00+09:00", 29, 70.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-14").await;
    let id = live(&pool, u).await[0].0;
    user_deletes(&pool, id, Some("user")).await;
    let before = stays(&pool, u)
        .await
        .into_iter()
        .find(|r| r.id == id)
        .unwrap();

    set(&pool, u, Some(50), None).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    let after = stays(&pool, u)
        .await
        .into_iter()
        .find(|r| r.id == id)
        .unwrap();
    assert_eq!(
        before, after,
        "本人が消した滞在の行（内容・削除の時刻・削除した者）が変わった"
    );
    assert!(live(&pool, u).await.is_empty(), "割れた断片が戻っている");
}

// Scenario: 基準を変えて割れても、消した時間帯の断片は戻らない
#[tokio::test]
async fn stay_erased_range_split_fragments_stay_hidden() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-15T10:00:00+09:00", 30, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-15T10:40:00+09:00", 40, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-15T11:30:00+09:00", 30, 0.0, Some(10.0)).await;
    set(&pool, u, None, Some(15)).await;
    rebuild(&pool, u, "2026-08-15").await;
    let one = live(&pool, u).await;
    assert_eq!(
        (one.len(), one[0].1, one[0].2),
        (
            1,
            t("2026-08-15T10:00:00+09:00"),
            t("2026-08-15T12:00:00+09:00")
        )
    );
    user_deletes(&pool, one[0].0, Some("user")).await;

    set(&pool, u, None, Some(10)).await;
    rebuild(&pool, u, "2026-08-15").await;
    let rows = stays(&pool, u).await;
    let fragments: Vec<_> = rows.iter().filter(|r| r.id != one[0].0).collect();
    assert_eq!(fragments.len(), 3, "3 件に割れていない: {rows:#?}");
    assert!(
        fragments
            .iter()
            .all(|r| r.deleted_by.as_deref() == Some("rebuild:erased-range")),
        "消した時間帯の断片が削除済みになっていない"
    );
    assert!(live(&pool, u).await.is_empty());
}

// Scenario: 消した範囲より長い滞在に統合されると丸ごと隠れる
// Scenario: 基準を戻して重ならなくなった断片は戻る
#[tokio::test]
async fn stay_erased_range_merge_hides_then_restores() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-16T08:00:00+09:00", 110, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-16T10:00:00+09:00", 60, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-16T11:10:00+09:00", 50, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-16").await;
    let rows = stays(&pool, u).await;
    assert_eq!(rows.len(), 3);
    user_deletes(
        &pool,
        id_starting(&rows, "2026-08-16T10:00:00+09:00"),
        Some("user"),
    )
    .await;

    set(&pool, u, None, Some(15)).await;
    rebuild(&pool, u, "2026-08-16").await;
    assert!(
        live(&pool, u).await.is_empty(),
        "08:00〜12:00 に統合された滞在が隠れていない（Q12）"
    );
    let merged: Vec<_> = stays(&pool, u)
        .await
        .into_iter()
        .filter(|r| {
            r.span()
                == (
                    t("2026-08-16T08:00:00+09:00"),
                    t("2026-08-16T12:00:00+09:00"),
                )
        })
        .collect();
    assert_eq!(merged.len(), 1);
    assert_eq!(
        merged[0].deleted_by.as_deref(),
        Some("rebuild:erased-range")
    );

    set(&pool, u, None, Some(10)).await;
    rebuild(&pool, u, "2026-08-16").await;
    let back = live(&pool, u).await;
    assert_eq!(
        back.iter().map(|(_, s, e)| (*s, *e)).collect::<Vec<_>>(),
        vec![
            (
                t("2026-08-16T08:00:00+09:00"),
                t("2026-08-16T09:50:00+09:00")
            ),
            (
                t("2026-08-16T11:10:00+09:00"),
                t("2026-08-16T12:00:00+09:00")
            ),
        ],
        "重ならなくなった断片が戻っていない、または消した時間帯の滞在が戻った"
    );
}

// Scenario: 削除した者の欄が空の削除も本人が消したものとして扱う
#[tokio::test]
async fn stay_erased_range_null_deleted_by() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-17T10:00:00+09:00", 60, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-17").await;
    let id = live(&pool, u).await[0].0;
    user_deletes(&pool, id, None).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    assert!(
        live(&pool, u).await.is_empty(),
        "削除した者が空の削除を作り直しの印と取り違えている（R14）"
    );
    let row = stays(&pool, u)
        .await
        .into_iter()
        .find(|r| r.id == id)
        .unwrap();
    assert_eq!(row.deleted_by, None, "本人の削除の行を書き換えている");
}

// Scenario: 作り直しで無くなった滞在は本人が消した時間帯にならない
#[tokio::test]
async fn stay_erased_range_absorbed_is_not_erased() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-18T12:00:00+09:00", 50, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-18T13:00:00+09:00", 20, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-08-18").await;
    let short = id_starting(&stays(&pool, u).await, "2026-08-18T13:00:00+09:00");

    set(&pool, u, None, Some(15)).await;
    rebuild(&pool, u, "2026-08-18").await;
    assert_eq!(live(&pool, u).await.len(), 1);

    set(&pool, u, None, Some(10)).await;
    rebuild(&pool, u, "2026-08-18").await;
    let back = live(&pool, u).await;
    assert!(
        back.contains(&(
            short,
            t("2026-08-18T13:00:00+09:00"),
            t("2026-08-18T13:20:00+09:00")
        )),
        "吸収された 13:00〜13:20 が本人の削除として扱われ、戻らない: {back:#?}"
    );
}

// ------------------------------------------------------------------ 3.5 位置の記録は変わらない

// Scenario: 半径を変えて作り直すと区切りが変わる
// Scenario: 作り直しで位置の記録は変わらない
#[tokio::test]
async fn stay_rebuild_keeps_locations() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-08-19T09:00:00+09:00", 30, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-08-19T09:31:00+09:00", 29, 70.0, Some(10.0)).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    assert_eq!(live(&pool, u).await.len(), 1, "既定の基準で 1 件のはず");

    let snapshot = || async {
        sqlx::query_as::<
            _,
            (
                uuid::Uuid,
                String,
                DateTime<Utc>,
                String,
                serde_json::Value,
                Option<DateTime<Utc>>,
            ),
        >(
            "SELECT id, raw, event_time, content_hash, payload, deleted_at FROM core.event
              WHERE user_id = $1 AND logical_source = 'c01-location' ORDER BY id",
        )
        .bind(u)
        .fetch_all(&pool)
        .await
        .unwrap()
    };
    let before = snapshot().await;
    set(&pool, u, Some(50), None).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    assert_eq!(
        live(&pool, u).await.len(),
        2,
        "半径 50 m で区切りが変わっていない"
    );
    let after = snapshot().await;
    assert_eq!(before.len(), 61);
    assert_eq!(
        before, after,
        "作り直しで位置の記録（件数・原文・出来事の時刻・内容の鍵）が変わった"
    );
}

// ------------------------------------------------------------------ 3.6 錠

// Scenario: 同じ日の作り直しが同時に 2 回走っても滞在は二重にならない
#[tokio::test]
async fn stay_rebuild_is_serialized() {
    let pool = testdb::pool().await;
    let other = testdb::pool().await;
    let (u, once) = (testdb::user(), testdb::user());
    for who in [u, once] {
        put_dwell(&pool, who, "2026-08-20T09:00:00+09:00", 30, 0.0, Some(10.0)).await;
        put_dwell(
            &pool,
            who,
            "2026-08-20T10:00:00+09:00",
            30,
            900.0,
            Some(10.0),
        )
        .await;
    }
    rebuild(&pool, once, "2026-08-20").await;

    let day = testdb::date("2026-08-20");
    let (a, b) = tokio::join!(
        stay_store::rebuild_day(&pool, u, day),
        stay_store::rebuild_day(&other, u, day)
    );
    a.unwrap();
    b.unwrap();
    let spans = |v: Vec<(uuid::Uuid, DateTime<Utc>, DateTime<Utc>)>| {
        v.into_iter().map(|(_, s, e)| (s, e)).collect::<Vec<_>>()
    };
    assert_eq!(
        spans(live(&pool, u).await),
        spans(live(&pool, once).await),
        "同時に 2 回走って滞在が二重になった"
    );
    assert_eq!(stays(&pool, u).await.len(), 2);
}

// ------------------------------------------------------------------ 3.7 索引

/// 1 日ぶんの位置を引く文が `event_by_source_time` に乗る（tasks 3.7 / design Risks）。
///
/// **行の少ない開発 DB の統計では計画が実データと違う**（実測: `event_hash_all` を選んだ）ので、
/// 使い捨ての DB に 1 人ぶん 14 日（60 秒ごと 20,160 件）の位置を入れて `ANALYZE` してから計画を見る。
#[tokio::test]
async fn stay_day_query_uses_index() {
    let (fresh, drop) = fresh_db().await;
    let got = async {
        for (_, sql) in crate::MIGRATIONS {
            sqlx::raw_sql(sql).execute(&fresh).await?;
        }
        sqlx::query(
            "INSERT INTO core.event
               (id, user_id, logical_source, device_id, origin, event_time,
                tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
             SELECT gen_random_uuid(), '00000000-0000-0000-0000-000000000000', 'c01-location', 'd',
                    'collected', '2026-08-01T00:00:00+09:00'::timestamptz + make_interval(mins => i),
                    540, 'Asia/Tokyo', 1, gen_random_uuid()::text,
                    '{\"lat\":35.68,\"lon\":139.76}', '{\"lat\":35.68,\"lon\":139.76}'
               FROM generate_series(0, 14 * 1440 - 1) AS i",
        )
        .execute(&fresh)
        .await?;
        sqlx::raw_sql("ANALYZE core.event").execute(&fresh).await?;
        let sql = stay_store::POINTS_SQL
            .replace("$1", "'{c01-location}'::text[]")
            .replace("$2", "'2026-08-07T00:00:00+09:00'::timestamptz")
            .replace("$3", "'2026-08-08T00:00:00+09:00'::timestamptz")
            .replace("$4", "'00000000-0000-0000-0000-000000000000'::uuid");
        let plan: Vec<(String,)> = sqlx::query_as(&format!("EXPLAIN {sql}"))
            .fetch_all(&fresh)
            .await?;
        Ok::<String, sqlx::Error>(plan.into_iter().map(|(l,)| l).collect::<Vec<_>>().join("\n"))
    }
    .await;
    drop.await;
    let plan = got.unwrap();
    assert!(
        plan.contains("event_by_source_time"),
        "位置を引く文が索引に乗らない:\n{plan}"
    );
    assert!(
        !plan.contains("Seq Scan on event"),
        "位置を引く文が全走査になる:\n{plan}"
    );
}

/// 使い捨ての DB を作る。**戻り値の 2 つ目を必ず待つ**（接続を閉じて DB を消す）。
async fn fresh_db() -> (sqlx::PgPool, impl std::future::Future<Output = ()>) {
    let admin = testdb::pool().await;
    let name = format!("st16_tmp_{}", uuid::Uuid::new_v4().simple());
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

// ------------------------------------------------------------------ API の足場

const TOKEN: &str = "test-token-0123456789abcdef";

async fn app() -> crate::App {
    crate::App::for_test(testdb::pool().await, TOKEN)
}

fn auth() -> axum::http::HeaderMap {
    let mut h = axum::http::HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

/// 位置の記録 1 件ぶんの取り込みの JSON（端末から届く形）。
fn loc_item(user: uuid::Uuid, at: DateTime<Utc>, north_m: f64) -> serde_json::Value {
    let (lat, lon) = offset(north_m, 0.0);
    serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": user, "logical_source": "c01-location",
        "external_id": null, "device_id": "test-dev", "origin": "collected",
        "event_time": at, "tz_offset_min": 540, "tz_id": "Asia/Tokyo", "schema_version": 1,
        "raw": format!(r#"{{"lat":{lat},"lon":{lon},"acc_m":12}}"#),
        "payload": {"lat": lat, "lon": lon, "acc_m": 12},
    })
}

/// `at` から 60 秒ごとに `minutes + 1` 件、同じ地点の取り込みの JSON。
fn loc_dwell(user: uuid::Uuid, at: &str, minutes: i64, north_m: f64) -> Vec<serde_json::Value> {
    (0..=minutes)
        .map(|i| loc_item(user, t(at) + Duration::minutes(i), north_m))
        .collect()
}

async fn post_locations(app: &crate::App, items: Vec<serde_json::Value>) -> serde_json::Value {
    use axum::{extract::State, Json};
    let (code, Json(res)) = crate::ingest(
        State(app.clone()),
        auth(),
        Json(serde_json::Value::Array(items)),
    )
    .await
    .expect("取り込み口");
    assert_eq!(code, axum::http::StatusCode::OK);
    serde_json::to_value(res).unwrap()
}

// ------------------------------------------------------------------ 4.1 自動の作り直し

// Scenario: 位置を送るとその日の滞在が出る
#[tokio::test]
async fn stay_auto_rebuild_after_ingest() {
    let app = app().await;
    let u = testdb::user();
    let res = post_locations(&app, loc_dwell(u, "2026-09-01T09:00:00+09:00", 20, 0.0)).await;
    assert!(res
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["accepted"] == true));
    let now = live(&app.pool, u).await;
    assert_eq!(
        now.iter().map(|(_, s, e)| (*s, *e)).collect::<Vec<_>>(),
        vec![(
            t("2026-09-01T09:00:00+09:00"),
            t("2026-09-01T09:20:00+09:00")
        )],
        "作り直しを指示しなくても滞在ができるはず"
    );
}

// Scenario: 位置が届かなかった日の滞在は変わらない
#[tokio::test]
async fn stay_auto_rebuild_other_days_untouched() {
    let app = app().await;
    let u = testdb::user();
    post_locations(&app, loc_dwell(u, "2026-09-11T09:00:00+09:00", 30, 0.0)).await;
    post_locations(&app, loc_dwell(u, "2026-09-13T09:00:00+09:00", 30, 0.0)).await;
    let before = stays(&app.pool, u).await;
    let eleventh = before
        .iter()
        .find(|r| stay_store::jst_date(r.event_time) == testdb::date("2026-09-11"))
        .unwrap()
        .clone();
    let versions_before = versions(&app.pool, eleventh.id).await.len();

    post_locations(&app, loc_dwell(u, "2026-09-13T12:00:00+09:00", 60, 0.0)).await;
    let after = stays(&app.pool, u)
        .await
        .into_iter()
        .find(|r| r.id == eleventh.id)
        .unwrap();
    assert_eq!(
        after, eleventh,
        "9 月 11 日の滞在（識別子・内容）が変わった"
    );
    assert_eq!(
        versions(&app.pool, eleventh.id).await.len(),
        versions_before,
        "9 月 11 日の滞在に前の版が積まれた"
    );
    assert_eq!(live(&app.pool, u).await.len(), 3);
}

// Scenario: 0 時の前後で別々に届いても日付をまたぐ滞在は 1 件のまま
#[tokio::test]
async fn stay_auto_rebuild_across_midnight_in_two_posts() {
    let app = app().await;
    let u = testdb::user();
    // 20:00〜23:59 と 00:00〜08:00 を 2 回に分けて送る
    post_locations(&app, loc_dwell(u, "2026-09-03T20:00:00+09:00", 239, 0.0)).await;
    post_locations(&app, loc_dwell(u, "2026-09-04T00:00:00+09:00", 480, 0.0)).await;
    let now = live(&app.pool, u).await;
    assert_eq!(
        now.iter().map(|(_, s, e)| (*s, *e)).collect::<Vec<_>>(),
        vec![(
            t("2026-09-03T20:00:00+09:00"),
            t("2026-09-04T08:00:00+09:00")
        )],
        "0 時で滞在が 2 件に割れた、または重なる滞在が並んだ"
    );
}

/// `tracing` の出力を捕まえる書き込み先。
#[derive(Clone, Default)]
struct Captured(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Scenario: 作り直しが失敗しても位置の記録は受け入れられる
// Scenario: 作り直しの失敗は位置の値を含まずに記録される
#[tokio::test]
async fn stay_auto_rebuild_failure_is_contained() {
    let pool = testdb::pool().await;
    // **作り直しの関数を差し替えて失敗させる**（design D5。表の権限は剥がさない）。
    // エラーの本文にわざと値を入れ、それがログに出ないことを見る
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| {
            Box::pin(async {
                Err(anyhow::anyhow!(
                    "lat=35.6891234 lon=139.7012345 at 2026-09-02T03:17:00Z"
                ))
            })
        }),
        ..crate::App::for_test(pool, TOKEN)
    };
    let u = testdb::user();
    let captured = Captured::default();
    let sink = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || sink.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);

    let items: Vec<_> = (0..=20)
        .map(|i| {
            let mut v = loc_item(
                u,
                t("2026-09-02T12:17:00+09:00") + Duration::minutes(i),
                0.0,
            );
            v["raw"] = serde_json::json!(r#"{"lat":35.6891234,"lon":139.7012345,"acc_m":12}"#);
            v["payload"] = serde_json::json!({"lat": 35.6891234, "lon": 139.7012345, "acc_m": 12});
            v
        })
        .collect();
    let res = post_locations(&app, items).await;
    drop(guard);

    let res = res.as_array().unwrap();
    assert_eq!(res.len(), 21);
    assert!(
        res.iter()
            .all(|r| r["accepted"] == true && r["error"].is_null()),
        "作り直しの失敗が応答に混ざった: {res:?}"
    );
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = 'c01-location'",
    )
    .bind(u)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(n, 21, "位置の記録が受け入れられていない");
    assert!(
        live(&app.pool, u).await.is_empty(),
        "差し替えた作り直しが呼ばれていない"
    );

    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    let failure: Vec<&str> = log.lines().filter(|l| l.contains("stay.rebuild")).collect();
    assert!(
        !failure.is_empty(),
        "作り直しの失敗がログに残っていない:\n{log}"
    );
    assert!(
        failure.iter().any(|l| l.contains("ERROR")),
        "失敗が ERROR で残っていない:\n{log}"
    );
    // 運用者が作り直しを叩ける手がかり（利用者・日・種別）が残る（R41）。どれも位置の値ではない
    assert!(
        failure.iter().any(|l| l.contains(&u.to_string())
            && l.contains("day=2026-09-02")
            && l.contains("failure=other")),
        "失敗のログに利用者・日・種別が無い:\n{log}"
    );
    for value in ["35.689", "139.701", "12:17", "03:17", "T03", "T12"] {
        assert!(
            !log.contains(value),
            "ログに位置の値 {value} が出ている:\n{log}"
        );
    }
}

// ------------------------------------------------------------------ 4.2 作り直しの API

async fn post_rebuild(
    app: &crate::App,
    headers: axum::http::HeaderMap,
    body: serde_json::Value,
) -> Result<serde_json::Value, axum::http::StatusCode> {
    use axum::extract::State;
    crate::stays_rebuild(
        State(app.clone()),
        headers,
        axum::body::Bytes::from(body.to_string()),
    )
    .await
    .map(|axum::Json(r)| serde_json::to_value(r).unwrap())
    .map_err(|(code, _)| code)
}

async fn criteria_list(app: &crate::App, user: uuid::Uuid) -> Vec<serde_json::Value> {
    use axum::extract::{Query, State};
    let axum::Json(v) = crate::stays_criteria_get(
        State(app.clone()),
        auth(),
        Query(crate::StaysCriteriaQuery {
            user_id: Some(user),
        }),
    )
    .await
    .unwrap();
    serde_json::to_value(v).unwrap().as_array().unwrap().clone()
}

// Scenario: 基準を変えても前の基準は一覧に残る
#[tokio::test]
async fn stays_rebuild_api_keeps_old_criteria() {
    let app = app().await;
    let u = testdb::user();
    put_dwell(
        &app.pool,
        u,
        "2026-09-05T09:00:00+09:00",
        30,
        0.0,
        Some(10.0),
    )
    .await;
    post_rebuild(&app, auth(), serde_json::json!({"user_id": u}))
        .await
        .unwrap();
    let got = post_rebuild(
        &app,
        auth(),
        serde_json::json!({"user_id": u, "radius_m": 50}),
    )
    .await
    .unwrap();
    let list = criteria_list(&app, u).await;
    assert_eq!(
        list.iter()
            .map(|c| c["radius_m"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        vec![100, 50]
    );
    assert!(
        list.iter().all(|c| c["created_at"].is_string()),
        "作った時刻が無い"
    );
    assert_eq!(
        got["criteria_id"], list[1]["id"],
        "新しい版で作り直していない"
    );
    assert_eq!(got["stays_after"], 1);
}

// Scenario: いまと同じ基準を添えても基準の版は増えない
#[tokio::test]
async fn stays_rebuild_api_same_criteria_adds_no_version() {
    let app = app().await;
    let u = testdb::user();
    post_rebuild(&app, auth(), serde_json::json!({"user_id": u}))
        .await
        .unwrap();
    assert_eq!(criteria_list(&app, u).await.len(), 1);
    post_rebuild(
        &app,
        auth(),
        serde_json::json!({"user_id": u, "radius_m": 100, "min_minutes": 10, "gap_minutes": 10}),
    )
    .await
    .unwrap();
    post_rebuild(&app, auth(), serde_json::json!({"user_id": u}))
        .await
        .unwrap();
    assert_eq!(
        criteria_list(&app, u).await.len(),
        1,
        "いまと同じ基準で版が増えた"
    );
}

// Scenario: 範囲外の基準は断られる
#[tokio::test]
async fn stays_rebuild_api_rejects_out_of_range() {
    let app = app().await;
    let u = testdb::user();
    put_dwell(
        &app.pool,
        u,
        "2026-09-06T09:00:00+09:00",
        30,
        0.0,
        Some(10.0),
    )
    .await;
    post_rebuild(&app, auth(), serde_json::json!({"user_id": u}))
        .await
        .unwrap();
    let before = (
        criteria_list(&app, u).await.len(),
        stays(&app.pool, u).await,
    );

    for bad in [
        serde_json::json!({"user_id": u, "radius_m": 0}),
        serde_json::json!({"user_id": u, "radius_m": 10_001}),
        serde_json::json!({"user_id": u, "min_minutes": 0}),
        serde_json::json!({"user_id": u, "gap_minutes": 1_441}),
        serde_json::json!({"user_id": u, "min_minutes": 1_441}),
        serde_json::json!({"user_id": u, "gap_minutes": 0}),
        serde_json::json!({"user_id": u, "radius_m": "wide"}),
    ] {
        let got = post_rebuild(&app, auth(), bad.clone()).await;
        assert_eq!(
            got,
            Err(axum::http::StatusCode::BAD_REQUEST),
            "{bad} が断られていない"
        );
    }
    // **断った直後に、基準の版も滞在も変わっていないことを見る**（R34）
    assert_eq!(
        criteria_list(&app, u).await.len(),
        before.0,
        "範囲外の指示で基準の版が増えた"
    );
    assert_eq!(
        stays(&app.pool, u).await,
        before.1,
        "範囲外の指示で滞在が変わった"
    );
    // 境目は通る
    post_rebuild(&app, auth(), serde_json::json!({"user_id": u, "radius_m": 10_000, "min_minutes": 1, "gap_minutes": 1_440}))
        .await
        .unwrap();
    assert_eq!(criteria_list(&app, u).await.len(), before.0 + 1);
}

// Scenario: 資格情報の無い作り直しの指示は断られる
#[tokio::test]
async fn stays_rebuild_api_requires_credentials() {
    let app = app().await;
    let u = testdb::user();
    put_dwell(
        &app.pool,
        u,
        "2026-09-07T09:00:00+09:00",
        30,
        0.0,
        Some(10.0),
    )
    .await;
    let got = post_rebuild(
        &app,
        axum::http::HeaderMap::new(),
        serde_json::json!({"user_id": u, "radius_m": 50}),
    )
    .await;
    assert_eq!(got, Err(axum::http::StatusCode::UNAUTHORIZED));
    assert!(
        criteria_list(&app, u).await.is_empty(),
        "資格情報の無い指示で基準が変わった"
    );
    assert!(
        stays(&app.pool, u).await.is_empty(),
        "資格情報の無い指示で滞在が作られた"
    );
}

/// 範囲外の検査は、基準も滞在も変えないうちに断る（上の Scenario の後半）。
#[tokio::test]
async fn stays_rebuild_api_rejection_changes_nothing() {
    let app = app().await;
    let u = testdb::user();
    put_dwell(
        &app.pool,
        u,
        "2026-09-08T09:00:00+09:00",
        30,
        0.0,
        Some(10.0),
    )
    .await;
    post_rebuild(&app, auth(), serde_json::json!({"user_id": u}))
        .await
        .unwrap();
    let (criteria, rows) = (criteria_list(&app, u).await, stays(&app.pool, u).await);
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE user_id = $1 AND logical_source = 'c01-location'")
        .bind(u)
        .execute(&app.pool)
        .await
        .unwrap();
    // 位置を消した後に範囲外で指示する。**通っていれば滞在は吸収されて消える**
    assert_eq!(
        post_rebuild(
            &app,
            auth(),
            serde_json::json!({"user_id": u, "radius_m": 0})
        )
        .await,
        Err(axum::http::StatusCode::BAD_REQUEST)
    );
    assert_eq!(
        criteria_list(&app, u).await,
        criteria,
        "範囲外の指示で基準の版が増えた"
    );
    assert_eq!(
        stays(&app.pool, u).await,
        rows,
        "範囲外の指示で滞在が変わった"
    );
}

// ------------------------------------------------------------------ 5.1 1 日の並び

use crate::stay_store::{DayEntry, EntryKind};

async fn day(pool: &sqlx::PgPool, user: uuid::Uuid, date: &str, now: &str) -> stay_store::DayView {
    stay_store::day_view(pool, user, testdb::date(date), t(now))
        .await
        .unwrap()
}

fn kinds(v: &stay_store::DayView, kind: EntryKind) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    v.entries
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| (e.start, e.end))
        .collect()
}

/// 北へ `north_m` ずつ 1 分ごとに進む位置（移動。どの 2 点も半径 100 m に入らない）。
async fn put_walk(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    at: &str,
    minutes: i64,
    from_m: f64,
    to_m: f64,
) {
    for i in 0..minutes {
        let north = from_m + (to_m - from_m) * (i + 1) as f64 / (minutes + 1) as f64;
        let at = (t(at) + Duration::minutes(i)).to_rfc3339();
        put_dwell(pool, user, &at, 0, north, Some(10.0)).await;
    }
}

// Scenario: 1 日の並びは種類と時刻を持つ
// Scenario: 解釈できない日付は断られる
// Scenario: 資格情報の無い 1 日の並びの求めは断られる
#[tokio::test]
async fn stays_day_api_shape_and_errors() {
    use axum::extract::{Query, State};
    let app = app().await;
    let u = testdb::user();
    put_dwell(
        &app.pool,
        u,
        "2026-07-01T09:00:00+09:00",
        30,
        0.0,
        Some(10.0),
    )
    .await;
    put_walk(&app.pool, u, "2026-07-01T09:31:00+09:00", 20, 0.0, 20_000.0).await;
    put_dwell(
        &app.pool,
        u,
        "2026-07-01T09:51:00+09:00",
        30,
        20_000.0,
        Some(10.0),
    )
    .await;
    stay_store::rebuild_all(&app.pool, u).await.unwrap();

    let call = |date: &str, headers| {
        crate::stays_get(
            State(app.clone()),
            headers,
            Query(crate::StaysQuery {
                date: date.into(),
                user_id: Some(u),
            }),
        )
    };
    let axum::Json(v) = call("2026-07-01", auth()).await.unwrap();
    let json = serde_json::to_value(&v).unwrap();
    let entries = json["entries"].as_array().unwrap();
    assert_eq!(entries.iter().filter(|e| e["kind"] == "stay").count(), 2);
    for e in entries {
        assert!(
            ["stay", "move", "no-record"].contains(&e["kind"].as_str().unwrap()),
            "種類が 3 つのどれでもない: {e}"
        );
        assert!(
            e["start"].is_string() && e["end"].is_string(),
            "始まりと終わりが無い: {e}"
        );
        assert_eq!(
            e["kind"] == "stay",
            e["id"].is_string(),
            "滞在だけが識別子を持つはず: {e}"
        );
    }
    let starts: Vec<&str> = entries
        .iter()
        .map(|e| e["start"].as_str().unwrap())
        .collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted, "始まりの時刻順でない");

    assert!(matches!(
        call("2026-13-40", auth()).await,
        Err((axum::http::StatusCode::BAD_REQUEST, _))
    ));
    assert!(matches!(
        call("yesterday", auth()).await,
        Err((axum::http::StatusCode::BAD_REQUEST, _))
    ));
    assert!(matches!(
        call("2026-07-01", axum::http::HeaderMap::new()).await,
        Err((axum::http::StatusCode::UNAUTHORIZED, _))
    ));
}

// Scenario: 日付をまたぐ滞在は両方の日に出る
#[tokio::test]
async fn stays_day_api_across_midnight_on_both_days() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-07-02T23:00:00+09:00", 180, 0.0, Some(10.0)).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    for date in ["2026-07-02", "2026-07-03"] {
        let v = day(&pool, u, date, "2026-09-01T00:00:00Z").await;
        assert_eq!(
            kinds(&v, EntryKind::Stay),
            vec![(
                t("2026-07-02T23:00:00+09:00"),
                t("2026-07-03T02:00:00+09:00")
            )],
            "{date} の一覧に 23:00 – 02:00 の滞在が実際の時刻で出ていない"
        );
    }
}

// Scenario: 消した滞在と吸収された滞在は一覧に出ない
#[tokio::test]
async fn stays_day_api_hides_deleted_and_absorbed() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-07-04T09:00:00+09:00", 30, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-07-04T09:35:00+09:00", 55, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-07-04T15:00:00+09:00", 30, 0.0, Some(10.0)).await;
    set(&pool, u, None, Some(5)).await;
    rebuild(&pool, u, "2026-07-04").await;
    let rows = stays(&pool, u).await;
    let (absorbed, deleted) = (
        id_starting(&rows, "2026-07-04T09:00:00+09:00"),
        id_starting(&rows, "2026-07-04T15:00:00+09:00"),
    );
    user_deletes(&pool, deleted, Some("user")).await;
    set(&pool, u, None, Some(10)).await;
    rebuild(&pool, u, "2026-07-04").await;

    let v = day(&pool, u, "2026-07-04", "2026-09-01T00:00:00Z").await;
    let ids: Vec<uuid::Uuid> = v.entries.iter().filter_map(|e| e.id).collect();
    assert_eq!(ids.len(), 1, "{v:#?}");
    assert!(!ids.contains(&absorbed), "吸収された滞在が一覧に出ている");
    assert!(!ids.contains(&deleted), "本人が消した滞在が一覧に出ている");
}

// Scenario: 滞在の間に移動の行が出る
#[tokio::test]
async fn stays_day_api_move_between_stays() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-07-05T08:00:00+09:00", 40, 0.0, Some(10.0)).await;
    put_walk(&pool, u, "2026-07-05T08:41:00+09:00", 41, 0.0, 50_000.0).await;
    put_dwell(
        &pool,
        u,
        "2026-07-05T09:22:00+09:00",
        38,
        50_000.0,
        Some(10.0),
    )
    .await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    let v = day(&pool, u, "2026-07-05", "2026-09-01T00:00:00Z").await;
    let moves = kinds(&v, EntryKind::Move);
    let wanted = (
        t("2026-07-05T08:40:00+09:00"),
        t("2026-07-05T09:22:00+09:00"),
    );
    assert!(
        moves.contains(&wanted),
        "08:40 – 09:22 の移動の行が無い: {v:#?}"
    );
    assert_eq!(wanted.1 - wanted.0, Duration::minutes(42));
    // 並びの中で 2 つの滞在の間にある
    let pos = |k: EntryKind, s: DateTime<Utc>| {
        v.entries
            .iter()
            .position(|e| e.kind == k && e.start == s)
            .unwrap()
    };
    let (a, m, b) = (
        pos(EntryKind::Stay, t("2026-07-05T08:00:00+09:00")),
        pos(EntryKind::Move, wanted.0),
        pos(EntryKind::Stay, t("2026-07-05T09:22:00+09:00")),
    );
    assert!(a < m && m < b);
}

// Scenario: 記録が欠けた時間は記録なしとして出る
#[tokio::test]
async fn stays_day_api_gap_is_no_record() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-07-06T08:00:00+09:00", 20, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-07-06T16:40:00+09:00", 20, 0.0, Some(10.0)).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    let v = day(&pool, u, "2026-07-06", "2026-09-01T00:00:00Z").await;
    let wanted = (
        t("2026-07-06T08:20:00+09:00"),
        t("2026-07-06T16:40:00+09:00"),
    );
    assert!(
        kinds(&v, EntryKind::NoRecord).contains(&wanted),
        "08:20 – 16:40 の記録なしの行が無い: {v:#?}"
    );
    assert!(
        !v.entries
            .iter()
            .any(|e| e.kind == EntryKind::Move && e.start < wanted.1 && e.end > wanted.0),
        "記録が無い時間が移動として出ている"
    );
}

// Scenario: 位置の記録が無い日は丸ごと記録なしになる
#[tokio::test]
async fn stays_day_api_empty_day_is_all_no_record() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let v = day(&pool, u, "2026-07-07", "2026-09-01T00:00:00Z").await;
    assert_eq!(
        v.entries,
        vec![DayEntry {
            kind: EntryKind::NoRecord,
            start: t("2026-07-07T00:00:00+09:00"),
            end: t("2026-07-08T00:00:00+09:00"),
            id: None,
            criteria_id: None,
        }]
    );
    assert!(v.criteria.is_empty());
}

// Scenario: 前の日から途切れず続く記録は日の頭を記録なしにしない
#[tokio::test]
async fn stays_day_api_continuous_from_previous_day() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // **分の頭からずらす**。前の日の記録を読まない実装だと、00:00 から最初の位置（00:00:30）までが記録なしになる
    put_dwell(
        &pool,
        u,
        "2026-07-08T23:00:30+09:00",
        8 * 60,
        0.0,
        Some(10.0),
    )
    .await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    let v = day(&pool, u, "2026-07-09", "2026-09-01T00:00:00Z").await;
    let head = (
        t("2026-07-09T00:00:00+09:00"),
        t("2026-07-09T07:00:00+09:00"),
    );
    assert!(
        !kinds(&v, EntryKind::NoRecord)
            .iter()
            .any(|(s, e)| *s < head.1 && *e > head.0),
        "前の日から続く記録なのに 00:00 – 07:00 に記録なしが出ている: {v:#?}"
    );
    // 尻（07:00 以降）は記録が無いので記録なし
    assert!(
        kinds(&v, EntryKind::NoRecord).contains(&(
            t("2026-07-09T07:00:30+09:00"),
            t("2026-07-10T00:00:00+09:00")
        )),
        "{v:#?}"
    );
}

// Scenario: 今日の一覧はいまより後を記録なしにしない
#[tokio::test]
async fn stays_day_api_today_stops_at_now() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-07-10T08:00:00+09:00", 55, 0.0, Some(10.0)).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    let now = t("2026-07-10T09:00:00+09:00");
    let v = day(&pool, u, "2026-07-10", "2026-07-10T09:00:00+09:00").await;
    assert!(
        v.entries
            .iter()
            .all(|e| e.kind == EntryKind::Stay || e.end <= now),
        "09:00 より後の時刻を持つ行がある: {v:#?}"
    );
    assert!(
        !v.entries.iter().any(|e| e.start > now),
        "09:00 より後に始まる行がある: {v:#?}"
    );
    // 最後の位置からいままでが 10 分以上なら、そこまでを記録なしにする
    let later = day(&pool, u, "2026-07-10", "2026-07-10T09:30:00+09:00").await;
    assert!(
        kinds(&later, EntryKind::NoRecord).contains(&(
            t("2026-07-10T08:55:00+09:00"),
            t("2026-07-10T09:30:00+09:00")
        )),
        "{later:#?}"
    );
}

// Scenario: 基準を変えて作り直すと一覧の基準の表示が変わる
#[tokio::test]
async fn stays_day_api_criteria_follow_rebuild() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-07-11T09:00:00+09:00", 30, 0.0, Some(10.0)).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    let v = day(&pool, u, "2026-07-11", "2026-09-01T00:00:00Z").await;
    assert_eq!(
        (
            v.criteria.len(),
            v.criteria[0].radius_m,
            v.criteria[0].min_minutes
        ),
        (1, 100, 10)
    );

    set(&pool, u, Some(50), None).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    let v = day(&pool, u, "2026-07-11", "2026-09-01T00:00:00Z").await;
    assert_eq!(
        v.criteria[0].radius_m, 50,
        "作り直した後も一覧の基準が 100 m のまま: {v:#?}"
    );
    assert_eq!(v.criteria.len(), 1);
    assert!(v
        .entries
        .iter()
        .filter(|e| e.kind == EntryKind::Stay)
        .all(|e| e.criteria_id == Some(v.criteria[0].criteria_id)));
}

// ------------------------------------------------------------------ 5.2 歩き回った日

// Scenario: 1 日歩き回った後、その日の滞在が一覧で出る
#[tokio::test]
async fn stays_day_api_walked_day() {
    let app = app().await;
    let u = testdb::user();
    const HOME: f64 = 0.0;
    const WORK: f64 = 20_000.0;
    const LUNCH: f64 = 23_000.0;
    let mut items = Vec::new();
    let mut dwell =
        |at: &str, minutes: i64, north: f64| items.extend(loc_dwell(u, at, minutes, north));
    dwell("2026-06-30T23:30:00+09:00", 510, HOME); // 〜08:00
    dwell("2026-07-01T08:30:00+09:00", 210, WORK); // 〜12:00
    dwell("2026-07-01T12:10:00+09:00", 40, LUNCH); // 〜12:50
    dwell("2026-07-01T13:00:00+09:00", 300, WORK); // 〜18:00
    dwell("2026-07-01T18:30:00+09:00", 360, HOME); // 〜翌 00:30
    let walks = [
        ("2026-07-01T08:01:00+09:00", 29, HOME, WORK),
        ("2026-07-01T12:01:00+09:00", 9, WORK, LUNCH),
        ("2026-07-01T12:51:00+09:00", 9, LUNCH, WORK),
        ("2026-07-01T18:01:00+09:00", 29, WORK, HOME),
    ];
    for (at, minutes, from, to) in walks {
        for i in 0..minutes {
            let north = from + (to - from) * (i + 1) as f64 / (minutes + 1) as f64;
            items.push(loc_item(u, t(at) + Duration::minutes(i), north));
        }
    }
    // 端末と同じく 200 件ずつ送る（作り直しは送るたびに走る）
    items.sort_by_key(|v| v["event_time"].as_str().unwrap().to_string());
    for chunk in items.chunks(200) {
        post_locations(&app, chunk.to_vec()).await;
    }

    let v = day(&app.pool, u, "2026-07-01", "2026-09-01T00:00:00Z").await;
    let stays = kinds(&v, EntryKind::Stay);
    assert_eq!(stays.len(), 5, "滞在の行が 5 件でない: {v:#?}");
    assert_eq!(
        kinds(&v, EntryKind::Move).len(),
        4,
        "移動の行が 4 件でない: {v:#?}"
    );
    assert!(
        kinds(&v, EntryKind::NoRecord).is_empty(),
        "途切れずに記録した日に記録なしが出ている: {v:#?}"
    );
    assert_eq!(stays[0].0, t("2026-06-30T23:30:00+09:00"));
    assert_eq!(stays[4].1, t("2026-07-02T00:30:00+09:00"));
    let mut sorted = stays.clone();
    sorted.sort();
    assert_eq!(stays, sorted, "始まりの時刻順でない");
}

// ------------------------------------------------------------------ 独立レビューで足したもの（review/code.md）

/// R40: 短い移動でつながった滞在が何日も続いても、作り直しの範囲が定まる。
///
/// 範囲の端の**近く**にある滞在まで広げていたときは、隣の滞在を 1 件ずつたどり、3 日目で
/// 「64 回で定まらない」になった（code-reviewer の実測）。
#[tokio::test]
async fn stay_rebuild_chain_of_short_moves_settles() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 同じ地点に 20 分とどまる → 3 分空けて 500 m 先へ、を 3 日ぶん
    let mut at = t("2026-05-01T00:00:00+09:00");
    let mut k = 0;
    while at < t("2026-05-04T00:00:00+09:00") {
        put_dwell(
            &pool,
            u,
            &at.to_rfc3339(),
            20,
            500.0 * f64::from(k % 2),
            Some(10.0),
        )
        .await;
        at += Duration::minutes(23);
        k += 1;
    }
    for day in ["2026-05-01", "2026-05-02", "2026-05-03"] {
        let started = std::time::Instant::now();
        stay_store::rebuild_day(&pool, u, testdb::date(day))
            .await
            .unwrap_or_else(|e| panic!("{day} の作り直しが失敗した: {e}"));
        assert!(
            started.elapsed() < std::time::Duration::from_secs(3),
            "{day} の作り直しが隣の滞在をたどっている"
        );
    }
    let once = live(&pool, u).await;
    assert!(once.len() > 180, "{}", once.len());
    for day in ["2026-05-02", "2026-05-03", "2026-05-01"] {
        rebuild(&pool, u, day).await;
    }
    assert_eq!(live(&pool, u).await, once, "2 周目で滞在が変わった");
}

/// R30: 範囲の端でとどまりが続けば、閉じるところまで広げる（0 時前の分が最短のとどまりに満たないとき）。
// Scenario: 0 時の前後で別々に届いても日付をまたぐ滞在は 1 件のまま
#[tokio::test]
async fn stay_auto_rebuild_short_tail_before_midnight() {
    let app = app().await;
    let u = testdb::user();
    post_locations(&app, loc_dwell(u, "2026-05-10T23:52:00+09:00", 7, 0.0)).await;
    post_locations(&app, loc_dwell(u, "2026-05-11T00:00:00+09:00", 20, 0.0)).await;
    assert_eq!(
        live(&app.pool, u)
            .await
            .iter()
            .map(|(_, s, e)| (*s, *e))
            .collect::<Vec<_>>(),
        vec![(
            t("2026-05-10T23:52:00+09:00"),
            t("2026-05-11T00:20:00+09:00")
        )],
        "0 時前の 7 分ぶんが滞在に入っていない（範囲を端のとどまりで広げていない）"
    );
    // 1 回で送った 23:55〜00:05（どちらの日も 10 分に満たない）も 1 件になる
    let v = testdb::user();
    post_locations(&app, loc_dwell(v, "2026-05-12T23:55:00+09:00", 10, 0.0)).await;
    assert_eq!(
        live(&app.pool, v)
            .await
            .iter()
            .map(|(_, s, e)| (*s, *e))
            .collect::<Vec<_>>(),
        vec![(
            t("2026-05-12T23:55:00+09:00"),
            t("2026-05-13T00:05:00+09:00")
        )]
    );
}

/// R54: Asia/Tokyo の 0:00〜8:59 に届いた位置でも、その日の滞在ができる（UTC の日付で作り直さない）。
// Scenario: 位置を送るとその日の滞在が出る
#[tokio::test]
async fn stay_auto_rebuild_early_morning_jst() {
    let app = app().await;
    let u = testdb::user();
    post_locations(&app, loc_dwell(u, "2026-05-14T02:00:00+09:00", 20, 0.0)).await;
    assert_eq!(
        live(&app.pool, u).await.len(),
        1,
        "早朝の位置で滞在ができない"
    );
}

/// R31 / R69: 重複だけのまとめ送りでも作り直す。受け入れなかった記録の日は作り直さない。
#[tokio::test]
async fn stay_auto_rebuild_counts_duplicates_not_rejections() {
    let pool = testdb::pool().await;
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = calls.clone();
    let app = crate::App {
        stays: crate::StayRebuilder::from_fn(move |_, user, day| {
            seen.lock().unwrap().push((user, day));
            Box::pin(async { Ok(()) })
        }),
        ..crate::App::for_test(pool, TOKEN)
    };
    let u = testdb::user();
    let batch = loc_dwell(u, "2026-05-15T09:00:00+09:00", 3, 0.0);
    post_locations(&app, batch.clone()).await;
    let again = post_locations(&app, batch).await;
    assert!(again
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["duplicate"] == true));
    assert_eq!(
        calls.lock().unwrap().len(),
        2,
        "重複だけのまとめ送りで作り直していない（D5）"
    );

    // 断られた記録（登録簿に無いソース）の日は作り直さない
    let mut bad = loc_item(u, t("2026-05-20T09:00:00+09:00"), 0.0);
    bad["logical_source"] = serde_json::json!("t-not-registered-at-all");
    let ok = loc_item(u, t("2026-05-15T09:10:00+09:00"), 0.0);
    post_locations(&app, vec![bad, ok]).await;
    let days: Vec<_> = calls.lock().unwrap().iter().map(|(_, d)| *d).collect();
    assert_eq!(days.len(), 3);
    assert!(
        !days.contains(&testdb::date("2026-05-20")),
        "断った記録の日を作り直した"
    );
}

/// R53: 作り直しは利用者の錠を待つ（同時に走らせたときのたまたまの交差に頼らない）。
// Scenario: 同じ日の作り直しが同時に 2 回走っても滞在は二重にならない
#[tokio::test]
async fn stay_rebuild_is_serialized_waits_for_lock() {
    let pool = testdb::pool().await;
    let holder = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-05-16T09:00:00+09:00", 30, 0.0, Some(10.0)).await;

    let mut tx = holder.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text))")
        .bind(stay_store::LOCK_KEY)
        .bind(u)
        .execute(&mut *tx)
        .await
        .unwrap();
    let task = tokio::spawn({
        let pool = pool.clone();
        async move { stay_store::rebuild_day(&pool, u, testdb::date("2026-05-16")).await }
    });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(
        !task.is_finished(),
        "錠を握られているのに作り直しが終わった（錠を取っていない）"
    );
    assert!(stays(&pool, u).await.is_empty());
    tx.rollback().await.unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(live(&pool, u).await.len(), 1);
}

/// R55: 吸収先は時間の重なりがいちばん大きい新しい滞在（ST17 / ST20 が紐づけを移すかを決める列）。
// Scenario: 吸収された滞在の行と吸収先が残る
#[tokio::test]
async fn stay_identity_absorbed_into_largest_overlap() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let x = put_stay_row(
        &pool,
        u,
        "2026-05-17T09:00:00+09:00",
        "2026-05-17T11:00:00+09:00",
    )
    .await;
    let y = put_stay_row(
        &pool,
        u,
        "2026-05-17T09:10:00+09:00",
        "2026-05-17T11:00:00+09:00",
    )
    .await;
    let z = put_stay_row(
        &pool,
        u,
        "2026-05-17T09:20:00+09:00",
        "2026-05-17T10:50:00+09:00",
    )
    .await;
    put_dwell(&pool, u, "2026-05-17T09:00:00+09:00", 40, 0.0, Some(10.0)).await;
    put_dwell(&pool, u, "2026-05-17T09:50:00+09:00", 70, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-05-17").await;
    let now = live(&pool, u).await;
    assert_eq!(
        now.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
        vec![y, x]
    );
    let (into,): (Option<uuid::Uuid>,) =
        sqlx::query_as("SELECT into_event_id FROM core.stay_absorbed WHERE event_id = $1")
            .bind(z)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(into, Some(x), "吸収先が重なりの最大（09:50〜11:00）でない");
}

/// R56: 位置の記録が無くなった日の滞在は、全期間の作り直しで吸収される（吸収先は無い）。
#[tokio::test]
async fn stay_rebuild_all_absorbs_stays_without_points() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-05-18T09:00:00+09:00", 30, 0.0, Some(10.0)).await;
    stay_store::rebuild_all(&pool, u).await.unwrap();
    let id = live(&pool, u).await[0].0;
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE user_id = $1 AND logical_source = 'c01-location'")
        .bind(u)
        .execute(&pool)
        .await
        .unwrap();
    stay_store::rebuild_all(&pool, u).await.unwrap();
    assert!(
        live(&pool, u).await.is_empty(),
        "位置の無い滞在が残っている"
    );
    let (into,): (Option<uuid::Uuid>,) =
        sqlx::query_as("SELECT into_event_id FROM core.stay_absorbed WHERE event_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(into, None);
}

/// R28: 重なりが同じなら、読み出しに出ている既存の滞在を先に割り当てる（spec の要件本文）。
#[tokio::test]
async fn stay_identity_tie_prefers_live() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 吸収済みの方を識別子の順で先に置く（「読み出しに出ている方を先に」を消すと必ず落ちる）
    let absorbed = put_stay_row_with(
        &pool,
        u,
        uuid::Uuid::from_u128(u.as_u128() & 0xffff_ffff),
        "2026-05-19T09:00:00+09:00",
        "2026-05-19T09:30:00+09:00",
        None,
    )
    .await;
    let live_row = put_stay_row_with(
        &pool,
        u,
        uuid::Uuid::from_u128(u128::MAX - (u.as_u128() & 0xffff_ffff)),
        "2026-05-19T09:00:00+09:00",
        "2026-05-19T09:30:00+09:00",
        None,
    )
    .await;
    sqlx::query(
        "UPDATE core.event SET deleted_at = now(), deleted_by = 'rebuild:absorbed' WHERE id = $1",
    )
    .bind(absorbed)
    .execute(&pool)
    .await
    .unwrap();
    put_dwell(&pool, u, "2026-05-19T09:00:00+09:00", 30, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-05-19").await;
    assert_eq!(
        live(&pool, u).await[0].0,
        live_row,
        "吸収済みの滞在が識別子を継いだ"
    );
}

/// R29 / D12: 端が触れるだけの重なり —— 引き継ぎでは数えず、本人が消した時間帯では数える。
#[tokio::test]
async fn stay_identity_touching_edges() {
    let pool = testdb::pool().await;
    // 引き継ぎ: 09:00〜09:30 の滞在は、09:30 に始まる滞在の識別子にならない
    let u = testdb::user();
    let old = put_stay_row(
        &pool,
        u,
        "2026-05-21T09:00:00+09:00",
        "2026-05-21T09:30:00+09:00",
    )
    .await;
    put_dwell(&pool, u, "2026-05-21T09:30:00+09:00", 30, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-05-21").await;
    let now = live(&pool, u).await;
    assert_eq!(now.len(), 1);
    assert_ne!(now[0].0, old, "端が触れるだけの滞在が識別子を継いだ（D12）");

    // 本人が消した時間帯: 09:00〜09:30 を消した後、09:30 に始まる滞在は隠れる
    let v = testdb::user();
    let erased = put_stay_row(
        &pool,
        v,
        "2026-05-21T09:00:00+09:00",
        "2026-05-21T09:30:00+09:00",
    )
    .await;
    user_deletes(&pool, erased, Some("user")).await;
    put_dwell(&pool, v, "2026-05-21T09:30:00+09:00", 30, 500.0, Some(10.0)).await;
    rebuild(&pool, v, "2026-05-21").await;
    assert!(
        live(&pool, v).await.is_empty(),
        "消した時間帯に端で触れる滞在が隠れていない（D12 / Q12）"
    );
}

/// R58: 内容（`raw`）は同じで、削除の印だけが変わる経路。
#[tokio::test]
async fn stay_erased_range_mark_only_changes() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-05-22T09:00:00+09:00", 60, 0.0, Some(10.0)).await;
    rebuild(&pool, u, "2026-05-22").await;
    let id = live(&pool, u).await[0].0;
    // 重なる短い行を本人が消す（作り直しの内容は変わらない）
    let erased = put_stay_row(
        &pool,
        u,
        "2026-05-22T09:10:00+09:00",
        "2026-05-22T09:20:00+09:00",
    )
    .await;
    user_deletes(&pool, erased, Some("user")).await;
    rebuild(&pool, u, "2026-05-22").await;
    assert!(
        live(&pool, u).await.is_empty(),
        "内容が同じだと隠す印を付けていない"
    );
    let versions_hidden = versions(&pool, id).await.len();

    // 本人が消したのを戻す（ST22 の「戻す」と同じ形）と、印も外れる
    sqlx::query("UPDATE core.event SET deleted_at = NULL, deleted_by = NULL WHERE id = $1")
        .bind(erased)
        .execute(&pool)
        .await
        .unwrap();
    rebuild(&pool, u, "2026-05-22").await;
    let back = live(&pool, u).await;
    assert_eq!(
        back.iter().map(|(i, _, _)| *i).collect::<Vec<_>>(),
        vec![id],
        "印が外れていない"
    );
    assert_eq!(
        versions(&pool, id).await.len(),
        versions_hidden,
        "印だけの変化で前の版を積んだ"
    );
}

/// R44: 読んだ後に削除の印が変わった行は書き換えない（錠を取らない書き手が本人の削除を付けた場合）。
#[tokio::test]
async fn stay_rebuild_does_not_overwrite_changed_marks() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let id = put_stay_row(
        &pool,
        u,
        "2026-05-23T09:00:00+09:00",
        "2026-05-23T10:00:00+09:00",
    )
    .await;
    // 作り直しが「読み出しに出ている」と読んだ後に、本人が消した
    user_deletes(&pool, id, Some("user")).await;
    let mut tx = pool.begin().await.unwrap();
    let got = stay_store::absorb_for_test(&mut tx, u, id, None).await;
    tx.rollback().await.unwrap();
    assert!(got.is_err(), "読んだ時点と印が違う行を吸収で書き換えた");
    let row = stays(&pool, u)
        .await
        .into_iter()
        .find(|r| r.id == id)
        .unwrap();
    assert_eq!(row.deleted_by.as_deref(), Some("user"));
}

// ------------------------------------------------------------------ 独立レビュー: API の本文

/// R60: 本文なしで作り直せる / 省いた基準はいまの値のまま / 知らない欄は断る。
#[tokio::test]
async fn stays_rebuild_api_body() {
    let app = app().await;
    let u = testdb::user();
    // 本文なしは既定の利用者を作り直す。**既定の利用者の基準は変えない**（テストの規律）ので、ここは空の本文が 200 で通ることだけを見る
    use axum::extract::State;
    let empty = crate::stays_rebuild(State(app.clone()), auth(), axum::body::Bytes::new()).await;
    assert!(empty.is_ok(), "本文なしの作り直しが断られた");

    post_rebuild(
        &app,
        auth(),
        serde_json::json!({"user_id": u, "min_minutes": 20, "gap_minutes": 30}),
    )
    .await
    .unwrap();
    post_rebuild(
        &app,
        auth(),
        serde_json::json!({"user_id": u, "radius_m": 60}),
    )
    .await
    .unwrap();
    let now = stay_store::current_criteria(&app.pool, u)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (now.radius_m, now.min_minutes, now.gap_minutes),
        (60, 20, 30),
        "省いた基準が既定に戻った"
    );

    assert_eq!(
        post_rebuild(
            &app,
            auth(),
            serde_json::json!({"user_id": u, "radius": 50})
        )
        .await,
        Err(axum::http::StatusCode::BAD_REQUEST),
        "綴り違いの欄が黙って無視された"
    );
    assert_eq!(criteria_list(&app, u).await.len(), 3);
}

// ------------------------------------------------------------------ 独立レビュー: 1 日の並び

/// R57: 2 日を超える滞在は、真ん中の日と最後の日の一覧にも出る。
#[tokio::test]
async fn stays_day_api_long_stay_on_every_day() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_stay_row(
        &pool,
        u,
        "2026-06-01T12:00:00+09:00",
        "2026-06-04T12:00:00+09:00",
    )
    .await;
    for date in ["2026-06-01", "2026-06-02", "2026-06-03", "2026-06-04"] {
        let v = day(&pool, u, date, "2026-09-01T00:00:00Z").await;
        assert_eq!(
            kinds(&v, EntryKind::Stay).len(),
            1,
            "{date} に 72 時間の滞在が出ていない"
        );
    }
    assert!(kinds(
        &day(&pool, u, "2026-06-05", "2026-09-01T00:00:00Z").await,
        EntryKind::Stay
    )
    .is_empty());
}

/// R33: 0:00 ちょうどに終わる滞在は翌日に出さない / 基準は新しい版から並べ、重複しない。
#[tokio::test]
async fn stays_day_api_boundary_and_criteria_order() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_stay_row(
        &pool,
        u,
        "2026-06-10T22:00:00+09:00",
        "2026-06-11T00:00:00+09:00",
    )
    .await;
    assert!(
        kinds(
            &day(&pool, u, "2026-06-11", "2026-09-01T00:00:00Z").await,
            EntryKind::Stay
        )
        .is_empty(),
        "前の日の 24:00 ちょうどに終わる滞在が翌日に出た"
    );
    assert_eq!(
        kinds(
            &day(&pool, u, "2026-06-10", "2026-09-01T00:00:00Z").await,
            EntryKind::Stay
        )
        .len(),
        1
    );

    let v = testdb::user();
    put_stay_row_with(
        &pool,
        v,
        uuid::Uuid::new_v4(),
        "2026-06-12T08:00:00+09:00",
        "2026-06-12T09:00:00+09:00",
        Some((5, 100, 10)),
    )
    .await;
    put_stay_row_with(
        &pool,
        v,
        uuid::Uuid::new_v4(),
        "2026-06-12T10:00:00+09:00",
        "2026-06-12T11:00:00+09:00",
        Some((9, 50, 10)),
    )
    .await;
    put_stay_row_with(
        &pool,
        v,
        uuid::Uuid::new_v4(),
        "2026-06-12T12:00:00+09:00",
        "2026-06-12T13:00:00+09:00",
        Some((5, 100, 10)),
    )
    .await;
    let got = day(&pool, v, "2026-06-12", "2026-09-01T00:00:00Z").await;
    assert_eq!(
        got.criteria
            .iter()
            .map(|c| c.criteria_id)
            .collect::<Vec<_>>(),
        vec![9, 5],
        "基準が新しい版から並んでいない"
    );
}

/// R32: 今日は、最後の位置からいままでが間隔に満たなければ記録なしにせず、移動も最後の位置までしか並べない。
// Scenario: 今日の一覧はいまより後を記録なしにしない
#[tokio::test]
async fn stays_day_api_today_short_tail() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-06-13T08:00:00+09:00", 30, 0.0, Some(10.0)).await;
    put_walk(&pool, u, "2026-06-13T08:31:00+09:00", 3, 0.0, 20_000.0).await; // 〜08:33
    let v = day(&pool, u, "2026-06-13", "2026-06-13T08:38:00+09:00").await;
    assert!(
        kinds(&v, EntryKind::NoRecord)
            .iter()
            .all(|(_, e)| *e <= t("2026-06-13T08:00:00+09:00")),
        "最後の位置から 5 分で記録なしが出た: {v:#?}"
    );
    assert!(
        v.entries
            .iter()
            .all(|e| e.kind == EntryKind::Stay || e.end <= t("2026-06-13T08:33:00+09:00")),
        "移動が最後の位置より先まで伸びた: {v:#?}"
    );
}

/// R65: 過ぎた日の尻は、次の日の最初の記録までの間隔で測る。
#[tokio::test]
async fn stays_day_api_tail_measured_into_next_day() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-06-14T23:00:00+09:00", 53, 0.0, Some(10.0)).await; // 〜23:53
    put_dwell(&pool, u, "2026-06-15T00:03:00+09:00", 20, 0.0, Some(10.0)).await;
    let v = day(&pool, u, "2026-06-14", "2026-09-01T00:00:00Z").await;
    assert!(
        kinds(&v, EntryKind::NoRecord).contains(&(
            t("2026-06-14T23:53:00+09:00"),
            t("2026-06-15T00:00:00+09:00")
        )),
        "{v:#?}"
    );
}

/// R35: 本人が消した滞在の時間を「移動」として出さない。
// Scenario: 消した滞在と吸収された滞在は一覧に出ない
#[tokio::test]
async fn stays_day_api_deleted_stay_is_not_a_move() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    put_dwell(&pool, u, "2026-06-16T10:00:00+09:00", 60, 0.0, Some(10.0)).await;
    put_dwell(
        &pool,
        u,
        "2026-06-16T11:01:00+09:00",
        30,
        5_000.0,
        Some(10.0),
    )
    .await;
    rebuild(&pool, u, "2026-06-16").await;
    let first = live(&pool, u).await[0].0;
    user_deletes(&pool, first, Some("user")).await;
    let v = day(&pool, u, "2026-06-16", "2026-09-01T00:00:00Z").await;
    let hole = (
        t("2026-06-16T10:00:00+09:00"),
        t("2026-06-16T11:00:00+09:00"),
    );
    assert!(
        !v.entries
            .iter()
            .any(|e| e.kind == EntryKind::Move && e.start < hole.1 && e.end > hole.0),
        "本人が消した滞在の時間が移動として出ている: {v:#?}"
    );
    assert!(!v.entries.iter().any(|e| e.id == Some(first)));
}
