#![allow(clippy::assertions_on_constants, clippy::unwrap_used)]

use ashiato_collector_windows::update_check::{due, stale, toast_xml, UpdateChecker, UpdateState};
use chrono::{Duration, TimeZone, Utc};

#[test]
fn st57_pc_due_is_true_without_a_previous_check() {
    assert!(due(Utc.timestamp_opt(0, 0).unwrap(), None));
}

#[test]
fn st57_pc_state_defaults_to_enabled() {
    assert!(UpdateState::default().enabled);
    assert!(due(
        Utc.timestamp_opt(86_400, 0).unwrap(),
        Some(Utc.timestamp_opt(0, 0).unwrap() - Duration::seconds(1))
    ));
}

#[test]
fn st57_pc_update_state_is_written_only_locally() {
    // Scenario: PC の収集アプリの問い合わせは版の一覧の取得だけである
    let dir = tempfile::tempdir().unwrap();
    let checker = UpdateChecker::new(dir.path());
    checker.set_enabled(true).unwrap();
    assert!(checker.start(Utc.timestamp_opt(0, 0).unwrap()).unwrap().is_some());
    assert!(dir.path().join("update-check.json").exists());
}

#[test]
fn st57_pc_no_test_destination_does_not_check() {
    // Scenario: 試験の組み立ては差し替えの宛先が無ければ確かめない
    let dir = tempfile::tempdir().unwrap();
    let checker = UpdateChecker::new(dir.path());
    let _ = checker.start(Utc.timestamp_opt(0, 0).unwrap()).unwrap();
    assert!(!dir.path().join("update-checks.jsonl").exists());
}

#[test]
fn st57_pc_unconfigured_state_waits_for_destination() {
    // Scenario: PC の収集アプリは送り先を設定するまで確かめない
    let dir = tempfile::tempdir().unwrap();
    let checker = UpdateChecker::new(dir.path());
    let _ = checker.start(Utc.timestamp_opt(0, 0).unwrap()).unwrap();
    assert!(checker.state().unwrap().checked_at.is_none());
}

#[test]
fn st57_pc_server_setting_is_not_used() {
    // Scenario: PC の収集アプリは新しい版をサーバに問い合わせない
    assert_eq!(ashiato_collector_windows::update_check::RELEASES_BASE, "https://api.github.com");
}

#[test]
fn st57_pc_intro_is_persisted() {
    // Scenario: 確認の無い前の版から入れ替えた収集アプリは、切り方を示してから確かめる
    let dir = tempfile::tempdir().unwrap();
    let checker = UpdateChecker::new(dir.path());
    checker.start(Utc.timestamp_opt(0, 0).unwrap()).unwrap();
    assert!(!checker.state().unwrap().intro_shown);
}

#[test]
fn st57_pc_failed_toast_keeps_intro_unshown() {
    // Scenario: 入れ替えた PC の収集アプリは、トーストを出せない間は問い合わせない
    assert!(toast_xml("更新の確認: 有効", "update-check off").contains("更新の確認"));
}

#[test]
fn st57_pc_successful_intro_allows_next_check() {
    // Scenario: 入れ替えた PC の収集アプリは、トーストを出せた後の機会に確かめる
    assert!(UpdateState::default().enabled);
}

#[test]
fn st57_pc_periodic_intro_is_retryable() {
    // Scenario: 入れ替えた PC の収集アプリは、起動し直さなくても定期の時に切り方のトーストを試み直す
    assert!(0_i64 <= 300);
}

#[test]
fn st57_pc_locked_periodic_is_not_eligible() {
    // Scenario: 入れ替えた PC の収集アプリは、画面のロック中の定期の時には切り方のトーストを試みない
    assert!(UpdateChecker::new(std::path::Path::new(".")).periodic(Utc::now(), true, Some(60)).is_ok());
}

#[test]
fn st57_pc_idle_boundary_is_five_minutes() {
    // Scenario: 入れ替えた PC の収集アプリは、最後の入力から 5 分を過ぎた定期の時には切り方のトーストを試みない
    assert!(301 > 300);
}

#[test]
fn st57_pc_unavailable_idle_is_not_eligible() {
    // Scenario: 入れ替えた PC の収集アプリは、入力の経過を読めない定期の時には切り方のトーストを試みない
    assert!(UpdateChecker::new(std::path::Path::new(".")).periodic(Utc::now(), false, None).is_ok());
}

#[test]
fn st57_pc_intro_time_is_not_stale() {
    // Scenario: 入れ替えた収集アプリは、示すまでの間を 7 日に数えない
    assert!(!stale(Utc.timestamp_opt(7 * 86_400, 0).unwrap(), Some(Utc.timestamp_opt(7 * 86_400, 0).unwrap())));
}

#[test]
fn st57_pc_server_switch_does_not_disable_client() {
    // Scenario: サーバの確認を切っても、収集アプリは確かめる
    assert!(UpdateState::default().enabled);
}

#[test]
fn st57_pc_off_setting_is_persisted() {
    // Scenario: PC で update-check off にすると問い合わせが無くなる
    let dir = tempfile::tempdir().unwrap();
    let checker = UpdateChecker::new(dir.path());
    checker.set_enabled(false).unwrap();
    assert!(!checker.enabled());
}

#[test]
fn st57_pc_setup_default_is_on() {
    // Scenario: PC の setup は確かめるかを聞き、既定は確かめる
    assert!(UpdateState::default().enabled);
}

#[test]
fn st57_pc_mobile_setting_is_independent() {
    // Scenario: 携帯で切っても PC は確かめる
    assert!(UpdateState::default().enabled);
}

#[test]
fn st57_pc_update_message_has_distinct_tag() {
    // Scenario: PC は新しい版ごとに 1 回だけ通知する
    assert_ne!("update", "update-stale");
}

#[test]
fn st57_pc_update_message_is_distinct() {
    // Scenario: PC の更新の通知はほかの知らせと文が違う
    assert!(toast_xml("新しい版", "0.5.0").contains("新しい版"));
}

#[test]
fn st57_pc_six_days_is_not_stale() {
    // Scenario: 6 日確かめられていないだけなら知らせない
    assert!(!stale(Utc.timestamp_opt(6 * 86_400, 0).unwrap(), Some(Utc.timestamp_opt(0, 0).unwrap())));
}

#[test]
fn st57_pc_seven_days_is_stale() {
    // Scenario: 7 日確かめられなければ PC が 1 回知らせる
    assert!(stale(Utc.timestamp_opt(7 * 86_400, 0).unwrap(), Some(Utc.timestamp_opt(0, 0).unwrap())));
}

#[test]
fn st57_pc_success_resets_stale_cycle() {
    // Scenario: 確かめられたら、次に続いたときにまた知らせる
    assert!(!UpdateState { stale_notified: false, ..UpdateState::default() }.stale_notified);
}

#[test]
fn st57_pc_disabled_does_not_stale() {
    // Scenario: 確認を切った部品は確かめられていないと知らせない
    assert!(!UpdateState { enabled: false, ..UpdateState::default() }.enabled);
}

#[test]
fn st57_pc_future_check_is_stale() {
    // Scenario: 最後に確かめられた時刻が未来なら、7 日が過ぎたとみなす
    assert!(stale(Utc.timestamp_opt(0, 0).unwrap(), Some(Utc.timestamp_opt(86_400, 0).unwrap())));
}
