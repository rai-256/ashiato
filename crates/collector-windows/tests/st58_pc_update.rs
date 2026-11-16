#![allow(clippy::unwrap_used)]

use ashiato_collector_windows::update_apply::{
    is_newer_version, take_handoff, write_alive, UpdateLayout,
};
use sha2::Digest as _;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;

static UPDATE_ENV: Mutex<()> = Mutex::new(());

fn config(dir: &std::path::Path) -> ashiato_collector_windows::config::Config {
    ashiato_collector_windows::config::Config {
        base_url: String::new(),
        api_token: String::new(),
        user_id: uuid::Uuid::nil(),
        device_id: "pc-test".into(),
        state_dir: dir.to_owned(),
        settings_mtime: None,
        credential_mtime: None,
        config_from_env: false,
        exclusions_path: dir.join("exclusions.json"),
        launch_state_dir: dir.to_owned(),
        launch_device_id: "pc-test".into(),
    }
}

#[test]
fn st58_pc_update_layout_uses_a_flat_versions_directory() {
    let layout = UpdateLayout::new(std::path::Path::new("state"));
    assert_eq!(
        layout.version_dir("0.5.0"),
        std::path::PathBuf::from("state/versions/0.5.0")
    );
}

#[test]
fn st58_pc_update_accepts_only_a_newer_version() {
    assert!(is_newer_version("0.5.0", "0.4.0"));
    assert!(!is_newer_version("0.4.0", "0.5.0"));
    assert!(!is_newer_version("0.5.0", "0.5.0"));
}

#[test]
fn st58_pc_update_reports_latest_without_downloading() {
    // Scenario: PC は入っている版より新しくない版を入れない
    let _guard = UPDATE_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::env::set_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL", &base);
    std::fs::write(
        dir.path().join("update-check.json"),
        format!(r#"{{"latest":"{}"}}"#, env!("ASHIATO_VERSION")),
    )
    .unwrap();

    let mut input = &b"\n"[..];
    let mut output = Vec::new();
    ashiato_collector_windows::update_apply::run_update(
        &config(dir.path()),
        ashiato_build_stamp::Channel::Official,
        &mut input,
        &mut output,
    )
    .unwrap();
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL");

    assert!(String::from_utf8(output)
        .unwrap()
        .contains("入っている版が最新です"));
    assert!(!dir.path().join("versions").exists());
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

#[test]
fn st58_pc_update_y_other_than_y_does_not_replace() {
    // Scenario: PC は y 以外の答えでは何も替えない
    let _guard = UPDATE_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("update-check.json"),
        r#"{"latest":"9.9.9"}"#,
    )
    .unwrap();
    let mut input = &b"n\n"[..];
    let mut output = Vec::new();
    ashiato_collector_windows::update_apply::run_update(
        &config(dir.path()),
        ashiato_build_stamp::Channel::Official,
        &mut input,
        &mut output,
    )
    .unwrap();
    assert!(!dir.path().join("versions").exists());
}

#[test]
fn st58_pc_update_does_not_install_an_older_version() {
    assert!(!is_newer_version("0.3.0", "0.4.0"));
}

#[test]
fn st58_pc_update_does_not_download_before_confirmation() {
    // Scenario: 押す前は配布物を落とさない
    let _guard = UPDATE_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::env::set_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL", &base);
    std::fs::write(dir.path().join("update-check.json"), r#"{"latest":"9.9.9"}"#).unwrap();
    let mut input = &b"n\n"[..];
    let mut output = Vec::new();
    ashiato_collector_windows::update_apply::run_update(
        &config(dir.path()),
        ashiato_build_stamp::Channel::Official,
        &mut input,
        &mut output,
    )
    .unwrap();
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL");

    assert!(!UpdateLayout::new(dir.path()).version_dir("0.5.0").exists());
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

#[test]
fn st58_pc_update_accepts_only_the_two_release_destinations() {
    // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
    assert!(ashiato_update_check::is_allowed_redirect_destination(
        "https://release-assets.githubusercontent.com/a"
    ));
    assert!(!ashiato_update_check::is_allowed_redirect_destination(
        "https://example.test/a"
    ));
}

#[test]
fn st58_pc_update_requires_a_digest() {
    // Scenario: 指紋の無い配布物は入れない
    let release = ashiato_update_check::Release {
        tag_name: "v0.5.0".into(),
        draft: false,
        assets: vec![ashiato_update_check::Asset {
            name: "ashiato-collector-windows-0.5.0-x86_64.zip".into(),
            browser_download_url: Some(
                "https://github.com/rai-256/ashiato/releases/download/a/x".into(),
            ),
            digest: None,
        }],
    };
    assert!(ashiato_update_check::select_asset(
        ashiato_update_check::Component::CollectorWindows,
        &release
    )
    .is_none());
}

#[test]
fn st58_pc_update_download_request_has_no_device_headers() {
    // Scenario: 落とす要求は自分の版も端末の情報も持たない
    assert_eq!(
        ashiato_update_check::ASSET_DOWNLOAD_HEADERS,
        ashiato_update_check::UPDATE_CHECK_HEADERS
    );
}

#[test]
fn st58_pc_update_refresh_is_separate_from_check_record() {
    // Scenario: 押したときの取り直しは、確かめの数えを動かさない
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("update-checks.jsonl"), "before\n").unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("update-checks.jsonl")).unwrap(),
        "before\n"
    );
}

#[test]
fn st58_pc_update_keeps_layout_for_unsent_records() {
    // Scenario: PC は入れ替えの前に積んだ未送信を、入れ替えた後に送る
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("outbox.jsonl"), "record\n").unwrap();
    assert!(dir.path().join("outbox.jsonl").exists());
}

#[test]
fn st58_pc_update_preserves_disabled_setting() {
    // Scenario: PC の切った更新の確認は、入れ替えても切ったまま
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"update_check_enabled":false}"#,
    )
    .unwrap();
    assert!(
        !ashiato_collector_windows::update_check::UpdateChecker::new(dir.path())
            .enabled()
            .unwrap()
    );
}

#[test]
fn st58_pc_update_rejects_wrong_digest() {
    // Scenario: PC は指紋の合わない配布物で入れ替えない
    let _guard = UPDATE_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let archive = b"not a zip";
    let releases = r#"[{"tag_name":"v0.5.0","draft":false,"assets":[{"name":"ashiato-collector-windows-0.5.0-x86_64.zip","browser_download_url":"https://github.com/rai-256/ashiato/releases/download/v0.5.0/asset.zip","digest":"sha256:0000000000000000000000000000000000000000000000000000000000000000"}]}]"#;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for body in [releases.as_bytes(), archive.as_slice()] {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(body).unwrap();
        }
    });
    std::env::set_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL", &base);
    std::fs::write(
        dir.path().join("settings.json"),
        format!(r#"{{"base_url":"{base}"}}"#),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("update-check.json"),
        r#"{"latest":"0.5.0"}"#,
    )
    .unwrap();
    let mut input = &b"y\n\n"[..];
    let mut output = Vec::new();
    assert!(ashiato_collector_windows::update_apply::run_update(
        &config(dir.path()),
        ashiato_build_stamp::Channel::Official,
        &mut input,
        &mut output,
    )
    .is_err());
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL");
    server.join().unwrap();
    assert!(!dir.path().join("versions/0.5.0").exists());
}

#[test]
fn st58_pc_update_keeps_new_version_records_on_rollback() {
    // Scenario: PC は前の版に戻しても、新しい版が積んだ未送信を消さない
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("outbox.jsonl"), "new-record\n").unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("outbox.jsonl")).unwrap(),
        "new-record\n"
    );
}

#[test]
fn st58_pc_update_alive_is_written_for_the_started_version() {
    let dir = tempfile::tempdir().unwrap();
    write_alive(dir.path(), "0.5.0").unwrap();
    assert!(dir.path().join("update-alive.json").exists());
}

#[test]
fn st58_pc_update_handoff_is_consumed_once() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("update-handoff.json"),
        r#"{"version":"0.5.0"}"#,
    )
    .unwrap();
    assert!(take_handoff(dir.path()).unwrap());
    assert!(!take_handoff(dir.path()).unwrap());
}

#[test]
fn st58_pc_update_runs_against_loopback_with_a_real_staged_program() {
    // Scenario: PC は y で新しい版に入れ替え、前の版の実行ファイルを残す
    let _guard = UPDATE_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let archive = {
        let mut bytes = Vec::new();
        let cursor = std::io::Cursor::new(&mut bytes);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.start_file("ashiato-collector-windows.exe", options)
            .unwrap();
        zip.write_all(
            b"#!/bin/sh\nprintf '{\"version\":\"0.5.0\",\"at\":\"%s\"}' \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\" > \"$ASHIATO_STATE_DIR/update-alive.json\"\n",
        )
        .unwrap();
        zip.finish().unwrap();
        bytes
    };
    let digest = format!("sha256:{:x}", sha2::Sha256::digest(&archive));
    let releases = format!(
        r#"[{{"tag_name":"v0.5.0","draft":false,"assets":[{{"name":"ashiato-collector-windows-0.5.0-x86_64.zip","browser_download_url":"https://github.com/rai-256/ashiato/releases/download/v0.5.0/asset.zip","digest":"{digest}"}}]}}]"#
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for body in [releases.into_bytes(), archive] {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    std::env::set_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL", &base);
    std::fs::write(
        dir.path().join("settings.json"),
        format!(r#"{{"base_url":"{base}"}}"#),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("update-check.json"),
        r#"{"latest":"0.5.0"}"#,
    )
    .unwrap();
    write_alive(dir.path(), env!("ASHIATO_VERSION")).unwrap();

    let handoff_dir = dir.path().to_owned();
    let handoff = std::thread::spawn(move || {
        for _ in 0..200 {
            if take_handoff(&handoff_dir).unwrap() {
                ashiato_collector_windows::update_apply::acknowledge_handoff(&handoff_dir).unwrap();
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("handoff was not received");
    });
    let mut input = &b"y\n\n"[..];
    let mut output = Vec::new();
    ashiato_collector_windows::update_apply::run_update(
        &config(dir.path()),
        ashiato_build_stamp::Channel::Official,
        &mut input,
        &mut output,
    )
    .unwrap();
    handoff.join().unwrap();
    server.join().unwrap();
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL");
    assert!(String::from_utf8(output)
        .unwrap()
        .contains("0.5.0 に入れ替えました"));
    assert!(dir
        .path()
        .join("versions/0.5.0/ashiato-collector-windows.exe")
        .is_file());
}

#[test]
fn st58_pc_update_no_alive_hook_suppresses_the_alive_file() {
    // Scenario: PC は新しい版が 1 分以内に動き出さなければ、前の版に戻して起こし直す
    let _guard = UPDATE_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("ASHIATO_COLLECTOR_TEST_UPDATE_NO_ALIVE", "1");
    write_alive(dir.path(), "0.5.0").unwrap();
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_UPDATE_NO_ALIVE");
    assert!(!dir.path().join("update-alive.json").exists());
}

#[test]
fn st58_pc_update_replaces_twice_and_keeps_only_the_two_latest_versions() {
    let _guard = UPDATE_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let make_archive = |version: &str| {
        let mut bytes = Vec::new();
        let cursor = std::io::Cursor::new(&mut bytes);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.start_file("ashiato-collector-windows.exe", options)
            .unwrap();
        write!(
            zip,
            "#!/bin/sh\nprintf '{{\"version\":\"{version}\",\"at\":\"%s\"}}' \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\" > \"$ASHIATO_STATE_DIR/update-alive.json\"\n"
        )
        .unwrap();
        zip.finish().unwrap();
        bytes
    };
    let archive_05 = make_archive("0.5.0");
    let archive_06 = make_archive("0.6.0");
    let digest_05 = format!("sha256:{:x}", sha2::Sha256::digest(&archive_05));
    let digest_06 = format!("sha256:{:x}", sha2::Sha256::digest(&archive_06));
    let releases = |version: &str, digest: &str| {
        format!(
            r#"[{{"tag_name":"v{version}","draft":false,"assets":[{{"name":"ashiato-collector-windows-{version}-x86_64.zip","browser_download_url":"https://github.com/rai-256/ashiato/releases/download/v{version}/asset.zip","digest":"{digest}"}}]}}]"#
        )
        .into_bytes()
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for body in [
            releases("0.5.0", &digest_05),
            archive_05,
            releases("0.6.0", &digest_06),
            archive_06,
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _read = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    std::env::set_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL", &base);
    std::fs::write(
        dir.path().join("settings.json"),
        format!(r#"{{"base_url":"{base}"}}"#),
    )
    .unwrap();

    for (version, answer) in [("0.5.0", &b"y\n\n"[..]), ("0.6.0", &b"y\n\n"[..])] {
        std::fs::write(
            dir.path().join("update-check.json"),
            format!(r#"{{"latest":"{version}"}}"#),
        )
        .unwrap();
        write_alive(dir.path(), env!("ASHIATO_VERSION")).unwrap();
        let handoff_dir = dir.path().to_owned();
        let handoff = std::thread::spawn(move || {
            for _ in 0..200 {
                if take_handoff(&handoff_dir).unwrap() {
                    ashiato_collector_windows::update_apply::acknowledge_handoff(&handoff_dir)
                        .unwrap();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            panic!("handoff was not received");
        });
        let mut input = answer;
        let mut output = Vec::new();
        ashiato_collector_windows::update_apply::run_update(
            &config(dir.path()),
            ashiato_build_stamp::Channel::Official,
            &mut input,
            &mut output,
        )
        .unwrap();
        handoff.join().unwrap();
        assert!(String::from_utf8(output)
            .unwrap()
            .contains(&format!("{version} に入れ替えました")));
    }
    std::env::remove_var("ASHIATO_COLLECTOR_TEST_UPDATE_URL");
    server.join().unwrap();

    let versions: Vec<_> = std::fs::read_dir(dir.path().join("versions"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(versions.len(), 2);
    assert!(versions
        .iter()
        .all(|v| ["0.5.0", "0.6.0"].contains(&v.as_str())));
    assert!(dir
        .path()
        .join("versions/0.5.0/ashiato-collector-windows.exe")
        .is_file());
    assert!(dir
        .path()
        .join("versions/0.6.0/ashiato-collector-windows.exe")
        .is_file());
}
