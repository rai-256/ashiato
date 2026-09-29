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
    let migrate = Command::new(env!("CARGO_BIN_EXE_ashiato-server"))
        .arg("migrate")
        .env_clear()
        .env("DATABASE_OWNER_URL", env("DATABASE_OWNER_URL"))
        .output()
        .unwrap();
    assert!(migrate.status.success(), "migrate が失敗した");

    let admin = url_as(
        &std::env::var("POSTGRES_USER").unwrap_or_else(|_| "ashiato".into()),
        &env("POSTGRES_PASSWORD"),
    );
    assert_refused(&start_with(&admin), "superuser");
    assert_refused(&start_with(&env("DATABASE_OWNER_URL")), "table_owner");
}
