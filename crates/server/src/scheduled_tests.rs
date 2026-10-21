// SPDX-License-Identifier: AGPL-3.0-only
//! 定期取得の枠（ST13 / design D5）。本番の一覧は空なので、取る手を差し込める `spawn_with` で回数を数える。
//! 時間は `tokio::time::pause` / `advance`（壁時計は読まない。その担保は `tools/check-scheduled-clock.sh`）。
#![allow(clippy::unwrap_used)]

use crate::archive_scan_interval_tests::{hold_clock, pool_for_paused_clock, settle};
use crate::scheduled::{FetchFuture, Fetcher};
use crate::testdb;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 回数と同時に走っている数を数える取る手。
#[derive(Default)]
struct Probe {
    calls: AtomicUsize,
    active: AtomicUsize,
    max_active: AtomicUsize,
}

struct CountingFetcher {
    name: String,
    probe: Arc<Probe>,
    takes: Duration,
    fails: bool,
}

impl CountingFetcher {
    fn boxed(name: &str, probe: &Arc<Probe>, takes: Duration, fails: bool) -> Box<dyn Fetcher> {
        Box::new(Self {
            name: name.to_owned(),
            probe: probe.clone(),
            takes,
            fails,
        })
    }
}

impl Fetcher for CountingFetcher {
    fn logical_source(&self) -> &str {
        &self.name
    }
    fn fetch<'a>(&'a self, _pool: &'a sqlx::PgPool, _user_id: uuid::Uuid) -> FetchFuture<'a> {
        Box::pin(async move {
            self.probe.calls.fetch_add(1, Ordering::SeqCst);
            let now = self.probe.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.probe.max_active.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(self.takes).await;
            self.probe.active.fetch_sub(1, Ordering::SeqCst);
            if self.fails {
                anyhow::bail!("わざと失敗する");
            }
            Ok(())
        })
    }
}

impl Probe {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

/// 自分専用の定期取得の行。共有の行は書き換えない。
async fn add_row(pool: &sqlx::PgPool, interval_sec: i32) -> String {
    let name = format!("test-sched-{}", uuid::Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind,
                                  acquisition, fetch_trigger, fetch_interval_sec)
         VALUES ($1, $1, 3600, 'none', 'scheduled', 'interval', $2)",
    )
    .bind(&name)
    .bind(interval_sec)
    .execute(pool)
    .await
    .unwrap();
    name
}

async fn drop_rows(pool: &sqlx::PgPool, names: &[&str]) {
    for name in names {
        sqlx::query("DELETE FROM core.source WHERE logical_source = $1")
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
    }
}

fn log_text() -> String {
    String::from_utf8_lossy(
        &crate::archive_flow_tests::captured_log()
            .0
            .lock()
            .unwrap()
            .clone(),
    )
    .into_owned()
}

/// 取る手が走るのを実時間で待つ（止めた時計は進まない）。
async fn wait_for_calls(probe: &Probe, n: usize) {
    for _ in 0..200 {
        if probe.calls() >= n {
            return;
        }
        settle(25).await;
    }
    panic!("取りに行った回数が {n} に届かない: {}", probe.calls());
}

/// 100 ミリ秒ずつ進め、そのつど眠りの timer と DB の読み直しを済ませる。刻みが粗いと、
/// 取得の後の読み直しの間に時計が先へ進み、眠りの始まりがずれて回数が減る。
async fn run_secs(secs: u64) {
    for _ in 0..secs * 10 {
        tokio::time::advance(Duration::from_millis(100)).await;
        settle(15).await;
    }
}

// Scenario: 起動の直後に 1 回取りに行く
#[tokio::test]
async fn scheduled_fetches_once_right_after_start() {
    let pool = pool_for_paused_clock().await;
    let name = add_row(&pool, 3600).await;
    let probe = Arc::new(Probe::default());
    tokio::time::pause();
    let started = tokio::time::Instant::now();
    let handle = crate::scheduled::spawn_with(
        vec![CountingFetcher::boxed(&name, &probe, Duration::ZERO, false)],
        pool.clone(),
        testdb::user(),
    );
    wait_for_calls(&probe, 1).await;
    let elapsed = started.elapsed();
    drop(handle);
    tokio::time::resume();
    drop_rows(&pool, &[&name]).await;

    assert_eq!(probe.calls(), 1, "起動の直後に 1 回");
    assert!(elapsed < Duration::from_secs(10), "10 秒以内: {elapsed:?}");
}

// Scenario: 取得の間隔ごとに取りに行く
#[tokio::test]
async fn scheduled_fetches_every_interval() {
    let pool = pool_for_paused_clock().await;
    let name = add_row(&pool, 2).await;
    let probe = Arc::new(Probe::default());
    tokio::time::pause();
    let handle = crate::scheduled::spawn_with(
        vec![CountingFetcher::boxed(&name, &probe, Duration::ZERO, false)],
        pool.clone(),
        testdb::user(),
    );
    wait_for_calls(&probe, 1).await;
    let before = probe.calls();
    let started = tokio::time::Instant::now();
    run_secs(11).await;
    let counted = probe.calls() - before + 1; // 起動の直後の 1 回を含めて 11 秒
    let elapsed = started.elapsed();
    drop(handle);
    tokio::time::resume();
    drop_rows(&pool, &[&name]).await;

    assert!(
        elapsed >= Duration::from_secs(11) && elapsed < Duration::from_secs(12),
        "仮想時間の経過は 11 秒のはず: {elapsed:?}"
    );
    assert!((5..=7).contains(&counted), "11 秒の回数: {counted}");
}

// Scenario: 止まっていた間に契機を過ぎても起動の直後に 1 回だけ取りに行く
#[tokio::test]
async fn scheduled_catches_up_with_one_fetch_after_a_stop() {
    let pool = pool_for_paused_clock().await;
    let name = add_row(&pool, 2).await;
    let probe = Arc::new(Probe::default());
    tokio::time::pause();
    let first = crate::scheduled::spawn_with(
        vec![CountingFetcher::boxed(&name, &probe, Duration::ZERO, false)],
        pool.clone(),
        testdb::user(),
    );
    wait_for_calls(&probe, 1).await;
    drop(first);
    // 止まっていた 1 分（契機を何度も過ぎる）
    tokio::time::advance(Duration::from_secs(60)).await;
    let before = probe.calls();
    let second = crate::scheduled::spawn_with(
        vec![CountingFetcher::boxed(&name, &probe, Duration::ZERO, false)],
        pool.clone(),
        testdb::user(),
    );
    wait_for_calls(&probe, before + 1).await;
    run_secs(1).await;
    let since_restart = probe.calls() - before;
    drop(second);
    tokio::time::resume();
    drop_rows(&pool, &[&name]).await;

    assert_eq!(since_restart, 1, "起動し直してから 1 秒で 1 回");
}

// Scenario: 前回の取得が終わる前に次の契機が来ても重ねない
#[tokio::test]
async fn scheduled_never_overlaps_a_slow_fetch() {
    let pool = pool_for_paused_clock().await;
    let name = add_row(&pool, 1).await;
    let probe = Arc::new(Probe::default());
    tokio::time::pause();
    let handle = crate::scheduled::spawn_with(
        vec![CountingFetcher::boxed(
            &name,
            &probe,
            Duration::from_secs(5),
            false,
        )],
        pool.clone(),
        testdb::user(),
    );
    wait_for_calls(&probe, 1).await;
    run_secs(20).await;
    drop(handle);
    tokio::time::resume();
    drop_rows(&pool, &[&name]).await;

    assert!(
        probe.calls() >= 2,
        "20 秒で複数回取りに行く: {}",
        probe.calls()
    );
    assert_eq!(
        probe.max_active.load(Ordering::SeqCst),
        1,
        "同時に走る取得は 1 つだけ"
    );
}

// Scenario: 1 つのソースの失敗はほかのソースを止めない
#[tokio::test]
async fn scheduled_one_failing_source_does_not_stop_the_others() {
    let pool = pool_for_paused_clock().await;
    let good_name = add_row(&pool, 2).await;
    let bad_name = add_row(&pool, 2).await;
    let good = Arc::new(Probe::default());
    let bad = Arc::new(Probe::default());
    tokio::time::pause();
    let handle = crate::scheduled::spawn_with(
        vec![
            CountingFetcher::boxed(&good_name, &good, Duration::ZERO, false),
            CountingFetcher::boxed(&bad_name, &bad, Duration::ZERO, true),
        ],
        pool.clone(),
        testdb::user(),
    );
    wait_for_calls(&good, 1).await;
    wait_for_calls(&bad, 1).await;
    run_secs(9).await;
    drop(handle);
    tokio::time::resume();
    drop_rows(&pool, &[&good_name, &bad_name]).await;

    assert!(
        good.calls() >= 4,
        "失敗しない側は間隔ごと: {}",
        good.calls()
    );
    assert!(
        bad.calls() >= 4,
        "失敗する側も次の契機で取りに行かれる: {}",
        bad.calls()
    );
    assert!(
        log_text()
            .lines()
            .any(|l| l.contains("scheduled_fetch_failed") && l.contains(&bad_name)),
        "失敗はログに出る"
    );
}

// Scenario: 取る手の無い定期取得のソースは取りに行かない
#[tokio::test]
async fn scheduled_source_without_a_fetcher_is_skipped_and_named_in_the_log() {
    let _ = crate::archive_flow_tests::captured_log(); // ログを集め始める
    let pool = testdb::pool().await;
    let name = add_row(&pool, 3600).await;
    let handle = crate::scheduled::spawn_with(Vec::new(), pool.clone(), testdb::user());
    for _ in 0..200 {
        if log_text().contains(&name) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    drop(handle);
    drop_rows(&pool, &[&name]).await;

    let lines: Vec<String> = log_text()
        .lines()
        .filter(|l| l.contains(&name))
        .map(str::to_owned)
        .collect();
    assert!(
        lines.iter().any(|l| l.contains("scheduled_no_fetcher")),
        "論理ソースの名前つきの行がログに出る: {lines:?}"
    );
    assert!(
        lines.iter().all(|l| !l.contains("scheduled_fetch_failed")),
        "取りに行かれていない: {lines:?}"
    );
}

// Scenario: 定期取得のソースが 0 本でも起動して失敗を出さない
#[tokio::test]
async fn scheduled_with_no_scheduled_sources_runs_quietly() {
    let _ = crate::archive_flow_tests::captured_log(); // ログを集め始める
    let pool = pool_for_paused_clock().await;
    // 本番の一覧（空）で起こす。ほかの試験の自分専用の行（test-sched-）は数えない
    let handle = crate::scheduled::spawn(pool.clone(), testdb::user());
    tokio::time::pause();
    hold_clock(settle(300)).await;
    tokio::time::advance(Duration::from_secs(60)).await;
    settle(100).await;
    tokio::time::resume();

    let noise: Vec<String> = log_text()
        .lines()
        .filter(|l| l.contains("scheduled_") && !l.contains("test-sched-"))
        .map(str::to_owned)
        .collect();
    drop(handle);
    assert!(
        noise
            .iter()
            .all(|l| !l.contains("scheduled_fetch_failed") && !l.contains("scheduled_registry")),
        "定期取得の失敗はログに出ない: {noise:?}"
    );
}

/// 取得の後に登録簿の間隔を読み直す: 眠る前に 3600 秒から 2 秒へ縮めると、次の契機は 2 秒後に来る。
#[tokio::test]
async fn scheduled_rereads_the_interval_after_each_fetch() {
    let pool = pool_for_paused_clock().await;
    let name = add_row(&pool, 2).await;
    let probe = Arc::new(Probe::default());
    tokio::time::pause();
    let handle = crate::scheduled::spawn_with(
        vec![CountingFetcher::boxed(
            &name,
            &probe,
            Duration::from_secs(1),
            false,
        )],
        pool.clone(),
        testdb::user(),
    );
    wait_for_calls(&probe, 1).await;
    // 1 回目の取得の最中に間隔を 3600 秒へ延ばす: 取得が終わってから読み直すので、眠りは 3600 秒になる
    hold_clock(
        sqlx::query("UPDATE core.source SET fetch_interval_sec = 3600 WHERE logical_source = $1")
            .bind(&name)
            .execute(&pool),
    )
    .await
    .unwrap();
    run_secs(10).await;
    let calls = probe.calls();
    drop(handle);
    tokio::time::resume();
    drop_rows(&pool, &[&name]).await;

    assert_eq!(calls, 1, "読み直した間隔（3600 秒）で眠る");
}

/// 失敗のログには種別と論理ソースの名前だけを出す（失敗の文言は記録の中身を含みうる）。
#[tokio::test]
async fn scheduled_failure_log_carries_no_error_text() {
    let _ = crate::archive_flow_tests::captured_log();
    let pool = testdb::pool().await;
    let name = add_row(&pool, 3600).await;
    let probe = Arc::new(Probe::default());
    let handle = crate::scheduled::spawn_with(
        vec![CountingFetcher::boxed(&name, &probe, Duration::ZERO, true)],
        pool.clone(),
        testdb::user(),
    );
    for _ in 0..200 {
        if log_text()
            .lines()
            .any(|l| l.contains("scheduled_fetch_failed") && l.contains(&name))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    drop(handle);
    drop_rows(&pool, &[&name]).await;

    let line = log_text()
        .lines()
        .find(|l| l.contains("scheduled_fetch_failed") && l.contains(&name))
        .map(str::to_owned)
        .expect("失敗のログが出る");
    assert!(
        !line.contains("わざと失敗する"),
        "失敗の文言は出さない: {line}"
    );
}
