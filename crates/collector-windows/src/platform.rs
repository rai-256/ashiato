// SPDX-License-Identifier: AGPL-3.0-only
//! Windows の実機で OS を触る側（design D5 / D15）。**ここだけが実機を要する。**
//!
//! 読んだ値を**どう解釈するか**は `winrules.rs` に出してある（Linux でも確かめられる。R9）。
//! ここに残るのは OS を呼ぶ手順だけ。
//!
//! # なぜ wrapper に委ねるか（design D15）
//!
//! 作業場の lint は `unsafe_code = "forbid"`。Win32 を自分で呼ぶと崩れるので、
//! **安全な wrapper 越しに呼ぶ**:
//!
//! | | |
//! |---|---|
//! | `active-win-pos-rs` | 前景の題名・アプリ名・実行ファイルのパス・プロセス識別子 |
//! | `user-idle-time` | 最後の入力からの経過時間（FR-81） |
//! | `uiautomation` | アドレスバーの読み取り（深掘り Q4）と、焦点が変わった通知（design D5） |
//! | `sysinfo`（`system` だけ） | OS が最後に起動した時刻（design D23） |
//!
//! # 変化の拾い方（design D5・**仮**）
//!
//! **OS の通知と 1 秒間隔の見回りの併用。** 焦点が変わった通知で即座に起き、
//! 同じウィンドウの題名だけの変化は最長 1 秒遅れで拾う。
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use uiautomation::controls::ControlType;
use uiautomation::patterns::UIValuePattern;
use uiautomation::{UIAutomation, UIElement};

use crate::browsers::Browsers;
use crate::engine::{Foreground, Observation, UrlRead};
use crate::heartbeat::blocker;
use crate::runtime::Source;
use crate::winrules;

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
    /// 焦点の通知を登録できたか（できなければ見回りだけで拾う）
    focus_events: bool,
    browsers: Browsers,
    changed: Arc<AtomicBool>,
}

impl std::fmt::Debug for WindowsSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsSource")
            .field("uiautomation", &self.automation.is_some())
            .field("focus_events", &self.focus_events)
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
        let focus_events = automation.as_ref().is_some_and(|a| {
            let handler = uiautomation::events::UIFocusChangedEventHandler::from(FocusFlag(
                Arc::clone(&changed),
            ));
            a.add_focus_changed_event_handler(None, &handler).is_ok()
        });
        Self {
            automation,
            focus_events,
            browsers: Browsers::from_env(),
            changed,
        }
    }

    /// 焦点の通知を登録できたか（**起動時にログへ出す**。黙って見回りだけにならない）。
    pub fn focus_events(&self) -> bool {
        self.focus_events
    }

    /// 前景が変わったという通知が来ていたか（来ていれば見回りを待たない）。
    ///
    /// **読むと下ろす** —— 下ろさないと、1 回の通知で回り続ける。
    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::Relaxed)
    }

    /// アドレスバーを読む（深掘り Q4 / design D12）。**補正しない。**
    fn read_url(&self, process_name: &str, pid: Option<u32>) -> UrlRead {
        if !self.browsers.contains(process_name) {
            return UrlRead::NotBrowser;
        }
        let (Some(a), Some(pid)) = (self.automation.as_ref(), pid) else {
            return UrlRead::Unavailable;
        };
        // 焦点のある要素から上に辿って、そのプロセスのウィンドウを見つける。
        // **HWND を文字列から起こさない** —— `active-win-pos-rs` の `window_id` は表示用の形
        let Ok(focused) = a.get_focused_element() else {
            return UrlRead::Unavailable;
        };
        let Some(window) = top_level_window(a, &focused) else {
            return UrlRead::Unavailable;
        };
        if window.get_process_id().ok() != Some(pid) {
            // 焦点が別のプロセスにある。**別のプロセスの URL を混ぜない**
            return UrlRead::Unavailable;
        }
        let bar = a
            .create_matcher()
            .from(window)
            .control_type(ControlType::Edit)
            .timeout(0)
            .depth(12)
            .find_first();
        winrules::url_from_value(bar.and_then(|b| b.get_pattern::<UIValuePattern>()?.get_value()))
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
        let idle = winrules::idle_from(user_idle_time::get_idle_time());
        let process_name = win
            .as_ref()
            .and_then(|w| winrules::process_name_of(&w.process_path));
        let locked = process_name
            .as_deref()
            .is_some_and(winrules::is_lock_screen);

        let foreground = match (win, process_name) {
            // ロック画面は**前景として残さない**（見ていたアプリではない）
            _ if locked => None,
            // **プロセス名が解決できない前景は「読めなかった」**（R29）。
            // 空の名前で進むと、プロセス名で指した除外の規則をすり抜ける
            (Some(w), Some(process_name)) => {
                let pid = u32::try_from(w.process_id).ok().filter(|p| *p != 0);
                let url = self.read_url(&process_name, pid);
                Some(Foreground {
                    app_name: w.app_name,
                    exe_path: w.process_path.to_string_lossy().to_string(),
                    process_name,
                    title: w.title,
                    url,
                })
            }
            _ => None,
        };
        Observation {
            at,
            foreground,
            idle,
            locked,
        }
    }

    fn persistent_blockers(&self) -> Vec<&'static str> {
        // **UI Automation を開けなかったら、ブラウザが前景でなくても挙げ続ける**（R26）。
        // 開けていない限り URL は 1 件も取れない
        if self.automation.is_none() {
            vec![blocker::UIAUTOMATION]
        } else {
            Vec::new()
        }
    }

    fn boot_time(&self) -> Option<DateTime<Utc>> {
        let secs = i64::try_from(sysinfo::System::boot_time()).ok()?;
        DateTime::from_timestamp(secs, 0).filter(|_| secs > 0)
    }
}
