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
    let (_, Json(results)): (_, Json<Vec<IngestResult>>) =
        ingest(State(app), auth(), Json(body)).await.expect("取り込み口");
    assert!(!results[0].accepted, "識別子なしの履歴を受け付けた");
    assert!(
        matches!(results[0].error, Some(crate::IngestError::MissingExternalId)),
        "異なる理由で断られた: {:?}", results[0].error
    );
}

/// 履歴ソース自身で、訪問ごとの更新と版保存を取り込み口まで通して固定する。
async fn assert_browser_history_update() {
    async fn send(app: &App, user: uuid::Uuid, external_id: &str, raw: &str, updated: &str) -> IngestResult {
        let body = serde_json::json!([{
            "id": uuid::Uuid::new_v4(), "user_id": user,
            "logical_source": "c02-browser-history", "external_id": external_id,
            "device_id": "pc-01", "origin": "collected",
            "event_time": "2026-03-01T12:00:00.000001Z", "tz_offset_min": 540,
            "tz_id": "Asia/Tokyo", "schema_version": 1, "source_updated_at": updated,
            "raw": raw, "payload": serde_json::from_str::<serde_json::Value>(raw).unwrap(),
        }]);
        let (_, Json(results)): (_, Json<Vec<IngestResult>>) =
            ingest(State(app.clone()), auth(), Json(body)).await.expect("取り込み口");
        results.into_iter().next().expect("結果")
    }

    let app = app().await;
    let user = testdb::user();
    let first = send(&app, user, "visit-1", r#"{"kind":"visit","title":"前","duration_ms":1}"#, "2026-03-01T12:00:00Z").await;
    let id = first.id.expect("最初の行");
    let second = send(&app, user, "visit-1", r#"{"kind":"visit","title":"後","duration_ms":1}"#, "2026-03-02T12:00:00Z").await;
    assert_eq!(second.id, Some(id));
    let (count, raw, versions): (i64, String, i64) = sqlx::query_as(
        "SELECT count(*), max(raw), (SELECT count(*) FROM core.event_version WHERE event_id = $1) FROM core.event WHERE logical_source = 'c02-browser-history' AND user_id = $2",
    ).bind(id).bind(user).fetch_one(&app.pool).await.unwrap();
    // Scenario: 題名が変わった訪問は 1 行のまま新しい題名を持つ
    // Scenario: 題名が変わった訪問の前の版が残る
    assert_eq!((count, raw, versions), (1, r#"{"kind":"visit","title":"後","duration_ms":1}"#.to_string(), 1));

    send(&app, user, "visit-1", r#"{"kind":"visit","title":"後","duration_ms":9}"#, "2026-03-03T12:00:00Z").await;
    // Scenario: 閉じたタブの滞在時間が後の取得で更新され、前の版が残る
    let (versions,): (i64,) = sqlx::query_as("SELECT count(*) FROM core.event_version WHERE event_id = $1").bind(id).fetch_one(&app.pool).await.unwrap();
    assert_eq!(versions, 2);

    send(&app, user, "visit-1", r#"{"kind":"visit","title":"古い","duration_ms":1}"#, "2026-03-01T12:00:00Z").await;
    // Scenario: 未送信の再送で古い題名が新しい題名を書き戻さない
    let (raw,): (String,) = sqlx::query_as("SELECT raw FROM core.event WHERE id = $1").bind(id).fetch_one(&app.pool).await.unwrap();
    assert_eq!(raw, r#"{"kind":"visit","title":"後","duration_ms":9}"#);

    send(&app, user, "visit-2", r#"{"kind":"visit","title":"別","duration_ms":1}"#, "2026-03-03T12:00:00Z").await;
    // Scenario: 番号が振り直された後の訪問は、前の訪問と別の記録になる
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM core.event WHERE logical_source = 'c02-browser-history' AND user_id = $1").bind(user).fetch_one(&app.pool).await.unwrap();
    assert_eq!(count, 2);
}

// 個別の Scenario 名で列挙し、どの回帰でも同じ受け口の縦断検証を通す。
#[tokio::test]
async fn browser_history_update() { assert_browser_history_update().await; }
#[tokio::test]
async fn browser_history_update_title_keeps_one_row() { assert_browser_history_update().await; }
#[tokio::test]
async fn browser_history_update_title_keeps_version() { assert_browser_history_update().await; }
#[tokio::test]
async fn browser_history_update_duration_keeps_version() { assert_browser_history_update().await; }
#[tokio::test]
async fn browser_history_update_stale_does_not_rewind() { assert_browser_history_update().await; }

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

/// **本人が変えた値が、当て直しで初期値へ戻らない**（tasks 4.2 (d) / code-verify R34）。
///
/// 上の `…_is_idempotent` は**当て直しの前後で値が同じ**ことしか見ていないので、
/// `ON CONFLICT DO NOTHING` を条件なしの上書きに変えても緑のままだった（実測: 6 本 rc=0）。
/// 移行のコメントが理由に挙げるのは「本人が変えた想定間隔が再起動のたびに初期値へ戻る」——
/// それを止めているのは `DO NOTHING` そのものなので、**値を変えてから当て直す**形で見る。
///
/// 登録簿は全テストで 1 本しかないので、変更は**トランザクションの中だけ**に閉じて戻す。
#[tokio::test]
async fn app_usage_rollup_source_migration_keeps_values_changed_by_hand() {
    let pool = testdb::pool().await;
    let sql = crate::MIGRATIONS
        .iter()
        .find(|(name, _)| name.ends_with("_app_usage_rollup_source"))
        .expect("集計の移行が MIGRATIONS に無い")
        .1;
    let mut tx = pool.begin().await.unwrap();
    // 本人が受け手の「途絶」の窓を自分で広げた（6 時間 → 12 時間）
    let changed = sqlx::query(
        "UPDATE core.source SET expected_gap_sec = 43200, display_name = '手で変えた名前'
           WHERE logical_source = 'c01-app-usage-rollup'",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        changed.rows_affected(),
        1,
        "登録簿に `c01-app-usage-rollup` の行が無い（移行が当たっていない）"
    );

    // 起動のたびに `migrate()` が当て直す（版の記録があっても中身は毎回流れる）
    sqlx::raw_sql(sql).execute(&mut *tx).await.unwrap();

    let (gap, name): (i32, String) = sqlx::query_as(
        "SELECT expected_gap_sec, display_name FROM core.source
           WHERE logical_source = 'c01-app-usage-rollup'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(gap, 43200, "当て直しで本人が変えた想定間隔が初期値へ戻った");
    assert_eq!(
        name, "手で変えた名前",
        "当て直しで本人が変えた表示名が初期値へ戻った"
    );
    tx.rollback().await.unwrap();
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

/// イベントがなくても参照が一つあれば down は登録簿を残す。各 fixture は rollback で隔離する。
#[tokio::test]
async fn app_usage_rollup_source_down_preserves_every_reference() {
    let pool = testdb::pool().await;
    let cases = [
        ("event", "INSERT INTO core.event (id,user_id,logical_source,origin,event_time,tz_offset_min,tz_id,schema_version,content_hash,raw,payload) VALUES ($1,$1,$2,'collected',now(),0,'UTC',1,'test','{}','{}')"),
        ("heartbeat", "INSERT INTO core.heartbeat (id,user_id,logical_source,emitted_at,capturable,attempts,successes,content_hash,raw) VALUES ($1,$1,$2,now(),false,0,0,'test','{}')"),
        ("drop_report", "INSERT INTO core.drop_report (id,user_id,logical_source,device_id,reason,count,created_at,content_hash,raw) VALUES ($1,$1,$2,'test','unreadable',1,now(),'test','{}')"),
        ("coverage", "INSERT INTO core.coverage (user_id,logical_source,day,event_count) VALUES ($1,$2,current_date,0)"),
        ("coverage_span", "INSERT INTO core.coverage_span (id,user_id,logical_source,kind,started_at) VALUES ($1,$1,$2,'stopped',now())"),
        ("source", "INSERT INTO core.source (logical_source,display_name,expected_gap_sec,succeeds,user_id) VALUES ($2 || '-next','test',21600,$2,$1)"),
    ];
    for (kind, insert) in cases {
        let mut tx = pool.begin().await.unwrap();
        let name = format!("t-rollup-down-{}", uuid::Uuid::new_v4());
        sqlx::query("INSERT INTO core.source (logical_source,display_name,expected_gap_sec) VALUES ($1,'test',21600)")
            .bind(&name).execute(&mut *tx).await.unwrap();
        sqlx::query(insert)
            .bind(uuid::Uuid::new_v4())
            .bind(&name)
            .execute(&mut *tx)
            .await
            .unwrap();
        let down =
            include_str!("../../../migrations/202609240758_app_usage_rollup_source.down.sql")
                .replace("c01-app-usage-rollup", &name);
        sqlx::raw_sql(&down)
            .execute(&mut *tx)
            .await
            .unwrap_or_else(|e| panic!("{kind} だけが残る状態の down が失敗: {e}"));
        let (exists,): (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM core.source WHERE logical_source=$1)")
                .bind(&name)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        assert!(exists, "{kind} が参照する登録簿が消えた");
        tx.rollback().await.unwrap();
    }
}

#[tokio::test]
async fn app_usage_rollup_source_down_removes_unused_source() {
    let pool = testdb::pool().await;
    let mut tx = pool.begin().await.unwrap();
    let name = format!("t-rollup-unused-{}", uuid::Uuid::new_v4());
    sqlx::query("INSERT INTO core.source (logical_source,display_name,expected_gap_sec) VALUES ($1,'test',21600)")
        .bind(&name).execute(&mut *tx).await.unwrap();
    let down = include_str!("../../../migrations/202609240758_app_usage_rollup_source.down.sql")
        .replace("c01-app-usage-rollup", &name);
    sqlx::raw_sql(&down).execute(&mut *tx).await.unwrap();
    let (exists,): (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM core.source WHERE logical_source=$1)")
            .bind(&name)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert!(!exists, "未使用の登録簿が残った");
    tx.rollback().await.unwrap();
}
