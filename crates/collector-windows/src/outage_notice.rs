// SPDX-License-Identifier: AGPL-3.0-only
//! 途切れの通知の決め方と問い合わせ（ST14 / design D11・D13・D14（仮））。
//!
//! 携帯（Kotlin の `OutageNotice`）と**同じ表**を書く。試験の印（Scenario 名）も同じものを置く。
//! OS の口（Windows のトースト）は `toast.rs`。ここは純粋で、ubuntu でも走る。
//!
//! **収集は止めない**: 問い合わせの失敗・通知を出せない・覚えを書けない、のどれも
//! ログに種別だけ残して続ける。記録の中身は載せない（C5）—— 通知は表示名と日時と定まった文言だけ。
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::Context as _;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::fsutil;
use crate::sender::Reply;
use crate::telemetry;

/// 通知の題（D14（仮））。
pub const TITLE: &str = "収集が途切れています";

/// 途切れているソース 1 つ。**記録の中身は持たない。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outage {
    pub episode_id: i64,
    pub logical_source: String,
    pub display_name: String,
    pub last_alive_at: DateTime<Utc>,
}

/// `GET /outages` の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutageResult {
    /// サーバに届かなかった・読めなかった
    Unreachable,
    /// サーバが起動直後で判定を待っている
    WarmingUp,
    Ready(Vec<Outage>),
}

/// 通知について覚えていること。`sounded` は音を鳴らした途切れの識別子、`shown` は通知が出ているか。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoticeState {
    pub sounded: BTreeSet<i64>,
    pub shown: bool,
}

/// 通知に対してやること。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeAction {
    Nothing,
    /// 黙って消す
    Dismiss,
    /// 全部の表示名を並べて出す。`sound` が false なら音なしの差し替え
    Show {
        outages: Vec<Outage>,
        sound: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeDecision {
    pub action: NoticeAction,
    pub next: NoticeState,
}

/// 通知の決め方（design D13 の表）。
///
/// 次の `sounded` を「いまの一覧の識別子」にするので、戻った途切れの識別子は消え、
/// 戻った後にまた途切れると新しい識別子になってまた鳴る。
pub fn decide(state: &NoticeState, result: &OutageResult) -> NoticeDecision {
    match result {
        OutageResult::Unreachable | OutageResult::WarmingUp => NoticeDecision {
            action: NoticeAction::Nothing,
            next: state.clone(),
        },
        OutageResult::Ready(outages) if outages.is_empty() => NoticeDecision {
            action: if state.shown {
                NoticeAction::Dismiss
            } else {
                NoticeAction::Nothing
            },
            next: NoticeState::default(),
        },
        OutageResult::Ready(outages) => {
            let ids: BTreeSet<i64> = outages.iter().map(|o| o.episode_id).collect();
            let sound = ids.iter().any(|id| !state.sounded.contains(id));
            NoticeDecision {
                action: NoticeAction::Show {
                    outages: outages.clone(),
                    sound,
                },
                next: NoticeState {
                    sounded: ids,
                    shown: true,
                },
            }
        }
    }
}

/// `Asia/Tokyo`（`DAY_TZ`）は夏時間が無く、UTC+9 の固定でよい。
const DAY_TZ_OFFSET_HOURS: i64 = 9;

/// 本文: `位置（最後 10/12 08:15）、写真（最後 10/11 22:00）`（D14（仮））。
pub fn body(outages: &[Outage]) -> String {
    outages
        .iter()
        .map(|o| {
            format!(
                "{}（最後 {}）",
                o.display_name,
                (o.last_alive_at + Duration::hours(DAY_TZ_OFFSET_HOURS)).format("%m/%d %H:%M")
            )
        })
        .collect::<Vec<_>>()
        .join("、")
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// トーストの XML。音を鳴らさないときは `<audio silent="true"/>`。
/// **押しても画面を開かない**（`launch` を付けない。D14（仮））。
pub fn toast_xml(outages: &[Outage], sound: bool) -> String {
    let audio = if sound {
        ""
    } else {
        r#"<audio silent="true"/>"#
    };
    format!(
        r#"<toast><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual>{audio}</toast>"#,
        escape_xml(TITLE),
        escape_xml(&body(outages)),
    )
}

#[derive(Deserialize)]
struct OutagesDto {
    status: String,
    #[serde(default)]
    outages: Vec<OutageDto>,
}

#[derive(Deserialize)]
struct OutageDto {
    episode_id: i64,
    logical_source: String,
    display_name: String,
    last_alive_at: DateTime<Utc>,
}

/// 応答を結果にする。2xx 以外・読めない本文・知らない状態は `Unreachable`。
pub fn parse_reply(reply: &Reply) -> OutageResult {
    if !(200..300).contains(&reply.status) {
        return OutageResult::Unreachable;
    }
    let Ok(dto) = serde_json::from_str::<OutagesDto>(&reply.body) else {
        return OutageResult::Unreachable;
    };
    match dto.status.as_str() {
        "warming_up" => OutageResult::WarmingUp,
        "ready" => OutageResult::Ready(
            dto.outages
                .into_iter()
                .map(|o| Outage {
                    episode_id: o.episode_id,
                    logical_source: o.logical_source,
                    display_name: o.display_name,
                    last_alive_at: o.last_alive_at,
                })
                .collect(),
        ),
        _ => OutageResult::Unreachable,
    }
}

/// `GET /outages?device_id=` の 1 回の問い合わせ。試験では偽物に差し替える。
/// **実装は 10 秒で打ち切る**（`sender::HttpOutageQuery`）。
pub trait OutageQuery: std::fmt::Debug + Send + Sync {
    fn fetch(&self, device_id: &str) -> OutageResult;
}

/// OS の通知の口（`toast.rs` が実装する）。
pub trait Toaster: std::fmt::Debug + Send + Sync {
    /// 出す（同じものがあれば差し替える）。`sound` が false なら画面の端には出さない。
    fn show(&self, xml: &str, sound: bool) -> anyhow::Result<()>;
    /// 黙って消す。
    fn dismiss(&self) -> anyhow::Result<()>;
}

/// 鳴らした途切れの覚え（`outage-notice.json`。収集アプリの状態の置き場）。
#[derive(Debug, Clone)]
pub struct NoticeStore {
    path: PathBuf,
}

impl NoticeStore {
    pub fn new(state_dir: &std::path::Path) -> Self {
        Self {
            path: state_dir.join("outage-notice.json"),
        }
    }

    /// 無い・読めないときは空（鳴っていないものとして扱う）。
    pub fn load(&self) -> NoticeState {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, state: &NoticeState) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec(state).context("覚えを直列化できない")?;
        fsutil::atomic_write(&self.path, &bytes)
    }
}

struct Inner {
    query: Arc<dyn OutageQuery>,
    toaster: Arc<dyn Toaster>,
    store: NoticeStore,
    log: fn(String),
}

impl Inner {
    /// 問い合わせ → 決め方 → 通知 → 覚えの書き込み。**失敗はログに種別だけ残して返る。**
    fn poll_once(&self, device_id: &str) {
        let decision = decide(&self.store.load(), &self.query.fetch(device_id));
        let shown = match &decision.action {
            NoticeAction::Nothing => Ok(()),
            NoticeAction::Dismiss => self.toaster.dismiss(),
            NoticeAction::Show { outages, sound } => {
                self.toaster.show(&toast_xml(outages, *sound), *sound)
            }
        };
        if let Err(e) = shown {
            // 出せなかったときは覚えを進めない（出せるようになったら鳴る）
            (self.log)(telemetry::line(
                "outage_notice_skipped",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
            return;
        }
        if let Err(e) = self.store.save(&decision.next) {
            (self.log)(telemetry::line(
                "outage_notice_state_unwritable",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
        }
    }
}

/// 時計の測定の契機で 1 回呼ばれる問い合わせ係（design D11）。
///
/// 問い合わせは別スレッドで行い、見回りを待たせない。前の問い合わせが終わっていなければ
/// この契機は見送る（2 本が同じ覚えを読んで二重に鳴らさない）。
#[derive(Debug)]
pub struct OutagePoller {
    inner: Arc<Inner>,
    running: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutagePollerInner").finish_non_exhaustive()
    }
}

impl OutagePoller {
    pub fn new(
        query: Arc<dyn OutageQuery>,
        toaster: Arc<dyn Toaster>,
        store: NoticeStore,
        log: fn(String),
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                query,
                toaster,
                store,
                log,
            }),
            running: None,
        }
    }

    /// 契機ごとに 1 回。待たない。
    pub fn poll(&mut self, device_id: &str) {
        if self.running.as_ref().is_some_and(|h| !h.is_finished()) {
            return;
        }
        let inner = Arc::clone(&self.inner);
        let device_id = device_id.to_string();
        self.running = std::thread::Builder::new()
            .name("outage-poller".into())
            .spawn(move || inner.poll_once(&device_id))
            .ok();
    }

    /// 試験用: 走っている問い合わせが終わるまで待つ。
    #[cfg(test)]
    pub(crate) fn wait(&mut self) {
        if let Some(h) = self.running.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::sync::Mutex;

    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn outage(id: i64, source: &str, name: &str, alive: &str) -> Outage {
        Outage {
            episode_id: id,
            logical_source: source.into(),
            display_name: name.into(),
            last_alive_at: at(alive),
        }
    }

    fn location() -> Outage {
        outage(1, "c01-location", "位置", "2026-10-11T23:15:00Z")
    }

    fn photo() -> Outage {
        outage(2, "c01-photo", "写真", "2026-10-11T13:00:00Z")
    }

    fn sounded(ids: &[i64]) -> NoticeState {
        NoticeState {
            sounded: ids.iter().copied().collect(),
            shown: true,
        }
    }

    fn names(action: &NoticeAction) -> Vec<&str> {
        match action {
            NoticeAction::Show { outages, .. } => {
                outages.iter().map(|o| o.display_name.as_str()).collect()
            }
            _ => vec![],
        }
    }

    fn sound_of(action: &NoticeAction) -> Option<bool> {
        match action {
            NoticeAction::Show { sound, .. } => Some(*sound),
            _ => None,
        }
    }

    // Scenario: 新しい途切れで音つきの通知が 1 通出る
    #[test]
    fn outage_notice_new_outage_shows_one_with_sound() {
        let d = decide(
            &NoticeState::default(),
            &OutageResult::Ready(vec![location()]),
        );
        assert_eq!(names(&d.action), vec!["位置"]);
        assert_eq!(sound_of(&d.action), Some(true));
    }

    // Scenario: 続く途切れでは音を鳴らさずに通知が残る
    #[test]
    fn outage_notice_continuing_outage_stays_silent() {
        let d = decide(&sounded(&[1]), &OutageResult::Ready(vec![location()]));
        assert_eq!(names(&d.action), vec!["位置"]);
        assert_eq!(sound_of(&d.action), Some(false));
    }

    // Scenario: 同時に途切れたソースは 1 通にまとまる
    #[test]
    fn outage_notice_simultaneous_outages_are_one_notice() {
        let d = decide(
            &NoticeState::default(),
            &OutageResult::Ready(vec![location(), photo()]),
        );
        assert_eq!(names(&d.action), vec!["位置", "写真"], "1 通に並ぶ");
        assert_eq!(sound_of(&d.action), Some(true), "音は 1 回");
        assert_eq!(d.next.sounded, BTreeSet::from([1, 2]));
    }

    // Scenario: 新しく途切れたソースが増えたときだけまた鳴る
    #[test]
    fn outage_notice_sounds_again_only_when_a_source_is_added() {
        let d = decide(
            &sounded(&[1]),
            &OutageResult::Ready(vec![location(), photo()]),
        );
        assert_eq!(names(&d.action), vec!["位置", "写真"]);
        assert_eq!(sound_of(&d.action), Some(true));
    }

    // Scenario: 途切れたソースが減ると音を鳴らさずに差し替わる
    #[test]
    fn outage_notice_fewer_sources_replace_silently() {
        let d = decide(&sounded(&[1, 2]), &OutageResult::Ready(vec![photo()]));
        assert_eq!(names(&d.action), vec!["写真"]);
        assert_eq!(sound_of(&d.action), Some(false));
        assert_eq!(d.next.sounded, BTreeSet::from([2]));
    }

    // Scenario: 戻ると通知が黙って消える
    #[test]
    fn outage_notice_recovery_dismisses_silently() {
        let d = decide(&sounded(&[1]), &OutageResult::Ready(vec![]));
        assert_eq!(d.action, NoticeAction::Dismiss);
        assert_eq!(d.next, NoticeState::default());
        // 出ていないなら何もしない
        let d = decide(&NoticeState::default(), &OutageResult::Ready(vec![]));
        assert_eq!(d.action, NoticeAction::Nothing);
    }

    // Scenario: サーバに届かない間は通知が変わらない
    #[test]
    fn outage_notice_unreachable_changes_nothing() {
        let s = sounded(&[1]);
        let d = decide(&s, &OutageResult::Unreachable);
        assert_eq!(d.action, NoticeAction::Nothing);
        assert_eq!(d.next, s);
    }

    // Scenario: 判定を待っている間は通知が変わらない
    #[test]
    fn outage_notice_warming_up_changes_nothing() {
        let s = sounded(&[1]);
        let d = decide(&s, &OutageResult::WarmingUp);
        assert_eq!(d.action, NoticeAction::Nothing);
        assert_eq!(d.next, s);
    }

    // Scenario: 同じ途切れでも端末ごとに 1 回鳴る
    #[test]
    fn outage_notice_each_device_sounds_once() {
        // 携帯の覚えと PC の覚えは別。同じ途切れ（識別子 1）でもそれぞれ初回は音つき
        let phone = NoticeState::default();
        let pc = NoticeState::default();
        let result = OutageResult::Ready(vec![location()]);
        assert_eq!(sound_of(&decide(&phone, &result).action), Some(true));
        assert_eq!(sound_of(&decide(&pc, &result).action), Some(true));
        // 2 回目は PC だけ鳴らない
        let pc = decide(&pc, &result).next;
        assert_eq!(sound_of(&decide(&pc, &result).action), Some(false));
    }

    // Scenario: 途切れの通知に記録の中身が載らない
    #[test]
    fn outage_notice_toast_carries_only_names_times_and_fixed_words() {
        let xml = toast_xml(&[location(), photo()], true);
        assert_eq!(
            xml,
            "<toast><visual><binding template=\"ToastGeneric\"><text>収集が途切れています</text>\
             <text>位置（最後 10/12 08:15）、写真（最後 10/11 22:00）</text></binding></visual></toast>"
        );
        assert!(!xml.contains("launch"), "押しても画面を開かない");
        // 音なしは audio silent
        let quiet = toast_xml(&[location()], false);
        assert!(quiet.contains(r#"<audio silent="true"/>"#));
        // 表示名の記号は XML として壊れない
        let odd = toast_xml(&[outage(3, "x", "a<b&c", "2026-10-11T00:00:00Z")], true);
        assert!(odd.contains("a&lt;b&amp;c"));
    }

    // Scenario: 収集アプリを起動し直しても同じ途切れでは鳴らない
    #[test]
    fn outage_notice_restart_keeps_the_sounded_set() {
        let dir = std::env::temp_dir().join(format!("ashiato-on-{}", uuid::Uuid::new_v4()));
        let result = OutageResult::Ready(vec![location()]);
        let first = decide(&NoticeStore::new(&dir).load(), &result);
        assert_eq!(sound_of(&first.action), Some(true));
        NoticeStore::new(&dir).save(&first.next).unwrap();
        // 起動し直し = 新しい NoticeStore が書いたものを読み直す
        let again = decide(&NoticeStore::new(&dir).load(), &result);
        assert_eq!(sound_of(&again.action), Some(false));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn outage_notice_unreadable_state_file_means_nothing_sounded() {
        let dir = std::env::temp_dir().join(format!("ashiato-on-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("outage-notice.json"), "{壊れている").unwrap();
        assert_eq!(NoticeStore::new(&dir).load(), NoticeState::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn outage_notice_reply_parsing() {
        let reply = |status, body: &str| Reply {
            status,
            body: body.into(),
        };
        assert_eq!(
            parse_reply(&reply(
                200,
                r#"{"status":"warming_up","judged_at":"2026-10-12T00:00:00Z"}"#
            )),
            OutageResult::WarmingUp
        );
        let ready = r#"{"status":"ready","judged_at":"2026-10-12T00:00:00Z","outages":[{"episode_id":1,"logical_source":"c01-location","display_name":"位置","last_alive_at":"2026-10-11T23:15:00Z"}]}"#;
        assert_eq!(
            parse_reply(&reply(200, ready)),
            OutageResult::Ready(vec![location()])
        );
        assert_eq!(parse_reply(&reply(401, ready)), OutageResult::Unreachable);
        assert_eq!(parse_reply(&reply(200, "{")), OutageResult::Unreachable);
        assert_eq!(
            parse_reply(&reply(200, r#"{"status":"???"}"#)),
            OutageResult::Unreachable
        );
    }

    /// 偽の問い合わせ。呼ばれた端末識別子を覚える。
    #[derive(Debug)]
    struct FakeQuery {
        result: Mutex<OutageResult>,
        asked: Mutex<Vec<String>>,
    }

    impl FakeQuery {
        fn new(result: OutageResult) -> Arc<Self> {
            Arc::new(Self {
                result: Mutex::new(result),
                asked: Mutex::new(Vec::new()),
            })
        }
    }

    impl OutageQuery for FakeQuery {
        fn fetch(&self, device_id: &str) -> OutageResult {
            self.asked.lock().unwrap().push(device_id.into());
            self.result.lock().unwrap().clone()
        }
    }

    /// 偽の通知の口。`ok = false` は出せない（権限が無い・登録簿が書けない）。
    #[derive(Debug)]
    struct FakeToaster {
        ok: bool,
        calls: Mutex<Vec<String>>,
    }

    impl FakeToaster {
        fn new(ok: bool) -> Arc<Self> {
            Arc::new(Self {
                ok,
                calls: Mutex::new(Vec::new()),
            })
        }
        fn result(&self) -> anyhow::Result<()> {
            if self.ok {
                Ok(())
            } else {
                anyhow::bail!("unreachable: 通知を出せない")
            }
        }
    }

    impl Toaster for FakeToaster {
        fn show(&self, xml: &str, sound: bool) -> anyhow::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("show sound={sound} {xml}"));
            self.result()
        }
        fn dismiss(&self) -> anyhow::Result<()> {
            self.calls.lock().unwrap().push("dismiss".into());
            self.result()
        }
    }

    fn quiet(_: String) {}

    // Scenario: 通知を出せなくても収集は止まらない
    #[test]
    fn outage_notice_failing_toaster_neither_panics_nor_stops_polling() {
        let dir = std::env::temp_dir().join(format!("ashiato-on-{}", uuid::Uuid::new_v4()));
        let query = FakeQuery::new(OutageResult::Ready(vec![location()]));
        let toaster = FakeToaster::new(false);
        let mut p = OutagePoller::new(
            query.clone(),
            toaster.clone(),
            NoticeStore::new(&dir),
            quiet,
        );
        p.poll("dev-1");
        p.wait();
        p.poll("dev-1");
        p.wait();
        // 契機ごとに問い合わせは続き、出せなかったので覚えは進まない（出せるようになったら鳴る）
        assert_eq!(query.asked.lock().unwrap().len(), 2);
        assert_eq!(toaster.calls.lock().unwrap().len(), 2);
        assert_eq!(NoticeStore::new(&dir).load(), NoticeState::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn outage_notice_poller_shows_then_remembers_then_dismisses() {
        let dir = std::env::temp_dir().join(format!("ashiato-on-{}", uuid::Uuid::new_v4()));
        let query = FakeQuery::new(OutageResult::Ready(vec![location()]));
        let toaster = FakeToaster::new(true);
        let mut p = OutagePoller::new(
            query.clone(),
            toaster.clone(),
            NoticeStore::new(&dir),
            quiet,
        );
        p.poll("dev-1");
        p.wait();
        p.poll("dev-1");
        p.wait();
        *query.result.lock().unwrap() = OutageResult::Ready(vec![]);
        p.poll("dev-1");
        p.wait();
        let calls = toaster.calls.lock().unwrap();
        assert!(calls[0].starts_with("show sound=true "), "{calls:?}");
        assert!(calls[1].starts_with("show sound=false "), "{calls:?}");
        assert_eq!(calls[2], "dismiss");
        assert_eq!(NoticeStore::new(&dir).load(), NoticeState::default());
        std::fs::remove_dir_all(&dir).ok();
    }
}
