// SPDX-License-Identifier: AGPL-3.0-only
//! 端末の時計のずれの測定記録（論理ソース `c01-clock`。ST05 / design D1）を、
//! **本物の PostgreSQL に対して**確かめる。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::coverage::{achievement, must_sources};
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

fn auth() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

/// 測定記録 1 件（外部識別子なし）。
fn clock_record(user: uuid::Uuid, event_time: &str) -> serde_json::Value {
    serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "user_id": user,
        "logical_source": "c01-clock",
        "external_id": null,
        "device_id": "test-dev",
        "origin": "collected",
        "event_time": event_time,
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": r#"{"skew_ms":120}"#,
        "payload": {},
    })
}

/// 使い捨ての DB を作る。**戻り値の 2 つ目を必ず待つ**（接続を閉じて DB を消す）。
async fn fresh_db() -> (sqlx::PgPool, impl std::future::Future<Output = ()>) {
    let admin = testdb::pool().await;
    let name = format!("st05_tmp_{}", uuid::Uuid::new_v4().simple());
    sqlx::raw_sql(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    let base = crate::testdb::url();
    let url = format!("{}/{name}", &base[..base.rfind('/').unwrap()]);
    let fresh = sqlx::PgPool::connect(&url).await.unwrap();
    let closer = fresh.clone();
    (fresh, async move {
        closer.close().await;
        sqlx::raw_sql(&format!("DROP DATABASE {name} WITH (FORCE)"))
            .execute(&admin)
            .await
            .unwrap();
    })
}

/// 全版を 2 回当てて落ちず、行が 1 行で識別子の種類が `none`（tasks 2.1）。
/// 共有の開発 DB で当て直すと他のテストの錠と競合するので、使い捨ての DB に当てる。
#[tokio::test]
async fn clock_source_migration_applies_twice() {
    let (fresh, drop) = fresh_db().await;
    let applied = async {
        for round in 1..=2 {
            for (label, sql) in crate::MIGRATIONS {
                sqlx::raw_sql(sql)
                    .execute(&fresh)
                    .await
                    .map_err(|e| format!("{round} 回目の {label}: {e}"))?;
            }
        }
        sqlx::query_as::<_, (i64, String)>(
            "SELECT count(*), min(external_id_kind) FROM core.source WHERE logical_source = 'c01-clock'",
        )
        .fetch_one(&fresh)
        .await
        .map_err(|e| e.to_string())
    }
    .await;
    drop.await;

    let (n, kind) = applied.unwrap();
    assert_eq!(n, 1, "2 回当てると c01-clock が二重になる");
    assert_eq!(kind, "none");
    assert!(
        crate::MIGRATIONS
            .iter()
            .any(|(n, _)| n.ends_with("_clock_source")),
        "移行が MIGRATIONS に無い"
    );
}

/// 戻しの移行は、**破棄の報告が c01-clock を参照していても**外部キーで落ちず、登録簿の行を残す。
/// 参照が無ければ登録簿の行を消す（review R8）。使い捨ての DB に当てる。
#[tokio::test]
async fn clock_source_down_keeps_row_referenced_by_drop_report() {
    const UP: &str = include_str!("../../../migrations/202609291230_clock_source.sql");
    const DOWN: &str = include_str!("../../../migrations/202609291230_clock_source.down.sql");
    let (fresh, drop) = fresh_db().await;
    let result = async {
        for (label, sql) in crate::MIGRATIONS {
            sqlx::raw_sql(sql)
                .execute(&fresh)
                .await
                .map_err(|e| format!("{label}: {e}"))?;
        }
        let count = |pool: sqlx::PgPool| async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM core.source WHERE logical_source = 'c01-clock'",
            )
            .fetch_one(&pool)
            .await
            .map_err(|e| e.to_string())
        };
        // 参照が無ければ消す（破棄の報告は削除できないので、こちらを先に見る）
        sqlx::raw_sql(DOWN)
            .execute(&fresh)
            .await
            .map_err(|e| format!("参照が無いのに戻しが落ちる: {e}"))?;
        let removed = count(fresh.clone()).await?;

        sqlx::raw_sql(UP)
            .execute(&fresh)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::query(
            "INSERT INTO core.drop_report \
               (id, user_id, logical_source, device_id, reason, count, created_at, content_hash, raw) \
             VALUES ($1, $2, 'c01-clock', 'test-dev', 'age', 1, now(), 'h', '{}')",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(uuid::Uuid::new_v4())
        .execute(&fresh)
        .await
        .map_err(|e| e.to_string())?;
        sqlx::raw_sql(DOWN)
            .execute(&fresh)
            .await
            .map_err(|e| format!("破棄の報告があると戻しが落ちる: {e}"))?;
        let kept = count(fresh.clone()).await?;
        Ok::<_, String>((kept, removed))
    }
    .await;
    drop.await;

    let (kept, removed) = result.unwrap();
    assert_eq!(kept, 1, "破棄の報告が参照しているのに登録簿の行を消した");
    assert_eq!(removed, 0, "参照が無いのに登録簿の行が残った");
}

/// Scenario: 識別子を持たない測定記録が格納される
#[tokio::test]
async fn clock_record_ingest_stores_record_without_external_id() {
    let app = App::for_test(testdb::pool().await, TOKEN);
    let u = testdb::user();
    let (code, res) = {
        let (code, Json(res)) = ingest(
            State(app.clone()),
            auth(),
            Json(serde_json::json!([clock_record(u, "2026-05-01T01:00:00Z")])),
        )
        .await
        .expect("取り込み口");
        (code, res)
    };
    assert_eq!(code, StatusCode::OK, "{res:?}");
    assert_eq!(res.len(), 1);
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = 'c01-clock'",
    )
    .bind(u)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(n, 1, "断られたか格納されていない: {res:?}");
}

/// Scenario: 測定記録だけの日は端末が主語の達成日にならない
///
/// 位置の記録の日は数えられる（対照）ので、「数えが常に 0」で通ることはない。
#[tokio::test]
async fn clock_record_ingest_day_is_not_device_achieved_day() {
    let (fresh, drop) = fresh_db().await;
    let result = async {
        for (label, sql) in crate::MIGRATIONS {
            sqlx::raw_sql(sql)
                .execute(&fresh)
                .await
                .map_err(|e| format!("{label}: {e}"))?;
        }
        Ok::<(), String>(())
    }
    .await;
    if let Err(e) = result {
        drop.await;
        panic!("{e}");
    }
    let app = App::for_test(fresh.clone(), TOKEN);
    let u = testdb::user();
    testdb::set_started_on(&fresh, "c01-location", "2026-05-01").await;
    // 05-01・05-02 は測定記録だけ。05-03 には位置の記録もある
    let (_, Json(res)) = ingest(
        State(app.clone()),
        auth(),
        Json(serde_json::json!([
            clock_record(u, "2026-05-01T01:00:00Z"),
            clock_record(u, "2026-05-02T01:00:00Z"),
            clock_record(u, "2026-05-03T01:00:00Z"),
        ])),
    )
    .await
    .expect("取り込み口");
    testdb::put_event(&fresh, u, "c01-location", "2026-05-03T02:00:00Z").await;

    let stored: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = 'c01-clock'",
    )
    .bind(u)
    .fetch_one(&fresh)
    .await
    .unwrap();
    let got = achievement(&fresh, Some(u), testdb::date("2026-05-04"), &must_sources())
        .await
        .unwrap();
    drop.await;

    assert_eq!(stored.0, 3, "測定記録が格納されていない: {res:?}");

    let loc = got
        .sources
        .iter()
        .find(|s| s.logical_source == "c01-location")
        .unwrap();
    assert_eq!(res.len(), 3);
    assert_eq!(loc.denominator, 3, "05-01〜05-03");
    assert_eq!(loc.achieved_days, 1, "位置の記録がある 05-03 だけ");
}
