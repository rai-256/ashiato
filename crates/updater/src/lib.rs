// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 更新係の一回の確認と、結果の永続化。

use ashiato_update_check::{CheckReason, CheckRecord, Component, Status};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};
use std::time::Duration;
#[cfg(windows)]
use std::time::Instant;

pub const GITHUB_API: &str = "https://api.github.com";
pub const RELEASES_URL: &str = "https://api.github.com/repos/rai-256/ashiato/releases?per_page=20";
const CHECK_FILE: &str = "checks.jsonl";
const SETTINGS_FILE: &str = "settings/check.json";
const TEST_HOOKS_FILE: &str = "settings/test-hooks.json";

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub enabled: bool,
    pub since: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TestHooks {
    url: Option<String>,
    now: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O: {e}"),
            Self::Json(e) => write!(f, "JSON: {e}"),
            Self::Invalid(e) => f.write_str(e),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

fn settings(dir: &Path) -> Result<Settings, Error> {
    let path = dir.join(SETTINGS_FILE);
    if !path.is_file() {
        return Ok(Settings {
            enabled: true,
            since: None,
        });
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn hooks(dir: &Path) -> TestHooks {
    #[cfg(ashiato_test_build)]
    {
        let _ = dir;
        TestHooks {
            url: std::env::var("ASHIATO_UPDATER_TEST_URL").ok(),
            now: std::env::var("ASHIATO_UPDATER_TEST_NOW")
                .ok()
                .and_then(|v| ashiato_update_check::parse_rfc3339(&v).ok()),
        }
    }
    #[cfg(not(ashiato_test_build))]
    {
        let _ = dir;
        TestHooks::default()
    }
}

fn now_and_url(dir: &Path) -> (DateTime<Utc>, Option<String>) {
    let hook = hooks(dir);
    #[cfg(ashiato_test_build)]
    let file = fs::read(dir.join(TEST_HOOKS_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice::<TestHooks>(&b).ok());
    #[cfg(not(ashiato_test_build))]
    let file: Option<TestHooks> = None;
    let file_now = file.as_ref().and_then(|h| h.now);
    let configured_url = hook
        .url
        .or_else(|| file.as_ref().and_then(|h| h.url.clone()));
    #[cfg(ashiato_test_build)]
    let url = configured_url;
    #[cfg(not(ashiato_test_build))]
    let url = Some(GITHUB_API.to_owned());
    (hook.now.or(file_now).unwrap_or_else(Utc::now), url)
}

fn installed_version() -> Result<String, Error> {
    #[cfg(ashiato_test_build)]
    if let Ok(value) = std::env::var("ASHIATO_UPDATER_VERSION") {
        return Ok(value);
    }
    Ok(env!("ASHIATO_VERSION").to_owned())
}

fn append_record(dir: &Path, record: &CheckRecord) -> Result<(), Error> {
    fs::create_dir_all(dir)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(CHECK_FILE))?;
    writeln!(file, "{}", serde_json::to_string(record)?)?;
    Ok(())
}

fn save_status(dir: &Path, status: &Status) -> Result<(), Error> {
    fs::create_dir_all(dir)?;
    let path = dir.join(ashiato_update_check::STATUS_FILE);
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(status)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn request(base: &str, component: Component, installed: &str) -> Result<Vec<u8>, CheckReason> {
    let path = match component {
        Component::Server | Component::CollectorWindows | Component::Android
            if installed.contains("-dev.") =>
        {
            ashiato_update_check::NIGHTLY_RELEASE_PATH
        }
        _ => ashiato_update_check::RELEASES_PATH,
    };
    let initial_url = if base == GITHUB_API && path == ashiato_update_check::RELEASES_PATH {
        RELEASES_URL.to_owned()
    } else {
        format!("{base}{path}")
    };
    let initial_uri: ureq::http::Uri = initial_url.parse().map_err(|_| CheckReason::Unreadable)?;
    let agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(0)
        .build()
        .new_agent();
    let mut url = initial_url;
    let response = 'redirects: {
        for _ in 0..=3 {
            let response = agent
                .get(&url)
                .header("User-Agent", ashiato_update_check::UPDATE_CHECK_USER_AGENT)
                .header("Accept", ashiato_update_check::UPDATE_CHECK_ACCEPT)
                .header("Accept-Encoding", "gzip")
                .call();
            let response = match response {
                Ok(r) => r,
                Err(ureq::Error::StatusCode(403 | 429)) => return Err(CheckReason::RateLimited),
                Err(ureq::Error::StatusCode(404)) => return Err(CheckReason::NotFound),
                Err(ureq::Error::StatusCode(_)) => return Err(CheckReason::Unreadable),
                Err(error) if is_timeout(&error) => return Err(CheckReason::Timeout),
                Err(_) => return Err(CheckReason::Unreachable),
            };
            if !(300..400).contains(&response.status().as_u16()) {
                break 'redirects response;
            }
            let Some(location) = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
            else {
                return Err(CheckReason::Unreadable);
            };
            let next: ureq::http::Uri = location.parse().map_err(|_| CheckReason::Unreadable)?;
            if next.scheme_str() != initial_uri.scheme_str()
                || next.authority() != initial_uri.authority()
            {
                return Err(CheckReason::Unreadable);
            }
            url = location.to_owned();
        }
        return Err(CheckReason::Unreadable);
    };
    response.into_body().read_to_vec().map_err(|error| {
        if is_timeout(&error) {
            CheckReason::Timeout
        } else {
            CheckReason::Unreadable
        }
    })
}

fn is_timeout(error: &ureq::Error) -> bool {
    matches!(error, ureq::Error::Timeout(_))
        || matches!(error, ureq::Error::Io(error) if error.kind() == std::io::ErrorKind::TimedOut)
}

/// 設定が有効で期限なら、一回だけ問い合わせて状態と JSON Lines を更新する。
pub fn once_at(dir: &Path) -> Result<Option<CheckRecord>, Error> {
    let settings = settings(dir)?;
    let (at, base) = now_and_url(dir);
    if !settings.enabled {
        return Ok(None);
    }
    let status_path = dir.join(ashiato_update_check::STATUS_FILE);
    let status = if status_path.is_file() {
        // A partially written status must not permanently stop the hourly service.
        ashiato_update_check::parse_status(&fs::read(&status_path)?).unwrap_or_default()
    } else {
        Status::default()
    };
    if !ashiato_update_check::due(at, status.checked_at) {
        return Ok(None);
    }
    let installed = installed_version()?;
    let component = Component::Server;
    let outcome = match base {
        None => Err(CheckReason::NoDestination),
        Some(base) => request(&base, component, &installed).and_then(|body| {
            ashiato_update_check::parse_releases(&body)
                .map_err(|_| CheckReason::Unreadable)
                .and_then(|releases| {
                    ashiato_update_check::choose(component, &installed, &releases)
                        .map_err(|_| CheckReason::Unreadable)
                        .map(|found| found.latest)
                })
        }),
    };
    let (ok, reason, latest) = match outcome {
        Ok(latest) => (true, None, latest),
        Err(reason) => (false, Some(reason), None),
    };
    let record = CheckRecord {
        at,
        ok,
        reason,
        latest: latest.clone(),
    };
    append_record(dir, &record)?;
    let mut status = status;
    status.checked_at = Some(at);
    status.last_reason = reason;
    if ok {
        if status.latest != latest && latest.is_some() {
            status.found_at = Some(at);
        }
        status.latest = latest;
        status.last_ok_at = Some(at);
    }
    save_status(dir, &status)?;
    Ok(Some(record))
}

/// 現在時刻で一回確認する。
pub fn once(dir: &Path) -> Result<Option<CheckRecord>, Error> {
    once_at(dir)
}

/// 更新係の Windows サービスを起動する。
#[cfg(windows)]
static SERVICE_DIR: OnceLock<PathBuf> = OnceLock::new();
#[cfg(windows)]
static SERVICE_STOP: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
fn run_service() -> u32 {
    let Some(dir) = SERVICE_DIR.get() else {
        return 1;
    };
    let mut next_check = Instant::now();
    while !SERVICE_STOP.load(Ordering::Relaxed) {
        if Instant::now() >= next_check {
            if let Err(error) = once(dir) {
                let _ = log_service_error(dir, &error);
            }
            next_check = Instant::now() + Duration::from_secs(3600);
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    0
}

#[cfg(windows)]
fn log_service_error(dir: &Path, error: &Error) -> Result<(), Error> {
    fs::create_dir_all(dir.join("logs"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("logs/updater.log"))?;
    writeln!(file, "{} {error}", Utc::now().to_rfc3339())?;
    Ok(())
}

/// 配布版の種類に対応する更新係のサービス名を返す。
pub fn service_name(development: bool) -> &'static str {
    if development {
        "ashiato-updater-dev"
    } else {
        "ashiato-updater"
    }
}

/// 配布版の種類に対応する更新係の置き場を返す。
pub fn default_dir(development: bool) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(if development {
            r"C:\ProgramData\ashiato-updater-dev"
        } else {
            r"C:\ProgramData\ashiato-updater"
        })
    }
    #[cfg(not(windows))]
    {
        let _ = development;
        std::env::current_dir()
            .expect("作業場を読めない")
            .join(".ashiato-updater")
    }
}

#[cfg(windows)]
pub fn service(dir: PathBuf, development: bool) -> Result<(), String> {
    let _ = SERVICE_DIR.set(dir);
    ashiato_winsvc::dispatch_named(service_name(development), run_service, || {
        SERVICE_STOP.store(true, Ordering::Relaxed)
    })
    .map_err(|e| e.to_string())
}
