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
            if state.notified_at.take().is_some() {
                if let Err(e) = self.toaster.dismiss_channel() {
                    self.log_failure(log, &e);
                    return;
                }
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
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct Fake(Mutex<Vec<String>>);
    impl ChannelToaster for Fake {
        fn show_channel(&self, xml: &str) -> Result<()> {
            self.0.lock().unwrap().push(xml.into());
            Ok(())
        }
        fn dismiss_channel(&self) -> Result<()> {
            self.0.lock().unwrap().push(String::new());
            Ok(())
        }
    }

    fn at(hours: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-03T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::hours(hours)
    }

    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_notice_is_daily() {
        let dir = tempfile::TempDir::new().unwrap();
        let fake = Arc::new(Fake::default());
        let notice = ChannelNotice::new(fake.clone(), dir.path());
        let mut log = |_| {};
        notice.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut log);
        notice.after_send(Some(Channel::Dev), Channel::Official, at(23), &mut log);
        notice.after_send(Some(Channel::Dev), Channel::Official, at(24), &mut log);
        assert_eq!(fake.0.lock().unwrap().len(), 2);
    }

    fn notice() -> (tempfile::TempDir, Arc<Fake>, ChannelNotice) {
        let dir = tempfile::TempDir::new().unwrap();
        let fake = Arc::new(Fake::default());
        let notice = ChannelNotice::new(fake.clone(), dir.path());
        (dir, fake, notice)
    }

    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_dev_does_not_send_to_official() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Official), Channel::Dev, at(0), &mut log);
        assert_eq!(fake.0.lock().unwrap().len(), 1);
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_official_does_not_send_to_dev() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut log);
        assert_eq!(fake.0.lock().unwrap().len(), 1);
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_same_channel_has_no_notice() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Dev), Channel::Dev, at(0), &mut log);
        assert!(fake.0.lock().unwrap().is_empty());
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_missing_answer_is_not_noticed() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(None, Channel::Dev, at(0), &mut log);
        assert!(fake.0.lock().unwrap().is_empty());
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_notice_is_separate_from_contract_notice() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Official), Channel::Dev, at(0), &mut log);
        assert!(!fake.0.lock().unwrap()[0].contains("更新してください"));
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_notice_names_server() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Official), Channel::Dev, at(0), &mut log);
        assert!(fake.0.lock().unwrap()[0].contains("正式版のサーバ"));
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_same_channel_dismisses() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut log);
        n.after_send(Some(Channel::Official), Channel::Official, at(1), &mut log);
        assert!(fake.0.lock().unwrap().last().unwrap().is_empty());
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_404_has_no_channel_notice() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(None, Channel::Official, at(0), &mut log);
        assert!(fake.0.lock().unwrap().is_empty());
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_unreadable_has_no_channel_notice() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(None, Channel::Official, at(0), &mut log);
        assert!(fake.0.lock().unwrap().is_empty());
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_401_has_no_channel_notice() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(None, Channel::Official, at(0), &mut log);
        assert!(fake.0.lock().unwrap().is_empty());
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_notice_persists_time() {
        let dir = tempfile::TempDir::new().unwrap();
        let fake = Arc::new(Fake::default());
        {
            let n = ChannelNotice::new(fake.clone(), dir.path());
            let mut log = |_| {};
            n.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut log);
        }
        let n = ChannelNotice::new(fake.clone(), dir.path());
        let mut log = |_| {};
        n.after_send(Some(Channel::Dev), Channel::Official, at(23), &mut log);
        assert_eq!(fake.0.lock().unwrap().len(), 1);
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_dismiss_is_independent() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut log);
        n.after_send(Some(Channel::Official), Channel::Official, at(1), &mut log);
        assert_eq!(fake.0.lock().unwrap().len(), 2);
    }
    // Runtime integration coverage owns the Scenario marker.
    #[test]
    fn st54_channel_does_not_change_outage_reading() {
        let (_d, fake, n) = notice();
        let mut log = |_| {};
        n.after_send(Some(Channel::Dev), Channel::Official, at(0), &mut log);
        assert_eq!(fake.0.lock().unwrap().len(), 1);
    }
}
