// SPDX-License-Identifier: AGPL-3.0-only
//! テストが本物の PostgreSQL を叩くための足場。**偽物に差し替えない。**
//!
//! 稼働記録の振る舞いは、書き換え禁止のトリガ・一意索引・`AT TIME ZONE` の
//! いずれも DB の側にある。模擬した DB で確かめると、**0004 が実測で見つけた
//! 「3 手の迂回」の型の穴がそのまま残る**。
//!
//! **接続できなければテストは落ちる。飛ばさない。** 飛ばすと、DB が無い環境で
//! 全部が緑になり、稼働記録の検査が 1 本も走っていないことに気付けない。
//!
//! **回の database（CH04 / design D1・D2・D3）。** 試験は `ashiato` database に書かない。過程ごとに回の id
//! （`ashiato_run_<乱数 12 桁の 16 進>`）を決め、`pool()` の初回が次の database を作る（名前は全部この id で始まる）:
//! - `<id>_tpl` —— 移行と権限を当てたテンプレート。当てた後は誰も繋がない
//! - `<id>` —— テンプレートの写し。`pool()`・`app_pool()`・`url()` の繋ぎ先
//!
//! 回が走っている印は、`postgres` database に繋いだ専用の接続が過程の終わりまで持つ助言ロック
//! （`pg_advisory_lock(4820253, hashtext('<id>'))`）。過程が殺されるとカーネルが接続を閉じ、ロックが外れる。
//! 片付けと作成は `pg_advisory_lock(4820252)` で 1 本ずつ行う。**`CREATE DATABASE` はこのファイルの中だけに書く。**
//! 消せない表のトリガは外さない（テンプレートにもそのまま入る）。
#![allow(clippy::unwrap_used)]

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;

/// この worktree のテスト用 DB の port。**規則は `tools/ports.sh` だけが持つ**ので、それを呼んで得る
/// （`ASHIATO_DB_PORT` > main の checkout・`-st<NN>`・`-verify`・それ以外の worktree の台帳）。
/// 並行して走る Story・change のテストが、別の木の DB に繋がらないようにする（2026-09-30・2026-10-21）。
fn db_port() -> u16 {
    static PORT: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    *PORT.get_or_init(|| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = std::process::Command::new("bash")
            .args(["-c", r#". tools/ports.sh && printf %s "$ASHIATO_DB_PORT""#])
            .current_dir(&root)
            .output()
            .expect("tools/ports.sh を呼べない（bash が要る）");
        let port = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "tools/ports.sh が port を決められない: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        port.parse()
            .unwrap_or_else(|_| panic!("tools/ports.sh の port が数でない: {port:?}"))
    })
}

/// 回の database の所有者の URL。**合言葉は環境（`.env` の `DATABASE_OWNER_URL`）からだけ**読み（ST28 / design D19）、
/// port だけをこの worktree の DB のものに差し替える（ST05。`tools/ports.sh` の `ashiato_db_url` と同じ）。
/// 同期の関数なので、回がまだ作られていなければ panic する（先に `pool()` を呼ぶ。design D3）。
pub fn url() -> String {
    RUN.get()
        .expect("回の database がまだ無い。先に testdb::pool() を呼ぶ")
        .url
        .clone()
}

/// 接続の URL の `@<host>:<port>/` の port を差し替える（利用者と合言葉の部分は触らない）。形が違えばそのまま返す。
fn with_port(url: &str, port: u16) -> String {
    let Some(at) = url.rfind('@') else {
        return url.to_string();
    };
    let rest = &url[at + 1..];
    let Some(slash) = rest.find('/') else {
        return url.to_string();
    };
    let host = rest[..slash]
        .rsplit_once(':')
        .map_or(&rest[..slash], |(h, _)| h);
    format!("{}@{host}:{port}{}", &url[..at], &rest[slash..])
}

/// 試験の 1 本の問い合わせの時間の上限の既定（CH04 / design D7）。
const DEFAULT_STATEMENT_TIMEOUT: &str = "60s";

/// 環境変数 `ASHIATO_TEST_STATEMENT_TIMEOUT` の値（無ければ `None`）から `statement_timeout` に置く値を決める。
/// 無ければ既定の 60 秒、あればその値（PostgreSQL の時間の書き方。`0` は上限なし）。純粋な関数
/// （環境変数は `statement_timeout()` だけが読む。試験は環境変数を書き換えずにこれを確かめる）。
pub fn statement_timeout_from(env_value: Option<&str>) -> String {
    match env_value.map(str::trim) {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => DEFAULT_STATEMENT_TIMEOUT.to_string(),
    }
}

/// いまの環境が決める上限（試験が、足場の接続に入った値と比べるためにも使う）。
pub(crate) fn statement_timeout() -> String {
    statement_timeout_from(
        std::env::var("ASHIATO_TEST_STATEMENT_TIMEOUT")
            .ok()
            .as_deref(),
    )
}

/// `statement_timeout` を繋ぐときの `options` で置いた接続の設定。
fn options_with_timeout(url: &str, timeout: &str) -> PgConnectOptions {
    PgConnectOptions::from_str(url)
        .unwrap_or_else(|e| panic!("接続の URL を読めない: {e}"))
        .options([("statement_timeout", timeout)])
}

/// 上限を引数で受けて、URL へ繋いだプールを作る（`pg_sleep` の試験が 1 秒を渡す。design D7）。
async fn connect_pool(url: &str, timeout: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options_with_timeout(url, timeout))
        .await
}

/// 回の database（design D1）。`pool()` の初回が作り、URL だけを覚える（プールは runtime を越えて持てない）。
static RUN: tokio::sync::OnceCell<Run> = tokio::sync::OnceCell::const_new();

struct Run {
    /// 回の database の URL（所有者）。
    url: String,
}

/// 片付けと作成の直列化の鍵（design D2。移行の直列化だった 4820251 とは別）。
const LOCK_SWEEP_AND_CREATE: i64 = 4_820_252;
/// 回の助言ロックの第 1 鍵（第 2 鍵は `hashtext('<id>')`）。
pub(crate) const LOCK_RUN_CLASS: i32 = 4_820_253;

/// 回の database の名前（`url()` の末尾）。
fn database_of_run() -> String {
    let url = url();
    url[url.rfind('/').unwrap() + 1..].to_string()
}

/// 接続の URL の末尾の database 名を差し替える。
fn with_database(url: &str, database: &str) -> String {
    let slash = url.rfind('/').expect("接続の URL に database が無い");
    format!("{}/{database}", &url[..slash])
}

/// 乱数 12 桁の 16 進。
fn random12() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

/// 回の助言ロックを、専用のスレッドと runtime で取り、過程が終わるまで手放さない（design D1 / C6）。
/// `#[tokio::test]` の runtime に置くと、その試験の終わりに接続ごと死ぬ。
fn hold_run_lock(postgres_url: String, run_id: String) {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::Builder::new()
        .name("testdb-run-lock".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let mut conn = <sqlx::PgConnection as sqlx::Connection>::connect(&postgres_url)
                    .await
                    .unwrap_or_else(|e| panic!("回の助言ロック用の接続ができない: {e}"));
                sqlx::query("SELECT pg_advisory_lock($1, hashtext($2))")
                    .bind(LOCK_RUN_CLASS)
                    .bind(&run_id)
                    .execute(&mut conn)
                    .await
                    .unwrap();
                tx.send(()).unwrap();
                // 過程が終わるまで接続を持つ（スレッドは過程の終わりに道連れで終わる）。
                std::future::pending::<()>().await;
                drop(conn);
            });
        })
        .expect("回の助言ロックのスレッドを立てられない");
    rx.recv().expect("回の助言ロックを取れなかった");
}

/// 名前が回の database（`ashiato_run_<12 桁の 16 進>` の後ろが空か `_…`）なら、その回の id。
pub(crate) fn run_id_of(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("ashiato_run_")?;
    let digits = rest.get(..12)?;
    let tail = &rest[12..];
    // 16 進は小文字だけ（`random12()` が作る字。`tools/check-test-db-growth.sh` の回の id と同じ定義）
    (digits
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        && (tail.is_empty() || tail.starts_with('_')))
    .then(|| &name[.."ashiato_run_".len() + 12])
}

/// 前の回の database の片付け（design D6）。呼ぶ側が `LOCK_SWEEP_AND_CREATE` を持っている。
/// 消すのは、持ち主のいない回（助言ロックを取れた）のものと、回の印の無い `_tmp_` の残りだけ。
/// 消せなかったものは名前と理由を出して戻る（次の回がまた消す）。
async fn sweep_previous_runs(admin: &mut sqlx::PgConnection) {
    let names: Vec<String> = sqlx::query_scalar("SELECT datname::text FROM pg_database")
        .fetch_all(&mut *admin)
        .await
        .unwrap();
    for name in names {
        if matches!(
            name.as_str(),
            "ashiato" | "postgres" | "template0" | "template1"
        ) {
            continue;
        }
        let doomed = if let Some(id) = run_id_of(&name) {
            let got: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1, hashtext($2))")
                .bind(LOCK_RUN_CLASS)
                .bind(id)
                .fetch_one(&mut *admin)
                .await
                .unwrap();
            if got {
                sqlx::query("SELECT pg_advisory_unlock($1, hashtext($2))")
                    .bind(LOCK_RUN_CLASS)
                    .bind(id)
                    .execute(&mut *admin)
                    .await
                    .unwrap();
            }
            got
        } else {
            name.contains("_tmp_") && !name.starts_with("ashiato_run_")
        };
        if !doomed {
            continue;
        }
        let sql = format!(
            "DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)",
            name.replace('"', "\"\"")
        );
        if let Err(e) = sqlx::raw_sql(&sql).execute(&mut *admin).await {
            // log-ok: 試験の足場の診断。database 名と DB のエラーだけで私的データは含まない
            eprintln!("testdb: {name} を片付けられなかった: {e}");
        }
    }
}

/// 前の回の database の片付けを、試験から呼べる形で（design D6）。
/// 片付けと作成の鍵を取って 1 本ずつ行い、終わったら外す。
pub async fn sweep() {
    use sqlx::Connection;
    let owner_url = with_port(&url_from_env("DATABASE_OWNER_URL"), db_port());
    let mut admin = sqlx::PgConnection::connect(&with_database(&owner_url, "postgres"))
        .await
        .unwrap_or_else(|e| panic!("テスト用 DB へ所有者で接続できない: {e}"));
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(LOCK_SWEEP_AND_CREATE)
        .execute(&mut admin)
        .await
        .unwrap();
    sweep_previous_runs(&mut admin).await;
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(LOCK_SWEEP_AND_CREATE)
        .execute(&mut admin)
        .await
        .unwrap();
    admin.close().await.unwrap();
}

/// `CREATE DATABASE <name> TEMPLATE <template>`。直前に閉じた接続の backend が残っていると
/// 「source database is being accessed by other users」（55006）で断られるので、少し待って打ち直す。
async fn create_from_template(admin: &mut sqlx::PgConnection, name: &str, template: &str) {
    let sql = format!("CREATE DATABASE {name} TEMPLATE {template}");
    for attempt in 0..100 {
        match sqlx::query(&sql).execute(&mut *admin).await {
            Ok(_) => return,
            Err(e)
                if attempt < 99
                    && e.as_database_error()
                        .and_then(|d| d.code())
                        .is_some_and(|c| c == "55006") =>
            {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            Err(e) => panic!("{sql} が通らない: {e}"),
        }
    }
}

/// 回の database の作成に入ったか。`OnceCell` は初期化が panic すると未設定に戻り、次の試験がまた作り直す
/// —— そのたびに回の助言ロックの接続と作りかけの `<id>_tpl` が過程の終わりまで残り、接続の上限を使い切って
/// 本当の原因（移行のエラーなど）を埋める（review/code.md R17）。2 度目は作らずに、最初の失敗を見るよう示して落ちる。
static RUN_ATTEMPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 回の database を作る（design D2 の 1〜6）。
async fn create_run() -> Run {
    use sqlx::Connection;
    assert!(
        !RUN_ATTEMPTED.swap(true, std::sync::atomic::Ordering::SeqCst),
        "回の database の作成が前に落ちている。最初に落ちた試験の出力（移行や権限のエラー）を見る"
    );
    let owner_url = with_port(&url_from_env("DATABASE_OWNER_URL"), db_port());
    let postgres_url = with_database(&owner_url, "postgres");
    let id = format!("ashiato_run_{}", random12());
    let tpl = format!("{id}_tpl");

    let mut admin = sqlx::PgConnection::connect(&postgres_url).await.unwrap_or_else(|e| {
        panic!("テスト用 DB へ所有者で接続できない: {e}\n  tools/db.sh up -d --wait db のあと tools/db-roles.sh を先に実行する")
    });
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(LOCK_SWEEP_AND_CREATE)
        .execute(&mut admin)
        .await
        .unwrap();
    hold_run_lock(postgres_url, id.clone());
    sweep_previous_runs(&mut admin).await;

    sqlx::raw_sql(&format!("CREATE DATABASE {tpl}"))
        .execute(&mut admin)
        .await
        .unwrap();
    // 移行は本番と同じ `crate::migrate`（版の記録と付与を錠の中で当てる。ST48）でテンプレートに当てる
    let tpl_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options_with_timeout(
            &with_database(&owner_url, &tpl),
            &statement_timeout(),
        ))
        .await
        .unwrap();
    crate::migrate(&tpl_pool).await.unwrap();
    tpl_pool.close().await;

    create_from_template(&mut admin, &id, &tpl).await;
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(LOCK_SWEEP_AND_CREATE)
        .execute(&mut admin)
        .await
        .unwrap();
    admin.close().await.unwrap();
    Run {
        url: with_database(&owner_url, &id),
    }
}

/// 試験ごとの写しは回の中で 1 本ずつ作る（同じテンプレートからの同時の写しを避ける。design D2 / P16）。
static COPY_ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 試験が持つ database（design D4・D5）。`isolated()` は回のテンプレートの写し、`scratch()` は空。
/// `Drop` が自分のスレッドと runtime で `DROP DATABASE IF EXISTS … WITH (FORCE)` を打って待つ。
/// 試験が通っても panic しても（巻き戻しの途中でも）走る。**Drop は panic しない**: 失敗は名前と理由を出して戻り、
/// 残りは次の回の片付け（design D6）が消す。
#[derive(Debug)]
pub struct TestDb {
    name: String,
    url: String,
}

impl TestDb {
    /// この database に繋いだプール（呼ぶたびに同じ database。プールは呼んだ runtime のもの）。
    pub fn pool(&self) -> impl std::future::Future<Output = sqlx::PgPool> + Send + 'static {
        // `&self` を future に持ち込まない（`tokio::spawn` の中で呼べるように）。
        let (url, name) = (self.url.clone(), self.name.clone());
        async move {
            connect_pool(&url, &statement_timeout())
                .await
                .unwrap_or_else(|e| panic!("{name} へ所有者で接続できない: {e}"))
        }
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let postgres_url = with_database(&self.url, "postgres");
        let name = self.name.clone();
        let job = std::thread::Builder::new()
            .name("testdb-drop".into())
            .spawn({
                let name = name.clone();
                move || -> Result<(), String> {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| format!("runtime を作れない: {e}"))?;
                    rt.block_on(async {
                        use sqlx::Connection;
                        let mut conn = sqlx::PgConnection::connect(&postgres_url)
                            .await
                            .map_err(|e| format!("postgres database へ繋げない: {e}"))?;
                        let mut dropped = Ok(());
                        // WITH (FORCE) が「permission denied to terminate process」で拒まれたことが 1 度ある
                        // （ch04_isolated_many_in_parallel。app の役で繋いだ試験は無く、ashiato_app の接続ではない）。
                        // 原因は突き止めていない（autovacuum など superuser の backend を疑うが、確かめていない）。
                        // そうした接続はすぐ去るので、100ms から倍々（上限 3.2 秒）で 7 回待つ合計約 9.5 秒までやり直して吸収する。
                        // それでも消せなければ、下で pg_stat_activity の繋がっているものを書き出し、次に起きたときに原因を名指す。
                        let mut wait_ms = 100;
                        for attempt in 0..8 {
                            if attempt > 0 {
                                tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
                                wait_ms = (wait_ms * 2).min(3200);
                            }
                            dropped =
                                sqlx::raw_sql(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
                                    .execute(&mut conn)
                                    .await
                                    .map(|_| ())
                                    .map_err(|e| format!("DROP DATABASE が通らない: {e}"));
                            if dropped.is_ok() {
                                break;
                            }
                        }
                        if dropped.is_err() {
                            let who: Vec<String> = sqlx::query_scalar(
                                "SELECT usename || '/' || backend_type FROM pg_stat_activity WHERE datname = $1",
                            )
                            .bind(&name)
                            .fetch_all(&mut conn)
                            .await
                            .unwrap_or_default();
                            dropped = dropped.map_err(|e| format!("{e}（繋がっているもの: {who:?}）"));
                        }
                        let _ = conn.close().await;
                        dropped
                    })
                }
            });
        let reason = match job.map(std::thread::JoinHandle::join) {
            Ok(Ok(Ok(()))) => return,
            Ok(Ok(Err(reason))) => reason,
            Ok(Err(_)) => "片付けのスレッドが panic した".to_string(),
            Err(e) => format!("片付けのスレッドを立てられない: {e}"),
        };
        // log-ok: 試験の足場の診断。database 名と所要時間・DB のエラーだけで私的データは含まない
        eprintln!("testdb: {name} を消せなかった（次の回の片付けが消す）: {reason}");
    }
}

/// 回のテンプレートの写しを、この試験だけの database として作る（書き出しの試験用。design D4）。
pub async fn isolated() -> TestDb {
    RUN.get_or_init(create_run).await;
    let run = database_of_run();
    let name = format!("{run}_x_{}", random12());
    let postgres_url = with_database(&url(), "postgres");
    let mut admin = <sqlx::PgConnection as sqlx::Connection>::connect(&postgres_url)
        .await
        .unwrap_or_else(|e| panic!("postgres database へ所有者で接続できない: {e}"));
    {
        let _one = COPY_ONE_AT_A_TIME.lock().await;
        let copy_started = std::time::Instant::now();
        create_from_template(&mut admin, &name, &format!("{run}_tpl")).await;
        // log-ok: 試験の足場の診断。database 名と所要時間・DB のエラーだけで私的データは含まない
        eprintln!("testdb: {name} の写しに {:?}", copy_started.elapsed());
    }
    let _ = sqlx::Connection::close(admin).await;
    TestDb {
        url: with_database(&url(), &name),
        name,
    }
}

/// 空の使い捨ての database（`<回の id>_tmp_…`。移行の当て方などを見る試験用。design D5）。
pub async fn scratch() -> TestDb {
    RUN.get_or_init(create_run).await;
    let name = format!("{}_tmp_{}", database_of_run(), random12());
    let postgres_url = with_database(&url(), "postgres");
    let mut admin = <sqlx::PgConnection as sqlx::Connection>::connect(&postgres_url)
        .await
        .unwrap_or_else(|e| panic!("postgres database へ所有者で接続できない: {e}"));
    sqlx::raw_sql(&format!("CREATE DATABASE {name}"))
        .execute(&mut admin)
        .await
        .unwrap_or_else(|e| panic!("CREATE DATABASE {name} が通らない: {e}"));
    let _ = sqlx::Connection::close(admin).await;
    TestDb {
        url: with_database(&url(), &name),
        name,
    }
}

/// 環境からだけ読む。**既定の URL（固定の合言葉入り）は持たない**（ST28 / design D19）。
/// 無ければ飛ばさずに落ちる。値（合言葉入りの URL）は出さない。
fn url_from_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        panic!("{name} が無い。.env を読み込む（set -a; . ./.env; set +a）か {name} を渡す")
    })
}

/// 接続済みのプールを返す。初回だけ回の database を作る（冒頭の注記）。
/// **所有者（`DATABASE_OWNER_URL`）で繋ぐ。** 門はトリガで効くので、所有者でも止まることを既存の試験が見る。
///
/// **プールは試験ごとに持つ。** 1 本を共有すると、背景の取り込み器を起こす結合
/// 試験どうしが同じ接続を奪い合って全体が止まる（実測）。そのぶん PostgreSQL 側の
/// 接続の上限を上げてある（`docker-compose.yml` の `max_connections`）。
pub async fn pool() -> sqlx::PgPool {
    pool_with_statement_timeout(&statement_timeout()).await
}

/// 上限を引数で受ける `pool()`（design D7）。環境変数は読まない。
pub async fn pool_with_statement_timeout(timeout: &str) -> sqlx::PgPool {
    let run = RUN.get_or_init(create_run).await;
    connect_pool(&run.url, timeout)
        .await
        .unwrap_or_else(|e| {
            panic!("回の database へ所有者で接続できない: {e}\n  tools/db.sh up -d --wait db のあと tools/db-roles.sh を先に実行する")
        })
}

/// 使い捨ての database（ST48 / design D16）。版の記録に知らない行を書く試験が、worktree で共有する DB を汚さないために使う。
/// 作るのは所有者の役割（`CREATEDB` を持つ）。CH04 の `scratch()` の口に載せる（design D16）。
#[derive(Debug)]
pub struct TempDb {
    pool: sqlx::PgPool,
    /// Drop で `DROP DATABASE … WITH (FORCE)` する（`TestDb` の Drop）。
    db: TestDb,
}

impl TempDb {
    /// 空の使い捨ての database（`testdb::scratch()` の `<回の id>_tmp_…`。CH04 の片付けが残りも消す）を作り、
    /// 所有者で繋いだプールを持つ。
    pub async fn create() -> Self {
        let db = scratch().await;
        let pool = connect_pool(&db.url(), &statement_timeout())
            .await
            .unwrap_or_else(|e| panic!("使い捨ての database へ所有者で接続できない: {e}"));
        Self { pool, db }
    }

    /// 所有者で繋いだプール（移行・付与・記録の書き換えに使う）。
    pub fn owner(&self) -> &sqlx::PgPool {
        &self.pool
    }

    /// アプリの役割（`DATABASE_URL`）で繋いだ新しいプール。
    pub async fn app(&self) -> sqlx::PgPool {
        let url = with_database(
            &with_port(&url_from_env("DATABASE_URL"), db_port()),
            self.db.name(),
        );
        PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap_or_else(|e| panic!("使い捨ての database へアプリの役割で接続できない: {e}"))
    }
}

/// アプリの役割（`DATABASE_URL`）で繋いだプール。**役割の試験だけが使う。** 繋ぎ先は回の database。
/// 先に回の database を作っておく。
pub async fn app_pool() -> sqlx::PgPool {
    RUN.get_or_init(create_run).await;
    let url = with_database(
        &with_port(&url_from_env("DATABASE_URL"), db_port()),
        &database_of_run(),
    );
    connect_pool(&url, &statement_timeout())
        .await
        .unwrap_or_else(|e| {
            panic!("テスト用 DB へアプリの役割で接続できない: {e}\n  役割が無いなら tools/db-roles.sh を先に実行する")
        })
}

/// 試験専用の使い捨て DB（所有者の接続・アプリの役割の接続つき）。表ごと書き直す（`VACUUM FULL`）ような、
/// 共有の DB の上では並んだ試験を止めてしまう操作のための場所（ST23）。中身は `isolated()` の写しで、
/// `Drop` は `TestDb` が DB を捨てる。
#[derive(Debug)]
pub struct ScratchDb {
    pub owner: sqlx::PgPool,
    pub app: sqlx::PgPool,
    /// 接続より後に落とす（フィールドは宣言の順に落ちる）。
    _db: TestDb,
}

/// 移行と権限を当てた使い捨て DB を、所有者とアプリの役割の両方で繋いで返す。
pub async fn isolated_with_app() -> ScratchDb {
    let db = isolated().await;
    let owner = db.pool().await;
    let app = PgPoolOptions::new()
        .max_connections(2)
        .connect(&with_database(
            &with_port(&url_from_env("DATABASE_URL"), db_port()),
            db.name(),
        ))
        .await
        .unwrap_or_else(|e| panic!("使い捨ての database へアプリの役割で接続できない: {e}"));
    ScratchDb {
        owner,
        app,
        _db: db,
    }
}

/// テストごとに固有の論理ソースを登録簿へ置く。
///
/// **テストどうしの隔離をこれで取る**（design D14）—— 1 つの DB を共有したまま、
/// 稼働記録・生存信号・停止/破棄はすべて `logical_source` で分かれる。
/// 表を作り直したり schema を掘ったりしない（本物と同じ表を叩くことに意味がある）。
///
/// **外部識別子の粒度は `none`**（ST03 / 深掘り Q4 / Q16）。登録簿の既定は
/// `'record'`（＝識別子が無ければ断る側）なので、**明示しないと端末からの
/// 記録が全件 400 になる**。粒度そのものを見るテストは `source_of_kind` を使う。
pub async fn source(pool: &sqlx::PgPool, prefix: &str, expected_gap_sec: i32) -> String {
    source_registered_on(pool, prefix, expected_gap_sec, FAR_PAST).await
}

/// 外部識別子の粒度を明示して登録簿へ置く（ST03 / 深掘り Q13 / Q18）。
/// `kind` は `record` / `subject` / `none`。
pub async fn source_of_kind(pool: &sqlx::PgPool, prefix: &str, kind: &str) -> String {
    let name = source(pool, prefix, 21_600).await;
    sqlx::query("UPDATE core.source SET external_id_kind = $2 WHERE logical_source = $1")
        .bind(&name)
        .bind(kind)
        .execute(pool)
        .await
        .unwrap();
    name
}

/// 粒度を**宣言しないまま**登録簿へ置く（既定に倒れることを見るテスト用）。
pub async fn source_undeclared(pool: &sqlx::PgPool, prefix: &str) -> String {
    let name = format!("t-{prefix}-{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, registered_at)
         VALUES ($1, $1, 21600, ($2::date::timestamp AT TIME ZONE 'Asia/Tokyo'))",
    )
    .bind(&name)
    .bind(FAR_PAST)
    .execute(pool)
    .await
    .unwrap();
    name
}

/// テストの記録は**過去の日**に置く（2025〜2026 年）。登録簿の既定の `registered_at` は
/// `now()` なので、そのままだと**第 8 回 Q29 の閾値がテストの記録を全部外す**。
///
/// 「そのソースはずっと前から登録されていた」を既定に取る。閾値そのものを見るテストは
/// `source_registered_on` で登録日を明示する。
const FAR_PAST: &str = "2000-01-01";

/// 登録日を明示して登録簿へ置く（第 8 回 Q29 の閾値を見るテスト用）。
pub async fn source_registered_on(
    pool: &sqlx::PgPool,
    prefix: &str,
    expected_gap_sec: i32,
    registered_on: &str,
) -> String {
    let name = format!("t-{prefix}-{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO core.source
           (logical_source, display_name, expected_gap_sec, registered_at, external_id_kind)
         VALUES ($1, $1, $2, ($3::date::timestamp AT TIME ZONE 'Asia/Tokyo'), 'none')",
    )
    .bind(&name)
    .bind(expected_gap_sec)
    .bind(registered_on)
    .execute(pool)
    .await
    .unwrap();
    name
}

/// メディアのソース（収集開始日を生存信号だけで決める。ST09 / design D13）を登録簿へ置く。
/// 印は本物の 4 つと同じ `heartbeat`。
pub async fn source_media(pool: &sqlx::PgPool, prefix: &str) -> String {
    let name = source(pool, prefix, 21_600).await;
    sqlx::query("UPDATE core.source SET started_on_basis = 'heartbeat' WHERE logical_source = $1")
        .bind(&name)
        .execute(pool)
        .await
        .unwrap();
    name
}

/// 退役した日を置く（FR-54 / FR-61。ST03 の R56 —— **真偽値ではなく日付**）。
pub async fn retire(pool: &sqlx::PgPool, source: &str, day: &str) {
    sqlx::query("UPDATE core.source SET retired_on = $2::date WHERE logical_source = $1")
        .bind(source)
        .bind(day)
        .execute(pool)
        .await
        .unwrap();
}

/// 引き継ぎ元を置く（第 8 回 Q31）。`source` が `predecessor` を引き継ぐ。
pub async fn set_succeeds(pool: &sqlx::PgPool, source: &str, predecessor: &str) {
    sqlx::query("UPDATE core.source SET succeeds = $2 WHERE logical_source = $1")
        .bind(source)
        .bind(predecessor)
        .execute(pool)
        .await
        .unwrap();
}

/// テストごとに他のテストが使わない日（2000 年代の前半から）。書き出しは除く判定が**利用者を問わない**ので、
/// 消した滞在の時間は、回の database の中で書き出しの試験以外が共有する。回の database を使う試験は、この日で互いを隔てる
/// （ST33 / spec-review R27）。書き出しの試験も記録をこの日に置くが、試験ごとの database（`isolated()`）で既に隔離されている（CH04）。
/// 1 回の実行の中では重ならず、実行どうしは乱数の起点でずらす。
pub fn unique_day() -> chrono::NaiveDate {
    use std::sync::atomic::{AtomicU32, Ordering};
    // 2000-01-01 から 5 年ぶん（1827 日）。37 は 1827 と互いに素なので、起点からの 1827 回は重ならない。
    const SPAN: u32 = 1827;
    static NEXT: std::sync::OnceLock<AtomicU32> = std::sync::OnceLock::new();
    let next = NEXT.get_or_init(|| AtomicU32::new(uuid::Uuid::new_v4().as_u128() as u32 % SPAN));
    let offset = next.fetch_add(37, Ordering::Relaxed) % SPAN;
    chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap() + chrono::Duration::days(i64::from(offset))
}

/// 取り込み器の試験が見回りの間隔を読む、自分専用の登録簿の行（間隔 1 秒）。
/// 共有の `s01-archive-inbox` を書き換えない（登録簿は利用者で分かれていない。ST13 / design D4）。
pub async fn archive_scan_row(pool: &sqlx::PgPool) -> &'static str {
    const NAME: &str = "test-archive-scan-1s";
    sqlx::query(
        "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind,
                                  acquisition, fetch_trigger, fetch_interval_sec)
         VALUES ($1, $1, 3600, 'none', 'server', 'interval', 1) ON CONFLICT DO NOTHING",
    )
    .bind(NAME)
    .execute(pool)
    .await
    .unwrap();
    NAME
}

/// 指定した書庫（`sha256`）の `archive_ledger_source` への `INSERT` で例外を投げる trigger を作る。
/// 「台帳の論理ソースごとの行を書く途中で D-01 への書き込みが失敗する」を試験の DB にだけ起こす
/// （本番のコードに試験用の分岐を置かない。ST13 / design D7）。外すのは `allow_ledger_source_insert`。
pub async fn fail_ledger_source_insert(pool: &sqlx::PgPool, sha256: &str) {
    assert!(sha256.chars().all(|c| c.is_ascii_hexdigit()));
    sqlx::query(&format!(
        // 試験は並走するので、関数も書庫ごとの名前にする（同じ名前の同時の作成は pg_proc の鍵で落ちる）。
        // **冪等にする**（final review R5）: 前の回が `allow_ledger_source_insert` の前に落ちて
        // 関数と trigger が残っていても、次の回の作成が落ちない。
        "CREATE OR REPLACE FUNCTION core.test_fail_ledger_source_{sha256}() RETURNS trigger AS $f$
         BEGIN
           IF EXISTS (SELECT 1 FROM core.archive_ledger WHERE id = NEW.ledger_id AND sha256 = '{sha256}') THEN
             RAISE EXCEPTION 'test: archive_ledger_source への書き込みを失敗させる';
           END IF;
           RETURN NEW;
         END $f$ LANGUAGE plpgsql"
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "DROP TRIGGER IF EXISTS test_fail_ledger_source_{sha256} ON core.archive_ledger_source"
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "CREATE TRIGGER test_fail_ledger_source_{sha256} BEFORE INSERT ON core.archive_ledger_source
         FOR EACH ROW EXECUTE FUNCTION core.test_fail_ledger_source_{sha256}()"
    ))
    .execute(pool)
    .await
    .unwrap();
}

/// `fail_ledger_source_insert` で作った trigger を外す。
pub async fn allow_ledger_source_insert(pool: &sqlx::PgPool, sha256: &str) {
    assert!(sha256.chars().all(|c| c.is_ascii_hexdigit()));
    sqlx::query(&format!(
        "DROP TRIGGER IF EXISTS test_fail_ledger_source_{sha256} ON core.archive_ledger_source"
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "DROP FUNCTION IF EXISTS core.test_fail_ledger_source_{sha256}()"
    ))
    .execute(pool)
    .await
    .unwrap();
}

/// テストごとに固有の利用者。稼働記録・生存信号は利用者でも分かれる（FR-29）。
pub fn user() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

/// 稼働記録を直に置く（取り込み口を通さずに状態の導出だけを見たいとき）。
pub async fn put_coverage(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    day: &str,
    count: i32,
) {
    for _ in 0..count {
        put_event(pool, user, source, &format!("{day}T12:00:00+09:00")).await;
    }
}

/// 記録を直に置く（取り込み口を通さずに状態の導出だけを見たいとき）。
/// `at` は RFC3339。**その時刻が属する日**（`Asia/Tokyo`）に記録があることになる。
///
/// **稼働記録（`core.coverage`）の行は置かない**（review/code-r2.md の H-7）。
/// 両方を常に同値で書いていたときは、状態の出どころを `core.event` から
/// `core.coverage.event_count` へ**戻しても検査が全部緑のまま通った** ——
/// R57 の回帰を止める役に立っていなかった。稼働記録の行が要る検査は
/// `put_coverage_row` で明示的に置く（本番では 2 つが食い違いうる）。
pub async fn put_event(pool: &sqlx::PgPool, user: uuid::Uuid, source: &str, at: &str) {
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test','collected',$4::timestamptz,540,'Asia/Tokyo',1,$5,'{}','{}')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(source)
    .bind(at)
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .unwrap();
}

/// 論理削除された記録を置く（FR-50）。**稼働記録はこれを書き換えない**
/// （design D34。丸ごと消した期間は⑤「破棄された期間」が担う）。
pub async fn put_deleted_event(pool: &sqlx::PgPool, user: uuid::Uuid, source: &str, at: &str) {
    put_event(pool, user, source, at).await;
    sqlx::query(
        "UPDATE core.event SET deleted_at = now(), deleted_by = 'test'
          WHERE user_id = $1 AND logical_source = $2
            AND event_time = $3::timestamptz AND deleted_at IS NULL",
    )
    .bind(user)
    .bind(source)
    .bind(at)
    .execute(pool)
    .await
    .unwrap();
}

/// 稼働記録の行だけを置く（**記録は置かない**）。
/// 正典の「稼働記録は新しく入った記録だけを数える」側を見る検査のため。
pub async fn put_coverage_row(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    day: &str,
    count: i32,
) {
    sqlx::query(
        "INSERT INTO core.coverage (user_id, logical_source, day, event_count)
         VALUES ($1,$2,$3::date,$4)
         ON CONFLICT (user_id, logical_source, day)
         DO UPDATE SET event_count = core.coverage.event_count + $4",
    )
    .bind(user)
    .bind(source)
    .bind(day)
    .bind(count)
    .execute(pool)
    .await
    .unwrap();
}

/// 生存信号を直に置く。`emitted` は RFC3339。
pub async fn put_heartbeat(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    emitted: &str,
    capturable: bool,
) {
    put_heartbeat_counts(pool, user, source, emitted, capturable, 1, 1).await;
}

/// 取得の試行回数と成功回数まで指定して生存信号を置く。
pub async fn put_heartbeat_counts(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    emitted: &str,
    capturable: bool,
    attempts: i32,
    successes: i32,
) {
    let blockers: Vec<String> = if capturable {
        vec![]
    } else {
        vec!["permission".into()]
    };
    sqlx::query(
        "INSERT INTO core.heartbeat
           (id, user_id, logical_source, device_id, emitted_at, capturable, blockers,
            attempts, successes, content_hash, raw)
         VALUES ($1,$2,$3,'test',$4::timestamptz,$5,$6,$7,$8,$9,'{}')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(source)
    .bind(emitted)
    .bind(capturable)
    .bind(&blockers)
    .bind(attempts)
    .bind(successes)
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .unwrap();
}

/// 停止・破棄の範囲を置く。`kind` は `stopped` か `dropped`。
pub async fn put_span(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    kind: &str,
    started: &str,
    ended: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO core.coverage_span
           (id, user_id, logical_source, kind, started_at, ended_at)
         VALUES ($1,$2,$3,$4,$5::timestamptz,$6::timestamptz)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(source)
    .bind(kind)
    .bind(started)
    .bind(ended)
    .execute(pool)
    .await
    .unwrap();
}

/// 収集開始日を直に置く。
pub async fn set_started_on(pool: &sqlx::PgPool, source: &str, day: &str) {
    sqlx::query(
        "UPDATE core.source SET collection_started_on = $2::date WHERE logical_source = $1",
    )
    .bind(source)
    .bind(day)
    .execute(pool)
    .await
    .unwrap();
}

/// 日付リテラル。テストの読みやすさのためだけに置く。
pub fn date(s: &str) -> chrono::NaiveDate {
    s.parse().unwrap()
}

/// 破棄の報告を直に置く（ST04）。`range` は RFC3339 の (始まり, 終わり)、`hours` は (UTC の正時, 件数)。
/// **口を通す試験は `drops_tests` にある。** ここは稼働状況の読み手の試験の材料。
pub async fn put_drop(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    range: Option<(&str, &str)>,
    hours: &[(&str, i32)],
) {
    let id = uuid::Uuid::new_v4();
    let count: i32 = hours.iter().map(|h| h.1).sum::<i32>().max(1);
    sqlx::query(
        "INSERT INTO core.drop_report
           (id, user_id, logical_source, device_id, reason, range_start, range_end,
            count, created_at, content_hash, raw)
         VALUES ($1,$2,$3,'test',$4,$5::timestamptz,$6::timestamptz,$7,now(),$8,'{}')",
    )
    .bind(id)
    .bind(user)
    .bind(source)
    .bind(if range.is_some() { "age" } else { "unreadable" })
    .bind(range.map(|r| r.0))
    .bind(range.map(|r| r.1))
    .bind(count)
    .bind(id.to_string())
    .execute(pool)
    .await
    .unwrap();
    for (hour, n) in hours {
        sqlx::query(
            "INSERT INTO core.drop_report_hour (report_id, hour, count) VALUES ($1,$2::timestamptz,$3)",
        )
        .bind(id)
        .bind(hour)
        .bind(n)
        .execute(pool)
        .await
        .unwrap();
    }
}
