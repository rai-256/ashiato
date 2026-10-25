// SPDX-License-Identifier: AGPL-3.0-only
//! サーバの起動を、**実行ファイルを起動して**確かめる（ST28 / design D4）。
//! 管理者・所有者の接続ではサーバは起動せず、終了コード 2 と理由の種別（値ではなく種別）で終わる。
#![allow(clippy::unwrap_used)]

use std::process::{Command, Output};

fn env(name: &str) -> String {
    // 接続先の URL は port だけ、この worktree の DB のものにする（`.env` の port は main の DB）
    if matches!(name, "DATABASE_URL" | "DATABASE_OWNER_URL") {
        return with_worktree_port(&raw_env(name));
    }
    raw_env(name)
}

fn raw_env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} が無い。.env を読み込む（set -a; . ./.env; set +a）"))
}

/// URL の `@<host>:<port>/` の port を、この worktree の DB のものに差し替える。
/// port は `tools/ports.sh`（規則の唯一の持ち主。`testdb.rs` も同じく呼ぶ）から得る。
fn with_worktree_port(url: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new("bash")
        .args(["-c", r#". tools/ports.sh && printf %s "$ASHIATO_DB_PORT""#])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "tools/ports.sh が port を決められない: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let port: u16 = String::from_utf8_lossy(&out.stdout).parse().unwrap();
    let at = url.rfind('@').unwrap();
    let rest = &url[at + 1..];
    let slash = rest.find('/').unwrap();
    let host = rest[..slash]
        .rsplit_once(':')
        .map_or(&rest[..slash], |(h, _)| h);
    format!("{}@{host}:{port}{}", &url[..at], &rest[slash..])
}

/// 所有者の URL の「ユーザ:合言葉」だけを差し替えた URL（接続先の host / port / DB は同じ）。
fn url_as(user: &str, password: &str) -> String {
    let owner = env("DATABASE_OWNER_URL");
    let (scheme, rest) = owner.split_once("://").unwrap();
    let (_, host) = rest.split_once('@').unwrap();
    format!("{scheme}://{user}:{password}@{host}")
}

/// 与えた接続先でサーバを起動する。**拒まれなければ待ち続けるので、拒まれない場合に備えて 20 秒で殺す。**
fn start_with(database_url: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ashiato-server"))
        .env_clear()
        .env("DATABASE_URL", database_url)
        .env("API_TOKEN", "test-token-0123456789abcdef")
        .env("WEB_PASSWORD", TEST_WEB_PASSWORD)
        .env("BIND", "127.0.0.1:0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() > deadline {
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    child.wait_with_output().unwrap()
}

fn assert_refused(out: &Output, reason: &str) {
    assert_eq!(out.status.code(), Some(2), "終了コードが 2 でない");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("kind=db_role") && stderr.contains(&format!("reason={reason}")),
        "標準エラーに種別が無い: {stderr}"
    );
    // 合言葉と接続先は出さない
    let all = format!("{stderr}{}", String::from_utf8_lossy(&out.stdout));
    for name in ["POSTGRES_PASSWORD", "OWNER_DB_PASSWORD"] {
        let value = env(name);
        assert!(!all.contains(&value), "{name} の値が出ている");
    }
}

/// Scenario: 管理者の接続ではサーバが起動しない
/// Scenario: 所有者の接続ではサーバが起動しない
#[test]
fn server_startup_refuses_privileged_role() {
    // 表が無いと所有者の拒否を見られないので、先に移行を当てる（本物と同じ手順）
    migrate_as_owner();

    let admin = url_as(
        &std::env::var("POSTGRES_USER").unwrap_or_else(|_| "ashiato".into()),
        &env("POSTGRES_PASSWORD"),
    );
    assert_refused(&start_with(&admin), "superuser");
    assert_refused(&start_with(&env("DATABASE_OWNER_URL")), "table_owner");

    // 所有者の役割を付けられた役割も所有者と同じに扱う（code-verify R16）。
    // 本物の ashiato_app に付けると並走する試験が拒まれるので、使い捨ての役割で見る
    let member = format!("ashiato_member_{}", std::process::id());
    let pw = format!("pw-{}", uuid::Uuid::new_v4().simple());
    let as_admin = |sql: String| {
        let admin = admin.clone();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let pool = sqlx::PgPool::connect(&admin).await.unwrap();
                sqlx::raw_sql(&sql).execute(&pool).await.unwrap();
            });
    };
    as_admin(format!(
        "DROP ROLE IF EXISTS {member}; CREATE ROLE {member} LOGIN PASSWORD '{pw}';
         GRANT ashiato_owner TO {member};"
    ));
    let out = start_with(&url_as(&member, &pw));
    as_admin(format!("DROP ROLE {member}"));
    assert_refused(&out, "owner_member");
}

const TEST_TOKEN: &str = "test-token-0123456789abcdef";
const TEST_WEB_PASSWORD: &str = "test-web-password-0123456789";

fn server_cmd() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_ashiato-server"));
    c.env_clear()
        .env("API_TOKEN", TEST_TOKEN)
        .env("WEB_PASSWORD", TEST_WEB_PASSWORD)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    c
}

/// 終わるまで待つ（20 秒で殺す）。
fn wait_exit(mut child: std::process::Child) -> Output {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() > deadline {
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    child.wait_with_output().unwrap()
}

fn assert_kind_refused(out: &Output, kind: &str) {
    assert_eq!(out.status.code(), Some(2), "終了コードが 2 でない");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&format!("kind={kind}")),
        "標準エラーに種別 {kind} が無い: {stderr}"
    );
}

/// 移行を当てる。**この binary の試験は並列に走るので、移行は 1 本ずつ**（同じ DB に同時に当てると落ちる）。
fn migrate_as_owner() {
    static MIGRATING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = MIGRATING.lock().unwrap_or_else(|e| e.into_inner());
    let m = Command::new(env!("CARGO_BIN_EXE_ashiato-server"))
        .arg("migrate")
        .env_clear()
        .env("DATABASE_OWNER_URL", env("DATABASE_OWNER_URL"))
        .env("MIGRATE_BACKUP", "off") // 試験の DB は使い捨て（移行の前の退避は migrate_backup の単体試験が見る）
        .output()
        .unwrap();
    assert!(m.status.success(), "migrate が失敗した");
}

/// `ss -ltnpH` の行のうち、この pid の口の待ち受けアドレス。
fn listening_addrs(pid: u32) -> Vec<String> {
    let out = Command::new("ss").args(["-ltnpH"]).output().unwrap();
    let needle = format!("pid={pid},");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.contains(&needle))
        .map(|l| l.split_whitespace().nth(3).unwrap().to_string())
        .collect()
}

fn is_loopback_local(addr: &str) -> bool {
    let (host, _) = addr.rsplit_once(':').unwrap();
    host.trim_matches(['[', ']'])
        .parse::<std::net::IpAddr>()
        .unwrap()
        .is_loopback()
}

/// Scenario: 既定では loopback でだけ待ち受ける
#[test]
fn server_startup_bind_default_is_loopback_only() {
    let default: std::net::SocketAddr = ashiato_server::net_guard::DEFAULT_BIND.parse().unwrap();
    assert!(
        default.ip().is_loopback(),
        "既定の待ち受けが loopback でない"
    );
    // 既定の port は固定なので、並べた worktree のサーバが使っていれば起動は確かめられない（code-verify R18）
    if std::net::TcpListener::bind(default).is_err() {
        eprintln!(
            "skip: {default} が使用中なので、既定での起動は確かめない（既定の値は上で確かめた）"
        );
        return;
    }
    migrate_as_owner();
    let mut child = server_cmd()
        .env("DATABASE_URL", env("DATABASE_URL"))
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut addrs = Vec::new();
    while std::time::Instant::now() < deadline {
        addrs = listening_addrs(child.id());
        if !addrs.is_empty() || child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let exited = child.try_wait().unwrap();
    child.kill().ok();
    let out = child.wait_with_output().unwrap();
    assert!(
        exited.is_none(),
        "サーバが待ち受ける前に終わった: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!addrs.is_empty(), "待ち受けの口が見えない");
    assert!(
        addrs.iter().all(|a| is_loopback_local(a)),
        "loopback 以外の口が開いている: {addrs:?}"
    );
}

/// Scenario: loopback 以外のアドレスでは起動しない
#[test]
fn server_startup_bind_refuses_non_loopback() {
    // DB は繋がない先（loopback の閉じた口）にして、判定が DB より前であることも見る
    let child = server_cmd()
        .env("DATABASE_URL", "postgres://x@127.0.0.1:1/ashiato")
        .env("BIND", "192.0.2.1:0")
        .spawn()
        .unwrap();
    assert_kind_refused(&wait_exit(child), "bind_not_loopback");
}

/// Scenario: 全インタフェースでは起動しない
#[test]
fn server_startup_bind_refuses_unspecified() {
    let child = server_cmd()
        .env("DATABASE_URL", "postgres://x@127.0.0.1:1/ashiato")
        .env("BIND", "0.0.0.0:0")
        .spawn()
        .unwrap();
    assert_kind_refused(&wait_exit(child), "bind_unspecified");
}

/// Scenario: DB の接続先が loopback でなければ起動しない
#[test]
fn server_startup_db_not_loopback_refuses_server_and_migrate() {
    let url = "postgres://x@192.0.2.1:5432/ashiato";
    let server = server_cmd()
        .env("DATABASE_URL", url)
        .env("BIND", "127.0.0.1:0")
        .spawn()
        .unwrap();
    let out = wait_exit(server);
    assert_kind_refused(&out, "db_not_loopback");

    let migrate = Command::new(env!("CARGO_BIN_EXE_ashiato-server"))
        .arg("migrate")
        .env_clear()
        .env("DATABASE_OWNER_URL", url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mout = wait_exit(migrate);
    assert_kind_refused(&mout, "db_not_loopback");

    // 接続を試みていない（接続の失敗・タイムアウトの痕跡が無い）。接続先の値も出ていない
    for o in [&out, &mout] {
        let all = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stderr),
            String::from_utf8_lossy(&o.stdout)
        );
        assert!(!all.contains("192.0.2.1"), "接続先が出ている");
        assert!(
            !all.contains("Connection") && !all.contains("timed out") && !all.contains("Io"),
            "接続を試みた痕跡がある: {all}"
        );
    }
}

/// 画面の合言葉の検査は DB より前。DB は繋がない先にして、判定が DB に触れないことも見る。
fn start_with_web_password(web_password: Option<&str>, api_token: &str) -> Output {
    let mut c = server_cmd();
    c.env("API_TOKEN", api_token)
        .env("DATABASE_URL", "postgres://x@127.0.0.1:1/ashiato")
        // 検査を通った場合は次の段（待ち受けの検査）で止める。DB には繋がない
        .env("BIND", "0.0.0.0:0");
    match web_password {
        Some(p) => c.env("WEB_PASSWORD", p),
        None => c.env_remove("WEB_PASSWORD"),
    };
    wait_exit(c.spawn().unwrap())
}

fn assert_web_password_refused(out: &Output, reason: &str, secrets: &[&str]) {
    assert_kind_refused(out, "web_password");
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        all.contains(&format!("reason={reason}")),
        "理由が無い: {all}"
    );
    for secret in secrets {
        assert!(!all.contains(secret), "合言葉の値が出ている");
    }
}

/// Scenario: 画面の合言葉が API の合言葉と同じだと起動しない
#[test]
fn server_startup_web_password_same_as_api_token_refuses() {
    let out = start_with_web_password(Some(TEST_TOKEN), TEST_TOKEN);
    assert_web_password_refused(&out, "same_as_api_token", &[TEST_TOKEN]);
}

/// Scenario: 画面の合言葉が無いか短いと起動しない
#[test]
fn server_startup_web_password_missing_or_short_refuses() {
    let out = start_with_web_password(None, TEST_TOKEN);
    assert_web_password_refused(&out, "missing", &[]);
    let fifteen = "a1b2c3d4e5f6g7h";
    assert_eq!(fifteen.len(), 15);
    let out = start_with_web_password(Some(fifteen), TEST_TOKEN);
    assert_web_password_refused(&out, "too_short", &[fifteen]);
    // 雛形（.env.example）の値のままは、長さが足りていても通さない（D16（仮））
    let placeholder = "change-me-web-password";
    assert!(placeholder.len() >= 16);
    let out = start_with_web_password(Some(placeholder), TEST_TOKEN);
    assert_web_password_refused(&out, "placeholder", &[placeholder]);
    // 16 文字ちょうどは検査を通り、次の段（待ち受け）で止まる
    let out = start_with_web_password(Some("a1b2c3d4e5f6g7h8"), TEST_TOKEN);
    assert_kind_refused(&out, "bind_unspecified");
}

/// 起動したサーバの標準出力（ログ）を、`done` が真になる行が出るまで読む（20 秒で諦める）。読み終えたらサーバを止める。
fn log_until(mut cmd: Command, done: impl Fn(&str) -> bool) -> String {
    use std::io::BufRead as _;
    let mut child = cmd.spawn().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
        {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut seen = String::new();
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        let Ok(line) = rx.recv_timeout(left) else {
            break;
        };
        seen.push_str(&line);
        seen.push('\n');
        if done(&line) {
            break;
        }
    }
    child.kill().ok();
    child.wait().ok();
    seen
}

/// 取り込み器の置き場を、この試験だけの一時ディレクトリに置く（本人のホームを触らない）。
fn archive_env(cmd: &mut Command, user: uuid::Uuid) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("ashiato-startup-{}", uuid::Uuid::new_v4().simple()));
    for d in ["inbox", "downloads", "copies"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    cmd.env("ASHIATO_ARCHIVE_USER_ID", user.to_string())
        .env("ASHIATO_INBOX_DIR", root.join("inbox"))
        .env("ASHIATO_DOWNLOADS_DIR", root.join("downloads"))
        .env("ASHIATO_ARCHIVE_COPY_DIR", root.join("copies"));
    root
}

/// S-01 の起動が定期取得の枠を起こすこと（code_verify R19）。枠の試験（`scheduled_tests.rs`）は `spawn_with` を
/// 直接呼ぶので、起動の配線を外しても落ちない。ここは実行ファイルを起動し、自分専用の定期取得の行
/// （取る手は無い）の名前が `scheduled_no_fetcher` でログに出ることを見る。
// Scenario: 取る手の無い定期取得のソースは取りに行かない
#[test]
fn server_startup_starts_the_scheduled_fetch() {
    migrate_as_owner();
    let name = format!("test-sched-startup-{}", uuid::Uuid::new_v4().simple());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let owner = rt
        .block_on(sqlx::PgPool::connect(&env("DATABASE_OWNER_URL")))
        .unwrap();
    rt.block_on(
        sqlx::query(
            "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind,
                                      acquisition, fetch_trigger, fetch_interval_sec)
             VALUES ($1, $1, 3600, 'none', 'scheduled', 'interval', 3600)",
        )
        .bind(&name)
        .execute(&owner),
    )
    .unwrap();
    let mut cmd = server_cmd();
    cmd.env("DATABASE_URL", env("DATABASE_URL"))
        .env("BIND", "127.0.0.1:0");
    let root = archive_env(&mut cmd, uuid::Uuid::new_v4());
    let log = log_until(cmd, |l| {
        l.contains("scheduled_no_fetcher") && l.contains(&name)
    });
    rt.block_on(
        sqlx::query("DELETE FROM core.source WHERE logical_source = $1")
            .bind(&name)
            .execute(&owner),
    )
    .unwrap();
    std::fs::remove_dir_all(root).ok();
    assert!(
        log.lines()
            .any(|l| l.contains("scheduled_no_fetcher") && l.contains(&name)),
        "起動した S-01 が定期取得の行を読んでいない: {log}"
    );
}

/// 書庫の利用者が未設定なら定期取得の枠も起こさず、そのことがログに出る（/code-review R30）。
#[test]
fn server_startup_without_archive_user_says_scheduled_fetch_is_off() {
    migrate_as_owner();
    let mut cmd = server_cmd();
    cmd.env("DATABASE_URL", env("DATABASE_URL"))
        .env("BIND", "127.0.0.1:0");
    let log = log_until(cmd, |l| l.contains("scheduled_disabled"));
    assert!(
        log.contains("scheduled_disabled"),
        "定期取得を起こさないことがログに出ない: {log}"
    );
}

// ---- ST48: `migrate` の順と止め方（design D4・D6）。使い捨ての database に実行ファイルを走らせる ----

mod st48_migrate {
    use super::*;
    use ashiato_server::migration_ledger::{BUILD_COMMIT, BUILD_VERSION};
    use std::path::{Path, PathBuf};

    /// URL の database 名（最後の `/` の後ろ。`?` 以降は残す）を差し替える。
    fn with_database(url: &str, database: &str) -> String {
        let at = url.rfind('@').unwrap();
        let slash = at + url[at..].find('/').unwrap();
        let tail = url[slash..].find('?').map_or("", |i| &url[slash + i..]);
        format!("{}/{database}{tail}", &url[..slash])
    }

    /// 使い捨ての database（design D16）。版の記録に知らない行を書くので、worktree で共有する DB は使わない。
    struct TempDb {
        name: String,
        pool: sqlx::PgPool,
    }

    impl TempDb {
        async fn create() -> Self {
            let name = format!("st48_{:012}", rand::random::<u64>() % 1_000_000_000_000);
            let admin = sqlx::PgPool::connect(&env("DATABASE_OWNER_URL"))
                .await
                .unwrap();
            // db-ok: 使い捨ての database。Drop で DROP DATABASE する（design D16）
            sqlx::raw_sql(&format!("CREATE DATABASE {name}"))
                .execute(&admin)
                .await
                .unwrap();
            admin.close().await;
            let pool = sqlx::PgPool::connect(&with_database(&env("DATABASE_OWNER_URL"), &name))
                .await
                .unwrap();
            Self { name, pool }
        }

        fn owner(&self) -> &sqlx::PgPool {
            &self.pool
        }

        fn owner_url(&self) -> String {
            with_database(&env("DATABASE_OWNER_URL"), &self.name)
        }

        async fn app(&self) -> sqlx::PgPool {
            sqlx::PgPool::connect(&with_database(&env("DATABASE_URL"), &self.name))
                .await
                .unwrap()
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let (name, url) = (self.name.clone(), env("DATABASE_OWNER_URL"));
            // Drop は同期。別スレッドの小さな runtime で消す（試験が落ちても残さない）
            let _ = std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    if let Ok(admin) = sqlx::PgPool::connect(&url).await {
                        // db-ok: 自分が作った使い捨ての database を消す（design D16）
                        let _ =
                            sqlx::raw_sql(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
                                .execute(&admin)
                                .await;
                    }
                });
            })
            .join();
        }
    }

    /// 使い捨ての database と、試験ごとの退避の置き場・偽の `pg_dump`（Drop で全部消える）。
    struct Rig {
        db: TempDb,
        dir: tempfile::TempDir,
    }

    impl Rig {
        async fn new() -> Self {
            Self {
                db: TempDb::create().await,
                dir: tempfile::tempdir().unwrap(),
            }
        }

        fn backups(&self) -> PathBuf {
            self.dir.path().join("backups")
        }

        /// 偽の `pg_dump`。呼ばれたら `--file` の指す先に書く。`ok` が偽なら何も書かずに失敗する。
        fn fake_pg_dump(&self, name: &str, ok: bool) -> PathBuf {
            use std::os::unix::fs::PermissionsExt as _;
            let path = self.dir.path().join(name);
            let body = if ok {
                "#!/bin/sh\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --file ]; then echo fake-dump > \"$2\"; fi\n  shift\ndone\n"
            } else {
                "#!/bin/sh\nexit 1\n"
            };
            std::fs::write(&path, body).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }

        /// 既定は成功する偽の `pg_dump` と、この試験の置き場で走らせる。
        fn migrate(&self) -> Output {
            let pg_dump = self.fake_pg_dump("pg_dump_ok", true);
            self.migrate_with(&self.backups(), &pg_dump, None)
        }

        fn migrate_with(&self, dir: &Path, pg_dump: &Path, backup: Option<&str>) -> Output {
            let mut c = Command::new(env!("CARGO_BIN_EXE_ashiato-server"));
            c.arg("migrate")
                .env_clear()
                .env("DATABASE_OWNER_URL", self.db.owner_url())
                .env("MIGRATE_BACKUP_DIR", dir)
                .env("PG_DUMP", pg_dump);
            if let Some(b) = backup {
                c.env("MIGRATE_BACKUP", b);
            }
            c.output().unwrap()
        }

        fn dumps_in(dir: &Path) -> usize {
            std::fs::read_dir(dir).map_or(0, |d| {
                d.filter_map(Result::ok)
                    .filter(|e| e.file_name().to_string_lossy().starts_with("pre-migrate-"))
                    .count()
            })
        }

        fn dumps(&self) -> usize {
            Self::dumps_in(&self.backups())
        }

        /// 行の数・名前・`applied_at` と、門の関数の定義。
        async fn snapshot(&self) -> (Vec<(String, String)>, String) {
            let rows: Vec<(String, String)> = sqlx::query_as(
                "SELECT name, applied_at::text FROM migration_ledger.applied ORDER BY name",
            )
            .fetch_all(self.db.owner())
            .await
            .unwrap();
            let def: String = sqlx::query_scalar(
                "SELECT pg_get_functiondef('core.reject_collected_rewrite'::regproc)",
            )
            .fetch_one(self.db.owner())
            .await
            .unwrap();
            (rows, def)
        }

        async fn exec(&self, sql: &str) {
            sqlx::raw_sql(sql).execute(self.db.owner()).await.unwrap();
        }

        /// 新しい版で当てた DB の印（知らない版の行 1 つ）。
        async fn add_future_row(&self) {
            self.exec(
                "INSERT INTO migration_ledger.applied (name, sha256, build_version, build_commit)
                 VALUES ('209912312359_from_the_future', 'abc', '0.9.9', 'deadbeef')",
            )
            .await;
        }
    }

    fn assert_ok(out: &Output) {
        assert!(
            out.status.success(),
            "migrate が失敗した: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn stderr(out: &Output) -> String {
        String::from_utf8_lossy(&out.stderr).into_owned()
    }

    fn after_a_second() {
        // 退避の名前は秒までの時刻。2 回目が 1 回目のファイルに重ならないようにする
        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    // Scenario: 同じ版で当て直しても、何も変わらずに通る
    #[tokio::test]
    async fn st48_migrate_rerun_changes_nothing() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        let first = rig.snapshot().await;
        assert!(!first.0.is_empty());
        after_a_second();
        assert_ok(&rig.migrate());
        assert_eq!(rig.snapshot().await, first);
    }

    // Scenario: 指紋の違う版は当て直さず、名前を示して続ける
    #[tokio::test]
    async fn st48_migrate_fingerprint_mismatch_warns() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        let name = ashiato_server::MIGRATIONS.last().unwrap().0;
        rig.exec(&format!(
            "UPDATE migration_ledger.applied SET sha256 = 'changed' WHERE name = '{name}'"
        ))
        .await;
        let before = rig.snapshot().await;
        after_a_second();
        let out = rig.migrate();
        assert_ok(&out);
        let all = format!("{}{}", String::from_utf8_lossy(&out.stdout), stderr(&out));
        assert!(all.contains(name), "版の名前が出ていない: {all}");
        assert_eq!(rig.snapshot().await, before, "当て直している");
        let sha: String =
            sqlx::query_scalar("SELECT sha256 FROM migration_ledger.applied WHERE name = $1")
                .bind(name)
                .fetch_one(rig.db.owner())
                .await
                .unwrap();
        assert_eq!(sha, "changed");
    }

    // Scenario: 当てる版が無い回にも、当てる前の退避を取る
    #[tokio::test]
    async fn st48_migrate_backs_up_even_with_nothing_to_apply() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        assert_eq!(rig.dumps(), 1);
        after_a_second();
        assert_ok(&rig.migrate()); // 2 回目は当てる版が無い
        assert_eq!(rig.dumps(), 2, "当てる版が無い回に退避を取っていない");
    }

    // Scenario: 新しい版で当てた DB に古い版の migrate を走らせると、1 つも当てずに止まる
    #[tokio::test]
    async fn st48_migrate_refuses_db_ahead() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        rig.add_future_row().await;
        let before = rig.snapshot().await;
        after_a_second();
        let out = rig.migrate();
        assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
        let err = stderr(&out);
        let lines: Vec<&str> = err.lines().collect();
        assert_eq!(lines[0], "error: kind=db_ahead", "{err}");
        assert!(
            lines.contains(&"  db: 209912312359_from_the_future (0.9.9 deadbeef)"),
            "{err}"
        );
        assert!(
            lines.contains(&format!("  self: {BUILD_VERSION} {BUILD_COMMIT}").as_str()),
            "{err}"
        );
        assert_eq!(rig.snapshot().await, before, "行か門が変わっている");
    }

    // Scenario: 止まるときは退避を取らない
    #[tokio::test]
    async fn st48_migrate_refusal_takes_no_backup() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        let taken = rig.dumps();
        rig.add_future_row().await;
        after_a_second();
        let out = rig.migrate();
        assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
        assert_eq!(rig.dumps(), taken, "止まる回に退避を取っている");
    }

    // Scenario: 版の記録が読めない DB では、何も書かずに止まる
    #[tokio::test]
    async fn st48_migrate_refuses_unreadable_ledger() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        rig.exec("UPDATE migration_ledger.form SET form = 2").await;
        let before = rig.snapshot().await;
        let taken = rig.dumps();
        after_a_second();
        let out = rig.migrate();
        assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
        let err = stderr(&out);
        assert_eq!(err.lines().next(), Some("error: kind=ledger_unreadable"));
        assert_eq!(rig.snapshot().await, before);
        assert_eq!(rig.dumps(), taken, "退避を取っている");
        let form: i32 = sqlx::query_scalar("SELECT form FROM migration_ledger.form")
            .fetch_one(rig.db.owner())
            .await
            .unwrap();
        assert_eq!(form, 2);
    }

    // Scenario: 退避に失敗すると、終了コード 1 で何も当てない
    #[tokio::test]
    async fn st48_migrate_backup_failure_applies_nothing() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        let name = ashiato_server::MIGRATIONS.last().unwrap().0;
        rig.exec(&format!(
            "DELETE FROM migration_ledger.applied WHERE name = '{name}'"
        ))
        .await;
        let failing = rig.fake_pg_dump("pg_dump_fails", false);
        let out = rig.migrate_with(&rig.backups(), &failing, None);
        assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
        let back: i64 =
            sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied WHERE name = $1")
                .bind(name)
                .fetch_one(rig.db.owner())
                .await
                .unwrap();
        assert_eq!(back, 0, "退避に失敗したのに版が当たっている");
    }

    // Scenario: 版の中身が当たらないと、終了コード 1 で終わる
    #[tokio::test]
    async fn st48_migrate_content_failure_exits_1() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        // 書き出しの台帳の版が作る表を、同じ名前で形の違う表に置き換え、その版の行を消す
        rig.exec(
            "DROP TABLE core.export_ledger;
             CREATE TABLE core.export_ledger (x integer);
             DELETE FROM migration_ledger.applied WHERE name = '202610131200_export_ledger'",
        )
        .await;
        after_a_second();
        let out = rig.migrate();
        assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
        let recorded: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM migration_ledger.applied WHERE name = '202610131200_export_ledger'",
        )
        .fetch_one(rig.db.owner())
        .await
        .unwrap();
        assert_eq!(recorded, 0, "当たらなかった版が記録されている");
    }

    // Scenario: 当てる版が無い回にも、アプリの役割への付与を当てる
    #[tokio::test]
    async fn st48_migrate_regrants_with_nothing_to_apply() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        rig.exec("REVOKE SELECT ON core.event FROM ashiato_app")
            .await;
        let app = rig.db.app().await;
        assert!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM core.event")
                .fetch_one(&app)
                .await
                .is_err()
        );
        after_a_second();
        assert_ok(&rig.migrate());
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM core.event")
            .fetch_one(&app)
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    // Scenario: 退避の設定を変えても、知らない版の当たった DB には当たらない
    #[tokio::test]
    async fn st48_migrate_backup_settings_never_bypass() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        rig.add_future_row().await;
        let before = rig.snapshot().await;
        let ok = rig.fake_pg_dump("pg_dump_ok", true);
        let other_dump = rig.fake_pg_dump("pg_dump_other", true);
        let other_dir = rig.dir.path().join("elsewhere");
        let off = rig.migrate_with(&rig.backups(), &ok, Some("off"));
        let moved = rig.migrate_with(&other_dir, &ok, None);
        let other = rig.migrate_with(&rig.backups(), &other_dump, None);
        for out in [off, moved, other] {
            assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
        }
        assert_eq!(rig.snapshot().await, before);
        assert_eq!(Rig::dumps_in(&other_dir), 0);
    }

    // ---- サーバの起動の検査（design D6・D17）。アプリの役割で起動し、`/healthz` が 200 になるかを見る ----

    /// 起動した結果。`healthy` は `/healthz` が 200 を返したことがあるか。
    struct Served {
        out: Output,
        healthy: bool,
    }

    /// `/healthz` を 1 回叩いて 200 か。繋がらなければ偽。
    fn healthz_ok(port: u16) -> bool {
        use std::io::{Read as _, Write as _};
        let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", port)) else {
            return false;
        };
        s.set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .ok();
        if s.write_all(b"GET /healthz HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
            .is_err()
        {
            return false;
        }
        let mut resp = String::new();
        s.read_to_string(&mut resp).ok();
        resp.starts_with("HTTP/1.1 200")
    }

    impl Rig {
        /// アプリの役割でサーバを起動する。落ちるか、`/healthz` が 200 になるか、20 秒で打ち切る。
        fn serve(&self) -> Served {
            let port = std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port();
            let mut cmd = server_cmd();
            cmd.env(
                "DATABASE_URL",
                with_database(&env("DATABASE_URL"), &self.db.name),
            )
            .env("BIND", format!("127.0.0.1:{port}"))
            // 取り込み器の置き場は既定（本人のホーム）を触らないよう、この試験の一時ディレクトリへ
            .env("ASHIATO_MEDIA_DIR", self.dir.path().join("media"));
            let mut child = cmd.spawn().unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
            let mut healthy = false;
            while child.try_wait().unwrap().is_none() {
                if healthz_ok(port) {
                    healthy = true;
                    child.kill().ok();
                    break;
                }
                if std::time::Instant::now() > deadline {
                    child.kill().ok();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Served {
                out: child.wait_with_output().unwrap(),
                healthy,
            }
        }
    }

    /// 拒否の共通: 終了コード 3・1 行目が `error: kind=<kind>`・待ち受けを開いていない。
    fn assert_refused_3(served: &Served, kind: &str) -> String {
        let err = stderr(&served.out);
        assert_eq!(served.out.status.code(), Some(3), "{err}");
        assert_eq!(
            err.lines().next(),
            Some(format!("error: kind={kind}").as_str()),
            "{err}"
        );
        assert!(!served.healthy, "待ち受けを開いている");
        err
    }

    // Scenario: 新しい版で当てた DB に古い版のサーバを起動すると、起動しない
    #[tokio::test]
    async fn st48_server_refuses_db_ahead() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        rig.add_future_row().await;
        let served = rig.serve();
        let err = assert_refused_3(&served, "db_ahead");
        assert!(
            err.contains("  db: 209912312359_from_the_future (0.9.9 deadbeef)"),
            "{err}"
        );
        assert!(
            err.contains(&format!("  self: {BUILD_VERSION} {BUILD_COMMIT}")),
            "{err}"
        );
    }

    // Scenario: まだ当たっていない版のある DB では、サーバは起動しない
    #[tokio::test]
    async fn st48_server_refuses_db_behind() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        rig.exec(
            "DELETE FROM migration_ledger.applied WHERE name IN
               (SELECT name FROM migration_ledger.applied ORDER BY name DESC LIMIT 2)",
        )
        .await;
        let served = rig.serve();
        let err = assert_refused_3(&served, "db_behind");
        assert!(err.contains("まだ当たっていない版が 2 個"), "{err}");
        assert!(err.contains("ashiato-server migrate"), "{err}");
    }

    // Scenario: 版の記録の無い DB では、サーバは起動しない
    #[tokio::test]
    async fn st48_server_refuses_pre_ledger_db() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        rig.exec("DROP SCHEMA migration_ledger CASCADE").await;
        let served = rig.serve();
        let err = assert_refused_3(&served, "db_behind");
        assert!(
            err.contains(&format!(
                "まだ当たっていない版が {} 個",
                ashiato_server::MIGRATIONS.len()
            )),
            "{err}"
        );
    }

    // Scenario: 版が揃っていれば、サーバは起動する
    #[tokio::test]
    async fn st48_server_starts_when_even() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        let served = rig.serve();
        assert!(
            served.healthy,
            "/healthz が 200 にならない: {}",
            stderr(&served.out)
        );
    }

    // Scenario: 指紋の違う版があっても、サーバは警告を出して起動する
    #[tokio::test]
    async fn st48_server_starts_with_fingerprint_warning() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        let name = ashiato_server::MIGRATIONS.last().unwrap().0;
        rig.exec(&format!(
            "UPDATE migration_ledger.applied SET sha256 = 'changed' WHERE name = '{name}'"
        ))
        .await;
        let served = rig.serve();
        assert!(
            served.healthy,
            "/healthz が 200 にならない: {}",
            stderr(&served.out)
        );
        let all = format!(
            "{}{}",
            String::from_utf8_lossy(&served.out.stdout),
            stderr(&served.out)
        );
        assert!(all.contains(name), "版の名前を含む警告が無い: {all}");
    }

    // Scenario: 版の記録が読めない DB では、サーバは起動しない
    #[tokio::test]
    async fn st48_server_refuses_unreadable_ledger() {
        let rig = Rig::new().await;
        assert_ok(&rig.migrate());
        rig.exec("UPDATE migration_ledger.form SET form = 2").await;
        let served = rig.serve();
        assert_refused_3(&served, "ledger_unreadable");
    }
}
