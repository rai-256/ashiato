#![allow(clippy::unwrap_used)]

use super::service::{
    load_settings, log_day, request_stop, wait_for_db, wait_for_db_until_stopped, ServerLog,
    SettingsError, StopSignal, WaitOutcome,
};
use super::web_session::{self, WebPasswordRefusal};
use chrono::NaiveDate;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 環境変数（書き出し・原本の置き場・設定のファイルの場所）を触る試験を 1 本ずつ走らせる。
/// 並ぶと、ほかの試験が置いた `ASHIATO_EXPORT_DIR` を読んでしまう。
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn st55_verifier_accepts_matching_password_and_rejects_other_password() {
    // ログインの口を通した確かめは st55_verifier_login_route_accepts_only_matching_password（review R42）
    let verifier = web_session::password_verifier_for_test("install-password-012345").unwrap();
    assert!(web_session::verify_password_verifier(
        &verifier,
        "install-password-012345"
    ));
    assert!(!web_session::verify_password_verifier(
        &verifier,
        "other-password-0123456"
    ));
}

#[test]
fn st55_verifier_requires_a_valid_verifier() {
    // 起動の拒否そのものは tests/server_startup.rs の st55_bin_service_mode_refuses_without_verifier（review R41）
    assert_eq!(
        web_session::check_web_password_verifier(None, "api-token-0123456789"),
        Err(WebPasswordRefusal::Missing)
    );
    assert_eq!(
        web_session::check_web_password_verifier(Some("not-a-verifier"), "api-token-0123456789"),
        Err(WebPasswordRefusal::Malformed)
    );
}

#[test]
fn st55_verifier_rejects_raw_password_in_installed_configuration() {
    // 起動の拒否そのものは tests/server_startup.rs の st55_bin_service_mode_refuses_raw_password（review R41）
    assert_eq!(
        web_session::check_installed_password_configuration(Some("raw-password"), Some("verifier")),
        Err(WebPasswordRefusal::RawPasswordPresent)
    );
}

#[test]
fn st55_verifier_rejects_api_token_as_password() {
    // 起動の拒否そのものは tests/server_startup.rs の st55_bin_service_mode_refuses_verifier_of_api_token（review R41）
    let verifier = web_session::password_verifier_for_test("api-token-0123456789").unwrap();
    assert_eq!(
        web_session::check_verifier_against_api(&verifier, "api-token-0123456789"),
        Err(WebPasswordRefusal::SameAsApiToken)
    );
}

#[test]
fn st55_verifier_changes_session_tag_material() {
    // Scenario: 照らす形で合言葉を決め直すと前の印が 401 になる
    let old = web_session::secret_tag_for_verifier("session-key-0123456789", "old-verifier");
    let new = web_session::secret_tag_for_verifier("session-key-0123456789", "new-verifier");
    assert_ne!(old, new);
    let old_login =
        web_session::WebLogin::new_with_verifier("session-key-0123456789", "old-verifier", 0);
    let new_login =
        web_session::WebLogin::new_with_verifier("session-key-0123456789", "new-verifier", 0);
    assert_eq!(
        old_login.status_for_tag_for_test(old_login.tag_for_test()),
        axum::http::StatusCode::NO_CONTENT
    );
    assert_eq!(
        new_login.status_for_tag_for_test(old_login.tag_for_test()),
        axum::http::StatusCode::UNAUTHORIZED
    );
}

#[test]
fn st55_verifier_rejects_session_key_matching_password() {
    let verifier = web_session::password_verifier_for_test("session-password-012345").unwrap();
    assert_eq!(
        web_session::check_web_session_key_verifier(
            Some("session-password-012345"),
            "api-token-0123456789",
            &verifier,
        ),
        Err(web_session::WebSessionKeyRefusal::SameAsWebPassword)
    );
}

#[test]
fn st55_service_settings_missing_is_classified_without_secret() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server.json");
    let error = load_settings(&path).unwrap_err();
    assert_eq!(error, SettingsError::Missing);
    assert!(!error.to_string().contains("secret-value"));
}

#[test]
fn st55_db_watch_interval_is_five_seconds() {
    // D35: 5 秒ごとに DB のサービスの状態を読む（review R74）
    assert_eq!(super::service::DB_WATCH_INTERVAL, Duration::from_secs(5));
}

#[test]
fn st55_db_watch_restarts_only_a_stopped_database() {
    // Scenario: DB のサービスが異常に終わると起こし直される
    assert_eq!(
        super::service::decide_db_watch(true, Ok(super::service::DbServiceState::Stopped)),
        super::service::DbWatchDecision::Restart
    );
}

#[test]
fn st55_db_watch_ignores_running_and_pending_database_states() {
    for state in [
        super::service::DbServiceState::Running,
        super::service::DbServiceState::StartPending,
        super::service::DbServiceState::StopPending,
    ] {
        assert_eq!(
            super::service::decide_db_watch(true, Ok(state)),
            super::service::DbWatchDecision::Ignore
        );
    }
}

#[test]
fn st55_db_watch_logs_query_failure_without_restarting() {
    assert_eq!(
        super::service::decide_db_watch(true, Err("query_failed".to_owned())),
        super::service::DbWatchDecision::Log("query_failed".to_owned())
    );
}

#[test]
fn st55_db_watch_uses_configured_database_service_name() {
    assert_eq!(
        super::service::configured_db_service_name(Some("ashiato-db-dev")),
        "ashiato-db-dev"
    );
    assert_eq!(
        super::service::configured_db_service_name(None),
        "ashiato-db"
    );
    // 空の値も鍵が無いのと同じに扱う（インストーラは必ず書く。review R66）
    assert_eq!(
        super::service::configured_db_service_name(Some("")),
        "ashiato-db"
    );
}

#[test]
fn st55_db_watch_settings_name_reaches_the_service_environment() {
    let _env = env_lock();
    // インストーラが server.json に書いた ASHIATO_DB_SERVICE を、service の口の準備がそのまま環境へ入れる。
    // 置き場のパスから名前を推さない（開発版の置き場でも、書かれた名前が勝つ。review R66）
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir
        .path()
        .join("ashiato-dev")
        .join("settings")
        .join("server.json");
    std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    std::fs::write(
        &settings_path,
        serde_json::json!({ "ASHIATO_DB_SERVICE": "ashiato-db-custom" }).to_string(),
    )
    .unwrap();
    std::env::remove_var("ASHIATO_DB_SERVICE");
    super::service::prepare_service(&settings_path).unwrap();
    assert_eq!(
        std::env::var("ASHIATO_DB_SERVICE").unwrap(),
        "ashiato-db-custom"
    );
    for key in [
        "ASHIATO_DB_SERVICE",
        "ASHIATO_EXPORT_DIR",
        "ASHIATO_MEDIA_DIR",
        "ASHIATO_ARCHIVE_COPY_DIR",
        "ASHIATO_SETTINGS_PATH",
    ] {
        std::env::remove_var(key);
    }
}

#[test]
fn st55_db_watch_error_log_writes_changes_and_forgets_on_success() {
    // 同じ理由は 5 秒ごとに書かない。うまくいったら忘れ、再発したらまた書く（review R65）
    let mut log = super::service::DbWatchErrorLog::default();
    assert!(log.failed("service_disabled"));
    assert!(!log.failed("service_disabled"));
    assert!(log.failed("access_denied"));
    log.succeeded();
    assert!(log.failed("access_denied"));
}

#[test]
fn st55_db_watch_suppresses_repeated_query_failure_logs() {
    assert!(super::service::should_log_db_watch_error(
        None,
        "access_denied"
    ));
    assert!(!super::service::should_log_db_watch_error(
        Some("access_denied"),
        "access_denied"
    ));
    assert!(super::service::should_log_db_watch_error(
        Some("access_denied"),
        "service_missing"
    ));
}

#[test]
fn st55_service_settings_unreadable_is_classified_without_secret() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("not-a-file");
    std::fs::create_dir(&path).unwrap();
    let error = load_settings(&path).unwrap_err();
    assert_eq!(error, SettingsError::Unreadable);
    assert!(!error.to_string().contains("secret-value"));
}

#[test]
fn st55_service_settings_malformed_is_classified_without_secret() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server.json");
    std::fs::write(&path, r#"{"API_TOKEN":"secret-value", "broken": }"#).unwrap();
    let error = load_settings(&path).unwrap_err();
    assert_eq!(error, SettingsError::Malformed);
    assert!(!error.to_string().contains("secret-value"));
}

#[test]
fn st55_service_settings_accepts_string_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server.json");
    std::fs::write(
        &path,
        r#"{"API_TOKEN":"secret-value", "BIND":"127.0.0.1:18700"}"#,
    )
    .unwrap();
    let settings = load_settings(&path).unwrap();
    assert_eq!(settings.0.get("BIND"), Some(&"127.0.0.1:18700".to_owned()));
}

#[test]
fn st55_place_defaults_are_under_the_service_state_directory() {
    let _env = env_lock();
    // Scenario: インストールした形では、指定が無ければ状態の置き場の export に書く
    let settings = Path::new("/state/settings/server.json");
    assert_eq!(
        super::service::installed_default_directories(settings),
        [
            PathBuf::from("/state/export"),
            PathBuf::from("/state/media"),
            PathBuf::from("/state/archive-copies"),
        ]
    );
}

#[test]
fn st55_place_media_default_is_not_based_on_service_profile() {
    let _env = env_lock();
    // Scenario: インストールした形では、設定が無ければ状態の置き場の media に置く
    let dir = tempfile::tempdir().unwrap();
    let service_profile = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings/server.json");
    std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    std::env::set_var("LOCALAPPDATA", service_profile.path());
    std::env::remove_var("ASHIATO_MEDIA_DIR");
    std::env::remove_var("ASHIATO_EXPORT_DIR");
    std::env::remove_var("ASHIATO_ARCHIVE_COPY_DIR");
    super::service::apply_settings_for_service(
        &settings_path,
        &super::service::Settings(std::collections::BTreeMap::new()),
    );
    assert_eq!(
        std::env::var("ASHIATO_MEDIA_DIR").unwrap(),
        dir.path().join("media").display().to_string()
    );
    assert!(!service_profile.path().join("ashiato").exists());
}

#[test]
fn st55_place_service_settings_keep_explicit_places_and_fill_empty_places() {
    let _env = env_lock();
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings/server.json");
    std::env::set_var("ASHIATO_EXPORT_DIR", "C:/explicit/export");
    std::env::set_var("ASHIATO_MEDIA_DIR", "");
    std::env::set_var("ASHIATO_ARCHIVE_COPY_DIR", "C:/explicit/archive");
    super::service::apply_settings_for_service(
        &settings_path,
        &super::service::Settings(std::collections::BTreeMap::new()),
    );
    assert_eq!(
        std::env::var("ASHIATO_EXPORT_DIR").unwrap(),
        "C:/explicit/export"
    );
    assert_eq!(
        std::env::var("ASHIATO_MEDIA_DIR").unwrap(),
        dir.path().join("media").display().to_string()
    );
    assert_eq!(
        std::env::var("ASHIATO_ARCHIVE_COPY_DIR").unwrap(),
        "C:/explicit/archive"
    );
    std::env::remove_var("ASHIATO_EXPORT_DIR");
    std::env::remove_var("ASHIATO_MEDIA_DIR");
    std::env::remove_var("ASHIATO_ARCHIVE_COPY_DIR");
}

#[tokio::test]
async fn st55_service_waits_for_db_until_connector_succeeds() {
    let attempts = std::sync::atomic::AtomicUsize::new(0);
    let outcome = wait_for_db(Duration::from_millis(1_100), || {
        let attempt = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async move {
            if attempt < 1 {
                Err("not ready")
            } else {
                Ok(())
            }
        }
    })
    .await;
    assert_eq!(outcome, WaitOutcome::Ready);
    assert!(attempts.load(std::sync::atomic::Ordering::SeqCst) >= 2);
}

#[tokio::test]
async fn st55_service_wait_db_rejects_wrong_password_without_waiting() {
    // 合言葉が DB と合わないのは待っても直らない。120 秒待って終了コード 1 で起こし直されを繰り返さず、
    // すぐ `Rejected`（終了コード 0 の拒否）にする（review R80）
    super::testdb::pool().await;
    let url = super::testdb::url();
    let scheme_end = url.find("://").unwrap() + 3;
    let at = url.rfind('@').unwrap();
    let user = url[scheme_end..at].split(':').next().unwrap();
    let wrong = format!(
        "{}{user}:wrong-password-r80{}",
        &url[..scheme_end],
        &url[at..]
    );
    let attempts = std::sync::atomic::AtomicUsize::new(0);
    let outcome = tokio::time::timeout(
        Duration::from_secs(10),
        wait_for_db(Duration::from_secs(120), || {
            attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let wrong = wrong.clone();
            async move { sqlx::PgPool::connect(&wrong).await.map(|_| ()) }
        }),
    )
    .await
    .expect("合言葉の誤りで待ち続けた");
    assert_eq!(outcome, WaitOutcome::Rejected);
    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 1);

    // 接続の文字列の誤りも同じ
    let outcome = wait_for_db(Duration::from_secs(120), || async {
        sqlx::PgPool::connect("not-a-url").await.map(|_| ())
    })
    .await;
    assert_eq!(outcome, WaitOutcome::Rejected);
}

#[tokio::test]
async fn st55_service_wait_db_times_out_with_reason() {
    let dir = tempfile::tempdir().unwrap();
    let outcome = wait_for_db(Duration::from_millis(5), || async {
        Err::<(), _>("not ready")
    })
    .await;
    assert_eq!(outcome, WaitOutcome::TimedOut);
    let today = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
    let mut log = ServerLog::new(dir.path(), today).unwrap();
    log.write_at(today, "kind=database_unavailable").unwrap();
    let contents = std::fs::read_to_string(dir.path().join("server-2026-11-03.log")).unwrap();
    assert!(contents.contains("database_unavailable"));
}

#[test]
fn st55_log_rotates_and_cleans_old_files() {
    let dir = tempfile::tempdir().unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
    std::fs::write(dir.path().join("server-2026-10-03.log"), "old").unwrap();
    std::fs::write(dir.path().join("server-2026-10-05.log"), "keep").unwrap();
    let mut log = ServerLog::new(dir.path(), today).unwrap();
    log.write_at(today, "first").unwrap();
    log.write_at(today.succ_opt().unwrap(), "second").unwrap();
    assert!(!dir.path().join("server-2026-10-03.log").exists());
    assert!(dir.path().join("server-2026-10-05.log").exists());
    assert!(dir.path().join("server-2026-11-03.log").exists());
    assert!(dir.path().join("server-2026-11-04.log").exists());
}

#[test]
fn st55_log_writes_startup_refusal_without_secret() {
    let dir = tempfile::tempdir().unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
    let mut log = ServerLog::new(dir.path(), today).unwrap();
    log.write_at(today, "kind=settings_missing").unwrap();
    let contents = std::fs::read_to_string(dir.path().join("server-2026-11-03.log")).unwrap();
    assert!(contents.contains("settings_missing"));
    assert!(!contents.contains("secret-value"));
}

#[test]
fn st55_log_keeps_non_server_files() {
    let dir = tempfile::tempdir().unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
    std::fs::write(dir.path().join("other.log"), "keep").unwrap();
    let _log = ServerLog::new(dir.path(), today).unwrap();
    assert!(dir.path().join("other.log").exists());
}

#[test]
fn st55_log_prunes_old_files_when_the_date_changes_without_restart() {
    // 止めずに動き続けるサービスでも、日付が変わって開き直すときに 30 日より古いログを消す（review R79）
    let dir = tempfile::tempdir().unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
    let mut log = ServerLog::new(dir.path(), today).unwrap();
    log.write_at(today, "first").unwrap();
    // 起動の後に 30 日の境を越える名前のファイル（起動のときには残す日付）
    std::fs::write(dir.path().join("server-2026-10-04.log"), "old").unwrap();
    std::fs::write(dir.path().join("server-2026-10-05.log"), "keep").unwrap();
    log.write_at(today.succ_opt().unwrap(), "second").unwrap();
    assert!(!dir.path().join("server-2026-10-04.log").exists());
    assert!(dir.path().join("server-2026-10-05.log").exists());
}

#[test]
fn st55_log_day_splits_at_tokyo_midnight() {
    // ログのファイルは日本の 0 時で替わる（UTC の 0 時 = 日本の 9 時ではない。review R86）
    use chrono::TimeZone;
    let before = chrono::Utc
        .with_ymd_and_hms(2026, 11, 3, 14, 59, 59)
        .unwrap();
    let after = chrono::Utc.with_ymd_and_hms(2026, 11, 3, 15, 0, 0).unwrap();
    assert_eq!(
        log_day(before),
        NaiveDate::from_ymd_opt(2026, 11, 3).unwrap()
    );
    assert_eq!(
        log_day(after),
        NaiveDate::from_ymd_opt(2026, 11, 4).unwrap()
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // 環境変数を読む書き出しの間ずっと、ほかの試験に触らせない
async fn st55_stop_removes_partial_dot_files() {
    let _env = env_lock();
    // Scenario: サービスの停止を受けると、書き途中の書き出しを残さずに終わる
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("ASHIATO_EXPORT_SELFTEST_DELAY_MS", "100");
    std::env::set_var("ASHIATO_EXPORT_DIR", dir.path());

    let db = super::testdb::isolated().await;
    let pool = db.pool().await;
    let config = super::export::ExportConfig::from_env();
    assert_eq!(config.delay, Duration::from_millis(100));
    assert_eq!(config.dir.as_deref(), Some(dir.path()));

    let jobs = super::export::ExportJobs::default();
    assert!(matches!(
        super::export::spawn(
            pool,
            &jobs,
            &config,
            super::export::Passphrase::new("st55-stop-passphrase".to_owned()),
        )
        .await,
        super::export::Spawned::Started(_)
    ));

    let partial = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(entry) = std::fs::read_dir(dir.path())
                .unwrap()
                .find_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().starts_with(".partial-"))
            {
                break entry.path();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(partial.is_file());
    assert!(jobs.running_view().is_some());

    request_stop();
    jobs.stop();
    super::service::cleanup_partial_files(dir.path()).unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        while jobs.running_view().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    assert!(std::fs::read_dir(dir.path()).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with('.')));

    // 次の起動時にも、前回の書き出しが走っているという状態を持ち越さない。
    let next_start_jobs = super::export::ExportJobs::default();
    assert!(next_start_jobs.running_view().is_none());
    super::service::cleanup_partial_files(dir.path()).unwrap();

    std::env::remove_var("ASHIATO_EXPORT_SELFTEST_DELAY_MS");
    std::env::remove_var("ASHIATO_EXPORT_DIR");
}

#[test]
fn st55_stop_cleanup_removes_only_partial_exports() {
    // 片付けは書き出しの書きかけだけを消し、置き場のほかの隠しファイルは残す（review R81）
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".partial-ashiato-export-x-1.zip"), "p").unwrap();
    std::fs::write(dir.path().join(".partial-ashiato-export-x-2.zip"), "p").unwrap();
    std::fs::write(dir.path().join(".keep"), "k").unwrap();
    std::fs::write(dir.path().join("ashiato-export.zip"), "z").unwrap();
    super::service::cleanup_partial_files(dir.path()).unwrap();
    let mut left: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(left, [".keep", "ashiato-export.zip"]);
}

#[tokio::test]
async fn st55_service_stop_before_wait_is_not_lost() {
    // 待つ者がいない間に来た停止の知らせも、後から待つ者に届く（review R14）
    let stop = StopSignal::new();
    stop.request();
    tokio::time::timeout(Duration::from_secs(1), stop.wait())
        .await
        .unwrap();
}

#[tokio::test]
async fn st55_service_stop_wakes_every_waiter() {
    let stop = std::sync::Arc::new(StopSignal::new());
    let first = tokio::spawn({
        let stop = stop.clone();
        async move { stop.wait().await }
    });
    let second = tokio::spawn({
        let stop = stop.clone();
        async move { stop.wait().await }
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    stop.request();
    tokio::time::timeout(Duration::from_secs(1), async {
        first.await.unwrap();
        second.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn st55_service_stop_during_db_wait_ends_the_wait() {
    // DB を待つ間（最長 120 秒）に来た停止を、待ちの後まで持ち越さない（review R14）
    let stop = std::sync::Arc::new(StopSignal::new());
    let sender = stop.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        sender.request();
    });
    let outcome = tokio::time::timeout(
        Duration::from_secs(5),
        wait_for_db_until_stopped(
            Duration::from_secs(120),
            || async { Err::<(), _>("not ready") },
            &stop,
        ),
    )
    .await
    .unwrap();
    assert_eq!(outcome, WaitOutcome::Stopped);
}

#[test]
fn st55_service_startup_refusal_is_written_only_for_the_service_entry() {
    let _env = env_lock();
    // 開発の形（設定のファイルの場所が無い）では、作業ディレクトリの logs/server に書かない（review R15・D17）。
    // 環境変数を取り合わないよう、無い場合とある場合を 1 本で順に見る
    std::env::remove_var("ASHIATO_SETTINGS_PATH");
    let relative = Path::new("logs").join("server");
    let existed = relative.exists();
    super::service::log_startup_refusal("web_password");
    if !existed {
        assert!(!relative.exists());
    }

    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("settings").join("server.json");
    std::env::set_var("ASHIATO_SETTINGS_PATH", &settings);
    super::service::log_startup_refusal("web_session_key");
    std::env::remove_var("ASHIATO_SETTINGS_PATH");
    let logs = dir.path().join("logs").join("server");
    let text = std::fs::read_dir(&logs)
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect::<String>();
    assert!(text.contains("kind=web_session_key"));
}

#[test]
fn st55_verifier_in_development_has_its_own_reason() {
    // 開発の口に照らすための値を渡したときの理由は「元の値がある」ではない（review R22）
    assert_eq!(
        WebPasswordRefusal::VerifierInDevelopment.reason(),
        "verifier_in_development"
    );
    assert_ne!(
        WebPasswordRefusal::VerifierInDevelopment.reason(),
        WebPasswordRefusal::RawPasswordPresent.reason()
    );
}

#[tokio::test]
async fn st55_verifier_login_route_accepts_only_matching_password() {
    // Scenario: インストールした形でも、照らすための値と合う合言葉でだけログインできる
    // ログインの口（POST /session → WebLogin::accepts）を通す。1 文字違いは 401 で印を出さない（review R42）
    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use tower::ServiceExt as _;
    let password = "install-password-012345";
    let verifier = web_session::password_verifier_for_test(password).unwrap();
    let mut login =
        web_session::WebLogin::new_with_verifier("session-key-0123456789abcd", &verifier, 0);
    login.failure_delay = Duration::ZERO;
    let app = super::App::with_login(
        crate::testdb::pool().await,
        "test-token-0123456789abcdef".to_owned(),
        login,
        super::build_channel(),
    );
    let post = |value: &str| {
        Request::builder()
            .method(Method::POST)
            .uri("/session")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "password": value }).to_string(),
            ))
            .unwrap()
    };
    let wrong = super::router(app.clone())
        .oneshot(post("install-password-012346"))
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    assert!(wrong.headers().get(header::SET_COOKIE).is_none());
    let empty = super::router(app.clone()).oneshot(post("")).await.unwrap();
    assert_eq!(empty.status(), StatusCode::UNAUTHORIZED);
    let ok = super::router(app.clone())
        .oneshot(post(password))
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::NO_CONTENT);
    assert!(ok.headers().get(header::SET_COOKIE).is_some());
}

#[test]
fn st55_place_service_entry_fills_default_places_and_keeps_owner_secret_out() {
    let _env = env_lock();
    // Scenario: インストールした形では、指定が無ければ状態の置き場の export に書く
    // service の口の準備（main.rs が呼ぶ prepare_service）を通す。配線を外すとサービスのプロファイルの下に書く（review R43）
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings").join("server.json");
    std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    let owner_authority = "ashiato_owner:owner-secret@127.0.0.1:55400";
    std::fs::write(
        &settings_path,
        serde_json::json!({
            "API_TOKEN": "api-token-0123456789",
            "DATABASE_OWNER_URL": format!("postgres://{owner_authority}/ashiato"),
        })
        .to_string(),
    )
    .unwrap();
    // DATABASE_OWNER_URL はほかの試験（testdb）が読むので消さない。設定の値が入らないことを見る
    for key in [
        "ASHIATO_EXPORT_DIR",
        "ASHIATO_MEDIA_DIR",
        "ASHIATO_ARCHIVE_COPY_DIR",
    ] {
        std::env::remove_var(key);
    }
    super::service::prepare_service(&settings_path).unwrap();
    assert_eq!(
        std::env::var("ASHIATO_EXPORT_DIR").unwrap(),
        dir.path().join("export").display().to_string()
    );
    assert_eq!(
        std::env::var("ASHIATO_MEDIA_DIR").unwrap(),
        dir.path().join("media").display().to_string()
    );
    assert_eq!(
        std::env::var("ASHIATO_ARCHIVE_COPY_DIR").unwrap(),
        dir.path().join("archive-copies").display().to_string()
    );
    assert_eq!(
        std::env::var_os("ASHIATO_SETTINGS_PATH").unwrap(),
        settings_path.as_os_str()
    );
    // 表の所有者の合言葉はサーバのプロセスの環境に入れない（review R48）
    assert!(!std::env::var("DATABASE_OWNER_URL")
        .unwrap_or_default()
        .contains("owner-secret"));
    assert_eq!(std::env::var("API_TOKEN").unwrap(), "api-token-0123456789");
    for key in [
        "ASHIATO_EXPORT_DIR",
        "ASHIATO_MEDIA_DIR",
        "ASHIATO_ARCHIVE_COPY_DIR",
        "ASHIATO_SETTINGS_PATH",
        "API_TOKEN",
    ] {
        std::env::remove_var(key);
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // 環境変数を書き換えて run_inner を回す間ずっと、ほかの試験に触らせない
async fn st55_service_run_inner_setting_refusals_are_typed_as_refused() {
    let _env = env_lock();
    // 本番の起動の口（run_inner）が、設定の誤りを型の付いた拒否で返すこと。型の無い誤りに戻ると
    // service の口では終了コード 1 になり、`failureflag 1` の回復が起こし直し続ける（review R72）
    use super::service::ServiceEnd;
    let keys = [
        "ASHIATO_SERVICE_MODE",
        "ASHIATO_SETTINGS_PATH",
        "DATABASE_URL",
        "API_TOKEN",
    ];
    let saved: Vec<_> = keys
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect();
    std::env::set_var("ASHIATO_SERVICE_MODE", "1");
    std::env::remove_var("ASHIATO_SETTINGS_PATH");
    // run_inner は panic の hook を差し替えるので、終わったら元に戻す
    let hook = std::panic::take_hook();
    let mut ends = Vec::new();
    std::env::remove_var("DATABASE_URL");
    std::env::remove_var("API_TOKEN");
    ends.push((
        "database_url_missing",
        ServiceEnd::classify(&super::run_inner().await),
    ));
    std::env::set_var("DATABASE_URL", "postgres://ashiato_app@127.0.0.1:1/ashiato");
    ends.push((
        "api_token_missing",
        ServiceEnd::classify(&super::run_inner().await),
    ));
    std::env::set_var("API_TOKEN", "short");
    ends.push((
        "api_token_short",
        ServiceEnd::classify(&super::run_inner().await),
    ));
    std::panic::set_hook(hook);
    for (key, value) in saved {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    for (kind, end) in ends {
        assert_eq!(end, ServiceEnd::Refused, "{kind}");
        assert_eq!(end.exit_code(), 0, "{kind}");
    }
}

#[test]
fn st55_service_unrecoverable_refusals_end_with_exit_code_zero() {
    let _env = env_lock();
    // 起こし直しても直らない拒否（設定・合言葉・鍵・待ち受け・役割・DB の版など）は SCM へ 0 を返し、
    // `failureflag 1` の回復に 10 秒ごとに起こし直させない（review R21・R60）。
    // 本番の拒否の口（refuse_startup・refuse_setting）が作る誤りを分ける。ログの置き場が無いので何も書かない
    use super::service::ServiceEnd;
    std::env::remove_var("ASHIATO_SETTINGS_PATH");
    for (kind, code) in [
        ("web_password", 2),
        ("web_session_key", 2),
        ("bind_not_loopback", 2),
        ("db_not_loopback", 2),
        ("db_role", 2),
        ("db_version", 3),
        ("channel_mismatch", 4),
        ("media_dir_relative", 2),
    ] {
        let end = ServiceEnd::classify(&Err(super::refuse_startup(true, kind, code)));
        assert_eq!(end, ServiceEnd::Refused, "{kind}");
        assert_eq!(end.exit_code(), 0, "{kind}");
        assert!(!end.needs_log(), "{kind}");
    }
    for kind in [
        "database_url_missing",
        "api_token_missing",
        "api_token_short",
        "web_session_max_age",
        "archive_config",
        "outage_warmup",
    ] {
        let error = super::refuse_setting(true, kind, anyhow::anyhow!("設定の誤り"));
        let end = ServiceEnd::classify(&Err(error));
        assert_eq!(end.exit_code(), 0, "{kind}");
        assert!(!end.needs_log(), "{kind}");
    }
    // 設定のファイルを読めない（main.rs の SETTINGS_REFUSED）も同じ拒否
    assert_eq!(ServiceEnd::Refused.exit_code(), 0);
    // 停止の知らせで終わったときも 0
    assert_eq!(ServiceEnd::classify(&Ok(())).exit_code(), 0);
}

#[test]
fn st55_service_database_timeout_and_other_failures_end_non_zero() {
    // DB を 120 秒待っても繋げないときだけは回復に起こし直させる（spec distribution「サービスの回復が起こし直す」）。
    // 実行環境を作れない・型の無い失敗（待った後の切断・待ち受けの失敗など）も起こし直せば直りうるので非 0（review R60）
    use super::service::{DatabaseUnavailable, ServiceEnd};
    let timeout = ServiceEnd::classify(&Err(DatabaseUnavailable.into()));
    assert_eq!(timeout, ServiceEnd::DatabaseUnavailable);
    assert_ne!(timeout.exit_code(), 0);
    // 理由は run_service が書いた。2 行目を書かない（review R67）
    assert!(!timeout.needs_log());
    assert_ne!(ServiceEnd::RuntimeUnavailable.exit_code(), 0);
    let other = ServiceEnd::classify(&Err(anyhow::anyhow!("connection reset")));
    assert_eq!(other, ServiceEnd::Failed);
    assert_ne!(other.exit_code(), 0);
    assert!(other.needs_log());
    // 開発の口では設定の誤りを型で包まない（端末に今までの文が出る）
    let dev = super::refuse_setting(
        false,
        "api_token_short",
        anyhow::anyhow!("API_TOKEN が短すぎる"),
    );
    assert_eq!(ServiceEnd::classify(&Err(dev)), ServiceEnd::Failed);
}

#[test]
fn st55_service_spec_numbers_are_pinned() {
    // spec で決めた数を本番の定数で固定する（試験が自分で渡す上限だけでは、本番の値を変えても落ちない。review R44）
    assert_eq!(super::service::DB_WAIT, Duration::from_secs(120));
    assert_eq!(super::service::DB_RETRY, Duration::from_secs(1));
    assert_eq!(super::service::SERVER_LOG_KEEP_DAYS, 30);
}

#[test]
fn st55_log_keeps_thirty_days_and_removes_the_thirty_first() {
    // Scenario: ログは DB が 7 日分、サーバが 30 日分残る
    // （サーバの側。DB の側は installer の st55_db_config_rotates_weekday_files）
    // 境目で見る: 30 日前は残り、31 日前は消える（review R44）
    let dir = tempfile::tempdir().unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
    for days in [30_u64, 31] {
        let date = today - chrono::Days::new(days);
        std::fs::write(
            dir.path()
                .join(format!("server-{}.log", date.format("%Y-%m-%d"))),
            b"x",
        )
        .unwrap();
    }
    ServerLog::new(dir.path(), today).unwrap();
    assert!(dir.path().join("server-2026-10-05.log").exists());
    assert!(!dir.path().join("server-2026-10-04.log").exists());
}
