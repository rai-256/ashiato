// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しから除く範囲（SQL の断片）と、書き出す前の件数と見込み（ST33 / design D5 / D5a / D5b / D18）。
//!
//! 断片を通した**件数**（そのテストの利用者の行）で形を固定する。ファイルでの確かめは 5.4。
//! 消した滞在の時間は全テストで共有されるので、記録と消す滞在は `testdb::unique_day()` の日に置く（R27）。
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::export;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

pub(crate) struct World {
    pub(crate) pool: sqlx::PgPool,
    pub(crate) day: chrono::NaiveDate,
    pub(crate) user: Uuid,
}

impl World {
    pub(crate) async fn new() -> Self {
        Self {
            pool: testdb::pool().await,
            day: testdb::unique_day(),
            user: testdb::user(),
        }
    }

    /// その日の `HH:MM`（UTC）。
    pub(crate) fn at(&self, hhmm: &str) -> String {
        format!("{}T{hhmm}:00Z", self.day)
    }

    /// 前の日の `HH:MM`。
    pub(crate) fn before(&self, hhmm: &str) -> String {
        format!("{}T{hhmm}:00Z", self.day.pred_opt().unwrap())
    }

    pub(crate) async fn put_as(
        &self,
        user: Uuid,
        source: &str,
        device: &str,
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
        .execute(&self.pool)
        .await
        .unwrap();
        id
    }

    pub(crate) async fn put(&self, source: &str, at: &str, payload: Value, raw: &str) -> Uuid {
        self.put_as(self.user, source, "pc", at, payload, raw).await
    }

    /// 前の版を 1 つ足す（番号は呼ぶ順）。
    pub(crate) async fn put_version(&self, event: Uuid, no: i32, at: &str, payload: Value) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO core.event_version
               (id, event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
             SELECT $1, e.id, e.user_id, e.logical_source, $2, $3::timestamptz, $4, '{}', $5
               FROM core.event e WHERE e.id = $6",
        )
        .bind(id)
        .bind(no)
        .bind(at)
        .bind(Uuid::new_v4().to_string())
        .bind(payload)
        .bind(event)
        .execute(&self.pool)
        .await
        .unwrap();
        id
    }

    /// 滞在を 1 件置く（`end` と緯度経度を持つ）。
    pub(crate) async fn stay(&self, start: &str, end: &str) -> Uuid {
        self.put_as(
            self.user,
            "s01-stay",
            "server",
            start,
            json!({"end": end, "lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await
    }

    /// 本人が消した滞在を置く。`versions` は前の版の `(始まり, 終わり)`。
    pub(crate) async fn erased_stay(
        &self,
        start: &str,
        end: &str,
        versions: &[(&str, &str)],
    ) -> Uuid {
        let id = self.stay(start, end).await;
        for (no, (s, e)) in versions.iter().enumerate() {
            self.put_version(
                id,
                no as i32 + 1,
                s,
                json!({"end": e, "lat": 35.0, "lon": 139.0}),
            )
            .await;
        }
        self.mark_deleted(id, "user").await;
        id
    }

    pub(crate) async fn mark_deleted(&self, id: Uuid, by: &str) {
        sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = $2 WHERE id = $1")
            .bind(id)
            .bind(by)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    /// 断片を通して残った記録の id（`ids` のうち）。
    async fn kept(&self, ids: &[Uuid]) -> BTreeSet<Uuid> {
        let x = export::excluded_sql("now()");
        let sql = format!(
            "WITH {} SELECT e.id FROM core.event_live e WHERE e.id = ANY($1) AND {}",
            x.ctes(),
            x.event_kept("e")
        );
        let got: Vec<(Uuid,)> = sqlx::query_as(&sql)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
            .unwrap();
        got.into_iter().map(|(i,)| i).collect()
    }

    /// 断片を通して残った前の版の id（`ids` のうち）。
    async fn kept_versions(&self, ids: &[Uuid]) -> BTreeSet<Uuid> {
        let x = export::excluded_sql("now()");
        let sql = format!(
            "WITH {} SELECT v.id FROM core.event_version_live v WHERE v.id = ANY($1) AND {}",
            x.ctes(),
            x.version_kept("v")
        );
        let got: Vec<(Uuid,)> = sqlx::query_as(&sql)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
            .unwrap();
        got.into_iter().map(|(i,)| i).collect()
    }

    /// `ids` がどれも残らない。
    async fn assert_all_excluded(&self, ids: &[Uuid]) {
        let kept = self.kept(ids).await;
        assert!(kept.is_empty(), "除かれるはずが残った: {} 件", kept.len());
    }
}

const MAP: &str = "https://www.google.com/maps/@35.6812,139.7671,15z";

/// 登録簿の全ソースが、緯度経度を「持つ / 持たない」のちょうど一方と、幅の読み方（点 / 次の記録まで）の
/// ちょうど一方に**名前で**置かれている（R7 / handoff C1）。SQL は一覧に無いソースを厳しい側に倒すが、
/// それに頼って新しいソースが黙って通らないよう、足した Story がここで名前つきで落ちる。
///
/// 読むのは**全移行だけを当てたまっさらな DB** の `core.source`（共有の DB には他のテストや台本が足した行が混ざる）。
/// 書庫が製品ごとに実行時に足す `c03-myactivity-<製品>` は前置きで置き、その形もここで確かめる。
#[tokio::test]
async fn export_excluded_sql_every_registered_source_is_classified_by_name() {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    let sources = async {
        for (label, sql) in crate::MIGRATIONS {
            sqlx::raw_sql(sql)
                .execute(&fresh)
                .await
                .map_err(|e| format!("{label}: {e}"))?;
        }
        sqlx::query_scalar::<_, String>("SELECT logical_source FROM core.source ORDER BY 1")
            .fetch_all(&fresh)
            .await
            .map_err(|e| e.to_string())
    }
    .await;
    drop.await;
    let mut sources = sources.unwrap();
    assert!(sources.len() >= 20, "登録簿が読めていない: {sources:?}");
    sources.push(format!("{}youtube", export::ARCHIVE_PRODUCT_PREFIX));

    let points: Vec<&str> = export::SPAN_RULES.iter().map(|(s, _)| *s).collect();
    let mut bad = Vec::new();
    for s in &sources {
        let coord = (
            export::listed(&export::NO_COORD_SOURCES, s),
            export::listed(export::COORD_SOURCES, s),
        );
        if coord.0 == coord.1 {
            bad.push(format!(
                "{s}: 緯度経度（持たない {} / 持つ {}）",
                coord.0, coord.1
            ));
        }
        let span = (
            export::listed(&points, s),
            export::listed(export::SPAN_NEXT_SOURCES, s),
        );
        if span.0 == span.1 {
            bad.push(format!(
                "{s}: 幅（点 {} / 次の記録まで {}）",
                span.0, span.1
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "一覧のちょうど一方に置かれていないソース: {bad:#?}"
    );
}

#[tokio::test]
async fn export_excluded_sql_marked_records_are_not_live() {
    let w = World::new().await;
    let stay = w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let pos = w
        .put(
            "c01-location",
            &w.at("01:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    w.mark_deleted(pos, "stay:cascade").await;
    // 消した滞在の行も連鎖で印が付いた位置の行も、残らない
    assert!(w.kept(&[stay, pos]).await.is_empty());
}

#[tokio::test]
async fn export_excluded_sql_unmarked_photo_location_inside_is_excluded() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let photo = w
        .put(
            "c01-photo",
            &w.at("01:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    w.assert_all_excluded(&[photo]).await;
}

#[tokio::test]
async fn export_excluded_sql_wide_record_crossing_the_erased_time_is_excluded() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let source = testdb::source(&w.pool, "wide", 3600).await;
    let crossing = w
        .put(
            &source,
            &w.at("00:30"),
            json!({"end_time": w.at("03:00"), "lat": 35.0}),
            "{}",
        )
        .await;
    // 終わりが消した時間の前なら出る
    let before = w
        .put(
            &source,
            &w.at("00:10"),
            json!({"end_time": w.at("00:20"), "lat": 35.0}),
            "{}",
        )
        .await;
    assert_eq!(w.kept(&[crossing, before]).await, BTreeSet::from([before]));
}

#[tokio::test]
async fn export_excluded_sql_end_time_in_epoch_millis_is_read() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let source = testdb::source(&w.pool, "millis", 3600).await;
    let end_ms = w
        .day
        .and_hms_opt(3, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    let crossing = w
        .put(
            &source,
            &w.at("00:30"),
            json!({"end_time": end_ms.to_string()}),
            "{}",
        )
        .await;
    w.assert_all_excluded(&[crossing]).await;
}

#[tokio::test]
async fn export_excluded_sql_duration_gives_the_width() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    // 20 分（µs）。00:50 始まりは 01:10 まで重なり、00:20 始まりは 00:40 で終わる
    let visit = w
        .put(
            "c02-browser-history",
            &w.at("00:50"),
            json!({"visit_duration_us": 1_200_000_000i64}),
            MAP,
        )
        .await;
    let early = w
        .put(
            "c02-browser-history",
            &w.at("00:20"),
            json!({"visit_duration_us": 1_200_000_000i64}),
            MAP,
        )
        .await;
    assert_eq!(w.kept(&[visit, early]).await, BTreeSet::from([early]));
}

#[tokio::test]
async fn export_excluded_sql_record_without_end_runs_to_the_next_record() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let a = w.put("c02-window", &w.at("00:10"), json!({}), MAP).await;
    let b = w.put("c02-window", &w.at("00:40"), json!({}), MAP).await;
    let c = w
        .put("c02-window", &w.at("02:30"), json!({}), "plain")
        .await;
    // 00:40 の記録は次の 02:30 まで続くので消した時間に重なる。00:10 は 00:40 まで
    assert_eq!(w.kept(&[a, b, c]).await, BTreeSet::from([a, c]));
}

#[tokio::test]
async fn export_excluded_sql_wider_earlier_version_of_the_erased_stay_counts() {
    let w = World::new().await;
    // 消した滞在は DB に残り続ける。広い前の版が他のテストの帯（00:00〜05:00 台）に被らないよう昼に置く
    w.erased_stay(
        &w.at("13:00"),
        &w.at("14:00"),
        &[(&w.at("12:00"), &w.at("14:00"))],
    )
    .await;
    let photo = w
        .put(
            "c01-photo",
            &w.at("12:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    w.assert_all_excluded(&[photo]).await;
}

#[tokio::test]
async fn export_excluded_sql_other_users_record_is_excluded_too() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let other = testdb::user();
    let pos = w
        .put_as(
            other,
            "c01-photo",
            "phone",
            &w.at("01:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    w.assert_all_excluded(&[pos]).await;
}

#[tokio::test]
async fn export_excluded_sql_earlier_version_of_a_live_stay_is_excluded() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    // いまの版は前の日の 23:00〜00:50 で生きているが、前の版は 03:00 まで広かった
    let live = w.stay(&w.before("23:00"), &w.at("00:50")).await;
    let old = w
        .put_version(
            live,
            1,
            &w.before("23:00"),
            json!({"end": w.at("03:00"), "lat": 35.0, "lon": 139.0}),
        )
        .await;
    assert_eq!(w.kept(&[live]).await, BTreeSet::from([live]));
    assert!(w.kept_versions(&[old]).await.is_empty());
}

#[tokio::test]
async fn export_excluded_sql_record_without_coordinates_is_kept() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let win = w
        .put(
            "c02-window",
            &w.at("01:30"),
            json!({"title": "memo"}),
            "memo",
        )
        .await;
    assert_eq!(w.kept(&[win]).await, BTreeSet::from([win]));
}

#[tokio::test]
async fn export_excluded_sql_location_outside_the_erased_time_is_kept() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let pos = w
        .put(
            "c01-location",
            &w.at("02:01"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    assert_eq!(w.kept(&[pos]).await, BTreeSet::from([pos]));
}

#[tokio::test]
async fn export_excluded_sql_source_not_known_to_lack_coordinates_is_excluded() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let source = testdb::source(&w.pool, "newsource", 3600).await;
    let rec = w
        .put(&source, &w.at("01:30"), json!({"note": "x"}), "x")
        .await;
    w.assert_all_excluded(&[rec]).await;
}

#[tokio::test]
async fn export_excluded_sql_payload_keys_count_even_for_sources_without_coordinates() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let a = w
        .put("c02-window", &w.at("01:30"), json!({"lng": 139.0}), "x")
        .await;
    let b = w
        .put(
            "c02-window",
            &w.at("01:31"),
            json!({"a": {"b": {"latitude": 35.0}}}),
            "x",
        )
        .await;
    let c = w
        .put(
            "c02-window",
            &w.at("01:32"),
            json!({"LatitudeE7": 356812000}),
            "x",
        )
        .await;
    // 名前が違う（`latlng`）ものは持たないとみなす
    let d = w
        .put("c02-window", &w.at("01:33"), json!({"latlng": 1}), "x")
        .await;
    assert_eq!(w.kept(&[a, b, c, d]).await, BTreeSet::from([d]));
}

#[tokio::test]
async fn export_excluded_sql_map_urls_in_window_raw_are_excluded() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let a = w.put("c02-window", &w.at("01:30"), json!({}), MAP).await;
    let b = w
        .put(
            "c02-window",
            &w.at("01:31"),
            json!({}),
            "https://www.google.com/maps/place/x/data=!3d35.6812!4d139.7671",
        )
        .await;
    w.assert_all_excluded(&[a, b]).await;
}

#[tokio::test]
async fn export_excluded_sql_short_encoded_dms_pluscode_forms_are_excluded() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let raws = [
        "https://maps.google.com/?ll=35.68,139.76",
        "https://www.google.com/maps?q=35.68%2C139.76",
        "35°40'52.3\"N 139°46'00.2\"E",
        "MQ7X+2V 千代田区",
        "https://www.openstreetmap.org/#map=15/35.68/139.76",
        "https://example.com/?lat=35.68&lon=139.76",
        "139.7671,35.6812",
    ];
    let mut ids = Vec::new();
    for (i, raw) in raws.iter().enumerate() {
        ids.push(
            w.put(
                "c02-window",
                &w.at(&format!("01:{:02}", 30 + i)),
                json!({}),
                raw,
            )
            .await,
        );
    }
    w.assert_all_excluded(&ids).await;
}

#[tokio::test]
async fn export_excluded_sql_version_of_an_excluded_parent_is_excluded() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    // 親は消した時間の中で除かれる。前の版の時刻は外
    let parent = w
        .put(
            "c01-photo",
            &w.at("01:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    let version = w
        .put_version(
            parent,
            1,
            &w.at("10:00"),
            json!({"lat": 35.0, "lon": 139.0}),
        )
        .await;
    // 外にある親の前の版は残る
    let outside = w
        .put(
            "c01-photo",
            &w.at("05:00"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    let outside_version = w
        .put_version(
            outside,
            1,
            &w.at("05:00"),
            json!({"lat": 35.0, "lon": 139.0}),
        )
        .await;
    assert!(w.kept(&[parent]).await.is_empty());
    assert!(w.kept_versions(&[version]).await.is_empty());
    assert_eq!(w.kept(&[outside]).await, BTreeSet::from([outside]));
    assert_eq!(
        w.kept_versions(&[outside_version]).await,
        BTreeSet::from([outside_version])
    );
}

#[tokio::test]
async fn export_excluded_sql_payload_expression_does_not_fail_on_non_objects() {
    let pool = testdb::pool().await;
    let sql = format!("SELECT {}", export::coord_in_payload_sql("$1::text::jsonb"));
    let cases = [
        (r#"{"a":1,"b":{"LatitudeE7":2}}"#, true),
        (r#"[1,{"lng":2}]"#, true),
        (r#"{"a":1,"b":[{"x":1}]}"#, false),
        ("5", false),
        (r#""s""#, false),
        ("null", false),
        ("[[1],[2,3]]", false),
        (r#"{"latlng":1}"#, false),
    ];
    for (payload, want) in cases {
        let got: (bool,) = sqlx::query_as(&sql)
            .bind(payload)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(got.0, want, "{payload}");
    }
}

#[tokio::test]
async fn export_excluded_sql_raw_expression_matches_the_designed_forms_only() {
    let pool = testdb::pool().await;
    let sql = format!("SELECT {}", export::coord_in_raw_sql("$1::text"));
    let hits = [
        "@35.6812,139.7671",
        "ll=35.68,139.76",
        "q=35.68%2C139.76",
        "q=35.6+139.7",
        "35.6812°, 139.7671°",
        "geo:-33.86,151.2",
        "139.7671,35.6812",
        "#map=15/35.68/139.76",
        "cp=35.68~139.76",
        "!3d35.6812!4d139.7671",
        "!2d139.7671!3d35.6812",
        "35°40'52.3\"N 139°46'00.2\"E",
        "?lat=35.68&lon=139.76",
        "{\"latitude\": \"35.68\"}",
        "MQ7X+2V 千代田区",
    ];
    let misses = [
        "Chrome 128.0.6613.84",
        "price 1,234.50 yen",
        "v1.2 notes",
        "?id=12345",
        "2026/09/07 12:30",
    ];
    for (list, want) in [(&hits[..], true), (&misses[..], false)] {
        for raw in list {
            let got: (bool,) = sqlx::query_as(&sql)
                .bind(*raw)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(got.0, want, "{raw}");
        }
    }
}

#[tokio::test]
async fn export_excluded_sql_end_before_start_is_read_by_the_next_record_width() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let source = testdb::source(&w.pool, "backwards", 3600).await;
    // 終わりが始まりより前（読み違え）。点にすると 00:30 で消した時間と重ならない
    let broken = w
        .put(
            &source,
            &w.at("00:30"),
            json!({"end_time": w.at("00:10"), "x": 1}),
            "x",
        )
        .await;
    let next = w.put(&source, &w.at("03:00"), json!({}), "x").await;
    // 終わりの無い最後の記録は写しの時点まで続く（他のテストの消した時間にかかる）ので、03:00 の後ろに記録を置く
    w.put(&source, &w.at("23:00"), json!({}), "x").await;
    // 次の記録（03:00）までの幅で読むので重なる。03:00 の記録は次の 23:00 までで、消した時間の外
    assert_eq!(w.kept(&[broken, next]).await, BTreeSet::from([next]));
}

// ---------------------------------------------------------------------------
// GET /export/summary
// ---------------------------------------------------------------------------

fn auth() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    h
}

// Scenario: 書き出しの見込みの行の数は件数の和と一致する
#[tokio::test]
async fn export_summary_rows_equal_the_sum_of_counts() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let source = testdb::source(&w.pool, "summary", 3600).await;
    let other = testdb::user();
    // 全利用者の和。消した時間の中の 1 件は数えない。
    // 終わりを持たない最後の記録は写しの時点まで続き他のテストの消した時間にかかるので、終わりを持たせる
    for (hhmm, end) in [("03:00", "03:01"), ("04:00", "04:01")] {
        let payload = json!({"end": w.at(end)});
        w.put(&source, &w.at(hhmm), payload.clone(), "x").await;
        w.put_as(other, &source, "phone", &w.at(hhmm), payload, "x")
            .await;
    }
    w.put(&source, &w.at("01:30"), json!({"end": w.at("01:31")}), "x")
        .await;

    let app = crate::App::for_test(w.pool.clone(), TOKEN);
    let got = crate::export_summary_get(State(app), auth())
        .await
        .unwrap()
        .0;
    let mine = got
        .sources
        .iter()
        .find(|s| s.logical_source == source)
        .unwrap();
    assert_eq!(mine.count, 4);
    assert_eq!(mine.display_name, source);
    assert_eq!(
        mine.first.to_rfc3339(),
        w.at("03:00").replace('Z', "+00:00")
    );
    assert_eq!(mine.last.to_rfc3339(), w.at("04:00").replace('Z', "+00:00"));
    assert!(
        got.sources.iter().all(|s| s.count > 0),
        "件数 0 のソースは出さない"
    );
    assert_eq!(
        got.estimate.rows,
        got.sources.iter().map(|s| s.count).sum::<i64>()
    );
    assert!(got.estimate.seconds >= 1);
}

// Scenario: 資格情報の無い件数と状態の求めは断られる
#[tokio::test]
async fn export_summary_without_credentials_is_refused() {
    let app = crate::App::for_test(testdb::pool().await, TOKEN);
    let denied = crate::export_summary_get(State(app), HeaderMap::new()).await;
    assert!(matches!(denied, Err((StatusCode::UNAUTHORIZED, _))));
}

#[test]
fn export_summary_estimate_defaults_without_a_previous_export() {
    let e = export::estimate(40_000, None);
    assert_eq!(
        e,
        export::Estimate {
            rows: 40_000,
            bytes: 40_000 * 180,
            seconds: 2
        }
    );
    // 1 未満は 1
    assert_eq!(
        export::estimate(0, None),
        export::Estimate {
            rows: 0,
            bytes: 0,
            seconds: 1
        }
    );
    assert_eq!(export::estimate(10, None).seconds, 1);
}

#[test]
fn export_summary_estimate_scales_the_previous_export() {
    let prev = export::Previous {
        rows: 1_000,
        bytes: 50_000,
        seconds: 10.0,
    };
    let e = export::estimate(3_000, Some(prev));
    assert_eq!(
        e,
        export::Estimate {
            rows: 3_000,
            bytes: 150_000,
            seconds: 30
        }
    );
    // 前回が 0 行なら比が取れないので既定に戻る
    let none = export::Previous {
        rows: 0,
        bytes: 0,
        seconds: 0.0,
    };
    assert_eq!(export::estimate(20_000, Some(none)).bytes, 20_000 * 180);
}
