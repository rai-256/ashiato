// SPDX-License-Identifier: AGPL-3.0-only
//! 途切れの通知を Windows のトーストとして出す（ST14 / design D11）。OS の口だけ。
//!
//! 差出人（AUMID）は `HKCU\Software\Classes\AppUserModelId\dev.ashiato.collector` に書く
//! （管理者権限は要らない。未パッケージの exe でも通知を出せる）。**unsafe は書かない** ——
//! 登録簿は `winreg`、通知は `windows` crate の safe な WinRT で触る（作業場は `unsafe_code = "forbid"`）。
//! 途切れの通知は `Tag = "outage"`・`Group = "ashiato"` の 1 通だけで、出し直せば前のものを置き換える。
use anyhow::Context as _;
use windows::core::HSTRING;
use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

use crate::contract_notice::ContractToaster;
use crate::outage_notice::Toaster;

/// 通知の差出人。
pub const AUMID: &str = "dev.ashiato.collector";
/// 通知の識別（途切れの通知は常に 1 通）。
pub const TAG: &str = "outage";
pub const GROUP: &str = "ashiato";
/// 「サーバを更新してください」の通知の識別（途切れの通知の `outage` と分ける。ST50 / design D5）。
pub const CONTRACT_TAG: &str = "contract";

/// 通知の設定の窓口。
#[derive(Debug)]
pub struct WindowsToaster {
    aumid: String,
}

/// 差出人を登録簿に書く（`DisplayName = "ashiato"`）。書けなければ `Err`（呼び出し側は警告だけで収集を続ける）。
pub fn register_aumid(aumid: &str) -> anyhow::Result<()> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(format!(r"Software\Classes\AppUserModelId\{aumid}"))
        .context("登録簿の鍵を作れない")?;
    key.set_value("DisplayName", &"ashiato")
        .context("登録簿へ書けない")
}

impl WindowsToaster {
    /// 差出人を登録して口を作る。
    pub fn register() -> anyhow::Result<Self> {
        Self::register_as(AUMID)
    }

    /// 差出人の名前を指定して登録する（試験が本物の登録と混ざらないように使う）。
    pub fn register_as(aumid: &str) -> anyhow::Result<Self> {
        register_aumid(aumid)?;
        Ok(Self {
            aumid: aumid.to_string(),
        })
    }

    fn history_remove(&self, tag: &str) -> windows::core::Result<()> {
        ToastNotificationManager::History()?.RemoveGroupedTagWithId(
            &HSTRING::from(tag),
            &HSTRING::from(GROUP),
            &HSTRING::from(self.aumid.as_str()),
        )
    }
}

/// 通知を組む。音なし（`sound = false`）は `SuppressPopup` を立てる。
/// 試験が組んだ通知の設定を読めるよう公開している。
pub fn build_toast(xml: &str, sound: bool) -> windows::core::Result<ToastNotification> {
    build_toast_tagged(xml, sound, TAG)
}

/// 札を指定して通知を組む。
pub fn build_toast_tagged(
    xml: &str,
    sound: bool,
    tag: &str,
) -> windows::core::Result<ToastNotification> {
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    toast.SetTag(&HSTRING::from(tag))?;
    toast.SetGroup(&HSTRING::from(GROUP))?;
    // 音なしの差し替えは画面の端に出さず、通知の一覧の中身だけを差し替える（spec-review R5）
    toast.SetSuppressPopup(!sound)?;
    Ok(toast)
}

impl WindowsToaster {
    fn show_tagged(&self, xml: &str, sound: bool, tag: &str) -> anyhow::Result<()> {
        // 例外の文言は外へ出さない（種別だけ）
        let toast = build_toast_tagged(xml, sound, tag)
            .map_err(|_| anyhow::anyhow!("toast build failed"))?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(self.aumid.as_str()))
            .and_then(|notifier| notifier.Show(&toast))
            .map_err(|_| anyhow::anyhow!("toast show failed"))
    }

    fn dismiss_tagged(&self, tag: &str) -> anyhow::Result<()> {
        self.history_remove(tag)
            .map_err(|_| anyhow::anyhow!("toast remove failed"))
    }
}

impl Toaster for WindowsToaster {
    fn show(&self, xml: &str, sound: bool) -> anyhow::Result<()> {
        self.show_tagged(xml, sound, TAG)
    }

    fn dismiss(&self) -> anyhow::Result<()> {
        self.dismiss_tagged(TAG)
    }
}

/// 札 `contract` の通知（途切れの通知の札 `outage` には触らない）。
impl ContractToaster for WindowsToaster {
    fn show_contract(&self, xml: &str) -> anyhow::Result<()> {
        self.show_tagged(xml, true, CONTRACT_TAG)
    }

    fn dismiss_contract(&self) -> anyhow::Result<()> {
        self.dismiss_tagged(CONTRACT_TAG)
    }
}
