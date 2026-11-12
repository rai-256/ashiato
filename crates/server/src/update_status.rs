// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 更新係が保存した状態を画面向けに読み出す口。

use ashiato_update_check::{
    parse_rfc3339, parse_status, stale, update_available, updater_not_running, CheckReason, Status,
};
use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SETTINGS_FILE: &str = "settings/check.json";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct CheckSettings {
    pub enabled: bool,
    pub since: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UpdateStatus {
    pub state: String,
    pub installed: String,
    pub latest: Option<String>,
    pub found_at: Option<DateTime<Utc>>,
    pub checked_at: Option<DateTime<Utc>>,
    pub last_ok_at: Option<DateTime<Utc>>,
    pub reason: Option<String>,
}

impl UpdateStatus {
    pub(crate) fn empty(installed: &str) -> Self {
        Self {
            state: "not_running".into(),
            installed: installed.into(),
            latest: None,
            found_at: None,
            checked_at: None,
            last_ok_at: None,
            reason: Some("not_running".into()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawCheckSettings {
    enabled: bool,
    since: Option<String>,
}

pub(crate) fn read_check_settings(dir: &Path) -> Result<CheckSettings, String> {
    let bytes = fs::read(dir.join(SETTINGS_FILE)).map_err(|_| "settings_unreadable".to_owned())?;
    if bytes.len() > ashiato_update_check::MAX_RELEASES_JSON_BYTES {
        return Err("settings_unreadable".into());
    }
    let raw: RawCheckSettings =
        serde_json::from_slice(&bytes).map_err(|_| "settings_unreadable".to_owned())?;
    let since = raw
        .since
        .as_deref()
        .map(parse_rfc3339)
        .transpose()
        .map_err(|_| "settings_unreadable".to_owned())?;
    Ok(CheckSettings {
        enabled: raw.enabled,
        since,
    })
}

fn dir_from_env() -> Option<PathBuf> {
    std::env::var_os("ASHIATO_UPDATER_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir())
}

#[utoipa::path(get, path = "/update-status", responses((status = 200, description = "更新確認の状態", body = UpdateStatus)))]
pub async fn get(State(app): State<crate::App>) -> Json<UpdateStatus> {
    let installed = env!("ASHIATO_VERSION");
    Json(dir_from_env().map_or_else(
        || UpdateStatus::empty(installed),
        |dir| read_status(&dir, installed, app.now()),
    ))
}

pub(crate) fn read_status(dir: &Path, installed: &str, now: DateTime<Utc>) -> UpdateStatus {
    match read_check_settings(dir) {
        Ok(settings) => match fs::read(dir.join(ashiato_update_check::STATUS_FILE)) {
            Ok(bytes) if bytes.len() <= ashiato_update_check::MAX_RELEASES_JSON_BYTES => {
                match parse_status(&bytes) {
                    Ok(status) => classify(Some(settings), Some(status), None, installed, now),
                    Err(_) => classify(Some(settings), None, Some("unreadable"), installed, now),
                }
            }
            _ => classify(Some(settings), None, Some("unreadable"), installed, now),
        },
        Err(_) => classify(None, None, Some("unreadable"), installed, now),
    }
}

pub(crate) fn classify(
    settings: Option<CheckSettings>,
    status: Option<Status>,
    invalid_latest: Option<&str>,
    installed: &str,
    now: DateTime<Utc>,
) -> UpdateStatus {
    let Some(settings) = settings else {
        return UpdateStatus::empty(installed);
    };
    if !settings.enabled {
        return base("off", installed, None, None, None, None, None);
    }
    if invalid_latest.is_some() {
        return base("unreadable", installed, None, None, None, None, None);
    }
    let Some(status) = status else {
        let since = settings.since;
        let state = if since
            .is_some_and(|since| now.signed_duration_since(since) >= chrono::Duration::days(7))
        {
            "stale"
        } else {
            "pending"
        };
        let reason = (state == "stale").then_some("not_running".into());
        return base(state, installed, None, None, None, None, reason);
    };
    let updater_stale = updater_not_running(now, status.checked_at);
    let stale_state = stale(now, status.last_ok_at.or(settings.since));
    let available = update_available(installed, status.latest.as_deref()).unwrap_or(false);
    let state = match (available, stale_state) {
        (true, true) => "available_stale",
        (true, false) => "available",
        (false, true) => "stale",
        (false, false) => "latest",
    };
    let reason = if stale_state && updater_stale {
        Some("not_running".into())
    } else {
        status.last_reason.map(reason_name)
    };
    base(
        state,
        installed,
        status.latest,
        status.found_at,
        status.checked_at,
        status.last_ok_at,
        reason,
    )
}

fn base(
    state: &str,
    installed: &str,
    latest: Option<String>,
    found_at: Option<DateTime<Utc>>,
    checked_at: Option<DateTime<Utc>>,
    last_ok_at: Option<DateTime<Utc>>,
    reason: Option<String>,
) -> UpdateStatus {
    UpdateStatus {
        state: state.into(),
        installed: installed.into(),
        latest,
        found_at,
        checked_at,
        last_ok_at,
        reason,
    }
}

fn reason_name(reason: CheckReason) -> String {
    match reason {
        CheckReason::Unreachable => "unreachable",
        CheckReason::Timeout => "timeout",
        CheckReason::RateLimited => "rate_limited",
        CheckReason::NotFound => "not_found",
        CheckReason::Unreadable => "unreadable",
        CheckReason::NoDestination => "no_destination",
    }
    .into()
}
