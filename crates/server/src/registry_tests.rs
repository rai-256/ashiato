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

// ================================================================ ST06 / 集計の論理ソース（tasks 4.2）

/// 集計の論理ソースが**全移行を当てた後に登録簿にある**（design D3）。
///
/// 無いと `core.event.logical_source` の外部キーで取り込みが 500 になり、
/// 端末は「受理されなかった」として未送信に残し続ける。
#[tokio::test]
async fn app_usage_rollup_source_is_registered() {
    let pool = testdb::pool().await;
    let (n, gap): (i64, i32) = sqlx::query_as(
        "SELECT count(*), coalesce(min(expected_gap_sec), 0)
           FROM core.source WHERE logical_source = $1",
    )
    .bind("c01-app-usage-rollup")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 1, "c01-app-usage-rollup が登録簿に無い");
    assert_eq!(
        gap, 21600,
        "想定間隔が 6 時間でない（生存信号の区間とずれると正常な運用が途絶に見える）"
    );
}

/// **`external_id_kind` が `'none'`**（design D3 / 独立レビュー R7）。
///
/// 既定の `'record'` のままだと `missing_external_id` で断られ、しかもその理由は
/// 「受け手側の設定で変わりうる」扱いなので**端末の未送信に永久に溜まる**。
/// **列の有無で分岐させない** —— 分岐すると「まだ列が無いから合格」で素通りする。
#[tokio::test]
async fn app_usage_rollup_source_external_id_kind_is_none() {
    let pool = testdb::pool().await;
    let (kind,): (String,) =
        sqlx::query_as("SELECT external_id_kind FROM core.source WHERE logical_source = $1")
            .bind("c01-app-usage-rollup")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        kind, "none",
        "集計のソースが識別子を要求している（端末の未送信に永久に溜まる）"
    );
}

/// **識別子なしの集計の要求が受理される**（tasks 4.2 (c)）。
///
/// 宣言だけを見ても、取り込み口が実際に通すかは分からない ——
/// 端末が積むのは `external_id` が `null` の 1 件なので、その形で通す。
#[tokio::test]
async fn app_usage_rollup_source_record_without_external_id_is_accepted() {
    let app = app().await;
    let u = testdb::user();
    let raw = format!(
        r#"{{"granularity":"daily","package":"dev.ashiato.example","begin":"2026-05-01T00:00:00Z","end":"2026-05-02T00:00:00Z","nonce":"{}"}}"#,
        uuid::Uuid::new_v4()
    );
    let body = serde_json::json!([{
        "id": uuid::Uuid::new_v4(),
        "user_id": u,
        "logical_source": "c01-app-usage-rollup",
        "external_id": null,
        "device_id": "phone-01",
        "origin": "collected",
        "event_time": "2026-05-02T00:00:00.000Z",
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
    assert!(
        res[0].accepted,
        "識別子が無いことを理由に断られた（端末の未送信に永久に溜まる）: {res:?}"
    );
}

/// **当て直しても値が変わらない**（tasks 4.2 (d)）。
///
/// `migrate()` は起動のたびに全版を当て直すので、条件なしで書く版は
/// **本人が変えた想定間隔を再起動のたびに初期値へ戻す**。
/// 行が増えないことも一緒に見る（主キーの衝突ではなく `ON CONFLICT DO NOTHING` で止まっているか）。
#[tokio::test]
async fn app_usage_rollup_source_migration_is_idempotent() {
    let pool = testdb::pool().await;
    let sql = crate::MIGRATIONS
        .iter()
        .find(|(name, _)| name.ends_with("_app_usage_rollup_source"))
        .expect("集計の移行が MIGRATIONS に無い（当て忘れると本番だけ登録簿に行が無い）")
        .1;
    // **「末尾にある」とは書かない**（並走する Story が末尾を取る）。見たいのは
    // 登録し忘れていないことと、**前提にしている版より後にあること** ——
    // `external_id_kind` の列は `202609120940_source_columns` が作る
    let names: Vec<&str> = crate::MIGRATIONS.iter().map(|(n, _)| *n).collect();
    let mine = names
        .iter()
        .position(|n| n.ends_with("_app_usage_rollup_source"))
        .expect("集計の移行が MIGRATIONS に無い");
    let columns = names
        .iter()
        .position(|n| n.ends_with("_source_columns"))
        .expect("ST03 の列の版が MIGRATIONS に無い");
    assert!(
        mine > columns,
        "集計の移行が、`external_id_kind` の列を作る版より前にある"
    );

    let before: (i64, String, i32) = read_rollup_source(&pool).await;
    sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    let after: (i64, String, i32) = read_rollup_source(&pool).await;
    assert_eq!(before, after, "当て直すと登録簿の行が変わる");
    assert_eq!(after.0, 1, "当て直すと行が増える");
}

async fn read_rollup_source(pool: &sqlx::PgPool) -> (i64, String, i32) {
    sqlx::query_as(
        "SELECT count(*), coalesce(min(external_id_kind), '?'), coalesce(min(expected_gap_sec), 0)
           FROM core.source WHERE logical_source = 'c01-app-usage-rollup'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}
