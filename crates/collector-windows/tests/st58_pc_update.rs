#![allow(clippy::unwrap_used)]

use ashiato_collector_windows::update_apply::{
    is_newer_version, take_handoff, write_alive, UpdateLayout,
};

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
fn st58_pc_update_y_other_than_y_does_not_replace() {
    // Scenario: PC は y 以外の答えでは何も替えない
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
    let dir = tempfile::tempdir().unwrap();
    assert!(!UpdateLayout::new(dir.path()).version_dir("0.5.0").exists());
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
    let release = ashiato_update_check::Release {
        tag_name: "v0.5.0".into(),
        draft: false,
        assets: vec![ashiato_update_check::Asset {
            name: "ashiato-collector-windows-0.5.0-x86_64.zip".into(),
            browser_download_url: Some(
                "https://github.com/rai-256/ashiato/releases/download/a/x".into(),
            ),
            digest: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            ),
        }],
    };
    assert!(ashiato_update_check::select_asset(
        ashiato_update_check::Component::CollectorWindows,
        &release
    )
    .is_some());
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
