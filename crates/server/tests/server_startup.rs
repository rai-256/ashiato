// SPDX-License-Identifier: AGPL-3.0-only
//! サーバの起動を、**実行ファイルを起動して**確かめる（ST28 / design D4）。
//! 管理者・所有者の接続ではサーバは起動せず、終了コード 2 と理由の種別（値ではなく種別）で終わる。
#![allow(clippy::unwrap_used)]

use std::process::{Command, Output};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} が無い。.env を読み込む（set -a; . ./.env; set +a）"))
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
