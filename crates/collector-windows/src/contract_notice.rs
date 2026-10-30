// SPDX-License-Identifier: AGPL-3.0-only
//! 「サーバを更新してください」の通知の決め方（ST50 / design D5・D22）。OS に依らない。
//!
//! 「S-01 の更新を待っている」状態（`contract_gate`）に入った契機で 1 回、待っている間は前に出してから
//! 24 時間経った契機で 1 回、状態を抜けたら消す。前に出した時刻は状態と同じ手元のファイルに持つ
//! （再起動しても 24 時間経つまで出し直さない）。OS の口は `toast.rs`（札 `contract`。途切れの札 `outage` と分ける）。
//!
//! **収集は止めない**: 通知を出せない・時刻を書けない、のどれもログに種別だけ残して続ける。
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::contract_gate::{ContractGate, Reasons};
use crate::telemetry;

/// 前に出してから出し直すまでの間隔（D5）。
const RENOTIFY_HOURS: i64 = 24;

const TITLE: &str = "ashiato";

/// 札 `contract` の通知の口（`toast.rs` が実装する）。途切れの通知の `Toaster` とは別の口。
pub trait ContractToaster: std::fmt::Debug + Send + Sync {
    /// 出す（前に出した `contract` の通知があれば置き換える）。
    fn show_contract(&self, xml: &str) -> anyhow::Result<()>;
    /// `contract` の通知だけを消す。
    fn dismiss_contract(&self) -> anyhow::Result<()>;
}

/// 通知に対してやること。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeAction {
    Nothing,
    Show,
    Dismiss,
}

/// 決め方。待っていて、まだ出していない・前に出してから 24 時間経った → 出す。待っていなくて出した覚えがある → 消す。
pub fn decide(
    waiting: bool,
    notified_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> NoticeAction {
    match (waiting, notified_at) {
        (true, None) => NoticeAction::Show,
        (true, Some(at)) if now - at >= Duration::hours(RENOTIFY_HOURS) => NoticeAction::Show,
        (true, Some(_)) | (false, None) => NoticeAction::Nothing,
        (false, Some(_)) => NoticeAction::Dismiss,
    }
}

/// 未送信の最も古い行を積んでからの日数（切り捨て。時計が戻っていれば 0）。
fn days_since(oldest: Option<DateTime<Utc>>, now: DateTime<Utc>) -> i64 {
    oldest.map_or(0, |o| (now - o).num_days().max(0))
}

/// 文言。理由が（e）だけのときは網の確認を添える（D22）。
pub fn message(reasons: Reasons, days: i64) -> String {
    let lead = if reasons.only_unreadable() {
        "サーバを更新するか、つながっている網を確かめてください"
    } else {
        "サーバを更新してください"
    };
    format!("{lead}（未送信 {days} 日分を PC に残しています）")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// トーストの XML。**押しても画面を開かない**（途切れの通知と同じ）。
pub fn toast_xml(message: &str) -> String {
    format!(
        r#"<toast><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual></toast>"#,
        escape_xml(TITLE),
        escape_xml(message),
    )
}

/// 送る契機ごとに 1 回呼ぶ通知係。
#[derive(Debug)]
pub struct ContractNotice {
    toaster: Arc<dyn ContractToaster>,
    /// 前の契機で出せなかった・消せなかった種別。**同じ種別が続く間はログに出さない**（final review R14）——
    /// 通知が切られた PC では契機ごと（5 分ごと）に同じ行が出続ける
    failing: std::sync::Mutex<Option<&'static str>>,
}

impl ContractNotice {
    pub fn new(toaster: Arc<dyn ContractToaster>) -> Self {
        Self {
            toaster,
            failing: std::sync::Mutex::new(None),
        }
    }

    /// 1 つの契機の後に呼ぶ。`oldest` は契約の版で手元に残している行の最も古いものを積んだ時刻。
    pub fn after_send(
        &self,
        gate: &mut ContractGate,
        now: DateTime<Utc>,
        oldest: Option<DateTime<Utc>>,
        log: &mut dyn FnMut(String),
    ) {
        // 再起動の後は理由がまだ空。読める答えを得るまで「待っていない」で通知を消さない
        if !gate.waiting() && !gate.reasons_known() {
            return;
        }
        let (result, next) = match decide(gate.waiting(), gate.notified_at(), now) {
            NoticeAction::Nothing => return,
            NoticeAction::Show => {
                let text = message(gate.reasons(), days_since(oldest, now));
                (self.toaster.show_contract(&toast_xml(&text)), Some(now))
            }
            NoticeAction::Dismiss => (self.toaster.dismiss_contract(), None),
        };
        // 出せなかった・消せなかったときは時刻を進めない（次の契機でやり直す）
        let result = result.and_then(|()| gate.set_notified_at(next));
        let kind = result.as_ref().err().map(telemetry::error_kind);
        let mut failing = self
            .failing
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // 出せない状態に入った・種別が変わった契機でだけ残す（R14）
        if kind.is_some() && *failing != kind {
            log(telemetry::line("contract_notice_skipped", None, None, kind));
        }
        *failing = kind;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::Mutex;

    use super::*;
    use crate::sender::{ContractAnswer, SendOutcome};

    fn t0() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-30T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn at(minutes: i64) -> DateTime<Utc> {
        t0() + Duration::minutes(minutes)
    }

    /// 札ごとに控える偽の通知の口。`contract` と `outage` を同じ控えに載せて札の違いを見る。
    #[derive(Debug, Default)]
    struct Fake {
        log: Mutex<Vec<(&'static str, String)>>,
    }

    impl Fake {
        fn shown(&self, tag: &str) -> Vec<String> {
            self.log
                .lock()
                .unwrap()
                .iter()
                .filter(|(t, x)| *t == tag && !x.is_empty())
                .map(|(_, x)| x.clone())
                .collect()
        }
        fn dismissed(&self, tag: &str) -> usize {
            self.log
                .lock()
                .unwrap()
                .iter()
                .filter(|(t, x)| *t == tag && x.is_empty())
                .count()
        }
    }

    impl ContractToaster for Fake {
        fn show_contract(&self, xml: &str) -> anyhow::Result<()> {
            self.log.lock().unwrap().push(("contract", xml.to_string()));
            Ok(())
        }
        fn dismiss_contract(&self) -> anyhow::Result<()> {
            self.log.lock().unwrap().push(("contract", String::new()));
            Ok(())
        }
    }

    impl crate::outage_notice::Toaster for Fake {
        fn show(&self, xml: &str, _sound: bool) -> anyhow::Result<()> {
            self.log.lock().unwrap().push(("outage", xml.to_string()));
            Ok(())
        }
        fn dismiss(&self) -> anyhow::Result<()> {
            self.log.lock().unwrap().push(("outage", String::new()));
            Ok(())
        }
    }

    struct Rig {
        dir: tempfile::TempDir,
        fake: Arc<Fake>,
        gate: ContractGate,
        notice: ContractNotice,
    }

    impl Rig {
        fn new() -> Self {
            let dir = tempfile::TempDir::new().unwrap();
            let fake = Arc::new(Fake::default());
            let gate = ContractGate::new(dir.path());
            let notice = ContractNotice::new(fake.clone());
            Self {
                dir,
                fake,
                gate,
                notice,
            }
        }

        /// 再起動（手元のファイルを残したまま作り直す）。
        fn restart(&mut self) {
            self.gate = ContractGate::new(self.dir.path());
            self.notice = ContractNotice::new(self.fake.clone());
        }

        /// 1 つの契機。行は t0 に積んだ 1 件だけ残っている。
        fn trigger(
            &mut self,
            answer: ContractAnswer,
            now: DateTime<Utc>,
            outcomes: &[SendOutcome],
        ) {
            let mut log = |_: String| {};
            self.gate.observe(answer, now, outcomes, &mut log);
            self.notice
                .after_send(&mut self.gate, now, Some(t0()), &mut log);
        }

        fn ahead(&mut self, now: DateTime<Utc>) {
            self.trigger(ContractAnswer::Version(1), now, &[ahead_outcome()]);
        }
    }

    fn ahead_outcome() -> SendOutcome {
        SendOutcome {
            posted: true,
            ahead: true,
            ..SendOutcome::default()
        }
    }

    /// 出せない間は契機ごとにログを出さず、出せない状態に入った契機で 1 行だけ残す（final review R14）。
    #[test]
    fn st50_notice_skipped_logged_once_per_failure() {
        #[derive(Debug, Default)]
        struct Broken {
            fail: std::sync::atomic::AtomicBool,
            shown: std::sync::atomic::AtomicUsize,
        }
        impl ContractToaster for Broken {
            fn show_contract(&self, _xml: &str) -> anyhow::Result<()> {
                if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                    anyhow::bail!("toast off");
                }
                self.shown.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
            fn dismiss_contract(&self) -> anyhow::Result<()> {
                Ok(())
            }
        }
        let dir = tempfile::TempDir::new().unwrap();
        let toaster = Arc::new(Broken::default());
        toaster
            .fail
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let notice = ContractNotice::new(toaster.clone());
        let mut gate = ContractGate::new(dir.path());
        let mut lines = Vec::new();
        let tick = |gate: &mut ContractGate, m: i64, lines: &mut Vec<String>| {
            let mut log = |l: String| lines.push(l);
            gate.observe(
                ContractAnswer::Version(1),
                at(m),
                &[ahead_outcome()],
                &mut log,
            );
            notice.after_send(gate, at(m), Some(t0()), &mut log);
        };
        for m in (0..60).step_by(5) {
            tick(&mut gate, m, &mut lines);
        }
        let skipped = |lines: &[String]| {
            lines
                .iter()
                .filter(|l| l.contains("contract_notice_skipped"))
                .count()
        };
        assert_eq!(skipped(&lines), 1, "{lines:?}");
        // 出せるようになったら出す。その後また出せなくなったら（24 時間後の出し直し）もう 1 行
        toaster
            .fail
            .store(false, std::sync::atomic::Ordering::SeqCst);
        tick(&mut gate, 65, &mut lines);
        assert_eq!(toaster.shown.load(std::sync::atomic::Ordering::SeqCst), 1);
        toaster
            .fail
            .store(true, std::sync::atomic::Ordering::SeqCst);
        tick(&mut gate, 65 + 24 * 60, &mut lines);
        tick(&mut gate, 70 + 24 * 60, &mut lines);
        assert_eq!(skipped(&lines), 2, "{lines:?}");
    }

    // Scenario: 契約の版で止まると PC に通知が 1 回出る
    #[test]
    fn st50_notice_first_time() {
        let mut r = Rig::new();
        r.ahead(at(0));
        let shown = r.fake.shown("contract");
        assert_eq!(shown.len(), 1);
        assert!(shown[0].contains("サーバを更新してください（未送信 0 日分を PC に残しています）"));
        // 同じ契機がもう一度来ても出し直さない
        r.ahead(at(5));
        assert_eq!(r.fake.shown("contract").len(), 1);
    }

    // Scenario: PC の通知は止まっている間 1 日に 1 回出直す
    #[test]
    fn st50_notice_daily() {
        let mut r = Rig::new();
        for m in (0..=2 * 24 * 60).step_by(5) {
            r.ahead(at(m));
        }
        let shown = r.fake.shown("contract");
        assert_eq!(shown.len(), 3, "{shown:?}");
        for (days, text) in shown.iter().enumerate() {
            assert!(text.contains(&format!("未送信 {days} 日分")), "{text}");
        }
    }

    // Scenario: 契約の版で止まると PC に通知が 1 回出る
    #[test]
    fn st50_notice_restart_then_transient_keeps_notified_at() {
        let mut r = Rig::new();
        r.ahead(at(0));
        r.restart();
        r.trigger(ContractAnswer::Transient, at(5), &[]);
        assert_eq!(r.fake.dismissed("contract"), 0);
        r.ahead(at(10));
        assert_eq!(r.fake.shown("contract").len(), 1, "再起動で出し直した");
    }

    // Scenario: 更新を促す通知は途切れの通知を置き換えない
    #[test]
    fn st50_notice_separate_tag() {
        use crate::outage_notice::Toaster as _;
        let mut r = Rig::new();
        r.fake.show("<toast/>", true).unwrap();
        r.ahead(at(0));
        let tags: Vec<&str> = r.fake.log.lock().unwrap().iter().map(|(t, _)| *t).collect();
        assert_eq!(tags, vec!["outage", "contract"]);
        // 更新を促す通知を消しても、途切れの通知には触れない
        r.trigger(ContractAnswer::Version(1), at(1), &[]);
        assert_eq!(r.fake.dismissed("contract"), 1);
        assert_eq!(r.fake.dismissed("outage"), 0);
    }

    // Scenario: S-01 を更新すると PC の更新を促す通知が消える
    #[test]
    fn st50_notice_cleared() {
        let mut r = Rig::new();
        r.ahead(at(0));
        assert_eq!(r.fake.dismissed("contract"), 0);
        r.trigger(
            ContractAnswer::Version(1),
            at(5),
            &[SendOutcome {
                posted: true,
                ..SendOutcome::default()
            }],
        );
        assert_eq!(r.fake.dismissed("contract"), 1);
        // 消した後は何度来ても消さない・出さない
        r.trigger(ContractAnswer::Version(1), at(10), &[]);
        assert_eq!(r.fake.dismissed("contract"), 1);
        assert_eq!(r.fake.shown("contract").len(), 1);
        // 次に止まったら、24 時間を待たずに出す
        r.ahead(at(15));
        assert_eq!(r.fake.shown("contract").len(), 2);
    }

    // Scenario: PC が到達できないだけでは更新を促さない
    #[test]
    fn st50_notice_not_for_transient() {
        let mut r = Rig::new();
        for m in (0..3 * 24 * 60).step_by(60) {
            r.trigger(ContractAnswer::Transient, at(m), &[]);
        }
        assert!(r.fake.log.lock().unwrap().is_empty());
    }

    // Scenario: PC は読めない 200 の答えが 24 時間続くまでは更新を促さない
    #[test]
    fn st50_notice_not_for_unreadable_200_under_24h() {
        let mut r = Rig::new();
        for m in (0..24 * 60).step_by(30) {
            r.trigger(ContractAnswer::Unreadable, at(m), &[]);
        }
        assert!(r.fake.log.lock().unwrap().is_empty());
    }

    // Scenario: PC は読めない 200 の答えが 24 時間続くと網の確認を添えて更新を促す
    #[test]
    fn st50_notice_unreadable_200_after_24h() {
        let mut r = Rig::new();
        r.trigger(ContractAnswer::Unreadable, at(0), &[]);
        r.trigger(ContractAnswer::Unreadable, at(24 * 60), &[]);
        let shown = r.fake.shown("contract");
        assert_eq!(shown.len(), 1);
        assert!(
            shown[0].contains("サーバを更新するか、つながっている網を確かめてください（未送信 1 日分を PC に残しています）"),
            "{}",
            shown[0]
        );
    }

    // Scenario: PC の再起動と到達できない間が挟まっても読めない 200 の数え始めは変わらない
    #[test]
    fn st50_notice_unreadable_200_survives_restart_and_transient() {
        let mut r = Rig::new();
        r.trigger(ContractAnswer::Unreadable, at(0), &[]);
        r.restart();
        // 到達できない区間と 503 の区間（どちらも読める答えではない）を挟む
        r.trigger(ContractAnswer::Transient, at(10 * 60), &[]);
        r.trigger(ContractAnswer::Transient, at(12 * 60), &[]);
        r.restart();
        r.trigger(ContractAnswer::Transient, at(14 * 60), &[]);
        assert!(r.fake.log.lock().unwrap().is_empty());
        // 数え始めから 24 時間後の読めない答えで（e）に入り、網の確認を添えて出す
        r.trigger(ContractAnswer::Unreadable, at(24 * 60), &[]);
        let shown = r.fake.shown("contract");
        assert_eq!(shown.len(), 1);
        assert!(shown[0].contains("つながっている網を確かめてください"));
    }

    // Scenario: PC は読める答えを 1 回得ると読めない 200 の数え始めをやり直す
    #[test]
    fn st50_notice_unreadable_200_reset_by_readable() {
        let mut r = Rig::new();
        r.trigger(ContractAnswer::Unreadable, at(0), &[]);
        r.trigger(ContractAnswer::Version(1), at(20 * 60), &[]);
        // 数え始めは捨てられた。最初の読めない答えから 24 時間が過ぎても促さない
        r.trigger(ContractAnswer::Unreadable, at(25 * 60), &[]);
        r.trigger(ContractAnswer::Unreadable, at(30 * 60), &[]);
        assert!(r.fake.log.lock().unwrap().is_empty());
        r.trigger(ContractAnswer::Unreadable, at(25 * 60 + 24 * 60), &[]);
        assert_eq!(r.fake.shown("contract").len(), 1);
    }

    // Scenario: PC が再起動しても 24 時間経つまで通知は出直さない
    #[test]
    fn st50_notice_survives_restart() {
        let mut r = Rig::new();
        r.ahead(at(0));
        r.restart();
        // 再起動で版 > A の印は消えているが、同じ理由で止まっていて、前に出した時刻だけが残る
        r.ahead(at(60));
        assert_eq!(r.fake.shown("contract").len(), 1, "再起動で出し直した");
        r.restart();
        r.ahead(at(24 * 60));
        assert_eq!(r.fake.shown("contract").len(), 2);
    }
}
