// SPDX-License-Identifier: AGPL-3.0-only
//! 「S-01 の更新を待っている」状態（ST50 / design D9 の（a）〜（e））。
//!
//! 送る契機ごとに、`GET /contract` の答えと 2 つの送り手（記録・生存信号）の結果から理由を決める。
//! 理由が 1 つでもあれば待っている。**待っていても行は捨てない**（D17）。知らせ（ST50 Task 9）はこの状態を読む。
//!
//! 200 の読めない答えが続いた数え始めの時刻は、再起動をまたぐ手元のファイルに持つ（D22・D24（仮））。
//! 数え直すのは読める答え（版 A）を得た契機だけ。届かない・時間切れ・5xx・404 が挟まっても捨てない。
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::fsutil;
use crate::sender::{ContractAnswer, SendOutcome};
use crate::telemetry;

/// 200 の読めない答えがこれだけ続いたら、更新を待っている状態に入る（D22）。
pub const UNREADABLE_LIMIT_HOURS: i64 = 24;

/// 手元のファイルに持つもの（数え始めと、更新を促す通知を前に出した時刻。D5・D22）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitState {
    /// 200 の読めない答えを最初に得た時刻。読める答えを得たら消える
    #[serde(default)]
    pub unreadable_since: Option<DateTime<Utc>>,
    /// 更新を促す通知を前に出した時刻。通知を消したら消える
    #[serde(default)]
    pub notified_at: Option<DateTime<Utc>>,
}

/// 待っている理由（D9 の（a）〜（e））。
// 設計の D9 の理由が（a）〜（e）と別々の事実なので、状態を持つ型にまとめず 1 つずつ持つ
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reasons {
    /// （a）版 > A の行が未送信にある
    pub ahead: bool,
    /// （b）問い合わせの口が 404 を返した
    pub no_endpoint: bool,
    /// （c）直前の送信が 426 だった
    pub refused: bool,
    /// （d）直前の送信で 1 件以上が `unknown_field` か `contract_unsupported` だった
    pub rejected: bool,
    /// （e）200 の読めない答えが 24 時間続いた
    pub unreadable_long: bool,
}

impl Reasons {
    /// 待っているか。
    pub fn waiting(&self) -> bool {
        self.ahead || self.no_endpoint || self.refused || self.rejected || self.unreadable_long
    }

    /// 理由が（e）だけか（知らせの文言を分ける。D22）。
    pub fn only_unreadable(&self) -> bool {
        self.unreadable_long && !(self.ahead || self.no_endpoint || self.refused || self.rejected)
    }
}

/// 状態の置き場（`contract-wait.json`）。
#[derive(Debug, Clone)]
pub struct WaitStore {
    path: std::path::PathBuf,
}

impl WaitStore {
    pub fn new(state_dir: &std::path::Path) -> Self {
        Self {
            path: state_dir.join("contract-wait.json"),
        }
    }

    /// 無い・読めないときは空（数え始めが無いものとして扱う）。
    pub fn load(&self) -> WaitState {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, state: &WaitState) -> anyhow::Result<()> {
        fsutil::atomic_write(&self.path, &serde_json::to_vec(state)?)
    }
}

/// 待っている状態を持つ係。
#[derive(Debug)]
pub struct ContractGate {
    store: WaitStore,
    state: WaitState,
    reasons: Reasons,
    /// 起動してから `Version` の答えを 1 度でも得たか（理由はメモリにしか無いので、それまで「待っていない」は当てにならない）。
    reasons_known: bool,
}

impl ContractGate {
    pub fn new(state_dir: &std::path::Path) -> Self {
        let store = WaitStore::new(state_dir);
        let state = store.load();
        Self {
            store,
            state,
            reasons: Reasons::default(),
            reasons_known: false,
        }
    }

    /// 理由を引き直せたか。偽の間は「待っていない」を、通知を消す根拠にしない。
    pub fn reasons_known(&self) -> bool {
        self.reasons_known
    }

    pub fn reasons(&self) -> Reasons {
        self.reasons
    }

    pub fn waiting(&self) -> bool {
        self.reasons.waiting()
    }

    /// 更新を促す通知を前に出した時刻。
    pub fn notified_at(&self) -> Option<DateTime<Utc>> {
        self.state.notified_at
    }

    /// 通知を出した（消した）時刻を手元のファイルに書く。書けなければ `Err`（呼び手は出し直しに任せる）。
    pub fn set_notified_at(&mut self, at: Option<DateTime<Utc>>) -> anyhow::Result<()> {
        let before = self.state.notified_at;
        self.state.notified_at = at;
        self.store.save(&self.state).inspect_err(|_| {
            self.state.notified_at = before;
        })
    }

    /// 1 つの契機の結果を取り込む。**待っている状態を抜けたら真**（呼び手は「飛ばす」印を捨てる。D14）。
    ///
    /// 一時的な失敗の契機は何も変えない（更新が原因ではないので、始めも終わりもしない）。
    pub fn observe(
        &mut self,
        answer: ContractAnswer,
        now: DateTime<Utc>,
        outcomes: &[SendOutcome],
        log: &mut dyn FnMut(String),
    ) -> bool {
        let was = self.reasons.waiting();
        let before = self.state.clone();
        match answer {
            ContractAnswer::Transient => return false,
            ContractAnswer::Version(_) => {
                self.reasons_known = true;
                self.state.unreadable_since = None;
                self.reasons.no_endpoint = false;
                self.reasons.unreadable_long = false;
                self.reasons.ahead = outcomes.iter().any(|o| o.ahead);
                // 送っていなければ、断られた行は無い（送れる行が無い）
                let posted = outcomes.iter().any(|o| o.posted);
                self.reasons.refused = posted && outcomes.iter().any(|o| o.refused);
                self.reasons.rejected = posted && outcomes.iter().any(|o| o.rejected_by_version);
            }
            ContractAnswer::NoEndpoint => {
                // A を得られない契機は理由を足すだけ。落とすのは Version の分岐だけ（D9）
                self.reasons.no_endpoint = true;
            }
            ContractAnswer::Unreadable => {
                let since = *self.state.unreadable_since.get_or_insert(now);
                self.reasons.unreadable_long =
                    now - since >= Duration::hours(UNREADABLE_LIMIT_HOURS);
            }
        }
        if self.state != before {
            if let Err(e) = self.store.save(&self.state) {
                log(telemetry::line(
                    "contract_wait_save_failed",
                    None,
                    None,
                    Some(telemetry::error_kind(&e)),
                ));
            }
        }
        was && !self.reasons.waiting()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn t(h: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-30T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::hours(h)
    }

    fn quiet() -> impl FnMut(String) {
        |_| {}
    }

    #[test]
    fn st50_gate_unreadable_waits_only_after_24_hours() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut g = ContractGate::new(dir.path());
        let mut log = quiet();
        g.observe(ContractAnswer::Unreadable, t(0), &[], &mut log);
        assert!(!g.waiting(), "最初の読めない答えですぐ促した");
        g.observe(ContractAnswer::Unreadable, t(23), &[], &mut log);
        assert!(!g.waiting());
        g.observe(ContractAnswer::Unreadable, t(24), &[], &mut log);
        assert!(g.waiting());
        assert!(g.reasons().only_unreadable());
    }

    #[test]
    fn st50_gate_count_survives_restart_and_only_a_readable_answer_resets_it() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut log = quiet();
        ContractGate::new(dir.path()).observe(ContractAnswer::Unreadable, t(0), &[], &mut log);
        // 再起動（新しい係）をまたいで数える。届かない・404 が挟まっても数え直さない
        let mut g = ContractGate::new(dir.path());
        g.observe(ContractAnswer::Transient, t(10), &[], &mut log);
        g.observe(ContractAnswer::NoEndpoint, t(12), &[], &mut log);
        assert!(g.reasons().no_endpoint && !g.reasons().unreadable_long);
        g.observe(ContractAnswer::Unreadable, t(25), &[], &mut log);
        assert!(g.reasons().unreadable_long, "24 時間の数えが再起動で消えた");
        // 読める答えで数え直し、状態を抜ける
        let left = g.observe(ContractAnswer::Version(1), t(26), &[], &mut log);
        assert!(left && !g.waiting());
        assert_eq!(WaitStore::new(dir.path()).load(), WaitState::default());
    }

    #[test]
    fn st50_gate_reasons_a_to_d_and_leaving() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut g = ContractGate::new(dir.path());
        let mut log = quiet();
        let posted = SendOutcome {
            posted: true,
            ..SendOutcome::default()
        };
        for (outcome, name) in [
            (
                SendOutcome {
                    ahead: true,
                    ..posted
                },
                "a",
            ),
            (
                SendOutcome {
                    refused: true,
                    ..posted
                },
                "c",
            ),
            (
                SendOutcome {
                    rejected_by_version: true,
                    ..posted
                },
                "d",
            ),
        ] {
            g.observe(ContractAnswer::Version(1), t(0), &[outcome], &mut log);
            assert!(g.waiting(), "{name}");
        }
        // 一時的な失敗は始めも終わりもしない
        assert!(!g.observe(ContractAnswer::Transient, t(1), &[], &mut log));
        assert!(g.waiting());
        // 何も引っかからない契機で抜ける
        assert!(g.observe(ContractAnswer::Version(2), t(2), &[posted], &mut log));
        assert!(!g.waiting());
    }

    #[test]
    fn st50_gate_transient_alone_never_holds() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut g = ContractGate::new(dir.path());
        let mut log = quiet();
        for h in 0..100 {
            g.observe(ContractAnswer::Transient, t(h), &[], &mut log);
        }
        assert!(!g.waiting());
    }

    #[test]
    fn st50_gate_unreadable_does_not_leave_a_wait_it_cannot_resolve() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut g = ContractGate::new(dir.path());
        let mut log = quiet();
        g.observe(ContractAnswer::NoEndpoint, t(0), &[], &mut log);
        assert!(g.reasons().no_endpoint);
        // A を得られない契機は理由を足すだけ。待つ状態を抜けたことにしない
        let left = g.observe(ContractAnswer::Unreadable, t(1), &[], &mut log);
        assert!(!left && g.reasons().no_endpoint && g.waiting());
        let left = g.observe(ContractAnswer::NoEndpoint, t(2), &[], &mut log);
        assert!(!left && g.waiting());
        let left = g.observe(ContractAnswer::Version(1), t(3), &[], &mut log);
        assert!(left && !g.waiting());
    }
}
