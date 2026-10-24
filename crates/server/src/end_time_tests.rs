// SPDX-License-Identifier: AGPL-3.0-only
//! 幅のある出来事の終わりの時刻（ST41）を、本物の PostgreSQL に対して確かめる。
//!
//! Task 1 は門と錠と CHECK（DB の側にあるので、どの接続でも同じに拒む）。
//! `core.event` / `core.event_version` を直に書く。取り込み口を通る試験は後続の Task が足す。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use sqlx::PgPool;
use uuid::Uuid;

const START: &str = "2026-09-08T02:00:00Z";
const END: &str = "2026-09-08T03:00:00Z";
const OTHER_END: &str = "2026-09-08T05:00:00Z";

/// 記録を終わりつきで置く。`end` は（時刻の RFC3339, ずれ(分), タイムゾーン識別子）。
/// 内容の鍵は試験ごとに固有。原文と解析済みは空でない（消去の形と区別するため）。
pub async fn put_with_end(
    pool: &PgPool,
    user: Uuid,
    source: &str,
    at: &str,
    end: Option<(&str, i32, &str)>,
    external_id: Option<&str>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, end_time, end_tz_offset_min, end_tz_id,
            external_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test','collected',$4::timestamptz,540,'Asia/Tokyo',
                 $5::timestamptz,$6,$7,$8,1,$9,'{\"v\":1}','{\"v\":1}')",
    )
    .bind(id)
    .bind(user)
    .bind(source)
    .bind(at)
    .bind(end.map(|e| e.0))
    .bind(end.map(|e| e.1))
    .bind(end.map(|e| e.2))
    .bind(external_id)
    .bind(Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .unwrap();
    id
}

/// 文を 1 つのまとまり（トランザクション）で撃つ。拒まれたら（SQLSTATE, 本文）を返す。
/// 門は COMMIT の瞬間に見るので、`commit` の失敗も拾う。
async fn run(pool: &PgPool, sql: &str) -> Option<(String, String)> {
    let mut tx = pool.begin().await.unwrap();
    let result = match sqlx::raw_sql(sql).execute(&mut *tx).await {
        Ok(_) => tx.commit().await,
        Err(e) => Err(e),
    };
    result.err().map(|e| {
        let db = e.as_database_error().expect("DB の側の拒否");
        (
            db.code().map(|c| c.to_string()).unwrap_or_default(),
            db.message().to_string(),
        )
    })
}

/// 更新前の版の履歴を 1 行書く SQL。`end` は履歴に載せる終わり（3 列）。
fn version_sql(
    event: Uuid,
    user: Uuid,
    source: &str,
    version_no: i32,
    end: Option<(&str, i32, &str)>,
) -> String {
    let (t, off, tz) = match end {
        Some((t, off, tz)) => (format!("'{t}'"), off.to_string(), format!("'{tz}'")),
        None => ("NULL".into(), "NULL".into(), "NULL".into()),
    };
    format!(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
            end_time, end_tz_offset_min, end_tz_id)
         SELECT id, '{user}', '{source}', {version_no}, event_time, content_hash, raw, payload,
                {t}, {off}, {tz}
           FROM core.event WHERE id = '{event}';"
    )
}

async fn end_of(pool: &PgPool, id: Uuid) -> (Option<String>, Option<i32>, Option<String>) {
    sqlx::query_as(
        "SELECT to_char(end_time AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                end_tz_offset_min, end_tz_id FROM core.event WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn tokyo(t: &str) -> (&str, i32, &str) {
    (t, 540, "Asia/Tokyo")
}

#[tokio::test]
async fn st41_gate_end_only_rewrite_is_rejected() {
    // Scenario: 収集した記録の終わりの時刻だけを履歴なしで変えると拒まれる
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-end", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;

    // 別の時刻にする
    let (code, msg) = run(
        &pool,
        &format!("UPDATE core.event SET end_time = '{OTHER_END}' WHERE id = '{id}';"),
    )
    .await
    .expect("履歴なしで終わりを変える書き換えが通った");
    assert_eq!(code, "P0001");
    assert!(msg.contains("更新前の版"), "門の本文: {msg}");

    // 3 列とも NULL にする（CHECK ではなく門で落ちる）
    let (code, msg) = run(
        &pool,
        &format!(
            "UPDATE core.event SET end_time = NULL, end_tz_offset_min = NULL, end_tz_id = NULL
              WHERE id = '{id}';"
        ),
    )
    .await
    .expect("履歴なしで終わりを消す書き換えが通った");
    assert_eq!(code, "P0001", "CHECK でなく門で落ちる: {msg}");
    assert!(msg.contains("更新前の版"), "門の本文: {msg}");

    let (t, off, tz) = end_of(&pool, id).await;
    assert_eq!(t.as_deref(), Some(END));
    assert_eq!((off, tz.as_deref()), (Some(540), Some("Asia/Tokyo")));
}

#[tokio::test]
async fn st41_gate_version_with_only_end_different_is_rejected() {
    // Scenario: 終わりだけが違う履歴を書いても、終わりの書き換えは通らない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-ver", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;

    // 履歴は原文・内容・始まり・鍵が更新前と同じで、終わりだけが更新前と違う
    let version = version_sql(id, user, &source, 1, Some(tokyo(OTHER_END)));
    let (code, msg) = run(
        &pool,
        &format!(
            "{version}
             UPDATE core.event SET end_time = '{OTHER_END}' WHERE id = '{id}';"
        ),
    )
    .await
    .expect("終わりだけ違う履歴で終わりの書き換えが通った");
    assert_eq!(code, "P0001");
    assert!(msg.contains("更新前の版"), "門の本文: {msg}");

    assert_eq!(end_of(&pool, id).await.0.as_deref(), Some(END));
}

#[tokio::test]
async fn st41_gate_erasure_cannot_move_end() {
    // Scenario: 本文の消去で終わりの時刻を変えると拒まれる
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-erase", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;

    let ledger = format!(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ('{id}','{user}','{source}','event','test');"
    );
    let (code, _) = run(
        &pool,
        &format!(
            "{ledger}
             UPDATE core.event SET raw = '', payload = '{{}}', end_time = '{OTHER_END}'
              WHERE id = '{id}';"
        ),
    )
    .await
    .expect("消去の顔で終わりを動かす書き換えが通った");
    assert_eq!(code, "P0001");

    let (raw, payload): (String, serde_json::Value) =
        sqlx::query_as("SELECT raw, payload FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(raw, "{\"v\":1}", "本文が残っている");
    assert_eq!(payload, serde_json::json!({"v": 1}));
    assert_eq!(end_of(&pool, id).await.0.as_deref(), Some(END));
    let ledgered: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.erasure_ledger WHERE event_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ledgered, 0, "拒まれたので台帳の行も残らない");
}

#[tokio::test]
async fn st41_gate_end_tz_can_be_corrected() {
    // Scenario: 収集した記録の終わりのずれとタイムゾーンは直せる
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-tz", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;

    let rejected = run(
        &pool,
        &format!(
            "UPDATE core.event SET end_tz_offset_min = 0, end_tz_id = 'UTC' WHERE id = '{id}';"
        ),
    )
    .await;
    assert!(
        rejected.is_none(),
        "履歴なしのずれ・TZ の訂正が拒まれた: {rejected:?}"
    );

    let (t, off, tz) = end_of(&pool, id).await;
    assert_eq!(t.as_deref(), Some(END), "終わりの時刻そのものは動かない");
    assert_eq!((off, tz.as_deref()), (Some(0), Some("UTC")));
}

#[tokio::test]
async fn st41_gate_version_end_is_frozen() {
    // Scenario: 履歴の前の版の終わりは書き換えられない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-vfrozen", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;
    let wrote = run(&pool, &version_sql(id, user, &source, 1, Some(tokyo(END)))).await;
    assert!(wrote.is_none(), "終わりつきの履歴が書けない: {wrote:?}");

    for set in [
        format!("end_time = '{OTHER_END}'"),
        "end_tz_offset_min = 0".to_string(),
        "end_tz_id = 'UTC'".to_string(),
    ] {
        let (code, msg) = run(
            &pool,
            &format!("UPDATE core.event_version SET {set} WHERE event_id = '{id}';"),
        )
        .await
        .unwrap_or_else(|| panic!("前の版の終わりが書き換えられた: {set}"));
        assert_eq!(code, "P0001", "{set}: {msg}");
        assert!(msg.contains("履歴は追記のみ"), "{set}: {msg}");
    }

    let kept: (Option<String>, Option<i32>, Option<String>) = sqlx::query_as(
        "SELECT to_char(end_time AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                end_tz_offset_min, end_tz_id FROM core.event_version WHERE event_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        kept,
        (Some(END.into()), Some(540), Some("Asia/Tokyo".into()))
    );
}

#[tokio::test]
async fn st41_gate_check_rejects_partial_end_in_db() {
    // Scenario: 取り込み口を通さない書き込みでも、終わりの一部だけ・範囲外のずれ・空の識別子は入らない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-check", "none").await;
    let parent = put_with_end(&pool, user, &source, START, None, None).await;

    // (end_time, end_tz_offset_min, end_tz_id)。どれも CHECK で落ちる
    let bad = [
        ("'2026-09-08T03:00:00Z'", "NULL", "NULL"),
        ("NULL", "540", "NULL"),
        ("NULL", "NULL", "'Asia/Tokyo'"),
        ("'2026-09-08T03:00:00Z'", "540", "NULL"),
        ("'2026-09-08T03:00:00Z'", "1440", "'Asia/Tokyo'"),
        ("'2026-09-08T03:00:00Z'", "-1440", "'Asia/Tokyo'"),
        ("'2026-09-08T03:00:00Z'", "540", "''"),
    ];
    for (t, off, tz) in bad {
        let event = format!(
            "INSERT INTO core.event
               (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
                end_time, end_tz_offset_min, end_tz_id,
                schema_version, content_hash, raw, payload)
             VALUES ('{}','{user}','{source}','test','collected','{START}',540,'Asia/Tokyo',
                     {t},{off},{tz},1,'{}','{{}}','{{}}');",
            Uuid::new_v4(),
            Uuid::new_v4()
        );
        let version = format!(
            "INSERT INTO core.event_version
               (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
                end_time, end_tz_offset_min, end_tz_id)
             VALUES ('{parent}','{user}','{source}',1,'{START}','h','{{}}','{{}}',{t},{off},{tz});"
        );
        for (table, sql) in [("core.event", event), ("core.event_version", version)] {
            let (code, msg) = run(&pool, &sql)
                .await
                .unwrap_or_else(|| panic!("{table} に入った: {t} / {off} / {tz}"));
            assert_eq!(code, "23514", "{table}: {t} / {off} / {tz}: {msg}");
        }
    }

    // 端の値（±1439）は入る
    let ok = put_with_end(
        &pool,
        user,
        &source,
        START,
        Some(("2026-09-08T03:00:00Z", 1439, "X")),
        None,
    )
    .await;
    assert_eq!(end_of(&pool, ok).await.1, Some(1439));

    // 移行を 2 回当てても落ちない（CHECK の重複も作らない）
    let (name, sql) = crate::MIGRATIONS
        .iter()
        .find(|(n, _)| n.contains("event_end_time"))
        .expect("終わりの時刻の移行が MIGRATIONS にある");
    for round in 1..=2 {
        let again = sqlx::raw_sql(sql).execute(&pool).await;
        assert!(
            again.is_ok(),
            "{name} の {round} 回目の当てが落ちた: {again:?}"
        );
    }
    let checks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_constraint
          WHERE conrelid IN ('core.event'::regclass, 'core.event_version'::regclass)
            AND conname LIKE '%\\_end\\_%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(checks, 4, "CHECK は表ごとに 2 つ");
}

// ---- Task 2: 取り込み口（`crate::ingest`）を通す ----

const INGEST_TOKEN: &str = "test-token-0123456789abcdef";

fn ingest_headers() -> axum::http::HeaderMap {
    let mut h = axum::http::HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {INGEST_TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

/// 取り込み口へ 1 回送る。まとめ送りの本文は配列。
async fn send(
    pool: &PgPool,
    items: Vec<serde_json::Value>,
) -> (axum::http::StatusCode, Vec<crate::IngestResult>) {
    let app = crate::App::for_test(pool.clone(), INGEST_TOKEN);
    let (code, axum::Json(res)) = crate::ingest(
        axum::extract::State(app),
        ingest_headers(),
        axum::Json(serde_json::Value::Array(items)),
    )
    .await
    .expect("取り込み口");
    (code, res)
}

/// 終わりを持たない記録 1 件ぶんの JSON（いまの収集アプリと同じ形）。原文は呼ぶたびに固有。
fn ev(source: &str, user: Uuid, at: &str) -> serde_json::Value {
    serde_json::json!({
        "id": Uuid::new_v4(),
        "user_id": user,
        "logical_source": source,
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": at,
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": format!("{{\"v\":\"{}\"}}", Uuid::new_v4()),
        "payload": {},
    })
}

/// 終わりの 3 欄を足した記録。
fn with_end(mut item: serde_json::Value, t: &str, off: i64, tz: &str) -> serde_json::Value {
    item["end_time"] = serde_json::json!(t);
    item["end_tz_offset_min"] = serde_json::json!(off);
    item["end_tz_id"] = serde_json::json!(tz);
    item
}

fn rejected_as_invalid_end(r: &crate::IngestResult) -> bool {
    !r.accepted && serde_json::to_value(r).unwrap()["error"] == "invalid_end_time"
}

/// 格納された終わり（時刻はマイクロ秒まで）と始まりのずれ・TZ。
async fn stored_end(pool: &PgPool, id: Uuid) -> (Option<String>, Option<i32>, Option<String>) {
    sqlx::query_as(
        "SELECT to_char(end_time AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),
                end_tz_offset_min, end_tz_id FROM core.event WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn count_in(pool: &PgPool, source: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM core.event WHERE logical_source = $1")
        .bind(source)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn ingest_source(pool: &PgPool, prefix: &str) -> String {
    testdb::source_of_kind(pool, prefix, "none").await
}

#[tokio::test]
async fn st41_ingest_end_is_stored_with_offset_and_tz() {
    // Scenario: 終わりの時刻つきの記録は、終わりのずれとタイムゾーンごと残る
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-store").await;
    let item = with_end(
        ev(&source, testdb::user(), "2026-10-01T14:00:00Z"),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (code, res) = send(&pool, vec![item]).await;
    assert_eq!(code, axum::http::StatusCode::OK);
    let id = res[0].id.expect("格納された");
    assert_eq!(
        stored_end(&pool, id).await,
        (
            Some("2026-10-01T22:00:00.000000Z".into()),
            Some(540),
            Some("Asia/Tokyo".into())
        )
    );
    let start: (String, i32, String) = sqlx::query_as(
        "SELECT to_char(event_time AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                tz_offset_min, tz_id FROM core.event WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        start,
        ("2026-10-01T14:00:00Z".into(), 540, "Asia/Tokyo".into())
    );
}

#[tokio::test]
async fn st41_ingest_start_and_end_keep_their_own_tz() {
    // Scenario: 始まりと終わりのタイムゾーンが違う記録も、それぞれのまま残る
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-tz").await;
    let item = with_end(
        ev(&source, testdb::user(), "2026-10-01T14:00:00Z"),
        "2026-10-01T22:00:00Z",
        60,
        "Europe/London",
    );
    let (_, res) = send(&pool, vec![item]).await;
    let id = res[0].id.expect("格納された");
    let start: (i32, String) =
        sqlx::query_as("SELECT tz_offset_min, tz_id FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(start, (540, "Asia/Tokyo".into()));
    let end = stored_end(&pool, id).await;
    assert_eq!((end.1, end.2), (Some(60), Some("Europe/London".into())));
}

#[tokio::test]
async fn st41_ingest_partial_end_is_rejected_per_item() {
    // Scenario: 終わりの 3 つの一部だけを持つ記録は受け付けない
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-partial").await;
    let user = testdb::user();
    let at = "2026-10-01T14:00:00Z";
    let mut only_time = ev(&source, user, at);
    only_time["end_time"] = serde_json::json!("2026-10-01T22:00:00Z");
    let mut only_offset = ev(&source, user, at);
    only_offset["end_tz_offset_min"] = serde_json::json!(540);
    let mut no_tz = ev(&source, user, at);
    no_tz["end_time"] = serde_json::json!("2026-10-01T22:00:00Z");
    no_tz["end_tz_offset_min"] = serde_json::json!(540);
    let good = ev(&source, user, at);

    let (code, res) = send(&pool, vec![only_time, only_offset, no_tz, good]).await;
    assert_eq!(code, axum::http::StatusCode::OK);
    assert!(res[..3].iter().all(rejected_as_invalid_end), "{res:?}");
    assert!(res[3].accepted);
    assert_eq!(count_in(&pool, &source).await, 1);
}

#[tokio::test]
async fn st41_ingest_end_offset_out_of_range_is_rejected() {
    // Scenario: 終わりのずれが範囲の外の記録は受け付けない
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-range").await;
    let user = testdb::user();
    let items = [1440, -1440]
        .into_iter()
        .map(|off| {
            with_end(
                ev(&source, user, "2026-10-01T14:00:00Z"),
                "2026-10-01T22:00:00Z",
                off,
                "Asia/Tokyo",
            )
        })
        .collect();
    let (_, res) = send(&pool, items).await;
    assert!(res.iter().all(rejected_as_invalid_end), "{res:?}");
    assert_eq!(count_in(&pool, &source).await, 0);
}

#[tokio::test]
async fn st41_ingest_end_offset_at_edges_is_accepted() {
    // Scenario: 終わりのずれが範囲の端の記録は受け付ける
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-edge").await;
    let user = testdb::user();
    let items = [1439, -1439]
        .into_iter()
        .map(|off| {
            with_end(
                ev(&source, user, "2026-10-01T14:00:00Z"),
                "2026-10-01T22:00:00Z",
                off,
                "Asia/Tokyo",
            )
        })
        .collect();
    let (_, res) = send(&pool, items).await;
    let offsets = [1439, -1439];
    for (r, off) in res.iter().zip(offsets) {
        let id = r.id.expect("格納された");
        assert_eq!(stored_end(&pool, id).await.1, Some(off));
    }
}

#[tokio::test]
async fn st41_ingest_future_and_long_spans_are_accepted() {
    // Scenario: 終わりが未来の記録も、幅が何年もある記録も受け付ける
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-long").await;
    let user = testdb::user();
    let year_later = (chrono::Utc::now() + chrono::Duration::days(365))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let future = with_end(
        ev(&source, user, "2026-10-01T14:00:00Z"),
        &year_later,
        540,
        "Asia/Tokyo",
    );
    let long = with_end(
        ev(&source, user, "2000-01-01T00:00:00Z"),
        "2030-01-01T00:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, res) = send(&pool, vec![future, long]).await;
    let a = stored_end(&pool, res[0].id.expect("格納された")).await;
    assert_eq!(a.0, Some(year_later.replace('Z', ".000000Z")));
    let b = stored_end(&pool, res[1].id.expect("格納された")).await;
    assert_eq!(b.0, Some("2030-01-01T00:00:00.000000Z".into()));
}

#[tokio::test]
async fn st41_ingest_end_is_truncated_to_micros() {
    // Scenario: マイクロ秒より細かい桁は切り捨てて格納される
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-trunc").await;
    let item = with_end(
        ev(&source, testdb::user(), "2026-10-01T14:00:00Z"),
        "2026-10-01T22:00:00.123456789Z",
        540,
        "Asia/Tokyo",
    );
    let (_, res) = send(&pool, vec![item]).await;
    let end = stored_end(&pool, res[0].id.expect("格納された")).await;
    assert_eq!(end.0, Some("2026-10-01T22:00:00.123456Z".into()));
}

#[tokio::test]
async fn st41_ingest_empty_or_nul_end_tz_is_rejected() {
    // Scenario: 終わりのタイムゾーンが空か格納できない文字を含む記録は受け付けない
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-badtz").await;
    let user = testdb::user();
    let items = ["", "Asia/\u{0}Tokyo"]
        .into_iter()
        .map(|tz| {
            with_end(
                ev(&source, user, "2026-10-01T14:00:00Z"),
                "2026-10-01T22:00:00Z",
                540,
                tz,
            )
        })
        .collect();
    let (code, res) = send(&pool, items).await;
    assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
    assert!(res.iter().all(rejected_as_invalid_end), "{res:?}");
    assert_eq!(count_in(&pool, &source).await, 0);
}

#[tokio::test]
async fn st41_ingest_unknown_tz_name_is_accepted() {
    // Scenario: 一覧に無いタイムゾーンの名前でも終わりは受け付ける
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-unktz").await;
    let item = with_end(
        ev(&source, testdb::user(), "2026-10-01T14:00:00Z"),
        "2026-10-01T22:00:00Z",
        540,
        "Mars/Olympus_Mons",
    );
    let (_, res) = send(&pool, vec![item]).await;
    let end = stored_end(&pool, res[0].id.expect("格納された")).await;
    assert_eq!(end.2, Some("Mars/Olympus_Mons".into()));
}

#[tokio::test]
async fn st41_ingest_end_before_start_is_kept() {
    // Scenario: 終わりが始まりより前の記録も、送られたまま残る
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-rev").await;
    let item = with_end(
        ev(&source, testdb::user(), "2026-10-01T22:00:00Z"),
        "2026-10-01T14:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, res) = send(&pool, vec![item]).await;
    let end = stored_end(&pool, res[0].id.expect("格納された")).await;
    assert_eq!(end.0, Some("2026-10-01T14:00:00.000000Z".into()));
}

#[tokio::test]
async fn st41_ingest_end_equal_to_start_is_kept() {
    // Scenario: 終わりと始まりが同じ時刻の記録も受け付ける
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-same").await;
    let item = with_end(
        ev(&source, testdb::user(), "2026-10-01T14:00:00Z"),
        "2026-10-01T14:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, res) = send(&pool, vec![item]).await;
    let end = stored_end(&pool, res[0].id.expect("格納された")).await;
    assert_eq!(end.0, Some("2026-10-01T14:00:00.000000Z".into()));
}

#[tokio::test]
async fn st41_ingest_without_end_is_unchanged() {
    // Scenario: 終わりを送らない記録は、終わりを持たない記録として今のまま残る
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-ing-none").await;
    let (code, res) = send(
        &pool,
        vec![ev(&source, testdb::user(), "2026-10-01T14:00:00Z")],
    )
    .await;
    assert_eq!(code, axum::http::StatusCode::OK);
    assert!(res[0].accepted);
    // 結果の JSON は欄を足す前と同じ鍵の集合
    let json = serde_json::to_value(&res[0]).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["accepted", "duplicate", "error", "id"]);
    assert_eq!(
        stored_end(&pool, res[0].id.unwrap()).await,
        (None, None, None)
    );
}

// ---- Task 3: 同じ記録の鍵と更新の経路（Q7）----

const T0: &str = "2026-10-01T14:00:00Z";

/// 外部の識別子と原文を固定した記録。送るたびに id は別（収集側が採番し直す形）。
fn ext_item(source: &str, user: Uuid, ext: &str, raw: &str) -> serde_json::Value {
    let mut item = ev(source, user, T0);
    item["external_id"] = serde_json::json!(ext);
    item["raw"] = serde_json::json!(format!("{{\"v\":\"{raw}\"}}"));
    item
}

async fn ext_source(pool: &PgPool, prefix: &str) -> String {
    testdb::source_of_kind(pool, prefix, "record").await
}

async fn version_count(pool: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM core.event_version WHERE event_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// 履歴の最新の版の終わり（時刻はマイクロ秒まで）。
async fn last_version_end(
    pool: &PgPool,
    id: Uuid,
) -> (Option<String>, Option<i32>, Option<String>) {
    sqlx::query_as(
        "SELECT to_char(end_time AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),
                end_tz_offset_min, end_tz_id
           FROM core.event_version WHERE event_id = $1 ORDER BY version_no DESC LIMIT 1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn end_us(t: &str, off: i32, tz: &str) -> (Option<String>, Option<i32>, Option<String>) {
    (Some(t.into()), Some(off), Some(tz.into()))
}

#[tokio::test]
async fn st41_update_end_arrives_later_by_external_id() {
    // Scenario: 進行中を終わりなしで送り、同じ外部の識別子で終わりつきを送ると終わりが入る
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ext_source(&pool, "st41-upd-later").await;
    let (_, first) = send(&pool, vec![ext_item(&source, user, "sleep-1", "a")]).await;
    let id = first[0].id.expect("格納された");

    let with = with_end(
        ext_item(&source, user, "sleep-1", "a"),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, second) = send(&pool, vec![with]).await;
    assert!(second[0].accepted);
    assert_eq!(second[0].id, Some(id));
    assert_eq!(count_in(&pool, &source).await, 1, "行は増えない");
    assert_eq!(
        stored_end(&pool, id).await,
        end_us("2026-10-01T22:00:00.000000Z", 540, "Asia/Tokyo")
    );
    assert_eq!(version_count(&pool, id).await, 1);
    assert_eq!(last_version_end(&pool, id).await, (None, None, None));
}

#[tokio::test]
async fn st41_update_end_only_change_keeps_previous_end() {
    // Scenario: 終わりだけを直した版は、前の終わりを履歴に残す
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ext_source(&pool, "st41-upd-fix").await;
    let first = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T07:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, r) = send(&pool, vec![first]).await;
    let id = r[0].id.expect("格納された");

    let fixed = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T07:30:00Z",
        480,
        "Asia/Singapore",
    );
    send(&pool, vec![fixed]).await;
    assert_eq!(
        stored_end(&pool, id).await,
        end_us("2026-10-01T07:30:00.000000Z", 480, "Asia/Singapore")
    );
    assert_eq!(version_count(&pool, id).await, 1);
    assert_eq!(
        last_version_end(&pool, id).await,
        end_us("2026-10-01T07:00:00.000000Z", 540, "Asia/Tokyo")
    );
}

#[tokio::test]
async fn st41_update_version_without_end_clears_end() {
    // Scenario: 終わりを持たない版が届くと、記録の終わりは空になり前の終わりは履歴に残る
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ext_source(&pool, "st41-upd-clear").await;
    let first = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, r) = send(&pool, vec![first]).await;
    let id = r[0].id.expect("格納された");

    // 原文が違うので内容の違う版。終わりを持たない
    send(&pool, vec![ext_item(&source, user, "e1", "b")]).await;
    assert_eq!(stored_end(&pool, id).await, (None, None, None));
    assert_eq!(version_count(&pool, id).await, 1);
    assert_eq!(
        last_version_end(&pool, id).await,
        end_us("2026-10-01T22:00:00.000000Z", 540, "Asia/Tokyo")
    );
}

#[tokio::test]
async fn st41_update_resend_without_end_keeps_end() {
    // Scenario: 終わりを持たない同じ版の再送で、入った終わりは消えない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ext_source(&pool, "st41-upd-resend").await;
    let (_, r) = send(&pool, vec![ext_item(&source, user, "sleep-1", "a")]).await;
    let id = r[0].id.expect("格納された");
    let with = with_end(
        ext_item(&source, user, "sleep-1", "a"),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    send(&pool, vec![with]).await;
    assert_eq!(version_count(&pool, id).await, 1);

    send(&pool, vec![ext_item(&source, user, "sleep-1", "a")]).await;
    assert_eq!(
        stored_end(&pool, id).await,
        end_us("2026-10-01T22:00:00.000000Z", 540, "Asia/Tokyo"),
        "入った終わりが消えた"
    );
    assert_eq!(version_count(&pool, id).await, 1, "履歴が増えた");
}

#[tokio::test]
async fn st41_update_end_tz_only_change_is_not_an_update() {
    // Scenario: 終わりのずれとタイムゾーンだけが違う版は更新にならない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ext_source(&pool, "st41-upd-tz").await;
    let first = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, r) = send(&pool, vec![first]).await;
    let id = r[0].id.expect("格納された");

    let other_tz = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T22:00:00Z",
        0,
        "UTC",
    );
    send(&pool, vec![other_tz]).await;
    assert_eq!(
        stored_end(&pool, id).await,
        end_us("2026-10-01T22:00:00.000000Z", 540, "Asia/Tokyo")
    );
    assert_eq!(version_count(&pool, id).await, 0);
}

#[tokio::test]
async fn st41_update_sub_micro_end_change_is_not_an_update() {
    // Scenario: マイクロ秒より細かい桁だけが違う終わりの版は更新にならない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ext_source(&pool, "st41-upd-subus").await;
    let first = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T22:00:00.123456Z",
        540,
        "Asia/Tokyo",
    );
    let (_, r) = send(&pool, vec![first]).await;
    let id = r[0].id.expect("格納された");

    let finer = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T22:00:00.123456789Z",
        540,
        "Asia/Tokyo",
    );
    send(&pool, vec![finer]).await;
    assert_eq!(version_count(&pool, id).await, 0);
    assert_eq!(
        stored_end(&pool, id).await.0.as_deref(),
        Some("2026-10-01T22:00:00.123456Z")
    );
}

#[tokio::test]
async fn st41_update_stale_end_change_is_ignored() {
    // Scenario: 古い更新時刻の版は、終わりだけが違っても書き換えない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ext_source(&pool, "st41-upd-stale").await;
    let mut newer = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    newer["source_updated_at"] = serde_json::json!("2026-10-02T00:00:00Z");
    let (_, r) = send(&pool, vec![newer]).await;
    let id = r[0].id.expect("格納された");

    let mut older = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T23:00:00Z",
        540,
        "Asia/Tokyo",
    );
    older["source_updated_at"] = serde_json::json!("2026-10-01T12:00:00Z");
    send(&pool, vec![older]).await;
    assert_eq!(
        stored_end(&pool, id).await.0.as_deref(),
        Some("2026-10-01T22:00:00.000000Z")
    );
    assert_eq!(version_count(&pool, id).await, 0);
}

#[tokio::test]
async fn st41_update_same_end_adds_no_version() {
    // Scenario: 終わりも内容も同じ版は履歴を増やさない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ext_source(&pool, "st41-upd-same").await;
    let mk = || {
        with_end(
            ext_item(&source, user, "e1", "a"),
            "2026-10-01T22:00:00Z",
            540,
            "Asia/Tokyo",
        )
    };
    let (_, r) = send(&pool, vec![mk()]).await;
    let id = r[0].id.expect("格納された");
    send(&pool, vec![mk()]).await;
    assert_eq!(count_in(&pool, &source).await, 1);
    assert_eq!(version_count(&pool, id).await, 0);
}

#[tokio::test]
async fn st41_dedup_end_only_difference_folds_without_external_id() {
    // Scenario: 外部の識別子の無い記録で終わりだけが違う到着は畳まれる
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ingest_source(&pool, "st41-dd-fold").await;
    let mut base = ev(&source, user, T0);
    base["raw"] = serde_json::json!("{\"v\":\"same\"}");
    let (_, r) = send(&pool, vec![base.clone()]).await;
    let id = r[0].id.expect("格納された");

    let mut later = with_end(base, "2026-10-01T22:00:00Z", 540, "Asia/Tokyo");
    later["id"] = serde_json::json!(Uuid::new_v4());
    let (_, r2) = send(&pool, vec![later]).await;
    assert!(r2[0].accepted);
    assert_eq!(r2[0].id, Some(id));
    assert_eq!(count_in(&pool, &source).await, 1);
    assert_eq!(stored_end(&pool, id).await, (None, None, None));
}

async fn soft_delete(pool: &PgPool, id: Uuid) {
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn st41_dedup_deleted_stays_deleted_on_end_only_difference() {
    // Scenario: 削除済みの記録は、終わりだけが違う到着で戻らない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ingest_source(&pool, "st41-dd-del").await;
    let mut base = with_end(
        ev(&source, user, T0),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    base["raw"] = serde_json::json!("{\"v\":\"same\"}");
    let (_, r) = send(&pool, vec![base.clone()]).await;
    let id = r[0].id.expect("格納された");
    soft_delete(&pool, id).await;

    let mut other = with_end(base, "2026-10-01T23:00:00Z", 540, "Asia/Tokyo");
    other["id"] = serde_json::json!(Uuid::new_v4());
    let (_, r2) = send(&pool, vec![other]).await;
    assert!(r2[0].accepted);
    assert_eq!(count_in(&pool, &source).await, 1, "新しい行ができた");
    let live: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.event WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(live, 0, "削除済みが戻った");
}

#[tokio::test]
async fn st41_dedup_deleted_external_stays_deleted_on_end_only_difference() {
    // Scenario: 外部の識別子のある削除済みの記録も、終わりだけが違う版で戻らない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ext_source(&pool, "st41-dd-delext").await;
    let first = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, r) = send(&pool, vec![first]).await;
    let id = r[0].id.expect("格納された");
    soft_delete(&pool, id).await;

    let other = with_end(
        ext_item(&source, user, "e1", "a"),
        "2026-10-01T23:00:00Z",
        540,
        "Asia/Tokyo",
    );
    send(&pool, vec![other]).await;
    let deleted: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT deleted_at FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(deleted.is_some(), "削除済みが戻った");
    assert_eq!(
        stored_end(&pool, id).await.0.as_deref(),
        Some("2026-10-01T22:00:00.000000Z")
    );
    assert_eq!(version_count(&pool, id).await, 0);
}

// ---- Task 4: 稼働記録の日の数え方（Q4）----

/// そのソースの稼働記録（日 → 件数）。行を直に読む。
async fn coverage_days(pool: &PgPool, user: Uuid, source: &str) -> Vec<(String, i32)> {
    sqlx::query_as(
        "SELECT day::text, event_count FROM core.coverage
          WHERE user_id = $1 AND logical_source = $2 ORDER BY day",
    )
    .bind(user)
    .bind(source)
    .fetch_all(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn st41_coverage_spanning_event_counts_on_start_day() {
    // Scenario: 日をまたぐ幅のある出来事は始まりの日に 1 件だけ数えられる
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ingest_source(&pool, "st41-cov-span").await;
    // Asia/Tokyo の 10/1 23:00 〜 10/2 7:00
    let item = with_end(
        ev(&source, user, "2026-10-01T14:00:00Z"),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, res) = send(&pool, vec![item]).await;
    assert!(res[0].accepted, "{res:?}");
    assert_eq!(
        coverage_days(&pool, user, &source).await,
        vec![("2026-10-01".to_string(), 1)]
    );
}

#[tokio::test]
async fn st41_coverage_foreign_tz_counts_on_tokyo_day() {
    // Scenario: 始まりが海外の地域の記録も、Asia/Tokyo の日で数えられる
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ingest_source(&pool, "st41-cov-london").await;
    // 現地（ロンドン）は 10/1 17:00 だが、Asia/Tokyo では 10/2 01:00
    let mut item = with_end(
        ev(&source, user, "2026-10-01T16:00:00Z"),
        "2026-10-01T18:00:00Z",
        60,
        "Europe/London",
    );
    item["tz_offset_min"] = serde_json::json!(60);
    item["tz_id"] = serde_json::json!("Europe/London");
    let (_, res) = send(&pool, vec![item]).await;
    assert!(res[0].accepted, "{res:?}");
    assert_eq!(
        coverage_days(&pool, user, &source).await,
        vec![("2026-10-02".to_string(), 1)]
    );
}

#[tokio::test]
async fn st41_coverage_started_on_uses_start() {
    // Scenario: 収集を始めた日は始まりの日で決まる
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-cov-started").await;
    let item = with_end(
        ev(&source, testdb::user(), "2026-10-01T14:00:00Z"),
        "2026-10-01T22:00:00Z",
        540,
        "Asia/Tokyo",
    );
    let (_, res) = send(&pool, vec![item]).await;
    assert!(res[0].accepted, "{res:?}");
    let started: Option<String> = sqlx::query_scalar(
        "SELECT collection_started_on::text FROM core.source WHERE logical_source = $1",
    )
    .bind(&source)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(started.as_deref(), Some("2026-10-01"));
}

// ---- Task 5: 期間に重なる記録を引く（`GET /events`。Q2 / Q3）----

const API_TOKEN: &str = "test-token-0123456789abcdef";

fn api_headers() -> axum::http::HeaderMap {
    let mut h = axum::http::HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {API_TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

fn query(from: Option<&str>, to: Option<&str>, source: Option<&str>) -> crate::EventsQuery {
    crate::EventsQuery {
        from: from.map(str::to_owned),
        to: to.map(str::to_owned),
        source: source.map(str::to_owned),
    }
}

/// ハンドラを直に呼ぶ。応答の行（JSON）をそのまま返す。
async fn events_rows(pool: &PgPool, q: crate::EventsQuery) -> Vec<serde_json::Value> {
    let app = crate::App::for_test(pool.clone(), API_TOKEN);
    let axum::Json(rows) = crate::events(
        axum::extract::State(app),
        api_headers(),
        axum::extract::Query(q),
    )
    .await
    .expect("events");
    match serde_json::to_value(rows).unwrap() {
        serde_json::Value::Array(a) => a,
        other => panic!("配列でない: {other}"),
    }
}

fn ids(rows: &[serde_json::Value]) -> Vec<String> {
    rows.iter()
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect()
}

/// そのソースの記録のうち、期間に重なるものの id（出来事の時刻の順）。
async fn range_ids(pool: &PgPool, source: &str, from: &str, to: &str) -> Vec<String> {
    ids(&events_rows(pool, query(Some(from), Some(to), Some(source))).await)
}

fn at(end: &str) -> Option<(&str, i32, &str)> {
    Some((end, 540, "Asia/Tokyo"))
}

#[tokio::test]
async fn st41_range_overlapping_span_is_returned() {
    // Scenario: 期間に重なる幅のある出来事が引ける
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-range-overlap").await;
    let id = put_with_end(
        &pool,
        testdb::user(),
        &source,
        "2026-10-01T14:00:00Z",
        at("2026-10-01T22:00:00Z"),
        None,
    )
    .await;
    let rows = events_rows(
        &pool,
        query(
            Some("2026-10-01T15:00:00Z"),
            Some("2026-10-01T16:00:00Z"),
            Some(&source),
        ),
    )
    .await;
    assert_eq!(ids(&rows), vec![id.to_string()]);
    assert_eq!(rows[0]["end_time"], "2026-10-01T22:00:00Z");
    assert_eq!(rows[0]["end_tz_offset_min"], 540);
    assert_eq!(rows[0]["end_tz_id"], "Asia/Tokyo");
}

#[tokio::test]
async fn st41_range_results_are_in_event_time_order() {
    // Scenario: 期間で引いた記録は出来事の時刻の順に並ぶ
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ingest_source(&pool, "st41-range-order").await;
    // 置く順は並びと変える
    let c = put_with_end(&pool, user, &source, "2026-10-01T15:10:00Z", None, None).await;
    let b = put_with_end(
        &pool,
        user,
        &source,
        "2026-10-01T15:05:00Z",
        at("2026-10-01T15:50:00Z"),
        None,
    )
    .await;
    let a = put_with_end(
        &pool,
        user,
        &source,
        "2026-10-01T13:00:00Z",
        at("2026-10-01T15:30:00Z"),
        None,
    )
    .await;
    assert_eq!(
        range_ids(
            &pool,
            &source,
            "2026-10-01T15:00:00Z",
            "2026-10-01T16:00:00Z"
        )
        .await,
        vec![a.to_string(), b.to_string(), c.to_string()]
    );
}

#[tokio::test]
async fn st41_range_source_only_returns_all_of_source() {
    // Scenario: 期間を指定せずソースだけを指定すると、そのソースのすべての記録が返る
    let pool = testdb::pool().await;
    let user = testdb::user();
    let sleep = ingest_source(&pool, "st41-range-all-sleep").await;
    let loc = ingest_source(&pool, "st41-range-all-loc").await;
    let late = put_with_end(
        &pool,
        user,
        &sleep,
        "2026-10-03T14:00:00Z",
        at("2026-10-03T22:00:00Z"),
        None,
    )
    .await;
    let early = put_with_end(
        &pool,
        user,
        &sleep,
        "2026-10-01T14:00:00Z",
        at("2026-10-01T22:00:00Z"),
        None,
    )
    .await;
    put_with_end(&pool, user, &loc, "2026-10-02T01:00:00Z", None, None).await;
    let rows = events_rows(&pool, query(None, None, Some(&sleep))).await;
    assert_eq!(ids(&rows), vec![early.to_string(), late.to_string()]);
}

#[tokio::test]
async fn st41_range_spans_crossing_either_edge_are_returned() {
    // Scenario: 期間より前に始まり期間の中で終わる記録も、期間の中で始まり期間の後に終わる記録も返る
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ingest_source(&pool, "st41-range-edges").await;
    let before = put_with_end(
        &pool,
        user,
        &source,
        "2026-10-01T13:00:00Z",
        at("2026-10-01T15:30:00Z"),
        None,
    )
    .await;
    let after = put_with_end(
        &pool,
        user,
        &source,
        "2026-10-01T15:30:00Z",
        at("2026-10-01T18:00:00Z"),
        None,
    )
    .await;
    assert_eq!(
        range_ids(
            &pool,
            &source,
            "2026-10-01T15:00:00Z",
            "2026-10-01T16:00:00Z"
        )
        .await,
        vec![before.to_string(), after.to_string()]
    );
}

#[tokio::test]
async fn st41_range_touching_edges_do_not_overlap() {
    // Scenario: 終わりちょうど・始まりちょうどの期間には重ならない
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-range-touch").await;
    put_with_end(
        &pool,
        testdb::user(),
        &source,
        "2026-10-01T14:00:00Z",
        at("2026-10-01T22:00:00Z"),
        None,
    )
    .await;
    assert!(range_ids(
        &pool,
        &source,
        "2026-10-01T22:00:00Z",
        "2026-10-02T00:00:00Z"
    )
    .await
    .is_empty());
    assert!(range_ids(
        &pool,
        &source,
        "2026-10-01T12:00:00Z",
        "2026-10-01T14:00:00Z"
    )
    .await
    .is_empty());
}

#[tokio::test]
async fn st41_range_point_is_in_one_of_adjacent_periods() {
    // Scenario: 隣り合う 2 つの期間で同じ点の記録は片方にだけ返る
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-range-adjacent").await;
    let id = put_with_end(
        &pool,
        testdb::user(),
        &source,
        "2026-10-01T15:00:00Z",
        None,
        None,
    )
    .await;
    assert!(range_ids(
        &pool,
        &source,
        "2026-10-01T14:00:00Z",
        "2026-10-01T15:00:00Z"
    )
    .await
    .is_empty());
    assert_eq!(
        range_ids(
            &pool,
            &source,
            "2026-10-01T15:00:00Z",
            "2026-10-01T16:00:00Z"
        )
        .await,
        vec![id.to_string()]
    );
}

#[tokio::test]
async fn st41_range_overnight_span_is_on_both_days() {
    // Scenario: 日をまたぐ出来事は、重なった日の期間のどちらにも返る
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-range-overnight").await;
    let id = put_with_end(
        &pool,
        testdb::user(),
        &source,
        "2026-10-01T14:00:00Z",
        at("2026-10-01T22:00:00Z"),
        None,
    )
    .await;
    let day1 = range_ids(
        &pool,
        &source,
        "2026-09-30T15:00:00Z",
        "2026-10-01T15:00:00Z",
    )
    .await;
    let day2 = range_ids(
        &pool,
        &source,
        "2026-10-01T15:00:00Z",
        "2026-10-02T15:00:00Z",
    )
    .await;
    assert_eq!(day1, vec![id.to_string()]);
    assert_eq!(day2, vec![id.to_string()]);
}

#[tokio::test]
async fn st41_range_reversed_end_is_a_point() {
    // Scenario: 終わりが始まりより前の記録は、始まりの 1 点として扱われる
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-range-reversed").await;
    let id = put_with_end(
        &pool,
        testdb::user(),
        &source,
        "2026-10-01T22:00:00Z",
        at("2026-10-01T14:00:00Z"),
        None,
    )
    .await;
    assert!(range_ids(
        &pool,
        &source,
        "2026-10-01T15:00:00Z",
        "2026-10-01T16:00:00Z"
    )
    .await
    .is_empty());
    let rows = events_rows(
        &pool,
        query(
            Some("2026-10-01T22:00:00Z"),
            Some("2026-10-01T23:00:00Z"),
            Some(&source),
        ),
    )
    .await;
    assert_eq!(ids(&rows), vec![id.to_string()]);
    assert_eq!(rows[0]["end_time"], "2026-10-01T14:00:00Z");
}

#[tokio::test]
async fn st41_range_zero_length_is_a_point() {
    // Scenario: 終わりと始まりが同じ記録は点として扱われる
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-range-zero").await;
    let id = put_with_end(
        &pool,
        testdb::user(),
        &source,
        "2026-10-01T15:00:00Z",
        at("2026-10-01T15:00:00Z"),
        None,
    )
    .await;
    assert_eq!(
        range_ids(
            &pool,
            &source,
            "2026-10-01T15:00:00Z",
            "2026-10-01T16:00:00Z"
        )
        .await,
        vec![id.to_string()]
    );
}

#[tokio::test]
async fn st41_range_source_filter() {
    // Scenario: ソースを指定すると、そのソースの記録だけが返る
    let pool = testdb::pool().await;
    let user = testdb::user();
    let sleep = ingest_source(&pool, "st41-range-filter-sleep").await;
    let loc = ingest_source(&pool, "st41-range-filter-loc").await;
    let s = put_with_end(
        &pool,
        user,
        &sleep,
        "2026-10-01T14:00:00Z",
        at("2026-10-01T22:00:00Z"),
        None,
    )
    .await;
    let l = put_with_end(&pool, user, &loc, "2026-10-01T15:30:00Z", None, None).await;
    let (from, to) = ("2026-10-01T15:00:00Z", "2026-10-01T16:00:00Z");
    assert_eq!(
        range_ids(&pool, &sleep, from, to).await,
        vec![s.to_string()]
    );
    assert_eq!(range_ids(&pool, &loc, from, to).await, vec![l.to_string()]);
}

#[tokio::test]
async fn st41_range_deleted_is_not_returned() {
    // Scenario: 期間で引いても削除済みの記録は返らない
    let pool = testdb::pool().await;
    let source = ingest_source(&pool, "st41-range-deleted").await;
    let id = put_with_end(
        &pool,
        testdb::user(),
        &source,
        "2026-10-01T14:00:00Z",
        at("2026-10-01T22:00:00Z"),
        None,
    )
    .await;
    soft_delete(&pool, id).await;
    assert!(range_ids(
        &pool,
        &source,
        "2026-10-01T15:00:00Z",
        "2026-10-01T16:00:00Z"
    )
    .await
    .is_empty());
}

#[tokio::test]
async fn st41_range_rows_without_end_keep_six_keys() {
    // Scenario: 終わりを持たない記録の行は、いまの形のまま返る
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ingest_source(&pool, "st41-range-keys").await;
    let plain = put_with_end(&pool, user, &source, "2026-10-01T14:00:00Z", None, None).await;
    let wide = put_with_end(
        &pool,
        user,
        &source,
        "2026-10-01T15:00:00Z",
        at("2026-10-01T16:00:00Z"),
        None,
    )
    .await;
    let rows = events_rows(&pool, query(None, None, Some(&source))).await;
    let keys = |r: &serde_json::Value| {
        let mut k: Vec<String> = r.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    let by_id = |id: Uuid| {
        rows.iter()
            .find(|r| r["id"] == id.to_string())
            .expect("行")
            .clone()
    };
    assert_eq!(
        keys(&by_id(plain)),
        [
            "event_time",
            "id",
            "logical_source",
            "origin",
            "raw",
            "tz_id"
        ]
    );
    assert_eq!(
        keys(&by_id(wide)),
        [
            "end_time",
            "end_tz_id",
            "end_tz_offset_min",
            "event_time",
            "id",
            "logical_source",
            "origin",
            "raw",
            "tz_id"
        ]
    );
}

#[tokio::test]
async fn st41_range_without_period_returns_all() {
    // Scenario: 期間を指定しなければ、いまのまますべての記録が返る
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = ingest_source(&pool, "st41-range-none").await;
    let b = put_with_end(&pool, user, &source, "2026-10-01T12:00:00Z", None, None).await;
    let a = put_with_end(
        &pool,
        user,
        &source,
        "2026-10-01T10:00:00Z",
        at("2026-10-01T11:00:00Z"),
        None,
    )
    .await;
    let gone = put_with_end(&pool, user, &source, "2026-10-01T11:00:00Z", None, None).await;
    soft_delete(&pool, gone).await;
    // 共有の DB なので、他の試験の記録は混ざる。置いた id だけを取り出して並びを見る
    let rows = events_rows(&pool, query(None, None, None)).await;
    let mine: Vec<String> = ids(&rows)
        .into_iter()
        .filter(|i| [a.to_string(), b.to_string(), gone.to_string()].contains(i))
        .collect();
    assert_eq!(mine, vec![a.to_string(), b.to_string()]);
    // 文字列の辞書順は小数秒つきの記録で狂うので、時刻に読み直して比べる
    let times: Vec<chrono::DateTime<chrono::Utc>> = rows
        .iter()
        .map(|r| {
            chrono::DateTime::parse_from_rfc3339(r["event_time"].as_str().unwrap())
                .unwrap()
                .with_timezone(&chrono::Utc)
        })
        .collect();
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "時刻順");
}

#[tokio::test]
async fn st41_range_bad_period_is_400() {
    // Scenario: 期間の片方だけ・読めない時刻・逆さの期間は断られる
    let pool = testdb::pool().await;
    let app = crate::App::for_test(pool.clone(), API_TOKEN);
    let cases = [
        (Some("2026-10-01T15:00:00Z"), None),
        (Some("2026-10-01T15:00:00Z"), Some("2026-13-40")),
        (Some("2026-10-01T15:00:00"), Some("2026-10-01T16:00:00Z")),
        (Some("2026-10-01T15:00:00Z"), Some("2026-10-01T15:00:00Z")),
        (Some("2026-10-01T16:00:00Z"), Some("2026-10-01T15:00:00Z")),
    ];
    for (from, to) in cases {
        let err = crate::events(
            axum::extract::State(app.clone()),
            api_headers(),
            axum::extract::Query(query(from, to, None)),
        )
        .await
        .expect_err("断られる");
        assert_eq!(
            err.0,
            axum::http::StatusCode::BAD_REQUEST,
            "{from:?} {to:?}"
        );
        for sent in [from, to].into_iter().flatten() {
            assert!(!err.1.contains(sent), "本文に送った値を含めない");
        }
    }
}

fn get_req(uri: &str, bearer: bool) -> axum::http::Request<axum::body::Body> {
    let mut b = axum::http::Request::builder().method("GET").uri(uri);
    if bearer {
        b = b.header("authorization", format!("Bearer {API_TOKEN}"));
    }
    b.body(axum::body::Body::empty()).unwrap()
}

/// 届いた行を手元に溜めるだけの書く口（DB へは書かない）。
#[derive(Clone, Default)]
struct Capture(std::sync::Arc<std::sync::Mutex<Vec<crate::access_log::AccessEntry>>>);

impl crate::access_log::AccessSink for Capture {
    fn record(&self, entry: crate::access_log::AccessEntry) -> crate::access_log::AccessFuture {
        self.0.lock().unwrap().push(entry);
        Box::pin(async { Ok(()) })
    }
}

async fn routed(
    capture: &Capture,
    req: axum::http::Request<axum::body::Body>,
) -> axum::http::StatusCode {
    use tower::ServiceExt as _;
    let app =
        crate::App::for_test(testdb::pool().await, API_TOKEN).with_access_sink(capture.clone());
    crate::router(app).oneshot(req).await.unwrap().status()
}

const PERIOD_URI: &str = "/events?from=2026-10-01T15:00:00Z&to=2026-10-01T16:00:00Z";

#[tokio::test]
async fn st41_range_without_credentials_is_401() {
    // Scenario: 資格情報の無い期間の求めは断られる
    let capture = Capture::default();
    assert_eq!(
        routed(&capture, get_req(PERIOD_URI, false)).await,
        axum::http::StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn st41_range_is_access_logged() {
    // Scenario: 期間で引いたことは読み出しの記録に残る
    let capture = Capture::default();
    assert_eq!(
        routed(&capture, get_req(PERIOD_URI, true)).await,
        axum::http::StatusCode::OK
    );
    let seen = capture.0.lock().unwrap();
    assert_eq!(seen.len(), 1, "その求めの行が 1 行");
    assert_eq!(seen[0].route, "/events");
    assert_eq!(seen[0].outcome, "ok");
    assert_eq!(seen[0].status, 200);
}

#[tokio::test]
async fn st41_range_uses_partial_index() {
    // 印なし（D10 の確かめ）: 終わりのある側の引き方が部分索引 `event_end_range` に当たる
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    let plan = async {
        for (label, sql) in crate::MIGRATIONS {
            sqlx::raw_sql(sql)
                .execute(&fresh)
                .await
                .map_err(|e| format!("{label}: {e}"))?;
        }
        sqlx::raw_sql(
            "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, registered_at)
             VALUES ('st41-explain', 'st41-explain', 21600, '2000-01-01');
             INSERT INTO core.event
               (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
                end_time, end_tz_offset_min, end_tz_id, schema_version, content_hash, raw, payload)
             SELECT gen_random_uuid(), '00000000-0000-0000-0000-000000000041', 'st41-explain',
                    'test', 'collected',
                    '2026-01-01T00:00:00Z'::timestamptz + n * interval '5 minutes', 540, 'Asia/Tokyo',
                    CASE WHEN n % 10 = 0
                         THEN '2026-01-01T00:00:00Z'::timestamptz + n * interval '5 minutes' + interval '2 hours' END,
                    CASE WHEN n % 10 = 0 THEN 540 END,
                    CASE WHEN n % 10 = 0 THEN 'Asia/Tokyo' END,
                    1, gen_random_uuid()::text, '{\"v\":1}', '{\"v\":1}'
               FROM generate_series(1, 10000) n;
             ANALYZE core.event;",
        )
        .execute(&fresh)
        .await
        .map_err(|e| e.to_string())?;
        let (plan,): (serde_json::Value,) = sqlx::query_as(&format!(
            "EXPLAIN (FORMAT JSON) {}",
            crate::EVENTS_END_SIDE_SQL
        ))
        .bind(chrono::DateTime::parse_from_rfc3339("2026-01-10T00:00:00Z").unwrap())
        .bind(chrono::DateTime::parse_from_rfc3339("2026-01-10T06:00:00Z").unwrap())
        .fetch_one(&fresh)
        .await
        .map_err(|e| e.to_string())?;
        Ok::<_, String>(plan.to_string())
    }
    .await;
    drop.await;
    let plan = plan.unwrap();
    assert!(plan.contains("event_end_range"), "索引に当たらない: {plan}");
}

// ---- Task 7: 滞在と収集済みの記録は終わりの列を持たない（Q5 / Q6）----

#[tokio::test]
async fn st41_stays_and_collected_rows_have_no_end_column() {
    // 位置の記録を置いて滞在を作り直しても、滞在も収集済みの記録も終わりの列は NULL のまま（D13（仮）・D14（仮））
    let pool = testdb::pool().await;
    let user = testdb::user();
    // 同じ場所に 40 分、1 分おきに 41 件
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         SELECT gen_random_uuid(), $1, 'c01-location', 'test', 'collected',
                '2026-08-13T01:00:00Z'::timestamptz + make_interval(mins => i),
                540, 'Asia/Tokyo', 1, gen_random_uuid()::text,
                '{\"lat\":35.0,\"lon\":139.0,\"acc_m\":10.0}',
                '{\"lat\":35.0,\"lon\":139.0,\"acc_m\":10.0}'::jsonb
           FROM generate_series(0, 40) AS i",
    )
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();

    crate::stay_store::rebuild_all(&pool, user).await.unwrap();

    let (stays, with_end, collected): (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE logical_source = $2),
                count(*) FILTER (WHERE end_time IS NOT NULL
                                    OR end_tz_offset_min IS NOT NULL
                                    OR end_tz_id IS NOT NULL),
                count(*) FILTER (WHERE logical_source = 'c01-location')
           FROM core.event WHERE user_id = $1",
    )
    .bind(user)
    .bind(crate::stay::SOURCE)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(stays >= 1, "滞在ができていない（試験が空振り）");
    assert_eq!(collected, 41);
    assert_eq!(with_end, 0, "滞在か収集済みの記録に終わりの列が書かれた");

    let versions_with_end: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.event_version v JOIN core.event e ON e.id = v.event_id
          WHERE e.user_id = $1 AND (v.end_time IS NOT NULL
             OR v.end_tz_offset_min IS NOT NULL OR v.end_tz_id IS NOT NULL)",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(versions_with_end, 0);
}
