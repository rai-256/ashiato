// SPDX-License-Identifier: AGPL-3.0-only
//! 試験の足場そのものの繋ぎ先を見る（CH04 / design D1・D3）。
#![allow(clippy::unwrap_used)]

use crate::testdb;

/// 接続先の `current_database()`。
async fn current_database(pool: &sqlx::PgPool) -> String {
    sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// URL の末尾（database 名）。
fn database_of(url: &str) -> &str {
    &url[url.rfind('/').unwrap() + 1..]
}

// Scenario: 試験の足場の繋ぎ先は、その回の database である
#[tokio::test]
async fn ch04_testdb_points_to_the_run_database() {
    let owner = testdb::pool().await;
    let app = testdb::app_pool().await;
    let owner_db = current_database(&owner).await;
    let app_db = current_database(&app).await;
    assert_eq!(
        owner_db, app_db,
        "pool() と app_pool() が別の database を指す"
    );
    assert!(
        owner_db.starts_with("ashiato_run_"),
        "回の database でない: {owner_db}"
    );
    assert_eq!(database_of(&testdb::url()), owner_db);
}

// Scenario: 試験が置いた記録は、ashiato database に入らない
#[tokio::test]
async fn ch04_testdb_records_stay_out_of_ashiato() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "ch04-out", 21_600).await;
    testdb::put_event(&pool, testdb::user(), &source, "2001-02-03T12:00:00+09:00").await;

    let in_run: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.event WHERE logical_source = $1")
            .bind(&source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(in_run, 1, "回の database に記録が無い");

    let run_url = testdb::url();
    let ashiato_url = format!("{}/ashiato", &run_url[..run_url.rfind('/').unwrap()]);
    let ashiato = sqlx::PgPool::connect(&ashiato_url).await.unwrap();
    // ashiato に表が無ければ「無い」と読む。
    let has_table: bool = sqlx::query_scalar("SELECT to_regclass('core.event') IS NOT NULL")
        .fetch_one(&ashiato)
        .await
        .unwrap();
    let in_ashiato: i64 = if has_table {
        sqlx::query_scalar("SELECT count(*) FROM core.event WHERE logical_source = $1")
            .bind(&source)
            .fetch_one(&ashiato)
            .await
            .unwrap()
    } else {
        0
    };
    ashiato.close().await;
    assert_eq!(in_ashiato, 0, "試験の記録が ashiato database に入った");
}

// Scenario: 回の database でも、消せない表は行の削除と TRUNCATE を断る
#[tokio::test]
async fn ch04_testdb_run_database_keeps_the_immutable_triggers() {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "ch04-trg", 21_600).await;
    testdb::put_event(&pool, testdb::user(), &source, "2001-02-03T12:00:00+09:00").await;

    let delete = sqlx::query("DELETE FROM core.event WHERE logical_source = $1")
        .bind(&source)
        .execute(&pool)
        .await;
    assert!(delete.is_err(), "行の削除が通った");
    let truncate = sqlx::query("TRUNCATE core.event").execute(&pool).await;
    assert!(truncate.is_err(), "TRUNCATE が通った");

    for name in ["event_no_delete", "event_no_truncate"] {
        let enabled: String = sqlx::query_scalar(
            "SELECT tgenabled::text FROM pg_trigger
              WHERE tgname = $1 AND tgrelid = 'core.event'::regclass",
        )
        .bind(name)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(enabled, "O", "{name} が常時有効でない");
    }
}

/// `pg_database` にその名前があるか。
async fn database_exists(name: &str) -> bool {
    let admin = testdb::pool().await;
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
        .bind(name)
        .fetch_one(&admin)
        .await
        .unwrap()
}

// Scenario: 試験ごとの database は、試験が通れば終わりに消える
#[tokio::test]
async fn ch04_isolated_database_is_dropped_after_the_test() {
    let db = testdb::isolated().await;
    let name = db.name().to_string();
    assert!(database_exists(&name).await, "作られていない: {name}");
    drop(db);
    assert!(!database_exists(&name).await, "消えていない: {name}");
}

/// 自分の runtime で future を走らせる別スレッド。panic はスレッドの join が受け止める
/// （`tokio::spawn` に sqlx の future を渡すと、rustc の高階の Send 推論が通らない）。
fn run_on_thread<T: Send + 'static, F: std::future::Future<Output = T>>(
    make: impl FnOnce() -> F + Send + 'static,
) -> std::thread::JoinHandle<T> {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(make())
    })
}

// Scenario: 試験ごとの database は、試験が panic しても消える
#[tokio::test]
async fn ch04_isolated_database_is_dropped_on_panic() {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let joined = run_on_thread(move || async move {
        let db = testdb::isolated().await;
        tx.send(db.name().to_string()).unwrap();
        let pool = db.pool().await;
        let source = testdb::source(&pool, "ch04-panic", 21_600).await;
        testdb::put_event(&pool, testdb::user(), &source, "2001-02-03T12:00:00+09:00").await;
        panic!("わざと panic する");
    })
    .join();
    assert!(joined.is_err(), "panic が伝わっていない");
    let name = rx.recv().unwrap();
    assert!(
        !database_exists(&name).await,
        "panic の後も残っている: {name}"
    );
}

// Scenario: 使い捨ての database の名前は、回の database の名前で始まる
#[tokio::test]
async fn ch04_scratch_database_name_starts_with_the_run() {
    drop(testdb::pool().await);
    let run = database_of(&testdb::url()).to_string();
    let db = testdb::scratch().await;
    assert!(
        db.name().starts_with(&format!("{run}_tmp_")),
        "回の名前で始まらない: {}",
        db.name()
    );
    let name = db.name().to_string();
    drop(db);
    assert!(!database_exists(&name).await, "消えていない: {name}");
}

/// 前提の確かめ（P16）: 16 本が同時に作って同時に消しても落ちない。
#[tokio::test]
async fn ch04_isolated_many_in_parallel() {
    drop(testdb::pool().await);
    let threads: Vec<_> = (0..16)
        .map(|_| {
            run_on_thread(|| async {
                let db = testdb::isolated().await;
                let pool = db.pool().await;
                let n: i64 = sqlx::query_scalar("SELECT count(*) FROM core.event")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                assert_eq!(n, 0);
                db.name().to_string()
            })
        })
        .collect();
    let names: Vec<String> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    for name in names {
        assert!(!database_exists(&name).await, "消えていない: {name}");
    }
}

/// 片付けの試験の材料づくりを 1 本ずつにする（ほかの試験の `sweep()` が作りかけの材料を消さないように。design の Risks）。
static SWEEP_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 材料の database（TEMPLATE を使わない空のもの）。
async fn create_empty_database(admin: &sqlx::PgPool, name: &str) {
    // db-ok: 片付けの試験の材料
    sqlx::raw_sql(&format!("CREATE DATABASE {name}"))
        .execute(admin)
        .await
        .unwrap();
}

async fn drop_database(admin: &sqlx::PgPool, name: &str) {
    sqlx::raw_sql(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
        .execute(admin)
        .await
        .unwrap();
}

/// `postgres` database に繋いだ所有者のプール（`CREATE DATABASE` の置き場）。
async fn postgres_admin() -> sqlx::PgPool {
    drop(testdb::pool().await);
    let url = testdb::url();
    let url = format!("{}/postgres", &url[..url.rfind('/').unwrap()]);
    sqlx::PgPool::connect(&url).await.unwrap()
}

// Scenario: 持ち主のいない回の database は、次の回の始まりに消える
#[tokio::test]
async fn ch04_sweep_drops_an_orphan_run() {
    let _serial = SWEEP_SERIAL.lock().await;
    let admin = postgres_admin().await;
    let id = format!(
        "ashiato_run_{}",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let names = [
        format!("{id}_tpl"),
        id.clone(),
        format!("{id}_x_aaaaaaaaaaaa"),
        format!("{id}_tmp_bbbbbbbbbbbb"),
    ];
    for name in &names {
        create_empty_database(&admin, name).await;
    }
    testdb::sweep().await;
    for name in &names {
        let gone = !database_exists(name).await;
        if !gone {
            drop_database(&admin, name).await;
        }
        assert!(gone, "持ち主のいない回の database が残っている: {name}");
    }
}

// Scenario: 走っている回の database は、接続が 0 本でも消えない
#[tokio::test]
async fn ch04_sweep_keeps_a_running_run_without_connections() {
    use sqlx::Connection;
    let _serial = SWEEP_SERIAL.lock().await;
    let admin = postgres_admin().await;
    let id = format!(
        "ashiato_run_{}",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    create_empty_database(&admin, &id).await;
    // 別の接続でその回の助言ロックを持つ（database には繋がない）。
    let url = testdb::url();
    let postgres_url = format!("{}/postgres", &url[..url.rfind('/').unwrap()]);
    let mut holder = sqlx::PgConnection::connect(&postgres_url).await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(4820253, hashtext($1))")
        .bind(&id)
        .execute(&mut holder)
        .await
        .unwrap();

    testdb::sweep().await;
    let kept = database_exists(&id).await;

    holder.close().await.unwrap();
    drop_database(&admin, &id).await;
    assert!(kept, "走っている回の database が消えた: {id}");
}

// Scenario: 回の印の無い使い捨ての database の残りは消え、ashiato と他の database は残る
#[tokio::test]
async fn ch04_sweep_drops_legacy_tmp_and_keeps_others() {
    let _serial = SWEEP_SERIAL.lock().await;
    let admin = postgres_admin().await;
    let rand = &uuid::Uuid::new_v4().simple().to_string()[..12];
    let legacy = format!("ch04x_tmp_{rand}");
    let other = format!("ch04keep_{rand}");
    create_empty_database(&admin, &legacy).await;
    create_empty_database(&admin, &other).await;

    testdb::sweep().await;
    let legacy_exists = database_exists(&legacy).await;
    let other_exists = database_exists(&other).await;
    let ashiato_exists = database_exists("ashiato").await;

    drop_database(&admin, &legacy).await;
    drop_database(&admin, &other).await;
    assert!(
        !legacy_exists,
        "印の無い _tmp_ の残りが残っている: {legacy}"
    );
    assert!(other_exists, "他の database が消えた: {other}");
    assert!(ashiato_exists, "ashiato が消えた");
}
