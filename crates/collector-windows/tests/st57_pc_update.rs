#![allow(clippy::unwrap_used)]

use ashiato_collector_windows::update_check::{
    due, stale, toast_xml, Toaster, UpdateChecker, UpdateState,
};
use chrono::{TimeZone, Utc};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Default)]
struct FakeToaster {
    fail: bool,
    calls: Mutex<Vec<String>>,
}
impl Toaster for FakeToaster {
    fn show_update(&self, _xml: &str, tag: &str) -> anyhow::Result<()> {
        self.calls.lock().unwrap().push(tag.into());
        if self.fail {
            anyhow::bail!("toast unavailable");
        }
        Ok(())
    }
}

fn server(body: &str) -> (String, Arc<Mutex<Vec<String>>>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&seen);
    let body = body.to_owned();
    let thread = std::thread::spawn(move || {
        let mut accepted = None;
        for _ in 0..100 {
            match listener.accept() {
                Ok(value) => {
                    accepted = Some(value);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10))
                }
                Err(_) => break,
            }
        }
        if let Some((mut stream, _)) = accepted {
            let mut request = Vec::new();
            let mut byte = [0; 1];
            while stream.read_exact(&mut byte).is_ok() {
                request.push(byte[0]);
                if request.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            output
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&request).into_owned());
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });
    (url, seen, thread)
}
fn releases(version: &str) -> String {
    format!(
        r#"[{{"tag_name":"v{version}","draft":false,"assets":[{{"name":"ashiato-collector-windows-{version}-x86_64.zip"}}]}}]"#
    )
}
fn configure(dir: &std::path::Path, url: &str) {
    std::fs::write(
        dir.join("settings.json"),
        format!(r#"{{"base_url":"{url}","update_check_enabled":true}}"#),
    )
    .unwrap();
}
fn env<T>(url: Option<&str>, f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(url) = url {
        std::env::set_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL", url);
    }
    std::env::set_var("ASHIATO_COLLECTOR_TEST_NOW", "2026-01-02T00:00:00Z");
    let result = f();
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL");
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_NOW");
    result
}
#[test]
fn st57_pc_due_is_true_without_a_previous_check() {
    let checked = Utc.timestamp_opt(0, 0).unwrap();
    assert!(due(checked + chrono::Duration::hours(24), Some(checked)));
    assert!(!due(checked + chrono::Duration::hours(23), Some(checked)));
    assert!(due(checked, None));
}

#[test]
fn st57_pc_update_state_is_written_only_locally() {
    // Scenario: PC の収集アプリの問い合わせは版の一覧の取得だけである
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    env(Some(&url), || {
        let c = UpdateChecker::new(dir.path());
        c.set_enabled(true).unwrap();
        assert!(c.start(Utc::now()).unwrap().is_some());
    });
    thread.join().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert!(dir.path().join("update-checks.jsonl").exists());
}
#[test]
fn st57_pc_no_test_destination_does_not_check() {
    // Scenario: 試験の組み立ては差し替えの宛先が無ければ確かめない
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    env(None, || {
        let c = UpdateChecker::new(dir.path());
        c.set_enabled(true).unwrap();
        assert!(c.start(Utc::now()).unwrap().is_some());
    });
    assert!(
        std::fs::read_to_string(dir.path().join("update-checks.jsonl"))
            .unwrap()
            .contains("no_destination")
    );
    thread.join().unwrap();
    assert!(seen.lock().unwrap().is_empty());
}
#[test]
fn st57_pc_unconfigured_state_waits_for_destination() {
    // Scenario: PC の収集アプリは送り先を設定するまで確かめない
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    env(Some(&url), || {
        let c = UpdateChecker::new(dir.path());
        c.set_enabled(true).unwrap();
        assert!(c.start(Utc::now()).unwrap().is_some());
    });
    assert!(
        std::fs::read_to_string(dir.path().join("update-checks.jsonl"))
            .unwrap()
            .contains("no_destination")
    );
    thread.join().unwrap();
    assert!(seen.lock().unwrap().is_empty());
}
#[test]
fn st57_pc_server_setting_is_not_used() {
    // Scenario: PC の収集アプリは新しい版をサーバに問い合わせない
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    env(Some(&url), || {
        let c = UpdateChecker::new(dir.path());
        c.set_enabled(true).unwrap();
        assert_eq!(
            c.start(Utc::now()).unwrap().unwrap().latest.as_deref(),
            Some("0.5.0")
        );
    });
    thread.join().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}
#[test]
fn st57_pc_intro_is_persisted() {
    // Scenario: 確認の無い前の版から入れ替えた収集アプリは、切り方を示してから確かめる
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    let c = UpdateChecker::new(dir.path()).with_toaster(Arc::new(FakeToaster::default()));
    env(Some(&url), || {
        assert!(c.start(Utc::now()).unwrap().is_none());
        assert!(c.state().unwrap().intro_shown);
    });
    thread.join().unwrap();
    assert!(seen.lock().unwrap().is_empty());
}
#[test]
fn st57_pc_failed_toast_keeps_intro_unshown() {
    // Scenario: 入れ替えた PC の収集アプリは、トーストを出せない間は問い合わせない
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    let c = UpdateChecker::new(dir.path()).with_toaster(Arc::new(FakeToaster {
        fail: true,
        ..Default::default()
    }));
    env(Some(&url), || {
        assert!(c.start(Utc::now()).is_err());
        assert!(!c.state().unwrap().intro_shown);
    });
    thread.join().unwrap();
    assert!(seen.lock().unwrap().is_empty());
}
#[test]
fn st57_pc_successful_intro_allows_next_check() {
    // Scenario: 入れ替えた PC の収集アプリは、トーストを出せた後の機会に確かめる
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    let c = UpdateChecker::new(dir.path()).with_toaster(Arc::new(FakeToaster::default()));
    env(Some(&url), || {
        c.start(Utc::now()).unwrap();
        assert!(c.start(Utc::now()).unwrap().is_some());
    });
    thread.join().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}
#[test]
fn st57_pc_periodic_intro_is_retryable() {
    // Scenario: 入れ替えた PC の収集アプリは、起動し直さなくても定期の時に切り方のトーストを試み直す
    let dir = tempfile::tempdir().unwrap();
    let t = Arc::new(FakeToaster {
        fail: true,
        ..Default::default()
    });
    let c = UpdateChecker::new(dir.path()).with_toaster(t.clone());
    assert!(c.start(Utc::now()).is_err());
    assert!(c.periodic(Utc::now(), false, Some(60)).is_err());
    assert_eq!(t.calls.lock().unwrap().len(), 2);
}
#[test]
fn st57_pc_locked_periodic_is_not_eligible() {
    // Scenario: 入れ替えた PC の収集アプリは、画面のロック中の定期の時には切り方のトーストを試みない
    let dir = tempfile::tempdir().unwrap();
    let t = Arc::new(FakeToaster {
        fail: true,
        ..Default::default()
    });
    let c = UpdateChecker::new(dir.path()).with_toaster(t.clone());
    assert!(c.start(Utc::now()).is_err());
    assert!(c.periodic(Utc::now(), true, Some(60)).is_ok());
    assert_eq!(t.calls.lock().unwrap().len(), 1);
}
#[test]
fn st57_pc_idle_boundary_is_five_minutes() {
    // Scenario: 入れ替えた PC の収集アプリは、最後の入力から 5 分を過ぎた定期の時には切り方のトーストを試みない
    let dir = tempfile::tempdir().unwrap();
    let t = Arc::new(FakeToaster {
        fail: true,
        ..Default::default()
    });
    let c = UpdateChecker::new(dir.path()).with_toaster(t.clone());
    assert!(c.start(Utc::now()).is_err());
    assert!(c.periodic(Utc::now(), false, Some(301)).is_ok());
    assert_eq!(t.calls.lock().unwrap().len(), 1);
    assert!(c.periodic(Utc::now(), false, Some(300)).is_err());
    assert_eq!(t.calls.lock().unwrap().len(), 2);
}
#[test]
fn st57_pc_unavailable_idle_is_not_eligible() {
    // Scenario: 入れ替えた PC の収集アプリは、入力の経過を読めない定期の時には切り方のトーストを試みない
    let dir = tempfile::tempdir().unwrap();
    let t = Arc::new(FakeToaster {
        fail: true,
        ..Default::default()
    });
    let c = UpdateChecker::new(dir.path()).with_toaster(t.clone());
    assert!(c.start(Utc::now()).is_err());
    assert!(c.periodic(Utc::now(), false, None).is_ok());
    assert_eq!(t.calls.lock().unwrap().len(), 1);
}
#[test]
fn st57_pc_intro_time_is_not_stale() {
    // Scenario: 入れ替えた収集アプリは、示すまでの間を 7 日に数えない
    assert!(!stale(
        Utc.timestamp_opt(7 * 86_400, 0).unwrap(),
        Some(Utc.timestamp_opt(7 * 86_400, 0).unwrap())
    ));
}
#[test]
fn st57_pc_server_switch_does_not_disable_client() {
    // Scenario: サーバの確認を切っても、収集アプリは確かめる
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"update_check_enabled":true,"mobile_update_check_enabled":false}"#,
    )
    .unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    env(Some(&url), || {
        let c = UpdateChecker::new(dir.path());
        c.set_enabled(true).unwrap();
        assert!(c.start(Utc::now()).unwrap().is_some());
    });
    thread.join().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}
#[test]
fn st57_pc_off_setting_is_persisted() {
    // Scenario: PC で update-check off にすると問い合わせが無くなる
    let dir = tempfile::tempdir().unwrap();
    let c = UpdateChecker::new(dir.path());
    c.set_enabled(false).unwrap();
    assert!(!c.enabled().unwrap());
}
#[test]
fn st57_pc_setup_default_is_on() {
    // Scenario: PC の setup は確かめるかを聞き、既定は確かめる
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    env(Some(&url), || {
        let c = UpdateChecker::new(dir.path());
        c.set_enabled(true).unwrap();
        assert!(c.start(Utc::now()).unwrap().is_some());
    });
    thread.join().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}
#[test]
fn st57_pc_mobile_setting_is_independent() {
    // Scenario: 携帯で切っても PC は確かめる
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"update_check_enabled":true,"mobile_update_check_enabled":false}"#,
    )
    .unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    env(Some(&url), || {
        let c = UpdateChecker::new(dir.path());
        c.set_enabled(true).unwrap();
        assert!(c.start(Utc::now()).unwrap().is_some());
    });
    thread.join().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}
#[test]
fn st57_pc_update_message_has_distinct_tag() {
    // Scenario: PC は新しい版ごとに 1 回だけ通知する
    let dir = tempfile::tempdir().unwrap();
    let (url, _, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    let t = Arc::new(FakeToaster::default());
    let c = UpdateChecker::new(dir.path()).with_toaster(t.clone());
    env(Some(&url), || {
        c.set_enabled(true).unwrap();
        c.start(Utc::now()).unwrap();
        assert!(t.calls.lock().unwrap().contains(&"update".into()));
    });
    thread.join().unwrap();
}
#[test]
fn st57_pc_update_message_is_distinct() {
    // Scenario: PC の更新の通知はほかの知らせと文が違う
    let xml = toast_xml("新しい版", "0.5.0");
    assert!(xml.contains("<text>新しい版</text>"));
    assert!(!xml.contains("更新を 7 日確かめられていません"));
}
#[test]
fn st57_pc_six_days_is_not_stale() {
    // Scenario: 6 日確かめられていないだけなら知らせない
    assert!(!stale(
        Utc.timestamp_opt(6 * 86_400, 0).unwrap(),
        Some(Utc.timestamp_opt(0, 0).unwrap())
    ));
}
#[test]
fn st57_pc_seven_days_is_stale() {
    // Scenario: 7 日確かめられなければ PC が 1 回知らせる
    assert!(stale(
        Utc.timestamp_opt(7 * 86_400, 0).unwrap(),
        Some(Utc.timestamp_opt(0, 0).unwrap())
    ));
}

#[test]
fn st57_pc_failed_first_check_uses_intro_time_for_staleness() {
    let dir = tempfile::tempdir().unwrap();
    let (url, _, thread) = server("not-json");
    configure(dir.path(), &url);
    let toaster = Arc::new(FakeToaster::default());
    let state = UpdateState {
        intro_shown_at: Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
        ..UpdateState::default()
    };
    std::fs::write(
        dir.path().join("update-check.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();

    env(Some(&url), || {
        let record = UpdateChecker::new(dir.path())
            .with_toaster(toaster.clone())
            .start(Utc::now())
            .unwrap()
            .unwrap();
        assert!(!record.ok);
        assert!(!toaster
            .calls
            .lock()
            .unwrap()
            .contains(&"update-stale".into()));
    });
    thread.join().unwrap();
}
#[test]
fn st57_pc_success_resets_stale_cycle() {
    // Scenario: 確かめられたら、次に続いたときにまた知らせる
    let dir = tempfile::tempdir().unwrap();
    let (url, _, thread) = server("[]");
    configure(dir.path(), &url);
    let state = UpdateState {
        stale_notified: true,
        ..UpdateState::default()
    };
    std::fs::write(
        dir.path().join("update-check.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    env(Some(&url), || {
        UpdateChecker::new(dir.path()).start(Utc::now()).unwrap();
    });
    thread.join().unwrap();
    assert!(
        !UpdateChecker::new(dir.path())
            .state()
            .unwrap()
            .stale_notified
    );
}
#[test]
fn st57_pc_disabled_does_not_stale() {
    // Scenario: 確認を切った部品は確かめられていないと知らせない
    let dir = tempfile::tempdir().unwrap();
    let c = UpdateChecker::new(dir.path());
    c.set_enabled(false).unwrap();
    assert!(c.start(Utc::now()).unwrap().is_none());
    assert!(!dir.path().join("update-checks.jsonl").exists());
}
#[test]
fn st57_pc_future_check_is_stale() {
    // Scenario: 最後に確かめられた時刻が未来なら、7 日が過ぎたとみなす
    assert!(stale(
        Utc.timestamp_opt(0, 0).unwrap(),
        Some(Utc.timestamp_opt(86_400, 0).unwrap())
    ));
}
