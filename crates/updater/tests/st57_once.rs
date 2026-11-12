#![allow(clippy::unwrap_used)]

use ashiato_updater::once;
use chrono::{Duration, Utc};
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::Instant;
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

fn sequence_server(responses: Vec<(&str, &str)>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let responses = responses
        .into_iter()
        .map(|(body, status)| (body.to_owned(), status.to_owned()))
        .collect::<Vec<_>>();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for (body, status) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0_u8; 4096];
            let count = stream.read(&mut bytes).unwrap();
            requests.push(String::from_utf8_lossy(&bytes[..count]).into_owned());
            let reply = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).unwrap();
        }
        requests
    });
    (address, handle)
}

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
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
    let request = server.join().unwrap();
    assert!(request.contains("GET /repos/rai-256/ashiato/releases/tags/nightly HTTP/1.1"));
    let request = request.to_ascii_lowercase();
    assert!(request.contains("user-agent: ashiato-update-check\r\n"));
    assert!(request.contains("accept: application/vnd.github+json\r\n"));
    assert!(request.contains("accept-encoding: gzip\r\n"));
    assert!(request.ends_with("\r\n\r\n"));
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
fn st57_same_destination_redirects_are_followed_up_to_the_limit() {
    let _guard = env_lock();
    let dir = setup();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let redirect_url = url.clone();
    let server = thread::spawn(move || {
        let (mut first, _) = listener.accept().unwrap();
        let mut bytes = [0_u8; 4096];
        let _ = first.read(&mut bytes).unwrap();
        let redirect = format!(
            "HTTP/1.1 301 Moved Permanently\r\nLocation: {redirect_url}/next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        first.write_all(redirect.as_bytes()).unwrap();
        let (mut second, _) = listener.accept().unwrap();
        let count = second.read(&mut bytes).unwrap();
        let request = String::from_utf8_lossy(&bytes[..count]).into_owned();
        second
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]")
            .unwrap();
        request
    });
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    assert!(once(dir.path()).unwrap().unwrap().ok);
    assert!(server.join().unwrap().contains("GET /next HTTP/1.1"));
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
    std::env::remove_var("ASHIATO_UPDATER_TEST_NOW");
    std::env::remove_var("ASHIATO_UPDATER_TEST_URL");
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
    let (url, server) = response_server("", "500 Internal Server Error");
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    let old = "one\ntwo\nthree\n";
    std::fs::write(dir.path().join("checks.jsonl"), old).unwrap();
    once(dir.path()).unwrap();
    let records = std::fs::read_to_string(dir.path().join("checks.jsonl")).unwrap();
    assert!(records.starts_with(old));
    assert_eq!(records.lines().count(), 4);
    assert!(records.ends_with("\"unreadable\",\"latest\":null}\n"));
    server.join().unwrap();
}

#[test]
fn st57_status_reasons_are_distinct() {
    // Scenario: 確かめられない理由を状態の番号で分ける
    let _guard = env_lock();
    let dir = setup();
    let (url, server) = sequence_server(vec![
        ("{}", "403 Forbidden"),
        ("{}", "429 Too Many Requests"),
        ("{}", "404 Not Found"),
        ("{}", "500 Internal Server Error"),
        ("not-json", "200 OK"),
    ]);
    set_env(dir.path(), &url, "2026-11-12T00:00:00Z", "0.4.0");
    let expected = [
        r#""rate_limited""#,
        r#""rate_limited""#,
        r#""not_found""#,
        r#""unreadable""#,
        r#""unreadable""#,
    ];
    for reason in expected {
        let record = once(dir.path()).unwrap().unwrap();
        assert_eq!(serde_json::to_string(&record.reason).unwrap(), reason);
        let checked_at = serde_json::json!({"checked_at": "2026-11-10T00:00:00Z"});
        std::fs::write(
            dir.path().join("status.json"),
            serde_json::to_vec(&checked_at).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(server.join().unwrap().len(), 5);
}

#[test]
fn st57_timeout_reason_is_recorded() {
    // Scenario: 答えない配布元は時間切れになる
    let _guard = env_lock();
    let dir = setup();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0_u8; 256];
        let _ = stream.read(&mut bytes);
        // Keep the peer open well beyond the client's global timeout.  Closing
        // just after 30 seconds races the timeout path and can look unreachable.
        thread::sleep(std::time::Duration::from_secs(45));
    });
    std::env::set_var("ASHIATO_UPDATER_TEST_URL", url);
    std::env::set_var("ASHIATO_UPDATER_TEST_NOW", "2026-11-12T00:00:00Z");
    std::env::set_var("ASHIATO_UPDATER_VERSION", "0.4.0");
    let started = Instant::now();
    let record = once(dir.path()).unwrap().unwrap();
    let elapsed = started.elapsed();
    assert!(elapsed >= std::time::Duration::from_secs(29));
    assert!(elapsed < std::time::Duration::from_secs(40));
    assert_eq!(
        record.reason,
        Some(ashiato_update_check::CheckReason::Timeout)
    );
    server.join().unwrap();
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
