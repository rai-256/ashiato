// SPDX-License-Identifier: AGPL-3.0-only
//! 記録ごとの部品の版（ST49）の移行と門を、本物の PostgreSQL に対して確かめる。
//!
//! `core.event` / `core.event_version` を直に書く（取り込み口を通る試験は後続の Task が足す）。
//! 移行の前後を見る試験は**使い捨ての DB**（`fresh_db`）でだけ走らせる（共有の DB に当て直すと、
//! 強い錠が並んで走る他のテストと deadlock する。`media_migration_tests.rs` と同じ）。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use sqlx::PgPool;
use uuid::Uuid;

const USER: &str = "00000000-0000-0000-0000-000000000049";
const START: &str = "2026-09-08T02:00:00Z";

fn st49_index() -> usize {
    crate::MIGRATIONS
        .iter()
        .position(|(name, _)| name.ends_with("_record_component_version"))
        .expect("ST49 の移行が MIGRATIONS に無い")
}

/// ST49 の 1 つ前の版まで当てた使い捨ての DB。
async fn before_st49_db() -> (PgPool, impl std::future::Future<Output = ()>) {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    for (label, sql) in &crate::MIGRATIONS[..st49_index()] {
        sqlx::raw_sql(sql)
            .execute(&fresh)
            .await
            .unwrap_or_else(|e| panic!("{label}: {e}"));
    }
    (fresh, drop)
}

/// ST49 の移行を 2 回当てる（2 回目も落ちないことを見る）。
async fn apply_st49_twice(pool: &PgPool) {
    let sql = crate::MIGRATIONS[st49_index()].1;
    sqlx::raw_sql(sql).execute(pool).await.unwrap();
    sqlx::raw_sql(sql).execute(pool).await.unwrap();
}

/// 記録を置く（原文と解析済みは空でない）。版は 3 列（collector / web / server）で、不明は `None`。
async fn put_event(pool: &PgPool, source: &str, versions: [Option<&str>; 3]) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload,
            collector_version, web_version, server_version)
         VALUES ($1,$2::uuid,$3,'test','collected',$4::timestamptz,540,'Asia/Tokyo',1,$5,
                 '{\"v\":1}','{\"v\":1}',$6,$7,$8)",
    )
    .bind(id)
    .bind(USER)
    .bind(source)
    .bind(START)
    .bind(id.to_string())
    .bind(versions[0])
    .bind(versions[1])
    .bind(versions[2])
    .execute(pool)
    .await
    .unwrap();
    id
}

/// ST49 の前の形（版の列が無い）の記録。
async fn put_event_plain(pool: &PgPool, source: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2::uuid,$3,'test','collected',$4::timestamptz,540,'Asia/Tokyo',1,$5,
                 '{\"v\":1}','{\"v\":1}')",
    )
    .bind(id)
    .bind(USER)
    .bind(source)
    .bind(START)
    .bind(id.to_string())
    .execute(pool)
    .await
    .unwrap();
    id
}

/// 文を 1 つのまとまり（トランザクション）で撃つ。拒まれたら本文を返す。
/// 門は COMMIT の瞬間に見るので、`commit` の失敗も拾う。
async fn run(pool: &PgPool, sql: &str) -> Option<String> {
    let mut tx = pool.begin().await.unwrap();
    let result = match sqlx::raw_sql(sql).execute(&mut *tx).await {
        Ok(_) => tx.commit().await,
        Err(e) => Err(e),
    };
    result.err().map(|e| {
        e.as_database_error()
            .expect("DB の側の拒否")
            .message()
            .to_string()
    })
}

/// 親の今の値をそのまま履歴へ積む SQL（`version_no` は 1）。
fn push_version_sql(event: Uuid, source: &str) -> String {
    format!(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
            collector_version, web_version, server_version)
         SELECT id, user_id, '{source}', 1, event_time, content_hash, raw, payload,
                collector_version, web_version, server_version
           FROM core.event WHERE id = '{event}';"
    )
}

async fn versions_of(pool: &PgPool, table: &str, key: &str, id: Uuid) -> [Option<String>; 3] {
    let row: (Option<String>, Option<String>, Option<String>) = sqlx::query_as(&format!(
        "SELECT collector_version, web_version, server_version FROM {table} WHERE {key} = $1"
    ))
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    [row.0, row.1, row.2]
}

const COLUMNS: [&str; 3] = ["collector_version", "web_version", "server_version"];

/// Scenario: 版を残し始める前に入った記録の版は「不明」のまま
#[tokio::test]
async fn st49_gate_versions_start_unknown() {
    let (fresh, drop) = before_st49_db().await;
    let event = put_event_plain(&fresh, "c01-screenshot").await;
    let history = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
         SELECT id, user_id, logical_source, 1, event_time, content_hash, raw, payload
           FROM core.event WHERE id = $1 RETURNING id",
    )
    .bind(event)
    .fetch_one(&fresh)
    .await
    .unwrap();
    apply_st49_twice(&fresh).await;
    let parent = versions_of(&fresh, "core.event", "id", event).await;
    let prior = versions_of(&fresh, "core.event_version", "id", history).await;
    let live = versions_of(&fresh, "core.event_live", "id", event).await;
    drop.await;
    assert_eq!(parent, [None, None, None], "記録の版が埋められた");
    assert_eq!(prior, [None, None, None], "前の版の履歴の版が埋められた");
    assert_eq!(live, [None, None, None], "読み口で版が埋まって見える");
}

/// 4 つの台帳に行を置いて ST49 を当て、`server_version` が NULL のままであることを見る。
async fn ledger_stays_unknown(table: &str, insert: &str) {
    let (fresh, drop) = before_st49_db().await;
    sqlx::raw_sql(insert).execute(&fresh).await.unwrap();
    apply_st49_twice(&fresh).await;
    let rows: (i64, i64) = sqlx::query_as(&format!(
        "SELECT count(*), count(server_version) FROM {table}"
    ))
    .fetch_one(&fresh)
    .await
    .unwrap();
    drop.await;
    assert_eq!(rows, (1, 0), "{table}: 行が消えたか、版が埋められた");
}

/// Scenario: 版を残し始める前の削除の台帳の行は、版が「不明」のまま
#[tokio::test]
async fn st49_gate_ledgers_start_unknown_deletion() {
    ledger_stays_unknown(
        "core.deletion_ledger",
        &format!(
            "INSERT INTO core.deletion_ledger (event_id, user_id, logical_source, action, cause_event_id, mark)
             VALUES (gen_random_uuid(), '{USER}', 's01-stay', 'erase', gen_random_uuid(), 'user');"
        ),
    )
    .await;
}

/// Scenario: 版を残し始める前の気分の通知の台帳の行は、版が「不明」のまま
#[tokio::test]
async fn st49_gate_ledgers_start_unknown_reminder() {
    ledger_stays_unknown(
        "core.feeling_reminder",
        &format!("INSERT INTO core.feeling_reminder (user_id, at) VALUES ('{USER}', '21:00');"),
    )
    .await;
}

/// Scenario: 版を残し始める前の滞在の基準の台帳の行は、版が「不明」のまま
#[tokio::test]
async fn st49_gate_ledgers_start_unknown_criteria() {
    ledger_stays_unknown(
        "core.stay_criteria",
        &format!(
            "INSERT INTO core.stay_criteria (user_id, radius_m, min_minutes, gap_minutes, sources)
             VALUES ('{USER}', 100, 10, 10, ARRAY['c01-location']);"
        ),
    )
    .await;
}

/// Scenario: 版を残し始める前の個人属性の種類の名前の台帳の行は、版が「不明」のまま
#[tokio::test]
async fn st49_gate_ledgers_start_unknown_kind_name() {
    ledger_stays_unknown(
        "core.attribute_kind_name",
        &format!(
            "INSERT INTO core.attribute_kind (id, user_id)
             VALUES ('00000000-0000-0000-0000-0000000000a1', '{USER}');
             INSERT INTO core.attribute_kind_name (kind_id, user_id, name)
             VALUES ('00000000-0000-0000-0000-0000000000a1', '{USER}', '住所');"
        ),
    )
    .await;
}

/// Scenario: 収集した記録の版を、前の版の履歴なしで書き換えると拒まれる
#[tokio::test]
async fn st49_gate_version_rewrite_without_history_rejected() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "st49-gate-rewrite", 21_600).await;
    for col in COLUMNS {
        // 版のある記録も、版が不明（NULL）の記録も、履歴なしでは書き換えられない
        for before in [Some("1.0+aaaa"), None] {
            let event = put_event(&pool, &source, [before; 3]).await;
            let msg = run(
                &pool,
                &format!("UPDATE core.event SET {col} = 'x+bbbb' WHERE id = '{event}';"),
            )
            .await
            .unwrap_or_else(|| panic!("{col}: 履歴なしの版の書き換えが通った"));
            assert!(
                msg.contains("更新前の版"),
                "{col}: 拒んだのが門でない: {msg}"
            );
            let after = versions_of(&pool, "core.event", "id", event).await;
            assert_eq!(
                after,
                [
                    before.map(String::from),
                    before.map(String::from),
                    before.map(String::from)
                ]
            );
        }
    }
}

/// Scenario: 前の版を同じまとまりに積めば、収集した記録の版を直せて、前の値が残る
#[tokio::test]
async fn st49_gate_version_fix_with_history_keeps_old() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "st49-gate-fix", 21_600).await;
    // 版のある記録を直す / 版が「不明」の記録に版を入れる（NULL の照合が通る）
    for before in [Some("1.0+aaaa"), None] {
        let event = put_event(&pool, &source, [before; 3]).await;
        let sql = format!(
            "{} UPDATE core.event SET collector_version='2.0+bbbb', web_version='2.0+bbbb',
                                      server_version='2.0+bbbb' WHERE id = '{event}';",
            push_version_sql(event, &source)
        );
        assert_eq!(
            run(&pool, &sql).await,
            None,
            "前の版つきの版の直しが拒まれた"
        );
        let now = versions_of(&pool, "core.event", "id", event).await;
        assert_eq!(
            now,
            [
                Some("2.0+bbbb".to_string()),
                Some("2.0+bbbb".into()),
                Some("2.0+bbbb".into())
            ]
        );
        let kept = versions_of(&pool, "core.event_version", "event_id", event).await;
        let want = before.map(String::from);
        assert_eq!(
            kept,
            [want.clone(), want.clone(), want],
            "前の値が履歴に残っていない"
        );
    }
}

/// Scenario: 前の版の履歴の版は書き換えられない
#[tokio::test]
async fn st49_gate_history_versions_are_frozen() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "st49-gate-frozen", 21_600).await;
    let event = put_event(&pool, &source, [Some("1.0+aaaa"); 3]).await;
    assert_eq!(run(&pool, &push_version_sql(event, &source)).await, None);
    for col in COLUMNS {
        let msg = run(
            &pool,
            &format!("UPDATE core.event_version SET {col} = 'x+bbbb' WHERE event_id = '{event}';"),
        )
        .await
        .unwrap_or_else(|| panic!("{col}: 前の版の履歴の版が書き換えられた"));
        // 錠の名指し（「履歴は追記のみ」）であること。名指しを外しても後ろの「本文の消去だけ」で拒まれるため
        assert!(
            msg.contains("履歴は追記のみ"),
            "{col}: 拒んだのが錠の名指しでない: {msg}"
        );
    }
    let kept = versions_of(&pool, "core.event_version", "event_id", event).await;
    assert_eq!(
        kept,
        [
            Some("1.0+aaaa".to_string()),
            Some("1.0+aaaa".into()),
            Some("1.0+aaaa".into())
        ]
    );
}

/// Scenario: 版の列を足しても、収集した記録の原文の書き換えは拒まれる
#[tokio::test]
async fn st49_gate_raw_rewrite_still_rejected() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "st49-gate-raw", 21_600).await;
    let event = put_event(&pool, &source, [Some("1.0+aaaa"); 3]).await;
    // 履歴なし
    let msg = run(
        &pool,
        &format!("UPDATE core.event SET raw = '{{\"v\":2}}', payload = '{{\"v\":2}}' WHERE id = '{event}';"),
    )
    .await
    .expect("履歴なしの原文の書き換えが通った");
    assert!(msg.contains("更新前の版"), "拒んだのが門でない: {msg}");
    // 版だけが更新前と違う履歴では通らない（照合は 3 列まで見る）
    let stale = format!(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
         SELECT id, user_id, '{source}', 1, event_time, content_hash, raw, payload
           FROM core.event WHERE id = '{event}';
         UPDATE core.event SET raw = '{{\"v\":2}}', payload = '{{\"v\":2}}' WHERE id = '{event}';"
    );
    assert!(
        run(&pool, &stale).await.is_some(),
        "版の違う履歴で原文の書き換えが通った"
    );
    let raw: String = sqlx::query_scalar("SELECT raw FROM core.event WHERE id = $1")
        .bind(event)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(raw, "{\"v\":1}");
}

/// Scenario: 前の版の読み口と前の版の表は、記録の表と同じ版の列を持つ
#[tokio::test]
async fn st49_cols_history_and_live_views_match() {
    let pool = testdb::pool().await;
    for relation in ["event", "event_version", "event_version_live", "event_live"] {
        let cols: Vec<(String, String)> = sqlx::query_as(
            "SELECT column_name, data_type FROM information_schema.columns
              WHERE table_schema = 'core' AND table_name = $1
                AND column_name = ANY($2) ORDER BY column_name",
        )
        .bind(relation)
        .bind(COLUMNS.map(String::from).to_vec())
        .fetch_all(&pool)
        .await
        .unwrap();
        let names: Vec<&str> = cols.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(
            names,
            ["collector_version", "server_version", "web_version"],
            "core.{relation} の版の列が揃っていない"
        );
        assert!(
            cols.iter().all(|c| c.1 == "text"),
            "core.{relation}: 型が text でない"
        );
    }
}

/// 戻して進め直せる。2 回目の戻しも当たり、前の形（版の列なし）に戻る。
#[tokio::test]
async fn st49_mig_down_and_up_again() {
    let (fresh, drop) = before_st49_db().await;
    apply_st49_twice(&fresh).await;
    let down = include_str!("../../../migrations/202610251000_record_component_version.down.sql");
    let first = sqlx::raw_sql(down).execute(&fresh).await;
    let second = sqlx::raw_sql(down).execute(&fresh).await;
    let left: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns
          WHERE table_schema = 'core' AND column_name IN ('collector_version','web_version','server_version')",
    )
    .fetch_one(&fresh)
    .await
    .unwrap();
    apply_st49_twice(&fresh).await;
    drop.await;
    first.unwrap();
    second.unwrap();
    assert_eq!(left, 0, "戻した後に版の列が残っている");
}

// ------------------------------------------------------------------ 取り込み口（Task 3）

mod ingest_door {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    const TOKEN: &str = "test-token-0123456789abcdef";
    const COMMIT_A: &str = "0123456789abcdef0123456789abcdef01234567";
    const COMMIT_B: &str = "89abcdef0123456789abcdef0123456789abcdef";
    const COMMIT_D: &str = "fedcba9876543210fedcba9876543210fedcba98";

    async fn app() -> crate::App {
        crate::App::for_test(testdb::pool().await, TOKEN)
    }

    /// 1 件の記録。`version` が `None` なら欄ごと持たない（いまの収集アプリと同じ形）。
    fn record(
        source: &str,
        user: Uuid,
        raw: &str,
        version: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let mut v = serde_json::json!({
            "id": Uuid::new_v4(),
            "user_id": user,
            "logical_source": source,
            "external_id": null,
            "device_id": "test-dev",
            "origin": "collected",
            "event_time": START,
            "tz_offset_min": 540,
            "tz_id": "Asia/Tokyo",
            "schema_version": 1,
            "raw": raw,
            "payload": {},
        });
        if let Some(version) = version {
            v["client_version"] = version;
        }
        v
    }

    fn post(body: &serde_json::Value, credential: (&str, String)) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri("/ingest")
            .header(header::CONTENT_TYPE, "application/json")
            .header(credential.0, credential.1)
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn api_token() -> (&'static str, String) {
        ("authorization", format!("Bearer {TOKEN}"))
    }

    /// 画面のログイン（`/session`）で印を得て、cookie の資格情報にする。
    async fn web_login(app: &crate::App, password: &str) -> (&'static str, String) {
        let app = app.clone();
        let res = crate::router(app)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/session")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "password": password }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        let set_cookie = res.headers()[header::SET_COOKIE].to_str().unwrap();
        let token = set_cookie.split(';').next().unwrap();
        ("cookie", token.to_owned())
    }

    async fn send(
        app: &crate::App,
        body: &serde_json::Value,
        credential: (&str, String),
    ) -> (StatusCode, serde_json::Value) {
        let res = crate::router(app.clone())
            .oneshot(post(body, credential))
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    async fn versions(app: &crate::App, id: &serde_json::Value) -> [Option<String>; 3] {
        let id: Uuid = id.as_str().unwrap().parse().unwrap();
        versions_of(&app.pool, "core.event", "id", id).await
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_owned())
    }

    #[tokio::test]
    // Scenario: 収集アプリの記録は、収集アプリの版と書いた S-01 の版を持つ
    async fn st49_ingest_collector_version_and_server_version() {
        let app = app().await;
        let source = testdb::source(&app.pool, "st49col", 21_600).await;
        let version = format!("0.1.0+{COMMIT_A}.debug");
        let body = record(
            &source,
            testdb::user(),
            r#"{"a":1}"#,
            Some(version.clone().into()),
        );
        let (code, res) = send(&app, &serde_json::json!([body]), api_token()).await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(
            versions(&app, &res[0]["id"]).await,
            [s(&version), None, s(crate::BUILD_STAMP)]
        );
    }

    #[tokio::test]
    // Scenario: 記録の S-01 の版は、その S-01 の組み立ての印と同じで、印の形を持つ
    async fn st49_ingest_server_version_is_build_stamp() {
        let app = app().await;
        let source = testdb::source(&app.pool, "st49stamp", 21_600).await;
        let body = record(&source, testdb::user(), r#"{"a":2}"#, None);
        let (_, res) = send(&app, &serde_json::json!([body]), api_token()).await;
        let [_, _, server] = versions(&app, &res[0]["id"]).await;
        let server = server.expect("S-01 の版が入っていない");
        assert_eq!(
            server,
            crate::BUILD_STAMP,
            "組み立ての印と一字一句同じでない"
        );
        // 印の形（D5）: ^[0-9][^+]*\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?$
        let (number, rest) = server.split_once('+').expect("`+` が無い");
        assert!(number.starts_with(|c: char| c.is_ascii_digit()), "{server}");
        let rest = rest.strip_suffix(".debug").unwrap_or(rest);
        let rest = rest.strip_suffix(".dirty").unwrap_or(rest);
        assert!(
            rest == "unknown"
                || (rest.len() == 40 && rest.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))),
            "commit の形が違う: {server}"
        );
    }

    #[tokio::test]
    // Scenario: 書庫の取り込みの記録は、書いた S-01 の版だけを持つ
    async fn st49_ingest_archive_gets_server_version_only() {
        let pool = testdb::pool().await;
        let source = testdb::source(&pool, "st49arc", 21_600).await;
        let item = record(&source, testdb::user(), r#"{"a":3}"#, None);
        let request: crate::IngestRequest = serde_json::from_value(item).unwrap();
        let crate::StoreOutcome::Inserted(id) = crate::store_one(&pool, request).await.unwrap()
        else {
            panic!("格納されなかった");
        };
        assert_eq!(
            versions_of(&pool, "core.event", "id", id).await,
            [None, None, s(crate::BUILD_STAMP)]
        );
    }

    #[tokio::test]
    // Scenario: 部品の版の欄を持たない要求の記録は、部品の版が「不明」で入る
    async fn st49_ingest_without_field_is_unchanged() {
        let app = app().await;
        let source = testdb::source(&app.pool, "st49old", 21_600).await;
        let body = record(&source, testdb::user(), r#"{"a":4}"#, None);
        assert!(body.get("client_version").is_none());
        let (code, res) = send(&app, &serde_json::json!([body]), api_token()).await;
        assert_eq!(code, StatusCode::OK);
        let mut keys: Vec<&str> = res[0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["accepted", "duplicate", "error", "id"]);
        assert_eq!(res[0]["accepted"], true);
        assert_eq!(
            versions(&app, &res[0]["id"]).await,
            [None, None, s(crate::BUILD_STAMP)]
        );
    }

    #[tokio::test]
    // Scenario: 部品の版が空か形の違う文字列でも、記録は断られない
    async fn st49_ingest_empty_or_odd_version_is_accepted() {
        let app = app().await;
        let source = testdb::source(&app.pool, "st49odd", 21_600).await;
        let user = testdb::user();
        let body = serde_json::json!([
            record(&source, user, r#"{"a":5}"#, Some("".into())),
            record(&source, user, r#"{"a":6}"#, Some(" not a version ".into())),
        ]);
        let (code, res) = send(&app, &body, api_token()).await;
        assert_eq!(code, StatusCode::OK);
        assert!(res
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["accepted"] == true));
        assert_eq!(versions(&app, &res[0]["id"]).await[0], None);
        assert_eq!(versions(&app, &res[1]["id"]).await[0], s(" not a version "));
    }

    #[tokio::test]
    // Scenario: 格納できない文字を含む部品の版は、その文字を取り除いて残る
    async fn st49_ingest_nul_is_removed() {
        let app = app().await;
        let source = testdb::source(&app.pool, "st49nul", 21_600).await;
        let user = testdb::user();
        let body = serde_json::json!([
            record(
                &source,
                user,
                r#"{"a":7}"#,
                Some(format!("0.1.0+\u{0}{COMMIT_A}").into())
            ),
            record(&source, user, r#"{"a":8}"#, None),
            record(&source, user, r#"{"a":9}"#, Some("\u{0}".into())),
        ]);
        let (code, res) = send(&app, &body, api_token()).await;
        assert_eq!(code, StatusCode::OK);
        assert!(res
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["accepted"] == true));
        assert_eq!(
            versions(&app, &res[0]["id"]).await[0],
            s(&format!("0.1.0+{COMMIT_A}"))
        );
        assert_eq!(
            versions(&app, &res[2]["id"]).await[0],
            None,
            "取り除いて空なら不明"
        );
    }

    #[tokio::test]
    // Scenario: 画面のログインの要求の部品の版は、画面の版として入る
    async fn st49_ingest_web_session_goes_to_web_version() {
        let password = format!("pw-{}", Uuid::new_v4());
        let app = app().await.with_web_login(&password, 1);
        let source = testdb::source(&app.pool, "st49web", 21_600).await;
        let version = format!("0.1.0+{COMMIT_B}");
        let body = record(
            &source,
            testdb::user(),
            r#"{"a":10}"#,
            Some(version.clone().into()),
        );
        let credential = web_login(&app, &password).await;
        let (code, res) = send(&app, &serde_json::json!([body]), credential).await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(
            versions(&app, &res[0]["id"]).await,
            [None, s(&version), s(crate::BUILD_STAMP)]
        );
    }

    #[tokio::test]
    // Scenario: 合言葉でもログインでもない書き手の部品の版は、どこにも入らない
    async fn st49_ingest_internal_writer_drops_client_version() {
        let pool = testdb::pool().await;
        let source = testdb::source(&pool, "st49int", 21_600).await;
        let item = record(
            &source,
            testdb::user(),
            r#"{"a":11}"#,
            Some(format!("0.1.0+{COMMIT_A}").into()),
        );
        let request: crate::IngestRequest = serde_json::from_value(item).unwrap();
        let crate::StoreOutcome::Inserted(id) = crate::store_one(&pool, request).await.unwrap()
        else {
            panic!("格納されなかった");
        };
        assert_eq!(
            versions_of(&pool, "core.event", "id", id).await,
            [None, None, s(crate::BUILD_STAMP)]
        );
    }

    #[tokio::test]
    // Scenario: 部品の版だけが違う同じ記録は、1 件のまま
    async fn st49_dedup_version_only_difference_folds() {
        let app = app().await;
        let source = testdb::source(&app.pool, "st49dup", 21_600).await;
        let user = testdb::user();
        let first = format!("0.1.0+{COMMIT_A}");
        let second = format!("0.1.0+{COMMIT_D}");
        let (_, one) = send(
            &app,
            &serde_json::json!([record(
                &source,
                user,
                r#"{"a":12}"#,
                Some(first.clone().into())
            )]),
            api_token(),
        )
        .await;
        let (_, two) = send(
            &app,
            &serde_json::json!([record(&source, user, r#"{"a":12}"#, Some(second.into()))]),
            api_token(),
        )
        .await;
        assert_eq!(two[0]["duplicate"], true);
        assert_eq!(two[0]["id"], one[0]["id"]);
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = $2",
        )
        .bind(user)
        .bind(&source)
        .fetch_one(&app.pool)
        .await
        .unwrap();
        assert_eq!(count, 1);
        assert_eq!(versions(&app, &one[0]["id"]).await[0], s(&first));
    }
}

// ------------------------------------------------------------------ 外部サービス由来の更新（Task 4）

mod external_update {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    const TOKEN: &str = "test-token-0123456789abcdef";
    const COMMIT_A: &str = "0123456789abcdef0123456789abcdef01234567";
    const COMMIT_B: &str = "89abcdef0123456789abcdef0123456789abcdef";

    async fn app() -> crate::App {
        crate::App::for_test(testdb::pool().await, TOKEN)
    }

    /// 外部の識別子を持つ記録（粒度 `record` のソース宛て）。
    fn record(source: &str, user: Uuid, ext: &str, raw: &str, updated: &str) -> serde_json::Value {
        serde_json::json!({
            "id": Uuid::new_v4(),
            "user_id": user,
            "logical_source": source,
            "external_id": ext,
            "device_id": "test-dev",
            "origin": "collected",
            "event_time": START,
            "tz_offset_min": 540,
            "tz_id": "Asia/Tokyo",
            "schema_version": 1,
            "raw": raw,
            "payload": {},
            "source_updated_at": updated,
            "client_version": format!("0.1.0+{COMMIT_B}"),
        })
    }

    async fn send(app: &crate::App, body: &serde_json::Value) -> serde_json::Value {
        let res = crate::router(app.clone())
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/ingest")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .body(Body::from(serde_json::json!([body]).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()[0].clone()
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_owned())
    }

    /// 「印 A の S-01 が入れた記録」を直に置く（INSERT は門に掛からない）。
    async fn seed_old(pool: &PgPool, source: &str, user: Uuid, ext: &str, updated: &str) -> Uuid {
        let id = Uuid::new_v4();
        let old = format!("0.1.0+{COMMIT_A}");
        sqlx::query(
            "INSERT INTO core.event
               (id, user_id, logical_source, external_id, external_ref, device_id, origin,
                event_time, tz_offset_min, tz_id, schema_version, content_hash, raw, payload,
                source_updated_at, collector_version, server_version)
             VALUES ($1,$2,$3,$4,$4,'test-dev','collected',$5::timestamptz,540,'Asia/Tokyo',1,
                     $6,'{\"v\":1}','{}',$7::timestamptz,$8,$8)",
        )
        .bind(id)
        .bind(user)
        .bind(source)
        .bind(ext)
        .bind(START)
        .bind(id.to_string())
        .bind(updated)
        .bind(&old)
        .execute(pool)
        .await
        .unwrap();
        id
    }

    fn old_versions() -> [Option<String>; 3] {
        let old = format!("0.1.0+{COMMIT_A}");
        [s(&old), None, s(&old)]
    }

    #[tokio::test]
    // Scenario: 外部サービス由来の更新で、版が新しい到着の版に替わり、前の版が履歴に残る
    async fn st49_update_versions_move_and_history_keeps_old() {
        let app = app().await;
        let user = testdb::user();
        let source = testdb::source_of_kind(&app.pool, "st49upd", "record").await;
        let id = seed_old(&app.pool, &source, user, "ext-1", "2026-09-08T03:00:00Z").await;
        let res = send(
            &app,
            &record(&source, user, "ext-1", r#"{"v":2}"#, "2026-09-08T04:00:00Z"),
        )
        .await;
        assert_eq!(res["accepted"], true, "{res}");
        assert_eq!(
            versions_of(&app.pool, "core.event", "id", id).await,
            [s(&format!("0.1.0+{COMMIT_B}")), None, s(crate::BUILD_STAMP)]
        );
        assert_eq!(
            versions_of(&app.pool, "core.event_version", "event_id", id).await,
            old_versions()
        );
        assert_eq!(
            versions_of(&app.pool, "core.event_version_live", "event_id", id).await,
            old_versions()
        );
    }

    /// 本表の版が動かず、前の版の履歴も `history_rows` 行のまま増えない。
    async fn assert_unchanged(app: &crate::App, id: Uuid, history_rows: i64) {
        assert_eq!(
            versions_of(&app.pool, "core.event", "id", id).await,
            old_versions()
        );
        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM core.event_version WHERE event_id = $1")
                .bind(id)
                .fetch_one(&app.pool)
                .await
                .unwrap();
        assert_eq!(rows, history_rows, "前の版の履歴が増えた");
    }

    #[tokio::test]
    // Scenario: 内容の変わらない再到着では、版は変わらない
    async fn st49_update_same_content_keeps_versions() {
        let app = app().await;
        let user = testdb::user();
        let source = testdb::source_of_kind(&app.pool, "st49same", "record").await;
        let first = record(&source, user, "ext-1", r#"{"v":1}"#, "2026-09-08T03:00:00Z");
        let res = send(&app, &first).await;
        let id: Uuid = res["id"].as_str().unwrap().parse().unwrap();
        let a = format!("0.1.0+{COMMIT_A}");
        let fix = format!(
            "{} UPDATE core.event SET collector_version = '{a}', server_version = '{a}'
              WHERE id = '{id}';",
            push_version_sql(id, &source)
        );
        assert_eq!(run(&app.pool, &fix).await, None);
        let mut again = first.clone();
        again["id"] = serde_json::json!(Uuid::new_v4());
        again["source_updated_at"] = serde_json::json!("2026-09-08T05:00:00Z");
        send(&app, &again).await;
        assert_unchanged(&app, id, 1).await;
    }

    #[tokio::test]
    // Scenario: 古い更新時刻の到着では、版は変わらない
    async fn st49_update_stale_keeps_versions() {
        let app = app().await;
        let user = testdb::user();
        let source = testdb::source_of_kind(&app.pool, "st49stale", "record").await;
        let id = seed_old(&app.pool, &source, user, "ext-1", "2026-09-08T03:00:00Z").await;
        send(
            &app,
            &record(&source, user, "ext-1", r#"{"v":2}"#, "2026-09-08T02:00:00Z"),
        )
        .await;
        assert_unchanged(&app, id, 0).await;
    }

    #[tokio::test]
    // Scenario: 削除済みの記録への更新では、版は変わらない
    async fn st49_update_deleted_keeps_versions() {
        let app = app().await;
        let user = testdb::user();
        let source = testdb::source_of_kind(&app.pool, "st49del", "record").await;
        let id = seed_old(&app.pool, &source, user, "ext-1", "2026-09-08T03:00:00Z").await;
        sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
            .bind(id)
            .execute(&app.pool)
            .await
            .unwrap();
        send(
            &app,
            &record(&source, user, "ext-1", r#"{"v":2}"#, "2026-09-08T04:00:00Z"),
        )
        .await;
        assert_unchanged(&app, id, 0).await;
    }
}

// ------------------------------------------------------------------ 滞在と滞在の基準（Task 5）

mod stay_versions {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    const COMMIT_A: &str = "0123456789abcdef0123456789abcdef01234567";

    fn s(v: &str) -> Option<String> {
        Some(v.to_owned())
    }

    /// `at` から 60 秒ごとに `minutes + 1` 件、同じ地点の位置の記録を直に置く（作り直しの規則だけを見る）。
    async fn put_dwell(pool: &PgPool, user: Uuid, at: &str, minutes: i64) {
        let t0: chrono::DateTime<chrono::Utc> = at.parse().unwrap();
        let times: Vec<_> = (0..=minutes)
            .map(|i| t0 + chrono::Duration::minutes(i))
            .collect();
        let raws: Vec<String> = times
            .iter()
            .map(|_| r#"{"lat":35.0,"lon":139.0,"acc_m":10}"#.to_owned())
            .collect();
        sqlx::query(
            "INSERT INTO core.event
               (id, user_id, logical_source, device_id, origin, event_time,
                tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
             SELECT gen_random_uuid(), $1, 'c01-location', 'test', 'collected', t,
                    540, 'Asia/Tokyo', 1, gen_random_uuid()::text, r, r::jsonb
               FROM unnest($2::timestamptz[], $3::text[]) AS u(t, r)",
        )
        .bind(user)
        .bind(&times)
        .bind(&raws)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn the_stay(pool: &PgPool, user: Uuid) -> Uuid {
        sqlx::query_scalar(
            "SELECT id FROM core.event
              WHERE user_id = $1 AND logical_source = 's01-stay' AND origin = 'derived'",
        )
        .bind(user)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    /// 滞在は収集した記録でないので、版を直に書き換えてよい（門に掛からない。D21（仮））。
    async fn pretend_written_by_old_server(pool: &PgPool, id: Uuid) -> String {
        let old = format!("0.1.0+{COMMIT_A}");
        sqlx::query("UPDATE core.event SET server_version = $2 WHERE id = $1")
            .bind(id)
            .bind(&old)
            .execute(pool)
            .await
            .unwrap();
        old
    }

    async fn history_rows(pool: &PgPool, id: Uuid) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM core.event_version WHERE event_id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    // Scenario: 新しい滞在は、書いた S-01 の版を持つ
    async fn st49_stay_new_has_server_version() {
        let pool = testdb::pool().await;
        let user = testdb::user();
        put_dwell(&pool, user, "2026-09-08T10:00:00+09:00", 20).await;
        crate::stay_store::rebuild_all(&pool, user).await.unwrap();
        let id = the_stay(&pool, user).await;
        assert_eq!(
            versions_of(&pool, "core.event", "id", id).await,
            [None, None, s(crate::BUILD_STAMP)]
        );
    }

    #[tokio::test]
    // Scenario: 作り直しで原文が変わった滞在は、版が替わり、前の版が履歴に残る
    async fn st49_stay_rebuild_with_new_raw_moves_version() {
        let pool = testdb::pool().await;
        let user = testdb::user();
        put_dwell(&pool, user, "2026-09-08T10:00:00+09:00", 20).await;
        crate::stay_store::rebuild_all(&pool, user).await.unwrap();
        let id = the_stay(&pool, user).await;
        let old = pretend_written_by_old_server(&pool, id).await;
        // 滞在が 10 分伸びる（原文の終わりが変わる）
        put_dwell(&pool, user, "2026-09-08T10:21:00+09:00", 10).await;
        crate::stay_store::rebuild_all(&pool, user).await.unwrap();
        assert_eq!(
            versions_of(&pool, "core.event", "id", id).await,
            [None, None, s(crate::BUILD_STAMP)],
            "本表の版が今の S-01 の版に替わっていない"
        );
        assert_eq!(
            versions_of(&pool, "core.event_version", "event_id", id).await,
            [None, None, Some(old)],
            "前の版が履歴に残っていない"
        );
        assert_eq!(history_rows(&pool, id).await, 1);
    }

    #[tokio::test]
    // Scenario: 作り直しで原文が変わらなかった滞在の版は変わらない
    async fn st49_stay_rebuild_unchanged_keeps_version() {
        let pool = testdb::pool().await;
        let user = testdb::user();
        put_dwell(&pool, user, "2026-09-08T10:00:00+09:00", 20).await;
        crate::stay_store::rebuild_all(&pool, user).await.unwrap();
        let id = the_stay(&pool, user).await;
        let old = pretend_written_by_old_server(&pool, id).await;
        let keeps = [None, None, Some(old)];

        crate::stay_store::rebuild_all(&pool, user).await.unwrap();
        assert_eq!(versions_of(&pool, "core.event", "id", id).await, keeps);
        assert_eq!(history_rows(&pool, id).await, 0);

        // 削除の印だけが違う（原文は同じ）更新でも版は変わらず、履歴も増えない
        sqlx::query(
            "UPDATE core.event SET deleted_at = now(), deleted_by = 'rebuild:erased-range'
              WHERE id = $1",
        )
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
        crate::stay_store::rebuild_all(&pool, user).await.unwrap();
        let live: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT deleted_at FROM core.event WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            live.is_none(),
            "削除の印だけの更新が走っていない（前提が崩れた）"
        );
        assert_eq!(versions_of(&pool, "core.event", "id", id).await, keeps);
        assert_eq!(history_rows(&pool, id).await, 0);
    }

    #[tokio::test]
    // Scenario: 滞在の基準を変えた台帳の行は、書いた S-01 の版を持つ
    async fn st49_stay_criteria_row_has_server_version() {
        let password = format!("pw-{}", Uuid::new_v4());
        let app = crate::App::for_test(testdb::pool().await, "test-token-0123456789abcdef")
            .with_web_login(&password, 1);
        let login = crate::router(app.clone())
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/session")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "password": password }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login.status(), StatusCode::NO_CONTENT);
        let cookie = login.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let user = testdb::user();
        let res = crate::router(app.clone())
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/stays/rebuild")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("cookie", cookie)
                    .body(Body::from(
                        serde_json::json!({ "user_id": user, "radius_m": 77 }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["criteria_id"]
            .as_i64()
            .unwrap();
        let got: Option<String> =
            sqlx::query_scalar("SELECT server_version FROM core.stay_criteria WHERE id = $1")
                .bind(id)
                .fetch_one(&app.pool)
                .await
                .unwrap();
        assert_eq!(got.as_deref(), Some(crate::BUILD_STAMP));
    }
}

// ------------------------------------------------------------------ 破棄の報告・原本の欠け・画面の操作の台帳（Task 6）

mod ledgers {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    const TOKEN: &str = "test-token-0123456789abcdef";
    const COMMIT_A: &str = "0123456789abcdef0123456789abcdef01234567";
    const COMMIT_D: &str = "fedcba9876543210fedcba9876543210fedcba98";

    async fn app() -> crate::App {
        crate::App::for_test(testdb::pool().await, TOKEN)
    }

    async fn call(
        app: &crate::App,
        method: Method,
        uri: &str,
        body: &serde_json::Value,
        credential: (&str, String),
    ) -> (StatusCode, serde_json::Value) {
        let res = crate::router(app.clone())
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(credential.0, credential.1)
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        )
    }

    fn api_token() -> (&'static str, String) {
        ("authorization", format!("Bearer {TOKEN}"))
    }

    /// 画面のログイン（`/session`）の cookie。
    async fn web_login(app: &crate::App, password: &str) -> (&'static str, String) {
        let res = crate::router(app.clone())
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/session")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "password": password }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        let set_cookie = res.headers()[header::SET_COOKIE].to_str().unwrap();
        ("cookie", set_cookie.split(';').next().unwrap().to_owned())
    }

    /// 画面のログインを持つ app（滞在の作り直しは何もしない）。
    async fn web_app() -> (crate::App, (&'static str, String)) {
        let password = format!("pw-{}", Uuid::new_v4());
        let app = crate::App {
            stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
            ..crate::App::for_test(testdb::pool().await, TOKEN)
        }
        .with_web_login(&password, 1);
        let cookie = web_login(&app, &password).await;
        (app, cookie)
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_owned())
    }

    /// 破棄の報告の本文。`raw` は同じ論理ソースで違えば別の報告になる。
    fn drop_body(source: &str, user: Uuid, raw: &str, version: Option<&str>) -> serde_json::Value {
        let mut v = serde_json::json!({
            "id": Uuid::new_v4(),
            "user_id": user,
            "logical_source": source,
            "device_id": "test-dev",
            "reason": "write_failed",
            "created_at": "2026-09-14T00:00:00Z",
            "count": 3,
            "raw": raw,
        });
        if let Some(version) = version {
            v["client_version"] = version.into();
        }
        v
    }

    fn missing_body(user: Uuid, sha: &str, version: Option<&str>) -> serde_json::Value {
        let mut v = serde_json::json!({
            "id": Uuid::new_v4(),
            "user_id": user,
            "logical_source": "c01-photo",
            "device_id": "phone-01",
            "sha256": sha,
            "reason": "vanished",
            "noticed_at": "2026-10-01T03:00:00Z",
        });
        if let Some(version) = version {
            v["client_version"] = version.into();
        }
        v
    }

    fn sha() -> String {
        use sha2::Digest as _;
        format!("{:x}", sha2::Sha256::digest(Uuid::new_v4().as_bytes()))
    }

    async fn drop_versions(pool: &PgPool, user: Uuid, source: &str) -> Vec<[Option<String>; 2]> {
        let rows: Vec<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT collector_version, server_version FROM core.drop_report
              WHERE user_id = $1 AND logical_source = $2 ORDER BY created_at, id",
        )
        .bind(user)
        .bind(source)
        .fetch_all(pool)
        .await
        .unwrap();
        rows.into_iter().map(|(c, v)| [c, v]).collect()
    }

    async fn missing_versions(pool: &PgPool, id: &serde_json::Value) -> [Option<String>; 2] {
        let id: Uuid = id.as_str().unwrap().parse().unwrap();
        let (c, v): (Option<String>, Option<String>) = sqlx::query_as(
            "SELECT collector_version, server_version FROM core.media_missing WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
        [c, v]
    }

    #[tokio::test]
    // Scenario: 破棄の報告は、作った収集アプリの版と書いた S-01 の版を持つ
    async fn st49_ledger_drop_report_versions() {
        let app = app().await;
        let user = Uuid::new_v4();
        let source = testdb::source(&app.pool, "st49drop", 21_600).await;
        let version = format!("0.1.0+{COMMIT_A}.debug");
        let (code, res) = call(
            &app,
            Method::POST,
            "/drops",
            &serde_json::json!([drop_body(&source, user, "r1", Some(&version))]),
            api_token(),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{res}");
        assert_eq!(
            drop_versions(&app.pool, user, &source).await,
            [[s(&version), s(crate::BUILD_STAMP)]]
        );
    }

    #[tokio::test]
    // Scenario: 原本の欠けは、作った収集アプリの版と書いた S-01 の版を持つ
    async fn st49_ledger_media_missing_versions() {
        let app = app().await;
        let version = format!("0.1.0+{COMMIT_A}");
        let (code, res) = call(
            &app,
            Method::POST,
            "/media/missing",
            &serde_json::json!([missing_body(Uuid::new_v4(), &sha(), Some(&version))]),
            api_token(),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{res}");
        assert_eq!(
            missing_versions(&app.pool, &res[0]["id"]).await,
            [s(&version), s(crate::BUILD_STAMP)]
        );
    }

    #[tokio::test]
    // Scenario: 部品の版だけが違う同じ破棄の報告は、1 件のまま
    async fn st49_ledger_drop_report_version_only_difference_folds() {
        let app = app().await;
        let user = Uuid::new_v4();
        let source = testdb::source(&app.pool, "st49fold", 21_600).await;
        let first = format!("0.1.0+{COMMIT_A}");
        let second = format!("0.1.0+{COMMIT_D}");
        let (_, one) = call(
            &app,
            Method::POST,
            "/drops",
            &serde_json::json!([drop_body(&source, user, "same", Some(&first))]),
            api_token(),
        )
        .await;
        let (_, two) = call(
            &app,
            Method::POST,
            "/drops",
            &serde_json::json!([drop_body(&source, user, "same", Some(&second))]),
            api_token(),
        )
        .await;
        assert_eq!(one[0]["duplicate"], false, "{one}");
        assert_eq!(two[0]["duplicate"], true, "{two}");
        assert_eq!(
            drop_versions(&app.pool, user, &source).await,
            [[s(&first), s(crate::BUILD_STAMP)]]
        );
    }

    #[tokio::test]
    // Scenario: 部品の版の欄を持たない破棄の報告と原本の欠けも受け付けられる
    async fn st49_ledger_reports_without_field_are_accepted() {
        let app = app().await;
        let user = Uuid::new_v4();
        let source = testdb::source(&app.pool, "st49none", 21_600).await;
        let (code, _) = call(
            &app,
            Method::POST,
            "/drops",
            &serde_json::json!([drop_body(&source, user, "r1", None)]),
            api_token(),
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(
            drop_versions(&app.pool, user, &source).await,
            [[None, s(crate::BUILD_STAMP)]]
        );
        let (code, res) = call(
            &app,
            Method::POST,
            "/media/missing",
            &serde_json::json!([missing_body(user, &sha(), None)]),
            api_token(),
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(
            missing_versions(&app.pool, &res[0]["id"]).await,
            [None, s(crate::BUILD_STAMP)]
        );
    }

    async fn ledger_versions(pool: &PgPool, event: Uuid, action: &str) -> Vec<Option<String>> {
        sqlx::query_scalar(
            "SELECT server_version FROM core.deletion_ledger WHERE event_id = $1 AND action = $2",
        )
        .bind(event)
        .bind(action)
        .fetch_all(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    // Scenario: 画面から消した操作の台帳の行は、書いた S-01 の版を持つ
    async fn st49_ledger_erase_row_has_server_version() {
        let (app, cookie) = web_app().await;
        let user = testdb::user();
        let stay = crate::deletion_tests::put_stay(
            &app.pool,
            user,
            "2026-09-12T10:00:00+09:00",
            "2026-09-12T11:00:00+09:00",
        )
        .await;
        let (code, res) = call(
            &app,
            Method::POST,
            "/stays/erase",
            &serde_json::json!({ "stay_id": stay }),
            cookie,
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{res}");
        assert_eq!(
            ledger_versions(&app.pool, stay, "erase").await,
            [s(crate::BUILD_STAMP)]
        );
    }

    #[tokio::test]
    // Scenario: 画面から戻した操作の台帳の行も、書いた S-01 の版を持つ
    async fn st49_ledger_restore_row_has_server_version() {
        let (app, cookie) = web_app().await;
        let user = testdb::user();
        let stay = crate::deletion_tests::put_stay(
            &app.pool,
            user,
            "2026-09-13T10:00:00+09:00",
            "2026-09-13T11:00:00+09:00",
        )
        .await;
        let (code, _) = call(
            &app,
            Method::POST,
            "/stays/erase",
            &serde_json::json!({ "stay_id": stay }),
            cookie.clone(),
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        let (code, res) = call(
            &app,
            Method::POST,
            "/stays/restore",
            &serde_json::json!({ "stay_ids": [stay] }),
            cookie,
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{res}");
        assert_eq!(
            ledger_versions(&app.pool, stay, "restore").await,
            [s(crate::BUILD_STAMP)]
        );
    }

    #[tokio::test]
    // Scenario: 通知の時刻を変えた台帳の行は、書いた S-01 の版を持つ
    async fn st49_ledger_feeling_reminder_has_server_version() {
        let (app, cookie) = web_app().await;
        let user = Uuid::new_v4();
        let (code, res) = call(
            &app,
            Method::POST,
            "/feelings/reminder",
            &serde_json::json!({ "user_id": user, "at": "21:30" }),
            cookie,
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{res}");
        let got: Vec<Option<String>> = sqlx::query_scalar(
            "SELECT server_version FROM core.feeling_reminder WHERE user_id = $1",
        )
        .bind(user)
        .fetch_all(&app.pool)
        .await
        .unwrap();
        assert_eq!(got, [s(crate::BUILD_STAMP)]);
    }

    #[tokio::test]
    // Scenario: 個人属性の種類を足した名前の台帳の行は、書いた S-01 の版を持つ
    async fn st49_ledger_kind_name_has_server_version() {
        let (app, cookie) = web_app().await;
        let user = Uuid::new_v4();
        let name = format!("st49-{}", Uuid::new_v4());
        let (code, res) = call(
            &app,
            Method::POST,
            "/attributes/kinds",
            &serde_json::json!({ "user_id": user, "name": name }),
            cookie,
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{res}");
        let got: Option<String> = sqlx::query_scalar(
            "SELECT server_version FROM core.attribute_kind_name WHERE user_id = $1 AND name = $2",
        )
        .bind(user)
        .bind(&name)
        .fetch_one(&app.pool)
        .await
        .unwrap();
        assert_eq!(got.as_deref(), Some(crate::BUILD_STAMP));
    }

    /// 機械の止め（D10）: `server_version` を持つ 8 表への INSERT が、列の並びに `server_version` を名指す。
    /// 列は NULL 可なので、書き忘れた INSERT は何も落ちない —— 文字列で見る。
    #[test]
    fn st49_ledger_inserts_name_server_version() {
        const TABLES: [&str; 8] = [
            "event",
            "event_version",
            "drop_report",
            "media_missing",
            "deletion_ledger",
            "feeling_reminder",
            "stay_criteria",
            "attribute_kind_name",
        ];
        const MARK: &str = "INSERT INTO core.";
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut checked = 0;
        let mut missing = Vec::new();
        // src/ の下を再帰で辿る（archive/・export/・coverage/ などの下位も見る）。tests/ は試験なので除く
        let mut files = Vec::new();
        let mut dirs = vec![dir];
        while let Some(d) = dirs.pop() {
            for entry in std::fs::read_dir(&d).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    if path.file_name().unwrap() != "tests" {
                        dirs.push(path);
                    }
                } else {
                    files.push(path);
                }
            }
        }
        for path in files {
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            if !name.ends_with(".rs") || name.ends_with("_tests.rs") || name == "testdb.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            // `#[cfg(test)]` の中身の mod（`mod x {` から行頭の `}` まで）は試験なので除く
            let mut test_mods: Vec<(usize, usize)> = Vec::new();
            for (at, _) in text.match_indices("#[cfg(test)]\n") {
                let after = &text[at + "#[cfg(test)]\n".len()..];
                let line = after.lines().next().unwrap_or_default();
                if line.contains("mod ") && line.ends_with('{') {
                    let end = after.find("\n}\n").map_or(text.len(), |e| at + e);
                    test_mods.push((at, end));
                }
            }
            let body = text.as_str();
            for (at, _) in body.match_indices(MARK) {
                if test_mods.iter().any(|(a, b)| (*a..*b).contains(&at)) {
                    continue;
                }
                let rest = &body[at + MARK.len()..];
                let table: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !TABLES.contains(&table.as_str()) {
                    continue;
                }
                let columns = &rest[..rest.find(')').expect("列の並びの閉じ括弧")];
                checked += 1;
                if !columns.contains("server_version") {
                    missing.push(format!("{name}: INSERT INTO core.{table}"));
                }
            }
        }
        assert!(checked >= 14, "検査した INSERT が少なすぎる: {checked}");
        assert!(
            missing.is_empty(),
            "server_version を名指さない INSERT: {missing:?}"
        );
    }
}
