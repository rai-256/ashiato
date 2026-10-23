// SPDX-License-Identifier: AGPL-3.0-only
//! 取り込みの「人が書いた記録」の規則（主張・主観・場所）の拒み方を固定する（CH01 / Task 4 / design D7）。
//!
//! 3 種類は同じ規則（由来が authored・端末 id 無し・外部識別子無し）を見るが、順と地域の扱いが少しずつ違う。
//! 規則を 1 つの関数にまとめる前後で、**どの理由の種別が返るか**が変わらないことをここで見る。
//! 試験の名前の接頭辞は `ch01_authored_`。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::feelings_ingest_tests::{app, day_raw, nonce_for, valence};
use crate::testdb;
use uuid::Uuid;

const NONCE: &str = "Zm9vYmFyYmF6cXV4MTIzNDU2";
const AT: &str = "2026-10-01T02:00:00Z";

/// 取り込み口へ 1 件の項目を送った結果
async fn send(app: &App, item: serde_json::Value) -> IngestResult {
    let (_, Json(mut res)) = ingest(
        State(app.clone()),
        crate::feelings_ingest_tests::auth(),
        Json(serde_json::json!([item])),
    )
    .await
    .expect("取り込み口");
    res.pop().expect("1 件ぶんの結果")
}

fn envelope(id: Uuid, user: Uuid, source: &str, raw: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id, "user_id": user, "logical_source": source,
        "external_id": null, "device_id": null, "origin": "authored",
        "event_time": AT, "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
        "schema_version": 1, "raw": raw, "payload": {},
    })
}

/// 断られた理由の種別。受理されていたら失敗させる
fn kind_of(r: &IngestResult) -> String {
    assert!(!r.accepted, "受理されている");
    serde_json::to_value(r).unwrap()["error"]
        .as_str()
        .expect("理由の種別")
        .to_string()
}

// ---------------------------------------------------------------- 主張

fn claim_raw(id: Uuid, kind: Uuid, value: Option<&str>) -> String {
    serde_json::json!({
        "claim": id, "nonce": NONCE, "kind": kind, "value": value,
        "valid_from": { "precision": "year", "date": "2019" },
        "supersedes": null, "note": null,
    })
    .to_string()
}

async fn address_kind(app: &App, user: Uuid) -> Uuid {
    let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 15).unwrap();
    let v = attributes_store::attributes_view(&app.pool, user, today)
        .await
        .unwrap();
    v.kinds.iter().find(|k| k.name == "住所").unwrap().id
}

/// 主張を、封筒の欄を `edit` で書き換えて送る
async fn send_claim(
    app: &App,
    user: Uuid,
    raw: impl FnOnce(Uuid) -> String,
    edit: impl FnOnce(&mut serde_json::Value),
) -> IngestResult {
    let id = Uuid::new_v4();
    let mut it = envelope(id, user, attributes::SOURCE, &raw(id));
    edit(&mut it);
    send(app, it).await
}

#[tokio::test]
// Scenario: 由来も外部識別子も違う主張は由来の理由で断る
async fn ch01_authored_claim_origin_beats_external_id() {
    let app = app().await;
    let u = testdb::user();
    let kind = address_kind(&app, u).await;
    let r = send_claim(
        &app,
        u,
        |id| claim_raw(id, kind, Some("A")),
        |it| {
            it["origin"] = "collected".into();
            it["external_id"] = "ext-1".into();
        },
    )
    .await;
    assert_eq!(kind_of(&r), "claim_not_authored");
}

#[tokio::test]
// Scenario: 形が崩れて由来も違う主張は形の理由で断る
async fn ch01_authored_claim_shape_beats_origin() {
    let app = app().await;
    let u = testdb::user();
    let r = send_claim(
        &app,
        u,
        |_| "これは JSON でない".to_string(),
        |it| it["origin"] = "collected".into(),
    )
    .await;
    assert_eq!(kind_of(&r), "malformed_claim");
}

#[tokio::test]
// Scenario: 地域のずれが範囲の外の主張は断る
async fn ch01_authored_claim_out_of_range_offset_is_rejected() {
    let app = app().await;
    let u = testdb::user();
    let kind = address_kind(&app, u).await;
    // 主張の行は消せないので、格納されないことは送る前と後の件数で見る（場所の側と同じ形。review/code.md R5）
    let count = || async {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM core.event WHERE user_id = $1")
            .bind(u)
            .fetch_one(&app.pool)
            .await
            .unwrap()
    };
    let before = count().await;
    for bad in [1440, -1440] {
        let r = send_claim(
            &app,
            u,
            |id| claim_raw(id, kind, Some("A")),
            |it| it["tz_offset_min"] = bad.into(),
        )
        .await;
        assert_eq!(kind_of(&r), "malformed_claim", "{bad}");
    }
    assert_eq!(count().await, before, "断った主張が格納されている");
}

#[tokio::test]
// Scenario: 地域のずれが範囲の端の主張は受け付ける
async fn ch01_authored_claim_edge_offsets_are_accepted() {
    let app = app().await;
    let u = testdb::user();
    let kind = address_kind(&app, u).await;
    for ok in [1439, -1439, 0] {
        let r = send_claim(
            &app,
            u,
            |id| claim_raw(id, kind, Some("A")),
            |it| it["tz_offset_min"] = ok.into(),
        )
        .await;
        assert!(r.accepted, "{ok} が断られている: {:?}", r.error);
    }
}

#[tokio::test]
// Scenario: 外部識別子を持ち地域のずれも範囲の外の主張は外部識別子の理由で断る
async fn ch01_authored_claim_external_id_beats_offset() {
    let app = app().await;
    let u = testdb::user();
    let kind = address_kind(&app, u).await;
    let r = send_claim(
        &app,
        u,
        |id| claim_raw(id, kind, Some("A")),
        |it| {
            it["external_id"] = "ext-1".into();
            it["tz_offset_min"] = 1440.into();
        },
    )
    .await;
    assert_eq!(kind_of(&r), "claim_has_external_id");
}

#[tokio::test]
// Scenario: 地域のずれが範囲の外で値も空の主張は地域の理由で断る
async fn ch01_authored_claim_offset_beats_empty_value() {
    let app = app().await;
    let u = testdb::user();
    let kind = address_kind(&app, u).await;
    let r = send_claim(
        &app,
        u,
        |id| claim_raw(id, kind, Some("   ")),
        |it| it["tz_offset_min"] = 1440.into(),
    )
    .await;
    assert_eq!(kind_of(&r), "malformed_claim");
}

// ---------------------------------------------------------------- 場所

fn place_raw(id: Uuid, name: &str) -> String {
    serde_json::json!({
        "record": id, "place": Uuid::new_v4(), "nonce": NONCE,
        "field": "name", "name": name,
    })
    .to_string()
}

async fn send_place(
    app: &App,
    user: Uuid,
    raw: impl FnOnce(Uuid) -> String,
    edit: impl FnOnce(&mut serde_json::Value),
) -> IngestResult {
    let id = Uuid::new_v4();
    let mut it = envelope(id, user, places::SOURCE, &raw(id));
    edit(&mut it);
    send(app, it).await
}

#[tokio::test]
// Scenario: 端末識別子と外部識別子を持つ場所の記録は由来の理由で断る
async fn ch01_authored_place_origin_beats_external_id() {
    let app = app().await;
    let r = send_place(
        &app,
        testdb::user(),
        |id| place_raw(id, "自宅"),
        |it| {
            it["device_id"] = "dev-1".into();
            it["external_id"] = "ext-1".into();
        },
    )
    .await;
    assert_eq!(kind_of(&r), "place_record_not_authored");
}

#[tokio::test]
// Scenario: 形が崩れて端末識別子も持つ場所の記録は形の理由で断る
async fn ch01_authored_place_shape_beats_origin() {
    let app = app().await;
    let r = send_place(
        &app,
        testdb::user(),
        |_| "これは JSON でない".to_string(),
        |it| it["device_id"] = "dev-1".into(),
    )
    .await;
    assert_eq!(kind_of(&r), "malformed_place_record");
}

#[tokio::test]
// Scenario: 地域のずれが範囲の外の場所の記録は断る
async fn ch01_authored_place_out_of_range_offset_is_rejected() {
    let app = app().await;
    let u = testdb::user();
    let id = Uuid::new_v4();
    let mut it = envelope(id, u, places::SOURCE, &place_raw(id, "自宅"));
    it["tz_offset_min"] = (-1440).into();
    let r = send(&app, it).await;
    assert_eq!(kind_of(&r), "malformed_place_record");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "断った記録が格納されている");
}

#[tokio::test]
// Scenario: 外部識別子を持ち地域のずれも範囲の外の場所の記録は外部識別子の理由で断る
async fn ch01_authored_place_external_id_beats_offset() {
    let app = app().await;
    let r = send_place(
        &app,
        testdb::user(),
        |id| place_raw(id, "自宅"),
        |it| {
            it["external_id"] = "ext-1".into();
            it["tz_offset_min"] = 1440.into();
        },
    )
    .await;
    assert_eq!(kind_of(&r), "place_record_has_external_id");
}

#[tokio::test]
// Scenario: 地域のずれが範囲の外で名前も空の場所の記録は地域の理由で断る
async fn ch01_authored_place_offset_beats_empty_name() {
    let app = app().await;
    let r = send_place(
        &app,
        testdb::user(),
        |id| place_raw(id, "   "),
        |it| it["tz_offset_min"] = 1440.into(),
    )
    .await;
    assert_eq!(kind_of(&r), "malformed_place_record");
}

// ---------------------------------------------------------------- 主観（順の組み合わせ。東京の +9:00 に限る）

async fn send_feeling(app: &App, edit: impl FnOnce(&mut serde_json::Value)) -> IngestResult {
    let id = Uuid::new_v4();
    let raw = day_raw(
        id,
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(1),
        None,
        &nonce_for(id),
    );
    let mut it = envelope(id, testdb::user(), feelings::SOURCE, &raw);
    edit(&mut it);
    send(app, it).await
}

#[tokio::test]
async fn ch01_authored_feeling_shape_beats_origin() {
    let app = app().await;
    let id = Uuid::new_v4();
    let mut it = envelope(id, testdb::user(), feelings::SOURCE, "これは JSON でない");
    it["origin"] = "collected".into();
    assert_eq!(kind_of(&send(&app, it).await), "malformed_feeling");
}

#[tokio::test]
async fn ch01_authored_feeling_offset_beats_origin() {
    let app = app().await;
    let r = send_feeling(&app, |it| {
        it["origin"] = "collected".into();
        it["tz_offset_min"] = 0.into();
    })
    .await;
    assert_eq!(kind_of(&r), "malformed_feeling");
}

#[tokio::test]
async fn ch01_authored_feeling_origin_beats_external_id() {
    let app = app().await;
    let r = send_feeling(&app, |it| {
        it["origin"] = "collected".into();
        it["external_id"] = "ext-1".into();
    })
    .await;
    assert_eq!(kind_of(&r), "feeling_not_authored");
}
