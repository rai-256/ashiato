// SPDX-License-Identifier: AGPL-3.0-only
//! 部品を噛み合わせる。**時計と OS を外から渡す**ので、実機を待たずに確かめられる。
//!
//! 1 回の見回り（`tick`）でやること:
//!
//! 1. 見回りの時刻が飛んでいたら「眠っていた」として残す（design D19）
//! 2. 前景と入力の状態を見て、記録になったものを未送信へ積む
//! 3. 「ここまで動いていた」の印を更新する（FR-82 の材料。design D10）
//! 4. 契機が来ていれば 時計のずれ / 生存信号 / 送信 を行う
//!
//! # 2 つの時計（design D19 / review/code.md R17）
//!
//! **記録の時刻は壁時計、契機（印・送信・生存信号・ずれ）は単調時計**で測る。
//! 契機を壁時計の差で測ると、時計が戻った幅だけ印も送信も止まり、
//! その間に電源が落ちた停止期間が 1 件も残らなかった。
//!
//! # 止まらない（design D22 / R16）
//!
//! **1 回の書き込みの失敗で収集を終わらせない。** 終わると次のログオンまで戻らず、
//! しかも次の起動の `powered-off` が「PC が止まっていた」を主張してしまう。
//! 失敗はログに出して次の見回りでやり直し、記録は手元に抱えて積み直す。
use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Utc};

use crate::clock::{self, ReferenceClock, SkewSchedule};
use crate::config::{Config, Zone};
use crate::contract::{HeartbeatRequest, IngestRequest, WindowPayload};
use crate::engine::{Engine, EngineState, IdleRead, Observation, UrlRead};
use crate::heartbeat::{self, blocker, Capability, CounterStore, Counters};
use crate::marker::{self, Marker, TOUCH_INTERVAL_SEC};
use crate::outbox::Outbox;
use crate::sender::{Sender, Transport};
use crate::telemetry;

/// 見回りの間隔（design D5・**仮**）。題名だけの変化は OS からの通知が
/// 来ないことがあるので、**最長 1 秒遅れ**で拾う。
pub const POLL_INTERVAL_SEC: i64 = 1;

/// 送信の契機（`docs/collector-contract.md` §送る間隔）。
pub const SEND_INTERVAL_SEC: i64 = 300;

/// これだけ見回りが飛んだら「眠っていた」と読む（design D19）。
pub const SUSPEND_GAP_SEC: i64 = 120;

/// 壁時計と単調時計の進みがこれだけ食い違ったら、**その場でずれを測る**（R17）。
pub const CLOCK_JUMP_SEC: i64 = 60;

/// OS を触る側。**実機では `platform::WindowsSource`、試験では偽物。**
pub trait Source: std::fmt::Debug {
    /// いまの前景と入力の状態を読む。
    fn observe(&mut self, at: DateTime<Utc>) -> Observation;

    /// 観測によらず**ずっと満たされていないもの**（UI Automation を開けなかった、など）。
    fn persistent_blockers(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// OS が最後に起動した時刻（design D23）。取れなければ `None`。
    fn boot_time(&self) -> Option<DateTime<Utc>> {
        None
    }
}

/// 収集の本体。
pub struct Runtime<'a> {
    user_id: uuid::Uuid,
    device_id: String,
    zone: Zone,
    engine: Engine,
    events: Outbox<IngestRequest>,
    beats: Outbox<HeartbeatRequest>,
    ingest: Sender,
    beat_sender: Sender,
    transport: &'a dyn Transport,
    reference: &'a dyn ReferenceClock,
    marker: Marker,
    engine_path: std::path::PathBuf,
    saved_state: Option<EngineState>,
    counters: Counters,
    counter_store: CounterStore,
    beat_schedule: heartbeat::Schedule,
    skew_schedule: SkewSchedule,
    /// 前回の見回りの（壁時計, 単調時計）
    last_tick: Option<(DateTime<Utc>, DateTime<Utc>)>,
    last_touch: Option<DateTime<Utc>>,
    last_send: Option<DateTime<Utc>>,
    /// 区間の間に一度でも満たされなかったもの（R26）
    interval_blockers: BTreeSet<String>,
    /// 置き場へ積めなかった記録。**捨てずに次の見回りで積み直す**（R16）
    unsaved: Vec<WindowPayload>,
    /// 単調時計の起点（`tick` が使う）
    anchor: Option<(std::time::Instant, DateTime<Utc>)>,
    log: fn(String),
}

/// **合言葉も本文も出さない**（R11 / R35）。件数と状態の有無だけ。
impl std::fmt::Debug for Runtime<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("events", &self.events.len())
            .field("beats", &self.beats.len())
            .field("unsaved", &self.unsaved.len())
            .field("engine", &self.engine)
            .finish()
    }
}

fn info(line: String) {
    tracing::info!("{line}");
}

impl<'a> Runtime<'a> {
    /// 置き場を開き、数えを読み戻す。
    ///
    /// **未送信の置き場が読めないときだけは起動しない** —— 読めないまま進むと、
    /// 次の書き直しで溜まっていた全件を消す（R20）。
    pub fn new(
        cfg: &Config,
        zone: Zone,
        engine: Engine,
        transport: &'a dyn Transport,
        reference: &'a dyn ReferenceClock,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let counter_store = CounterStore::new(&cfg.state_dir);
        let (counters, broken_counters) = counter_store.load_or_quarantine();
        if broken_counters {
            info(telemetry::line("counters_quarantined", Some(1), None, None));
        }
        let events: Outbox<IngestRequest> = Outbox::open(cfg.state_dir.join("outbox.jsonl"))?;
        // **生存信号は別のファイル**（同じ JSONL に混ぜると読み戻しで
        // 片方が「壊れた行」に見えて退避に回る。ST02 の契約）
        let beats: Outbox<HeartbeatRequest> = Outbox::open(cfg.state_dir.join("heartbeat.jsonl"))?;
        for (name, n) in [
            ("events", events.broken_lines()),
            ("beats", beats.broken_lines()),
        ] {
            if n > 0 {
                info(telemetry::line(
                    "outbox_broken_lines",
                    Some(n),
                    None,
                    Some(name),
                ));
            }
        }
        Ok(Self {
            user_id: cfg.user_id,
            device_id: cfg.device_id.clone(),
            zone,
            engine,
            events,
            beats,
            ingest: Sender::ingest(),
            beat_sender: Sender::heartbeat(),
            transport,
            reference,
            marker: Marker::new(&cfg.state_dir),
            engine_path: cfg.state_dir.join("engine.json"),
            saved_state: None,
            counters: counters.unwrap_or_else(|| Counters::new(now)),
            counter_store,
            beat_schedule: heartbeat::Schedule::new(),
            skew_schedule: SkewSchedule::new(),
            last_tick: None,
            last_touch: None,
            last_send: None,
            interval_blockers: BTreeSet::new(),
            unsaved: Vec::new(),
            anchor: None,
            log: info,
        })
    }

    /// 起動時にやること（壁時計と単調時計は今の値）。
    pub fn start(&mut self, source: &dyn Source) {
        let (wall, mono) = self.clocks();
        self.start_at(source, wall, mono);
    }

    /// 起動時にやること。**前回の停止から今回の起動までを 1 件残す**（FR-82）。
    /// 前回のプロセスが開いたままにした離席と除外の数えも、ここで閉じる（R1 / R4）。
    ///
    /// 印を読むのは**この 1 回だけ** —— 読んでから上書きするので、
    /// 順序を逆にすると停止期間が永久に消える。
    pub fn start_at(&mut self, source: &dyn Source, wall: DateTime<Utc>, mono: DateTime<Utc>) {
        let (last_seen, broken) = self.marker.read_or_quarantine();
        if broken {
            (self.log)(telemetry::line("marker_quarantined", Some(1), None, None));
        }
        let clean = self.marker.take_clean_stop();
        let previous = self.load_engine_state();
        let mut records = self.engine.close_previous(previous, last_seen);
        if let Some(p) = marker::powered_off_span(last_seen, wall, source.boot_time(), clean) {
            records.push(p);
        } else if last_seen.is_some_and(|l| l >= wall) {
            // **時計が戻っていた**。区間は作れないので、ずれをすぐ測る（R17 / R22 の穴）
            (self.log)(telemetry::line("clock_went_back", Some(1), None, None));
            self.skew_schedule = SkewSchedule::new();
        }
        for p in records {
            self.push(p);
        }
        self.save_engine_state();
        self.touch(wall, mono);
    }

    /// 1 回の見回り（壁時計と単調時計は今の値）。
    pub fn tick(&mut self, source: &mut dyn Source) {
        let (wall, mono) = self.clocks();
        self.tick_at(source, wall, mono);
    }

    /// 1 回の見回り。**失敗しても戻り値で止まらない**（design D22）。
    pub fn tick_at(&mut self, source: &mut dyn Source, wall: DateTime<Utc>, mono: DateTime<Utc>) {
        // 1. 見回りが飛んでいたら「眠っていた」（design D19）
        if let Some((last_wall, last_mono)) = self.last_tick {
            let wall_gap = wall - last_wall;
            let mono_gap = mono - last_mono;
            if wall_gap >= Duration::seconds(SUSPEND_GAP_SEC) {
                for p in self.engine.report_suspend(last_wall, wall, Some(mono_gap)) {
                    self.push(p);
                }
            }
            if (wall_gap - mono_gap).num_seconds().abs() >= CLOCK_JUMP_SEC {
                // **壁時計が飛んだ**。破れた直後こそ測る（扉 #5 / R17）
                (self.log)(telemetry::line("clock_jump", Some(1), None, None));
                self.skew_schedule = SkewSchedule::new();
            }
        }
        self.last_tick = Some((wall, mono));

        // 2. 観測 → 記録
        self.retry_unsaved();
        let obs = source.observe(wall);
        let cap = capability_of(&obs);
        self.interval_blockers.extend(cap.blockers.iter().cloned());
        self.interval_blockers
            .extend(source.persistent_blockers().into_iter().map(String::from));
        if !obs.locked {
            // **ロック中は試行に数えない**（R6）。数えると夜のあいだ取得率が 0 になり、
            // `capability_of` が「取れないではない」と決めたことと逆を報告する
            self.counters.record(cap.capturable);
        }
        for p in self.engine.observe(obs) {
            self.push(p);
        }
        self.save_engine_state();
        if let Err(e) = self.counter_store.save(&self.counters) {
            (self.log)(telemetry::line(
                "counters_save_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
        }

        // 3. 「ここまで動いていた」の印（FR-82 の材料）
        if self
            .last_touch
            .is_none_or(|t| mono - t >= Duration::seconds(TOUCH_INTERVAL_SEC))
        {
            self.touch(wall, mono);
        }

        // 4. 契機（**単調時計で測る**）
        self.maybe_measure_skew(wall, mono);
        self.maybe_beat(wall, mono);
        self.maybe_send(wall, mono);
    }

    /// 終わるときに手元のものを吐き出す（除外の数えを抱えたまま消さない）。
    pub fn stop(&mut self) {
        let (wall, _) = self.clocks();
        self.stop_at(wall);
    }

    /// 終わるときにやること。**自分で止まった印を残す**（design D23）。
    pub fn stop_at(&mut self, wall: DateTime<Utc>) {
        for p in self.engine.flush(wall) {
            self.push(p);
        }
        self.save_engine_state();
        if let Err(e) = self.marker.touch(wall) {
            (self.log)(telemetry::line(
                "marker_touch_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
        }
        if let Err(e) = self.marker.mark_clean_stop() {
            (self.log)(telemetry::line(
                "clean_stop_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
        }
        self.send();
    }

    /// 溜まっている件数（記録・生存信号）。
    pub fn pending(&self) -> (usize, usize) {
        (self.events.len(), self.beats.len())
    }

    /// 単調時計の起点から、いまの（壁時計, 単調時計）を作る。
    fn clocks(&mut self) -> (DateTime<Utc>, DateTime<Utc>) {
        let wall = Utc::now();
        let (i0, w0) = *self.anchor.get_or_insert((std::time::Instant::now(), wall));
        let elapsed = Duration::from_std(i0.elapsed()).unwrap_or_else(|_| Duration::zero());
        (wall, w0 + elapsed)
    }

    /// 記録を未送信へ積む。**積めなければ手元に抱えて次の見回りで積み直す**（R16）。
    fn push(&mut self, p: WindowPayload) {
        match self.try_push(&p) {
            Ok(()) => {}
            Err(e) => {
                (self.log)(telemetry::line(
                    "outbox_add_failed",
                    Some(1),
                    None,
                    Some(telemetry::error_kind(&e)),
                ));
                self.unsaved.push(p);
            }
        }
    }

    fn try_push(&mut self, p: &WindowPayload) -> anyhow::Result<()> {
        let at = DateTime::parse_from_rfc3339(&p.at)?.with_timezone(&Utc);
        let req = IngestRequest::of(p, self.user_id, &self.device_id, at, &self.zone)?;
        self.events.add(req)
    }

    fn retry_unsaved(&mut self) {
        for p in std::mem::take(&mut self.unsaved) {
            self.push(p);
        }
    }

    fn touch(&mut self, wall: DateTime<Utc>, mono: DateTime<Utc>) {
        match self.marker.touch(wall) {
            Ok(()) => self.last_touch = Some(mono),
            Err(e) => (self.log)(telemetry::line(
                "marker_touch_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            )),
        }
    }

    fn load_engine_state(&self) -> EngineState {
        match std::fs::read_to_string(&self.engine_path) {
            Ok(t) => serde_json::from_str(&t).unwrap_or_else(|_| {
                (self.log)(telemetry::line(
                    "engine_state_unreadable",
                    Some(1),
                    None,
                    None,
                ));
                EngineState::default()
            }),
            Err(_) => EngineState::default(),
        }
    }

    /// 離席の区間と除外の数えを置き場に落とす（design D21）。**変わったときだけ書く。**
    fn save_engine_state(&mut self) {
        let state = self.engine.state();
        if self.saved_state == Some(state) {
            return;
        }
        let written = serde_json::to_vec(&state)
            .map_err(anyhow::Error::from)
            .and_then(|b| crate::fsutil::atomic_write(&self.engine_path, &b));
        match written {
            Ok(()) => self.saved_state = Some(state),
            Err(e) => (self.log)(telemetry::line(
                "engine_state_save_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            )),
        }
    }

    fn maybe_measure_skew(&mut self, wall: DateTime<Utc>, mono: DateTime<Utc>) {
        if !self.skew_schedule.due(mono) {
            return;
        }
        match self.reference.now() {
            Ok(reference) => {
                let p = clock::measure(wall, reference, &self.reference.source());
                self.push(p);
                self.skew_schedule.mark(mono);
            }
            // **推測で埋めない。** 1 分後に測り直す（毎秒は叩かない）
            Err(e) => {
                (self.log)(telemetry::line(
                    "clock_skew_unavailable",
                    None,
                    None,
                    Some(telemetry::error_kind(&e)),
                ));
                self.skew_schedule.failed(mono);
            }
        }
    }

    fn maybe_beat(&mut self, wall: DateTime<Utc>, mono: DateTime<Utc>) {
        if !self.beat_schedule.due(mono) {
            return;
        }
        let (attempts, successes) = self.counters.take(wall);
        if let Err(e) = self.counter_store.save(&self.counters) {
            (self.log)(telemetry::line(
                "counters_save_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
        }
        let cap = Capability::from_blockers(&self.interval_blockers);
        self.interval_blockers.clear();
        match heartbeat::signal(
            self.user_id,
            &self.device_id,
            wall,
            &cap,
            attempts,
            successes,
        )
        .and_then(|sig| self.beats.add(sig))
        {
            Ok(()) => self.beat_schedule.mark(mono),
            Err(e) => (self.log)(telemetry::line(
                "heartbeat_add_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            )),
        }
    }

    fn maybe_send(&mut self, wall: DateTime<Utc>, mono: DateTime<Utc>) {
        if self
            .last_send
            .is_none_or(|t| mono - t >= Duration::seconds(SEND_INTERVAL_SEC))
        {
            // 除外の数えは送信の契機ごとに吐き出す（design D18）
            for p in self.engine.flush(wall) {
                self.push(p);
            }
            self.save_engine_state();
            self.send();
            self.last_send = Some(mono);
        }
    }

    fn send(&mut self) {
        let log = self.log;
        let mut log = |line: String| log(line);
        if let Err(e) = self
            .ingest
            .flush(&mut self.events, self.transport, &mut log)
        {
            log(telemetry::line(
                "send_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
        }
        if let Err(e) = self
            .beat_sender
            .flush(&mut self.beats, self.transport, &mut log)
        {
            log(telemetry::line(
                "send_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
        }
    }
}

/// 取得できる状態かを観測から決める（design D4）。
///
/// **URL の経路は「前景がブラウザのとき」しか判らない** ——
/// ブラウザでない前景を見ている間に「応答しない」と報告すると、
/// 取得率が生活の都合（どのアプリを使ったか）で上下する。
fn capability_of(obs: &Observation) -> Capability {
    let idle_ok = obs.idle != IdleRead::Unavailable;
    // **ロック中は前景が無くても「取れない」ではない。** 前景が無いのは画面が閉じているからで、
    // 収集は壊れていない —— ここを取得不能に数えると、成功条件 1（NFR-13）の
    // 分子が「席にいた日」に化ける
    let foreground = obs.locked || obs.foreground.is_some();
    let url_ok = !matches!(
        obs.foreground.as_ref().map(|f| &f.url),
        Some(UrlRead::Unavailable)
    );
    let mut cap = Capability::of(foreground, url_ok);
    if !idle_ok {
        // 経過時間が読めないと離席・ロック・スリープが残らない（R24）
        cap.blockers.push(blocker::IDLE.to_string());
        cap.capturable = false;
    }
    cap
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::engine::Foreground;
    use crate::exclusion::{Exclusions, Rule};
    use crate::sender::Reply;
    use std::cell::RefCell;

    fn t(sec: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-13T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::seconds(sec)
    }

    fn cfg() -> Config {
        Config {
            base_url: "http://127.0.0.1:1".into(),
            api_token: "t".into(),
            user_id: uuid::Uuid::nil(),
            device_id: "dev-1".into(),
            state_dir: std::env::temp_dir().join(format!("ashiato-rt-{}", uuid::Uuid::new_v4())),
        }
    }

    fn zone() -> Zone {
        Zone {
            id: "Asia/Tokyo".into(),
            offset_min: 540,
        }
    }

    /// 前景を返す偽の OS。**状態を外から切り替えられる。**
    #[derive(Debug)]
    struct FakeSource {
        app: String,
        title: String,
        url: UrlRead,
        locked: bool,
        idle_sec: Option<i64>,
        boot: Option<DateTime<Utc>>,
    }

    impl FakeSource {
        fn new(app: &str) -> Self {
            Self {
                app: app.into(),
                title: "題名".into(),
                url: UrlRead::NotBrowser,
                locked: false,
                idle_sec: Some(0),
                boot: None,
            }
        }
    }

    impl Source for FakeSource {
        fn observe(&mut self, at: DateTime<Utc>) -> Observation {
            Observation {
                at,
                foreground: (!self.locked).then(|| Foreground {
                    app_name: self.app.clone(),
                    exe_path: format!(r"C:\apps\{}.exe", self.app),
                    process_name: format!("{}.exe", self.app),
                    title: self.title.clone(),
                    url: self.url.clone(),
                }),
                idle: self.idle_sec.map_or(IdleRead::Unavailable, |s| {
                    IdleRead::Elapsed(Duration::seconds(s))
                }),
                locked: self.locked,
            }
        }

        fn boot_time(&self) -> Option<DateTime<Utc>> {
            self.boot
        }
    }

    /// 常に受理する偽の取り込み口。
    #[derive(Debug, Default)]
    struct AcceptAll {
        bodies: RefCell<Vec<(String, String)>>,
    }

    impl Transport for AcceptAll {
        fn post(&self, path: &str, body: &str) -> anyhow::Result<Reply> {
            self.bodies
                .borrow_mut()
                .push((path.to_string(), body.to_string()));
            let n = serde_json::from_str::<Vec<serde_json::Value>>(body)
                .map(|v| v.len())
                .unwrap_or(0);
            let items: Vec<serde_json::Value> = (0..n)
                .map(|_| {
                    serde_json::json!({"id": null, "duplicate": false,
                                            "accepted": true, "error": null})
                })
                .collect();
            Ok(Reply {
                status: 200,
                body: serde_json::to_string(&items).unwrap(),
            })
        }
    }

    /// 偽の取り込み口が受け取った本文の中身（`path` の分だけ）。
    fn sent(t: &AcceptAll, path: &str) -> Vec<serde_json::Value> {
        t.bodies
            .borrow()
            .iter()
            .filter(|(p, _)| p == path)
            .flat_map(|(_, b)| serde_json::from_str::<Vec<serde_json::Value>>(b).unwrap())
            .collect()
    }

    fn of_kind(t: &AcceptAll, kind: &str) -> Vec<serde_json::Value> {
        sent(t, "/ingest")
            .into_iter()
            .filter(|r| r["payload"]["kind"] == kind)
            .collect()
    }

    #[derive(Debug)]
    struct FixedReference(DateTime<Utc>);

    impl ReferenceClock for FixedReference {
        fn now(&self) -> anyhow::Result<DateTime<Utc>> {
            Ok(self.0)
        }
        fn source(&self) -> String {
            "127.0.0.1:1".into()
        }
    }

    fn runtime<'a>(
        cfg: &Config,
        engine: Engine,
        transport: &'a AcceptAll,
        reference: &'a FixedReference,
    ) -> Runtime<'a> {
        Runtime::new(cfg, zone(), engine, transport, reference, t(0)).unwrap()
    }

    /// 壁時計と単調時計を揃えて `sec` 秒ぶん回す。
    fn run(rt: &mut Runtime, src: &mut FakeSource, from: i64, to: i64, step: usize) {
        for sec in (from..=to).step_by(step) {
            rt.tick_at(src, t(sec), t(sec));
        }
    }

    /// **起動時に「PC が止まっていた」が 1 件積まれ、OS の起動時刻が載る**（FR-82 / D23）。
    #[test]
    fn start_records_powered_off_span() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        Marker::new(&cfg.state_dir).touch(t(-50_000)).unwrap();

        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut src = FakeSource::new("editor");
        src.boot = Some(t(-60));
        rt.start_at(&src, t(0), t(0));
        assert_eq!(rt.pending().0, 1, "止まっていた期間が積まれていない");
        let raw = rt.events.snapshot()[0].raw.clone();
        assert!(
            raw.contains("powered-off") && raw.contains("range_end"),
            "{raw}"
        );
        assert!(raw.contains(&format!(
            "\"boot_at\":\"{}\"",
            crate::contract::rfc3339(t(-60))
        )));
        assert!(
            !raw.contains("clean_stop"),
            "自分で止まっていないのに印が付いた"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **自分で止まった回の次は `clean_stop` が付く**（R16 / D23）。
    #[test]
    fn clean_stop_is_carried_to_next_start() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        {
            let mut rt = runtime(
                &cfg,
                Engine::new(Exclusions::default()),
                &transport,
                &reference,
            );
            let mut src = FakeSource::new("editor");
            rt.start_at(&src, t(0), t(0));
            run(&mut rt, &mut src, 0, 10, 1);
            rt.stop_at(t(10));
        }
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        rt.start_at(&FakeSource::new("editor"), t(3600), t(3600));
        let off = rt
            .events
            .snapshot()
            .iter()
            .find(|r| r.payload["kind"] == "powered-off")
            .expect("停止期間")
            .clone();
        assert_eq!(off.payload["clean_stop"], true);
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **見回りの時刻が飛んだら「眠っていた」として残し、単調時計の進みも載せる**（D19）。
    #[test]
    fn suspend_gap_is_recorded() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        rt.tick_at(&mut src, t(0), t(0));
        // 1 時間ぶん見回りが飛ぶ（単調時計も 1 時間進んだ = 本当に止まっていた）
        rt.tick_at(&mut src, t(3_600), t(3_600));
        let suspended: Vec<_> = sent(&transport, "/ingest")
            .into_iter()
            .filter(|r| r["payload"]["reason"] == "suspended")
            .collect();
        assert_eq!(
            suspended.len(),
            2,
            "眠っていた区間が出入りの 2 件になっていない"
        );
        assert_eq!(suspended[0]["payload"]["transition"], "enter");
        assert_eq!(
            suspended[1]["payload"]["range_end"],
            serde_json::json!(crate::contract::rfc3339(t(3_600)))
        );
        assert_eq!(suspended[1]["payload"]["mono_gap_ms"], 3_600_000);
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **1 日回すと、生存信号 4 件・時計のずれ 24 件**（R19。Runtime の層で契機を守る）。
    ///
    /// Scenario: 想定間隔ごとに生存信号が届く
    /// Scenario: 1 時間ごとにずれの測定記録が残る
    #[test]
    fn ticks_keep_heartbeat_and_skew_intervals_for_a_day() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 86_399, 10);
        assert_eq!(
            sent(&transport, "/heartbeat").len(),
            4,
            "生存信号の間隔が想定間隔と違う"
        );
        assert_eq!(
            of_kind(&transport, "clock-skew").len(),
            24,
            "ずれの測定が 1 時間ごとでない"
        );
        assert_eq!(of_kind(&transport, "foreground").len(), 1);
        assert_eq!(rt.pending(), (0, 0), "受理された分が取り除かれていない");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **動作中は 1 分ごとに印が進む**（R8 / tasks 5.1）。
    #[test]
    fn marker_advances_every_minute_while_running() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        let m = Marker::new(&cfg.state_dir);
        run(&mut rt, &mut src, 1, 59, 1);
        assert_eq!(m.read().unwrap(), Some(t(0)), "1 分より早く書いている");
        run(&mut rt, &mut src, 60, 180, 1);
        assert_eq!(
            m.read().unwrap(),
            Some(t(180)),
            "1 分ごとに印が進んでいない"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **時計が戻っても印と送信は止まらない。時計だけが進んだことも後から分かる**（R17）。
    #[test]
    fn clock_jumps_do_not_stop_the_intervals() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 120, 1);
        let skews_before = of_kind(&transport, "clock-skew").len();

        // 壁時計が 2 時間戻る。単調時計はそのまま進む
        for s in 121..=300 {
            rt.tick_at(&mut src, t(s - 7200), t(s));
        }
        let m = Marker::new(&cfg.state_dir);
        assert_eq!(
            m.read().unwrap(),
            Some(t(300 - 7200)),
            "時計が戻った間に印が止まった"
        );
        assert!(
            of_kind(&transport, "clock-skew").len() > skews_before,
            "時計が戻った直後にずれを測っていない"
        );

        // 壁時計だけが 3 分進む（単調時計は 1 秒）→ 区間は残るが、単調時計の進みで見分けられる
        rt.tick_at(&mut src, t(301 - 7200 + 180), t(301));
        run(&mut rt, &mut src, 302, 700, 1);
        let jumped: Vec<_> = sent(&transport, "/ingest")
            .into_iter()
            .filter(|r| r["payload"]["reason"] == "suspended")
            .collect();
        assert_eq!(
            jumped[1]["payload"]["mono_gap_ms"], 1_000,
            "時計の飛びと眠りが見分けられない"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **6 時間ロックしたままでも取得率は壊れない**（R6）。
    #[test]
    fn locked_hours_are_not_failures() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        rt.tick_at(&mut src, t(0), t(0));
        src.locked = true;
        run(&mut rt, &mut src, 10, EXPECTED_GAP, 10);
        let beats = sent(&transport, "/heartbeat");
        let last = beats.last().unwrap();
        assert_eq!(last["capturable"], true);
        assert_eq!(
            last["attempts"], last["successes"],
            "ロック中を失敗として数えた: {last}"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    const EXPECTED_GAP: i64 = crate::EXPECTED_GAP_SEC;

    /// **区間の間に一度でも欠けたものが生存信号に残る**（R26）。
    #[test]
    fn blockers_are_sticky_within_the_interval() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut src = FakeSource::new("chrome");
        rt.start_at(&src, t(0), t(0));
        rt.tick_at(&mut src, t(0), t(0)); // 起動直後の信号
        src.url = UrlRead::Unavailable;
        run(&mut rt, &mut src, 10, 3600, 10); // 1 時間 URL が読めない
        src.url = UrlRead::Read("example.com".into());
        src.idle_sec = None; // 経過時間も読めない時期がある
        run(&mut rt, &mut src, 3610, 3700, 10);
        src.idle_sec = Some(0);
        run(&mut rt, &mut src, 3710, EXPECTED_GAP, 10); // 信号の直前は全部読める
        let beats = sent(&transport, "/heartbeat");
        let last = beats.last().unwrap();
        assert_eq!(
            last["capturable"], false,
            "区間の途中の欠けが消えた: {last}"
        );
        assert_eq!(
            last["blockers"],
            serde_json::json!(["idle", "uiautomation"])
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **書き込みが失敗しても見回りを続け、記録は抱えて積み直す**（R16 / D22）。
    #[test]
    fn write_failures_do_not_stop_collection() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        // 印と数えの置き場を「書けない」形にする（同名のディレクトリを置く）
        for name in ["last-seen.txt", "counters.json", "engine.json"] {
            let p = cfg.state_dir.join(name);
            let _ = std::fs::remove_file(&p);
            std::fs::create_dir_all(p.join("blocker")).unwrap();
        }
        run(&mut rt, &mut src, 1, 90, 1);
        src.app = "mail".into();
        run(&mut rt, &mut src, 91, 400, 1);
        let fg = of_kind(&transport, "foreground");
        assert!(
            fg.iter().any(|r| r["payload"]["app_name"] == "mail"),
            "書き込みの失敗の後に収集が止まった"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **落ちる前に開いていた離席と除外の数えが、次の起動で閉じられる**（R1 / R4）。
    #[test]
    fn restart_closes_previous_away_and_excluded() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let rules = || Exclusions {
            rules: vec![Rule::ProcessName {
                value: "vault.exe".into(),
            }],
        };
        {
            let mut rt = runtime(&cfg, Engine::new(rules()), &transport, &reference);
            let mut src = FakeSource::new("vault");
            rt.start_at(&src, t(0), t(0));
            run(&mut rt, &mut src, 0, 30, 1);
            for i in 0..10 {
                src.title = format!("項目 {i}");
                rt.tick_at(&mut src, t(31 + i), t(31 + i));
            }
            src.idle_sec = Some(400);
            rt.tick_at(&mut src, t(100), t(100));
            // stop を呼ばずに落ちる
        }
        let mut rt = runtime(&cfg, Engine::new(rules()), &transport, &reference);
        rt.start_at(&FakeSource::new("editor"), t(900), t(900));
        rt.tick_at(&mut FakeSource::new("editor"), t(900), t(900));
        let excluded: u64 = of_kind(&transport, "excluded")
            .iter()
            .map(|r| r["payload"]["excluded_count"].as_u64().unwrap())
            .sum();
        assert_eq!(
            excluded, 11,
            "落ちる前の除外の数えが消えた（入った 1 + 題名 10）"
        );
        let closed: Vec<_> = of_kind(&transport, "idle")
            .into_iter()
            .filter(|r| r["payload"]["ended_by"] == "restart")
            .collect();
        assert_eq!(closed.len(), 1, "開いていた離席が閉じられていない");
        // 閉じる時刻は「ここまで動いていた」の印（1 分ごと。最後に書いたのは t=100）
        assert_eq!(
            closed[0]["payload"]["range_end"],
            serde_json::json!(crate::contract::rfc3339(t(100)))
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **除外した本文は、Runtime を通って取り込み口へ送られる本文にも現れない**（FR-83）。
    #[test]
    fn excluded_body_never_reaches_the_transport() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let rules = Exclusions {
            rules: vec![Rule::ProcessName {
                value: "vault.exe".into(),
            }],
        };
        let mut rt = runtime(&cfg, Engine::new(rules), &transport, &reference);
        let mut src = FakeSource::new("vault");
        src.title = "銀行 / 本人の口座".into();
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 400, 1);
        src.app = "editor".into();
        src.title = "文書".into();
        run(&mut rt, &mut src, 401, 700, 1);
        let all: String = transport
            .bodies
            .borrow()
            .iter()
            .map(|(_, b)| b.clone())
            .collect();
        assert!(
            !all.contains("銀行") && !all.contains("vault"),
            "除外した本文が送られた"
        );
        let state = std::fs::read_to_string(cfg.state_dir.join("engine.json")).unwrap();
        assert!(!state.contains("銀行"), "置き場に本文が落ちた");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// URL の経路・経過時間が読めないときだけ、生存信号がその名前を挙げる。
    #[test]
    fn capability_follows_the_url_path() {
        let fg = |url: UrlRead| Observation {
            at: t(0),
            foreground: Some(Foreground {
                app_name: "b".into(),
                exe_path: r"C:\b.exe".into(),
                process_name: "b.exe".into(),
                title: "題名".into(),
                url,
            }),
            idle: IdleRead::Elapsed(Duration::zero()),
            locked: false,
        };
        assert!(capability_of(&fg(UrlRead::NotBrowser)).capturable);
        assert!(capability_of(&fg(UrlRead::Read("x".into()))).capturable);
        let bad = capability_of(&fg(UrlRead::Unavailable));
        assert_eq!(bad.blockers, [blocker::UIAUTOMATION]);
        let none = capability_of(&Observation {
            at: t(0),
            foreground: None,
            idle: IdleRead::Elapsed(Duration::zero()),
            locked: false,
        });
        assert_eq!(none.blockers, [blocker::FOREGROUND]);
        let blind_idle = capability_of(&Observation {
            idle: IdleRead::Unavailable,
            ..fg(UrlRead::NotBrowser)
        });
        assert_eq!(blind_idle.blockers, [blocker::IDLE]);
        let locked = capability_of(&Observation {
            at: t(0),
            foreground: None,
            idle: IdleRead::Elapsed(Duration::zero()),
            locked: true,
        });
        assert!(locked.capturable, "ロック中を取れないと報告した");
    }
}
