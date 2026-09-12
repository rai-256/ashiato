// SPDX-License-Identifier: AGPL-3.0-only
//! 冪等の判定・更新と履歴・削除済みの保護・門（ST03）を、**本物の PostgreSQL に対して**確かめる。
//!
//! 取り込み口を通さない経路（psql から直に撃つ）は `tools/check-immutable.sh` が見る ——
//! **アプリ層の実装でもここの検査は緑になる**ので、DB 側を観測するのはあちらだけ。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

async fn app() -> App {
    App {
        pool: testdb::pool().await,
        token: TOKEN.into(),
    }
}

fn auth() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

async fn post(app: &App, body: serde_json::Value) -> (StatusCode, Vec<IngestResult>) {
    let (code, Json(res)) = ingest(State(app.clone()), auth(), Json(body))
        .await
        .expect("取り込み口");
    (code, res)
}

/// 記録 1 件ぶんの JSON。**収集側の識別子は毎回新しく振る**（深掘り Q14）——
/// 外部の識別子から導くと、更新された記録が「同じ識別子・違う内容」として届き、
/// Q1（更新する）と Q5（400 で断る）が同じ到着に逆を指す。
fn ev(source: &str, user: uuid::Uuid, raw: &str) -> serde_json::Value {
    serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "user_id": user,
        "logical_source": source,
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": "2026-05-01T01:00:00Z",
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": raw,
        "payload": {},
    })
}

/// 欄を 1 つ差し替えた記録。
fn with(mut item: serde_json::Value, key: &str, value: serde_json::Value) -> serde_json::Value {
    item[key] = value;
    item
}

async fn count_rows(app: &App, source: &str) -> i64 {
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM core.event WHERE logical_source = $1")
        .bind(source)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    n
}

async fn raw_of(app: &App, id: uuid::Uuid) -> String {
    let (raw,): (String,) = sqlx::query_as("SELECT raw FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    raw
}

async fn soft_delete(app: &App, id: uuid::Uuid) {
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
        .bind(id)
        .execute(&app.pool)
        .await
        .unwrap();
}

// ------------------------------------------------------------------ 冪等の判定（2 段）

/// Scenario: 端末からの再送で行が増えない
#[tokio::test]
async fn dedup_by_hash_when_no_external_id() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "dedup-hash", "none").await;
    let u = testdb::user();
    for i in 0..3 {
        let (code, res) = post(&app, serde_json::json!([ev(&s, u, r#"{"seq":"same"}"#)])).await;
        assert_eq!(code, StatusCode::OK);
        assert!(res[0].accepted, "{i} 回目が受理されていない");
        assert_eq!(res[0].duplicate, i > 0, "{i} 回目の重複の判定が違う");
    }
    assert_eq!(count_rows(&app, &s).await, 1, "再送で行が増えた");
}

/// Scenario: 外部サービスの記録を再送しても行が増えない
#[tokio::test]
async fn dedup_by_external_id() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "dedup-ext", "record").await;
    let u = testdb::user();
    for i in 0..3 {
        let item = with(
            ev(&s, u, r#"{"seq":"ext-same"}"#),
            "external_id",
            serde_json::json!("ext-1"),
        );
        let (code, res) = post(&app, serde_json::json!([item])).await;
        assert_eq!(code, StatusCode::OK);
        assert!(res[0].accepted, "{i} 回目が受理されていない");
    }
    assert_eq!(count_rows(&app, &s).await, 1, "再送で行が増えた");
}

/// **内容が同じでも外部識別子が違えば別の行**（深掘り Q6）。
/// 内容の鍵の索引を一意のまま残すと、ここが一意違反で落ちる。
///
/// Scenario: 内容が同じでも外部識別子が違えば別の行になる
#[tokio::test]
async fn same_content_different_external_id_makes_two_rows() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "same-content", "record").await;
    let u = testdb::user();
    for ext in ["ext-a", "ext-b"] {
        let item = with(
            ev(&s, u, r#"{"seq":"twins"}"#),
            "external_id",
            serde_json::json!(ext),
        );
        let (_, res) = post(&app, serde_json::json!([item])).await;
        assert!(res[0].accepted && !res[0].duplicate, "{ext} が畳まれた");
    }
    assert_eq!(count_rows(&app, &s).await, 2, "識別子が違うのに畳まれた");
}

/// **判定は利用者ごと**（深掘り Q2 / Q15）。索引に利用者識別子を足しただけで、
/// `content_hash` の作り方は ST01 のまま（`hash_is_pinned` の期待値は変わらない）。
///
/// Scenario: 別の利用者の同じ内容は畳まれない
#[tokio::test]
async fn different_user_not_deduped() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "per-user", "none").await;
    for u in [testdb::user(), testdb::user()] {
        let (_, res) = post(&app, serde_json::json!([ev(&s, u, r#"{"seq":"mine"}"#)])).await;
        assert!(res[0].accepted && !res[0].duplicate, "利用者をまたいで畳まれた");
    }
    assert_eq!(count_rows(&app, &s).await, 2);
}

/// **収集側の識別子を使い回した 1 件だけを断る**（深掘り Q5）。
/// `id` は主キーなので、断らないと重複違反で 500 になり**まとめ送り全体が落ちる**。
///
/// Scenario: 収集側の識別子が同じで内容が違えば断られる
#[tokio::test]
async fn id_reused_is_rejected_without_stopping_the_batch() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "id-reuse", "none").await;
    let u = testdb::user();

    let first = ev(&s, u, r#"{"seq":"first"}"#);
    let reused_id = first["id"].clone();
    let (_, res) = post(&app, serde_json::json!([first])).await;
    let stored_id = res[0].id.unwrap();

    let (code, res) = post(
        &app,
        serde_json::json!([
            with(ev(&s, u, r#"{"seq":"other"}"#), "id", reused_id),
            ev(&s, u, r#"{"seq":"innocent"}"#),
        ]),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "1 件でも入れば 200");
    assert!(!res[0].accepted, "使い回しが受理された");
    assert!(
        matches!(res[0].error, Some(IngestError::IdReused)),
        "理由の種別が違う: {:?}",
        res[0].error
    );
    assert!(res[1].accepted, "同じ要求の正しい記録が巻き込まれた");
    assert_eq!(
        raw_of(&app, stored_id).await,
        r#"{"seq":"first"}"#,
        "既存の記録が書き換わっている"
    );
}

/// **1 件 1 トランザクション**（design D3）。門は COMMIT 時に落ちるので、
/// まとめ送りを 1 トランザクションにすると 1 件の失敗が全件を巻き戻す。
///
/// 同じまとまりの中で 2 件目が 1 件目の重複になることが、
/// **1 件目が既に COMMIT されている**ことの観測になる。
#[tokio::test]
async fn one_bad_item_does_not_roll_back_others() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "per-item-tx", "none").await;
    let u = testdb::user();
    let first = ev(&s, u, r#"{"seq":"a"}"#);
    let resend = with(first.clone(), "id", serde_json::json!(uuid::Uuid::new_v4()));

    let (code, res) = post(
        &app,
        serde_json::json!([
            first,
            resend,
            with(
                ev(&s, u, r#"{"seq":"bad"}"#),
                "origin",
                serde_json::json!("guessed")
            ),
            ev(&s, u, r#"{"seq":"b"}"#),
        ]),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(
        res.iter().map(|r| r.accepted).collect::<Vec<_>>(),
        vec![true, true, false, true]
    );
    assert!(
        res[1].duplicate,
        "同じまとまりの 2 件目が重複になっていない（1 件目がまだ COMMIT されていない）"
    );
    assert_eq!(count_rows(&app, &s).await, 2, "巻き戻っている");
}

/// **重複のときも、返す識別子は DB にある行のもの**（R11 / spec の MODIFIED）。
/// 送り主が名乗った識別子をそのまま返すと、**DB に無い識別子が受理として返る**。
///
/// Scenario: 重複のとき返る識別子でその記録を読み出せる
#[tokio::test]
async fn duplicate_returns_stored_id() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "dup-id", "none").await;
    let u = testdb::user();
    let raw = r#"{"seq":"who-am-i"}"#;

    let (_, res) = post(&app, serde_json::json!([ev(&s, u, raw)])).await;
    let stored_id = res[0].id.unwrap();

    // **違う収集側の識別子を名乗って同じものを送る**
    let again = ev(&s, u, raw);
    let sent_id: uuid::Uuid = again["id"].as_str().unwrap().parse().unwrap();
    let (_, res) = post(&app, serde_json::json!([again])).await;
    assert!(res[0].duplicate);
    assert_eq!(res[0].id, Some(stored_id), "送り主の識別子を返している");
    assert_ne!(res[0].id, Some(sent_id));
    // その識別子で読み出せる
    assert_eq!(raw_of(&app, stored_id).await, raw);
}

// ------------------------------------------------------------------ 更新と履歴

/// 外部サービス由来の更新を当てて、行を数える（深掘り Q1）。
async fn ingest_ext(
    app: &App,
    source: &str,
    user: uuid::Uuid,
    ext: &str,
    raw: &str,
    updated_at: Option<&str>,
) -> IngestResult {
    let mut item = with(
        ev(source, user, raw),
        "external_id",
        serde_json::json!(ext),
    );
    if let Some(t) = updated_at {
        item = with(item, "source_updated_at", serde_json::json!(t));
    }
    let (_, mut res) = post(app, serde_json::json!([item])).await;
    res.remove(0)
}

async fn versions_of(app: &App, event_id: uuid::Uuid) -> Vec<(i32, String)> {
    sqlx::query_as(
        "SELECT version_no, raw FROM core.event_version WHERE event_id = $1 ORDER BY version_no",
    )
    .bind(event_id)
    .fetch_all(&app.pool)
    .await
    .unwrap()
}

/// **Story の完了の判定 2。** 更新で行は増えず、前の版が履歴に残る。
///
/// Scenario: 更新で行が増えず、前の版が残る
#[tokio::test]
async fn external_update_keeps_one_row_and_one_version() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "ext-update", "record").await;
    let u = testdb::user();

    let first = ingest_ext(&app, &s, u, "e1", r#"{"v":1}"#, None).await;
    let id = first.id.unwrap();
    let second = ingest_ext(&app, &s, u, "e1", r#"{"v":2}"#, None).await;

    assert!(second.accepted);
    assert_eq!(second.id, Some(id), "更新なのに別の行を指している");
    assert_eq!(count_rows(&app, &s).await, 1, "更新で行が増えた");
    assert_eq!(raw_of(&app, id).await, r#"{"v":2}"#, "新しい内容になっていない");
    assert_eq!(
        versions_of(&app, id).await,
        vec![(1, r#"{"v":1}"#.to_string())],
        "前の版が履歴に残っていない"
    );
}

/// **履歴の原文もバイト単位で残る**（R39 / FR-18）。`jsonb` はキー順を変え、
/// 重複キーを落とし、数値表記を展開する —— Q1 の答えで前の版はここにしか無い。
///
/// Scenario: 履歴の原文も並び・重複・表記を保つ
#[tokio::test]
async fn version_raw_is_byte_identical() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "ver-raw", "record").await;
    let u = testdb::user();
    let weird = r#"{"b":1,"a":2,"a":3,"n":1.100,"m":1e2,"z":"  spaced  "}"#;

    let first = ingest_ext(&app, &s, u, "e1", weird, None).await;
    let id = first.id.unwrap();
    ingest_ext(&app, &s, u, "e1", r#"{"v":"next"}"#, None).await;

    let versions = versions_of(&app, id).await;
    assert_eq!(versions.len(), 1);
    assert_eq!(
        versions[0].1, weird,
        "履歴の原文が構造として解釈し直されている"
    );
    // 検査が空振りしていないこと（`jsonb` に通すと値が変わる）
    let (canon,): (String,) = sqlx::query_as("SELECT ($1::jsonb)::text")
        .bind(weird)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_ne!(canon, weird, "jsonb でも同じ値になる（検査が空振り）");
}

/// 履歴は自分の感度も削除の印も持たず、**常に親に従う**（深掘り Q21 / design D6）。
/// 読み出しは `core.event_version_live` 越しにだけ行う（R49）。
///
/// Scenario: 履歴は親の感度に従う
/// Scenario: 履歴は親の削除に従う
#[tokio::test]
async fn history_follows_parent_sensitivity_and_deletion() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "ver-live", "record").await;
    let u = testdb::user();
    let id = ingest_ext(&app, &s, u, "e1", r#"{"v":1}"#, None)
        .await
        .id
        .unwrap();
    ingest_ext(&app, &s, u, "e1", r#"{"v":2}"#, None).await;

    // **履歴表に感度と削除の列が無い**（tasks 5.2 / Q21）
    let (extra,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM information_schema.columns
          WHERE table_schema='core' AND table_name='event_version'
            AND column_name IN ('sensitivity','deleted_at','deleted_by')",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(extra, 0, "履歴が自分の感度・削除の印を持っている");

    // 親を締めると、束ねた形で読んだ履歴も締まる
    sqlx::query("UPDATE core.event SET sensitivity = 3 WHERE id = $1")
        .bind(id)
        .execute(&app.pool)
        .await
        .unwrap();
    let (sens,): (i16,) =
        sqlx::query_as(&format!("SELECT sensitivity FROM {VERSION_VIEW} WHERE event_id = $1"))
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(sens, 3, "履歴が親の感度に従っていない");

    // 親を消すと、束ねた形からも消える
    soft_delete(&app, id).await;
    let (left,): (i64,) =
        sqlx::query_as(&format!("SELECT count(*) FROM {VERSION_VIEW} WHERE event_id = $1"))
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(left, 0, "親を消しても履歴の版が読める");
}

// ------------------------------------------------------------------ 更新時刻による順序

/// Scenario: 古い版は既存行を書き換えない
#[tokio::test]
async fn stale_update_is_ignored() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "stale", "record").await;
    let u = testdb::user();
    let id = ingest_ext(&app, &s, u, "e1", r#"{"v":2}"#, Some("2026-05-02T00:00:00Z"))
        .await
        .id
        .unwrap();

    let old = ingest_ext(&app, &s, u, "e1", r#"{"v":1}"#, Some("2026-05-01T00:00:00Z")).await;
    assert!(old.accepted, "古い到着は受理として返す（再送を諦められる）");
    assert_eq!(raw_of(&app, id).await, r#"{"v":2}"#, "内容が巻き戻った");
    assert!(versions_of(&app, id).await.is_empty(), "履歴が増えた");
}

/// Scenario: 更新時刻の無い到着が保存済みの値を消さない
#[tokio::test]
async fn missing_updated_at_does_not_clear_stored_value() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "keep-ts", "record").await;
    let u = testdb::user();
    let id = ingest_ext(&app, &s, u, "e1", r#"{"v":1}"#, Some("2026-05-02T00:00:00Z"))
        .await
        .id
        .unwrap();

    ingest_ext(&app, &s, u, "e1", r#"{"v":2}"#, None).await;

    let (kept,): (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT source_updated_at FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(kept.is_some(), "保存済みの更新時刻が消えた（以後、古い版を止められない）");
    // 消えていないので、古い到着はまだ止まる
    ingest_ext(&app, &s, u, "e1", r#"{"v":0}"#, Some("2026-05-01T00:00:00Z")).await;
    assert_eq!(raw_of(&app, id).await, r#"{"v":2}"#);
}

/// Scenario: 更新時刻の無い到着は届いた順で適用される
#[tokio::test]
async fn updates_without_timestamp_apply_in_arrival_order() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "arrival", "record").await;
    let u = testdb::user();
    let id = ingest_ext(&app, &s, u, "e1", r#"{"v":1}"#, None).await.id.unwrap();
    ingest_ext(&app, &s, u, "e1", r#"{"v":2}"#, None).await;
    ingest_ext(&app, &s, u, "e1", r#"{"v":3}"#, None).await;
    assert_eq!(raw_of(&app, id).await, r#"{"v":3}"#, "届いた順で適用されていない");
    assert_eq!(versions_of(&app, id).await.len(), 2);
}

/// **`>=` で当てる**（design D8）。`>` にすると `accepted` を返しながら内容が変わらず、
/// **応答から見えない**。
///
/// Scenario: 同じ更新時刻でも内容が違えば適用される
#[tokio::test]
async fn same_updated_at_still_applies() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "same-ts", "record").await;
    let u = testdb::user();
    let t = "2026-05-02T00:00:00Z";
    let id = ingest_ext(&app, &s, u, "e1", r#"{"v":1}"#, Some(t)).await.id.unwrap();
    ingest_ext(&app, &s, u, "e1", r#"{"v":2}"#, Some(t)).await;
    assert_eq!(raw_of(&app, id).await, r#"{"v":2}"#, "同じ更新時刻で落ちている");
}

// ------------------------------------------------------------------ 削除済みの保護

/// Scenario: 削除済みへの再送は復活させない
/// Scenario: 削除済みへの再送は受理として返る
#[tokio::test]
async fn deleted_duplicate_is_accepted() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "del-resend", "none").await;
    let u = testdb::user();
    let raw = r#"{"seq":"gone"}"#;
    let id = post(&app, serde_json::json!([ev(&s, u, raw)])).await.1[0]
        .id
        .unwrap();
    soft_delete(&app, id).await;

    let (_, res) = post(&app, serde_json::json!([ev(&s, u, raw)])).await;
    assert!(res[0].accepted, "受理として返らないと永久に送られ続ける");
    assert_eq!(count_rows(&app, &s).await, 1, "行が増えた");
    let (alive,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM core.event_live WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(alive, 0, "削除済みの記録が復活した");
}

/// **Q3 で本人に「二度と入りません」と伝えた文面**が、Q6 の答えの下でも成り立つこと。
/// 内容の鍵を狭めたので、削除済みの行に対してだけは内容の鍵も当て続ける（Q19）。
///
/// Scenario: 違う外部識別子でも消した本文は入らない
#[tokio::test]
async fn deleted_content_does_not_return_via_other_external_id() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "del-ext", "record").await;
    let u = testdb::user();
    let raw = r#"{"seq":"erased"}"#;
    let id = ingest_ext(&app, &s, u, "e1", raw, None).await.id.unwrap();
    soft_delete(&app, id).await;

    let again = ingest_ext(&app, &s, u, "e2", raw, None).await;
    assert!(again.accepted, "受理として返らないと永久に送られ続ける");
    assert_eq!(count_rows(&app, &s).await, 1, "違う識別子で消した本文が戻った");
}

/// Scenario: 削除済みへの外部の更新は取り込まない
#[tokio::test]
async fn deleted_row_is_not_updated_by_external_change() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "del-update", "record").await;
    let u = testdb::user();
    let id = ingest_ext(&app, &s, u, "e1", r#"{"v":1}"#, None).await.id.unwrap();
    soft_delete(&app, id).await;

    let res = ingest_ext(&app, &s, u, "e1", r#"{"v":2}"#, None).await;
    assert!(res.accepted);
    assert_eq!(raw_of(&app, id).await, r#"{"v":1}"#, "削除済みが書き換わった");
    assert!(versions_of(&app, id).await.is_empty(), "履歴が積まれた");
    let (deleted,): (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT deleted_at FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(deleted.is_some(), "削除の印が消えた（1 行足せば復活する）");
}

/// **判定は更新の経路にも当てる**（R48）。当てないと、生きている別の行が
/// 外部からの更新で**消した本文に化ける**（`core.event_live` に見える）。
///
/// Scenario: 更新で消した本文に化けない
#[tokio::test]
async fn update_cannot_resurrect_deleted_content() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "no-morph", "record").await;
    let u = testdb::user();
    let erased_raw = r#"{"seq":"secret"}"#;

    let gone = ingest_ext(&app, &s, u, "e1", erased_raw, None).await.id.unwrap();
    soft_delete(&app, gone).await;
    let alive = ingest_ext(&app, &s, u, "e2", r#"{"seq":"innocent"}"#, None)
        .await
        .id
        .unwrap();

    // 生きている行を、消した本文へ更新しようとする
    let res = ingest_ext(&app, &s, u, "e2", erased_raw, None).await;
    assert!(res.accepted);
    assert_eq!(
        raw_of(&app, alive).await,
        r#"{"seq":"innocent"}"#,
        "生きている行が消した本文に化けた"
    );
    assert!(versions_of(&app, alive).await.is_empty());
}

/// **外部識別子を持たない記録では削除済みの判定を撃たない**（design D9）——
/// `event_dedup_hash` が削除済みの行も含めて弾くので、追加の問い合わせは要らない。
#[tokio::test]
async fn no_extra_query_without_external_id() {
    use ingest::ExternalIdKind::{None as KNone, Record, Subject};
    assert!(ingest::needs_deleted_check(Record, Some("e1")));
    assert!(!ingest::needs_deleted_check(Record, None));
    assert!(!ingest::needs_deleted_check(KNone, Some("e1")));
    assert!(!ingest::needs_deleted_check(Subject, Some("v1")));

    // 撃たなくても削除済みは弾けている（索引の側が担保している）
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "no-probe", "none").await;
    let u = testdb::user();
    let raw = r#"{"seq":"hashonly"}"#;
    let id = post(&app, serde_json::json!([ev(&s, u, raw)])).await.1[0]
        .id
        .unwrap();
    soft_delete(&app, id).await;
    post(&app, serde_json::json!([ev(&s, u, raw)])).await;
    assert_eq!(count_rows(&app, &s).await, 1, "索引が削除済みを弾いていない");
}

// ------------------------------------------------------------------ 登録簿の宣言

/// Scenario: 記録ごとと宣言したソースで識別子を欠けば断られる
#[tokio::test]
async fn record_kind_requires_external_id() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "need-ext", "record").await;
    let u = testdb::user();
    let (code, res) = post(&app, serde_json::json!([ev(&s, u, r#"{"v":1}"#)])).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(!res[0].accepted);
    assert!(
        matches!(res[0].error, Some(IngestError::MissingExternalId)),
        "理由の種別が違う: {:?}",
        res[0].error
    );
    assert_eq!(count_rows(&app, &s).await, 0);
}

/// **書き忘れは断る側へ倒す**（深掘り Q16 / Q18）。緩い側に倒すと、
/// 識別子なしで入った記録に**後から識別子を足す手段が無い**。
///
/// Scenario: 宣言の無いソースは断る側に倒れる
#[tokio::test]
async fn undeclared_source_defaults_to_record() {
    let app = app().await;
    let s = testdb::source_undeclared(&app.pool, "undeclared").await;
    let u = testdb::user();
    let (code, res) = post(&app, serde_json::json!([ev(&s, u, r#"{"v":1}"#)])).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(
        matches!(res[0].error, Some(IngestError::MissingExternalId)),
        "宣言を書き忘れたソースが緩い側へ倒れている: {:?}",
        res[0].error
    );
}

/// 登録簿の既定が `'record'`（＝断る側）であることを DB の側で固定する（tasks 1.4）。
#[tokio::test]
async fn external_id_kind_defaults_to_record() {
    let app = app().await;
    let s = testdb::source_undeclared(&app.pool, "default-kind").await;
    let (kind,): (String,) =
        sqlx::query_as("SELECT external_id_kind FROM core.source WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(kind, "record", "既定が緩い側に倒れている（深掘り Q16）");
}

/// **対象ごとの識別子は別の列に持ち、判定に使わない**（深掘り Q24）。
/// 判定に使うと、同じ対象についての 2 件目が一意違反で落ちる。
///
/// Scenario: 対象ごとの識別子は重複の判定に使われない
#[tokio::test]
async fn subject_ref_is_not_used_for_dedup() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "subject", "subject").await;
    let u = testdb::user();
    for v in ["watch-1", "watch-2"] {
        let item = with(
            with(
                ev(&s, u, &format!(r#"{{"watch":"{v}"}}"#)),
                "external_ref",
                serde_json::json!("video-42"),
            ),
            "external_id",
            serde_json::Value::Null,
        );
        let (_, res) = post(&app, serde_json::json!([item])).await;
        assert!(res[0].accepted && !res[0].duplicate, "{v} が畳まれた");
    }
    assert_eq!(count_rows(&app, &s).await, 2, "同じ対象の 2 件目が落ちた");
    let (with_ref,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM core.event WHERE logical_source = $1 AND external_ref = 'video-42'",
    )
    .bind(&s)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(with_ref, 2, "対象の識別子が保持されていない");
}

/// **Q25 の除外そのもの。** 対象ごとの識別子しか無いソースでは、
/// 外部サービス側の更新が行を増やす（完了の判定 2 はこのクラスを対象外にしている）。
///
/// Scenario: 対象ごとのソースでは更新が行を増やす
#[tokio::test]
async fn subject_scoped_update_adds_a_row() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "subject-upd", "subject").await;
    let u = testdb::user();
    for raw in [r#"{"title":"前"}"#, r#"{"title":"後"}"#] {
        let item = with(ev(&s, u, raw), "external_ref", serde_json::json!("video-7"));
        post(&app, serde_json::json!([item])).await;
    }
    assert_eq!(
        count_rows(&app, &s).await,
        2,
        "対象ごとのソースで更新が畳まれている（Q25 の除外が消えている）"
    );
}

/// 派生の作り直し（FR-31）は **ST16 の担当**で、この capability は畳まない（深掘り Q7）。
///
/// Scenario: 派生の作り直しはこの capability が畳まない
#[tokio::test]
async fn derived_rebuild_is_not_folded() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "derived", "none").await;
    let u = testdb::user();
    for raw in [r#"{"stay":"v1"}"#, r#"{"stay":"v2"}"#] {
        let item = with(
            with(ev(&s, u, raw), "origin", serde_json::json!("derived")),
            "device_id",
            serde_json::Value::Null,
        );
        let (_, res) = post(&app, serde_json::json!([item])).await;
        assert!(res[0].accepted);
    }
    assert_eq!(
        count_rows(&app, &s).await,
        2,
        "作り直した派生が畳まれている（ST16 の判断をこの capability が先取りしている）"
    );
}

/// 登録簿まわりの 3 本（tasks 8.10）。
///
/// Scenario: 未登録のソースは拒否される
/// Scenario: 登録するだけで受け付けられる
/// Scenario: 退役は日付で残る
#[tokio::test]
async fn registry_scenarios() {
    let app = app().await;
    let u = testdb::user();

    // 未登録のソースは拒否される
    let (code, res) = post(
        &app,
        serde_json::json!([ev("t-not-registered-at-all", u, r#"{"v":1}"#)]),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(matches!(res[0].error, Some(IngestError::UnknownSource)));

    // 登録簿に 1 行足すだけで受け付けられる（API のコードは変えない）
    let s = testdb::source_of_kind(&app.pool, "registry", "none").await;
    let (code, res) = post(&app, serde_json::json!([ev(&s, u, r#"{"v":1}"#)])).await;
    assert_eq!(code, StatusCode::OK);
    assert!(res[0].accepted);

    // 退役は**日付**で残る（R56。真偽値だと退役より前の本物の途絶が遡って消える）
    testdb::retire(&app.pool, &s, "2026-06-01").await;
    let (retired,): (Option<chrono::NaiveDate>,) =
        sqlx::query_as("SELECT retired_on FROM core.source WHERE logical_source = $1")
            .bind(&s)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(retired, Some(testdb::date("2026-06-01")));
}

/// **空文字の識別子を格納の前に断る**（R12）。空文字は NULL ではないので
/// 部分索引に載り、**2 件目で一意違反になってまとめ送り全体が 500 になる**。
///
/// Scenario: 空の外部識別子は断られる
#[tokio::test]
async fn empty_external_id_is_rejected() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "empty-ext", "record").await;
    let u = testdb::user();
    for i in 0..2 {
        let item = with(
            ev(&s, u, &format!(r#"{{"seq":{i}}}"#)),
            "external_id",
            serde_json::json!(""),
        );
        let (code, res) = post(&app, serde_json::json!([item])).await;
        assert_eq!(code, StatusCode::BAD_REQUEST, "{i} 件目が通った");
        assert!(
            matches!(res[0].error, Some(IngestError::EmptyExternalId)),
            "理由の種別が違う: {:?}",
            res[0].error
        );
    }
    assert_eq!(count_rows(&app, &s).await, 0);
}

// ------------------------------------------------------------------ 畳んで読む置き場

/// **置き場だけを作る**（深掘り Q8）。実際に使うのは閲覧・検索・AI・書き出しの各 Story。
///
/// Scenario: 同じ内容の複数行が 1 件として読める
#[tokio::test]
async fn folded_view_returns_one_row() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "folded", "record").await;
    let u = testdb::user();
    for ext in ["f1", "f2"] {
        ingest_ext(&app, &s, u, ext, r#"{"seq":"same-body"}"#, None).await;
    }
    assert_eq!(count_rows(&app, &s).await, 2);

    // **名前は 1 か所から引く**（`FOLDED_VIEW`）—— 移行だけを直してコード側の
    // doc コメントが古くなる、が起きないようにする
    let (folded, rows): (i64, i64) = sqlx::query_as(&format!(
        "SELECT count(*), coalesce(max(folded_rows), 0)
           FROM {FOLDED_VIEW} WHERE user_id = $1 AND logical_source = $2"
    ))
    .bind(u)
    .bind(&s)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(folded, 1, "畳んだ形で 1 件になっていない");
    assert_eq!(rows, 2, "畳んだ元の件数が出ていない");
}

// ------------------------------------------------------------------ 消去（親と履歴を同じまとまりで）

/// **消去は親とその記録のすべての履歴を同じトランザクションで**（R40 / tasks 7.4c）。
/// 分けると 2 段目（履歴だけを残す操作）がそのまま開口部になる。
///
/// 消す操作そのものは ST23 の担当で、ST03 が作るのは**台帳と門**。ここは門を確かめる。
#[tokio::test]
async fn erasure_removes_parent_and_all_versions() {
    let app = app().await;
    let s = testdb::source_of_kind(&app.pool, "erase", "record").await;
    let u = testdb::user();
    let id = ingest_ext(&app, &s, u, "e1", r#"{"v":1}"#, None).await.id.unwrap();
    ingest_ext(&app, &s, u, "e1", r#"{"v":2}"#, None).await;
    assert_eq!(versions_of(&app, id).await.len(), 1);

    // **親だけを消そうとすると COMMIT で落ちる**
    let mut tx = app.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, $3, 'event', 'test')",
    )
    .bind(id)
    .bind(u)
    .bind(&s)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}'::jsonb WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        tx.commit().await.is_err(),
        "親だけを消せた（履歴が 2 段目の開口部になる）"
    );
    assert_eq!(raw_of(&app, id).await, r#"{"v":2}"#, "落ちたのに消えている");

    // **親と履歴を同じまとまりで消せば通る**
    let mut tx = app.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, $3, 'event', 'test')",
    )
    .bind(id)
    .bind(u)
    .bind(&s)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event_version SET raw = '', payload = '{}'::jsonb WHERE event_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}'::jsonb WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.expect("台帳のある消去が通らない");

    assert_eq!(raw_of(&app, id).await, "");
    assert_eq!(versions_of(&app, id).await, vec![(1, String::new())]);
}
