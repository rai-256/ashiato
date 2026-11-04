//! 系統の違いを知らせる PC 通知（D12（仮））。

use std::sync::Arc;

use anyhow::Result;
use ashiato_build_stamp::Channel;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::fsutil;
use crate::telemetry;

pub trait ChannelToaster: std::fmt::Debug + Send + Sync {
    fn show_channel(&self, xml: &str) -> Result<()>;
    fn dismiss_channel(&self) -> Result<()>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct State {
    #[serde(default)]
    notified_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct ChannelNotice {
    toaster: Arc<dyn ChannelToaster>,
    path: std::path::PathBuf,
    state: std::sync::Mutex<State>,
    failing: std::sync::Mutex<bool>,
}

impl ChannelNotice {
    pub fn new(toaster: Arc<dyn ChannelToaster>, state_dir: &std::path::Path) -> Self {
        let path = state_dir.join("channel-notice.json");
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self {
            toaster,
            path,
            state: std::sync::Mutex::new(state),
            failing: std::sync::Mutex::new(false),
        }
    }

    pub fn after_send(
        &self,
        server: Option<Channel>,
        self_channel: Channel,
        now: DateTime<Utc>,
        log: &mut dyn FnMut(String),
    ) {
        let Some(server) = server else { return };
        let mismatch = server != self_channel;
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !mismatch {
            if state.notified_at.is_some() {
                // 消せなかったときは時刻を残す（次の契機で消し直す。final review R6）
                if let Err(e) = self.toaster.dismiss_channel() {
                    self.log_failure(log, &e);
                    return;
                }
                state.notified_at = None;
                if let Err(e) = self.save(&state) {
                    self.log_failure(log, &e);
                }
            }
            return;
        }
        let due = state
            .notified_at
            .is_none_or(|at| now - at >= Duration::hours(24));
        if !due {
            return;
        }
        let message = match server {
            Channel::Dev => "送り先が開発版のサーバです。送り先を確かめてください",
            Channel::Official => "送り先が正式版のサーバです。送り先を確かめてください",
        };
        let xml = crate::contract_notice::toast_xml(message);
        if let Err(e) = self.toaster.show_channel(&xml) {
            self.log_failure(log, &e);
            return;
        }
        state.notified_at = Some(now);
        if let Err(e) = self.save(&state) {
            self.log_failure(log, &e);
        }
    }

    fn save(&self, state: &State) -> Result<()> {
        fsutil::atomic_write(&self.path, &serde_json::to_vec(state)?)
    }

    fn log_failure(&self, log: &mut dyn FnMut(String), error: &anyhow::Error) {
        let mut failing = self
            .failing
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !*failing {
            log(telemetry::line(
                "channel_notice_skipped",
                None,
                None,
                Some(telemetry::error_kind(error)),
            ));
        }
        *failing = true;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::sync::Mutex;

    /// 出した通知を本文で、消した通知を空の文字列で積む。`fail_dismiss` が残っている間は消すのに失敗する。
    #[derive(Debug, Default)]
    struct Fake {
        calls: Mutex<Vec<String>>,
        fail_dismiss: Mutex<usize>,
    }
    impl ChannelToaster for Fake {
        fn show_channel(&self, xml: &str) -> Result<()> {
            self.calls.lock().unwrap().push(xml.into());
            Ok(())
        }
        fn dismiss_channel(&self) -> Result<()> {
            let mut fail = self.fail_dismiss.lock().unwrap();
            if *fail > 0 {
                *fail -= 1;
                anyhow::bail!("dismiss failed");
            }
            self.calls.lock().unwrap().push(String::new());
            Ok(())
        }
    }
    impl Fake {
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    fn at(hours: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-03T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::hours(hours)
    }

    fn notice() -> (tempfile::TempDir, Arc<Fake>, ChannelNotice) {
        let dir = tempfile::TempDir::new().unwrap();
        let fake = Arc::new(Fake::default());
        let notice = ChannelNotice::new(fake.clone(), dir.path());
        (dir, fake, notice)
    }

    // Scenario の印は runtime.rs の統合の試験が持つ。ここは知らせの係だけを見る
    #[test]
    fn st54_channel_notice_is_daily() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut log);
        n.after_send(Some(Channel::Dev), Channel::Official, at(23), &mut log);
        n.after_send(Some(Channel::Dev), Channel::Official, at(24), &mut log);
        assert_eq!(fake.calls().len(), 2);
    }

    #[test]
    fn st54_channel_mismatch_names_server_and_is_not_contract_notice() {
        for (server, me, name) in [
            (Channel::Official, Channel::Dev, "正式版のサーバ"),
            (Channel::Dev, Channel::Official, "開発版のサーバ"),
        ] {
            let (_d, fake, n) = notice();
            let mut log = |_| {};
            n.after_send(Some(server), me, at(0), &mut log);
            let calls = fake.calls();
            assert_eq!(calls.len(), 1);
            assert!(calls[0].contains(name), "{}", calls[0]);
            assert!(!calls[0].contains("更新してください"));
        }
    }

    #[test]
    fn st54_channel_same_channel_has_no_notice() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Dev), Channel::Dev, at(0), &mut log);
        assert!(fake.calls().is_empty());
    }

    /// 系統を読めない答え（404・読めない 200・401。どれも `None` で来る）では出さない。
    #[test]
    fn st54_channel_unknown_answer_has_no_notice() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(None, Channel::Dev, at(0), &mut log);
        n.after_send(None, Channel::Official, at(0), &mut log);
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn st54_channel_same_channel_dismisses() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut log);
        n.after_send(Some(Channel::Official), Channel::Official, at(1), &mut log);
        n.after_send(Some(Channel::Official), Channel::Official, at(2), &mut log);
        let calls = fake.calls();
        assert_eq!(calls.len(), 2, "消すのは 1 回だけ");
        assert!(calls[1].is_empty());
    }

    /// 消すのに失敗したら、出した時刻を残して次の契機で消し直す（final review R6）。
    #[test]
    fn st54_channel_dismiss_failure_retries_next_time() {
        let (dir, fake, n) = notice();
        let mut lines = Vec::new();
        n.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut |l| {
            lines.push(l)
        });
        *fake.fail_dismiss.lock().unwrap() = 1;
        n.after_send(
            Some(Channel::Official),
            Channel::Official,
            at(1),
            &mut |l| lines.push(l),
        );
        assert_eq!(fake.calls().len(), 1, "消せていないのに消したと数えた");
        assert!(lines.iter().any(|l| l.contains("channel_notice_skipped")));
        n.after_send(
            Some(Channel::Official),
            Channel::Official,
            at(2),
            &mut |l| lines.push(l),
        );
        assert_eq!(fake.calls().len(), 2);
        assert!(fake.calls()[1].is_empty());
        // 消し直した後は、起こし直しても消す時刻が残っていない
        let restarted = ChannelNotice::new(fake.clone(), dir.path());
        restarted.after_send(
            Some(Channel::Official),
            Channel::Official,
            at(3),
            &mut |_| {},
        );
        assert_eq!(fake.calls().len(), 2);
    }

    #[test]
    fn st54_channel_notice_persists_time() {
        let dir = tempfile::TempDir::new().unwrap();
        let fake = Arc::new(Fake::default());
        {
            let n = ChannelNotice::new(fake.clone(), dir.path());
            n.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut |_| {});
        }
        let n = ChannelNotice::new(fake.clone(), dir.path());
        n.after_send(Some(Channel::Dev), Channel::Official, at(23), &mut |_| {});
        assert_eq!(fake.calls().len(), 1);
    }
}
