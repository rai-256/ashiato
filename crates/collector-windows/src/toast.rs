// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 途切れの通知を Windows のトーストとして出す（ST14 / design D11）。OS の口だけ。
//!
//! 差出人（AUMID）は `HKCU\Software\Classes\AppUserModelId\dev.ashiato.collector` に書く
//! （管理者権限は要らない。未パッケージの exe でも通知を出せる）。**unsafe は書かない** ——
//! 登録簿は `winreg`、通知は `windows` crate の safe な WinRT で触る（作業場は `unsafe_code = "deny"`。
//! 許すのは DPAPI を呼ぶ `secret.rs` だけで、このファイルは許していない）。
//! 途切れの通知は `Tag = "outage"`・`Group = "ashiato"` の 1 通だけで、出し直せば前のものを置き換える。
use anyhow::Context as _;
use windows::core::{IInspectable, HSTRING};
use windows::Data::Xml::Dom::XmlDocument;
use windows::Foundation::TypedEventHandler;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

use crate::channel_notice::ChannelToaster;
use crate::contract_notice::ContractToaster;
use crate::outage_notice::Toaster;

/// 通知の差出人。
pub use crate::toast_names::{aumid, display_name};
/// 通知の識別（途切れの通知は常に 1 通）。
pub const TAG: &str = "outage";
pub const GROUP: &str = "ashiato";
/// 「サーバを更新してください」の通知の識別（途切れの通知の `outage` と分ける。ST50 / design D5）。
pub const CONTRACT_TAG: &str = "contract";
/// 未設定の通知。押すと `setup` 引数で新しい窓を起動する。
pub const SETUP_TAG: &str = "setup";
pub const AUTH_TAG: &str = "auth";
pub const UNREACHABLE_TAG: &str = "unreachable";
pub const DESTINATION_TAG: &str = "destination";
/// 系統の違いの通知（契約の版の通知とは別）。
pub const CHANNEL_TAG: &str = "channel";

/// 通知の設定の窓口。
pub struct WindowsToaster {
    aumid: String,
    /// 押すと `setup` の窓を開く通知と、その受け手の登録（札ごとに 1 つ。通知を手放すと受け手も外れる）
    setup_activations: std::sync::Mutex<Vec<(&'static str, ToastNotification, i64)>>,
}

impl std::fmt::Debug for WindowsToaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsToaster")
            .field("aumid", &self.aumid)
            .finish_non_exhaustive()
    }
}

/// 差出人を登録簿に書く（`DisplayName = "ashiato"`）。書けなければ `Err`（呼び出し側は警告だけで収集を続ける）。
pub fn register_aumid(aumid: &str, display_name: &str) -> anyhow::Result<()> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(format!(r"Software\Classes\AppUserModelId\{aumid}"))
        .context("登録簿の鍵を作れない")?;
    key.set_value("DisplayName", &display_name)
        .context("登録簿へ書けない")
}

impl WindowsToaster {
    /// 押すと `setup` の窓を開く通知を出す。未設定・断られ・届かない・保留の全部がこれを通る（review/code.md R8）。
    fn show_opening_setup(&self, xml: &str, sound: bool, tag: &'static str) -> anyhow::Result<()> {
        let toast = build_toast_tagged(xml, sound, tag)
            .map_err(|_| anyhow::anyhow!("toast build failed"))?;
        let exe = std::env::current_exe().context("収集アプリの場所を読めない")?;
        let handler = TypedEventHandler::<ToastNotification, IInspectable>::new(move |_, _| {
            let _ = spawn_setup_window(&exe);
            Ok(())
        });
        let token = toast
            .Activated(&handler)
            .map_err(|_| anyhow::anyhow!("toast activation registration failed"))?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(self.aumid.as_str()))
            .and_then(|notifier| notifier.Show(&toast))
            .map_err(|_| anyhow::anyhow!("toast show failed"))?;
        let mut activations = self
            .setup_activations
            .lock()
            .map_err(|_| anyhow::anyhow!("toast activation lock failed"))?;
        // 同じ札の前の通知は置き換わったので、受け手も新しいものだけを持つ
        activations.retain(|(held, _, _)| *held != tag);
        activations.push((tag, toast, token));
        Ok(())
    }

    /// 差出人を登録して口を作る。
    pub fn register(channel: ashiato_build_stamp::Channel) -> anyhow::Result<Self> {
        register_aumid(aumid(channel), display_name(channel))?;
        Ok(Self {
            aumid: aumid(channel).to_string(),
            setup_activations: std::sync::Mutex::new(Vec::new()),
        })
    }

    /// 差出人の名前を指定して登録する（試験が本物の登録と混ざらないように使う）。
    pub fn register_as(aumid: &str) -> anyhow::Result<Self> {
        register_aumid(aumid, "ashiato")?;
        Ok(Self {
            aumid: aumid.to_string(),
            setup_activations: std::sync::Mutex::new(Vec::new()),
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

impl crate::destination_gate::DestinationToaster for WindowsToaster {
    fn show_destination(
        &self,
        kind: crate::destination_gate::NoticeKind,
        message: &str,
    ) -> anyhow::Result<()> {
        let tag = match kind {
            crate::destination_gate::NoticeKind::Auth => AUTH_TAG,
            crate::destination_gate::NoticeKind::Unreachable => UNREACHABLE_TAG,
        };
        self.show_opening_setup(&setup_toast_xml("ashiato", message), false, tag)
    }

    fn show_destination_hold(&self, message: &str) -> anyhow::Result<()> {
        self.show_opening_setup(&setup_toast_xml("ashiato", message), false, DESTINATION_TAG)
    }

    fn show_setup(&self) -> anyhow::Result<()> {
        self.show_opening_setup(
            &setup_toast_xml(
                "送り先が設定されていません",
                "押して setup を開いてください",
            ),
            true,
            SETUP_TAG,
        )
    }

    fn dismiss_setup(&self) -> anyhow::Result<()> {
        if let Ok(mut activations) = self.setup_activations.lock() {
            activations.retain(|(held, _, _)| *held != SETUP_TAG);
        }
        self.dismiss_tagged(SETUP_TAG)
    }
}

/// 押すと `setup` を開く通知の XML。文は固定の文言だけを入れる（資格情報は入らない）。
fn setup_toast_xml(title: &str, body: &str) -> String {
    format!(
        r#"<toast launch="{}"><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual></toast>"#,
        crate::SETUP_ARGUMENT,
        title,
        body
    )
}

fn spawn_setup_window(exe: &std::path::Path) -> std::io::Result<std::process::Child> {
    use std::os::windows::process::CommandExt as _;
    std::process::Command::new(exe)
        .arg(crate::SETUP_ARGUMENT)
        .creation_flags(0x0000_0010)
        .spawn()
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

impl ChannelToaster for WindowsToaster {
    fn show_channel(&self, xml: &str) -> anyhow::Result<()> {
        self.show_tagged(xml, true, CHANNEL_TAG)
    }

    fn dismiss_channel(&self) -> anyhow::Result<()> {
        self.dismiss_tagged(CHANNEL_TAG)
    }
}
