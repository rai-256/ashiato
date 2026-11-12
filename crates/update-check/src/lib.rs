// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services

use chrono::{DateTime, Duration, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};

/// GitHub release に許される入力の最大サイズ。
pub const MAX_RELEASES_JSON_BYTES: usize = 64 * 1024;
/// 正式版・ベータ版の更新確認の問い合わせ先。
pub const RELEASES_PATH: &str = "/repos/rai-256/ashiato/releases?per_page=20";
/// 開発版の更新確認の問い合わせ先。
pub const NIGHTLY_RELEASE_PATH: &str = "/repos/rai-256/ashiato/releases/tags/nightly";
/// 更新確認で許される HTTP 見出しの名前。
pub const UPDATE_CHECK_HEADERS: [&str; 5] = [
    "Host",
    "User-Agent",
    "Accept",
    "Accept-Encoding",
    "Connection",
];
/// 更新確認の User-Agent。
pub const UPDATE_CHECK_USER_AGENT: &str = "ashiato-update-check";
/// GitHub API の応答形式。
pub const UPDATE_CHECK_ACCEPT: &str = "application/vnd.github+json";
/// 状態ファイルの名前。
pub const STATUS_FILE: &str = "status.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Component {
    Server,
    CollectorWindows,
    Android,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub draft: bool,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub latest: Option<String>,
    pub update_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckReason {
    Unreachable,
    Timeout,
    RateLimited,
    NotFound,
    Unreadable,
    NoDestination,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRecord {
    pub at: DateTime<Utc>,
    pub ok: bool,
    pub reason: Option<CheckReason>,
    pub latest: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub latest: Option<String>,
    pub found_at: Option<DateTime<Utc>>,
    pub checked_at: Option<DateTime<Utc>>,
    pub last_ok_at: Option<DateTime<Utc>>,
    pub last_reason: Option<CheckReason>,
    pub notified_version: Option<String>,
    #[serde(default)]
    pub stale_notified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    TooLarge,
    InvalidJson(String),
    InvalidVersion(String),
    InvalidTimestamp(String),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => f.write_str("release JSON が 64 KiB を超えている"),
            Self::InvalidJson(e) => write!(f, "release JSON が読めない: {e}"),
            Self::InvalidVersion(e) => write!(f, "版番号が読めない: {e}"),
            Self::InvalidTimestamp(e) => write!(f, "RFC 3339 が読めない: {e}"),
        }
    }
}

impl std::error::Error for InputError {}

/// 64 KiB 以下の GitHub release JSON を読む。
pub fn parse_releases(input: &[u8]) -> Result<Vec<Release>, InputError> {
    if input.len() > MAX_RELEASES_JSON_BYTES {
        return Err(InputError::TooLarge);
    }
    serde_json::from_slice(input).map_err(|error| InputError::InvalidJson(error.to_string()))
}

/// サイズを制限し、保存した状態を検証しながら読む。
pub fn parse_status(input: &[u8]) -> Result<Status, InputError> {
    if input.len() > MAX_RELEASES_JSON_BYTES {
        return Err(InputError::TooLarge);
    }
    let status: Status = serde_json::from_slice(input)
        .map_err(|error| InputError::InvalidJson(error.to_string()))?;
    validate_optional_version(status.latest.as_deref())?;
    validate_optional_version(status.notified_version.as_deref())?;
    Ok(status)
}

/// サイズを制限し、記録の版と時刻を検証しながら JSON Lines の一行を読む。
pub fn parse_record_line(input: &[u8]) -> Result<CheckRecord, InputError> {
    if input.len() > MAX_RELEASES_JSON_BYTES {
        return Err(InputError::TooLarge);
    }
    let record: CheckRecord = serde_json::from_slice(input)
        .map_err(|error| InputError::InvalidJson(error.to_string()))?;
    validate_optional_version(record.latest.as_deref())?;
    Ok(record)
}

/// RFC 3339 の時刻を読む。
pub fn parse_rfc3339(input: &str) -> Result<DateTime<Utc>, InputError> {
    DateTime::parse_from_rfc3339(input)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| InputError::InvalidTimestamp(error.to_string()))
}

/// 部品のチャンネルに従って、配布物から最新の版と更新の有無を選ぶ。
pub fn choose(
    component: Component,
    installed: &str,
    releases: &[Release],
) -> Result<CheckResult, InputError> {
    let installed_version = parse_version(installed)?;
    let is_dev = is_dev(&installed_version);
    let is_beta = is_beta(&installed_version);
    let latest = releases
        .iter()
        .filter_map(|release| {
            if release.draft
                || (is_dev && release.tag_name != "nightly")
                || (!is_dev && release.tag_name == "nightly")
            {
                return None;
            }
            release
                .assets
                .iter()
                .filter_map(|asset| asset_version(component, &asset.name, is_dev))
                .filter(|version| candidate_matches(version, is_dev, is_beta))
                .max()
        })
        .max();
    let update_available = latest
        .as_ref()
        .is_some_and(|version| version > &installed_version);
    Ok(CheckResult {
        latest: latest.map(|version| version.to_string()),
        update_available,
    })
}

/// 保存済みの最新版本とインストール済み版本を比べ、更新の有無を決める。
pub fn update_available(installed: &str, latest: Option<&str>) -> Result<bool, InputError> {
    let installed = parse_version(installed)?;
    latest
        .map(parse_version)
        .transpose()
        .map(|latest| latest.is_some_and(|latest| latest > installed))
}

/// 最後の確認から 24 時間以上経過したかを返す。未記録と未来の時刻は due とする。
pub fn due(now: DateTime<Utc>, last_checked_at: Option<DateTime<Utc>>) -> bool {
    last_checked_at
        .is_none_or(|last| last > now || now.signed_duration_since(last) >= Duration::hours(24))
}

/// 最後に成功した確認から 7 日以上経過したかを返す。
pub fn stale(now: DateTime<Utc>, last_ok_at_or_since: Option<DateTime<Utc>>) -> bool {
    last_ok_at_or_since
        .is_none_or(|last| last > now || now.signed_duration_since(last) >= Duration::days(7))
}

/// 更新係の確認時刻が 48 時間以上古いかを返す。
pub fn updater_not_running(now: DateTime<Utc>, checked_at: Option<DateTime<Utc>>) -> bool {
    checked_at.is_none_or(|checked| {
        checked > now || now.signed_duration_since(checked) >= Duration::hours(48)
    })
}

fn parse_version(value: &str) -> Result<Version, InputError> {
    Version::parse(value.strip_prefix('v').unwrap_or(value))
        .map_err(|error| InputError::InvalidVersion(error.to_string()))
}

fn validate_optional_version(value: Option<&str>) -> Result<(), InputError> {
    value.map(parse_version).transpose().map(|_| ())
}

fn is_dev(version: &Version) -> bool {
    version.pre.as_str().starts_with("dev.")
}
fn is_beta(version: &Version) -> bool {
    version.pre.as_str().starts_with("beta.")
}
fn version_is_prerelease(version: &Version) -> bool {
    !version.pre.is_empty()
}

fn candidate_matches(version: &Version, dev: bool, beta: bool) -> bool {
    if dev {
        is_dev(version)
    } else if beta {
        is_beta(version) || !version_is_prerelease(version)
    } else {
        !version_is_prerelease(version)
    }
}

fn asset_version(component: Component, name: &str, dev: bool) -> Option<Version> {
    let (prefix, suffix) = match (component, dev) {
        (Component::Server, _) => ("ashiato-server-", "-windows-x86_64.zip"),
        (Component::CollectorWindows, _) => ("ashiato-collector-windows-", "-x86_64.zip"),
        (Component::Android, true) => ("ashiato-collector-dev-", ".apk"),
        (Component::Android, false) => ("ashiato-collector-", ".apk"),
    };
    name.strip_prefix(prefix)
        .and_then(|name| name.strip_suffix(suffix))
        .and_then(|value| parse_version(value).ok())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn release(tag_name: &str, assets: &[&str]) -> Release {
        Release {
            tag_name: tag_name.to_owned(),
            draft: false,
            assets: assets
                .iter()
                .map(|name| Asset {
                    name: (*name).to_owned(),
                })
                .collect(),
        }
    }

    // Scenario: 正式版の部品は、より新しい正式版だけを更新ありにする
    #[test]
    fn st57_choose_official_uses_newer_official_only() {
        let releases = vec![
            release(
                "v0.5.0-beta.1",
                &["ashiato-server-0.5.0-beta.1-windows-x86_64.zip"],
            ),
            release("v0.4.1", &["ashiato-server-0.4.1-windows-x86_64.zip"]),
        ];
        let found = choose(Component::Server, "0.4.0", &releases).unwrap();
        assert_eq!(found.latest.as_deref(), Some("0.4.1"));
        assert!(found.update_available);
    }

    // Scenario: ベータ版の部品は、より新しいベータ版と正式版を更新ありにする
    #[test]
    fn st57_choose_beta_includes_beta_and_official() {
        let releases = vec![
            release(
                "v0.4.0-beta.3",
                &["ashiato-server-0.4.0-beta.3-windows-x86_64.zip"],
            ),
            release("v0.4.0", &["ashiato-server-0.4.0-windows-x86_64.zip"]),
        ];
        let found = choose(Component::Server, "0.4.0-beta.2", &releases).unwrap();
        assert_eq!(found.latest.as_deref(), Some("0.4.0"));
        assert!(found.update_available);
    }

    // Scenario: 正式版を入れた後は、ベータ版を知らせない
    #[test]
    fn st57_choose_official_does_not_include_beta() {
        let found = choose(
            Component::Server,
            "0.4.0",
            &[release(
                "v0.5.0-beta.1",
                &["ashiato-server-0.5.0-beta.1-windows-x86_64.zip"],
            )],
        )
        .unwrap();
        assert_eq!(found.latest, None);
        assert!(!found.update_available);
    }

    // Scenario: ベータ版の部品は、土台の版の上がったベータ版も更新ありにする
    #[test]
    fn st57_choose_beta_includes_new_base_beta() {
        let found = choose(
            Component::Server,
            "0.4.0-beta.2",
            &[release(
                "v0.5.0-beta.1",
                &["ashiato-server-0.5.0-beta.1-windows-x86_64.zip"],
            )],
        )
        .unwrap();
        assert_eq!(found.latest.as_deref(), Some("0.5.0-beta.1"));
    }

    // Scenario: 開発版の部品は、印 nightly の配布物の版だけを見る
    #[test]
    fn st57_choose_dev_uses_nightly_asset() {
        let found = choose(
            Component::Server,
            "0.4.0-dev.20261102",
            &[
                release(
                    "nightly",
                    &["ashiato-server-0.4.0-dev.20261110-windows-x86_64.zip"],
                ),
                release("v0.5.0", &["ashiato-server-0.5.0-windows-x86_64.zip"]),
            ],
        )
        .unwrap();
        assert_eq!(found.latest.as_deref(), Some("0.4.0-dev.20261110"));
    }

    // Scenario: 開発版は土台の版が上がった版を新しいとみなす
    #[test]
    fn st57_choose_dev_compares_base_version() {
        let found = choose(
            Component::Server,
            "0.4.0-dev.20261110",
            &[release(
                "nightly",
                &["ashiato-server-0.5.0-dev.20261101-windows-x86_64.zip"],
            )],
        )
        .unwrap();
        assert!(found.update_available);
    }

    // Scenario: 開発版の入れ替えの途中で 2 つの版が並べば、新しい方を取る
    #[test]
    fn st57_choose_dev_takes_newer_asset_during_replacement() {
        let found = choose(
            Component::Android,
            "0.4.0-dev.20261102",
            &[release(
                "nightly",
                &[
                    "ashiato-collector-dev-0.4.0-dev.20261102.apk",
                    "ashiato-collector-dev-0.4.0-dev.20261110.apk",
                ],
            )],
        )
        .unwrap();
        assert_eq!(found.latest.as_deref(), Some("0.4.0-dev.20261110"));
    }

    // Scenario: 下書きのリリースは数えない
    #[test]
    fn st57_choose_ignores_draft() {
        assert!(choose(
            Component::Server,
            "0.4.0",
            &[Release {
                tag_name: "v0.5.0".into(),
                draft: true,
                assets: vec![Asset {
                    name: "ashiato-server-0.5.0-windows-x86_64.zip".into()
                }]
            }]
        )
        .unwrap()
        .latest
        .is_none());
    }
    // Scenario: 開発版の先行公開をベータ版に数えない
    #[test]
    fn st57_choose_dev_not_beta() {
        assert!(choose(
            Component::Server,
            "0.4.0-beta.2",
            &[release(
                "nightly",
                &["ashiato-server-0.5.0-dev.20261110-windows-x86_64.zip"]
            )]
        )
        .unwrap()
        .latest
        .is_none());
    }
    // Scenario: 自分の部品の配布物が無いリリースは数えない
    #[test]
    fn st57_choose_requires_component_asset() {
        assert!(choose(
            Component::CollectorWindows,
            "0.4.0",
            &[release("v0.5.0", &["other.zip"])]
        )
        .unwrap()
        .latest
        .is_none());
    }
    // Scenario: 自分の版が一覧より新しければ更新なし
    #[test]
    fn st57_choose_newer_installed_is_no_update() {
        assert!(
            !choose(
                Component::Server,
                "0.5.0",
                &[release(
                    "v0.4.0",
                    &["ashiato-server-0.4.0-windows-x86_64.zip"]
                )]
            )
            .unwrap()
            .update_available
        );
    }
    #[test]
    fn st57_choose_empty_is_checked_without_update() {
        let found = choose(Component::Server, "0.4.0", &[]).unwrap();
        assert_eq!(found.latest, None);
        assert!(!found.update_available);
    }
    // Scenario: 新しい版を入れた後は、古い結果でも更新ありにならない
    #[test]
    fn st57_choose_status_is_not_update_after_install() {
        let status = parse_status(
            r#"{"latest":"0.5.0","found_at":"2026-11-11T00:00:00Z","checked_at":"2026-11-11T00:00:00Z","last_ok_at":"2026-11-11T00:00:00Z","last_reason":null,"notified_version":null}"#
                .as_bytes(),
        )
        .unwrap();
        assert!(!update_available("0.5.0", status.latest.as_deref()).unwrap());
    }
    #[test]
    fn st57_choose_all_components_have_asset_rules() {
        for (component, name) in [
            (Component::Server, "ashiato-server-0.4.0-windows-x86_64.zip"),
            (
                Component::CollectorWindows,
                "ashiato-collector-windows-0.4.0-x86_64.zip",
            ),
            (Component::Android, "ashiato-collector-0.4.0.apk"),
        ] {
            assert_eq!(
                choose(component, "0.3.0", &[release("v0.4.0", &[name])])
                    .unwrap()
                    .latest
                    .as_deref(),
                Some("0.4.0")
            );
        }
    }

    #[test]
    fn st57_choose_timing_boundaries() {
        let now = DateTime::parse_from_rfc3339("2026-11-12T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(!due(now, Some(now - Duration::hours(23))));
        assert!(due(now, Some(now - Duration::hours(25))));
        assert!(due(now, Some(now + Duration::days(3))));
        assert!(!stale(now, Some(now - Duration::days(6))));
        assert!(stale(now, Some(now - Duration::days(7))));
        assert!(!stale(now, Some(now - Duration::seconds(7 * 24 * 60 * 60 - 1))));
        assert!(stale(now, Some(now - Duration::days(8))));
        assert!(stale(now, Some(now + Duration::days(365 * 3))));
        assert!(!updater_not_running(now, Some(now - Duration::hours(47))));
        assert!(updater_not_running(now, Some(now - Duration::hours(49))));
        assert!(due(now, None));
        assert!(stale(now, None));
        assert!(updater_not_running(now, None));
    }

    #[test]
    fn st57_choose_rejects_untrusted_input_and_keeps_status_shape() {
        assert_eq!(
            parse_releases(&vec![b' '; MAX_RELEASES_JSON_BYTES + 1]),
            Err(InputError::TooLarge)
        );
        assert!(parse_rfc3339("not-a-time").is_err());
        let status = Status::default();
        let json = serde_json::to_value(status).unwrap();
        assert!(json.get("latest").is_some());
        assert!(json.get("checked_at").is_some());
        assert!(parse_status(br#"{"latest":"not-semver","stale_notified":false}"#).is_err());
        assert!(
            parse_record_line(br#"{"at":"not-a-time","ok":true,"reason":null,"latest":null}"#)
                .is_err()
        );
    }

    #[test]
    fn st57_choose_all_contract_vectors() {
        #[derive(Deserialize)]
        struct VectorFile {
            cases: Vec<VectorCase>,
            timing: Vec<TimingVector>,
        }
        #[derive(Deserialize)]
        struct VectorCase {
            component: Component,
            installed: String,
            releases: Vec<Release>,
            latest: Option<String>,
            update_available: bool,
        }
        #[derive(Deserialize)]
        struct TimingVector {
            kind: String,
            elapsed_hours: Option<i64>,
            elapsed_days: Option<i64>,
            elapsed_seconds: Option<i64>,
            expected: bool,
        }

        let vectors: VectorFile = serde_json::from_str(include_str!(
            "../../../docs/contracts/update-check-vectors.json"
        ))
        .unwrap();
        for vector in vectors.cases {
            let found = choose(vector.component, &vector.installed, &vector.releases).unwrap();
            assert_eq!(found.latest, vector.latest);
            assert_eq!(found.update_available, vector.update_available);
        }
        let now = DateTime::parse_from_rfc3339("2026-11-12T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        for vector in vectors.timing {
            let result = match vector.kind.as_str() {
                "due" => due(
                    now,
                    vector
                        .elapsed_hours
                        .map(|hours| now - Duration::hours(hours)),
                ),
                "stale" | "stale_seconds" => stale(
                    now,
                    vector.elapsed_seconds.map(|seconds| now - Duration::seconds(seconds))
                        .or_else(|| vector.elapsed_days.map(|days| now - Duration::days(days))),
                ),
                "updater_not_running" => updater_not_running(
                    now,
                    vector
                        .elapsed_hours
                        .map(|hours| now - Duration::hours(hours)),
                ),
                other => panic!("未知の時期ベクトル: {other}"),
            };
            assert_eq!(result, vector.expected, "timing vector: {}", vector.kind);
        }
    }
}
