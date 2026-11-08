//! PC の送り先変更を保留し、401/到達不能の知らせを決める。

use anyhow::Context as _;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

pub const AUTH_AFTER: Duration = Duration::hours(1);
pub const RENOTIFY_AFTER: Duration = Duration::hours(24);
/// 「届かない」を知らせるまでの、設定を保存してからの時間（spec）。
pub const UNREACHABLE_AFTER: Duration = Duration::hours(24);

/// 知らせの文（spec と一字一句。試験が見る。review/code.md R10）。
pub const AUTH_TEXT: &str =
    "サーバに断られています。資格情報か送り先が違います。設定し直してください";
pub const UNREACHABLE_TEXT: &str = "送り先に届きません。設定を確かめてください";
pub const HOLD_TEXT: &str = "送り先の確認を待っています";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateAction {
    Send,
    Ask,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Auth,
    Unreachable,
}

pub trait DestinationToaster: std::fmt::Debug + Send + Sync {
    fn show_destination(&self, kind: NoticeKind, message: &str) -> anyhow::Result<()>;
    fn show_destination_hold(&self, message: &str) -> anyhow::Result<()>;
    /// 「送り先が設定されていません」（押すと `setup` の窓を開く）
    fn show_setup(&self) -> anyhow::Result<()>;
    /// 設定したら未設定の知らせを消す
    fn dismiss_setup(&self) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct Store(std::path::PathBuf);

impl Store {
    pub fn new(dir: &std::path::Path) -> Self {
        Self(dir.join("setup-state.json"))
    }
    /// 読む。壊れていたら [`Self::load_or_quarantine`] と同じく退避して進む（`setup` も収集も止めない）。
    pub fn load(&self) -> anyhow::Result<State> {
        Ok(self.load_or_quarantine()?.0)
    }
    /// 読む。**中身が壊れていたら退避して、承認した送り先の無い状態を書き直して進む**（review/code.md R26）——
    /// 起動を止めると収集が止まり、`setup` も同じところで落ちて立て直せない。ファイルは残すので門は閉じる側に倒れる
    /// （未送信があれば保留。承認は `setup` の y で取り直す）。戻り値の 2 つ目は退避したか。
    pub fn load_or_quarantine(&self) -> anyhow::Result<(State, bool)> {
        let text = match std::fs::read_to_string(&self.0) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok((State::default(), false))
            }
            Err(e) => return Err(e.into()),
        };
        if let Ok(state) = serde_json::from_str(&text) {
            return Ok((state, false));
        }
        std::fs::rename(&self.0, self.0.with_extension("broken.json"))
            .context("壊れた setup-state.json を退避できない")?;
        // 承認は失ったので保留から始め、すぐ「送り先の確認を待っています」を出す（黙って止めない。review/code.md R43）
        let state = State {
            destination_notified_at: Some(Utc::now()),
            ..State::default()
        };
        self.save(&state)?;
        Ok((state, true))
    }
    pub fn exists(&self) -> bool {
        self.0.exists()
    }
    pub fn save(&self, state: &State) -> anyhow::Result<()> {
        crate::fsutil::atomic_write(&self.0, &serde_json::to_vec(state)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct State {
    pub approved_url: Option<String>,
    pub destination_notified_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub destination_notice_at: Option<DateTime<Utc>>,
    pub auth_since: Option<DateTime<Utc>>,
    pub auth_notified_at: Option<DateTime<Utc>>,
    pub first_response_at: Option<DateTime<Utc>>,
    pub unreachable_since: Option<DateTime<Utc>>,
    pub unreachable_notified_at: Option<DateTime<Utc>>,
    /// 設定の文字列を保存した時刻。「届かない」の 24 時間はここから数える（spec。R10）
    #[serde(default)]
    pub setup_saved_at: Option<DateTime<Utc>>,
}

/// scheme/host の大文字、既定 port、path 末尾の `/` を揃えて比較する。
pub fn canonical_url(input: &str) -> String {
    let input = input.trim();
    let Some((scheme, rest)) = input.split_once("://") else {
        return input.to_ascii_lowercase();
    };
    let (authority, suffix) = rest.split_once('/').unwrap_or((rest, ""));
    let authority = authority.to_ascii_lowercase();
    let authority = authority
        .strip_suffix(":443")
        .filter(|_| scheme.eq_ignore_ascii_case("https"))
        .or_else(|| {
            authority
                .strip_suffix(":80")
                .filter(|_| scheme.eq_ignore_ascii_case("http"))
        })
        .unwrap_or(&authority);
    let path = format!("/{suffix}").trim_end_matches('/').to_string();
    format!(
        "{}://{}{}",
        scheme.to_ascii_lowercase(),
        authority,
        if path == "/" { "" } else { &path }
    )
}

pub fn gate(approved_url: Option<&str>, new_url: &str, pending: usize) -> GateAction {
    // 承認した送り先が無いときも聞く（バックアップから戻した・入れて初めて設定する）
    if approved_url.is_some_and(|old| canonical_url(old) == canonical_url(new_url)) || pending == 0
    {
        GateAction::Send
    } else {
        GateAction::Ask
    }
}

pub fn pending_in(dir: &std::path::Path) -> usize {
    ["outbox.jsonl", "heartbeat.jsonl", "drops.jsonl"]
        .iter()
        .map(|name| count_lines(&dir.join(name)))
        .sum()
}

/// 破棄の報告の未送信（`drops.jsonl`）の数。収集は記録と生存信号を手元に持つので、ここだけを置き場から数える。
pub fn pending_drops(dir: &std::path::Path) -> usize {
    count_lines(&dir.join("drops.jsonl"))
}

fn count_lines(path: &std::path::Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSummary {
    pub count: usize,
    pub oldest: Option<DateTime<Utc>>,
    pub newest: Option<DateTime<Utc>>,
}

pub fn pending_summary(dir: &std::path::Path) -> PendingSummary {
    let mut summary = PendingSummary {
        count: 0,
        oldest: None,
        newest: None,
    };
    for name in ["outbox.jsonl", "heartbeat.jsonl", "drops.jsonl"] {
        let Ok(text) = std::fs::read_to_string(dir.join(name)) else {
            continue;
        };
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            summary.count += 1;
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            // 生存信号は `emitted_at` だけを持つ（review/code.md R14）
            let at = [
                "event_time",
                "emitted_at",
                "at",
                "observed_at",
                "created_at",
            ]
            .iter()
            .find_map(|key| value.get(key)?.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|t| t.with_timezone(&Utc));
            if let Some(at) = at {
                summary.oldest = Some(summary.oldest.map_or(at, |old| old.min(at)));
                summary.newest = Some(summary.newest.map_or(at, |new| new.max(at)));
            }
        }
    }
    summary
}

pub fn allows(state: &State, url: &str, pending: usize) -> bool {
    gate(state.approved_url.as_deref(), url, pending) == GateAction::Send
}

pub fn answer(
    state: &mut State,
    new_url: &str,
    response: Option<bool>,
    pending: usize,
    now: DateTime<Utc>,
) -> GateAction {
    match gate(state.approved_url.as_deref(), new_url, pending) {
        GateAction::Send => {
            state.approved_url = Some(new_url.to_owned());
            state.destination_notified_at = None;
            GateAction::Send
        }
        GateAction::Ask if response == Some(true) => {
            state.approved_url = Some(new_url.to_owned());
            state.destination_notified_at = None;
            GateAction::Send
        }
        GateAction::Ask => {
            state.destination_notified_at.get_or_insert(now);
            state.destination_notice_at = None;
            GateAction::Hold
        }
        GateAction::Hold => GateAction::Hold,
    }
}

pub fn destination_notice(state: &State, now: DateTime<Utc>) -> bool {
    state.destination_notified_at.is_some()
        && state
            .destination_notice_at
            .is_none_or(|at| now - at >= RENOTIFY_AFTER)
}

pub fn observe_response(state: &mut State, status: Option<u16>, now: DateTime<Utc>) {
    match status {
        Some(401) => {
            state.auth_since.get_or_insert(now);
            state.first_response_at.get_or_insert(now);
            state.unreachable_since = None;
        }
        Some(status) => {
            state.first_response_at.get_or_insert(now);
            state.unreachable_since = None;
            if status < 500 {
                state.auth_since = None;
                state.auth_notified_at = None;
            }
        }
        None => {
            state.unreachable_since.get_or_insert(now);
        }
    }
}

pub fn notice_kind(state: &State, now: DateTime<Utc>) -> Option<NoticeKind> {
    if let Some(since) = state.auth_since {
        if now - since >= AUTH_AFTER
            && state
                .auth_notified_at
                .is_none_or(|at| now - at >= RENOTIFY_AFTER)
        {
            return Some(NoticeKind::Auth);
        }
    }
    // 「届かない」は、設定を保存してから答え（401・5xx を含む状態符号）が 1 度も返らないまま 24 時間（spec）。
    // 一度でも答えが返れば数えない。起点は保存の時刻で、保存し直すと数え直す（review/code.md R10）
    if let Some(saved) = state.setup_saved_at {
        if state.first_response_at.is_none()
            && now - saved >= UNREACHABLE_AFTER
            && state
                .unreachable_notified_at
                .is_none_or(|at| now - at >= RENOTIFY_AFTER)
        {
            return Some(NoticeKind::Unreachable);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn t(minutes: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::minutes(minutes)
    }

    #[test]
    // Scenario: 送ると答えれば送り始める
    fn st56_gate_sends_after_yes() {
        let mut s = State {
            approved_url: Some("https://a.example".into()),
            ..State::default()
        };
        assert_eq!(
            answer(&mut s, "https://b.example", Some(true), 1, t(0)),
            GateAction::Send
        );
    }

    #[test]
    // Scenario: 未送信があるまま送り先を変えると、答えるまで送らない
    fn st56_gate_different_destination_holds_until_answered() {
        let mut s = State {
            approved_url: Some("https://a.example".into()),
            ..State::default()
        };
        assert!(!allows(&s, "https://b.example", 1));
        assert_eq!(
            answer(&mut s, "https://b.example", None, 1, t(0)),
            GateAction::Hold
        );
        assert!(!allows(&s, "https://b.example", 1));
    }

    #[test]
    // Scenario: PC は未送信が無ければ、保存した送り先へそのまま送る
    fn st56_gate_sends_without_pending() {
        let mut s = State::default();
        assert_eq!(
            answer(&mut s, "https://b.example", None, 0, t(0)),
            GateAction::Send
        );
    }

    #[test]
    // Scenario: PC は保留の間、知らせで促す
    fn st56_gate_holds_and_notifies() {
        let mut s = State {
            approved_url: Some("https://a.example".into()),
            ..State::default()
        };
        assert_eq!(
            answer(&mut s, "https://b.example", Some(false), 1, t(0)),
            GateAction::Hold
        );
        assert!(s.destination_notified_at.is_some());
    }

    #[test]
    // Scenario: PC は 401 が 1 時間続くと知らせる
    fn st56_gate_auth_notice_after_hour() {
        let mut s = State::default();
        observe_response(&mut s, Some(401), t(0));
        assert_eq!(notice_kind(&s, t(59)), None);
        assert_eq!(notice_kind(&s, t(61)), Some(NoticeKind::Auth));
    }

    #[test]
    // Scenario: PC は断られ続ける間 24 時間ごとに出し直す
    fn st56_gate_auth_renotifies_daily() {
        let mut s = State::default();
        observe_response(&mut s, Some(401), t(0));
        s.auth_notified_at = Some(t(61));
        assert_eq!(notice_kind(&s, t(61 + 24 * 60 - 1)), None);
        assert_eq!(notice_kind(&s, t(61 + 24 * 60 + 1)), Some(NoticeKind::Auth));
    }

    #[test]
    // Scenario: PC は断られる状態を再起動をまたいで数える
    fn st56_gate_auth_state_survives_restart() {
        let mut s = State::default();
        observe_response(&mut s, Some(401), t(0));
        let restored = serde_json::from_str::<State>(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(notice_kind(&restored, t(61)), Some(NoticeKind::Auth));
    }

    #[test]
    // Scenario: PC は届けば数えをやめる
    fn st56_gate_success_resets_auth() {
        let mut s = State::default();
        observe_response(&mut s, Some(401), t(0));
        observe_response(&mut s, Some(200), t(61));
        observe_response(&mut s, Some(401), t(62));
        assert_eq!(notice_kind(&s, t(62 + 59)), None);
    }

    #[test]
    // Scenario: PC は一度も届かない送り先を 24 時間で知らせる
    fn st56_gate_unreachable_notice_after_day() {
        // 起点は保存の時刻。届かない失敗が後から始まっても、保存から数える
        let mut s = State {
            setup_saved_at: Some(t(0)),
            ..State::default()
        };
        observe_response(&mut s, None, t(600));
        assert_eq!(notice_kind(&s, t(24 * 60 - 1)), None);
        assert_eq!(
            notice_kind(&s, t(24 * 60 + 1)),
            Some(NoticeKind::Unreachable)
        );
        s.unreachable_notified_at = Some(t(24 * 60 + 1));
        assert_eq!(notice_kind(&s, t(48 * 60)), None);
        assert_eq!(
            notice_kind(&s, t(48 * 60 + 2)),
            Some(NoticeKind::Unreachable)
        );
    }

    #[test]
    fn st56_gate_unreachable_is_not_counted_once_answered() {
        // 一度でも答えが返った送り先は、後で届かなくなっても「届かない」と知らせない（R10）
        let mut s = State {
            setup_saved_at: Some(t(0)),
            ..State::default()
        };
        observe_response(&mut s, Some(200), t(1));
        observe_response(&mut s, None, t(2));
        assert_eq!(notice_kind(&s, t(48 * 60)), None);
        // 設定の保存が無い（印で渡した送り先）なら数えない
        let mut env = State::default();
        observe_response(&mut env, None, t(0));
        assert_eq!(notice_kind(&env, t(48 * 60)), None);
    }

    #[test]
    fn st56_gate_notice_texts_follow_spec() {
        assert_eq!(
            AUTH_TEXT,
            "サーバに断られています。資格情報か送り先が違います。設定し直してください"
        );
        assert_eq!(
            UNREACHABLE_TEXT,
            "送り先に届きません。設定を確かめてください"
        );
        assert_eq!(HOLD_TEXT, "送り先の確認を待っています");
    }

    #[test]
    fn st56_gate_pending_summary_reads_heartbeat_time() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("heartbeat.jsonl"),
            "{\"emitted_at\":\"2026-11-01T00:00:00Z\"}\n",
        )
        .unwrap();
        let summary = pending_summary(dir.path());
        assert_eq!(summary.count, 1);
        assert_eq!(summary.oldest, Some(t(0)));
    }

    #[test]
    // Scenario: PC は 401 だけを返す送り先を「届かない」と知らせない
    fn st56_gate_auth_is_not_unreachable() {
        let mut s = State::default();
        observe_response(&mut s, Some(401), t(0));
        assert_eq!(notice_kind(&s, t(24 * 60 + 1)), Some(NoticeKind::Auth));
    }

    #[test]
    fn st56_gate_server_error_keeps_auth_count() {
        let mut s = State::default();
        observe_response(&mut s, Some(401), t(0));
        observe_response(&mut s, Some(503), t(30));
        assert_eq!(s.auth_since, Some(t(0)));
        assert_eq!(notice_kind(&s, t(61)), Some(NoticeKind::Auth));
    }

    #[test]
    fn st56_gate_pending_notice_is_immediate_and_then_daily() {
        let mut s = State {
            destination_notified_at: Some(t(0)),
            ..State::default()
        };
        assert!(destination_notice(&s, t(0)));
        s.destination_notice_at = Some(t(0));
        assert!(!destination_notice(&s, t(24 * 60 - 1)));
        assert!(destination_notice(&s, t(24 * 60 + 1)));
    }

    #[test]
    fn st56_gate_corrupt_store_is_quarantined_and_stays_closed() {
        // review/code.md R26: 壊れた中身で止めない。退避して、承認の無い状態のファイルを残す（門は閉じる側）
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        std::fs::write(dir.path().join("setup-state.json"), "{壊れている").unwrap();
        let state = store.load().unwrap();
        assert_eq!(state.approved_url, None);
        assert!(
            destination_notice(&state, Utc::now()),
            "退避した後に保留を知らせない"
        );
        assert!(store.exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("setup-state.broken.json")).unwrap(),
            "{壊れている"
        );
        assert!(!allows(&store.load().unwrap(), "https://b.example", 1));
    }
}
