// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの重なりの判定から、候補の行 × 消した時間の入れ子を取る change（CH04 / design D9・D11）の試験。
//!
//! 試験ごとの database（`testdb::isolated()`）で走らせる。他の試験の消した滞在が混ざらず、読んだ量を測れる。
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::export::{self, OVERLAP_CONDITION};
use crate::export_window_tests::{erased_stay, put_as, put_version, MAP};
use crate::testdb;

/// CH02 の最後の重なりの条件（足す前の形。design D11）。
const CH02_CONDITION: &str = "EXISTS (SELECT 1 FROM erased_spans s WHERE s.start_at <= w.width_end AND s.end_at >= w.event_time)";

/// 今の断片の最後の重なりの条件を CH02 の形に置き換えた CTE の並び。置き換えは記録の側と前の版の側の 2 か所。
fn ch02_ctes(snapshot_at: &str) -> String {
    let ctes = export::excluded_sql(snapshot_at).ctes().to_string();
    assert_eq!(
        ctes.matches(OVERLAP_CONDITION).count(),
        2,
        "重なりの条件が記録の側と前の版の側の 2 か所に無い"
    );
    ctes.replace(OVERLAP_CONDITION, CH02_CONDITION)
}

fn ids_of(rows: Vec<sqlx::postgres::PgRow>) -> BTreeSet<Uuid> {
    rows.iter().map(|r| r.get::<Uuid, _>("id")).collect()
}

/// `ctes` が除く記録の id と前の版の id。
async fn excluded_by(
    conn: &mut sqlx::PgConnection,
    ctes: &str,
) -> (BTreeSet<Uuid>, BTreeSet<Uuid>) {
    let mut out = Vec::new();
    for cte in [export::EXCLUDED_EVENT, export::EXCLUDED_VERSION] {
        let rows = sqlx::query(&format!("WITH {ctes} SELECT id FROM {cte}"))
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        out.push(ids_of(rows));
    }
    let versions = out.pop().unwrap();
    (out.pop().unwrap(), versions)
}

fn at(hhmm: &str) -> String {
    format!("2000-01-01T{hhmm}:00Z")
}

/// 写しの時点の式（消した時間の前・中・後の固定の時刻と `now()`）。
const SNAPSHOTS: [&str; 4] = [
    "now()",
    "'2000-01-01T00:30:00Z'::timestamptz",
    "'2000-01-01T06:30:00Z'::timestamptz",
    "'2000-01-01T23:00:00Z'::timestamptz",
];

/// 新旧を、同じトランザクション・同じ写しの時点の式で走らせて、除く記録と前の版の id が一致することを見る。
/// 除く側と残る側の両方が空でないことも返す。
async fn assert_same_as_ch02(pool: &sqlx::PgPool) -> (usize, usize) {
    let mut seen = (0, 0);
    for t in SNAPSHOTS {
        let mut tx = pool.begin().await.unwrap();
        let new = excluded_by(&mut tx, export::excluded_sql(t).ctes()).await;
        let old = excluded_by(&mut tx, &ch02_ctes(t)).await;
        tx.rollback().await.unwrap();
        assert_eq!(new, old, "t = {t}: 除く記録か前の版が足す前と違う");
        if t == "now()" {
            seen = (new.0.len(), new.1.len());
        }
    }
    seen
}

#[tokio::test]
async fn ch04_overlap_matches_ch02() {
    // Scenario: 直した判定は、足す前の判定と同じ記録と前の版を除く
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let u = Uuid::new_v4();
    let none = json!({});

    // 点の記録（幅は時刻 1 点）。消した滞在 A（01:00〜02:00）の端に触れる・中・外れる・1 µs の差
    for t in [
        "2000-01-01T00:59:59.999999Z",
        "2000-01-01T01:00:00Z",
        "2000-01-01T01:30:00Z",
        "2000-01-01T02:00:00Z",
        "2000-01-01T02:00:00.000001Z",
        "2000-01-01T12:00:00Z",
    ] {
        put_as(&pool, u, "c01-location", Some("p"), t, none.clone(), "{}").await;
    }
    // 終わりを持たない記録（次の記録まで・写しの時点まで）と座標のない記録
    let w1 = put_as(
        &pool,
        u,
        "c02-window",
        Some("pc"),
        &at("00:10"),
        none.clone(),
        MAP,
    )
    .await;
    put_as(
        &pool,
        u,
        "c02-window",
        Some("pc"),
        &at("00:40"),
        none.clone(),
        MAP,
    )
    .await;
    put_as(
        &pool,
        u,
        "c02-window",
        Some("pc"),
        &at("02:30"),
        none.clone(),
        "plain",
    )
    .await;
    put_as(
        &pool,
        u,
        "c02-window",
        Some("pc"),
        &at("23:30"),
        none.clone(),
        MAP,
    )
    .await;
    // 終わりを持つ記録（始まりより前の終わりも）
    for (start, end) in [("00:30", "01:00"), ("02:00", "03:00"), ("04:00", "03:00")] {
        put_as(
            &pool,
            u,
            "c02-window",
            Some("e"),
            &at(start),
            json!({"range_end": at(end)}),
            MAP,
        )
        .await;
    }
    // 消した滞在の前の版の広い時間に届く幅と、前の版の行
    let wide = put_as(
        &pool,
        u,
        "c02-window",
        Some("wide"),
        &at("03:20"),
        none.clone(),
        MAP,
    )
    .await;
    put_as(
        &pool,
        u,
        "c02-window",
        Some("wide"),
        &at("03:35"),
        none.clone(),
        "plain",
    )
    .await;
    put_version(&pool, w1, u, 1, &at("00:20"), json!({"lat": 35.0})).await;
    put_version(&pool, w1, u, 2, &at("01:30"), json!({"lat": 35.0})).await;
    put_version(&pool, wide, u, 1, &at("03:00"), json!({"lat": 35.0})).await;

    // 消した滞在 0 件
    assert_same_as_ch02(&pool).await;

    // 消した滞在: A、A に重なる B、A に端だけ触れる C、終わりが始まりより前の D（前の版も）、
    // 内容に end を持たない E、end を持たない前の版を持つ F（前の版の広い時間 03:30〜04:30）
    erased_stay(&pool, u, &at("01:00"), &at("02:00"), &[]).await;
    erased_stay(&pool, u, &at("01:30"), &at("02:30"), &[]).await;
    erased_stay(&pool, u, &at("02:30"), &at("03:00"), &[]).await;
    erased_stay(
        &pool,
        u,
        &at("10:00"),
        &at("09:00"),
        &[(&at("10:30"), &at("10:00"))],
    )
    .await;
    let e = put_as(
        &pool,
        u,
        "s01-stay",
        Some("server"),
        &at("12:00"),
        json!({"lat": 35.0}),
        "{}",
    )
    .await;
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'user' WHERE id = $1")
        .bind(e)
        .execute(&pool)
        .await
        .unwrap();
    let f = erased_stay(
        &pool,
        u,
        &at("20:00"),
        &at("20:30"),
        &[(&at("03:30"), &at("04:30"))],
    )
    .await;
    put_version(&pool, f, u, 2, &at("05:00"), json!({"lat": 35.0})).await;

    let (events, versions) = assert_same_as_ch02(&pool).await;
    assert!(
        events > 0 && versions > 0,
        "配置が効いていない: {events} {versions}"
    );
}

// ---- 消した時間を読む量（design D9・tasks 8.3）----

/// 消した時間を読む節（`erased_spans` と、そこから作る CTE）の名前。
const ERASED_CTES: [&str; 3] = ["erased_spans", "erased_range", "erased_odd"];

/// 計画の JSON の、消した時間を読む節（`CTE Scan` で `CTE Name` が `ERASED_CTES`）が読んだ行の数の和。
/// 節ごとに `(Actual Rows + Rows Removed by Filter) × Actual Loops`（CH02 の `rows_read` と同じ式）。
/// `EXISTS` が SubPlan の形でも、捨てた行は `Rows Removed by Filter` に入る。
fn erased_rows_read(plan: &Value) -> u64 {
    let node = match plan {
        Value::Array(items) => return items.iter().map(erased_rows_read).sum(),
        Value::Object(o) => match o.get("Plan") {
            Some(root) => return erased_rows_read(root),
            None => o,
        },
        _ => return 0,
    };
    let count = |key: &str| node.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    let scans_erased = node.get("Node Type").and_then(Value::as_str) == Some("CTE Scan")
        && node
            .get("CTE Name")
            .and_then(Value::as_str)
            .is_some_and(|n| ERASED_CTES.contains(&n));
    let own = if scans_erased {
        ((count("Actual Rows") + count("Rows Removed by Filter")) * count("Actual Loops")) as u64
    } else {
        0
    };
    let children: u64 = node
        .get("Plans")
        .and_then(Value::as_array)
        .map(|ps| ps.iter().map(erased_rows_read).sum())
        .unwrap_or(0);
    own + children
}

#[test]
fn ch04_overlap_reads_counts_only_the_erased_cte_scans() {
    let plan = json!([{"Plan": {
        "Node Type": "Result", "Actual Rows": 1, "Actual Loops": 1,
        "Plans": [
            {"Node Type": "CTE Scan", "CTE Name": "erased_spans", "Actual Rows": 3,
             "Rows Removed by Filter": 7, "Actual Loops": 4},
            {"Node Type": "CTE Scan", "CTE Name": "erased_range", "Actual Rows": 1, "Actual Loops": 2},
            {"Node Type": "CTE Scan", "CTE Name": "span_next", "Actual Rows": 999, "Actual Loops": 1},
            {"Node Type": "Seq Scan", "Relation Name": "event", "Actual Rows": 999, "Actual Loops": 1},
            {"Node Type": "CTE Scan", "CTE Name": "erased_odd", "Actual Rows": 5, "Actual Loops": 0}
        ]
    }}]);
    // (3 + 7) × 4 + 1 × 2 + 0
    assert_eq!(erased_rows_read(&plan), 42);
}

/// 消した滞在 `m` 件（1 時間ずつ別の日）と記録 `n` 件（緯度経度つき。半分は消した時間の中、半分はどの消した時間にも重ならない）。
async fn seed_scale(pool: &sqlx::PgPool, n: i64, m: i64) {
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         SELECT gen_random_uuid(), $1, 's01-stay', 'server', 'derived',
                '2000-01-01T10:00:00Z'::timestamptz + i * interval '2 days',
                540, 'Asia/Tokyo', 1, gen_random_uuid()::text, '{}',
                jsonb_build_object('end', to_char(
                  ('2000-01-01T11:00:00Z'::timestamptz + i * interval '2 days') AT TIME ZONE 'UTC',
                  'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), 'lat', 35.0, 'lon', 139.0)
           FROM generate_series(0, $2::int - 1) i",
    )
    .bind(user)
    .bind(m)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'user' WHERE logical_source = 's01-stay'")
        .execute(pool)
        .await
        .unwrap();
    // 偶数番は消した時間の中（10:30）、奇数番は次の日の 10:30（隣の消した時間との間。最後の消した時間より後に出ないよう m - 1 で巻く）
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         SELECT gen_random_uuid(), $1, 'c01-location', 'dev-' || i, 'collected',
                CASE WHEN i % 2 = 0
                     THEN '2000-01-01T10:30:00Z'::timestamptz + ((i / 2) % $3::int) * interval '2 days'
                     ELSE '2000-01-02T10:30:00Z'::timestamptz + ((i / 2) % ($3::int - 1)) * interval '2 days'
                END,
                540, 'Asia/Tokyo', 1, gen_random_uuid()::text, '{}', '{\"lat\": 35.0, \"lon\": 139.0}'::jsonb
           FROM generate_series(0, $2::int - 1) i",
    )
    .bind(user)
    .bind(n)
    .bind(m)
    .execute(pool)
    .await
    .unwrap();
}

/// `ctes` で除く範囲を求める文の `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)`。
async fn explain_ctes(pool: &sqlx::PgPool, ctes: &str) -> Value {
    let q = format!(
        "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
         WITH {ctes} SELECT (SELECT count(*) FROM {}), (SELECT count(*) FROM {})",
        export::EXCLUDED_EVENT,
        export::EXCLUDED_VERSION
    );
    sqlx::query_scalar(&q).fetch_one(pool).await.unwrap()
}

/// `(n, m)` を置いた database で、(今の断片, 足す前の形) の消した時間を読む量。
async fn scale_erased_reads(n: i64, m: i64) -> (u64, u64) {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    seed_scale(&pool, n, m).await;
    sqlx::query("ANALYZE").execute(&pool).await.unwrap();
    let new = erased_rows_read(&explain_ctes(&pool, export::excluded_sql("now()").ctes()).await);
    let old = erased_rows_read(&explain_ctes(&pool, &ch02_ctes("now()")).await);
    (new, old)
}

#[tokio::test]
async fn ch04_overlap_reads_erased_spans_linearly() {
    // Scenario: 記録と消した滞在を一緒に 4 倍にしても、消した時間を読む量は 4 倍の桁に収まる
    const LIMIT: u64 = 8;
    let (new_1, old_1) = scale_erased_reads(1_000, 10).await;
    let (new_4, old_4) = scale_erased_reads(4_000, 40).await;
    eprintln!("消した時間を読む量: 今 {new_1} → {new_4} / 足す前 {old_1} → {old_4}");
    assert!(
        old_1 > 0,
        "足す前の形で読んだ量が取れていない（測り方が節を捕まえていない）"
    );
    assert!(
        old_4 >= LIMIT * old_1,
        "足す前の形が 4 倍で {LIMIT} 倍未満（測り方が入れ子を捕まえていない）: {old_1} → {old_4}"
    );
    assert!(new_1 > 0, "今の形で読んだ量が取れていない");
    assert!(
        new_4 < LIMIT * new_1,
        "消した時間を読む量が 4 倍で {LIMIT} 倍以上: {new_1} → {new_4}"
    );
}
