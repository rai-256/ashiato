// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 更新係が保存した状態を画面向けに読み出す口。

use ashiato_update_check::{
    parse_rfc3339, parse_status, stale, update_available, updater_not_running, CheckReason, Status,
};
use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

const SETTINGS_FILE: &str = "settings/check.json";
const MAX_STATUS_JSON_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadFileError {
    Missing,
    Unreadable,
    TooLarge,
}

fn read_bounded(path: &Path, max_bytes: usize) -> Result<Vec<u8>, ReadFileError> {
    let file = File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ReadFileError::Missing
        } else {
            ReadFileError::Unreadable
        }
    })?;
    if file
        .metadata()
        .map_err(|_| ReadFileError::Unreadable)?
        .len()
        > max_bytes as u64
    {
        return Err(ReadFileError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(max_bytes.min(4096));
    file.take((max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ReadFileError::Unreadable)?;
    if bytes.len() > max_bytes {
        return Err(ReadFileError::TooLarge);
    }
    Ok(bytes)
}

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
    let bytes = read_bounded(
        &dir.join(SETTINGS_FILE),
        ashiato_update_check::MAX_RELEASES_JSON_BYTES,
    )
    .map_err(|error| match error {
        ReadFileError::Missing => "settings_missing",
        ReadFileError::Unreadable | ReadFileError::TooLarge => "settings_unreadable",
    })?;
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

fn updater_dir() -> Option<PathBuf> {
    if std::env::var_os("ASHIATO_SERVICE_MODE").is_some() {
        return Some(crate::service::installed_updater_directory());
    }
    std::env::var_os("ASHIATO_UPDATER_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir())
}

#[utoipa::path(get, path = "/update-status", responses((status = 200, description = "更新確認の状態", body = UpdateStatus)))]
pub async fn get(State(app): State<crate::App>) -> Json<UpdateStatus> {
    let installed = env!("ASHIATO_VERSION");
    Json(updater_dir().map_or_else(
        || UpdateStatus::empty(installed),
        |dir| read_status(&dir, installed, app.now()),
    ))
}

pub(crate) fn read_status(dir: &Path, installed: &str, now: DateTime<Utc>) -> UpdateStatus {
    match read_check_settings(dir) {
        Ok(settings) => match read_bounded(
            &dir.join(ashiato_update_check::STATUS_FILE),
            MAX_STATUS_JSON_BYTES,
        ) {
            Ok(bytes) => match parse_status(&bytes) {
                Ok(status) => classify(Some(settings), Some(status), None, installed, now),
                Err(_) => classify(Some(settings), None, Some("unreadable"), installed, now),
            },
            Err(ReadFileError::Missing) => classify(Some(settings), None, None, installed, now),
            Err(ReadFileError::Unreadable | ReadFileError::TooLarge) => {
                classify(Some(settings), None, Some("unreadable"), installed, now)
            }
        },
        Err(reason) => classify(
            None,
            None,
            (reason != "settings_missing").then_some("unreadable"),
            installed,
            now,
        ),
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
        if invalid_latest.is_some() {
            return base("unreadable", installed, None, None, None, None, None);
        }
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
