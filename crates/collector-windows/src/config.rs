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
#[derive(Debug, Clone)]
pub struct Config {
    /// 取り込み口の基点（`http://127.0.0.1:8787`）
    pub base_url: String,
    /// 合言葉（PERM-8。**ログに出さない**）
    pub api_token: String,
    /// 利用者（FR-29 / PERM-1）
    pub user_id: uuid::Uuid,
    /// どの端末が生成したか（FR-24。`origin=collected` なら必須）
    pub device_id: String,
    /// 未送信の置き場
    pub state_dir: std::path::PathBuf,
}

impl Config {
    /// 環境変数から読む。**1 つでも欠けていればエラー**にする ——
    /// 既定で埋めると、送り先を間違えたまま動く。
    pub fn from_env() -> anyhow::Result<Self> {
        let var = |k: &str| std::env::var(k).with_context(|| format!("{k} が無い"));
        let user_id = var("ASHIATO_USER_ID")?;
        Ok(Self {
            base_url: var("ASHIATO_BASE_URL")?,
            api_token: var("ASHIATO_API_TOKEN")?,
            user_id: uuid::Uuid::parse_str(user_id.trim())
                .context("ASHIATO_USER_ID が uuid ではない")?,
            device_id: var("ASHIATO_DEVICE_ID")?,
            state_dir: std::path::PathBuf::from(
                std::env::var("ASHIATO_STATE_DIR").unwrap_or_else(|_| ".".into()),
            ),
        })
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
}
