// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの除く範囲と消した滞在の時間が、終わりの時刻を列から先に読むこと（ST41 / design D11）。
//!
//! 配置と読み方は `export_excluded_tests` の `World` と `kept` / `kept_versions` に、
//! 書き出しの行の読み方は `export_file_tests` に合わせる。記録と消す滞在は `testdb::unique_day()` の日に置く。
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use serde_json::{json, Value};
use uuid::Uuid;

use crate::export_excluded_tests::World;
use crate::export_file_tests::{export_once, scan};
use crate::testdb;

/// 終わりの列（`end_tz` は +09:00 / Asia/Tokyo）を持つ記録を直に置く。
async fn put_ended(w: &World, source: &str, at: &str, end: Option<&str>, payload: Value) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, end_time, end_tz_offset_min, end_tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'pc',$4,$5::timestamptz,540,'Asia/Tokyo',
                 $6::timestamptz, CASE WHEN $6::text IS NULL THEN NULL ELSE 540 END,
                 CASE WHEN $6::text IS NULL THEN NULL ELSE 'Asia/Tokyo' END,
                 1,$7,'{}',$8)",
    )
    .bind(id)
    .bind(w.user)
    .bind(source)
    .bind(if source == "s01-stay" {
        "derived"
    } else {
        "collected"
    })
    .bind(at)
    .bind(end)
    .bind(Uuid::new_v4().to_string())
    .bind(payload)
    .execute(&w.pool)
    .await
    .unwrap();
    id
}

/// 前の版を終わりの列つきで置く。
async fn put_ended_version(
    w: &World,
    event: Uuid,
    no: i32,
    at: &str,
    end: &str,
    payload: Value,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event_version
           (id, event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
            end_time, end_tz_offset_min, end_tz_id)
         SELECT $1, e.id, e.user_id, e.logical_source, $2, $3::timestamptz, $4, '{}', $5,
                $6::timestamptz, 540, 'Asia/Tokyo'
           FROM core.event e WHERE e.id = $7",
    )
    .bind(id)
    .bind(no)
    .bind(at)
    .bind(Uuid::new_v4().to_string())
    .bind(payload)
    .bind(end)
    .bind(event)
    .execute(&w.pool)
    .await
    .unwrap();
    id
}

/// 緯度経度を持つとみなされ、点の規則も削除の印の連鎖も無いソース。
async fn plain_source(w: &World) -> String {
    testdb::source_of_kind(&w.pool, "endcol", "none").await
}

fn set(ids: &[Uuid]) -> BTreeSet<Uuid> {
    ids.iter().copied().collect()
}

// Scenario: 終わりを列に持つ緯度経度つきの記録は、消した時間と重なれば出ない
#[tokio::test]
async fn st41_export_end_column_span_is_excluded() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let s = plain_source(&w).await;
    let crossing = put_ended(
        &w,
        &s,
        &w.at("00:30"),
        Some(&w.at("03:00")),
        json!({"lat": 35.0}),
    )
    .await;
    // 同じソースの後続（終わりの列で縛る）。列を読まないと crossing は 00:40 で終わり、消した時間と重ならず残る
    let after = put_ended(
        &w,
        &s,
        &w.at("00:40"),
        Some(&w.at("00:50")),
        json!({"lat": 35.0}),
    )
    .await;
    assert_eq!(w.kept(&[crossing, after]).await, set(&[after]));
}

// Scenario: 列の終わりは内容の終わりより先に読まれる
#[tokio::test]
async fn st41_export_end_column_wins_over_payload() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let s = plain_source(&w).await;
    let column_reaches = put_ended(
        &w,
        &s,
        &w.at("00:10"),
        Some(&w.at("01:30")),
        json!({"end": w.at("00:20"), "lat": 35.0}),
    )
    .await;
    let payload_reaches = put_ended(
        &w,
        &s,
        &w.at("00:11"),
        Some(&w.at("00:20")),
        json!({"end": w.at("01:30"), "lat": 35.0}),
    )
    .await;
    assert_eq!(
        w.kept(&[column_reaches, payload_reaches]).await,
        set(&[payload_reaches])
    );
}

// Scenario: 列の終わりが記録の時刻より前なら、内容の終わりから読む
#[tokio::test]
async fn st41_export_reversed_end_column_falls_back_to_payload() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let s = plain_source(&w).await;
    let id = put_ended(
        &w,
        &s,
        &w.at("00:30"),
        Some(&w.at("00:10")),
        json!({"end": w.at("01:30"), "lat": 35.0}),
    )
    .await;
    assert!(w.kept(&[id]).await.is_empty());
}

// Scenario: 列の終わりが記録の時刻と同じなら、内容の終わりから読む
#[tokio::test]
async fn st41_export_end_column_equal_to_start_falls_back_to_payload() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let s = plain_source(&w).await;
    let id = put_ended(
        &w,
        &s,
        &w.at("00:30"),
        Some(&w.at("00:30")),
        json!({"end": w.at("01:30"), "lat": 35.0}),
    )
    .await;
    assert!(w.kept(&[id]).await.is_empty());
}

// Scenario: 前の版の終わりの列も幅として読まれる
#[tokio::test]
async fn st41_export_version_end_column_is_read() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let s = plain_source(&w).await;
    // 生きている版の幅は終わりの列で縛る（無いと「いままで」に伸び、ほかの日の消した滞在と重なる）
    let live = put_ended(
        &w,
        &s,
        &w.at("03:00"),
        Some(&w.at("03:30")),
        json!({"lat": 35.0}),
    )
    .await;
    let reaching = put_ended_version(
        &w,
        live,
        1,
        &w.at("00:30"),
        &w.at("01:30"),
        json!({"lat": 35.0}),
    )
    .await;
    // 終わりの列が消した時間より前の版は、次の記録まで（いま）に伸ばされず出る
    let before = put_ended_version(
        &w,
        live,
        2,
        &w.at("00:10"),
        &w.at("00:20"),
        json!({"lat": 35.0}),
    )
    .await;
    assert_eq!(w.kept(&[live]).await, set(&[live]));
    assert_eq!(w.kept_versions(&[reaching, before]).await, set(&[before]));
}

// Scenario: 終わりを列に持つ消した滞在の時間も、消した時間に数える
#[tokio::test]
async fn st41_export_erased_stay_end_column_is_read() {
    let w = World::new().await;
    let stay = put_ended(
        &w,
        "s01-stay",
        &w.at("00:00"),
        Some(&w.at("02:00")),
        json!({"lat": 35.0, "lon": 139.0}),
    )
    .await;
    w.mark_deleted(stay, "user").await;
    let s = plain_source(&w).await;
    // 幅を終わりの列（01:40）で縛る。無いと「いま」に伸び、ほかの日の消した滞在と重なって通ってしまう
    let inside = put_ended(
        &w,
        &s,
        &w.at("01:30"),
        Some(&w.at("01:40")),
        json!({"lat": 35.0}),
    )
    .await;
    assert!(w.kept(&[inside]).await.is_empty());
}

// Scenario: 書き出しの行は終わりの 3 つの列を持つ
#[tokio::test]
async fn st41_export_rows_have_end_columns() {
    let w = World::new().await;
    // 緯度経度を持たない点のソース（ほかの日の消した滞在に巻き込まれない）
    let s = "c01-clock";
    let with_end = put_ended(
        &w,
        &s,
        &w.at("05:00"),
        Some(&w.at("06:00")),
        json!({"v": 1}),
    )
    .await;
    let without = put_ended(&w, s, &w.at("07:00"), None, json!({"v": 2})).await;
    let version = put_ended_version(
        &w,
        with_end,
        1,
        &w.at("05:00"),
        &w.at("05:30"),
        json!({"v": 0}),
    )
    .await;
    let path = export_once(&w.pool).await;

    let row = |rows: Vec<Value>| {
        assert_eq!(rows.len(), 1);
        rows.into_iter().next().unwrap()
    };
    let ended = row(scan(&path, "events.jsonl", &with_end.to_string()));
    assert_eq!(
        ended["end_time"],
        json!(format!("{}T06:00:00.000000Z", w.day))
    );
    assert_eq!(ended["end_tz_offset_min"], json!(540));
    assert_eq!(ended["end_tz_id"], json!("Asia/Tokyo"));
    let open = row(scan(&path, "events.jsonl", &without.to_string()));
    for k in ["end_time", "end_tz_offset_min", "end_tz_id"] {
        assert_eq!(open[k], Value::Null, "{k}");
    }
    let old = row(scan(&path, "event_versions.jsonl", &version.to_string()));
    assert_eq!(
        old["end_time"],
        json!(format!("{}T05:30:00.000000Z", w.day))
    );
    assert_eq!(old["end_tz_offset_min"], json!(540));
    assert_eq!(old["end_tz_id"], json!("Asia/Tokyo"));
}
