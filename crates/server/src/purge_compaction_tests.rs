// SPDX-License-Identifier: AGPL-3.0-only
//! 夜の書き直し（ST23 Task 7 / design D11）を、サーバと同じ `ashiato_app` の接続で、本物の PostgreSQL に対して確かめる。
//!
//! 表のファイル（`relfilenode`）が替わったかで書き直しを見る。`VACUUM FULL` は表の排他ロックを取り、
//! 共有の DB の上では並んだ試験を止める（逆に、並んだ試験の読み出しの後ろで待たされて失敗にもなる）ので、
//! 試験ごとに**使い捨ての DB**（`testdb::scratch`）で走らせる。結果の行は、走らせる前の最大の `id` より後で見る。
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::export::{ExportError, FailKind, Hooks, Stage};
use crate::export_job_tests::{app_with, post_status, wait_idle, wait_until_blocked};
use crate::export_tests::tmp_dir;
use crate::purge_compaction::{self, next_run_after, TableResult, TABLES};
use crate::purge_tests::{purge_now, scene_basic, scene_erase};
use crate::testdb;
use axum::http::StatusCode;

const POLL: Duration = Duration::from_millis(20);

fn idle() -> bool {
    false
}

async fn db_now(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT now()")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn filenode(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar("SELECT relfilenode::bigint FROM pg_class WHERE oid = $1::regclass")
        .bind(table)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn filenodes(pool: &PgPool) -> Vec<i64> {
    let mut v = Vec::new();
    for t in TABLES {
        v.push(filenode(pool, t).await);
    }
    v
}

/// 滞在を 1 件、本文ごと消す。
async fn erase_body(owner: &PgPool) {
    let (stay, _) = scene_basic(owner, testdb::user()).await;
    scene_erase(owner, stay).await;
    purge_now(owner, stay).await;
}

/// 前の晩までの消去を書き直しておく（以降の晩の対象が、この試験の消去だけになる）。
async fn settle(app: &PgPool, owner: &PgPool) -> DateTime<Utc> {
    sqlx::query("GRANT MAINTAIN ON core.event, core.event_version TO ashiato_app")
        .execute(owner)
        .await
        .unwrap();
    let now = db_now(owner).await;
    purge_compaction::run_once(app, now, &idle, POLL)
        .await
        .unwrap();
    now
}

/// 結果の行のいまの最大の `id`（これより後が、これから走らせる 1 回の分）。
async fn last_id(owner: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT coalesce(max(id), 0)::bigint FROM core.purge_compaction")
        .fetch_one(owner)
        .await
        .unwrap()
}

async fn outcome_rows(owner: &PgPool, table: &str, after_id: i64) -> Vec<(String, Option<String>)> {
    sqlx::query_as(
        "SELECT outcome, sqlstate FROM core.purge_compaction
          WHERE table_name = $1 AND id > $2 ORDER BY id",
    )
    .bind(table)
    .bind(after_id)
    .fetch_all(owner)
    .await
    .unwrap()
}

fn result_of<'a>(results: &'a [TableResult], table: &str) -> &'a TableResult {
    results.iter().find(|r| r.table == table).unwrap()
}

// Scenario: 本文を消した晩に表が書き直される
#[tokio::test]
async fn purge_compaction_rewrites_tables_after_erasure() {
    let db = testdb::scratch().await;
    let (owner, app) = (db.owner.clone(), db.app.clone());
    settle(&app, &owner).await;

    erase_body(&owner).await;
    let before = filenodes(&owner).await;
    let marker = last_id(&owner).await;
    let now = db_now(&owner).await;
    let results = purge_compaction::run_once(&app, now, &idle, POLL)
        .await
        .unwrap();

    let after = filenodes(&owner).await;
    for (i, table) in TABLES.iter().enumerate() {
        assert_ne!(before[i], after[i], "{table} のファイルが替わる");
        assert_eq!(result_of(&results, table).outcome, "rewritten", "{table}");
        assert_eq!(
            outcome_rows(&owner, table, marker).await,
            vec![("rewritten".to_string(), None)],
            "{table} の成功が残る"
        );
    }
}

// Scenario: 本文を消していない晩は書き直さない
#[tokio::test]
async fn purge_compaction_skips_night_without_erasure() {
    let db = testdb::scratch().await;
    let (owner, app) = (db.owner.clone(), db.app.clone());
    erase_body(&owner).await;
    // 前の晩と同じ締め切りで走らせる（並んだ試験が後から足す消去は数えない）
    let now = settle(&app, &owner).await;

    let before = filenodes(&owner).await;
    let marker = last_id(&owner).await;
    let results = purge_compaction::run_once(&app, now, &idle, POLL)
        .await
        .unwrap();

    assert!(results.is_empty(), "対象の表が無い");
    assert_eq!(
        filenodes(&owner).await,
        before,
        "どの表のファイルも替わらない"
    );
    for table in TABLES {
        assert!(
            outcome_rows(&owner, table, marker).await.is_empty(),
            "{table} に行を足さない"
        );
    }
}

// Scenario: 書き直せなかったら失敗として残り、次の晩にもう一度書き直す
#[tokio::test]
async fn purge_compaction_failure_is_recorded_and_retried_next_night() {
    let db = testdb::scratch().await;
    let (owner, app) = (db.owner.clone(), db.app.clone());
    settle(&app, &owner).await;
    erase_body(&owner).await;

    sqlx::query("REVOKE MAINTAIN ON core.event, core.event_version FROM ashiato_app")
        .execute(&owner)
        .await
        .unwrap();
    let before = filenodes(&owner).await;
    let marker = last_id(&owner).await;
    let night1 = db_now(&owner).await;
    let first = purge_compaction::run_once(&app, night1, &idle, POLL).await;
    sqlx::query("GRANT MAINTAIN ON core.event, core.event_version TO ashiato_app")
        .execute(&owner)
        .await
        .unwrap();
    let first = first.unwrap();

    assert_eq!(
        filenodes(&owner).await,
        before,
        "1 晩目はファイルが替わらない"
    );
    for table in TABLES {
        assert_eq!(result_of(&first, table).outcome, "not_rewritten", "{table}");
        let rows = outcome_rows(&owner, table, marker).await;
        assert_eq!(
            rows,
            vec![("not_rewritten".to_string(), None)],
            "{table} の失敗が残る"
        );
    }
    // 記録は表の名前・時刻・結果・SQLSTATE だけ（記録の値を持つ列が無い）
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::text FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'purge_compaction' ORDER BY ordinal_position",
    )
    .fetch_all(&owner)
    .await
    .unwrap();
    assert_eq!(
        columns,
        [
            "id",
            "table_name",
            "started_at",
            "finished_at",
            "outcome",
            "sqlstate"
        ]
    );

    let night2 = db_now(&owner).await;
    let second = purge_compaction::run_once(&app, night2, &idle, POLL)
        .await
        .unwrap();
    let after = filenodes(&owner).await;
    for (i, table) in TABLES.iter().enumerate() {
        assert_eq!(result_of(&second, table).outcome, "rewritten", "{table}");
        assert_ne!(before[i], after[i], "2 晩目に {table} のファイルが替わる");
    }
}

/// 消去より古いスナップショット（別の接続の REPEATABLE READ）が残っている晩は、書き直しを「済み」にしない（final review R5）。
/// `VACUUM FULL` はそのスナップショットから見える古い本文を新しいファイルへ写すので、ファイルが替わっても痕跡は残る。
/// スナップショットが閉じた次の晩に書き直す。
#[tokio::test]
async fn purge_compaction_old_snapshot_is_not_rewritten_and_retried() {
    let db = testdb::scratch().await;
    let (owner, app) = (db.owner.clone(), db.app.clone());
    settle(&app, &owner).await;

    // 消す前にスナップショットを取る（表には触らない —— 触ると排他ロックの待ちで lock_timeout になる）
    let mut held = owner.acquire().await.unwrap();
    sqlx::query("BEGIN ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *held)
        .await
        .unwrap();
    sqlx::query("SELECT 1").execute(&mut *held).await.unwrap();

    erase_body(&owner).await;
    let before = filenodes(&owner).await;
    let marker = last_id(&owner).await;
    let night1 = db_now(&owner).await;
    let first = purge_compaction::run_once(&app, night1, &idle, POLL)
        .await
        .unwrap();
    for table in TABLES {
        assert_eq!(result_of(&first, table).outcome, "not_rewritten", "{table}");
        assert_eq!(
            outcome_rows(&owner, table, marker).await,
            vec![("not_rewritten".to_string(), None)],
            "{table} は済みとして残らない"
        );
    }
    assert_eq!(
        filenodes(&owner).await,
        before,
        "古いスナップショットがある間は書き直さない"
    );

    sqlx::query("ROLLBACK").execute(&mut *held).await.unwrap();
    drop(held);
    let night2 = db_now(&owner).await;
    let second = purge_compaction::run_once(&app, night2, &idle, POLL)
        .await
        .unwrap();
    let after = filenodes(&owner).await;
    for (i, table) in TABLES.iter().enumerate() {
        assert_eq!(result_of(&second, table).outcome, "rewritten", "{table}");
        assert_ne!(before[i], after[i], "次の晩に {table} のファイルが替わる");
    }
}

/// 置き場の一時の下を、落ちても消す。
struct DirGuard(PathBuf);

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// Scenario: 書き出しの最中は書き直しを始めない
#[tokio::test]
async fn purge_compaction_waits_while_exporting() {
    let db = testdb::scratch().await;
    let (owner, app) = (db.owner.clone(), db.app.clone());
    settle(&app, &owner).await;
    erase_body(&owner).await;

    // 本物の書き出しの job を走らせ、最初の項目を書き始める前で止めておく。
    // 放したら、その項目で失敗させてすぐ終える（共有の DB の全行を書き出すと分単位でかかる）
    let dir = DirGuard(tmp_dir("compaction-wait"));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let released = Arc::new(AtomicBool::new(false));
    let hooks = Hooks::none()
        .with_before_file({
            let gate = gate.clone();
            move |_| {
                let gate = gate.clone();
                Box::pin(async move {
                    gate.acquire().await.unwrap().forget();
                })
            }
        })
        .with_fail({
            let released = released.clone();
            move |s| {
                (s == Stage::Item && released.load(Ordering::SeqCst))
                    .then(|| ExportError::new(FailKind::Io, "試験が書き出しを終える"))
            }
        });
    let server = app_with(&owner, &dir.0, hooks);
    let (code, body) = post_status(&server, crate::export_job_tests::PASS).await;
    assert_eq!(code, StatusCode::ACCEPTED, "{body}");
    wait_until_blocked(&server).await;
    assert!(server.is_exporting(), "書き出しの job が走っている");

    let before = filenodes(&owner).await;
    let marker = last_id(&owner).await;
    let now = db_now(&owner).await;
    let job = tokio::spawn({
        let (app, server) = (app.clone(), server.clone());
        async move {
            // 本番の spawn と同じ判断（App::is_exporting）を渡す
            purge_compaction::run_once(&app, now, &|| server.is_exporting(), POLL)
                .await
                .unwrap()
        }
    });

    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!job.is_finished(), "書き出しの間は待つ");
    assert_eq!(
        filenodes(&owner).await,
        before,
        "書き出しの間はファイルが替わらない"
    );
    for table in TABLES {
        assert!(
            outcome_rows(&owner, table, marker).await.is_empty(),
            "{table} に行を足さない"
        );
    }

    released.store(true, Ordering::SeqCst);
    gate.add_permits(1);
    wait_idle(&server).await;
    let results = job.await.unwrap();
    let after = filenodes(&owner).await;
    for (i, table) in TABLES.iter().enumerate() {
        assert_eq!(result_of(&results, table).outcome, "rewritten", "{table}");
        assert_ne!(
            before[i], after[i],
            "書き出しが終わってから {table} が替わる"
        );
    }
}

#[test]
fn purge_compaction_next_run_is_0330_tokyo() {
    let at = |s: &str| s.parse::<DateTime<Utc>>().unwrap();
    // 03:30 JST = 前日 18:30 UTC
    assert_eq!(
        next_run_after(at("2026-10-24T00:00:00+09:00")),
        at("2026-10-24T03:30:00+09:00")
    );
    assert_eq!(
        next_run_after(at("2026-10-24T03:29:59+09:00")),
        at("2026-10-24T03:30:00+09:00")
    );
    assert_eq!(
        next_run_after(at("2026-10-24T03:30:00+09:00")),
        at("2026-10-25T03:30:00+09:00")
    );
    assert_eq!(
        next_run_after(at("2026-10-24T23:59:00+09:00")),
        at("2026-10-25T03:30:00+09:00")
    );
}

/// 実物の量の時間を測る（design D11 の反転条件。`tools/st23-compaction-volume.sh` だけが `--ignored` で呼ぶ）。
/// 使い捨ての DB（`DATABASE_OWNER_URL` / `DATABASE_URL`）に 1 年分の行と 1 日分の消去が入っている前提で、
/// `run_once` を 1 回走らせて表ごとの秒数と大きさを標準出力に出す。
#[tokio::test]
#[allow(clippy::print_stdout)] // 測った秒数を道具に渡す口
#[ignore = "tools/st23-compaction-volume.sh が使い捨ての DB を用意して呼ぶ"]
async fn compaction_volume_run_once() {
    let connect = |name: &str| {
        let url = std::env::var(name).unwrap_or_else(|_| panic!("{name} が無い"));
        async move {
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&url)
                .await
                .unwrap()
        }
    };
    let owner = connect("DATABASE_OWNER_URL").await;
    let app = connect("DATABASE_URL").await;
    let size = |table: &'static str| {
        let owner = owner.clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT pg_total_relation_size($1::regclass)")
                .bind(table)
                .fetch_one(&owner)
                .await
                .unwrap()
        }
    };
    let mut before = Vec::new();
    for t in TABLES {
        before.push(size(t).await);
    }
    let marker = last_id(&owner).await;
    let now = db_now(&owner).await;
    let results = purge_compaction::run_once(&app, now, &idle, POLL)
        .await
        .unwrap();

    let mut total = 0.0;
    for (i, t) in TABLES.iter().enumerate() {
        let seconds: f64 = sqlx::query_scalar(
            "SELECT extract(epoch FROM finished_at - started_at)::float8
               FROM core.purge_compaction WHERE table_name = $1 AND id > $2",
        )
        .bind(t)
        .bind(marker)
        .fetch_one(&owner)
        .await
        .unwrap();
        total += seconds;
        println!(
            "table={t} outcome={} seconds={seconds:.1} bytes_before={} bytes_after={}",
            result_of(&results, t).outcome,
            before[i],
            size(t).await
        );
    }
    println!("seconds_total={total:.1}");
    for t in TABLES {
        assert_eq!(result_of(&results, t).outcome, "rewritten", "{t}");
    }
}
