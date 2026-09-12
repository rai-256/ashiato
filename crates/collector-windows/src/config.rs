// SPDX-License-Identifier: AGPL-3.0-only
//! 接続先と資格情報、置き場。**コミットしない**（製造準備 A-2）——
//! 環境変数から読む。
//!
//! 揃っていなければ**送信を始めないが、取得は続ける**（C-01 の `Config` と同じ向き）。
//! 記録は未送信に積まれ、設定してから送られる —— **捨てない**。
use anyhow::Context as _;

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
    /// 取り込み口の基点（`http://127.0.0.1:8787`）
    pub base_url: String,
    /// 合言葉（PERM-8。**ログにも `Debug` にも出さない**）
    pub api_token: String,
    /// 利用者（FR-29 / PERM-1）
    pub user_id: uuid::Uuid,
    /// どの端末が生成したか（FR-24。`origin=collected` なら必須）
    pub device_id: String,
    /// 未送信・印・数え・除外の登録の置き場
    pub state_dir: std::path::PathBuf,
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

impl Config {
    /// 環境変数から読む。
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// 名前から値を引く関数で読む。**1 つでも欠けていればエラー**にする ——
    /// 既定で埋めると、送り先を間違えたまま動く。
    ///
    /// **置き場にも既定を持たない**（R27）。`.` に倒すと、起動のしかた次第で
    /// 置き場が `C:\Windows\System32` などに移り、前の置き場の未送信・印・除外の登録が
    /// 黙って置き去りになる（**除外が外れ、直前の停止期間が「初回起動」に化ける**）。
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
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
        Ok(Self {
            base_url: var("ASHIATO_BASE_URL")?,
            api_token: var("ASHIATO_API_TOKEN")?,
            user_id: uuid::Uuid::parse_str(user_id.trim())
                .context("ASHIATO_USER_ID が uuid ではない")?,
            device_id: var("ASHIATO_DEVICE_ID")?,
            state_dir,
        })
    }

    /// 変数の名前と値の組（自動起動の `.cmd` に書く。R3）。
    pub fn as_vars(&self) -> [(&'static str, String); 5] {
        [
            (VARS[0], self.base_url.clone()),
            (VARS[1], self.api_token.clone()),
            (VARS[2], self.user_id.to_string()),
            (VARS[3], self.device_id.clone()),
            (VARS[4], self.state_dir.display().to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn abs_dir() -> String {
        std::env::temp_dir().join("ashiato").display().to_string()
    }

    /// **置き場を含む 5 つが揃わなければ読まない**（R27）。相対パスの置き場も断る。
    #[test]
    fn config_requires_every_var_including_state_dir() {
        let full = |k: &str| {
            Some(match k {
                "ASHIATO_BASE_URL" => "http://127.0.0.1:8787".to_string(),
                "ASHIATO_API_TOKEN" => "t0123456789".to_string(),
                "ASHIATO_USER_ID" => uuid::Uuid::nil().to_string(),
                "ASHIATO_DEVICE_ID" => "pc-01".to_string(),
                "ASHIATO_STATE_DIR" => abs_dir(),
                _ => return None,
            })
        };
        assert!(Config::from_lookup(full).is_ok());
        for missing in VARS {
            let got = Config::from_lookup(|k| if k == missing { None } else { full(k) });
            assert!(got.is_err(), "{missing} が無いのに読めた");
        }
        let relative = Config::from_lookup(|k| {
            if k == "ASHIATO_STATE_DIR" {
                Some(".".into())
            } else {
                full(k)
            }
        });
        assert!(relative.is_err(), "相対パスの置き場を受けた");
    }

    /// `Debug` に合言葉が出ない（R11）。
    #[test]
    fn debug_hides_the_token() {
        let c = Config {
            base_url: "http://x".into(),
            api_token: "SUPER-SECRET-TOKEN".into(),
            user_id: uuid::Uuid::nil(),
            device_id: "pc".into(),
            state_dir: std::path::PathBuf::from(abs_dir()),
        };
        assert!(!format!("{c:?}").contains("SUPER-SECRET"));
    }
}
