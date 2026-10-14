// SPDX-License-Identifier: AGPL-3.0-only
//! 形の合わない主観の記録を断ることを、**本物の PostgreSQL に対して**確かめる（ST17 / Task 3）。
//!
//! 断る理由の種別は spec「形の合わない主観の記録は受け付けない」の表。取り込み口を通して送り、
//! 1 件ごとの結果と格納された件数を見る。利用者で隔離する（`testdb::user()`）。
//!
//! 試験の名前の接頭辞は `st17_reject_`。3.1（形）は `st17_reject_shape_`、3.2（紐づけ先）は `st17_reject_target_`。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::feelings_ingest_tests::{app, count, item, nonce_for, put_stay, send, stored, valence};
use crate::testdb;

const STAY_START: &str = "2026-10-07T09:12:00+09:00";
const STAY_END: &str = "2026-10-07T11:40:00+09:00";
const DAY_TIME: &str = "2026-10-07T00:00:00+09:00";

/// 日の記録の原文（JSON）。各試験が欄を書き換えて送る
fn day(id: uuid::Uuid) -> serde_json::Value {
    serde_json::json!({
        "feeling": id, "nonce": nonce_for(id),
        "target": {"kind": "day", "date": "2026-10-07"},
        "written_at": "2026-10-07T21:30:05+09:00", "written_tz": "Asia/Tokyo",
        "scales": valence(1), "note": null,
    })
}

fn stay_raw(id: uuid::Uuid, stay: uuid::Uuid, start: &str, end: &str) -> serde_json::Value {
    let mut v = day(id);
    v["target"] = serde_json::json!({"kind": "stay", "stay": stay, "start": start, "end": end});
    v
}

/// 断った理由の種別（応答に出る文字列）
fn kind_of(r: &IngestResult) -> String {
    serde_json::to_value(r.error.expect("断った理由"))
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

/// 送って、断られた理由の種別を返す。受理されたら `None`
async fn reject_kind(
    app: &App,
    user: uuid::Uuid,
    id: uuid::Uuid,
    event_time: &str,
    raw: &serde_json::Value,
) -> Option<String> {
    let r = send(
        app,
        item(
            id,
            user,
            event_time,
            &raw.to_string(),
            serde_json::json!({}),
        ),
    )
    .await;
    if r.accepted {
        None
    } else {
        Some(kind_of(&r))
    }
}

async fn expect_shape(raw_edit: impl FnOnce(&mut serde_json::Value), why: &str) {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let mut raw = day(id);
    raw_edit(&mut raw);
    let got = reject_kind(&app, user, id, DAY_TIME, &raw).await;
    assert_eq!(got.as_deref(), Some(why));
    assert_eq!(count(&app, user).await, 0, "断った記録は格納されない");
}

// ------------------------------------------------------------------ 3.1 形

// Scenario: 快–不快の無い記録は受け付けない
#[tokio::test]
async fn st17_reject_shape_without_valence() {
    expect_shape(
        |r| r["scales"] = serde_json::json!([{"id": "energy", "version": 1, "value": 1}]),
        "invalid_scale",
    )
    .await;
}

// Scenario: 範囲の外の快–不快は受け付けない
#[tokio::test]
async fn st17_reject_shape_valence_out_of_range() {
    expect_shape(|r| r["scales"] = valence(3), "invalid_scale").await;
}

// Scenario: 整数でない尺度の値は受け付けない
#[tokio::test]
async fn st17_reject_shape_non_integer_scale() {
    expect_shape(
        |r| {
            r["scales"] = serde_json::json!([
                {"id": "valence", "version": 1, "value": 1},
                {"id": "energy", "version": 1, "value": 0.5},
            ])
        },
        "invalid_scale",
    )
    .await;
}

// Scenario: 知らない尺度は受け付けない
#[tokio::test]
async fn st17_reject_shape_unknown_scale_version() {
    expect_shape(
        |r| r["scales"] = serde_json::json!([{"id": "valence", "version": 2, "value": 1}]),
        "invalid_scale",
    )
    .await;
}

// Scenario: 同じ尺度が 2 つある記録は受け付けない
#[tokio::test]
async fn st17_reject_shape_duplicate_scale() {
    expect_shape(
        |r| {
            r["scales"] = serde_json::json!([
                {"id": "valence", "version": 1, "value": 1},
                {"id": "valence", "version": 1, "value": 2},
            ])
        },
        "invalid_scale",
    )
    .await;
}

// Scenario: 長すぎるひとことは切り詰めずに断る
#[tokio::test]
async fn st17_reject_shape_note_over_the_limit() {
    assert_eq!(feelings::NOTE_MAX_CHARS, 2000, "上限は名指しで固定する");
    expect_shape(
        |r| r["note"] = serde_json::json!("あ".repeat(feelings::NOTE_MAX_CHARS + 1)),
        "invalid_note",
    )
    .await;
}

async fn accepted_note(note: String) -> (App, uuid::Uuid, uuid::Uuid) {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let mut raw = day(id);
    raw["note"] = serde_json::json!(note);
    assert_eq!(reject_kind(&app, user, id, DAY_TIME, &raw).await, None);
    (app, user, id)
}

// Scenario: 上限ちょうどのひとことは受け付ける
#[tokio::test]
async fn st17_reject_shape_note_at_the_limit() {
    let (app, user, id) = accepted_note("あ".repeat(feelings::NOTE_MAX_CHARS)).await;
    assert_eq!(count(&app, user).await, 1);
    let note = stored(&app, id).await.payload["note"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(note.chars().count(), 2000, "切り詰めない");
}

// Scenario: ひとことの上限はコードポイントで数える
#[tokio::test]
async fn st17_reject_shape_note_limit_counts_code_points() {
    // 😀 は UTF-16 で 2 単位・コードポイントで 1。2,000 個は UTF-16 では 4,000 単位
    let (app, user, id) = accepted_note("😀".repeat(2000)).await;
    assert_eq!(count(&app, user).await, 1);
    let note = stored(&app, id).await.payload["note"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(note.chars().count(), 2000);
    // 2,001 個は断る（境界を両側から固定する）
    expect_shape(
        |r| r["note"] = serde_json::json!("😀".repeat(2001)),
        "invalid_note",
    )
    .await;
}

// Scenario: 空のひとことはひとこと無しとして受け付ける
#[tokio::test]
async fn st17_reject_shape_blank_note_is_no_note() {
    let (app, user, id) = accepted_note("  \n　 ".to_string()).await;
    assert_eq!(count(&app, user).await, 1);
    assert!(stored(&app, id).await.payload["note"].is_null());
}

// Scenario: 記入の日時の読めない記録は受け付けない
#[tokio::test]
async fn st17_reject_shape_written_at_without_offset() {
    expect_shape(
        |r| r["written_at"] = serde_json::json!("2026-10-07T21:30:00"),
        "malformed_feeling",
    )
    .await;
}

// Scenario: 記録の地域が東京でない主観は受け付けない
#[tokio::test]
async fn st17_reject_shape_region_not_tokyo() {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let raw = day(id);
    let mut it = item(id, user, DAY_TIME, &raw.to_string(), serde_json::json!({}));
    it["tz_offset_min"] = serde_json::json!(0);
    let r = send(&app, it).await;
    assert!(!r.accepted);
    assert_eq!(kind_of(&r), "malformed_feeling");
    assert_eq!(count(&app, user).await, 0);
}

// Scenario: 乱数が短い主観は受け付けない
#[tokio::test]
async fn st17_reject_shape_short_nonce() {
    expect_shape(
        |r| r["nonce"] = serde_json::json!("AAAAAAAAAAA"),
        "malformed_feeling",
    )
    .await;
}

async fn send_with(edit: impl FnOnce(&mut serde_json::Value)) -> (IngestResult, i64) {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let raw = day(id);
    let mut it = item(id, user, DAY_TIME, &raw.to_string(), serde_json::json!({}));
    edit(&mut it);
    let r = send(&app, it).await;
    (r, count(&app, user).await)
}

// Scenario: 本人が書いたでない主観は受け付けない
#[tokio::test]
async fn st17_reject_shape_origin_not_authored() {
    let (r, n) = send_with(|it| it["origin"] = serde_json::json!("collected")).await;
    assert_eq!(kind_of(&r), "feeling_not_authored");
    assert_eq!(n, 0);
}

// Scenario: 端末識別子を持つ主観は受け付けない
#[tokio::test]
async fn st17_reject_shape_with_device_id() {
    let (r, n) = send_with(|it| it["device_id"] = serde_json::json!(uuid::Uuid::new_v4())).await;
    assert_eq!(kind_of(&r), "feeling_not_authored");
    assert_eq!(n, 0);
}

// Scenario: 外部識別子を持つ主観は受け付けない
#[tokio::test]
async fn st17_reject_shape_with_external_id() {
    let (r, n) = send_with(|it| it["external_id"] = serde_json::json!("ext-1")).await;
    assert_eq!(kind_of(&r), "feeling_has_external_id");
    assert_eq!(n, 0);
}

// Scenario: 2 つに当たる記録は表の上の種別で断られる
#[tokio::test]
async fn st17_reject_shape_first_row_of_the_table_wins() {
    expect_shape(
        |r| {
            r["scales"] = valence(3);
            r["note"] = serde_json::json!("あ".repeat(2001));
        },
        "invalid_scale",
    )
    .await;
}

// Scenario: 主観の拒否の応答に値が含まれない
#[tokio::test]
async fn st17_reject_shape_response_has_no_value_or_note() {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let note = "ひみつのひとこと-7f3a";
    let mut raw = day(id);
    raw["scales"] = valence(3);
    raw["note"] = serde_json::json!(note);
    let it = item(id, user, DAY_TIME, &raw.to_string(), serde_json::json!({}));
    let r = send(&app, it).await;
    assert!(!r.accepted);
    let body = serde_json::to_string(&r).unwrap();
    assert!(!body.contains(note), "{body}");
    assert!(!body.contains("\"value\""), "{body}");
}

// ------------------------------------------------------------------ 3.2 紐づけ先

// Scenario: 登録簿に無い紐づけ先の種類は受け付けない
#[tokio::test]
async fn st17_reject_target_kind_not_in_registry() {
    expect_shape(
        |r| r["target"] = serde_json::json!({"kind": "person", "person": uuid::Uuid::new_v4()}),
        "invalid_feeling_target",
    )
    .await;
}

// Scenario: 暦に無い日は紐づけ先にできない
#[tokio::test]
async fn st17_reject_target_day_not_in_calendar() {
    expect_shape(
        |r| r["target"] = serde_json::json!({"kind": "day", "date": "2026-02-30"}),
        "invalid_feeling_target",
    )
    .await;
}

// Scenario: 無い滞在は紐づけ先にできない
#[tokio::test]
async fn st17_reject_target_stay_does_not_exist() {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let raw = stay_raw(id, uuid::Uuid::new_v4(), STAY_START, STAY_END);
    let got = reject_kind(&app, user, id, STAY_START, &raw).await;
    assert_eq!(got, Some("invalid_feeling_target".to_string()));
    assert_eq!(count(&app, user).await, 0);
}

// Scenario: 別の利用者の滞在は紐づけ先にできない
#[tokio::test]
async fn st17_reject_target_stay_of_another_user() {
    let app = app().await;
    let (owner, user) = (testdb::user(), testdb::user());
    let (id, stay) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    put_stay(&app, owner, stay, STAY_START, STAY_END).await;
    let raw = stay_raw(id, stay, STAY_START, STAY_END);
    let got = reject_kind(&app, user, id, STAY_START, &raw).await;
    assert_eq!(got, Some("invalid_feeling_target".to_string()));
    assert_eq!(count(&app, user).await, 0);
}

// Scenario: 滞在の区間と重ならない始まりと終わりは受け付けない
#[tokio::test]
async fn st17_reject_target_span_does_not_overlap_the_stay() {
    let app = app().await;
    let user = testdb::user();
    let (id, stay) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    put_stay(&app, user, stay, STAY_START, STAY_END).await;
    let (s, e) = ("2020-01-01T00:00:00+09:00", "2020-01-01T01:00:00+09:00");
    let raw = stay_raw(id, stay, s, e);
    let got = reject_kind(&app, user, id, s, &raw).await;
    assert_eq!(got, Some("invalid_feeling_target".to_string()));
    assert_eq!(count(&app, user).await, 0);
}

// Scenario: 作り直しで少し動いた滞在の区間でも受け付ける
#[tokio::test]
async fn st17_reject_target_moved_stay_span_still_overlaps() {
    let app = app().await;
    let user = testdb::user();
    let (id, stay) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    // いまの区間は 9:15–11:40。画面が読んだのは 9:12–11:40
    put_stay(&app, user, stay, "2026-10-07T09:15:00+09:00", STAY_END).await;
    let raw = stay_raw(id, stay, STAY_START, STAY_END);
    assert_eq!(reject_kind(&app, user, id, STAY_START, &raw).await, None);
    assert_eq!(
        stored(&app, id).await.event_time,
        "2026-10-07T00:12:00Z"
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap(),
        "記録の滞在の始まりは 9:12 のまま"
    );
}

// Scenario: 出来事の時刻が対象の時刻と違う記録は受け付けない
#[tokio::test]
async fn st17_reject_target_event_time_differs_from_the_target() {
    let app = app().await;
    let user = testdb::user();
    // 日: 出来事の時刻を記入の日時にする
    let id = uuid::Uuid::new_v4();
    let got = reject_kind(&app, user, id, "2026-10-08T08:10:00+09:00", &day(id)).await;
    assert_eq!(got, Some("invalid_feeling_target".to_string()));
    // 滞在: 出来事の時刻が記録の滞在の始まりと違う
    let (id, stay) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    put_stay(&app, user, stay, STAY_START, STAY_END).await;
    let raw = stay_raw(id, stay, STAY_START, STAY_END);
    let got = reject_kind(&app, user, id, "2026-10-07T12:00:00+09:00", &raw).await;
    assert_eq!(got, Some("invalid_feeling_target".to_string()));
    assert_eq!(count(&app, user).await, 0);
}

// Scenario: 消した滞在を紐づけ先にできる
#[tokio::test]
async fn st17_reject_target_deleted_stay_is_accepted() {
    let app = app().await;
    let user = testdb::user();
    let (id, stay) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    put_stay(&app, user, stay, STAY_START, STAY_END).await;
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
        .bind(stay)
        .execute(&app.pool)
        .await
        .unwrap();
    let raw = stay_raw(id, stay, STAY_START, STAY_END);
    assert_eq!(reject_kind(&app, user, id, STAY_START, &raw).await, None);
    assert_eq!(count(&app, user).await, 1);
}

// Scenario: 吸収された滞在を紐づけ先にできる
#[tokio::test]
async fn st17_reject_target_absorbed_stay_is_accepted() {
    let app = app().await;
    let user = testdb::user();
    let (id, stay, into) = (
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
    );
    put_stay(&app, user, stay, STAY_START, STAY_END).await;
    put_stay(&app, user, into, "2026-10-07T09:00:00+09:00", STAY_END).await;
    let cid: i64 = sqlx::query_scalar(
        "INSERT INTO core.stay_criteria (user_id, radius_m, min_minutes, gap_minutes, sources)
         VALUES ($1, 100, 10, 10, '{c01-location}') RETURNING id",
    )
    .bind(user)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.stay_absorbed (event_id, user_id, into_event_id, criteria_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(stay)
    .bind(user)
    .bind(into)
    .bind(cid)
    .execute(&app.pool)
    .await
    .unwrap();
    let raw = stay_raw(id, stay, STAY_START, STAY_END);
    assert_eq!(reject_kind(&app, user, id, STAY_START, &raw).await, None);
    assert_eq!(count(&app, user).await, 1);
}

// design D21: 再送（同じ原文の 2 回目）は紐づけ先の検査をやり直さない。
// 1 回目を受けた後に滞在が作り直しで別の区間へ動いても、同じ原文の再送は断らずに畳む（受理した記録を後から断らない）。
// 新しい原文（別の識別子・乱数）で同じ紐づけ先を送ると断られることを並べて見て、検査が飛ばされたのは再送だけだと確かめる。
#[tokio::test]
async fn st17_reject_target_resend_skips_the_target_check() {
    use crate::feelings_ingest_tests::t;
    let app = app().await;
    let user = testdb::user();
    let (id, stay) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    put_stay(&app, user, stay, STAY_START, STAY_END).await;
    let raw = stay_raw(id, stay, STAY_START, STAY_END).to_string();
    let it = item(id, user, STAY_START, &raw, serde_json::json!({}));
    let first = send(&app, it.clone()).await;
    assert!(first.accepted && !first.duplicate, "{first:?}");

    // 滞在が作り直しで重ならない区間へ動いた（いま検査すれば断る）
    sqlx::query("UPDATE core.event SET event_time = $2, payload = $3 WHERE id = $1")
        .bind(stay)
        .bind(t("2020-01-01T00:00:00Z"))
        .bind(serde_json::json!({ "end": "2020-01-01T01:00:00Z" }))
        .execute(&app.pool)
        .await
        .unwrap();

    let again = send(&app, it).await;
    assert!(
        again.accepted && again.duplicate,
        "再送は断らず畳む: {again:?}"
    );
    assert_eq!(count(&app, user).await, 1, "再送で増えない");

    let other = uuid::Uuid::new_v4();
    let fresh = stay_raw(other, stay, STAY_START, STAY_END);
    assert_eq!(
        reject_kind(&app, user, other, STAY_START, &fresh).await,
        Some("invalid_feeling_target".to_string()),
        "新しい原文は検査される"
    );
    assert_eq!(count(&app, user).await, 1);
}
