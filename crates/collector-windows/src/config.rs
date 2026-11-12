// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 接続先と資格情報、置き場。**コミットしない**（製造準備 A-2）——
//! 環境変数から読む。
//!
//! 揃っていなければ**送信を始めないが、取得は続ける**（C-01 の `Config` と同じ向き）。
//! 記録は未送信に積まれ、設定してから送られる —— **捨てない**。
use anyhow::Context as _;
use ashiato_build_stamp::Channel;
use serde::{Deserialize, Serialize};

/// 記録に載せるタイムゾーン（FR-20）。**識別子と分の差の両方**を持つ。
///
/// 片方だけでは足りない —— 識別子は夏時間の規則を、分の差はその瞬間の実際の差を表す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    /// IANA のタイムゾーン識別子（`Asia/Tokyo`）
    pub id: String,
    /// UTC との差（分）
    pub offset_min: i32,
}

impl Zone {
    /// いまの PC の設定から読む。
    ///
    /// **識別子が読めないときは落とす。** 既定で `UTC` などを入れると、
    /// 記録が「タイムゾーンを持っている」ように見えて中身が嘘になる（FR-20）。
    pub fn current() -> anyhow::Result<Self> {
        let id = iana_time_zone::get_timezone().context("タイムゾーン識別子が読めない")?;
        let offset_min = chrono::Local::now().offset().local_minus_utc() / 60;
        Ok(Self { id, offset_min })
    }
}

/// 送信先と資格情報。**環境変数から読む**（`ASHIATO_` 接頭辞）。
#[derive(Clone)]
pub struct Config {
    /// 取り込み口の基点（`http://127.0.0.1:18787`）
    pub base_url: String,
    /// 合言葉（PERM-8。**ログにも `Debug` にも出さない**）
    pub api_token: String,
    /// 利用者（FR-29 / PERM-1）
    pub user_id: uuid::Uuid,
    /// どの端末が生成したか（FR-24。`origin=collected` なら必須）
    pub device_id: String,
    /// 未送信・印・数え・除外の登録の置き場
    pub state_dir: std::path::PathBuf,
    pub settings_mtime: Option<std::time::SystemTime>,
    pub credential_mtime: Option<std::time::SystemTime>,
    /// 開発と試験の印（`--config-from-env`）で環境変数から読んだ。送り先は承認したものとして扱い（D13）、
    /// 置き場の設定のファイルを読み直さない（review/code.md R11）
    pub config_from_env: bool,
    /// 除外の登録のファイル。開発版は正式版と共有する（正式版の置き場の `exclusions.json`）。
    /// 置き場の名前が `dev` かどうかでは決めない（`ASHIATO_STATE_DIR=C:\data\dev` の正式版が親を読む。review/code.md R42）
    pub exclusions_path: std::path::PathBuf,
    /// 起動に渡された置き場（開発版は `dev` を付ける前）。自動起動の台本へはこれを書く
    pub launch_state_dir: std::path::PathBuf,
    /// 起動に渡された端末識別子（開発版は `-dev` を付ける前）。自動起動の台本へはこれを書く
    pub launch_device_id: String,
}

/// **合言葉を出さない**（R11）。
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("base_url", &self.base_url)
            .field("api_token", &"***")
            .field("user_id", &self.user_id)
            .field("device_id", &self.device_id)
            .field("state_dir", &self.state_dir)
            .field("config_from_env", &self.config_from_env)
            .finish()
    }
}

/// 読む変数の名前。**自動起動の `.cmd` もこの一覧から書く**（R3）。
pub const VARS: [&str; 5] = [
    "ASHIATO_BASE_URL",
    "ASHIATO_API_TOKEN",
    "ASHIATO_USER_ID",
    "ASHIATO_DEVICE_ID",
    "ASHIATO_STATE_DIR",
];

/// **loopback 以外へ平文で送らない**（ST28 design D13）。`http://` で host が
/// `127.0.0.1` / `::1` / `localhost` でなければ断る。`https://` はどの host でも通す。
fn ensure_no_cleartext_off_loopback(base_url: &str) -> anyhow::Result<()> {
    let url = base_url.trim();
    let Some((scheme, rest)) = url.split_once("://") else {
        return Ok(());
    };
    if !scheme.eq_ignore_ascii_case("http") {
        return Ok(());
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let hostport = authority.rsplit('@').next().unwrap_or("");
    let host = match hostport.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => hostport.split(':').next().unwrap_or(""),
    };
    anyhow::ensure!(
        matches!(host.to_ascii_lowercase().as_str(), "127.0.0.1" | "::1" | "localhost"),
        "ASHIATO_BASE_URL が http:// で、接続先が暗号化されていない（loopback 以外へ平文で送らない）。https:// にする"
    );
    Ok(())
}

impl Config {
    /// 起動の印に応じて環境変数または利用者の置き場から読む。系統は入口が版番号から 1 回だけ決めて渡す（ST54 D1）。
    pub fn from_launch(args: &[String], channel: Channel) -> anyhow::Result<Self> {
        let script = crate::autostart::read_startup_script(channel)?;
        Self::from_sources(
            args,
            |key| std::env::var(key).ok(),
            script.as_deref(),
            channel,
        )
    }

    /// 設定源を差し替えて、置き場と端末識別子の規則を検証する。
    pub fn from_sources(
        args: &[String],
        get: impl Fn(&str) -> Option<String>,
        startup_script: Option<&str>,
        channel: Channel,
    ) -> anyhow::Result<Self> {
        let marked = args.iter().any(|arg| arg == "--config-from-env")
            || get("ASHIATO_CONFIG_FROM_ENV").as_deref() == Some("1");
        if marked {
            return Self::from_lookup(get, channel);
        }

        let script_vars = startup_script
            .map(crate::autostart::read_vars)
            .unwrap_or_default();
        let script_value = |key: &str| {
            script_vars
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        };
        let state_dir = get("ASHIATO_STATE_DIR")
            .filter(|value| !value.trim().is_empty())
            .or_else(|| script_value("ASHIATO_STATE_DIR"))
            .map(std::path::PathBuf::from)
            .or_else(|| {
                get("LOCALAPPDATA")
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| std::path::PathBuf::from(value).join("ashiato/collector"))
            })
            .with_context(|| {
                "LOCALAPPDATA が読めず、ASHIATO_STATE_DIR も無いため置き場を決められない"
            })?;
        anyhow::ensure!(
            state_dir.is_absolute(),
            "収集アプリの置き場は絶対パスでなければならない"
        );

        let is_dev = channel == Channel::Dev;
        let release_dir = state_dir.clone();
        let actual_dir = if is_dev {
            state_dir.join("dev")
        } else {
            state_dir
        };
        std::fs::create_dir_all(&actual_dir)
            .with_context(|| format!("収集アプリの置き場を作れない: {}", actual_dir.display()))?;

        let settings = read_settings_or_default(&actual_dir)?;
        // ST56 の途中の形の settings.json が持つ平文の資格情報は、settings.json を書き直す前に
        // credential.bin へ移す（書き直しで消え、次の読み直しで資格情報が空になる。review/code.md R35）
        if let Some(token) = settings.legacy_api_token.as_deref() {
            if read_credential(&actual_dir)?.is_none() {
                write_credential(
                    &actual_dir,
                    settings.base_url.as_deref().unwrap_or_default(),
                    token,
                )?;
            }
        }
        let launch_state_dir = release_dir.clone();
        let script_device = script_value("ASHIATO_DEVICE_ID");
        let root_device = if is_dev {
            read_settings_or_default(&release_dir)?.device_id
        } else {
            None
        };
        let has_root_device = root_device.is_some();
        let device_id = get("ASHIATO_DEVICE_ID")
            .filter(|value| !value.trim().is_empty())
            .or(script_device)
            .or(settings.device_id)
            .or(root_device)
            .unwrap_or_else(generate_device_id);
        let launch_device_id = device_id.clone();
        let device_id = if is_dev && !device_id.ends_with("-dev") {
            format!("{device_id}-dev")
        } else {
            device_id
        };
        if !is_dev {
            let settings = SettingsFile {
                base_url: settings.base_url.clone(),
                device_id: Some(device_id.clone()),
                update_check_enabled: settings.update_check_enabled,
                legacy_api_token: None,
            };
            write_settings_if_needed(&release_dir, &settings)?;
        } else if !has_root_device {
            let root_id = device_id.strip_suffix("-dev").unwrap_or(&device_id);
            write_settings_if_needed(
                &release_dir,
                &SettingsFile {
                    base_url: read_settings_or_default(&release_dir)?.base_url,
                    device_id: Some(root_id.to_string()),
                    update_check_enabled: read_settings_or_default(&release_dir)?.update_check_enabled,
                    legacy_api_token: None,
                },
            )?;
        }

        let base_url = settings.base_url.unwrap_or_default();
        // 別の送り先に結んだ資格情報は使わない（`setup` の書きかけで落ちた。設定し直すまで未設定。R34）
        let api_token = token_for(read_credential(&actual_dir)?, &base_url)
            .ok()
            .flatten()
            .unwrap_or_default();
        if !base_url.is_empty() {
            ensure_no_cleartext_off_loopback(&base_url)?;
        }
        Ok(Self {
            base_url,
            api_token,
            user_id: uuid::Uuid::nil(),
            device_id,
            state_dir: actual_dir.clone(),
            settings_mtime: settings_mtime(&actual_dir),
            credential_mtime: credential_mtime(&actual_dir),
            config_from_env: false,
            exclusions_path: if is_dev { release_dir } else { actual_dir }.join("exclusions.json"),
            launch_state_dir,
            launch_device_id,
        })
    }

    /// 送り先と資格情報がまだ無く、設定の入力を促す状態かを返す。
    pub fn needs_setup(&self) -> bool {
        self.base_url.is_empty() || self.api_token.is_empty()
    }

    /// 別プロセスの `setup` が保存した値を、次の送信契機で読み直す。
    pub fn reload_from_disk(&mut self) -> anyhow::Result<bool> {
        // 印で起動したら環境変数の値だけを使う（spec「置き場の設定のファイルを読まない」。R11）
        if self.config_from_env {
            return Ok(false);
        }
        let current_mtime = settings_mtime(&self.state_dir);
        let current_credential_mtime = credential_mtime(&self.state_dir);
        // 見回りのたびに読み直して DPAPI で戻さない。どちらのファイルも変わっていなければそのまま（review/code.md R38）
        if current_mtime == self.settings_mtime && current_credential_mtime == self.credential_mtime
        {
            return Ok(false);
        }
        let settings = read_settings_or_default(&self.state_dir)?;
        let base_url = settings.base_url.unwrap_or_default();
        // `setup` が settings.json と credential.bin を書く間に読むと、新しい送り先と古い資格情報が揃う。
        // 資格情報が別の送り先に結ばれていたら今回は読み直さない（次の見回りで揃ったものを読む。review/code.md R34）
        let Ok(token) = token_for(read_credential(&self.state_dir)?, &base_url) else {
            return Ok(false);
        };
        let new_token = token.unwrap_or_default();
        let changed = self.base_url != base_url || self.api_token != new_token;
        ensure_no_cleartext_off_loopback(&base_url)?;
        self.base_url = base_url;
        self.api_token = new_token;
        self.settings_mtime = current_mtime;
        self.credential_mtime = current_credential_mtime;
        Ok(changed)
    }

    /// 画面で作った設定の文字列を保存する。入力は保存前に完全に解釈する。
    pub fn save_setup_string(&self, input: &str) -> anyhow::Result<()> {
        self.save_setup_string_at(input, chrono::Utc::now())
    }

    /// [`Self::save_setup_string`] の時刻を渡せる形（試験が「保存から 24 時間」を数える）。
    ///
    /// **門の状態（setup-state.json）を先に書く**（review/code.md R13）—— 送り先を先に書くと、その間に
    /// 収集が読み直したとき門の状態が無いまま開いて、新しい送り先へ未送信を送る。
    /// 保存し直すと、断られ・届かないの数えをやめ、「届かない」の 24 時間はこの時刻から数える（R10）。
    pub fn save_setup_string_at(
        &self,
        input: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<()> {
        let setup =
            crate::setup_string::parse_setup_string(input).context("設定の文字列を解釈できない")?;
        ensure_no_cleartext_off_loopback(&setup.url)?;
        let store = crate::destination_gate::Store::new(&self.state_dir);
        let mut state = store.load()?;
        let pending = crate::destination_gate::pending_in(&self.state_dir);
        match crate::destination_gate::gate(state.approved_url.as_deref(), &setup.url, pending) {
            crate::destination_gate::GateAction::Send => {
                state.approved_url = Some(setup.url.clone());
                state.destination_notified_at = None;
                state.destination_notice_at = None;
            }
            // 答えずに窓を閉じても保留の知らせが出るよう、ここで保留に入れる（y で解く。R2）
            // 新しい保留はすぐ知らせる（前の保留の知らせの 24 時間を引き継がない。review/code.md R37）
            _ => {
                state.destination_notified_at.get_or_insert(now);
                state.destination_notice_at = None;
            }
        }
        state.setup_saved_at = Some(now);
        state.auth_since = None;
        state.auth_notified_at = None;
        state.first_response_at = None;
        state.unreachable_since = None;
        state.unreachable_notified_at = None;
        store.save(&state)?;
        // 資格情報は送り先に結んで包む。読む側は結んだ送り先と settings.json の送り先が揃うまで使わない（R34）
        write_credential(&self.state_dir, &setup.url, &setup.token)?;
        write_settings_if_needed(
            &self.state_dir,
            &SettingsFile {
                base_url: Some(setup.url.clone()),
                device_id: Some(self.device_id.clone()),
                update_check_enabled: read_settings_or_default(&self.state_dir)?.update_check_enabled,
                legacy_api_token: None,
            },
        )?;
        Ok(())
    }

    pub fn settings_mtime(&self) -> Option<std::time::SystemTime> {
        settings_mtime(&self.state_dir)
    }

    /// 名前から値を引く関数で読む。**1 つでも欠けていればエラー**にする ——
    /// 既定で埋めると、送り先を間違えたまま動く。
    ///
    /// **置き場にも既定を持たない**（R27）。`.` に倒すと、起動のしかた次第で
    /// 置き場が `C:\Windows\System32` などに移り、前の置き場の未送信・印・除外の登録が
    /// 黙って置き去りになる（**除外が外れ、直前の停止期間が「初回起動」に化ける**）。
    pub fn from_lookup(
        get: impl Fn(&str) -> Option<String>,
        channel: Channel,
    ) -> anyhow::Result<Self> {
        let var = |k: &str| {
            get(k)
                .filter(|v| !v.trim().is_empty())
                .with_context(|| format!("{k} が無い"))
        };
        let user_id = var("ASHIATO_USER_ID")?;
        let state_dir = std::path::PathBuf::from(var("ASHIATO_STATE_DIR")?);
        anyhow::ensure!(
            state_dir.is_absolute(),
            "ASHIATO_STATE_DIR は絶対パスで書く（相対だと起動のしかたで置き場が変わる）"
        );
        let base_url = var("ASHIATO_BASE_URL")?;
        ensure_no_cleartext_off_loopback(&base_url)?;
        let device_id = var("ASHIATO_DEVICE_ID")?;
        let launch_state_dir = state_dir.clone();
        let launch_device_id = device_id.clone();
        let (state_dir, exclusions_path, device_id) = match channel {
            Channel::Dev => {
                let state_dir = state_dir.join("dev");
                let device_id = if device_id.ends_with("-dev") {
                    device_id
                } else {
                    format!("{device_id}-dev")
                };
                (
                    state_dir,
                    launch_state_dir.join("exclusions.json"),
                    device_id,
                )
            }
            Channel::Official => {
                let exclusions_path = state_dir.join("exclusions.json");
                (state_dir, exclusions_path, device_id)
            }
        };
        Ok(Self {
            base_url,
            api_token: var("ASHIATO_API_TOKEN")?,
            user_id: uuid::Uuid::parse_str(user_id.trim())
                .context("ASHIATO_USER_ID が uuid ではない")?,
            device_id,
            settings_mtime: settings_mtime(&state_dir),
            credential_mtime: credential_mtime(&state_dir),
            config_from_env: true,
            state_dir,
            exclusions_path,
            launch_state_dir,
            launch_device_id,
        })
    }

    #[cfg(test)]
    fn from_lookup_official(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        Self::from_lookup(get, Channel::Official)
    }

    /// 変数の名前と値の組（自動起動の `.cmd` に書く。R3）。
    pub fn as_vars(&self) -> [(&'static str, String); 5] {
        [
            (VARS[0], self.base_url.clone()),
            (VARS[1], self.api_token.clone()),
            (VARS[2], self.user_id.to_string()),
            (VARS[3], self.launch_device_id.clone()),
            (VARS[4], self.launch_state_dir.display().to_string()),
        ]
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SettingsFile {
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    device_id: Option<String>,
    #[serde(default)]
    update_check_enabled: Option<bool>,
    /// ST56 より前の settings.json の読み取りだけを互換のため許す。
    #[serde(default, rename = "api_token", skip_serializing)]
    legacy_api_token: Option<String>,
}

/// `credential.bin` の中身（DPAPI で包む）。資格情報を、それを貼った送り先に結ぶ（review/code.md R34）。
#[derive(Serialize, Deserialize)]
struct CredentialBlob {
    url: String,
    token: String,
}

/// 読んだ資格情報。`url` が無いのは ST56 の途中の形（資格情報だけを包んだもの）。
struct Credential {
    url: Option<String>,
    token: String,
}

fn read_credential(dir: &std::path::Path) -> anyhow::Result<Option<Credential>> {
    let path = dir.join("credential.bin");
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let Ok(value) = crate::secret::unprotect(&bytes) else {
        return Ok(None);
    };
    if let Ok(blob) = serde_json::from_slice::<CredentialBlob>(&value) {
        return Ok(Some(Credential {
            url: Some(blob.url),
            token: blob.token,
        }));
    }
    let Ok(token) = String::from_utf8(value) else {
        return Ok(None);
    };
    Ok(Some(Credential { url: None, token }))
}

fn write_credential(dir: &std::path::Path, url: &str, token: &str) -> anyhow::Result<()> {
    let blob = serde_json::to_vec(&CredentialBlob {
        url: url.to_owned(),
        token: token.to_owned(),
    })?;
    crate::fsutil::atomic_write(&dir.join("credential.bin"), &crate::secret::protect(&blob)?)
}

/// 送り先 `base_url` で使ってよい資格情報。別の送り先に結ばれていれば `Err(())`（書きかけ）。
#[allow(clippy::result_unit_err)]
fn token_for(credential: Option<Credential>, base_url: &str) -> Result<Option<String>, ()> {
    match credential {
        None => Ok(None),
        Some(Credential { url: Some(url), .. })
            if crate::destination_gate::canonical_url(&url)
                != crate::destination_gate::canonical_url(base_url) =>
        {
            Err(())
        }
        Some(credential) => Ok(Some(credential.token)),
    }
}

fn read_settings(dir: &std::path::Path) -> anyhow::Result<SettingsFile> {
    let bytes = std::fs::read(dir.join("settings.json"))?;
    serde_json::from_slice(&bytes).context("settings.json が読めない")
}

fn read_settings_or_default(dir: &std::path::Path) -> anyhow::Result<SettingsFile> {
    match read_settings(dir) {
        Ok(settings) => Ok(settings),
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            Ok(SettingsFile::default())
        }
        Err(error) => Err(error),
    }
}

fn settings_mtime(dir: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(dir.join("settings.json"))
        .ok()?
        .modified()
        .ok()
}

fn credential_mtime(dir: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(dir.join("credential.bin"))
        .ok()?
        .modified()
        .ok()
}

fn write_settings_if_needed(dir: &std::path::Path, settings: &SettingsFile) -> anyhow::Result<()> {
    let path = dir.join("settings.json");
    let bytes = serde_json::to_vec_pretty(settings)?;
    if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
        crate::fsutil::atomic_write(&path, &bytes)?;
    }
    Ok(())
}

fn generate_device_id() -> String {
    let hex = uuid::Uuid::new_v4().simple().to_string();
    format!("pc-{}", &hex[..12])
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::collections::HashMap;

    /// タイムゾーンは**識別子と差の両方**を持つ（FR-20）。
    #[test]
    fn zone_has_both_id_and_offset() {
        let z = Zone::current().expect("この PC のタイムゾーン");
        assert!(!z.id.is_empty());
        assert!(
            (-24 * 60..=24 * 60).contains(&z.offset_min),
            "差が時刻として成り立たない: {}",
            z.offset_min
        );
    }

    fn abs_dir() -> (tempfile::TempDir, String) {
        let tmp = tempfile::Builder::new()
            .prefix("ashiato-cfg-")
            .tempdir()
            .expect("tempdir");
        let dir = tmp.path().join("ashiato").display().to_string();
        (tmp, dir)
    }

    /// **置き場を含む 5 つが揃わなければ読まない**（R27）。相対パスの置き場も断る。
    #[test]
    fn config_requires_every_var_including_state_dir() {
        let (_tmp, dir) = abs_dir();
        let full = |k: &str| {
            Some(match k {
                "ASHIATO_BASE_URL" => "http://127.0.0.1:8787".to_string(),
                "ASHIATO_API_TOKEN" => "t0123456789".to_string(),
                "ASHIATO_USER_ID" => uuid::Uuid::nil().to_string(),
                "ASHIATO_DEVICE_ID" => "pc-01".to_string(),
                "ASHIATO_STATE_DIR" => dir.clone(),
                _ => return None,
            })
        };
        assert!(Config::from_lookup_official(full).is_ok());
        for missing in VARS {
            let got = Config::from_lookup_official(|k| if k == missing { None } else { full(k) });
            assert!(got.is_err(), "{missing} が無いのに読めた");
        }
        let relative = Config::from_lookup_official(|k| {
            if k == "ASHIATO_STATE_DIR" {
                Some(".".into())
            } else {
                full(k)
            }
        });
        assert!(relative.is_err(), "相対パスの置き場を受けた");
    }

    fn env(dir: &str, device: &str) -> impl Fn(&str) -> Option<String> {
        let dir = dir.to_owned();
        let device = device.to_owned();
        move |k| {
            Some(match k {
                "ASHIATO_BASE_URL" => "http://127.0.0.1:8787".into(),
                "ASHIATO_API_TOKEN" => "token".into(),
                "ASHIATO_USER_ID" => uuid::Uuid::nil().to_string(),
                "ASHIATO_DEVICE_ID" => device.clone(),
                "ASHIATO_STATE_DIR" => dir.clone(),
                _ => return None,
            })
        }
    }

    #[test]
    fn st54_dev_config_separates_state_and_keeps_exclusions_at_parent() {
        // Scenario: 開発版は、渡された置き場の下の dev を置き場にする
        let tmp = tempfile::tempdir().unwrap();
        let cfg =
            Config::from_lookup(env(tmp.path().to_str().unwrap(), "pc-1"), Channel::Dev).unwrap();
        assert_eq!(cfg.state_dir, tmp.path().join("dev"));
        assert_eq!(cfg.exclusions_path, tmp.path().join("exclusions.json"));
        assert_eq!(cfg.device_id, "pc-1-dev");
    }

    #[test]
    fn st54_dev_config_does_not_duplicate_device_suffix() {
        // Scenario: すでに -dev で終わる端末識別子には、重ねて付けない
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config::from_lookup(env(tmp.path().to_str().unwrap(), "pc-1-dev"), Channel::Dev)
            .unwrap();
        assert_eq!(cfg.device_id, "pc-1-dev");
    }

    #[test]
    fn st54_official_config_keeps_values() {
        // Scenario: 正式版は、渡された置き場と端末識別子をそのまま使う
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config::from_lookup(env(tmp.path().to_str().unwrap(), "pc-1"), Channel::Official)
            .unwrap();
        assert_eq!(cfg.state_dir, tmp.path());
        assert_eq!(cfg.device_id, "pc-1");
        assert_eq!(cfg.exclusions_path, tmp.path().join("exclusions.json"));
    }

    #[test]
    fn st54_dev_autostart_vars_are_original_values() {
        // Scenario: 開発版の自動起動の項目は、渡された置き場と端末識別子をそのまま書く
        let tmp = tempfile::tempdir().unwrap();
        let cfg =
            Config::from_lookup(env(tmp.path().to_str().unwrap(), "pc-1"), Channel::Dev).unwrap();
        let vars = cfg.as_vars();
        assert_eq!(vars[3].1, "pc-1");
        assert_eq!(vars[4].1, tmp.path().display().to_string());
    }

    #[test]
    fn st54_channel_environment_variable_is_not_read() {
        // Scenario: 系統の名前の環境変数を渡しても、系統は変わらない
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config::from_lookup(
            |k| {
                if k == "ASHIATO_CHANNEL" {
                    Some("official".into())
                } else {
                    env(tmp.path().to_str().unwrap(), "pc-1")(k)
                }
            },
            Channel::Dev,
        )
        .unwrap();
        assert_eq!(cfg.device_id, "pc-1-dev");
    }

    #[test]
    fn st54_official_default_exclusion_path_is_under_state() {
        // Scenario: 版番号を渡さずに組んだ部品は正式版として振る舞う
        let tmp = tempfile::tempdir().unwrap();
        let version_channel = ashiato_build_stamp::channel(env!("ASHIATO_VERSION"));
        assert_eq!(version_channel, Channel::Official);
        assert_eq!(
            crate::autostart::entry_name(version_channel),
            "ashiato-collector.cmd"
        );
        assert_eq!(
            crate::toast_names::aumid(version_channel),
            "dev.ashiato.collector"
        );
        assert_eq!(crate::toast_names::display_name(version_channel), "ashiato");
        let cfg = Config::from_lookup(env(tmp.path().to_str().unwrap(), "pc-1"), version_channel)
            .unwrap();
        assert_eq!(cfg.exclusions_path, cfg.state_dir.join("exclusions.json"));
    }

    #[test]
    fn st54_dev_exclusion_path_is_not_under_dev_state() {
        // Scenario: 開発版は、渡された置き場の除外の登録を読む
        let tmp = tempfile::tempdir().unwrap();
        let cfg =
            Config::from_lookup(env(tmp.path().to_str().unwrap(), "pc-1"), Channel::Dev).unwrap();
        assert_eq!(cfg.exclusions_path, tmp.path().join("exclusions.json"));
        assert_ne!(cfg.exclusions_path, cfg.state_dir.join("exclusions.json"));
    }

    #[test]
    fn st54_dev_state_dir_differs_from_official_state_dir() {
        // Scenario: 正式版と同じ置き場を渡しても、開発版と正式版は両方起動する
        let tmp = tempfile::tempdir().unwrap();
        let official =
            Config::from_lookup(env(tmp.path().to_str().unwrap(), "pc-1"), Channel::Official)
                .unwrap();
        let dev =
            Config::from_lookup(env(tmp.path().to_str().unwrap(), "pc-1"), Channel::Dev).unwrap();
        let official_marker = crate::marker::Marker::new(&official.state_dir);
        let dev_marker = crate::marker::Marker::new(&dev.state_dir);
        official_marker.touch(chrono::Utc::now()).unwrap();
        dev_marker.touch(chrono::Utc::now()).unwrap();
        assert!(official_marker.read().unwrap().is_some());
        assert!(dev_marker.read().unwrap().is_some());
        let official_outbox = official.state_dir.join("outbox.jsonl");
        let dev_outbox = dev.state_dir.join("outbox.jsonl");
        assert_ne!(official_outbox, dev_outbox);
    }

    /// `Debug` に合言葉が出ない（R11）。
    #[test]
    fn debug_hides_the_token() {
        let (_tmp, dir) = abs_dir();
        let c = Config {
            base_url: "http://x".into(),
            api_token: "SUPER-SECRET-TOKEN".into(),
            user_id: uuid::Uuid::nil(),
            device_id: "pc".into(),
            state_dir: std::path::PathBuf::from(dir),
            settings_mtime: None,
            credential_mtime: None,
            config_from_env: false,
            exclusions_path: std::path::PathBuf::from("exclusions.json"),
            launch_state_dir: std::path::PathBuf::from("state"),
            launch_device_id: "pc".into(),
        };
        assert!(!format!("{c:?}").contains("SUPER-SECRET"));
    }

    fn env_with_base(base: &'static str, dir: String) -> impl Fn(&str) -> Option<String> {
        move |k| {
            Some(match k {
                "ASHIATO_BASE_URL" => base.to_string(),
                "ASHIATO_API_TOKEN" => "t0123456789".to_string(),
                "ASHIATO_USER_ID" => uuid::Uuid::nil().to_string(),
                "ASHIATO_DEVICE_ID" => "pc-01".to_string(),
                "ASHIATO_STATE_DIR" => dir.clone(),
                _ => return None,
            })
        }
    }

    // Scenario: 平文の接続先では PC の収集器が起動しない
    #[test]
    fn base_url_cleartext_off_loopback_is_refused() {
        let (_tmp, dir) = abs_dir();
        for bad in [
            "http://example.invalid:8787",
            "http://example.invalid",
            "http://example.test:8787/x",
            "HTTP://s01.lan",
            "http://127.0.0.1@evil.example",
        ] {
            let e = Config::from_lookup_official(env_with_base(bad, dir.clone())).expect_err(bad);
            assert!(e.to_string().contains("暗号化されていない"), "{bad}: {e}");
        }
    }

    #[test]
    fn base_url_cleartext_loopback_and_https_are_accepted() {
        let (_tmp, dir) = abs_dir();
        for ok in [
            "http://127.0.0.1:8787",
            "http://[::1]:8787",
            "http://localhost:8787/",
            "https://example.invalid",
            "https://example.test",
        ] {
            assert!(
                Config::from_lookup_official(env_with_base(ok, dir.clone())).is_ok(),
                "{ok}"
            );
        }
    }

    /// 置き場の設定と資格情報の更新時刻を、前に見た値より確実に後へずらす。Linux の更新時刻は粗い時計
    /// （数 ms 刻み）で付くので、続けて書くと同じ時刻になり、別のプロセスの書き込みを「変わっていない」と読む。
    fn advance_mtimes(dir: &std::path::Path) {
        static STEP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let step = STEP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let at = std::time::SystemTime::now() + std::time::Duration::from_secs(60 * step);
        for name in ["settings.json", "credential.bin"] {
            if let Ok(file) = std::fs::OpenOptions::new().write(true).open(dir.join(name)) {
                file.set_modified(at).unwrap();
            }
        }
    }

    fn launch_env(values: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let values: HashMap<String, String> = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        move |key| values.get(key).cloned()
    }

    // Scenario: 既定の置き場を使う
    #[test]
    fn st56_config_uses_default_state_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let env = launch_env(&[(
            "LOCALAPPDATA",
            tmp.path().to_str().expect("utf-8 temp path"),
        )]);
        let cfg = Config::from_sources(&[], env, None, Channel::Official).expect("config");
        assert_eq!(cfg.state_dir, tmp.path().join("ashiato/collector"));
    }

    // Scenario: 前の形の台本の置き場と端末の識別子を引き継ぐ
    #[test]
    fn st56_config_inherits_state_and_device_from_startup_script() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = tmp.path().join("old-state");
        let script = format!(
            "set \"ASHIATO_STATE_DIR={}\"\r\nset \"ASHIATO_DEVICE_ID=pc-01\"\r\n",
            state.display()
        );
        let cfg = Config::from_sources(&[], launch_env(&[]), Some(&script), Channel::Official)
            .expect("config");
        assert_eq!(cfg.state_dir, state);
        assert_eq!(cfg.device_id, "pc-01");
    }

    // Scenario: 端末の識別子は最初の起動で作って覚える
    #[test]
    fn st56_config_persists_generated_device_id() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let env = launch_env(&[(
            "LOCALAPPDATA",
            tmp.path().to_str().expect("utf-8 temp path"),
        )]);
        let first = Config::from_sources(&[], &env, None, Channel::Official).expect("config");
        let second = Config::from_sources(&[], &env, None, Channel::Official).expect("config");
        assert_eq!(first.device_id, second.device_id);
        assert!(first.device_id.starts_with("pc-"));
        assert_eq!(first.device_id.len(), 15);
        assert!(first.device_id[3..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    // Scenario: 開発版は既定の置き場の下の dev と、末尾に -dev の識別子を使う
    #[test]
    fn st56_config_separates_dev_state_but_reuses_release_device_id() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("ashiato/collector");
        std::fs::create_dir_all(&root).expect("root");
        std::fs::write(
            root.join("settings.json"),
            r#"{"device_id":"pc-0123456789ab"}"#,
        )
        .expect("settings");
        let cfg = Config::from_sources(
            &[],
            launch_env(&[(
                "LOCALAPPDATA",
                tmp.path().to_str().expect("utf-8 temp path"),
            )]),
            None,
            Channel::Dev,
        )
        .expect("config");
        assert_eq!(cfg.state_dir, root.join("dev"));
        assert_eq!(cfg.device_id, "pc-0123456789ab-dev");
        assert!(!cfg.state_dir.join("settings.json").exists());
    }

    // Scenario: 印を付けると環境変数で動く
    #[test]
    fn st56_config_marker_uses_all_environment_values() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let values = [
            ("ASHIATO_BASE_URL", "https://example.test/api"),
            ("ASHIATO_API_TOKEN", "token-for-test-0123456789"),
            ("ASHIATO_USER_ID", "00000000-0000-0000-0000-000000000000"),
            ("ASHIATO_DEVICE_ID", "pc-env"),
            (
                "ASHIATO_STATE_DIR",
                tmp.path().to_str().expect("utf-8 temp path"),
            ),
        ];
        let cfg = Config::from_sources(
            &["--config-from-env".into()],
            launch_env(&values),
            None,
            Channel::Official,
        )
        .expect("config");
        assert_eq!(cfg.base_url, "https://example.test/api");
        assert_eq!(cfg.api_token, "token-for-test-0123456789");
        assert_eq!(cfg.device_id, "pc-env");
        assert_eq!(cfg.state_dir, tmp.path());
    }

    #[test]
    fn st56_config_without_marker_ignores_destination_credentials() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let values = [
            ("ASHIATO_BASE_URL", "https://old.example.test/api"),
            ("ASHIATO_API_TOKEN", "old-token"),
            ("ASHIATO_USER_ID", "11111111-1111-1111-1111-111111111111"),
            (
                "LOCALAPPDATA",
                tmp.path().to_str().expect("utf-8 temp path"),
            ),
        ];
        let state = tmp.path().join("ashiato/collector");
        std::fs::create_dir_all(&state).expect("state");
        std::fs::write(
            state.join("settings.json"),
            r#"{"base_url":"https://new.example.test/api"}"#,
        )
        .expect("settings");
        let cfg = Config::from_sources(&[], launch_env(&values), None, Channel::Official)
            .expect("config");
        assert_eq!(cfg.base_url, "https://new.example.test/api");
        assert!(cfg.api_token.is_empty());
        assert_eq!(cfg.user_id, uuid::Uuid::nil());
    }

    #[test]
    fn st56_config_without_settings_has_no_destination_and_nil_user() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cfg = Config::from_sources(
            &[],
            launch_env(&[(
                "LOCALAPPDATA",
                tmp.path().to_str().expect("utf-8 temp path"),
            )]),
            None,
            Channel::Official,
        )
        .expect("config");
        assert!(cfg.base_url.is_empty());
        assert!(cfg.api_token.is_empty());
        assert_eq!(cfg.user_id, uuid::Uuid::nil());
    }

    // Scenario: 既定の置き場が決まらなければ起動を止める
    #[test]
    fn st56_config_requires_a_default_state_dir() {
        let error = Config::from_sources(&[], launch_env(&[]), None, Channel::Official)
            .expect_err("missing state dir must fail");
        assert!(error.to_string().contains("LOCALAPPDATA"));
    }

    #[test]
    fn st56_config_saves_pasted_setup_string() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cfg = Config::from_sources(
            &[],
            launch_env(&[(
                "LOCALAPPDATA",
                tmp.path().to_str().expect("utf-8 temp path"),
            )]),
            None,
            Channel::Official,
        )
        .expect("config");
        let encoded = "ashiato-collector:1:eyJ1cmwiOiJodHRwczovL3BjLmV4YW1wbGUudHMubmV0L2FwaSIsInRva2VuIjoidG9rZW4tZm9yLXRlc3QtMDEyMzQ1Njc4OSJ9";
        cfg.save_setup_string(encoded).expect("save setup");
        let loaded = Config::from_sources(
            &[],
            launch_env(&[("LOCALAPPDATA", tmp.path().to_str().unwrap())]),
            None,
            Channel::Official,
        )
        .expect("reload config");
        assert_eq!(loaded.base_url, "https://pc.example.ts.net/api");
        assert_eq!(loaded.api_token, "token-for-test-0123456789");
        assert!(!loaded.needs_setup());
        assert!(
            !std::fs::read_to_string(cfg.state_dir.join("settings.json"))
                .unwrap()
                .contains("token-for-test-0123456789")
        );
    }

    // Scenario: 読めない文字列は保存しない
    #[test]
    fn st56_setupcmd_unreadable_string_does_not_overwrite_saved_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let state = tmp.path().join("ashiato/collector");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            state.join("settings.json"),
            r#"{"base_url":"https://old.example","device_id":"pc-01"}"#,
        )
        .unwrap();
        let cfg = Config::from_sources(
            &[],
            launch_env(&[("LOCALAPPDATA", tmp.path().to_str().unwrap())]),
            None,
            Channel::Official,
        )
        .unwrap();
        assert!(cfg.save_setup_string("hello").is_err());
        assert_eq!(
            Config::from_sources(
                &[],
                launch_env(&[("LOCALAPPDATA", tmp.path().to_str().unwrap())]),
                None,
                Channel::Official
            )
            .unwrap()
            .base_url,
            "https://old.example"
        );
    }

    #[test]
    fn st56_setupcmd_reload_reads_a_setup_written_by_another_process() {
        let tmp = tempfile::tempdir().unwrap();
        let env = launch_env(&[("LOCALAPPDATA", tmp.path().to_str().unwrap())]);
        let mut cfg = Config::from_sources(&[], &env, None, Channel::Official).unwrap();
        std::fs::write(
            cfg.state_dir.join("settings.json"),
            r#"{"base_url":"https://new.example","device_id":"pc-01"}"#,
        )
        .unwrap();
        advance_mtimes(&cfg.state_dir);
        assert!(cfg.reload_from_disk().unwrap());
        assert_eq!(cfg.base_url, "https://new.example");
    }

    #[test]
    fn st56_setupcmd_reload_reads_changed_credential_without_settings_change() {
        let tmp = tempfile::tempdir().unwrap();
        let env = launch_env(&[("LOCALAPPDATA", tmp.path().to_str().unwrap())]);
        let state = tmp.path().join("ashiato/collector");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            state.join("settings.json"),
            r#"{"base_url":"https://same.example","device_id":"pc-01"}"#,
        )
        .unwrap();
        let mut cfg = Config::from_sources(&[], &env, None, Channel::Official).unwrap();
        let original_settings_mtime = cfg.settings_mtime;
        std::fs::write(
            cfg.state_dir.join("credential.bin"),
            crate::secret::protect(b"new-token-0123456789").unwrap(),
        )
        .unwrap();
        assert_eq!(cfg.settings_mtime, original_settings_mtime);
        assert!(cfg.reload_from_disk().unwrap());
        assert_eq!(cfg.api_token, "new-token-0123456789");
    }

    // Scenario: 印を付けなければ、環境変数の資格情報を使わない
    #[test]
    fn st56_config_saved_credentials_are_selected_without_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = tmp.path().join("ashiato/collector");
        std::fs::create_dir_all(&state).expect("state");
        std::fs::write(
            state.join("settings.json"),
            r#"{"base_url":"http://127.0.0.1:8787","api_token":"new-token-0123456789","device_id":"pc-01"}"#,
        )
        .expect("settings");
        let cfg = Config::from_sources(
            &[],
            launch_env(&[
                ("LOCALAPPDATA", tmp.path().to_str().unwrap()),
                ("ASHIATO_API_TOKEN", "old-token"),
            ]),
            None,
            Channel::Official,
        )
        .expect("config");
        assert_eq!(cfg.api_token, "new-token-0123456789");
    }

    /// 試験用の設定の文字列（base64url・詰め物なし）。
    fn setup_string(url: &str, token: &str) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let json = serde_json::json!({ "url": url, "token": token }).to_string();
        let mut out = String::new();
        for chunk in json.as_bytes().chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |acc, (i, b)| acc | (u32::from(*b) << (16 - 8 * i)));
            for i in 0..=chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            }
        }
        format!("ashiato-collector:1:{out}")
    }

    #[test]
    fn st56_config_reload_does_not_pair_a_new_url_with_an_old_credential() {
        // review/code.md R34: setup の書きかけ（新しい送り先・古い資格情報）を読まない
        let tmp = tempfile::tempdir().unwrap();
        let env = launch_env(&[("LOCALAPPDATA", tmp.path().to_str().unwrap())]);
        let first = Config::from_sources(&[], &env, None, Channel::Official).unwrap();
        first
            .save_setup_string(&setup_string("https://a.example/api", "token-a-0123456789"))
            .unwrap();
        let mut cfg = Config::from_sources(&[], &env, None, Channel::Official).unwrap();
        assert_eq!(cfg.api_token, "token-a-0123456789");
        // settings.json だけ B に書かれた瞬間
        std::fs::write(
            cfg.state_dir.join("settings.json"),
            format!(
                r#"{{"base_url":"https://b.example/api","device_id":"{}"}}"#,
                cfg.device_id
            ),
        )
        .unwrap();
        advance_mtimes(&cfg.state_dir);
        assert!(!cfg.reload_from_disk().unwrap(), "書きかけを読み直した");
        assert_eq!(cfg.base_url, "https://a.example/api");
        assert_eq!(cfg.api_token, "token-a-0123456789");
        // 揃ったら読む
        cfg.save_setup_string(&setup_string("https://b.example/api", "token-b-0123456789"))
            .unwrap();
        advance_mtimes(&cfg.state_dir);
        assert!(cfg.reload_from_disk().unwrap());
        assert_eq!(cfg.base_url, "https://b.example/api");
        assert_eq!(cfg.api_token, "token-b-0123456789");
        // 起動のときも、別の送り先に結んだ資格情報は使わない
        std::fs::write(
            cfg.state_dir.join("settings.json"),
            format!(
                r#"{{"base_url":"https://c.example/api","device_id":"{}"}}"#,
                cfg.device_id
            ),
        )
        .unwrap();
        let started = Config::from_sources(&[], &env, None, Channel::Official).unwrap();
        assert!(started.api_token.is_empty());
    }

    #[test]
    fn st56_config_legacy_token_survives_the_settings_rewrite() {
        // review/code.md R35: settings.json の平文の資格情報は credential.bin へ移してから書き直す
        let tmp = tempfile::tempdir().unwrap();
        let state = tmp.path().join("ashiato/collector");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            state.join("settings.json"),
            r#"{"base_url":"https://x.example/api","api_token":"legacy-token-0123456789"}"#,
        )
        .unwrap();
        let env = launch_env(&[("LOCALAPPDATA", tmp.path().to_str().unwrap())]);
        let mut cfg = Config::from_sources(&[], &env, None, Channel::Official).unwrap();
        assert_eq!(cfg.api_token, "legacy-token-0123456789");
        assert!(!std::fs::read_to_string(state.join("settings.json"))
            .unwrap()
            .contains("legacy-token"));
        cfg.settings_mtime = None; // 読み直しを強いる
        cfg.reload_from_disk().unwrap();
        assert_eq!(cfg.api_token, "legacy-token-0123456789");
    }

    #[test]
    fn st56_config_new_hold_is_notified_at_once() {
        // review/code.md R37: 前の保留を知らせて 2 時間後に別の送り先を貼ったら、新しい保留をすぐ知らせる
        let tmp = tempfile::tempdir().unwrap();
        let env = launch_env(&[("LOCALAPPDATA", tmp.path().to_str().unwrap())]);
        let cfg = Config::from_sources(&[], &env, None, Channel::Official).unwrap();
        std::fs::write(cfg.state_dir.join("outbox.jsonl"), "{}\n").unwrap();
        let store = crate::destination_gate::Store::new(&cfg.state_dir);
        let now = chrono::Utc::now();
        store
            .save(&crate::destination_gate::State {
                approved_url: Some("https://a.example/api".into()),
                destination_notified_at: Some(now - chrono::Duration::hours(3)),
                destination_notice_at: Some(now - chrono::Duration::hours(2)),
                ..Default::default()
            })
            .unwrap();
        cfg.save_setup_string_at(
            &setup_string("https://c.example/api", "token-c-0123456789"),
            now,
        )
        .unwrap();
        let state = store.load().unwrap();
        assert!(crate::destination_gate::destination_notice(&state, now));
    }

    #[test]
    fn st56_config_release_state_dir_named_dev_keeps_its_own_exclusions() {
        // review/code.md R42: 正式版で置き場の名前が dev でも、親の除外の登録を読まない
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("data/dev");
        let env = launch_env(&[("ASHIATO_STATE_DIR", dir.to_str().unwrap())]);
        let cfg = Config::from_sources(&[], &env, None, Channel::Official).unwrap();
        assert_eq!(cfg.exclusions_path, dir.join("exclusions.json"));
        let dev = Config::from_sources(&[], &env, None, Channel::Dev).unwrap();
        assert_eq!(dev.exclusions_path, dir.join("exclusions.json"));
        assert_eq!(dev.state_dir, dir.join("dev"));
    }

    #[test]
    fn st56_config_broken_settings_are_not_replaced_with_a_new_device_id() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = tmp.path().join("ashiato/collector");
        std::fs::create_dir_all(&state).expect("state");
        std::fs::write(state.join("settings.json"), b"not-json").expect("settings");
        let error = Config::from_sources(
            &[],
            launch_env(&[("LOCALAPPDATA", tmp.path().to_str().unwrap())]),
            None,
            Channel::Official,
        )
        .expect_err("broken settings must stop startup");
        assert!(error.to_string().contains("settings.json"));
        assert_eq!(
            std::fs::read(state.join("settings.json")).unwrap(),
            b"not-json"
        );
    }
}
