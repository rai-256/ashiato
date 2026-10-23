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
