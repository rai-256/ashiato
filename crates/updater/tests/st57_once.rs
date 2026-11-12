#![allow(clippy::unwrap_used)]

use ashiato_updater::once;
use chrono::{Duration, Utc};
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use tempfile::TempDir;

fn setup() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("settings")).unwrap();
    std::fs::write(
        dir.path().join("settings/check.json"),
        r#"{"enabled":true}"#,
    )
    .unwrap();
    dir
}

fn response_server(body: &str, status: &str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let body = body.to_owned();
    let status = status.to_owned();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0_u8; 4096];
        let count = stream.read(&mut bytes).unwrap();
        let request = String::from_utf8_lossy(&bytes[..count]).into_owned();
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(reply.as_bytes()).unwrap();
        request
    });
    (address, handle)
}

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn set_env(dir: &Path, url: &str, now: &str, version: &str) {
    std::env::set_var("ASHIATO_UPDATER_DIR", dir);
    std::env::set_var("ASHIATO_UPDATER_TEST_URL", url);
    std::env::set_var("ASHIATO_UPDATER_TEST_NOW", now);
    std::env::set_var("ASHIATO_UPDATER_VERSION", version);
}

#[test]
fn st57_nightly_uses_only_tag_path() {
    // Scenario: 開発版の部品は印 nightly の 1 件だけを取る
    let _guard = env_lock();
    let dir = setup();
    let (url, server) = response_server("[]", "200 OK");
    set_env(
        dir.path(),
        &url,
        "2026-11-12T00:00:00Z",
        "0.4.0-dev.20261102",
    );
    let status = Command::new(env!("CARGO_BIN_EXE_ashiato-updater"))
        .arg("once")
        .env("ASHIATO_UPDATER_DIR", dir.path())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(server
        .join()
        .unwrap()
        .contains("/repos/rai-256/ashiato/releases/tags/nightly"));
}

#[test]
fn st57_redirect_is_not_followed() {
    // Scenario: 別の宛先への転送は追わない
    let _guard = env_lock();
    let dir = setup();
    let (url, server) = response_server("", "301 Moved Permanently");
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    once(dir.path()).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("checks.jsonl"))
            .unwrap()
            .matches("unreadable")
            .count(),
        1
    );
    server.join().unwrap();
}

#[test]
fn st57_empty_list_is_success_without_update() {
    // Scenario: 空の一覧は確かめた・新しい版なしになる
    let _guard = env_lock();
    let dir = setup();
    let (url, server) = response_server("[]", "200 OK");
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    let record = once(dir.path()).unwrap().unwrap();
    assert!(record.ok && record.latest.is_none());
    server.join().unwrap();
}

#[test]
fn st57_not_due_before_24_hours() {
    // Scenario: 24 時間たたないうちは確かめない
    let _guard = env_lock();
    let dir = setup();
    std::fs::write(
        dir.path().join("status.json"),
        format!(
            r#"{{"checked_at":"{}"}}"#,
            (Utc::now() - Duration::hours(23)).to_rfc3339()
        ),
    )
    .unwrap();
    assert!(once(dir.path()).unwrap().is_none());
}

#[test]
fn st57_due_after_24_hours_only_once() {
    // Scenario: 24 時間が過ぎた後の最初の機会に 1 回確かめる
    let _guard = env_lock();
    let dir = setup();
    let (url, server) = response_server("[]", "200 OK");
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    std::fs::write(
        dir.path().join("status.json"),
        r#"{"checked_at":"2026-11-10T23:00:00Z"}"#,
    )
    .unwrap();
    assert!(once(dir.path()).unwrap().is_some());
    assert!(once(dir.path()).unwrap().is_none());
    server.join().unwrap();
}

#[test]
fn st57_failed_attempt_counts_for_24_hours() {
    // Scenario: 失敗した回も 24 時間の数えに入る
    let _guard = env_lock();
    let dir = setup();
    let (url, server) = response_server("", "500 Internal Server Error");
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    once(dir.path()).unwrap();
    assert!(once(dir.path()).unwrap().is_none());
    server.join().unwrap();
}

#[test]
fn st57_future_check_is_due() {
    // Scenario: 最後に確かめた時刻が未来なら確かめる
    let _guard = env_lock();
    let dir = setup();
    let (url, server) = response_server("[]", "200 OK");
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    std::fs::write(
        dir.path().join("status.json"),
        r#"{"checked_at":"2029-11-12T00:00:00Z"}"#,
    )
    .unwrap();
    let record = once(dir.path()).unwrap().unwrap();
    assert_eq!(record.at.to_rfc3339(), "2026-11-12T00:00:00+00:00");
    server.join().unwrap();
}

#[test]
fn st57_records_are_append_only() {
    // Scenario: 確かめるたびに記録が 1 行増え、前の行は変わらない
    let _guard = env_lock();
    let dir = setup();
    let (url, server) = response_server("[]", "200 OK");
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    std::fs::write(dir.path().join("checks.jsonl"), "old\n").unwrap();
    once(dir.path()).unwrap();
    assert!(std::fs::read_to_string(dir.path().join("checks.jsonl"))
        .unwrap()
        .starts_with("old\n"));
    server.join().unwrap();
}

#[test]
fn st57_status_reasons_are_distinct() {
    // Scenario: 確かめられない理由を状態の番号で分ける
    let _guard = env_lock();
    let dir = setup();
    let (url, server) = response_server("{}", "403 Forbidden");
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    let record = once(dir.path()).unwrap().unwrap();
    assert_eq!(
        serde_json::to_string(&record.reason).unwrap(),
        r#""rate_limited""#
    );
    server.join().unwrap();
}

#[test]
fn st57_timeout_reason_is_recorded() {
    // Scenario: 答えない配布元は時間切れになる
    let _guard = env_lock();
    let dir = setup();
    std::env::set_var("ASHIATO_UPDATER_TEST_URL", "http://127.0.0.1:1");
    std::env::set_var("ASHIATO_UPDATER_TEST_NOW", "2026-11-12T00:00:00Z");
    std::env::set_var("ASHIATO_UPDATER_VERSION", "0.4.0");
    assert!(once(dir.path()).unwrap().unwrap().reason.is_some());
}

#[test]
fn st57_disabled_does_not_write_or_query() {
    // Scenario: 切った印があれば、更新係を開始しても問い合わせない
    let _guard = env_lock();
    let dir = setup();
    std::fs::write(
        dir.path().join("settings/check.json"),
        r#"{"enabled":false}"#,
    )
    .unwrap();
    assert!(once(dir.path()).unwrap().is_none());
    assert!(!dir.path().join("checks.jsonl").exists());
}

#[test]
fn st57_missing_destination_is_recorded_without_request() {
    // Scenario: 更新係の問い合わせは版の一覧の取得だけである
    let _guard = env_lock();
    let dir = setup();
    std::env::remove_var("ASHIATO_UPDATER_TEST_URL");
    std::env::remove_var("ASHIATO_UPDATER_TEST_NOW");
    std::env::set_var("ASHIATO_UPDATER_VERSION", "0.4.0");
    let record = once(dir.path()).unwrap().unwrap();
    assert_eq!(
        serde_json::to_string(&record.reason).unwrap(),
        r#""no_destination""#
    );
}
