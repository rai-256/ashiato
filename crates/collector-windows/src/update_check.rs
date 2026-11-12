//! PC 収集アプリ自身の更新確認。サーバには問い合わせず、配布元の版一覧だけを読む。

use ashiato_update_check::{CheckReason, CheckRecord, Component};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration as StdDuration;

pub const RELEASES_BASE: &str = "https://api.github.com";
const STATUS_FILE: &str = "update-check.json";
const RECORDS_FILE: &str = "update-checks.jsonl";
const SETTINGS_FILE: &str = "settings.json";
const INTRO_TAG: &str = "update-intro";
const UPDATE_TAG: &str = "update";
const STALE_TAG: &str = "update-stale";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub latest: Option<String>,
    pub found_at: Option<DateTime<Utc>>,
    pub checked_at: Option<DateTime<Utc>>,
    pub last_ok_at: Option<DateTime<Utc>>,
    pub last_reason: Option<CheckReason>,
    pub notified_version: Option<String>,
    #[serde(default)]
    pub stale_notified: bool,
    #[serde(default)]
    pub intro_shown: bool,
    pub intro_shown_at: Option<DateTime<Utc>>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self { enabled: true, latest: None, found_at: None, checked_at: None, last_ok_at: None,
            last_reason: None, notified_version: None, stale_notified: false,
            intro_shown: true, intro_shown_at: None }
    }
}

fn default_enabled() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    at: DateTime<Utc>,
    ok: bool,
    reason: Option<CheckReason>,
    latest: Option<String>,
}

pub fn due(now: DateTime<Utc>, last_checked_at: Option<DateTime<Utc>>) -> bool {
    ashiato_update_check::due(now, last_checked_at)
}

pub fn stale(now: DateTime<Utc>, since: Option<DateTime<Utc>>) -> bool {
    ashiato_update_check::stale(now, since)
}

pub trait Toaster: std::fmt::Debug + Send + Sync {
    fn show_update(&self, xml: &str, tag: &str) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct UpdateChecker {
    dir: PathBuf,
    toaster: Option<Arc<dyn Toaster>>,
}

impl UpdateChecker {
    pub fn new(dir: &Path) -> Self { Self { dir: dir.to_owned(), toaster: None } }
    pub fn with_toaster(mut self, toaster: Arc<dyn Toaster>) -> Self { self.toaster = Some(toaster); self }

    pub fn state(&self) -> anyhow::Result<UpdateState> {
        let path = self.dir.join(STATUS_FILE);
        match fs::read(path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(UpdateState { intro_shown: false, ..UpdateState::default() }),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        let path = self.dir.join(SETTINGS_FILE);
        let mut value: serde_json::Value = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => serde_json::json!({}),
        };
        value["update_check_enabled"] = serde_json::Value::Bool(enabled);
        fs::create_dir_all(&self.dir)?;
        crate::fsutil::atomic_write(&path, &serde_json::to_vec_pretty(&value)?)?;
        if enabled && !self.dir.join(STATUS_FILE).exists() {
            let state = UpdateState { intro_shown_at: Some(Utc::now()), ..UpdateState::default() };
            self.save(&state)?;
        }
        Ok(())
    }

    pub fn enabled(&self) -> bool {
        fs::read(self.dir.join(SETTINGS_FILE)).ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|v| v.get("update_check_enabled").and_then(|v| v.as_bool()))
            .unwrap_or(true)
    }

    /// 起動時。切り方をまだ示していない入れ替え直後は、示せるまで確認を止める。
    pub fn start(&self, now: DateTime<Utc>) -> anyhow::Result<Option<CheckRecord>> {
        if !self.enabled() { return Ok(None); }
        let mut state = self.state()?;
        if !state.intro_shown {
            if self.show_intro()? {
                state.intro_shown = true;
                state.intro_shown_at = Some(now);
                self.save(&state)?;
            }
            return Ok(None);
        }
        self.check(now, state)
    }

    /// 定期の確認。起動時に出せなかった案内は、ロックされておらず直近入力時だけ再試行する。
    pub fn periodic(&self, now: DateTime<Utc>, locked: bool, idle_seconds: Option<i64>) -> anyhow::Result<Option<CheckRecord>> {
        if !self.enabled() { return Ok(None); }
        let state = self.state()?;
        if !state.intro_shown {
            if locked || !idle_seconds.is_some_and(|s| (0..=300).contains(&s)) { return Ok(None); }
            if self.show_intro()? {
                let mut shown = state;
                shown.intro_shown = true;
                shown.intro_shown_at = Some(now);
                self.save(&shown)?;
            }
            return Ok(None);
        }
        self.check(now, state)
    }

    fn show_intro(&self) -> anyhow::Result<bool> {
        let Some(toaster) = &self.toaster else { return Ok(false) };
        let xml = toast_xml("更新の確認: 有効", "切るには ashiato-collector-windows.exe update-check off");
        toaster.show_update(&xml, INTRO_TAG).map(|()| true)
    }

    fn check(&self, now: DateTime<Utc>, mut state: UpdateState) -> anyhow::Result<Option<CheckRecord>> {
        if !due(now, state.checked_at) { return Ok(None); }
        let outcome = match update_url() {
            None => Err(CheckReason::NoDestination),
            Some(base) => request(&base).and_then(|body| {
                ashiato_update_check::parse_releases(&body).map_err(|_| CheckReason::Unreadable)
                    .and_then(|releases| ashiato_update_check::choose(Component::CollectorWindows, installed_version(), &releases)
                        .map(|r| r.latest).map_err(|_| CheckReason::Unreadable))
            }),
        };
        let (ok, reason, latest) = match outcome { Ok(v) => (true, None, v), Err(e) => (false, Some(e), None) };
        let record = CheckRecord { at: now, ok, reason, latest: latest.clone() };
        let line = Record { at: now, ok, reason, latest: latest.clone() };
        fs::create_dir_all(&self.dir)?;
        let mut f = OpenOptions::new().create(true).append(true).open(self.dir.join(RECORDS_FILE))?;
        writeln!(f, "{}", serde_json::to_string(&line)?)?;
        state.checked_at = Some(now); state.last_reason = reason;
        if ok { state.latest = latest.clone(); state.last_ok_at = Some(now); state.found_at = Some(now); state.stale_notified = false; }
        if let Some(version) = latest.as_deref().filter(|v| ashiato_update_check::update_available(installed_version(), Some(v)).unwrap_or(false)) {
            if state.notified_version.as_deref() != Some(version)
                && self.show("新しい版", version, UPDATE_TAG)? { state.notified_version = Some(version.to_owned());
            }
        }
        let since = state.intro_shown_at.or(state.last_ok_at);
        if !ok && stale(now, since) && !state.stale_notified {
            let last = state.last_ok_at.map_or_else(|| "—".to_owned(), |v| v.format("%m/%d").to_string());
            if self.show("更新を 7 日確かめられていません", &format!("最後に確かめた {last}・{}", reason_text(reason)), STALE_TAG)? { state.stale_notified = true; }
        }
        self.save(&state)?;
        Ok(Some(record))
    }

    fn show(&self, title: &str, body: &str, tag: &str) -> anyhow::Result<bool> {
        let Some(toaster) = &self.toaster else { return Ok(false) };
        toaster.show_update(&toast_xml(title, body), tag).map(|()| true)
    }

    fn save(&self, state: &UpdateState) -> anyhow::Result<()> {
        fs::create_dir_all(&self.dir)?;
        crate::fsutil::atomic_write(&self.dir.join(STATUS_FILE), &serde_json::to_vec_pretty(state)?)
    }
}

fn reason_text(reason: Option<CheckReason>) -> &'static str {
    match reason { Some(CheckReason::Unreachable) => "配布元に届かない", Some(CheckReason::Timeout) => "時間切れ", Some(CheckReason::RateLimited) => "問い合わせの上限", Some(CheckReason::NotFound) => "配布元が見つからない", Some(CheckReason::Unreadable) => "読めない答え", Some(CheckReason::NoDestination) | None => "配布元が見つからない" }
}

fn installed_version() -> &'static str {
    #[cfg(ashiato_test_build)]
    if let Ok(v) = std::env::var("ASHIATO_COLLECTOR_TEST_VERSION") { return Box::leak(v.into_boxed_str()); }
    env!("ASHIATO_VERSION")
}

fn update_url() -> Option<String> {
    #[cfg(ashiato_test_build)] { std::env::var("ASHIATO_COLLECTOR_TEST_UPDATE_URL").ok().filter(|v| !v.trim().is_empty()) }
    #[cfg(not(ashiato_test_build))] { Some(RELEASES_BASE.to_owned()) }
}

fn request(base: &str) -> Result<Vec<u8>, CheckReason> {
    let url = format!("{base}{}", ashiato_update_check::RELEASES_PATH);
    let agent = ureq::Agent::config_builder().timeout_connect(Some(StdDuration::from_secs(10))).timeout_global(Some(StdDuration::from_secs(30))).build().new_agent();
    let response = agent.get(&url).header("User-Agent", ashiato_update_check::UPDATE_CHECK_USER_AGENT).header("Accept", ashiato_update_check::UPDATE_CHECK_ACCEPT).call().map_err(|e| match e { ureq::Error::StatusCode(403 | 429) => CheckReason::RateLimited, ureq::Error::StatusCode(404) => CheckReason::NotFound, ureq::Error::Timeout(_) => CheckReason::Timeout, _ => CheckReason::Unreachable })?;
    response.into_body().read_to_vec().map_err(|_| CheckReason::Unreadable)
}

pub fn toast_xml(title: &str, body: &str) -> String {
    fn esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") }
    format!(r#"<toast><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual></toast>"#, esc(title), esc(body))
}
