// SPDX-License-Identifier: AGPL-3.0-only
//! 主観の記録の取り込み口を、**本物の PostgreSQL に対して**確かめる（ST17 / Task 2）。
//!
//! 取り込み口（`ingest`）を通して入れ、格納された行を直に見る（読み出しは Task 4）。
//! **利用者で隔離する**（`testdb::user()`）。主観の行は DB の錠で消せないので、毎回新しい利用者で書く。
//!
//! 試験の名前の接頭辞は `st17_ingest_`。2.2 は `st17_ingest_nonce_`、2.3 は `st17_ingest_notcov_` で切る。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::feelings;
use crate::testdb;
use chrono::{DateTime, Utc};

const TOKEN: &str = "test-token-0123456789abcdef";
/// 22 文字以上の base64url。値を固定にして、写っていないことを文字列で探せるようにする
const NONCE: &str = "Qk9YLVNFQ1JFVC1OT05DRS0wMQ";

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

fn t(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

/// 日の記録の原文（画面の `feelings.ts` と同じ形。design D1）。
fn day_raw(
    id: uuid::Uuid,
    date: &str,
    written_at: &str,
    scales: serde_json::Value,
    note: Option<&str>,
    nonce: &str,
) -> String {
    serde_json::json!({
        "feeling": id, "nonce": nonce,
        "target": {"kind": "day", "date": date},
        "written_at": written_at, "written_tz": "Asia/Tokyo",
        "scales": scales, "note": note,
    })
    .to_string()
}

fn valence(v: i64) -> serde_json::Value {
    serde_json::json!([{"id": "valence", "version": 1, "value": v}])
}

/// 取り込み口へ送る 1 件。**由来は「本人が書いた」・端末と外部識別子なし**
fn item(
    id: uuid::Uuid,
    user: uuid::Uuid,
    event_time: &str,
    raw: &str,
    payload: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "id": id, "user_id": user, "logical_source": feelings::SOURCE,
        "external_id": null, "device_id": null, "origin": "authored",
        "event_time": event_time, "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
        "schema_version": 1, "raw": raw, "payload": payload,
    })
}

async fn send(app: &App, item: serde_json::Value) -> IngestResult {
    let (_, Json(mut res)) = ingest(State(app.clone()), auth(), Json(serde_json::json!([item])))
        .await
        .expect("取り込み口");
    res.pop().expect("1 件ぶんの結果")
}

/// 日の記録を 1 件書く。`(識別子, 結果)`。
async fn write_day(
    app: &App,
    user: uuid::Uuid,
    date: &str,
    written_at: &str,
    scales: serde_json::Value,
    note: Option<&str>,
) -> (uuid::Uuid, IngestResult) {
    let id = uuid::Uuid::new_v4();
    let raw = day_raw(id, date, written_at, scales, note, &nonce_for(id));
    let r = send(
        app,
        item(
            id,
            user,
            &format!("{date}T00:00:00+09:00"),
            &raw,
            serde_json::json!({}),
        ),
    )
    .await;
    (id, r)
}

/// 記録ごとに違う乱数（22 文字以上の base64url）
fn nonce_for(id: uuid::Uuid) -> String {
    format!("{}{}", id.simple(), "AAAAAAAAAAAAAAAAAAAAAA")
}

struct Stored {
    event_time: DateTime<Utc>,
    sensitivity: i32,
    raw: String,
    payload: serde_json::Value,
}

async fn stored(app: &App, id: uuid::Uuid) -> Stored {
    let (event_time, sensitivity, raw, payload): (DateTime<Utc>, i16, String, serde_json::Value) =
        sqlx::query_as(
            "SELECT event_time, sensitivity, raw, payload FROM core.event WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    Stored {
        event_time,
        sensitivity: i32::from(sensitivity),
        raw,
        payload,
    }
}

async fn count(app: &App, user: uuid::Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = $2")
        .bind(user)
        .bind(feelings::SOURCE)
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

fn scale_value(p: &serde_json::Value, id: &str) -> Option<i64> {
    p["scales"]
        .as_array()?
        .iter()
        .find(|s| s["id"] == id)
        .and_then(|s| s["value"].as_i64())
}

// ------------------------------------------------------------------ 2.1 取り込み口

// Scenario: 快–不快だけで記録が 1 件入る
#[tokio::test]
async fn st17_ingest_control_char_in_written_tz_rejects_only_that_item() {
    let app = app().await;
    let user = testdb::user();
    let bad_id = uuid::Uuid::new_v4();
    let bad_raw = day_raw(
        bad_id,
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(1),
        None,
        &nonce_for(bad_id),
    )
    .replace("Asia/Tokyo", "\\u0000");
    assert!(bad_raw.contains("\\u0000"));
    let good_id = uuid::Uuid::new_v4();
    let good_raw = day_raw(
        good_id,
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(1),
        None,
        &nonce_for(good_id),
    );
    let items = serde_json::json!([
        item(
            bad_id,
            user,
            "2026-10-07T00:00:00+09:00",
            &bad_raw,
            serde_json::json!({})
        ),
        item(
            good_id,
            user,
            "2026-10-07T00:00:00+09:00",
            &good_raw,
            serde_json::json!({})
        ),
    ]);
    let (_, Json(res)) = ingest(State(app.clone()), auth(), Json(items))
        .await
        .expect("まとめ送りは落ちない");
    assert!(
        !res[0].accepted && res[0].error.is_some(),
        "制御文字は 1 件の拒否"
    );
    assert!(res[1].accepted, "同じ束の他の 1 件は入る");
}

#[tokio::test]
async fn st17_ingest_valence_only_is_stored() {
    let app = app().await;
    let user = testdb::user();
    let (id, r) = write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(1),
        None,
    )
    .await;
    assert!(r.accepted && !r.duplicate, "{r:?}");
    assert_eq!(count(&app, user).await, 1);
    let s = stored(&app, id).await;
    assert_eq!(scale_value(&s.payload, "valence"), Some(1));
    assert_eq!(scale_value(&s.payload, "energy"), None, "元気さを持たない");
    assert!(s.payload["note"].is_null(), "ひとことを持たない");
}

// Scenario: 元気さとひとことを添えた記録が入る
#[tokio::test]
async fn st17_ingest_energy_and_note_are_stored() {
    let app = app().await;
    let scales = serde_json::json!([
        {"id": "valence", "version": 1, "value": -1},
        {"id": "energy", "version": 1, "value": 2},
    ]);
    let (id, r) = write_day(
        &app,
        testdb::user(),
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        scales,
        Some("久しぶりに走った"),
    )
    .await;
    assert!(r.accepted, "{r:?}");
    let s = stored(&app, id).await;
    assert_eq!(scale_value(&s.payload, "valence"), Some(-1));
    assert_eq!(scale_value(&s.payload, "energy"), Some(2));
    assert_eq!(s.payload["note"], "久しぶりに走った");
}

// Scenario: 尺度は識別子と版とともに残る
#[tokio::test]
async fn st17_ingest_scales_keep_id_and_version() {
    let app = app().await;
    let scales = serde_json::json!([
        {"id": "valence", "version": 1, "value": 0},
        {"id": "energy", "version": 1, "value": 1},
    ]);
    let (id, _) = write_day(
        &app,
        testdb::user(),
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        scales,
        None,
    )
    .await;
    let s = stored(&app, id).await;
    let got: Vec<(String, i64)> = s.payload["scales"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            (
                x["id"].as_str().unwrap().to_string(),
                x["version"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![("valence".to_string(), 1), ("energy".to_string(), 1)]
    );
}

// Scenario: 日の記録の出来事の時刻はその日の 0 時
#[tokio::test]
async fn st17_ingest_day_event_time_is_midnight_of_the_day() {
    let app = app().await;
    let (id, r) = write_day(
        &app,
        testdb::user(),
        "2026-10-07",
        "2026-10-08T08:10:00+09:00",
        valence(1),
        None,
    )
    .await;
    assert!(r.accepted, "{r:?}");
    assert_eq!(stored(&app, id).await.event_time, t("2026-10-06T15:00:00Z"));
}

// Scenario: 記入の日時は出来事の時刻と別に残る
#[tokio::test]
async fn st17_ingest_written_at_is_kept_apart_from_event_time() {
    let app = app().await;
    let (id, _) = write_day(
        &app,
        testdb::user(),
        "2026-10-07",
        "2026-10-08T08:10:00+09:00",
        valence(1),
        None,
    )
    .await;
    let s = stored(&app, id).await;
    let written: DateTime<Utc> = s.payload["written_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(written, t("2026-10-07T23:10:00Z"), "2026-10-08T08:10+09:00");
    assert_ne!(written, s.event_time);
}

// Scenario: 滞在の記録の出来事の時刻は書いた時点の滞在の始まり
#[tokio::test]
async fn st17_ingest_stay_event_time_is_the_start_when_written() {
    let app = app().await;
    let user = testdb::user();
    let (id, stay) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let (start, end) = ("2026-10-07T09:12:00+09:00", "2026-10-07T11:40:00+09:00");
    let raw = serde_json::json!({
        "feeling": id, "nonce": nonce_for(id),
        "target": {"kind": "stay", "stay": stay, "start": start, "end": end},
        "written_at": "2026-10-07T12:00:00+09:00", "written_tz": "Asia/Tokyo",
        "scales": valence(1), "note": null,
    })
    .to_string();
    let r = send(&app, item(id, user, start, &raw, serde_json::json!({}))).await;
    assert!(r.accepted, "{r:?}");
    let s = stored(&app, id).await;
    assert_eq!(s.event_time, t("2026-10-07T00:12:00Z"));
    assert_eq!(s.payload["target"]["stay"], stay.to_string());
    assert_eq!(
        t(s.payload["target"]["start"].as_str().unwrap()),
        t("2026-10-07T00:12:00Z")
    );
    assert_eq!(
        t(s.payload["target"]["end"].as_str().unwrap()),
        t("2026-10-07T02:40:00Z")
    );
}

// Scenario: 滞在の時刻が作り直しで動いても記録の時刻は動かない
#[tokio::test]
async fn st17_ingest_stay_time_move_does_not_move_the_record() {
    let app = app().await;
    let user = testdb::user();
    let (id, stay) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let (start, end) = ("2026-10-07T09:12:00+09:00", "2026-10-07T11:40:00+09:00");
    let raw = serde_json::json!({
        "feeling": id, "nonce": nonce_for(id),
        "target": {"kind": "stay", "stay": stay, "start": start, "end": end},
        "written_at": "2026-10-07T12:00:00+09:00", "written_tz": "Asia/Tokyo",
        "scales": valence(1), "note": null,
    })
    .to_string();
    let r = send(&app, item(id, user, start, &raw, serde_json::json!({}))).await;
    assert!(r.accepted, "{r:?}");
    let before = stored(&app, id).await;
    // 作り直しで同じ識別子の滞在の始まりが動く（滞在の行は作り直しで書き換わる）
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,'s01-stay',$3,'derived',$4,540,'Asia/Tokyo',1,$3,'{}','{}')",
    )
    .bind(stay)
    .bind(user)
    .bind(stay.to_string())
    .bind(t("2026-10-07T00:12:00Z"))
    .execute(&app.pool)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET event_time = $2 WHERE id = $1")
        .bind(stay)
        .bind(t("2026-10-07T00:30:00Z"))
        .execute(&app.pool)
        .await
        .unwrap();
    let after = stored(&app, id).await;
    assert_eq!(after.event_time, before.event_time);
    assert_eq!(after.payload["target"], before.payload["target"]);
}

// Scenario: 昨日の分を後から書ける
#[tokio::test]
async fn st17_ingest_yesterday_can_be_written_later() {
    let app = app().await;
    let (id, r) = write_day(
        &app,
        testdb::user(),
        "2026-10-07",
        "2026-10-08T09:00:00+09:00",
        valence(2),
        None,
    )
    .await;
    assert!(r.accepted, "{r:?}");
    let s = stored(&app, id).await;
    assert_eq!(s.payload["target"]["date"], "2026-10-07");
    assert_eq!(s.event_time, t("2026-10-06T15:00:00Z"));
}

// Scenario: 未来の日を紐づけ先にする記録を受け付ける
#[tokio::test]
async fn st17_ingest_future_day_is_accepted() {
    let app = app().await;
    let (id, r) = write_day(
        &app,
        testdb::user(),
        "2099-01-01",
        "2026-10-08T09:00:00+09:00",
        valence(0),
        None,
    )
    .await;
    assert!(r.accepted, "{r:?}");
    assert_eq!(stored(&app, id).await.event_time, t("2098-12-31T15:00:00Z"));
}

// Scenario: 同じ値を書き直しても 1 件増える
#[tokio::test]
async fn st17_ingest_rewriting_the_same_value_adds_one() {
    let app = app().await;
    let user = testdb::user();
    for _ in 0..2 {
        let (_, r) = write_day(
            &app,
            user,
            "2026-10-07",
            "2026-10-07T21:30:05+09:00",
            valence(1),
            Some("同じ"),
        )
        .await;
        assert!(r.accepted && !r.duplicate, "{r:?}");
    }
    assert_eq!(count(&app, user).await, 2);
}

// Scenario: 同じ記録の再送は増えない
#[tokio::test]
async fn st17_ingest_resending_the_same_record_adds_none() {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let raw = day_raw(
        id,
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(1),
        None,
        &nonce_for(id),
    );
    let it = item(
        id,
        user,
        "2026-10-07T00:00:00+09:00",
        &raw,
        serde_json::json!({}),
    );
    let first = send(&app, it.clone()).await;
    let second = send(&app, it).await;
    assert!(first.accepted && !first.duplicate, "{first:?}");
    assert!(second.accepted && second.duplicate, "{second:?}");
    assert_eq!(count(&app, user).await, 1);
}

// Scenario: 主観の原文が 1 バイトも変わらずに残る
#[tokio::test]
async fn st17_ingest_raw_is_kept_byte_for_byte() {
    let app = app().await;
    let id = uuid::Uuid::new_v4();
    // 空白・キー順・非 ASCII・NFD のひとこと。どれも触られない
    let raw = format!(
        "{{ \"note\" : \"か\u{3099}\", \"scales\":[ {{\"id\":\"valence\",\"version\":1,\"value\":1}} ],\
         \"written_tz\":\"Asia/Tokyo\", \"written_at\":\"2026-10-07T21:30:05+09:00\",\
         \"target\":{{\"kind\":\"day\",\"date\":\"2026-10-07\"}}, \"nonce\":\"{}\", \"feeling\":\"{id}\" }}",
        nonce_for(id)
    );
    let r = send(
        &app,
        item(
            id,
            testdb::user(),
            "2026-10-07T00:00:00+09:00",
            &raw,
            serde_json::json!({}),
        ),
    )
    .await;
    assert!(r.accepted, "{r:?}");
    assert_eq!(stored(&app, id).await.raw.as_bytes(), raw.as_bytes());
}

// Scenario: 合成済みでないひとことは合成済みで読み出される
#[tokio::test]
async fn st17_ingest_note_is_stored_as_nfc() {
    let app = app().await;
    let nfd = "か\u{3099}e\u{301}";
    let (id, r) = write_day(
        &app,
        testdb::user(),
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(1),
        Some(nfd),
    )
    .await;
    assert!(r.accepted, "{r:?}");
    assert_eq!(stored(&app, id).await.payload["note"], "\u{304c}\u{e9}");
}

// Scenario: 原文と食い違う解析済みの主観を送っても原文の値で格納される
#[tokio::test]
async fn st17_ingest_payload_is_rebuilt_from_raw() {
    let app = app().await;
    let id = uuid::Uuid::new_v4();
    let raw = day_raw(
        id,
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(2),
        None,
        &nonce_for(id),
    );
    let lie = serde_json::json!({"scales": [{"id": "valence", "version": 1, "value": -2}]});
    let r = send(
        &app,
        item(id, testdb::user(), "2026-10-07T00:00:00+09:00", &raw, lie),
    )
    .await;
    assert!(r.accepted, "{r:?}");
    assert_eq!(
        scale_value(&stored(&app, id).await.payload, "valence"),
        Some(2)
    );
}

// Scenario: 前後は格納した記録に持たない
#[tokio::test]
async fn st17_ingest_no_before_after_field() {
    let app = app().await;
    let (id, _) = write_day(
        &app,
        testdb::user(),
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(1),
        None,
    )
    .await;
    let s = stored(&app, id).await;
    for text in [s.raw.as_str(), &s.payload.to_string()] {
        for word in ["before", "after", "phase", "前", "後"] {
            assert!(
                !text.contains(word),
                "前/後を表す欄らしきものが残っている: {word}"
            );
        }
    }
}

// Scenario: 主観はローカル AI までで格納される
#[tokio::test]
async fn st17_ingest_sensitivity_is_local_ai() {
    let app = app().await;
    let (id, _) = write_day(
        &app,
        testdb::user(),
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(1),
        None,
    )
    .await;
    assert_eq!(stored(&app, id).await.sensitivity, 2);
}

// Scenario: 主観以外の既定は主観を足しても変わらない
#[tokio::test]
async fn st17_ingest_other_sources_keep_their_default() {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let it = serde_json::json!({
        "id": id, "user_id": user, "logical_source": "c01-location",
        "external_id": null, "device_id": "test-dev", "origin": "collected",
        "event_time": "2026-10-07T03:00:00Z", "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
        "schema_version": 1, "raw": r#"{"lat":35.0,"lon":139.0,"acc_m":12}"#,
        "payload": {"lat": 35.0, "lon": 139.0, "acc_m": 12},
    });
    let r = send(&app, it).await;
    assert!(r.accepted, "{r:?}");
    assert_eq!(stored(&app, id).await.sensitivity, 1);
}

// ------------------------------------------------------------------ 2.2 乱数（D4）

// Scenario: 主観の乱数は解析済みに写らない
#[tokio::test]
async fn st17_ingest_nonce_is_not_in_the_parsed_value() {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let raw = day_raw(
        id,
        "2026-10-07",
        "2026-10-07T21:30:05+09:00",
        valence(1),
        Some("ひとこと"),
        NONCE,
    );
    let r = send(
        &app,
        item(
            id,
            user,
            "2026-10-07T00:00:00+09:00",
            &raw,
            serde_json::json!({"nonce": NONCE}),
        ),
    )
    .await;
    assert!(r.accepted, "{r:?}");
    let s = stored(&app, id).await;
    assert!(s.raw.contains(NONCE), "原文には乱数が入っている（前提）");
    assert!(
        !s.payload.to_string().contains(NONCE),
        "解析済みに乱数が写っている"
    );
    // 原文以外の列（読み出しの元）のどこにも写らない。送り主の解析済みに乱数を入れても同じ
    let others: String =
        sqlx::query_scalar("SELECT (to_jsonb(e) - 'raw')::text FROM core.event e WHERE id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(!others.contains(NONCE), "原文以外の列に乱数が写っている");
}

// Scenario: 消去後に残る列と正しい値から主観の鍵を作り直せない
#[tokio::test]
async fn st17_ingest_nonce_erased_record_key_cannot_be_rebuilt() {
    let app = app().await;
    let user = testdb::user();
    let id = uuid::Uuid::new_v4();
    let (written_at, note) = ("2026-10-07T21:30:05+09:00", None);
    let raw = day_raw(id, "2026-10-07", written_at, valence(1), note, NONCE);
    let r = send(
        &app,
        item(
            id,
            user,
            "2026-10-07T00:00:00+09:00",
            &raw,
            serde_json::json!({}),
        ),
    )
    .await;
    assert!(r.accepted, "{r:?}");

    // 本文を台帳つきで消去する（原文と解析済みを空にし、時刻と鍵は変えない）
    let mut tx = app.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, 's01-feeling', 'event', 'check')",
    )
    .bind(id)
    .bind(user)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}'::jsonb WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let (kept_id, event_time, kept_hash): (uuid::Uuid, DateTime<Utc>, String) =
        sqlx::query_as("SELECT id, event_time, content_hash FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    // 対照: 乱数まで正しければ、残った列と同じ鍵になる（組み方が合っている証拠）
    let right = ingest::content_hash_of(feelings::SOURCE, event_time, &raw);
    assert_eq!(right, kept_hash, "対照の組み方が実物と合っていない");
    // 攻撃: 識別子・出来事の時刻・紐づけ先・記入の日時・尺度は正しく、乱数だけ知らない
    for guess in ["", "AAAAAAAAAAAAAAAAAAAAAA", "Qk9YLVNFQ1JFVC1OT05DRS0wMg"] {
        let forged = day_raw(kept_id, "2026-10-07", written_at, valence(1), note, guess);
        assert_ne!(
            ingest::content_hash_of(feelings::SOURCE, event_time, &forged),
            kept_hash,
            "乱数を知らずに鍵を作り直せてしまう"
        );
    }
}

// ------------------------------------------------------------------ 2.3 稼働状況の外（D19）

// Scenario: 主観のソースは稼働状況に含まれない
#[tokio::test]
async fn st17_ingest_notcov_source_is_outside_coverage() {
    let app = app().await;
    let user = testdb::user();
    assert!(
        coverage::must_sources()
            .iter()
            .all(|(n, _)| n != feelings::SOURCE),
        "must_sources に主観が入っている"
    );
    let Json(out) = coverage_get(
        State(app.clone()),
        auth(),
        Query(CoverageQuery {
            from: testdb::date("2026-10-01"),
            to: testdb::date("2026-10-10"),
            user_id: Some(user),
        }),
    )
    .await
    .expect("稼働状況");
    assert!(!out.is_empty(), "他のソースは返る（前提）");
    assert!(out.iter().all(|s| s.logical_source != feelings::SOURCE));
    assert!(out.iter().all(|s| s.named_source != feelings::SOURCE));
    // 主観の記録が 10 日間 1 件も無いのに、途絶の日は主観には付かない（そもそも主観の行が無い）
    let outage_in_feeling = out
        .iter()
        .filter(|s| s.logical_source == feelings::SOURCE)
        .flat_map(|s| &s.days)
        .any(|d| d.state == coverage::DayState::Outage);
    assert!(!outage_in_feeling);
}
