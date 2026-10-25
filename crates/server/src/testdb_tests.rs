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

// Scenario: 書き出しの試験は、同じ回の他の試験が消した滞在を判定しない
#[tokio::test]
async fn ch04_export_isolated_does_not_see_other_tests() {
    let run = testdb::pool().await;
    let day = testdb::unique_day();
    let user = testdb::user();
    let insert = "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test',$4,$5::timestamptz,540,'Asia/Tokyo',1,$6,'{}',$7)";
    let stay = uuid::Uuid::new_v4();
    sqlx::query(insert)
        .bind(stay)
        .bind(user)
        .bind("s01-stay")
        .bind("derived")
        .bind(format!("{day}T03:00:00Z"))
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(serde_json::json!({"end": format!("{day}T04:00:00Z"), "lat": 35.0, "lon": 139.0}))
        .execute(&run)
        .await
        .unwrap();
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'user' WHERE id = $1")
        .bind(stay)
        .execute(&run)
        .await
        .unwrap();
    sqlx::query(insert)
        .bind(uuid::Uuid::new_v4())
        .bind(user)
        .bind("c01-location")
        .bind("collected")
        .bind(format!("{day}T03:30:00Z"))
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(serde_json::json!({"lat": 35.0, "lon": 139.0}))
        .execute(&run)
        .await
        .unwrap();
    let in_run: i64 = sqlx::query_scalar("SELECT count(*) FROM core.stay_erased WHERE id = $1")
        .bind(stay)
        .fetch_one(&run)
        .await
        .unwrap();
    assert_eq!(in_run, 1, "回の database に消した滞在が無い（前提）");

    let db = testdb::isolated().await;
    let pool = db.pool().await;
    for table in ["core.event", "core.stay_erased"] {
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 0, "試験ごとの database に {table} の行がある");
    }
    let summary = crate::export::read_summary(&pool).await.unwrap();
    let total: i64 = summary.sources.iter().map(|s| s.count).sum();
    assert_eq!(total, 0, "記録 0 件でない: {:?}", summary.sources);
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
    // 別の接続でその回の助言ロックを持つ（database には繋がない）。作る前に持つ —— 作ってから持つと、
    // その間に別の過程の片付けが持ち主のいない回として消せる。
    let url = testdb::url();
    let postgres_url = format!("{}/postgres", &url[..url.rfind('/').unwrap()]);
    let mut holder = sqlx::PgConnection::connect(&postgres_url).await.unwrap();
    sqlx::query("SELECT pg_advisory_lock($1, hashtext($2))")
        .bind(testdb::LOCK_RUN_CLASS)
        .bind(&id)
        .execute(&mut holder)
        .await
        .unwrap();
    create_empty_database(&admin, &id).await;

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

// Scenario: 上限を超える問い合わせは、止まって失敗する
#[tokio::test]
async fn ch04_statement_timeout_cancels_a_long_query() {
    let pool = testdb::pool_with_statement_timeout("1s").await;
    let started = std::time::Instant::now();
    let err = sqlx::query("SELECT pg_sleep(5)")
        .execute(&pool)
        .await
        .unwrap_err();
    let code = err
        .as_database_error()
        .and_then(|d| d.code())
        .map(|c| c.to_string());
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "5 秒を待った: {:?}",
        started.elapsed()
    );
    assert_eq!(code.as_deref(), Some("57014"), "取り消しでない: {err}");
}

async fn shown_statement_timeout(pool: &sqlx::PgPool) -> String {
    sqlx::query_scalar("SHOW statement_timeout")
        .fetch_one(pool)
        .await
        .unwrap()
}

// Scenario: 上限の既定は 60 秒で、試験の足場の接続に入っている
#[tokio::test]
async fn ch04_statement_timeout_default_is_set() {
    // 既定そのもの（環境変数を置いた環境でも、既定の値で繋いだ接続で見る。review/code.md R19）
    let default = testdb::pool_with_statement_timeout(&testdb::statement_timeout_from(None)).await;
    assert_eq!(shown_statement_timeout(&default).await, "1min");
    // 足場の全部の接続に、いまの環境が決める上限が入っている（review/code.md R13）
    let want = shown_statement_timeout(
        &testdb::pool_with_statement_timeout(&testdb::statement_timeout()).await,
    )
    .await;
    let isolated = testdb::isolated().await;
    let scratch = testdb::scratch().await;
    for (what, pool) in [
        ("pool()", testdb::pool().await),
        ("app_pool()", testdb::app_pool().await),
        ("isolated().pool()", isolated.pool().await),
        ("scratch().pool()", scratch.pool().await),
    ] {
        assert_eq!(
            shown_statement_timeout(&pool).await,
            want,
            "{what} に上限が入っていない"
        );
        pool.close().await;
    }
}

// Scenario: 環境変数の値が上限になり、0 なら上限を置かない
#[test]
fn ch04_statement_timeout_env_is_read() {
    // 環境変数は並走する試験に効くので、この試験の実行ファイルを子の過程で起こして渡す（review/code.md R13）
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "testdb_tests::ch04_statement_timeout_env_child",
            "--ignored",
            "--test-threads=1",
        ])
        .env("ASHIATO_TEST_STATEMENT_TIMEOUT", "7s")
        .env("CH04_EXPECT_STATEMENT_TIMEOUT", "7s")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && text.contains("1 passed"),
        "子の過程で環境変数の値が上限にならない:\n{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `ch04_statement_timeout_env_is_read` が子の過程で起こす側（単独では走らせない）。
#[test]
#[ignore = "ch04_statement_timeout_env_is_read が環境変数を渡して起こす"]
fn ch04_statement_timeout_env_child() {
    assert_eq!(
        testdb::statement_timeout(),
        std::env::var("CH04_EXPECT_STATEMENT_TIMEOUT").unwrap()
    );
}

// Scenario: 環境変数の値が上限になり、0 なら上限を置かない
#[test]
fn ch04_statement_timeout_from_env_value() {
    assert_eq!(testdb::statement_timeout_from(None), "60s");
    assert_eq!(testdb::statement_timeout_from(Some("1s")), "1s");
    assert_eq!(testdb::statement_timeout_from(Some("0")), "0");
}

/// 回の id は `ashiato_run_` と小文字の 16 進 12 桁（`tools/check-test-db-growth.sh` の正規表現と同じ定義。review/code.md R24）。
#[test]
fn ch04_run_id_is_lowercase_hex() {
    let id = "ashiato_run_0123456789ab";
    assert_eq!(testdb::run_id_of(id), Some(id));
    assert_eq!(testdb::run_id_of(&format!("{id}_tpl")), Some(id));
    assert_eq!(testdb::run_id_of(&format!("{id}_x_cccccccccccc")), Some(id));
    assert_eq!(testdb::run_id_of("ashiato_run_ABCDEF012345"), None);
    assert_eq!(testdb::run_id_of("ashiato_run_gggggggggggg"), None);
    assert_eq!(testdb::run_id_of("ashiato_run_0123456789a"), None);
    assert_eq!(testdb::run_id_of(&format!("{id}x")), None);
}
