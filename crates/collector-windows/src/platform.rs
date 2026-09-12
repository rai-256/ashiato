// SPDX-License-Identifier: AGPL-3.0-only
//! Windows の実機で OS を触る側（design D5 / D15）。**ここだけが実機を要する。**
//!
//! # なぜ wrapper に委ねるか（design D15）
//!
//! 作業場の lint は `unsafe_code = "forbid"`（`Cargo.toml` の
//! `[workspace.lints.rust]`）で、これは**ワークスペース全体の保証**。
//! Win32 を自分で呼ぶと崩れるので、**安全な wrapper 越しに呼ぶ**:
//!
//! | | |
//! |---|---|
//! | `active-win-pos-rs` | 前景の題名・アプリ名・実行ファイルのパス・プロセス識別子 |
//! | `user-idle-time` | 最後の入力からの経過時間（FR-81） |
//! | `uiautomation` | アドレスバーの読み取り（深掘り Q4）と、焦点が変わった通知（design D5） |
//!
//! # 変化の拾い方（design D5・**仮**）
//!
//! **OS の通知と 1 秒間隔の見回りの併用。** 焦点が変わった通知で即座に起き、
//! 同じウィンドウの題名だけの変化（タブ切り替え・動画の再生位置）は
//! 通知が来ないことがあるので最長 1 秒遅れで拾う。
//! **取りこぼした変化は後から作れない**ので、取りこぼさない側を既定にする。
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use uiautomation::controls::ControlType;
use uiautomation::patterns::UIValuePattern;
use uiautomation::{UIAutomation, UIElement};

use crate::browsers::Browsers;
use crate::engine::{Foreground, IdleRead, Observation, UrlRead};
use crate::runtime::Source;

/// ロック画面を出しているプロセス（Windows 10 / 11）。
///
/// **前景として記録しない** —— ロック中は「何も見ていない」であって、
/// `LockApp` というアプリを使っていたのではない。
const LOCK_SCREEN: [&str; 2] = ["lockapp.exe", "logonui.exe"];

/// 焦点が変わったことを受け取る係。**印を立てるだけ**にする ——
/// 通知の糸で OS を触ると、重い処理が UI を止める。
#[derive(Debug)]
struct FocusFlag(Arc<AtomicBool>);

impl uiautomation::events::CustomFocusChangedEventHandler for FocusFlag {
    fn handle(&self, _sender: &UIElement) -> uiautomation::Result<()> {
        self.0.store(true, Ordering::Relaxed);
        Ok(())
    }
}

/// 実機の OS を読む係。
pub struct WindowsSource {
    /// UI Automation の口。**開けなくても収集は続ける**（URL だけが取れない）
    automation: Option<UIAutomation>,
    browsers: Browsers,
    changed: Arc<AtomicBool>,
}

impl std::fmt::Debug for WindowsSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsSource")
            .field("uiautomation", &self.automation.is_some())
            .field("browsers", &self.browsers)
            .finish()
    }
}

impl WindowsSource {
    /// 開く。**UI Automation が開けなくても失敗にしない** ——
    /// 前景と入力は取れるので、そのぶんは残す。取れない側は生存信号が報告する。
    pub fn open() -> Self {
        let automation = UIAutomation::new().ok();
        let changed = Arc::new(AtomicBool::new(false));
        if let Some(a) = automation.as_ref() {
            let handler = uiautomation::events::UIFocusChangedEventHandler::from(FocusFlag(
                Arc::clone(&changed),
            ));
            // **失敗しても見回りで拾える**（design D5 の併用がここで効く）
            let _ = a.add_focus_changed_event_handler(None, &handler);
        }
        Self {
            automation,
            browsers: Browsers::from_env(),
            changed,
        }
    }

    /// 前景が変わったという通知が来ていたか（来ていれば見回りを待たない）。
    ///
    /// **読むと下ろす** —— 下ろさないと、1 回の通知で回り続ける。
    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::Relaxed)
    }

    /// アドレスバーを読む（深掘り Q4 / design D12）。**補正しない。**
    fn read_url(&self, process_name: &str, pid: u32) -> UrlRead {
        if !self.browsers.contains(process_name) {
            return UrlRead::NotBrowser;
        }
        let Some(a) = self.automation.as_ref() else {
            return UrlRead::Unavailable;
        };
        // 焦点のある要素から上に辿って、そのプロセスのウィンドウを見つける。
        // **HWND を文字列から起こさない** —— `active-win-pos-rs` の `window_id` は
        // 表示用の形なので、版が変われば形が変わる
        let Ok(focused) = a.get_focused_element() else {
            return UrlRead::Unavailable;
        };
        let Some(window) = top_level_window(a, &focused) else {
            return UrlRead::Unavailable;
        };
        if window.get_process_id().ok() != Some(pid) {
            // 焦点が別のプロセスにある（別の窓が前景に見えている）。
            // **読めなかったことにする** —— 別のプロセスの URL を混ぜない
            return UrlRead::Unavailable;
        }
        let bar = a
            .create_matcher()
            .from(window)
            .control_type(ControlType::Edit)
            .timeout(0)
            .depth(12)
            .find_first();
        match bar.and_then(|b| b.get_pattern::<UIValuePattern>()?.get_value()) {
            // **見えている文字列をそのまま。** 空なら「読めなかった」ではなく空として残す
            Ok(value) => UrlRead::Read(value),
            Err(_) => UrlRead::Unavailable,
        }
    }
}

/// 焦点のある要素から、上に辿って最初のウィンドウを返す。
fn top_level_window(a: &UIAutomation, from: &UIElement) -> Option<UIElement> {
    let walker = a.get_control_view_walker().ok()?;
    let mut cur = from.clone();
    for _ in 0..16 {
        if cur.get_control_type().ok() == Some(ControlType::Window) {
            return Some(cur);
        }
        cur = walker.get_parent(&cur).ok()?;
    }
    None
}

impl Source for WindowsSource {
    fn observe(&mut self, at: DateTime<Utc>) -> Observation {
        let win = active_win_pos_rs::get_active_window().ok();
        let idle = match user_idle_time::get_idle_time() {
            Ok(d) => IdleRead::Elapsed(Duration::from_std(d).unwrap_or_else(|_| Duration::zero())),
            Err(_) => IdleRead::Unavailable,
        };
        let process_name = win
            .as_ref()
            .and_then(|w| w.process_path.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let locked = LOCK_SCREEN
            .iter()
            .any(|l| l.eq_ignore_ascii_case(&process_name));

        let foreground = match win {
            // ロック画面は**前景として残さない**（見ていたアプリではない）
            _ if locked => None,
            Some(w) => {
                let pid = u32::try_from(w.process_id).unwrap_or_default();
                let url = self.read_url(&process_name, pid);
                Some(Foreground {
                    app_name: w.app_name,
                    exe_path: w.process_path.to_string_lossy().to_string(),
                    process_name,
                    title: w.title,
                    url,
                })
            }
            None => None,
        };
        Observation {
            at,
            foreground,
            idle,
            locked,
        }
    }
}
