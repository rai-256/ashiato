#![allow(clippy::unwrap_used)]

use ashiato_collector_windows::update_check::{
    due, toast_xml, Toaster, UpdateChecker, UpdateState,
};
use ashiato_collector_windows::{
    config::Config,
    setup::{run_setup_flow, SetupOutcome},
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
    texts: Mutex<Vec<String>>,
}
impl Toaster for FakeToaster {
    fn show_update(&self, xml: &str, tag: &str) -> anyhow::Result<()> {
        self.calls.lock().unwrap().push(tag.into());
        self.texts.lock().unwrap().push(xml.into());
        if self.fail {
            anyhow::bail!("toast unavailable");
        }
        Ok(())
    }
}

fn server_n(
    body: &str,
    expected: usize,
) -> (String, Arc<Mutex<Vec<String>>>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&seen);
    let body = body.to_owned();
    let thread = std::thread::spawn(move || {
        for _ in 0..expected {
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
            let Some((mut stream, _)) = accepted else {
                break;
            };
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
fn server(body: &str) -> (String, Arc<Mutex<Vec<String>>>, std::thread::JoinHandle<()>) {
    server_n(body, 1)
}
fn releases(version: &str) -> String {
    format!(
        r#"[{{"tag_name":"v{version}","draft":false,"assets":[{{"name":"ashiato-collector-windows-{version}-x86_64.zip"}}]}}]"#
    )
}
fn configure(dir: &std::path::Path, url: &str) {
    std::fs::write(
        dir.join("settings.json"),
        format!(r#"{{"base_url":"{url}"}}"#),
    )
    .unwrap();
}
fn configure_with_mobile_setting(dir: &std::path::Path, url: &str, enabled: bool) {
    std::fs::write(
        dir.join("settings.json"),
        format!(r#"{{"base_url":"{url}","mobile_update_check_enabled":{enabled}}}"#),
    )
    .unwrap();
}
fn env_at<T>(url: Option<&str>, now: &str, f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL");
    if let Some(url) = url {
        std::env::set_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL", url);
    }
    std::env::set_var("ASHIATO_COLLECTOR_TEST_NOW", now);
    let result = f();
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL");
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_NOW");
    result
}
fn env<T>(url: Option<&str>, f: impl FnOnce() -> T) -> T {
    env_at(url, "2026-01-02T00:00:00Z", f)
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
    let requests = seen.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let request = requests.first().unwrap().to_ascii_lowercase();
    assert!(request.starts_with("get /repos/rai-256/ashiato/releases?per_page=20 http/1.1\r\n"));
    for line in request.lines().skip(1).filter(|line| !line.is_empty()) {
        let name = line.split_once(':').unwrap().0;
        assert!(
            matches!(
                name,
                "host" | "user-agent" | "accept" | "accept-encoding" | "connection"
            ),
            "unexpected header: {line}"
        );
    }
    assert!(!request.contains(env!("CARGO_PKG_VERSION")));
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
    let dir = tempfile::tempdir().unwrap();
    let (url, _, thread) = server("not-json");
    configure(dir.path(), &url);
    let toaster = Arc::new(FakeToaster::default());
    let state = UpdateState {
        checked_at: Some(Utc.with_ymd_and_hms(2025, 12, 31, 0, 0, 0).unwrap()),
        intro_shown_at: Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
        ..UpdateState::default()
    };
    std::fs::write(
        dir.path().join("update-check.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    env_at(Some(&url), "2026-01-07T00:00:00Z", || {
        let record = UpdateChecker::new(dir.path())
            .with_toaster(toaster.clone())
            .start(Utc::now())
            .unwrap()
            .unwrap();
        assert!(!record.ok);
    });
    thread.join().unwrap();
    assert!(!toaster
        .calls
        .lock()
        .unwrap()
        .contains(&"update-stale".into()));
}
#[test]
fn st57_pc_server_switch_does_not_disable_client() {
    // Scenario: サーバの確認を切っても、収集アプリは確かめる
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure_with_mobile_setting(dir.path(), &url, false);
    std::fs::write(
        dir.path().join("update-check.json"),
        serde_json::to_vec(&UpdateState::default()).unwrap(),
    )
    .unwrap();
    env(Some(&url), || {
        let c = UpdateChecker::new(dir.path());
        assert!(c.start(Utc::now()).unwrap().is_some());
    });
    thread.join().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}
#[test]
fn st57_pc_off_setting_stops_start_and_periodic_after_intro() {
    // Scenario: PC で update-check off にすると問い合わせが無くなる
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    let c = UpdateChecker::new(dir.path());
    c.set_enabled(true).unwrap();
    // 入れ替えの門ではなく enabled の門を通る状態を先に作る。
    c.state().unwrap();
    c.set_enabled(false).unwrap();
    assert!(!c.enabled().unwrap());
    assert!(c.start(Utc::now()).unwrap().is_none());
    assert!(c.periodic(Utc::now(), false, Some(0)).unwrap().is_none());
    thread.join().unwrap();
    assert!(seen.lock().unwrap().is_empty());
}
#[test]
fn st57_pc_setup_default_is_on() {
    // Scenario: PC の setup は確かめるかを聞き、既定は確かめる
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        base_url: "http://127.0.0.1:1".into(),
        api_token: "token".into(),
        user_id: uuid::Uuid::nil(),
        device_id: "pc-test".into(),
        state_dir: dir.path().to_owned(),
        settings_mtime: None,
        credential_mtime: None,
        config_from_env: false,
        exclusions_path: dir.path().join("exclusions.json"),
        launch_state_dir: dir.path().to_owned(),
        launch_device_id: "pc-test".into(),
    };
    let mut input = std::io::Cursor::new(b"\n\n".to_vec());
    let mut output = Vec::new();
    let connect = |_: &str, _: &str| -> Box<dyn ashiato_collector_windows::sender::Transport> {
        unreachable!("既存の送り先なので接続しない")
    };
    assert_eq!(
        run_setup_flow(&cfg, &mut input, &mut output, &connect).unwrap(),
        SetupOutcome::Saved
    );
    assert!(String::from_utf8(output)
        .unwrap()
        .contains("新しい版を確かめますか [Y/n]"));
    assert!(UpdateChecker::new(dir.path()).enabled().unwrap());
}
#[test]
fn st57_pc_mobile_setting_is_independent() {
    // Scenario: 携帯で切っても PC は確かめる
    let dir = tempfile::tempdir().unwrap();
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure_with_mobile_setting(dir.path(), &url, false);
    std::fs::write(
        dir.path().join("update-check.json"),
        serde_json::to_vec(&UpdateState::default()).unwrap(),
    )
    .unwrap();
    env(Some(&url), || {
        let c = UpdateChecker::new(dir.path());
        assert!(c.start(Utc::now()).unwrap().is_some());
    });
    thread.join().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}
#[test]
fn st57_pc_update_message_is_once_per_version() {
    // Scenario: PC は新しい版ごとに 1 回だけ通知する
    let dir = tempfile::tempdir().unwrap();
    let (url, _, thread) = server_n(&releases("0.5.0"), 2);
    configure(dir.path(), &url);
    let t = Arc::new(FakeToaster::default());
    let c = UpdateChecker::new(dir.path()).with_toaster(t.clone());
    env(Some(&url), || {
        c.set_enabled(true).unwrap();
        c.start(Utc::now()).unwrap();
        // 24 時間を過ぎた同じ版の 2 回目は、問い合わせても通知しない。
        c.start(Utc::now() + chrono::Duration::hours(25)).unwrap();
        assert_eq!(
            t.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|tag| *tag == "update")
                .count(),
            1
        );
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
    let dir = tempfile::tempdir().unwrap();
    let (url, _, thread) = server("not-json");
    configure(dir.path(), &url);
    let toaster = Arc::new(FakeToaster::default());
    let state = UpdateState {
        checked_at: Some(Utc.with_ymd_and_hms(2025, 12, 31, 0, 0, 0).unwrap()),
        last_ok_at: Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
        ..UpdateState::default()
    };
    std::fs::write(
        dir.path().join("update-check.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    env_at(Some(&url), "2026-01-07T00:00:00Z", || {
        UpdateChecker::new(dir.path())
            .with_toaster(toaster.clone())
            .start(Utc::now())
            .unwrap();
    });
    thread.join().unwrap();
    assert!(!toaster
        .calls
        .lock()
        .unwrap()
        .contains(&"update-stale".into()));
}
#[test]
fn st57_pc_seven_days_is_stale() {
    // Scenario: 7 日確かめられなければ PC が 1 回知らせる
    let dir = tempfile::tempdir().unwrap();
    let (url, _, thread) = server("not-json");
    configure(dir.path(), &url);
    let toaster = Arc::new(FakeToaster::default());
    let state = UpdateState {
        checked_at: Some(Utc.with_ymd_and_hms(2025, 12, 31, 0, 0, 0).unwrap()),
        last_ok_at: Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
        ..UpdateState::default()
    };
    std::fs::write(
        dir.path().join("update-check.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    let checker = UpdateChecker::new(dir.path()).with_toaster(toaster.clone());
    env_at(Some(&url), "2026-01-08T00:00:00Z", || {
        let record = checker.start(Utc::now()).unwrap().unwrap();
        assert!(!record.ok);
        assert!(checker.start(Utc::now()).unwrap().is_none());
    });
    thread.join().unwrap();
    let calls = toaster.calls.lock().unwrap();
    assert_eq!(calls.iter().filter(|tag| *tag == "update-stale").count(), 1);
    let texts = toaster.texts.lock().unwrap();
    assert!(texts
        .iter()
        .any(|xml| xml.contains("最後に確かめた 1/1・読めない答え")));
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
    let (url, seen, thread) = server(&releases("0.5.0"));
    configure(dir.path(), &url);
    let c = UpdateChecker::new(dir.path());
    c.set_enabled(true).unwrap();
    c.set_enabled(false).unwrap();
    assert!(c.start(Utc::now()).unwrap().is_none());
    assert!(c.periodic(Utc::now(), false, Some(0)).unwrap().is_none());
    assert!(!dir.path().join("update-checks.jsonl").exists());
    thread.join().unwrap();
    assert!(seen.lock().unwrap().is_empty());
}
#[test]
fn st57_pc_future_check_is_stale() {
    // Scenario: 最後に確かめられた時刻が未来なら、7 日が過ぎたとみなす
    let dir = tempfile::tempdir().unwrap();
    let (url, _, thread) = server("not-json");
    configure(dir.path(), &url);
    let toaster = Arc::new(FakeToaster::default());
    let state = UpdateState {
        checked_at: Some(Utc.with_ymd_and_hms(2025, 12, 31, 0, 0, 0).unwrap()),
        last_ok_at: Some(Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap()),
        ..UpdateState::default()
    };
    std::fs::write(
        dir.path().join("update-check.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    env_at(Some(&url), "2026-01-01T00:00:00Z", || {
        UpdateChecker::new(dir.path())
            .with_toaster(toaster.clone())
            .start(Utc::now())
            .unwrap();
    });
    thread.join().unwrap();
    assert!(toaster
        .calls
        .lock()
        .unwrap()
        .contains(&"update-stale".into()));
}
