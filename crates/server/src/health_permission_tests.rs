// SPDX-License-Identifier: AGPL-3.0-only
//! 健康データの権限の記録（論理ソース `c01-health-permission`。ST11 / design D1・D12）を、
//! **本物の PostgreSQL に対して**確かめる。登録簿の行・取り込み・途切れの判定・稼働状況の格子・書き出し。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::export_excluded_tests::World;
use crate::export_file_tests::{export_once, present};
use crate::outage::judge;
use crate::outage_store::inputs;
use crate::testdb;
use chrono::Utc;

const TOKEN: &str = "test-token-0123456789abcdef";
const SOURCE: &str = "c01-health-permission";
const DOWN: &str =
    include_str!("../../../migrations/202610251100_health_permission_source.down.sql");

fn auth() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

/// 権限の記録 1 件（外部識別子なし）。`payload` は design D2 の形。
fn permission_record(user: uuid::Uuid, event_time: &str) -> serde_json::Value {
    let payload = serde_json::json!({
        "kind": "health-permission",
        "trigger": "request_result",
        "device_time": event_time,
        "elapsed_ms": 123456789,
        "boot_count": 42,
        "availability": "available",
        "features": {"history": true, "background": true},
        "granted": {"sleep": true, "heart_rate": true, "steps": true,
                    "exercise": true, "history": true, "background": false},
        "auto_requested": true,
    });
    serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "user_id": user,
        "logical_source": SOURCE,
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": event_time,
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": payload.to_string(),
        "payload": payload,
    })
}

async fn ingest_one(app: &App, record: serde_json::Value) -> (StatusCode, Vec<serde_json::Value>) {
    let (code, Json(res)) = ingest(
        State(app.clone()),
        auth(),
        Json(serde_json::json!([record])),
    )
    .await
    .expect("取り込み口");
    (
        code,
        res.into_iter().map(|r| serde_json::json!(r)).collect(),
    )
}

/// 全版を当てた使い捨ての DB で `f` を走らせる。
async fn with_migrated_db<F, Fut, T>(f: F) -> T
where
    F: FnOnce(sqlx::PgPool) -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    let result = async {
        // 本番と同じく版の記録を持たせて当てる（戻しが版の記録の行を消すので、記録の表が要る）
        let conn = fresh.acquire().await.map_err(|e| e.to_string())?;
        crate::migration_ledger::apply_migrations(conn, crate::MIGRATIONS)
            .await
            .map_err(|e| format!("移行が当たらない: {e:?}"))?;
        f(fresh.clone()).await
    }
    .await;
    drop.await;
    result.unwrap()
}

async fn ledger_count(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied WHERE name LIKE '%_health_permission_source'")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn row_count(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM core.source WHERE logical_source = $1")
        .bind(SOURCE)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// 2 回当てても行が 1 つで、登録簿の値が design D1 の表どおり（ST13 の trigger が `acquisition` / `fetch_trigger` を入れる）。
#[tokio::test]
async fn st11_registry_migration_applies_twice_with_the_designed_row() {
    let (n, row) = with_migrated_db(|pool| async move {
        for (label, sql) in crate::MIGRATIONS {
            sqlx::raw_sql(sql)
                .execute(&pool)
                .await
                .map_err(|e| format!("2 回目の {label}: {e}"))?;
        }
        let row: (String, i32, String, bool, String, String, Option<i32>) = sqlx::query_as(
            "SELECT display_name, expected_gap_sec, external_id_kind, outage_target,
                    acquisition, fetch_trigger, fetch_interval_sec
               FROM core.source WHERE logical_source = $1",
        )
        .bind(SOURCE)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok((row_count(&pool).await, row))
    })
    .await;
    assert_eq!(n, 1, "2 回当てると二重になる");
    assert_eq!(row.0, "携帯端末の健康データの権限");
    assert_eq!(row.1, 21600);
    assert_eq!(row.2, "none", "external_id_kind");
    assert!(!row.3, "outage_target");
    assert_eq!(row.4, "device", "acquisition");
    assert_eq!(row.5, "on_change", "fetch_trigger");
    assert_eq!(row.6, None, "fetch_interval_sec");
    assert!(
        crate::MIGRATIONS
            .iter()
            .any(|(n, _)| n.ends_with("_health_permission_source")),
        "移行が MIGRATIONS に無い"
    );
}

/// 戻しの移行は、参照が無ければ登録簿の行を消す。
#[tokio::test]
async fn st11_registry_down_removes_row_when_unreferenced() {
    let removed = with_migrated_db(|pool| async move {
        sqlx::raw_sql(DOWN)
            .execute(&pool)
            .await
            .map_err(|e| format!("参照が無いのに戻しが落ちる: {e}"))?;
        Ok((row_count(&pool).await, ledger_count(&pool).await))
    })
    .await;
    assert_eq!(removed.0, 0, "参照が無いのに登録簿の行が残った");
    assert_eq!(removed.1, 0, "戻しが版の記録の行を消さない");
}

/// 戻しの移行は、破棄の報告が参照していても外部キーで落ちず、登録簿の行を残す。
#[tokio::test]
async fn st11_registry_down_keeps_row_while_referenced() {
    let kept = with_migrated_db(|pool| async move {
        sqlx::query(
            "INSERT INTO core.drop_report \
               (id, user_id, logical_source, device_id, reason, count, created_at, content_hash, raw) \
             VALUES ($1, $2, $3, 'test-dev', 'age', 1, now(), 'h', '{}')",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(uuid::Uuid::new_v4())
        .bind(SOURCE)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
        sqlx::raw_sql(DOWN)
            .execute(&pool)
            .await
            .map_err(|e| format!("破棄の報告があると戻しが落ちる: {e}"))?;
        Ok((row_count(&pool).await, ledger_count(&pool).await))
    })
    .await;
    assert_eq!(kept.0, 1, "破棄の報告が参照しているのに登録簿の行を消した");
    assert_eq!(kept.1, 0, "登録簿の行を残しても、版の記録の行は消す");
}

// Scenario: 識別子を持たない健康データの権限の記録が格納される
#[tokio::test]
async fn st11_ingest_stores_record_without_external_id() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let u = testdb::user();
    let (code, res) = ingest_one(&app, permission_record(u, "2026-05-01T01:00:00Z")).await;
    assert_eq!(code, StatusCode::OK, "{res:?}");
    assert_eq!(res.len(), 1);
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = $2",
    )
    .bind(u)
    .bind(SOURCE)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(n, 1, "断られたか格納されていない: {res:?}");
}

// Scenario: 健康データの権限の記録が何日も届かなくても途切れの知らせは出ない
#[tokio::test]
async fn st11_no_outage_when_no_record_arrives_for_30_days() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let u = testdb::user();
    let now = Utc::now();
    let (code, res) = ingest_one(&app, permission_record(u, &now.to_rfc3339())).await;
    assert_eq!(code, StatusCode::OK, "{res:?}");

    let later = now + chrono::Duration::days(30);
    let found = inputs(&app.pool, u, later).await.unwrap();
    let mine = found
        .iter()
        .find(|s| s.logical_source == SOURCE)
        .expect("inputs に無い");
    assert!(!mine.target, "登録簿の印が判定の対象のまま");
    let outages = judge(&found, later);
    assert!(
        outages.iter().all(|o| o.logical_source != SOURCE),
        "途切れの知らせが出た: {outages:?}"
    );

    // 対照: 対象にして収集開始日を置けば、同じ材料で 30 日後は途切れになる（判定が常に空で通っていない）
    let mut as_target = mine.clone();
    as_target.target = true;
    as_target.collection_started_on = Some(testdb::date("2026-01-01"));
    assert!(
        judge(&[as_target], later)
            .iter()
            .any(|o| o.logical_source == SOURCE),
        "対照が途切れにならない"
    );
}

// Scenario: 稼働状況の格子に健康データの権限の行は出ない
#[tokio::test]
async fn st11_coverage_grid_has_no_health_permission_row() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let u = testdb::user();
    let (code, res) = ingest_one(&app, permission_record(u, "2026-05-01T01:00:00Z")).await;
    assert_eq!(code, StatusCode::OK, "{res:?}");

    let Json(grid) = coverage_get(
        State(app.clone()),
        auth(),
        Query(CoverageQuery {
            from: testdb::date("2026-05-01"),
            to: testdb::date("2026-05-03"),
            user_id: Some(u),
        }),
    )
    .await
    .expect("稼働状況");
    assert!(!grid.is_empty(), "格子が空（対照にならない）");
    let names: Vec<&str> = grid.iter().map(|g| g.logical_source.as_str()).collect();
    assert!(!names.contains(&SOURCE), "格子に出た: {names:?}");
    assert!(
        !coverage::must_sources().iter().any(|(n, _)| n == SOURCE),
        "達成の対象に入った"
    );
}

// Scenario: 書き出しに健康データの権限の記録が入る
#[tokio::test]
async fn st11_export_includes_health_permission_record() {
    let w = World::new().await;
    let payload = permission_record(w.user, &w.at("01:00"))["payload"].clone();
    let id = w
        .put(
            SOURCE,
            &w.at("01:00"),
            payload.clone(),
            &payload.to_string(),
        )
        .await;

    let path = export_once(&w.pool).await;
    let got = present(&path, "events.jsonl", &[id]);
    assert!(got.contains(&id), "書き出しに入っていない");
}
