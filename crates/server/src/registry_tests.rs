// SPDX-License-Identifier: AGPL-3.0-only
//! 登録簿（`core.source`）の**本物の行**を、全移行を当てた後の状態で確かめる（ST07 / design D2）。
//!
//! ここだけは**テスト用のソースを作らない** —— 見たいのは
//! 「`c02-window` がどう宣言されているか」そのもので、作った行では確かめられない。
#![allow(clippy::unwrap_used)]

use axum::{extract::State, http::HeaderMap, Json};

use crate::testdb;
use crate::{ingest, App, IngestResult};

const TOKEN: &str = "test-token-0123456789abcdef";

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

/// **`c02-window` が「記録ごと」ではない**ことを、全移行を当てた後に見る（tasks 1.1）。
///
/// FR-23 は「書き忘れたときは『記録ごと』に倒す」と定めており、登録簿の既定は
/// `'record'`（＝識別子が無ければ 400 で断る側）。`c02-window` は外部サービス上の
/// 識別子を持たないので、**宣言が `'record'` のままだと PC からの記録が 1 件も入らない。**
///
/// **列の有無で分岐させない** —— 分岐すると「まだ列が無いから合格」で素通りする。
/// **誰が倒したかにも依存しない** —— いまは ST03 の移行
/// （`202609120940_source_columns.sql`）が倒しているが、条件が将来変わっても落ちる。
#[tokio::test]
async fn c02_window_external_id_kind_is_not_record() {
    let pool = testdb::pool().await;
    let (kind,): (String,) =
        sqlx::query_as("SELECT external_id_kind FROM core.source WHERE logical_source = $1")
            .bind("c02-window")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(
        kind, "record",
        "c02-window が「記録ごと」と宣言されている（PC からの記録が全件 400 になる）"
    );
}

/// c02-browser-history は訪問ごとの外部識別子で更新・版管理する（ST08 design D7）。
#[tokio::test]
async fn browser_history_record_id_is_required() {
    let pool = testdb::pool().await;
    let (kind,): (String,) =
        sqlx::query_as("SELECT external_id_kind FROM core.source WHERE logical_source = $1")
            .bind("c02-browser-history")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(kind, "record", "履歴は訪問ごとの識別子を要求する");
}

/// 識別子のない履歴は、同じ訪問の更新先を決められないため断る。
///
/// Scenario: 識別子を欠いた履歴の記録は断られる
#[tokio::test]
async fn browser_history_without_external_id_is_rejected() {
    let app = app().await;
    let raw = r#"{"kind":"visit","at":"2026-03-01T12:00:00.000001Z","browser":"chrome"}"#;
    let body = serde_json::json!([{
        "id": uuid::Uuid::new_v4(), "user_id": testdb::user(),
        "logical_source": "c02-browser-history", "external_id": null,
        "device_id": "pc-01", "origin": "collected",
        "event_time": "2026-03-01T12:00:00.000001Z", "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo", "schema_version": 1, "raw": raw,
        "payload": serde_json::from_str::<serde_json::Value>(raw).unwrap(),
    }]);
    let (_, Json(results)): (_, Json<Vec<IngestResult>>) = ingest(State(app), auth(), Json(body))
        .await
        .expect("取り込み口");
    assert!(!results[0].accepted, "識別子なしの履歴を受け付けた");
    assert!(
        matches!(
            results[0].error,
            Some(crate::IngestError::MissingExternalId)
        ),
        "異なる理由で断られた: {:?}",
        results[0].error
    );
}

/// 履歴ソース自身で、訪問ごとの更新と版保存を取り込み口まで通して固定する。
async fn assert_browser_history_update() {
    async fn send(
        app: &App,
        user: uuid::Uuid,
        external_id: &str,
        raw: &str,
        updated: &str,
    ) -> IngestResult {
        let body = serde_json::json!([{
            "id": uuid::Uuid::new_v4(), "user_id": user,
            "logical_source": "c02-browser-history", "external_id": external_id,
            "device_id": "pc-01", "origin": "collected",
            "event_time": "2026-03-01T12:00:00.000001Z", "tz_offset_min": 540,
            "tz_id": "Asia/Tokyo", "schema_version": 1, "source_updated_at": updated,
            "raw": raw, "payload": serde_json::from_str::<serde_json::Value>(raw).unwrap(),
        }]);
        let (_, Json(results)): (_, Json<Vec<IngestResult>>) =
            ingest(State(app.clone()), auth(), Json(body))
                .await
                .expect("取り込み口");
        results.into_iter().next().expect("結果")
    }

    let app = app().await;
    let user = testdb::user();
    let first = send(
        &app,
        user,
        "visit-1",
        r#"{"kind":"visit","title":"前","duration_ms":1}"#,
        "2026-03-01T12:00:00Z",
    )
    .await;
    let id = first.id.expect("最初の行");
    let second = send(
        &app,
        user,
        "visit-1",
        r#"{"kind":"visit","title":"後","duration_ms":1}"#,
        "2026-03-02T12:00:00Z",
    )
    .await;
    assert_eq!(second.id, Some(id));
    let (count, raw, versions): (i64, String, i64) = sqlx::query_as(
        "SELECT count(*), max(raw), (SELECT count(*) FROM core.event_version WHERE event_id = $1) FROM core.event WHERE logical_source = 'c02-browser-history' AND user_id = $2",
    ).bind(id).bind(user).fetch_one(&app.pool).await.unwrap();
    // Scenario: 題名が変わった訪問は 1 行のまま新しい題名を持つ
    // Scenario: 題名が変わった訪問の前の版が残る
    assert_eq!(
        (count, raw, versions),
        (
            1,
            r#"{"kind":"visit","title":"後","duration_ms":1}"#.to_string(),
            1
        )
    );

    send(
        &app,
        user,
        "visit-1",
        r#"{"kind":"visit","title":"後","duration_ms":9}"#,
        "2026-03-03T12:00:00Z",
    )
    .await;
    // Scenario: 閉じたタブの滞在時間が後の取得で更新され、前の版が残る
    let (versions,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM core.event_version WHERE event_id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(versions, 2);

    send(
        &app,
        user,
        "visit-1",
        r#"{"kind":"visit","title":"古い","duration_ms":1}"#,
        "2026-03-01T12:00:00Z",
    )
    .await;
    // Scenario: 未送信の再送で古い題名が新しい題名を書き戻さない
    let (raw,): (String,) = sqlx::query_as("SELECT raw FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(raw, r#"{"kind":"visit","title":"後","duration_ms":9}"#);

    send(
        &app,
        user,
        "visit-2",
        r#"{"kind":"visit","title":"別","duration_ms":1}"#,
        "2026-03-03T12:00:00Z",
    )
    .await;
    // Scenario: 番号が振り直された後の訪問は、前の訪問と別の記録になる
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM core.event WHERE logical_source = 'c02-browser-history' AND user_id = $1").bind(user).fetch_one(&app.pool).await.unwrap();
    assert_eq!(count, 2);
}

// 個別の Scenario 名で列挙し、どの回帰でも同じ受け口の縦断検証を通す。
#[tokio::test]
async fn browser_history_update() {
    assert_browser_history_update().await;
}
#[tokio::test]
async fn browser_history_update_title_keeps_one_row() {
    assert_browser_history_update().await;
}
#[tokio::test]
async fn browser_history_update_title_keeps_version() {
    assert_browser_history_update().await;
}
#[tokio::test]
async fn browser_history_update_duration_keeps_version() {
    assert_browser_history_update().await;
}
#[tokio::test]
async fn browser_history_update_stale_does_not_rewind() {
    assert_browser_history_update().await;
}

/// **識別子を持たない記録が受け付けられる**（spec / FR-23 / FR-61）。
///
/// Scenario: 識別子を持たない記録が受け付けられる
#[tokio::test]
async fn window_record_without_external_id_is_accepted() {
    let app = app().await;
    let u = testdb::user();
    let raw = format!(
        r#"{{"kind":"foreground","at":"2026-03-01T12:00:00.000Z","app_name":"editor","title":"{}"}}"#,
        uuid::Uuid::new_v4()
    );
    let body = serde_json::json!([{
        "id": uuid::Uuid::new_v4(),
        "user_id": u,
        "logical_source": "c02-window",
        "external_id": null,
        "device_id": "pc-01",
        "origin": "collected",
        "event_time": "2026-03-01T12:00:00.000Z",
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": raw,
        "payload": serde_json::from_str::<serde_json::Value>(&raw).unwrap(),
    }]);
    let (code, Json(res)): (_, Json<Vec<IngestResult>>) =
        ingest(State(app.clone()), auth(), Json(body))
            .await
            .expect("取り込み口");
    assert_eq!(code, axum::http::StatusCode::OK, "{res:?}");
    assert!(res[0].accepted, "識別子が無いことを理由に断られた: {res:?}");

    // **格納されたら感度は既定（1 = 外部 AI に出してよい）**（深掘り Q3）
    let (sensitivity,): (i16,) = sqlx::query_as(
        "SELECT sensitivity FROM core.event
          WHERE logical_source = 'c02-window' AND user_id = $1",
    )
    .bind(u)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(
        sensitivity, 1,
        "ウィンドウの記録が既定より厳しい感度で入っている（QS-7 / QS-10 が答えられなくなる）"
    );
}
