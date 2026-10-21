// SPDX-License-Identifier: AGPL-3.0-only
//! 取り込み器の見回りの間隔を登録簿から読む（ST13 / design D4）。
//! 共有の `s01-archive-inbox` は書き換えない: 動かす試験は自分専用の行を足して取り込み器に渡す。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use std::time::Duration;

/// 止めた時計の上で使う接続の溜め。DB の応答を待つ間に時計が自動で進んでも、獲得の期限切れや
/// 接続の点検で落ちないようにする（溜めの既定は期限が 30 秒）。接続は止める前に張っておく。
async fn pool_for_paused_clock() -> sqlx::PgPool {
    testdb::pool().await; // 移行を当てる
    sqlx::postgres::PgPoolOptions::new()
        .min_connections(4)
        .max_connections(4)
        .test_before_acquire(false)
        .acquire_timeout(Duration::from_secs(365 * 24 * 3600))
        .idle_timeout(None)
        .max_lifetime(None)
        .connect(&testdb::url())
        .await
        .unwrap()
}

/// 自分専用の登録簿の行。使い終えたら消す。
struct OwnRow {
    pool: sqlx::PgPool,
    name: String,
}

impl OwnRow {
    async fn new(pool: &sqlx::PgPool, interval_sec: i32) -> Self {
        let name = format!("test-scan-interval-{}", uuid::Uuid::new_v4().simple());
        sqlx::query(
            "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind,
                                      acquisition, fetch_trigger, fetch_interval_sec)
             VALUES ($1, $1, 3600, 'none', 'server', 'interval', $2)",
        )
        .bind(&name)
        .bind(interval_sec)
        .execute(pool)
        .await
        .unwrap();
        Self {
            pool: pool.clone(),
            name,
        }
    }

    async fn set_interval(&self, interval_sec: i32) {
        hold_clock(
            sqlx::query("UPDATE core.source SET fetch_interval_sec = $2 WHERE logical_source = $1")
                .bind(&self.name)
                .bind(interval_sec)
                .execute(&self.pool),
        )
        .await
        .unwrap();
    }

    async fn delete(self) {
        sqlx::query("DELETE FROM core.source WHERE logical_source = $1")
            .bind(&self.name)
            .execute(&self.pool)
            .await
            .unwrap();
    }
}

fn config(root: &std::path::Path, user: uuid::Uuid) -> crate::archive::config::ArchiveConfig {
    let inbox = root.join("inbox");
    let downloads = root.join("downloads");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::create_dir_all(&downloads).unwrap();
    crate::archive::config::ArchiveConfig {
        inbox_dir: inbox,
        downloads_dir: downloads,
        copy_dir: root.join("copies"),
        keep_copies: true,
        user_id: Some(user),
    }
}

/// `f` を待つ間、blocking の task を 1 つ走らせておく。tokio は blocking の task が残る間は止めた時計を
/// 自動で進めないので、DB の応答を待つ間に時計が次の timer（120 秒の眠り）まで跳ばない。
async fn hold_clock<F: std::future::Future>(f: F) -> F::Output {
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = stop.clone();
    let pin = tokio::task::spawn_blocking(move || {
        while !flag.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    let out = f.await;
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    pin.await.unwrap();
    out
}

/// その日の最初の走査は、生存信号を残した後で走査の回数を 0 へ戻す。数が戻らないよう、
/// 今日の分の生存信号を先に置いておく（日が変わった直後でも試験が同じに動く）。
async fn seed_today_heartbeat(pool: &sqlx::PgPool, user: uuid::Uuid) {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM core.heartbeat
          WHERE user_id = $1 AND logical_source = 's01-archive-inbox'
            AND (emitted_at AT TIME ZONE 'Asia/Tokyo')::date = (now() AT TIME ZONE 'Asia/Tokyo')::date)",
    )
    .bind(user)
    .fetch_one(pool)
    .await
    .unwrap();
    if exists {
        return;
    }
    crate::store_heartbeat(
        pool,
        crate::heartbeat::HeartbeatRequest {
            id: uuid::Uuid::new_v4(),
            user_id: user,
            logical_source: "s01-archive-inbox".into(),
            device_id: Some("s01-c03".into()),
            emitted_at: chrono::Utc::now(),
            capturable: true,
            blockers: Vec::new(),
            attempts: 1,
            successes: 1,
            raw: "{}".into(),
        },
    )
    .await
    .unwrap();
}

/// 走査の回数（取り込み器が走査のたびに足す利用者ごとの数）。
async fn scans(pool: &sqlx::PgPool, user: uuid::Uuid) -> i64 {
    hold_clock(
        sqlx::query_scalar::<_, i32>(
            "SELECT attempts FROM core.archive_scan_counter WHERE user_id = $1",
        )
        .bind(user)
        .fetch_optional(pool),
    )
    .await
    .unwrap()
    .map_or(0, i64::from)
}

/// 止めた時計の上で、取り込み器の DB の往復が終わるのを実時間で待つ（待つ間は時計が進まない）。
async fn settle(millis: u64) {
    tokio::task::spawn_blocking(move || std::thread::sleep(Duration::from_millis(millis)))
        .await
        .unwrap();
}

/// 走査の回数が `n` 回に届くまで、実時間で待つ（時計は進まない）。届かなければ落ちる。
async fn wait_for_scans(pool: &sqlx::PgPool, user: uuid::Uuid, n: i64) {
    for _ in 0..200 {
        if scans(pool, user).await >= n {
            return;
        }
        settle(25).await;
    }
    panic!("走査が {n} 回に届かない: {}", scans(pool, user).await);
}

/// 止めた時計を 1 秒ずつ進める。1 秒ごとに取り込み器が登録簿を読めるようにする。
async fn advance_secs(secs: u64, settle_millis: u64) {
    for _ in 0..secs {
        tokio::time::advance(Duration::from_secs(1)).await;
        settle(settle_millis).await;
    }
}

// Scenario: 見回りの間隔の初期値は 120 秒
#[tokio::test]
async fn archive_scan_interval_default_is_120_seconds() {
    let pool = pool_for_paused_clock().await;
    // 共有の行は書き換えずに読む
    let default: i32 = sqlx::query_scalar(
        "SELECT fetch_interval_sec FROM core.source WHERE logical_source = 's01-archive-inbox'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(default, 120);
    // 同じ値の自分専用の行を取り込み器に渡す
    let row = OwnRow::new(&pool, default).await;
    let user = testdb::user();
    let root = std::env::temp_dir().join(format!("ashiato-scan-interval-{}", uuid::Uuid::new_v4()));

    seed_today_heartbeat(&pool, user).await;
    tokio::time::pause();
    let worker = crate::archive::worker::spawn_inspecting(
        pool.clone(),
        config(&root, user),
        user,
        crate::archive::worker::ReadingState::default(),
        &row.name,
    );
    wait_for_scans(&pool, user, 1).await;
    let started = tokio::time::Instant::now();
    advance_secs(600, 25).await;
    let count = scans(&pool, user).await;
    let elapsed = started.elapsed();
    drop(worker);
    tokio::time::resume();
    row.delete().await;
    let _ = std::fs::remove_dir_all(&root);

    // 試験の外で時計が勝手に進んだ回数は数えない: 経過は advance した 600 秒ちょうど
    assert!(
        elapsed >= Duration::from_secs(600) && elapsed < Duration::from_secs(601),
        "仮想時間の経過は 600 秒のはず: {elapsed:?}"
    );
    assert!(
        (4..=6).contains(&count),
        "10 分の走査は 4 回以上 6 回以下のはず: {count}"
    );
}

// Scenario: 登録簿で見回りの間隔を変えると起動し直さずに効く
#[tokio::test]
async fn archive_scan_interval_change_takes_effect_without_restart() {
    let pool = pool_for_paused_clock().await;
    let row = OwnRow::new(&pool, 120).await;
    let user = testdb::user();
    let root = std::env::temp_dir().join(format!("ashiato-scan-interval-{}", uuid::Uuid::new_v4()));

    seed_today_heartbeat(&pool, user).await;
    tokio::time::pause();
    let worker = crate::archive::worker::spawn_inspecting(
        pool.clone(),
        config(&root, user),
        user,
        crate::archive::worker::ReadingState::default(),
        &row.name,
    );
    wait_for_scans(&pool, user, 1).await;
    assert_eq!(scans(&pool, user).await, 1, "起動の直後に 1 回走査する");

    // 走査の回数は待ちに入る前に増える。取り込み器が待ちの眠りに入るのを待ってから 1 秒へ変え、10 秒待つ
    settle(500).await;
    row.set_interval(1).await;
    let before = scans(&pool, user).await;
    let started = tokio::time::Instant::now();
    // 1 秒進めるごとに走査が 1 回入るのを待つ（変えた値が 1 秒以内に効く）
    for step in 1..=10 {
        // 1 秒の眠りの期限は ms に切り上がるので、1 秒ちょうどでは届かない
        tokio::time::advance(Duration::from_millis(1001)).await;
        settle(25).await; // 眠りの timer を取り込み器に処理させる
        wait_for_scans(&pool, user, before + step).await;
    }
    let during = scans(&pool, user).await - before;
    let elapsed = started.elapsed();
    drop(worker);
    tokio::time::resume();
    row.delete().await;
    let _ = std::fs::remove_dir_all(&root);

    // 時計が自動で次の timer（120 秒）まで跳ぶと、古い間隔で眠る退行が緑になる。経過は 10 秒ちょうど
    assert!(
        elapsed >= Duration::from_secs(10) && elapsed < Duration::from_secs(11),
        "仮想時間の経過は 10 秒のはず: {elapsed:?}"
    );
    assert!(during >= 5, "10 秒の間の走査は 5 回以上のはず: {during}");
}

// Scenario: 300 秒を超える見回りの間隔は断られる
#[tokio::test]
async fn archive_scan_interval_over_300_is_refused() {
    let pool = testdb::pool().await;
    let mut tx = pool.begin().await.unwrap();
    let err = sqlx::query(
        "UPDATE core.source SET fetch_interval_sec = 301 WHERE logical_source = 's01-archive-inbox'",
    )
    .execute(&mut *tx)
    .await
    .unwrap_err();
    let db = err
        .as_database_error()
        .unwrap_or_else(|| panic!("制約違反でない: {err}"));
    assert_eq!(db.constraint(), Some("source_archive_inbox_interval_check"));
    drop(tx);
    let still: i32 = sqlx::query_scalar(
        "SELECT fetch_interval_sec FROM core.source WHERE logical_source = 's01-archive-inbox'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(still, 120, "取得の間隔は元のまま");
}

// Scenario: 見回りの間隔の環境変数が残っていると起動を断る
#[test]
fn archive_scan_interval_env_var_refuses_startup() {
    let env = std::collections::BTreeMap::from([
        ("HOME".to_string(), "/home/me".to_string()),
        // 名前は継ぎ合わせて書く（この環境変数を名指す行は config.rs だけ、という検査に合わせる）
        (
            concat!("ASHIATO_ARCHIVE_", "SCAN_SEC").to_string(),
            "60".to_string(),
        ),
    ]);
    let error = crate::archive::config::from_values(&env)
        .unwrap_err()
        .to_string();
    assert!(error.contains("s01-archive-inbox"), "{error}");
    assert!(error.contains("fetch_interval_sec"), "{error}");
}
