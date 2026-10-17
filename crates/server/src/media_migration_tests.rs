// SPDX-License-Identifier: AGPL-3.0-only
//! 写真と動画の移行 `media_capture`（ST09 / design D7 / D13 / D15 / D17）。
//!
//! 移行を当て直したり戻したりするテストは**使い捨ての DB**（`fresh_db`）でだけ走らせる。
//! 共有の DB に当て直すと、ALTER / DROP TRIGGER の強い錠が並んで走る他のテストと deadlock する。
#![allow(clippy::unwrap_used)]

use axum::{extract::State, http::HeaderMap, Json};
use sqlx::PgPool;

use crate::testdb;
use crate::{ingest, App, IngestResult};

const TOKEN: &str = "test-token-0123456789abcdef";

fn media_sql() -> &'static str {
    crate::MIGRATIONS
        .iter()
        .find(|(name, _)| name.ends_with("_media_capture"))
        .expect("ST09 の移行が MIGRATIONS に無い")
        .1
}

const MEDIA_DOWN: &str = include_str!("../../../migrations/202610181200_media_capture.down.sql");

/// 全版を当てた使い捨ての DB。
async fn migrated_db() -> (PgPool, impl std::future::Future<Output = ()>) {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    for (label, sql) in crate::MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(&fresh)
            .await
            .unwrap_or_else(|e| panic!("{label}: {e}"));
    }
    (fresh, drop)
}

async fn scalar_bool(pool: &PgPool, sql: &str) -> bool {
    sqlx::query_scalar::<_, bool>(sql)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// 全版を 2 回当てても壊れない。移行は ORDER の末尾にある。
#[tokio::test]
async fn st09_mig_applies_twice_and_is_last() {
    let (fresh, drop) = migrated_db().await;
    let again = sqlx::raw_sql(media_sql()).execute(&fresh).await;
    drop.await;
    again.unwrap();
    assert!(
        crate::MIGRATIONS
            .last()
            .is_some_and(|(name, _)| name.ends_with("_media_capture")),
        "移行が MIGRATIONS の末尾に無い"
    );
}

/// Scenario: 4 つのソースが登録簿に 6 時間の間隔で並ぶ
#[tokio::test]
async fn st09_mig_four_sources_in_registry_with_six_hours() {
    let pool = testdb::pool().await;
    let rows: Vec<(String, i32, String, String, bool)> = sqlx::query_as(
        "SELECT logical_source, expected_gap_sec, external_id_kind, started_on_basis, outage_target
           FROM core.source
          WHERE logical_source IN ('c01-photo','c01-screenshot','c01-image','c01-video')
          ORDER BY logical_source",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    assert_eq!(
        names,
        ["c01-image", "c01-photo", "c01-screenshot", "c01-video"]
    );
    for r in &rows {
        assert_eq!(r.1, 21_600, "{} の想定間隔が 6 時間でない", r.0);
        assert_eq!(r.2, "record", "{} が識別子を要求しない", r.0);
        assert_eq!(r.3, "heartbeat", "{} の収集開始日の印が違う", r.0);
    }
    let photo = rows.iter().find(|r| r.0 == "c01-photo").unwrap();
    assert!(
        rows.iter().all(|r| r.4 == photo.4),
        "足した 3 つの outage_target が c01-photo と違う"
    );
}

/// Scenario: メディアのソースは識別子を欠く記録を断る
#[tokio::test]
async fn st09_mig_media_sources_reject_records_without_external_id() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    let raw = r#"{"kind":"media","display_name":"a.jpg"}"#;
    let body = serde_json::json!([{
        "id": uuid::Uuid::new_v4(), "user_id": testdb::user(),
        "logical_source": "c01-photo", "external_id": null,
        "device_id": "phone-01", "origin": "collected",
        "event_time": "2026-03-01T12:00:00.000001Z", "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo", "schema_version": 1, "raw": raw,
        "payload": serde_json::from_str::<serde_json::Value>(raw).unwrap(),
    }]);
    let (_, Json(results)): (_, Json<Vec<IngestResult>>) =
        ingest(State(app), h, Json(body)).await.expect("取り込み口");
    assert!(!results[0].accepted, "識別子なしの写真を受け付けた");
    assert!(
        matches!(
            results[0].error,
            Some(crate::IngestError::MissingExternalId)
        ),
        "異なる理由で断られた: {:?}",
        results[0].error
    );
}

/// Scenario: 文字抽出の版を入れる列がある
#[tokio::test]
async fn st09_mig_text_extraction_has_engine_version_columns_and_no_rows() {
    let (fresh, drop) = migrated_db().await;
    let cols: Result<Vec<(String,)>, _> = sqlx::query_as(
        "SELECT column_name FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'text_extraction'",
    )
    .fetch_all(&fresh)
    .await;
    let rows: Result<i64, _> = sqlx::query_scalar("SELECT count(*) FROM core.text_extraction")
        .fetch_one(&fresh)
        .await;
    drop.await;
    let cols: Vec<String> = cols.unwrap().into_iter().map(|c| c.0).collect();
    for want in [
        "event_id",
        "text",
        "engine",
        "engine_version",
        "extracted_at",
        "user_id",
    ] {
        assert!(cols.iter().any(|c| c == want), "{want} 列が無い: {cols:?}");
    }
    assert_eq!(rows.unwrap(), 0, "表の行が 0 件でない");
}

/// 1 行入れ、`stmt` を実行して断られ、行が変わっていないことを見る。
/// 行は残さない（トランザクションごと巻き戻す。表は追記のみで、消せない）。
async fn assert_rejected_and_unchanged(setup: &[&str], stmt: &str, probe: &str, want: &str) {
    let pool = testdb::pool().await;
    let mut tx = pool.begin().await.unwrap();
    for s in setup {
        sqlx::query(s).execute(&mut *tx).await.unwrap();
    }
    sqlx::query("SAVEPOINT attempt")
        .execute(&mut *tx)
        .await
        .unwrap();
    let err = sqlx::query(stmt).execute(&mut *tx).await.unwrap_err();
    assert!(
        err.to_string().contains("追記のみ"),
        "門以外の理由で断られた: {err}"
    );
    sqlx::query("ROLLBACK TO attempt")
        .execute(&mut *tx)
        .await
        .unwrap();
    let got: String = sqlx::query_scalar(probe).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(got, want, "断られたのに行が変わっている");
    tx.rollback().await.unwrap();
}

/// Scenario: 文字抽出の行は書き換えられない
#[tokio::test]
async fn st09_mig_text_extraction_row_cannot_be_rewritten() {
    let ev = uuid::Uuid::new_v4();
    let setup = [
        format!(
            "INSERT INTO core.event (id,user_id,logical_source,origin,event_time,tz_offset_min,tz_id,
               schema_version,content_hash,raw,payload)
             VALUES ('{ev}','{ev}','c01-screenshot','collected',now(),0,'UTC',1,'{ev}','{{}}','{{}}')"
        ),
        format!(
            "INSERT INTO core.text_extraction (id,user_id,event_id,engine,engine_version,extracted_at,text)
             VALUES ('{ev}','{ev}','{ev}','ocr','1.0',now(),'こんにちは')"
        ),
    ];
    let setup: Vec<&str> = setup.iter().map(String::as_str).collect();
    let probe = format!("SELECT engine_version FROM core.text_extraction WHERE id = '{ev}'");
    assert_rejected_and_unchanged(
        &setup,
        &format!("UPDATE core.text_extraction SET engine_version = '2.0' WHERE id = '{ev}'"),
        &probe,
        "1.0",
    )
    .await;
    assert_rejected_and_unchanged(
        &setup,
        &format!("DELETE FROM core.text_extraction WHERE id = '{ev}'"),
        &probe,
        "1.0",
    )
    .await;
}

/// Scenario: 原本の欠けは書き換えられない
#[tokio::test]
async fn st09_mig_media_missing_row_cannot_be_rewritten() {
    let id = uuid::Uuid::new_v4();
    let setup = [format!(
        "INSERT INTO core.media_missing (id,user_id,device_id,logical_source,sha256,reason,noticed_at)
         VALUES ('{id}','{id}','phone-01','c01-photo','abc','vanished',now())"
    )];
    let setup: Vec<&str> = setup.iter().map(String::as_str).collect();
    let probe = format!("SELECT reason FROM core.media_missing WHERE id = '{id}'");
    assert_rejected_and_unchanged(
        &setup,
        &format!("UPDATE core.media_missing SET reason = 'changed' WHERE id = '{id}'"),
        &probe,
        "vanished",
    )
    .await;
    assert_rejected_and_unchanged(
        &setup,
        &format!("DELETE FROM core.media_missing WHERE id = '{id}'"),
        &probe,
        "vanished",
    )
    .await;
}

/// アプリの役割の接続でも、3 表は書き込み（INSERT）だけが通り、書き換え・削除は門が断る（D15 の 4）。
/// 権限は `grants.sql` が全表に同じ付与をするので、書き換え禁止はトリガが持つ。
#[tokio::test]
async fn st09_mig_app_role_can_only_append_to_media_tables() {
    let pool = testdb::app_pool().await;
    let id = uuid::Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.media_original (user_id, sha256, logical_source, size_bytes)
         VALUES ($1, 'abc', 'c01-image', 10)",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .unwrap();
    for stmt in [
        "UPDATE core.media_original SET size_bytes = 11",
        "DELETE FROM core.media_original",
    ] {
        sqlx::query("SAVEPOINT attempt")
            .execute(&mut *tx)
            .await
            .unwrap();
        let err = sqlx::query(stmt).execute(&mut *tx).await.unwrap_err();
        assert!(err.to_string().contains("追記のみ"), "{stmt}: {err}");
        sqlx::query("ROLLBACK TO attempt")
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.rollback().await.unwrap();
}

/// 生存信号の `network` は任意の欄で、既存の行は NULL のまま。門は列を足しても効く。
#[tokio::test]
async fn st09_mig_heartbeat_network_is_nullable_and_row_stays_locked() {
    let (fresh, drop) = migrated_db().await;
    let nullable = scalar_bool(
        &fresh,
        "SELECT is_nullable = 'YES' FROM information_schema.columns
          WHERE table_schema='core' AND table_name='heartbeat' AND column_name='network'",
    )
    .await;
    testdb::put_heartbeat(
        &fresh,
        testdb::user(),
        "c01-photo",
        "2026-10-20T03:00:00Z",
        true,
    )
    .await;
    let net: Option<bool> = sqlx::query_scalar("SELECT network FROM core.heartbeat")
        .fetch_one(&fresh)
        .await
        .unwrap();
    let rewrite = sqlx::query("UPDATE core.heartbeat SET network = true")
        .execute(&fresh)
        .await;
    drop.await;
    assert!(nullable, "network が NULL を許さない");
    assert_eq!(net, None);
    assert!(rewrite.is_err(), "生存信号の門が効いていない");
}

/// Scenario: 汚れた写真の収集開始日は移行で直る
#[tokio::test]
async fn st09_mig_polluted_photo_started_on_is_repaired() {
    let (fresh, drop) = migrated_db().await;
    let u = testdb::user();
    sqlx::query(
        "UPDATE core.source SET registered_at = '2026-10-01T00:00:00+09' WHERE logical_source = 'c01-photo'",
    )
    .execute(&fresh)
    .await
    .unwrap();
    testdb::put_event(&fresh, u, "c01-photo", "2021-10-20T12:00:00+09:00").await;
    testdb::put_heartbeat(&fresh, u, "c01-photo", "2026-10-20T03:00:00Z", true).await;
    testdb::set_started_on(&fresh, "c01-photo", "2021-10-20").await;
    // 他のソース（印が record）は触らない
    testdb::set_started_on(&fresh, "c02-window", "2021-10-20").await;
    let before: (i64, String) = sqlx::query_as(
        "SELECT count(*), min(content_hash) FROM core.event WHERE logical_source = 'c01-photo'",
    )
    .fetch_one(&fresh)
    .await
    .unwrap();

    sqlx::raw_sql(media_sql()).execute(&fresh).await.unwrap();

    let started = |s: &'static str| {
        let fresh = fresh.clone();
        async move {
            sqlx::query_scalar::<_, Option<chrono::NaiveDate>>(
                "SELECT collection_started_on FROM core.source WHERE logical_source = $1",
            )
            .bind(s)
            .fetch_one(&fresh)
            .await
            .unwrap()
        }
    };
    let photo = started("c01-photo").await;
    let video = started("c01-video").await;
    let window = started("c02-window").await;
    let after: (i64, String) = sqlx::query_as(
        "SELECT count(*), min(content_hash) FROM core.event WHERE logical_source = 'c01-photo'",
    )
    .fetch_one(&fresh)
    .await
    .unwrap();
    drop.await;

    assert_eq!(photo, Some(testdb::date("2026-10-20")));
    assert_eq!(video, None, "生存信号の無いソースは NULL");
    assert_eq!(
        window,
        Some(testdb::date("2021-10-20")),
        "印が record のソースを動かした"
    );
    assert_eq!(before, after, "写真の記録が変わっている");
}

/// 印は列を足した回だけ当てる。本人が `record` へ戻したものを、当て直しで撃ち戻さない（D13 の反転）。
#[tokio::test]
async fn st09_mig_basis_reset_to_record_survives_reapply() {
    let (fresh, drop) = migrated_db().await;
    sqlx::query(
        "UPDATE core.source SET started_on_basis = 'record' WHERE logical_source = 'c01-image'",
    )
    .execute(&fresh)
    .await
    .unwrap();
    sqlx::raw_sql(media_sql()).execute(&fresh).await.unwrap();
    let basis: String = sqlx::query_scalar(
        "SELECT started_on_basis FROM core.source WHERE logical_source = 'c01-image'",
    )
    .fetch_one(&fresh)
    .await
    .unwrap();
    drop.await;
    assert_eq!(basis, "record");
}

/// 記録のあるソースの識別子の宣言は、当て直しで触らない（ST08 D7 と同じ）。0 件なら record にする。
#[tokio::test]
async fn st09_mig_external_id_kind_is_set_only_when_no_records() {
    let (fresh, drop) = migrated_db().await;
    sqlx::query(
        "UPDATE core.source SET external_id_kind = 'none'
          WHERE logical_source IN ('c01-photo','c01-image')",
    )
    .execute(&fresh)
    .await
    .unwrap();
    testdb::put_event(&fresh, testdb::user(), "c01-photo", "2026-03-01T00:00:00Z").await;
    sqlx::raw_sql(media_sql()).execute(&fresh).await.unwrap();
    let kinds: Vec<(String, String)> = sqlx::query_as(
        "SELECT logical_source, external_id_kind FROM core.source
          WHERE logical_source IN ('c01-photo','c01-image') ORDER BY 1",
    )
    .fetch_all(&fresh)
    .await
    .unwrap();
    drop.await;
    assert_eq!(
        kinds,
        [
            ("c01-image".to_string(), "record".to_string()),
            ("c01-photo".to_string(), "none".to_string())
        ]
    );
}

/// 戻し手順: 行が無ければ表・欄・足したソースを落とし、進め直せる。行があれば表を落とさず残す。
#[tokio::test]
async fn st09_mig_down_drops_when_empty_and_keeps_tables_with_rows() {
    let (fresh, drop) = migrated_db().await;
    sqlx::raw_sql(MEDIA_DOWN).execute(&fresh).await.unwrap();
    let gone = scalar_bool(
        &fresh,
        "SELECT to_regclass('core.media_original') IS NULL
            AND to_regclass('core.media_missing') IS NULL
            AND to_regclass('core.text_extraction') IS NULL
            AND to_regprocedure('core.reject_media_change()') IS NULL
            AND NOT EXISTS (SELECT 1 FROM information_schema.columns
                             WHERE table_schema='core' AND table_name='source' AND column_name='started_on_basis')
            AND NOT EXISTS (SELECT 1 FROM information_schema.columns
                             WHERE table_schema='core' AND table_name='heartbeat' AND column_name='network')
            AND NOT EXISTS (SELECT 1 FROM core.source
                             WHERE logical_source IN ('c01-screenshot','c01-image','c01-video'))",
    )
    .await;
    // 2 回目の戻し・進め直し
    let twice = sqlx::raw_sql(MEDIA_DOWN).execute(&fresh).await;
    let reapplied = sqlx::raw_sql(media_sql()).execute(&fresh).await;

    // 行があれば残す
    sqlx::query(
        "INSERT INTO core.media_original (user_id, sha256, logical_source, size_bytes)
         VALUES ('00000000-0000-0000-0000-000000000000', 'abc', 'c01-image', 10)",
    )
    .execute(&fresh)
    .await
    .unwrap();
    let kept_run = sqlx::raw_sql(MEDIA_DOWN).execute(&fresh).await;
    let kept = scalar_bool(
        &fresh,
        "SELECT to_regclass('core.media_original') IS NOT NULL
            AND to_regclass('core.text_extraction') IS NOT NULL
            AND EXISTS (SELECT 1 FROM core.media_original)
            AND EXISTS (SELECT 1 FROM core.source WHERE logical_source = 'c01-image')",
    )
    .await;
    drop.await;

    assert!(gone, "行が無いのに表・欄・ソースが残っている");
    twice.unwrap();
    reapplied.unwrap();
    kept_run.unwrap();
    assert!(kept, "行があるのに表かソースを落とした");
}
