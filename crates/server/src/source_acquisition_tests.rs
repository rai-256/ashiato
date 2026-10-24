// SPDX-License-Identifier: AGPL-3.0-only
//! 登録簿の取り方・契機・取得の間隔（ST13 / design D1 / D2 / D3）。
//! 共有の登録簿の行は書き換えない: 変える試験は transaction の中で行い、最後に戻す。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use sqlx::Row;

const MIGRATION: &str = "_source_acquisition";

fn this_migration() -> (usize, &'static str) {
    let i = crate::MIGRATIONS
        .iter()
        .position(|(n, _)| n.ends_with(MIGRATION))
        .unwrap();
    (i, crate::MIGRATIONS[i].1)
}

/// 使い捨ての DB に、この移行の前までの版を当てる。移行を当て直す試験は DDL で表に排他の錠を取るので、
/// 共有の DB でなくここで行う（共有の DB で行うと、同時に走る書き出しの試験と錠を取り合って落ちる）。
async fn fresh_before_this_migration() -> (sqlx::PgPool, impl std::future::Future<Output = ()>) {
    let (pool, drop) = crate::stay_tests::fresh_db().await;
    for (label, sql) in &crate::MIGRATIONS[..this_migration().0] {
        sqlx::raw_sql(sql)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("{label}: {e}"));
    }
    (pool, drop)
}

async fn read(pool: &sqlx::PgPool, source: &str) -> (String, String, Option<i32>) {
    let r = sqlx::query(
        "SELECT acquisition, fetch_trigger, fetch_interval_sec FROM core.source WHERE logical_source = $1",
    )
    .bind(source)
    .fetch_one(pool)
    .await
    .unwrap();
    (r.get(0), r.get(1), r.get(2))
}

/// 断られた変更が DB の制約違反であること（名前つき）と、値が元のままであることを見る。
async fn assert_refused(sql: &str, source: &str, constraint: &str) {
    let pool = testdb::pool().await;
    let before = read(&pool, source).await;
    let mut tx = pool.begin().await.unwrap();
    let err = sqlx::query(sql).execute(&mut *tx).await.unwrap_err();
    let db = err
        .as_database_error()
        .unwrap_or_else(|| panic!("制約違反でない: {err}"));
    assert_eq!(
        db.code().as_deref(),
        Some("23514"),
        "check_violation でない: {err}"
    );
    assert_eq!(db.constraint(), Some(constraint), "{err}");
    drop(tx);
    assert_eq!(read(&pool, source).await, before, "値が元のままである");
}

// Scenario: 移行を当てると各ソースが取り方と契機を持つ
#[tokio::test]
async fn migration_gives_each_source_acquisition_and_trigger() {
    let pool = testdb::pool().await;
    assert_eq!(
        read(&pool, "c01-location").await,
        ("device".into(), "interval".into(), Some(60))
    );
    assert_eq!(
        read(&pool, "c03-youtube-watch").await,
        ("archive".into(), "manual".into(), None)
    );
    assert_eq!(
        read(&pool, "s01-archive-inbox").await,
        ("server".into(), "interval".into(), Some(120))
    );
}

#[tokio::test]
async fn migration_values_follow_the_table_of_d3() {
    let pool = testdb::pool().await;
    let expect: [(&str, &str, &str, Option<i32>); 12] = [
        ("c01-location", "device", "interval", Some(60)),
        ("c01-app-usage", "device", "interval", Some(1800)),
        ("c01-app-usage-rollup", "device", "once", None),
        ("c01-photo", "device", "on_change", None),
        ("c01-clock", "device", "interval", Some(3600)),
        ("c02-window", "pc", "on_change", None),
        ("c02-browser-history", "pc", "interval", Some(86400)),
        ("s01-archive-inbox", "server", "interval", Some(120)),
        ("s01-stay", "server", "derived", None),
        ("s01-attribute", "server", "authored", None),
        ("s01-feeling", "server", "authored", None),
        ("s01-place", "server", "authored", None),
    ];
    for (s, a, t, i) in expect {
        assert_eq!(read(&pool, s).await, (a.into(), t.into(), i), "{s}");
    }
    let c03: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.source WHERE logical_source LIKE 'c03-%'
           AND (acquisition <> 'archive' OR fetch_trigger <> 'manual' OR fetch_interval_sec IS NOT NULL)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(c03, 0);
}

// Scenario: 取り方の無いソースは登録簿に残らない
#[tokio::test]
async fn no_source_lacks_acquisition_or_trigger() {
    let pool = testdb::pool().await;
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.source WHERE acquisition IS NULL OR fetch_trigger IS NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 0, "0 行");
    let nullable: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'source'
            AND column_name IN ('acquisition', 'fetch_trigger') AND is_nullable = 'YES'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(nullable, 0, "2 列とも NOT NULL");
}

// Scenario: 想定間隔より長い取得の間隔は断られる
#[tokio::test]
async fn interval_longer_than_expected_gap_is_refused() {
    let pool = testdb::pool().await;
    let gap: i32 = sqlx::query_scalar(
        "SELECT expected_gap_sec FROM core.source WHERE logical_source = 'c01-location'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(gap, 21600);
    assert_refused(
        "UPDATE core.source SET fetch_interval_sec = 21601 WHERE logical_source = 'c01-location'",
        "c01-location",
        "source_fetch_interval_within_gap_check",
    )
    .await;
}

// Scenario: 想定間隔と同じ取得の間隔は受け付けられる
#[tokio::test]
async fn interval_equal_to_expected_gap_is_accepted() {
    let pool = testdb::pool().await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "UPDATE core.source SET fetch_interval_sec = 21600 WHERE logical_source = 'c01-location'",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let v: Option<i32> = sqlx::query_scalar(
        "SELECT fetch_interval_sec FROM core.source WHERE logical_source = 'c01-location'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(v, Some(21600));
}

#[tokio::test]
async fn interval_of_zero_is_refused() {
    assert_refused(
        "UPDATE core.source SET fetch_interval_sec = 0 WHERE logical_source = 'c01-location'",
        "c01-location",
        "source_fetch_interval_within_gap_check",
    )
    .await;
}

#[tokio::test]
async fn archive_inbox_interval_over_300_is_refused() {
    assert_refused(
        "UPDATE core.source SET fetch_interval_sec = 301 WHERE logical_source = 's01-archive-inbox'",
        "s01-archive-inbox",
        "source_archive_inbox_interval_check",
    )
    .await;
}

#[tokio::test]
async fn unknown_acquisition_or_trigger_is_refused() {
    assert_refused(
        "UPDATE core.source SET acquisition = 'cloud' WHERE logical_source = 'c01-location'",
        "c01-location",
        "source_fetch_values_check",
    )
    .await;
    assert_refused(
        "UPDATE core.source SET fetch_trigger = 'sometimes' WHERE logical_source = 'c01-photo'",
        "c01-photo",
        "source_fetch_values_check",
    )
    .await;
}

// Scenario: 取り方を指定せずに足したソースは名前から取り方が入る
#[tokio::test]
async fn source_added_without_acquisition_gets_it_from_prefix() {
    let pool = testdb::pool().await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO core.source (logical_source, display_name, expected_gap_sec) VALUES ('c01-example', 'x', 21600)")
        .execute(&mut *tx)
        .await
        .unwrap();
    let r = sqlx::query("SELECT acquisition, fetch_trigger, fetch_interval_sec FROM core.source WHERE logical_source = 'c01-example'")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(r.get::<String, _>(0), "device");
    assert_eq!(r.get::<String, _>(1), "on_change");
    assert_eq!(r.get::<Option<i32>, _>(2), None);
}

#[tokio::test]
async fn prefix_defaults_cover_pc_archive_and_server() {
    let pool = testdb::pool().await;
    let mut tx = pool.begin().await.unwrap();
    for (name, a, t) in [
        ("c02-example", "pc", "on_change"),
        ("c03-example", "archive", "manual"),
        ("s01-example", "server", "derived"),
    ] {
        sqlx::query("INSERT INTO core.source (logical_source, display_name, expected_gap_sec) VALUES ($1, 'x', 21600)")
            .bind(name)
            .execute(&mut *tx)
            .await
            .unwrap();
        let r = sqlx::query(
            "SELECT acquisition, fetch_trigger FROM core.source WHERE logical_source = $1",
        )
        .bind(name)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(
            (r.get::<String, _>(0), r.get::<String, _>(1)),
            (a.into(), t.into()),
            "{name}"
        );
    }
}

// Scenario: 前置きの分からないソースはサーバの取り方で入る
#[tokio::test]
async fn source_with_unknown_prefix_is_server_derived() {
    let pool = testdb::pool().await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO core.source (logical_source, display_name, expected_gap_sec) VALUES ('x09-unknown', 'x', 21600)")
        .execute(&mut *tx)
        .await
        .unwrap();
    let r = sqlx::query("SELECT acquisition, fetch_trigger, fetch_interval_sec FROM core.source WHERE logical_source = 'x09-unknown'")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(r.get::<String, _>(0), "server");
    assert_eq!(r.get::<String, _>(1), "derived");
    assert_eq!(r.get::<Option<i32>, _>(2), None);
}

// Scenario: 取り方を足す前に別の移行が足した行にも取り方が入る
#[tokio::test]
async fn row_added_before_the_migration_gets_acquisition() {
    let (pool, drop) = fresh_before_this_migration().await;
    // 取り方の列を知らない INSERT をしてから、この移行を当てる
    sqlx::query(
        "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('c01-example', 'x', 21600, 'none')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(this_migration().1)
        .execute(&pool)
        .await
        .unwrap();
    let got = read(&pool, "c01-example").await;
    drop.await;
    assert_eq!(got, ("device".into(), "on_change".into(), None));
}

// Scenario: 間隔ごとでない契機のソースに取得の間隔を持たせると断られる
#[tokio::test]
async fn interval_on_a_non_interval_source_is_refused() {
    assert_refused(
        "UPDATE core.source SET fetch_interval_sec = 3600 WHERE logical_source = 'c03-youtube-watch'",
        "c03-youtube-watch",
        "source_fetch_interval_iff_interval_check",
    )
    .await;
}

#[tokio::test]
async fn interval_trigger_without_interval_is_refused() {
    assert_refused(
        "UPDATE core.source SET fetch_interval_sec = NULL WHERE logical_source = 'c01-location'",
        "c01-location",
        "source_fetch_interval_iff_interval_check",
    )
    .await;
}

// Scenario: 製品ごとに増えるマイアクティビティのソースは書庫の取り方で入る
#[tokio::test]
async fn myactivity_source_is_archive_manual() {
    let pool = testdb::pool().await;
    let name = format!("c03-myactivity-acq{}", uuid::Uuid::new_v4().simple());
    crate::archive::worker::ensure_myactivity_source(&pool, &name, "テスト")
        .await
        .unwrap();
    assert_eq!(
        read(&pool, &name).await,
        ("archive".into(), "manual".into(), None)
    );
}

// Scenario: 本人が変えた取得の間隔は移行を当て直しても戻らない
#[tokio::test]
async fn changed_interval_survives_reapplying_the_migration() {
    let (pool, drop) = fresh_before_this_migration().await;
    sqlx::raw_sql(this_migration().1)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE core.source SET fetch_interval_sec = 60 WHERE logical_source = 's01-archive-inbox'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(this_migration().1)
        .execute(&pool)
        .await
        .unwrap();
    let got = read(&pool, "s01-archive-inbox").await;
    drop.await;
    assert_eq!(got, ("server".into(), "interval".into(), Some(60)));
}
