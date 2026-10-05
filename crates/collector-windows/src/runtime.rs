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
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::clock::{ReferenceClock, SkewSchedule, Uptime};
use crate::clock_record::{skew_record, RecordContext};
use crate::clock_worker::{ClockReading, ClockWorker, WorkerPoll};
use crate::config::{Config, Zone};
use crate::contract::{ClockTrigger, HeartbeatRequest, IngestRequest, WindowPayload};
use crate::engine::{Engine, EngineState, IdleRead, Observation, UrlRead};
use crate::heartbeat::{self, blocker, Capability, CounterStore, Counters};
use crate::history::collect::{HistoryCollector, HistoryReader, ProfileHealth};
use crate::history::contract::Visit;
use crate::marker::{self, Marker, TOUCH_INTERVAL_SEC};
use crate::outbox::Outbox;
use crate::sender::{Sender, Transport};
use crate::telemetry;
use crate::time_sync::TimeSyncSource;

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

/// 時計のずれを測るために読む 3 つの口。**試験では偽物に差し替える。**
#[derive(Debug)]
pub struct ClockInputs {
    /// 取り込み口の応答の日付
    pub reference: Arc<dyn ReferenceClock>,
    /// Windows の時刻同期の状態
    pub time_sync: Arc<dyn TimeSyncSource>,
    /// 起動からの経過時間
    pub uptime: Arc<dyn Uptime>,
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
    reference: Arc<dyn ReferenceClock>,
    time_sync: Arc<dyn TimeSyncSource>,
    /// 起動からの経過時間（記録に載せる。design D9）
    uptime: Arc<dyn Uptime>,
    /// いま読んでいる測定の契機（記録に載せる）
    skew_trigger: ClockTrigger,
    /// 基準を読んでいる作業スレッド。**走っている間は次の測定を始めない**（design D7）
    clock_worker: Option<ClockWorker>,
    /// 試験用: 起こした作業スレッドをその場で待つ（結果は次の見回りで拾う。既定は待つ）
    #[cfg(test)]
    settle_clock_worker: bool,
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
    /// ブラウザ履歴の取得（ST08）。**読みは別スレッド**で、見回りは待たない（design D3）
    history: Option<HistoryCollector>,
    /// ブラウザ履歴の生存信号（design D12）。**ウィンドウのとは別の契機・別の数え・別の和**
    history_beat: HistoryBeat,
    state_dir: std::path::PathBuf,
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

/// 履歴のソースの生存信号の状態（24 時間ごと。起動直後に 1 回）。
struct HistoryBeat {
    schedule: heartbeat::Schedule,
    counters: Counters,
    store: CounterStore,
    /// 区間の間に一度でも欠けたもの（ウィンドウの R26 と同じ和）
    blockers: BTreeSet<String>,
    /// 区間に読み（か確かめ）が 1 回でもあったか。無ければ信号の前に開けるかを確かめる
    read_seen: bool,
}

impl HistoryBeat {
    fn new(state_dir: &std::path::Path, now: DateTime<Utc>) -> Self {
        let store = CounterStore::named(state_dir, "counters-browser-history.json");
        let (counters, _) = store.load_or_quarantine();
        Self {
            schedule: heartbeat::Schedule::with_interval(Duration::seconds(
                heartbeat::HISTORY_EXPECTED_GAP_SEC,
            )),
            counters: counters.unwrap_or_else(|| Counters::new(now)),
            store,
            blockers: BTreeSet::new(),
            read_seen: false,
        }
    }

    /// プロファイル 1 つの読み（か確かめ）1 回を 1 試行と数える。空は「1 つも見つからない」。
    fn record(&mut self, profiles: &[ProfileHealth]) {
        self.read_seen = true;
        if profiles.is_empty() {
            self.blockers
                .insert(heartbeat::history_blocker::NONE_FOUND.to_string());
        }
        for p in profiles {
            self.counters.record(p.readable);
            if !p.readable {
                self.blockers.insert(heartbeat::history_blocker::unreadable(
                    p.browser.name(),
                    &p.directory,
                ));
            }
        }
    }
}

fn info(line: String) {
    // log-ok: 呼び出し元は telemetry::line の行だけを渡す（件数・所要時間・種別しか受け取らない組み立て）
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
        clocks: ClockInputs,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let ClockInputs {
            reference,
            time_sync,
            uptime,
        } = clocks;
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
            time_sync,
            uptime,
            skew_trigger: ClockTrigger::Start,
            clock_worker: None,
            #[cfg(test)]
            settle_clock_worker: true,
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
            history: None,
            history_beat: HistoryBeat::new(&cfg.state_dir, now),
            state_dir: cfg.state_dir.clone(),
            anchor: None,
            log: info,
        })
    }

    /// ブラウザ履歴の取得を有効にする。**読み手を差し込めるようにしてある**（試験は偽の読み手を渡す）。
    #[must_use]
    pub fn with_history(mut self, reader: std::sync::Arc<dyn HistoryReader>) -> Self {
        self.history = Some(HistoryCollector::new(reader, &self.state_dir));
        self
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
            self.skew_schedule.reset(ClockTrigger::Jump);
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
                self.skew_schedule.reset(ClockTrigger::Jump);
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
        self.maybe_measure_skew(wall, mono, &*source);
        self.maybe_beat(wall, mono);
        self.maybe_history(wall);
        self.maybe_history_beat(wall, mono);
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

    /// 基準の読み取りを進める。**読み取りは作業スレッドで行い、ここでは待たない**（design D7）。
    ///
    /// 結果が来ていれば測定記録を積み、走っていなくて契機が来ていれば作業スレッドを起こす。
    /// 測り直し（60 秒）は読み取りが終わってから数える。
    fn maybe_measure_skew(
        &mut self,
        wall: DateTime<Utc>,
        mono: DateTime<Utc>,
        source: &dyn Source,
    ) {
        if let Some(worker) = self.clock_worker.as_mut() {
            let reading = match worker.poll() {
                WorkerPoll::Pending => return,
                WorkerPoll::Done(reading) => *reading,
                // **作業スレッドが落ちても見回りは止まらない**（D11）。2 つとも取れなかったとして扱う
                WorkerPoll::Failed => ClockReading::worker_failed(self.reference.source()),
            };
            self.clock_worker = None;
            self.finish_skew(reading, wall, mono, source);
        }
        if self.clock_worker.is_none() && self.skew_schedule.due(mono) {
            self.skew_trigger = self.skew_schedule.begin(mono);
            self.clock_worker = Some(ClockWorker::spawn(
                Arc::clone(&self.reference),
                Arc::clone(&self.time_sync),
                Arc::clone(&self.uptime),
            ));
            // 読み取りの間は契機が来ない。飛び・戻りで作り直されたら、終わった後にまた測る
            #[cfg(test)]
            if self.settle_clock_worker {
                if let Some(w) = self.clock_worker.as_mut() {
                    w.settle();
                }
            }
        }
    }

    /// 作業スレッドが返した読み取りを記録にする。**差は応答を受け取った直後の壁時計で計算する**（Q3 ②）。
    ///
    /// 1 つも取れなければ、取れなかった記録を**周期に 1 件だけ**残し、60 秒後に測り直す。
    /// 測り直しで取れたら `retry` の記録を 1 件残す（design D4 / D10）。
    fn finish_skew(
        &mut self,
        reading: ClockReading,
        wall: DateTime<Utc>,
        mono: DateTime<Utc>,
        source: &dyn Source,
    ) {
        let reason = unavailable_error(&reading);
        let ctx = RecordContext {
            trigger: self.skew_trigger,
            at: wall,
            uptime_ms: self.uptime.millis(),
            boot_at: source.boot_time(),
        };
        let p = skew_record(&ctx, reading);
        if p.clock_available == Some(true) {
            self.skew_schedule.succeeded();
            self.push(p);
            return;
        }
        // **推測で埋めない。** 1 分後に測り直す（毎秒は叩かない）
        (self.log)(telemetry::line(
            "clock_skew_unavailable",
            None,
            None,
            Some(&reason),
        ));
        self.skew_schedule.failed(mono);
        if self.skew_schedule.record_unavailable() {
            self.push(p);
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

    /// 履歴の取得を進める。**読みの完了は待たない**（待つと眠りの判定に化ける。design D3）。
    fn maybe_history(&mut self, wall: DateTime<Utc>) {
        let Some(history) = self.history.as_mut() else {
            return;
        };
        let (user_id, device_id, zone, events) =
            (self.user_id, &self.device_id, &self.zone, &mut self.events);
        let mut queue = |visit: &Visit| {
            events.add(IngestRequest::of_visit(
                visit, user_id, device_id, wall, zone,
            )?)
        };
        match history.tick(wall, &mut queue) {
            None => {}
            Some(Ok(queued)) => (self.log)(telemetry::history_line(
                "history_fetched",
                Some(queued),
                None,
                None,
            )),
            // 文言には URL や表示名が入りうるので、種別だけを出す
            Some(Err(_)) => (self.log)(telemetry::history_line(
                "history_fetch_failed",
                Some(1),
                None,
                Some("failed"),
            )),
        }
    }

    /// 履歴のソースの生存信号。区間に読みが 1 回も無いときは、**別スレッドで開けるかを確かめてから**出す（design D12）。
    fn maybe_history_beat(&mut self, wall: DateTime<Utc>, mono: DateTime<Utc>) {
        let (Some(history), beat) = (self.history.as_mut(), &mut self.history_beat) else {
            return;
        };
        let mut counted = false;
        if let Some(profiles) = history.take_reads() {
            beat.record(&profiles);
            counted = true;
        }
        if history.is_probing() {
            // 確かめの結果が出たら、それを区間の読みとして数えて出す
            let Some(profiles) = history.poll_probe() else {
                return self.save_history_counters_if(counted);
            };
            beat.record(&profiles);
        } else if !beat.schedule.due(mono) {
            return self.save_history_counters_if(counted);
        } else if !beat.read_seen {
            history.start_probe();
            return self.save_history_counters_if(counted);
        }
        self.emit_history_beat(wall, mono);
    }

    fn emit_history_beat(&mut self, wall: DateTime<Utc>, mono: DateTime<Utc>) {
        let beat = &mut self.history_beat;
        let (attempts, successes) = beat.counters.take(wall);
        let cap = Capability::from_blockers(&beat.blockers);
        beat.blockers.clear();
        beat.read_seen = false;
        self.save_history_counters();
        match heartbeat::history_signal(
            self.user_id,
            &self.device_id,
            wall,
            &cap,
            attempts,
            successes,
        )
        .and_then(|sig| self.beats.add(sig))
        {
            Ok(()) => self.history_beat.schedule.mark(mono),
            Err(e) => (self.log)(telemetry::line(
                "heartbeat_add_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            )),
        }
    }

    fn save_history_counters_if(&self, counted: bool) {
        if counted {
            self.save_history_counters();
        }
    }

    fn save_history_counters(&self) {
        if let Err(e) = self.history_beat.store.save(&self.history_beat.counters) {
            (self.log)(telemetry::line(
                "counters_save_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
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

/// 取れなかったときのログの `error`。**2 つの基準の理由を両方**、`<基準>:<理由>` で並べる（review R10）。
/// 理由は `TimeSyncError::reason` / `telemetry::error_kind` が名付けた種別だけなので、値は混ざらない。
fn unavailable_error(reading: &crate::clock_worker::ClockReading) -> String {
    use crate::contract::{SOURCE_S01_DATE, SOURCE_TIME_SYNC};
    [
        (SOURCE_S01_DATE, reading.reference.as_ref().err()),
        (SOURCE_TIME_SYNC, reading.time_sync.as_ref().err()),
    ]
    .into_iter()
    .filter_map(|(source, reason)| reason.map(|r| format!("{source}:{r}")))
    .collect::<Vec<_>>()
    .join(",")
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
    use crate::clock;
    use crate::engine::Foreground;
    use crate::exclusion::{Exclusions, Rule};
    use crate::sender::Reply;
    use crate::time_sync::TimeSyncError;
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

    /// 基準の時刻は固定。応答を受け取った直後の壁時計は、読むたびに 1 秒ずつ進む（記録の時刻が重ならない）。
    #[derive(Debug)]
    struct FixedReference(DateTime<Utc>, std::sync::atomic::AtomicI64);

    impl FixedReference {
        fn new(time: DateTime<Utc>) -> Self {
            Self(time, std::sync::atomic::AtomicI64::new(0))
        }
    }

    impl ReferenceClock for FixedReference {
        fn now(&self) -> anyhow::Result<clock::ReferenceReading> {
            let n = self.1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(clock::ReferenceReading {
                time: self.0,
                uptime_before_ms: 0,
                uptime_after_ms: 0,
                wall_after: self.0 + Duration::seconds(n),
            })
        }
        fn source(&self) -> String {
            "127.0.0.1:1".into()
        }
    }

    /// 起動からの経過時間は固定の偽物。
    #[derive(Debug)]
    struct FixedUptime;

    impl Uptime for FixedUptime {
        fn millis(&self) -> u64 {
            42_000
        }
    }

    /// 時刻同期の状態は読めない偽物。
    #[derive(Debug)]
    struct NoTimeSync;

    impl TimeSyncSource for NoTimeSync {
        fn read(&self) -> Result<crate::time_sync::TimeSyncReading, TimeSyncError> {
            Err(TimeSyncError::SpawnFailed)
        }
    }

    fn runtime<'a>(
        cfg: &Config,
        engine: Engine,
        transport: &'a AcceptAll,
        reference: &'a FixedReference,
    ) -> Runtime<'a> {
        Runtime::new(
            cfg,
            zone(),
            engine,
            transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(reference.0)),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
        )
        .unwrap()
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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
        let reference = FixedReference::new(t(0));
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

    /// 基準を読むのに `gate` が開くまで（最長 5 秒）かかる偽物。
    #[derive(Debug)]
    struct SlowReference(std::sync::Mutex<std::sync::mpsc::Receiver<()>>);

    impl ReferenceClock for SlowReference {
        fn now(&self) -> anyhow::Result<clock::ReferenceReading> {
            let _ = self
                .0
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(5));
            anyhow::bail!("打ち切られた")
        }
        fn source(&self) -> String {
            "127.0.0.1:1".into()
        }
    }

    /// 読み取りに失敗するか panic する偽物。読まれた回数を数える。
    #[derive(Debug)]
    struct BrokenReference {
        panics: bool,
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl ReferenceClock for BrokenReference {
        fn now(&self) -> anyhow::Result<clock::ReferenceReading> {
            self.calls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if self.panics {
                panic!("作業スレッドの試験用の panic");
            }
            anyhow::bail!("基準を取れない")
        }
        fn source(&self) -> String {
            "127.0.0.1:1".into()
        }
    }

    fn runtime_with<'a>(
        cfg: &Config,
        transport: &'a AcceptAll,
        reference: Arc<dyn ReferenceClock>,
    ) -> Runtime<'a> {
        Runtime::new(
            cfg,
            zone(),
            Engine::new(Exclusions::default()),
            transport,
            ClockInputs {
                reference,
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
        )
        .unwrap()
    }

    /// 見回りの先頭の壁時計ではなく、応答を受け取った直後の壁時計で差を計算する。
    ///
    /// Scenario: 差に使う PC の時計は基準を読む前後の間で読む
    #[test]
    fn clock_reference_difference_ignores_the_patrol_start_wall() {
        struct Skewed;
        impl std::fmt::Debug for Skewed {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Skewed")
            }
        }
        impl ReferenceClock for Skewed {
            fn now(&self) -> anyhow::Result<clock::ReferenceReading> {
                // 見回りの先頭（t(0)）から応答を受け取るまでに PC の時計が 30 秒進んだ
                Ok(clock::ReferenceReading {
                    time: t(30),
                    uptime_before_ms: 1,
                    uptime_after_ms: 2,
                    wall_after: t(30),
                })
            }
            fn source(&self) -> String {
                "127.0.0.1:1".into()
            }
        }
        let cfg = cfg();
        let transport = AcceptAll::default();
        let mut rt = runtime_with(&cfg, &transport, Arc::new(Skewed));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        rt.tick_at(&mut src, t(0), t(0));
        rt.tick_at(&mut src, t(1), t(1));
        let raws: Vec<String> = rt.events.snapshot().iter().map(|r| r.raw.clone()).collect();
        let skew = raws
            .iter()
            .find(|r| r.contains("clock-skew"))
            .expect("測定記録が積まれていない");
        assert!(
            skew.contains("\"skew_ms\":0"),
            "その 30 秒が差に混ざっている: {skew}"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 基準の読み取りが打ち切りまでかかる間も、1 秒ごとの見回りは止まらず、切り替えは残る。
    ///
    /// Scenario: 基準の読み取りが長引いても前景の切り替えは記録に残る
    #[test]
    fn clock_worker_slow_read_does_not_hold_the_patrol() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let (gate, slow) = std::sync::mpsc::channel();
        let mut rt = runtime_with(
            &cfg,
            &transport,
            Arc::new(SlowReference(std::sync::Mutex::new(slow))),
        );
        rt.settle_clock_worker = false;
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        let started = std::time::Instant::now();
        run(&mut rt, &mut src, 0, 3, 1);
        assert!(rt.clock_worker.is_some(), "読み取りが走っていない");
        src.app = "browser".into();
        run(&mut rt, &mut src, 4, 8, 1);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "見回りが読み取りを待っている"
        );
        assert!(
            rt.clock_worker.is_some(),
            "読み取りはまだ終わっていないはず"
        );
        let foreground = rt
            .events
            .snapshot()
            .iter()
            .filter(|r| r.raw.contains("\"kind\":\"foreground\""))
            .count();
        assert_eq!(foreground, 1, "切り替えの記録が 1 件残っていない");
        drop(gate);
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 取れなかったときのログには、2 つの基準の理由が両方出る（review R10）。
    #[test]
    fn clock_skew_unavailable_log_names_both_reasons() {
        let reading = crate::clock_worker::ClockReading {
            source: "127.0.0.1:1".into(),
            reference: Err("unreachable".into()),
            time_sync: Err("service_stopped".into()),
        };
        assert_eq!(
            unavailable_error(&reading),
            "s01-date:unreachable,windows-time-sync:service_stopped"
        );
        let line = telemetry::line(
            "clock_skew_unavailable",
            None,
            None,
            Some(&unavailable_error(&reading)),
        );
        assert!(
            line.ends_with("error=s01-date:unreachable,windows-time-sync:service_stopped"),
            "{line}"
        );
    }

    /// 基準の読み取りが失敗し続けても、送信の契機で未送信の記録は送られる。
    ///
    /// Scenario: 測定が失敗し続けても送信は続く
    #[test]
    fn clock_worker_failing_reads_do_not_stop_sending() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reference = BrokenReference {
            panics: false,
            calls: Arc::clone(&calls),
        };
        let mut rt = runtime_with(&cfg, &transport, Arc::new(reference));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 100, 1);
        src.app = "browser".into();
        run(&mut rt, &mut src, 101, 400, 1);
        assert!(
            calls.load(std::sync::atomic::Ordering::Relaxed) >= 2,
            "測り直していない"
        );
        assert!(
            !of_kind(&transport, "foreground").is_empty(),
            "未送信の記録が送られていない"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 作業スレッドが panic しても見回りは止まらず、取れなかったとして 60 秒後に測り直す。
    #[test]
    fn clock_worker_panic_is_unavailable_and_retried_after_a_minute() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reference = BrokenReference {
            panics: true,
            calls: Arc::clone(&calls),
        };
        let mut rt = runtime_with(&cfg, &transport, Arc::new(reference));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 130, 1);
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::Relaxed),
            3,
            "0 秒・1 分後・2 分後に測り直すはず"
        );
        assert_eq!(
            skew_records(&mut rt, &transport, 131).len(),
            1,
            "取れなかった記録は測り直しのたびには増えない"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    // ---- 測定記録（clock_skew_record_*。ST05 design D4 / D10） ----

    /// 取れる・取れないを外から切り替えられ、PC の時計の進みも外から決められる基準の偽物。
    #[derive(Debug, Clone)]
    struct SwitchReference {
        ok: Arc<std::sync::atomic::AtomicBool>,
        /// 応答を受け取った直後の PC の時計の、基準の時刻からの進み（ミリ秒）
        pc_ahead_ms: Arc<std::sync::atomic::AtomicI64>,
    }

    impl SwitchReference {
        fn new(ok: bool) -> Self {
            Self {
                ok: Arc::new(std::sync::atomic::AtomicBool::new(ok)),
                pc_ahead_ms: Arc::new(std::sync::atomic::AtomicI64::new(0)),
            }
        }
        fn set_ok(&self, ok: bool) {
            self.ok.store(ok, std::sync::atomic::Ordering::Relaxed);
        }
    }

    impl ReferenceClock for SwitchReference {
        fn now(&self) -> anyhow::Result<crate::clock::ReferenceReading> {
            if !self.ok.load(std::sync::atomic::Ordering::Relaxed) {
                anyhow::bail!("基準を取れない");
            }
            let ahead = self.pc_ahead_ms.load(std::sync::atomic::Ordering::Relaxed);
            Ok(crate::clock::ReferenceReading {
                time: t(1_000),
                uptime_before_ms: 41_000,
                uptime_after_ms: 41_999,
                wall_after: t(1_000) + Duration::milliseconds(ahead),
            })
        }
        fn source(&self) -> String {
            "127.0.0.1:1".into()
        }
    }

    /// 測定記録だけを、積まれた順に返す（送ってから読む）。
    fn skew_records(rt: &mut Runtime, transport: &AcceptAll, end: i64) -> Vec<serde_json::Value> {
        rt.stop_at(t(end));
        of_kind(transport, "clock-skew")
            .into_iter()
            .map(|r| r["payload"].clone())
            .collect()
    }

    /// 1 時間ごとに測定記録が残り、最初は起動・以降は 1 時間ごとの契機を持つ。
    ///
    /// Scenario: 1 時間ごとにずれの測定記録が残る
    /// Scenario: PC の測定記録に測った契機が残る
    /// Scenario: PC の測定記録に起動の識別と起動からの経過時間が入っている
    #[test]
    fn clock_skew_record_is_left_hourly_with_trigger_and_boot_identity() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let mut rt = runtime_with(&cfg, &transport, Arc::new(SwitchReference::new(true)));
        let mut src = FakeSource::new("editor");
        src.boot = Some(t(-60));
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 7_300, 10);
        let recs = skew_records(&mut rt, &transport, 7_301);
        let triggers: Vec<_> = recs.iter().map(|p| p["clock_trigger"].clone()).collect();
        assert_eq!(triggers, ["start", "hourly", "hourly"], "{recs:?}");
        for p in &recs {
            assert_eq!(p["uptime_ms"], 42_000);
            assert_eq!(p["boot_at"], crate::contract::rfc3339(t(-60)));
        }
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 壁時計が飛ぶと、次の 1 時間の契機を待たずに、飛んだ後の差を持つ記録が残る。
    ///
    /// Scenario: 壁時計が飛ぶとその場で測る
    /// Scenario: 時計が飛んだ直後の測定は飛んだ後の差を持つ
    /// Scenario: PC の測定記録に測った契機が残る
    #[test]
    fn clock_skew_record_is_taken_at_once_after_the_clock_jumps() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = SwitchReference::new(true);
        let mut rt = runtime_with(&cfg, &transport, Arc::new(reference.clone()));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 100, 1);

        // PC の時計が 5 分先へ飛ぶ（単調時計はそのまま進む）
        reference
            .pc_ahead_ms
            .store(300_000, std::sync::atomic::Ordering::Relaxed);
        for s in 101..=110 {
            rt.tick_at(&mut src, t(s + 300), t(s));
        }
        let recs = skew_records(&mut rt, &transport, 111);
        assert_eq!(recs.len(), 2, "{recs:?}");
        assert_eq!(recs[0]["clock_trigger"], "start");
        assert_eq!(recs[0]["skew_ms"], 0);
        assert_eq!(recs[1]["clock_trigger"], "jump");
        assert_eq!(recs[1]["skew_ms"], 300_000, "飛んだ後の差");
        assert_eq!(
            recs[1]["clock_references"][0]["skew_ms"], 300_000,
            "基準ごとの差も飛んだ後"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 基準が 1 つも取れないと、取れなかった印の付いた記録が 1 件残り、基準ごとの理由を持つ。
    ///
    /// Scenario: PC で基準が 1 つも取れないと取れなかった印の付いた記録が残る
    /// Scenario: PC の取れなかった記録は基準ごとの理由を持つ
    #[test]
    fn clock_skew_record_unavailable_has_a_reason_per_source() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let mut rt = runtime_with(&cfg, &transport, Arc::new(SwitchReference::new(false)));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 10, 1);
        let recs = skew_records(&mut rt, &transport, 11);
        assert_eq!(recs.len(), 1, "{recs:?}");
        let p = &recs[0];
        assert_eq!(p["clock_available"], false);
        assert_eq!(p["clock_references"], serde_json::json!([]));
        let un = p["clock_unavailable"].as_array().unwrap();
        let of = |source: &str| un.iter().find(|u| u["source"] == source).unwrap();
        assert_eq!(of("s01-date")["reason"], "unreachable");
        assert_eq!(of("windows-time-sync")["reason"], "spawn_failed");
        assert!(p.get("skew_ms").is_none(), "取れなかった記録に差は無い");
        assert!(p.get("skew_reference").is_none());
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 取れない状態が 1 時間続いても、その契機の取れなかった記録は 1 件（測り直しのたびには増えない）。
    /// 次の 1 時間の契機では改めて 1 件残る。
    ///
    /// Scenario: PC の測り直しのたびには記録を増やさない
    #[test]
    fn clock_skew_record_unavailable_is_not_repeated_by_retries() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reference = BrokenReference {
            panics: false,
            calls: Arc::clone(&calls),
        };
        let mut rt = runtime_with(&cfg, &transport, Arc::new(reference));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 3_500, 5);
        assert!(
            calls.load(std::sync::atomic::Ordering::Relaxed) >= 50,
            "測り直していない"
        );
        assert_eq!(skew_records(&mut rt, &transport, 3_501).len(), 1);

        // 次の 1 時間の契機
        run(&mut rt, &mut src, 3_502, 3_700, 5);
        let recs = skew_records(&mut rt, &transport, 3_701);
        assert_eq!(recs.len(), 2, "{recs:?}");
        assert_eq!(recs[1]["clock_trigger"], "hourly");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 取れなかった契機のあと、測り直しで取れたら、別の 1 件が `retry` で残る。
    ///
    /// Scenario: PC の測り直しで取れたら別の 1 件が残る
    #[test]
    fn clock_skew_record_retry_that_succeeds_leaves_another_one() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = SwitchReference::new(false);
        let mut rt = runtime_with(&cfg, &transport, Arc::new(reference.clone()));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 200, 1);
        reference.set_ok(true);
        run(&mut rt, &mut src, 201, 400, 1);
        let recs = skew_records(&mut rt, &transport, 401);
        assert_eq!(recs.len(), 2, "{recs:?}");
        assert_eq!(recs[0]["clock_available"], false);
        assert_eq!(recs[0]["clock_trigger"], "start");
        assert_eq!(recs[1]["clock_available"], true);
        assert_eq!(recs[1]["clock_trigger"], "retry");
        assert!(recs[1].get("skew_ms").is_some());
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 取れた測定のあとは、測り直しの記録は増えない（次は 1 時間後）。
    #[test]
    fn clock_skew_record_success_stops_the_retries() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = SwitchReference::new(false);
        let mut rt = runtime_with(&cfg, &transport, Arc::new(reference.clone()));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 100, 1);
        reference.set_ok(true);
        run(&mut rt, &mut src, 101, 1_000, 1);
        assert_eq!(skew_records(&mut rt, &transport, 1_001).len(), 2);
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// PC の時計が進んでいても、測定記録も前景の記録も観測したときの PC の時計のまま。
    ///
    /// Scenario: PC の記録の時刻は補正されない
    #[test]
    fn clock_skew_record_time_is_not_corrected() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = SwitchReference::new(true);
        // PC の時計は基準より 10 分進んでいる
        reference
            .pc_ahead_ms
            .store(600_000, std::sync::atomic::Ordering::Relaxed);
        let mut rt = runtime_with(&cfg, &transport, Arc::new(reference));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 5, 1);
        rt.stop_at(t(6));
        let fg = of_kind(&transport, "foreground");
        assert_eq!(fg.len(), 1);
        assert_eq!(fg[0]["payload"]["at"], crate::contract::rfc3339(t(0)));
        assert_eq!(fg[0]["event_time"], crate::contract::rfc3339(t(0)));
        let skew = of_kind(&transport, "clock-skew");
        assert_eq!(skew[0]["payload"]["skew_ms"], 600_000);
        assert_eq!(
            skew[0]["payload"]["at"],
            crate::contract::rfc3339(t(1)),
            "測定記録の時刻は積んだときの PC の時計で、差で補正されない"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 起動したとき壁時計が前回の印より戻っていたら、契機は `jump`（戻り）で残る。
    ///
    /// Scenario: PC の測定記録に測った契機が残る
    #[test]
    fn clock_skew_record_after_the_clock_went_back_has_jump_trigger() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        Marker::new(&cfg.state_dir).touch(t(500)).unwrap();
        let mut rt = runtime_with(&cfg, &transport, Arc::new(SwitchReference::new(true)));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 10, 1);
        let recs = skew_records(&mut rt, &transport, 11);
        assert_eq!(recs.len(), 1, "{recs:?}");
        assert_eq!(recs[0]["clock_trigger"], "jump");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    // ---- ブラウザ履歴の取得（ST08）。読み手は偽物で、取り込み口まで通す ----

    use crate::history::collect::{ProfileRead, ReadOutcome};
    use crate::history::locate::Browser;
    use crate::history::read::ReadVisit;
    use std::sync::{mpsc, Arc, Mutex};

    fn read_visit(id: i64, title: &str) -> ReadVisit {
        ReadVisit {
            id,
            visit_time_raw: 13_402_627_200_000_000 + id,
            url: format!("https://example.test/{id}"),
            title: Some(title.into()),
            at: t(-86_400),
            transition: 0,
            from_visit: None,
            opener_visit: None,
            duration_us: None,
            originator_cache_guid: None,
            originator_visit_id: None,
            is_known_to_sync: None,
        }
    }

    /// 読み終えるのを `release` で止められる偽の読み手（本物の「3 分かかる」を待たずに再現する）。
    #[derive(Debug)]
    struct FakeReader {
        gate: Option<Mutex<mpsc::Receiver<()>>>,
        visits: Mutex<Vec<ReadVisit>>,
        reads: Mutex<usize>,
        /// 次の読みで、プロファイルが読めない / ディレクトリごと無い、を再現する
        profile: Mutex<ProfileState>,
        /// ほかのプロファイル（ブラウザ・ディレクトリ名・訪問）。除外の写像を見るためのもの
        others: Mutex<Vec<(Browser, String, Vec<ReadVisit>)>>,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum ProfileState {
        Readable,
        Unreadable,
        /// 読みに出てこないが、ディレクトリは在る（置き場が一時的に読めない・DB を作り直し中）
        Missing,
        Gone,
    }

    impl FakeReader {
        fn new(visits: Vec<ReadVisit>) -> Arc<Self> {
            Arc::new(Self {
                gate: None,
                visits: Mutex::new(visits),
                reads: Mutex::new(0),
                profile: Mutex::new(ProfileState::Readable),
                others: Mutex::new(Vec::new()),
            })
        }

        fn gated(visits: Vec<ReadVisit>) -> (Arc<Self>, mpsc::Sender<()>) {
            let (tx, rx) = mpsc::channel();
            let reader = Arc::new(Self {
                gate: Some(Mutex::new(rx)),
                visits: Mutex::new(visits),
                reads: Mutex::new(0),
                profile: Mutex::new(ProfileState::Readable),
                others: Mutex::new(Vec::new()),
            });
            (reader, tx)
        }

        fn reads(&self) -> usize {
            *self.reads.lock().unwrap()
        }
    }

    impl HistoryReader for FakeReader {
        fn read(&self) -> anyhow::Result<ReadOutcome> {
            if let Some(gate) = &self.gate {
                gate.lock().unwrap().recv().ok();
            }
            *self.reads.lock().unwrap() += 1;
            let visits = match *self.profile.lock().unwrap() {
                ProfileState::Gone | ProfileState::Missing => {
                    return Ok(ReadOutcome::default());
                }
                ProfileState::Unreadable => Err(anyhow::anyhow!("履歴 DB を開けない")),
                ProfileState::Readable => Ok(self.visits.lock().unwrap().clone()),
            };
            let mut profiles = vec![ProfileRead {
                browser: Browser::Chrome,
                directory: "Default".into(),
                visits,
            }];
            for (browser, directory, visits) in self.others.lock().unwrap().iter() {
                profiles.push(ProfileRead {
                    browser: *browser,
                    directory: directory.clone(),
                    visits: Ok(visits.clone()),
                });
            }
            Ok(ReadOutcome {
                profiles,
                names: Vec::new(),
            })
        }

        fn probe(&self) -> Vec<crate::history::collect::ProfileHealth> {
            let readable = match *self.profile.lock().unwrap() {
                ProfileState::Gone | ProfileState::Missing => return Vec::new(),
                ProfileState::Unreadable => false,
                ProfileState::Readable => true,
            };
            vec![crate::history::collect::ProfileHealth {
                browser: Browser::Chrome,
                directory: "Default".into(),
                readable,
            }]
        }

        fn profile_dir_absent(&self, _browser: Browser, _directory: &str) -> bool {
            *self.profile.lock().unwrap() == ProfileState::Gone
        }
    }

    fn history_runtime<'a>(
        cfg: &Config,
        transport: &'a AcceptAll,
        reference: &'a FixedReference,
        reader: Arc<FakeReader>,
    ) -> Runtime<'a> {
        runtime(
            cfg,
            Engine::new(Exclusions::default()),
            transport,
            reference,
        )
        .with_history(reader)
    }

    fn write_last_success(cfg: &Config, at: DateTime<Utc>) {
        let path = cfg.state_dir.join("browser-history/last_success.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!(r#"{{"last_success":"{}"}}"#, at.to_rfc3339())).unwrap();
    }

    /// 読みが終わるまで（実時間で）見回りを回す。
    fn tick_until_read(rt: &mut Runtime, src: &mut FakeSource, reader: &FakeReader, from: i64) {
        for sec in from..from + 2000 {
            rt.tick_at(src, t(sec), t(sec));
            if reader.reads() > 0 && rt.history.as_ref().is_some_and(|h| !h.is_reading()) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("履歴の取得が終わらない");
    }

    fn history_records(t: &AcceptAll) -> Vec<serde_json::Value> {
        sent(t, "/ingest")
            .into_iter()
            .filter(|r| r["logical_source"] == "c02-browser-history")
            .collect()
    }

    fn history_beats(t: &AcceptAll) -> Vec<serde_json::Value> {
        sent(t, "/heartbeat")
            .into_iter()
            .filter(|r| r["logical_source"] == "c02-browser-history")
            .collect()
    }

    /// 履歴の生存信号が `want` 件届くまで（実時間で）見回りを回す。確かめも読みも別スレッドなので同じ時刻に回し直す。
    fn tick_until_history_beats(
        rt: &mut Runtime,
        src: &mut FakeSource,
        transport: &AcceptAll,
        sec: i64,
        want: usize,
    ) {
        for _ in 0..400 {
            rt.tick_at(src, t(sec), t(sec));
            rt.send();
            if history_beats(transport).len() >= want {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("履歴の生存信号が届かない");
    }

    /// Scenario: ブラウザ履歴のソースにも想定間隔ごとに生存信号が届く
    /// Scenario: ブラウザ履歴の生存信号はウィンドウの生存信号と別の件である
    #[test]
    fn history_heartbeat_every_day_and_separate_from_window_at_runtime() {
        let cfg = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        // 2 日回す。24 時間より前には増えない
        for sec in (600..2 * 86_400).step_by(600) {
            rt.tick_at(&mut src, t(sec), t(sec));
            if sec == 86_400 - 600 {
                rt.send();
                assert_eq!(history_beats(&transport).len(), 1, "24 時間より早く出た");
            }
            if sec == 86_400 {
                tick_until_history_beats(&mut rt, &mut src, &transport, sec, 2);
            }
        }
        rt.send();
        assert_eq!(history_beats(&transport).len(), 2);
        let window = sent(&transport, "/heartbeat")
            .into_iter()
            .filter(|r| r["logical_source"] == "c02-window")
            .count();
        assert!(
            window >= 8,
            "ウィンドウの生存信号が別に届いていない: {window}"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: 読めないプロファイルが 1 つでもあれば取得できないとして報告される
    /// Scenario: 読めなかったブラウザとプロファイルが満たされていないものに挙がる
    #[test]
    fn history_heartbeat_reports_unreadable_profile_at_runtime() {
        let cfg = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        *reader.profile.lock().unwrap() = ProfileState::Unreadable;
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        // 起動直後は読みの結果を待たずに確かめる。読みが終わっていても同じ結果になる
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        let beat = &history_beats(&transport)[0];
        assert_eq!(beat["capturable"], false);
        assert_eq!(
            beat["blockers"],
            serde_json::json!(["history-unreadable:chrome:Default"])
        );
        assert!(!beat["raw"].as_str().unwrap().contains("example.test"));
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: 履歴が 1 つも見つからなければ取得できないとして報告される
    #[test]
    fn history_heartbeat_reports_none_found_at_runtime() {
        let cfg = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(Vec::new());
        *reader.profile.lock().unwrap() = ProfileState::Missing;
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        let beat = &history_beats(&transport)[0];
        assert_eq!(beat["capturable"], false);
        assert_eq!(beat["blockers"], serde_json::json!(["history-none-found"]));
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 区間に読みが 1 回も無い（直近の取得が 24 時間以内で読みを始めない）起動直後でも、開けるかを確かめて報告する。
    ///
    /// Scenario: 区間に読みが無くても、開けるかを確かめてから報告する
    #[test]
    fn history_heartbeat_probes_when_not_read_at_runtime() {
        let cfg = cfg();
        write_last_success(&cfg, t(0));
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        *reader.profile.lock().unwrap() = ProfileState::Unreadable;
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        assert_eq!(reader.reads(), 0, "確かめだけで行を読んでいる");
        let beat = &history_beats(&transport)[0];
        assert_eq!(beat["capturable"], false);
        assert_eq!(beat["attempts"], 1, "確かめが 1 試行に数えられていない");
        assert_eq!(
            beat["blockers"],
            serde_json::json!(["history-unreadable:chrome:Default"])
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 起動直後に 1 件出る（読みも契機も待たない）。
    ///
    /// Scenario: ブラウザ履歴のソースにも想定間隔ごとに生存信号が届く
    #[test]
    fn history_heartbeat_on_start_at_runtime() {
        let cfg = cfg();
        write_last_success(&cfg, t(0));
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        assert_eq!(history_beats(&transport)[0]["capturable"], true);
        assert!(cfg.state_dir.join("counters-browser-history.json").exists());
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: 起動時に前回の成功から 24 時間以上経っていれば取得する
    #[test]
    fn history_schedule_runtime_fetches_on_first_tick_after_24h() {
        let cfg = cfg();
        write_last_success(&cfg, t(0) - Duration::hours(24));
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        assert_eq!(reader.reads(), 1);
        rt.send();
        assert_eq!(history_records(&transport).len(), 1);
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: 前回の成功から 24 時間経たないうちは取得しない
    #[test]
    fn history_schedule_runtime_skips_within_24h() {
        let cfg = cfg();
        write_last_success(&cfg, t(0) - Duration::hours(24) + Duration::seconds(60));
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 30, 1);
        std::thread::sleep(std::time::Duration::from_millis(100));
        run(&mut rt, &mut src, 31, 40, 1);
        assert_eq!(reader.reads(), 0, "24 時間経たないのに読んだ");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: 動作中に前回の成功から 24 時間経つと取得する
    #[test]
    fn history_schedule_runtime_fetches_when_24h_pass_while_running() {
        let cfg = cfg();
        write_last_success(&cfg, t(0));
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 10, 1);
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(reader.reads(), 0);
        let due = Duration::hours(24).num_seconds();
        rt.tick_at(&mut src, t(due - 1), t(due - 1));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(reader.reads(), 0, "24 時間の 1 秒前に読んだ");
        tick_until_read(&mut rt, &mut src, &reader, due);
        assert_eq!(reader.reads(), 1);
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 取得の成功は置き場へ書かれ、次の起動では 24 時間経つまで読まない。
    #[test]
    fn history_schedule_runtime_persists_success_across_restart() {
        let cfg = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        {
            let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
            let mut src = FakeSource::new("editor");
            rt.start_at(&src, t(0), t(0));
            tick_until_read(&mut rt, &mut src, &reader, 0);
        }
        let again = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, again.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(3600), t(3600));
        run(&mut rt, &mut src, 3600, 3640, 1);
        std::thread::sleep(std::time::Duration::from_millis(100));
        run(&mut rt, &mut src, 3641, 3650, 1);
        assert_eq!(again.reads(), 0);
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 1 回目で送った訪問は、2 回目に読んでも積まれない。変わった訪問と新しい訪問だけが積まれる。
    #[test]
    fn history_runtime_second_fetch_queues_only_the_difference() {
        let cfg = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a"), read_visit(2, "b")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        rt.send();
        assert_eq!(history_records(&transport).len(), 2);

        let day = Duration::hours(24).num_seconds();
        tick_until_read_again(&mut rt, &mut src, &reader, day);
        rt.send();
        assert_eq!(
            history_records(&transport).len(),
            2,
            "変わらない訪問を積んだ"
        );

        *reader.visits.lock().unwrap() = vec![read_visit(1, "a"), read_visit(2, "題名が変わった")];
        tick_until_read_again(&mut rt, &mut src, &reader, 2 * day + 10);
        rt.send();
        assert_eq!(history_records(&transport).len(), 3);
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    fn history_urls(t: &AcceptAll) -> Vec<String> {
        history_records(t)
            .iter()
            .map(|r| r["payload"]["url"].as_str().unwrap().to_string())
            .collect()
    }

    /// Scenario: 初回の取得で過去の履歴が入る
    #[test]
    fn history_runtime_first_fetch_sends_past_visits() {
        let cfg = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let mut old = read_visit(1, "一年前");
        old.at = t(0) - Duration::days(365);
        let reader = FakeReader::new(vec![old, read_visit(2, "昨日")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        rt.send();
        let mut urls = history_urls(&transport);
        urls.sort();
        assert_eq!(
            urls,
            vec!["https://example.test/1", "https://example.test/2"]
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: 前回の取得の後に古い時刻で入った訪問も取り込まれる
    #[test]
    fn history_runtime_late_arriving_old_visit_is_sent() {
        let cfg = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "最初")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        rt.send();
        assert_eq!(history_urls(&transport), vec!["https://example.test/1"]);

        // 前回の取得より前の時刻の訪問が、同期で後から履歴 DB に入る。
        let mut late = read_visit(2, "後から同期");
        late.at = t(0) - Duration::days(30);
        reader.visits.lock().unwrap().push(late);
        let day = Duration::hours(24).num_seconds();
        tick_until_read_again(&mut rt, &mut src, &reader, day);
        rt.send();
        assert_eq!(
            history_urls(&transport),
            vec!["https://example.test/1", "https://example.test/2"]
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    fn vanished_records(t: &AcceptAll) -> Vec<serde_json::Value> {
        history_records(t)
            .into_iter()
            .filter(|r| r["payload"]["kind"] == "vanished")
            .collect()
    }

    fn visit_id_of(t: &AcceptAll, url: &str) -> String {
        history_records(t)
            .iter()
            .find(|r| r["payload"]["url"] == url)
            .unwrap()["external_id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    /// 1 回目に `first` を取り、1 日後に `second` を取って、2 回目までに積まれた `vanished` を返す。
    fn fetch_then_change(
        first: Vec<ReadVisit>,
        change: impl FnOnce(&FakeReader),
    ) -> (AcceptAll, Vec<serde_json::Value>) {
        let cfg = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(first);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        rt.send();
        change(&reader);
        tick_until_read_again(
            &mut rt,
            &mut src,
            &reader,
            Duration::hours(24).num_seconds(),
        );
        rt.send();
        std::fs::remove_dir_all(&cfg.state_dir).ok();
        let vanished = vanished_records(&transport);
        (transport, vanished)
    }

    /// Scenario: 履歴から 1 件消すと次の取得で「消えた」記録が残る
    #[test]
    fn history_vanished_is_recorded_after_next_fetch() {
        let (transport, vanished) =
            fetch_then_change(vec![read_visit(1, "a"), read_visit(2, "b")], |r| {
                r.visits.lock().unwrap().retain(|v| v.id != 2);
            });
        assert_eq!(vanished.len(), 1);
        let gone = visit_id_of(&transport, "https://example.test/2");
        assert_eq!(vanished[0]["payload"]["vanished"][0]["external_id"], gone);
        assert_eq!(
            vanished[0]["payload"]["vanished"].as_array().unwrap().len(),
            1
        );
    }

    /// Scenario: 消えた訪問の、訪問から取得までの日数が本文にある
    #[test]
    fn history_vanished_has_age_days_over_90() {
        let mut old = read_visit(1, "古い");
        old.at = t(0) - Duration::days(100);
        let (_, vanished) = fetch_then_change(vec![old], |r| r.visits.lock().unwrap().clear());
        let age = vanished[0]["payload"]["vanished"][0]["age_days"]
            .as_i64()
            .unwrap();
        assert!(age > 90, "age_days = {age}");
    }

    /// Scenario: 同期で入った訪問が消えたことが本文にある
    #[test]
    fn history_vanished_marks_foreign_visit() {
        let mut synced = read_visit(1, "他の端末");
        synced.originator_cache_guid = Some("other-device".into());
        let (_, vanished) = fetch_then_change(vec![synced, read_visit(2, "ここ")], |r| {
            r.visits.lock().unwrap().clear();
            r.visits.lock().unwrap().push(read_visit(2, "ここ"));
        });
        assert_eq!(vanished[0]["payload"]["vanished"][0]["foreign"], true);
    }

    /// Scenario: 表が作り直されたことが本文にある
    #[test]
    fn history_vanished_marks_recreated_table() {
        let (_, vanished) = fetch_then_change(vec![read_visit(5, "a"), read_visit(6, "b")], |r| {
            *r.visits.lock().unwrap() = vec![read_visit(1, "作り直した後")];
        });
        let items = vanished[0]["payload"]["vanished"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i["table_recreated"] == true));
        assert!(items.iter().all(|i| i["profile_gone"] == false));
    }

    /// Scenario: プロファイルが無くなったことが本文にある
    #[test]
    fn history_vanished_marks_gone_profile() {
        let (_, vanished) = fetch_then_change(vec![read_visit(1, "a")], |r| {
            *r.profile.lock().unwrap() = ProfileState::Gone;
        });
        assert_eq!(vanished.len(), 1);
        let item = &vanished[0]["payload"]["vanished"][0];
        assert_eq!(item["profile_gone"], true);
        assert_eq!(item["table_recreated"], false);
    }

    /// Scenario: 消えた経路を名指しする値を持たない
    /// Scenario: 消えた記録に URL と題名が載らない
    #[test]
    fn history_vanished_has_no_named_cause_url_or_title() {
        let (_, vanished) = fetch_then_change(vec![read_visit(1, "秘密の題名")], |r| {
            r.visits.lock().unwrap().clear();
        });
        assert_eq!(vanished.len(), 1);
        let mut keys: Vec<_> = vanished[0]["payload"]["vanished"][0]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "age_days",
                "external_id",
                "foreign",
                "profile_gone",
                "table_recreated"
            ]
        );
        let body = vanished[0].to_string();
        assert!(!body.contains("example.test") && !body.contains("秘密の題名"));
        assert!(!body.contains("deleted") && !body.contains("expired"));
    }

    #[test]
    fn history_vanished_not_gone_while_profile_dir_exists() {
        let (_, vanished) = fetch_then_change(vec![read_visit(1, "a")], |r| {
            *r.profile.lock().unwrap() = ProfileState::Missing;
        });
        assert!(vanished.is_empty(), "ディレクトリが在るのに消えたとした");
    }

    #[test]
    fn history_vanished_all_gone_is_not_recreated_table_at_runtime() {
        let (_, vanished) = fetch_then_change(vec![read_visit(1, "a")], |r| {
            r.visits.lock().unwrap().clear();
        });
        assert_eq!(
            vanished[0]["payload"]["vanished"][0]["table_recreated"],
            false
        );
    }

    /// Scenario: 読めなかったプロファイルでは消えた記録を出さない
    #[test]
    fn history_vanished_skips_unreadable_profile_at_runtime() {
        let (_, vanished) = fetch_then_change(vec![read_visit(1, "a")], |r| {
            *r.profile.lock().unwrap() = ProfileState::Unreadable;
        });
        assert!(vanished.is_empty());
    }

    /// Scenario: 取得をやり直しても「消えた」記録は増えない
    #[test]
    fn history_vanished_is_idempotent_on_retry_at_runtime() {
        let cfg = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        rt.send();
        let ledger = cfg.state_dir.join("browser-history/chrome/Default.ledger");
        let before = std::fs::read(&ledger).unwrap();

        reader.visits.lock().unwrap().clear();
        let day = Duration::hours(24).num_seconds();
        tick_until_read_again(&mut rt, &mut src, &reader, day);
        rt.send();
        drop(rt);
        // 積んだ後・取得の成功を書く前に止まった（帳面は前のまま）。起動し直して、少し後にやり直す
        std::fs::write(&ledger, before).unwrap();
        std::fs::remove_file(cfg.state_dir.join("browser-history/last_success.json")).unwrap();
        let mut again = history_runtime(&cfg, &transport, &reference, reader.clone());
        let before_reads = reader.reads();
        for sec in day + 1000..day + 3000 {
            again.tick_at(&mut src, t(sec), t(sec));
            if reader.reads() > before_reads && !again.history.as_ref().unwrap().is_reading() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        again.send();
        let mut ids: Vec<_> = vanished_records(&transport)
            .iter()
            .map(|r| r["external_id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids.len(), 2, "やり直しで積み直した");
        ids.dedup();
        assert_eq!(ids.len(), 1, "同じ事実が別の識別子になった");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 次の読みが終わるまで（`reads` が増えるまで）見回しを回す。
    fn tick_until_read_again(
        rt: &mut Runtime,
        src: &mut FakeSource,
        reader: &FakeReader,
        from: i64,
    ) {
        let before = reader.reads();
        for sec in from..from + 2000 {
            rt.tick_at(src, t(sec), t(sec));
            if reader.reads() > before && rt.history.as_ref().is_some_and(|h| !h.is_reading()) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("履歴の取得が終わらない");
    }

    /// 取り込み口が止まっていても取得は未送信へ積まれ、戻った後に届く。
    ///
    /// Scenario: 取り込み口が止まっている間に取得した履歴が後から届く
    #[test]
    fn history_success_only_after_outbox() {
        #[derive(Debug)]
        struct Switch {
            up: std::cell::Cell<bool>,
            inner: AcceptAll,
        }
        impl Transport for Switch {
            fn post(&self, path: &str, body: &str) -> anyhow::Result<Reply> {
                if self.up.get() {
                    self.inner.post(path, body)
                } else {
                    anyhow::bail!("取り込み口が止まっている")
                }
            }
        }
        let cfg = cfg();
        let transport = Switch {
            up: std::cell::Cell::new(false),
            inner: AcceptAll::default(),
        };
        let reference = FixedReference(t(0));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
            t(0),
        )
        .unwrap()
        .with_history(reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        rt.send();
        assert!(history_records(&transport.inner).is_empty());
        assert!(rt.pending().0 >= 1, "止まっている間は未送信に残る");

        transport.up.set(true);
        rt.send();
        let stored = history_records(&transport.inner);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0]["payload"]["url"], "https://example.test/1");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 読みが数分かかっても見回りは続き、ウィンドウのソースに眠りも履歴の記録も入らない。
    ///
    /// Scenario: 履歴の取得でウィンドウのソースの記録は増えない
    #[test]
    fn history_slow_read_does_not_disturb_window() {
        let window_records = |with_history: bool| {
            let cfg = cfg();
            let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
            let (reader, release) = FakeReader::gated(vec![read_visit(1, "a")]);
            let mut rt = Runtime::new(
                &cfg,
                zone(),
                Engine::new(Exclusions::default()),
                &transport,
                &reference,
                t(0),
            )
            .unwrap();
            if with_history {
                rt = rt.with_history(reader.clone());
            }
            let mut src = FakeSource::new("editor");
            rt.start_at(&src, t(0), t(0));
            // 読み手が 3 分止まっている間も、1 秒ごとの見回りは続く
            run(&mut rt, &mut src, 0, 180, 1);
            assert_eq!(reader.reads(), 0, "読みはまだ終わっていない");
            release.send(()).unwrap();
            if with_history {
                tick_until_read(&mut rt, &mut src, &reader, 181);
            } else {
                run(&mut rt, &mut src, 181, 190, 1);
            }
            rt.send();
            let window: Vec<_> = sent(&transport, "/ingest")
                .into_iter()
                .filter(|r| r["logical_source"] == "c02-window")
                .collect();
            assert!(
                window.iter().all(|r| r["payload"]["reason"] != "suspended"),
                "履歴の読みで眠りが入った"
            );
            assert!(
                window.iter().all(|r| r["payload"]["kind"] != "visit"),
                "履歴の記録がウィンドウのソースに入った"
            );
            if with_history {
                assert_eq!(history_records(&transport).len(), 1);
            }
            std::fs::remove_dir_all(&cfg.state_dir).ok();
            window.len()
        };
        assert_eq!(window_records(true), window_records(false));
    }

    // ---- 履歴の除外（ST08 Task 7）。取り込み口へ送られた本文で確かめる ----

    fn write_rules(cfg: &Config, rules: &str) {
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        std::fs::write(
            cfg.state_dir.join("exclusions.json"),
            format!(r#"{{"rules": {rules}}}"#),
        )
        .unwrap();
    }

    fn with_other_profile(reader: &FakeReader, directory: &str, visits: Vec<ReadVisit>) {
        reader
            .others
            .lock()
            .unwrap()
            .push((Browser::Chrome, directory.into(), visits));
    }

    /// 1 回取得して送る。取り込み口に届いた全部の本文（文字列）を返す。
    fn fetch_and_send(cfg: &Config, reader: &Arc<FakeReader>) -> (AcceptAll, String) {
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let mut rt = history_runtime(cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, reader, 0);
        rt.send();
        let all = transport
            .bodies
            .borrow()
            .iter()
            .map(|(_, b)| b.clone())
            .collect::<String>();
        (transport, all)
    }

    fn excluded_records(t: &AcceptAll) -> Vec<serde_json::Value> {
        history_records(t)
            .into_iter()
            .filter(|r| r["payload"]["kind"] == "excluded")
            .collect()
    }

    /// Scenario: ブラウザのプロセスを除外するとその全プロファイルの履歴が送られない
    #[test]
    fn history_exclusion_process_covers_all_profiles() {
        let cfg = cfg();
        write_rules(&cfg, r#"[{"match":"process-name","value":"chrome.exe"}]"#);
        let reader = FakeReader::new(vec![read_visit(1, "ひとつめの題名")]);
        with_other_profile(&reader, "Profile 2", vec![read_visit(2, "ふたつめの題名")]);
        let (transport, all) = fetch_and_send(&cfg, &reader);
        for leaked in [
            "example.test/1",
            "example.test/2",
            "ひとつめの題名",
            "ふたつめの題名",
        ] {
            assert!(
                !all.contains(leaked),
                "除外したはずの本文が送られた: {leaked}"
            );
        }
        assert!(history_urls_of_kind(&transport, "visit").is_empty());
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    fn history_urls_of_kind(t: &AcceptAll, kind: &str) -> Vec<String> {
        history_records(t)
            .iter()
            .filter(|r| r["payload"]["kind"] == kind)
            .map(|r| r["payload"]["url"].as_str().unwrap().to_string())
            .collect()
    }

    /// Scenario: 履歴で除外した件数が残る
    #[test]
    fn history_exclusion_count_is_recorded() {
        let cfg = cfg();
        write_rules(&cfg, r#"[{"match":"process-name","value":"chrome.exe"}]"#);
        let reader = FakeReader::new(vec![
            read_visit(1, "a"),
            read_visit(2, "b"),
            read_visit(3, "c"),
        ]);
        let (transport, _) = fetch_and_send(&cfg, &reader);
        let excluded = excluded_records(&transport);
        assert_eq!(excluded.len(), 1, "1 回・1 プロファイルにつき 1 件");
        assert_eq!(excluded[0]["payload"]["excluded_count"], 3);
        assert!(
            excluded[0]["payload"]["url"].is_null() && excluded[0]["payload"]["title"].is_null()
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 2 回目の取得（1 日後）まで回して送る。
    fn fetch_twice(cfg: &Config, reader: &Arc<FakeReader>) -> AcceptAll {
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let mut rt = history_runtime(cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, reader, 0);
        rt.send();
        tick_until_read_again(&mut rt, &mut src, reader, Duration::hours(24).num_seconds());
        rt.send();
        transport
    }

    /// Scenario: 除外した訪問は取得のたびに数え直されない
    #[test]
    fn history_exclusion_is_not_recounted_each_fetch() {
        let cfg = cfg();
        write_rules(&cfg, r#"[{"match":"process-name","value":"chrome.exe"}]"#);
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let transport = fetch_twice(&cfg, &reader);
        let counted: u64 = excluded_records(&transport)
            .iter()
            .map(|r| r["payload"]["excluded_count"].as_u64().unwrap())
            .sum();
        assert_eq!(counted, 1, "2 回目の取得で数え直した");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: 題名の部分一致の登録はページの題名に当たる
    #[test]
    fn history_exclusion_title_matches_page_title() {
        let cfg = cfg();
        write_rules(
            &cfg,
            r#"[{"match":"title-contains","value":"シークレット"}]"#,
        );
        let reader = FakeReader::new(vec![
            read_visit(1, "シークレットの頁"),
            read_visit(2, "普通"),
        ]);
        let (transport, all) = fetch_and_send(&cfg, &reader);
        assert!(!all.contains("example.test/1") && !all.contains("シークレットの頁"));
        assert_eq!(
            history_urls_of_kind(&transport, "visit"),
            vec!["https://example.test/2"]
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: URL の部分一致の登録は履歴にも効く
    #[test]
    fn history_exclusion_url_matches_history() {
        let cfg = cfg();
        write_rules(
            &cfg,
            r#"[{"match":"url-contains","value":"example.test/1"}]"#,
        );
        let reader = FakeReader::new(vec![read_visit(1, "a"), read_visit(2, "b")]);
        let (transport, all) = fetch_and_send(&cfg, &reader);
        assert!(!all.contains("example.test/1"));
        assert_eq!(
            history_urls_of_kind(&transport, "visit"),
            vec!["https://example.test/2"]
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: プロファイルを指す登録はそのプロファイルの履歴だけを除く
    #[test]
    fn history_exclusion_profile_is_specific() {
        let cfg = cfg();
        write_rules(
            &cfg,
            r#"[{"match":"browser-profile","browser":"chrome","profile":"Default"}]"#,
        );
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        with_other_profile(&reader, "Profile 2", vec![read_visit(2, "b")]);
        let (transport, _) = fetch_and_send(&cfg, &reader);
        assert_eq!(
            history_urls_of_kind(&transport, "visit"),
            vec!["https://example.test/2"]
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: 取得をやり直しても除外の件数は増えない
    #[test]
    fn history_exclusion_retry_counts_once() {
        let cfg = cfg();
        write_rules(&cfg, r#"[{"match":"process-name","value":"chrome.exe"}]"#);
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let (transport, reference) = (AcceptAll::default(), FixedReference(t(0)));
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        drop(rt);
        // 除外の記録を積んだ後・取得の成功を書く前に止まった（帳面は書く前のまま）。起動し直してやり直す
        std::fs::remove_dir_all(cfg.state_dir.join("browser-history")).unwrap();
        let mut again = history_runtime(&cfg, &transport, &reference, reader.clone());
        let before = reader.reads();
        for sec in 1000..3000 {
            again.tick_at(&mut src, t(sec), t(sec));
            if reader.reads() > before && !again.history.as_ref().unwrap().is_reading() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        again.send();
        assert!(reader.reads() > before, "やり直しの取得が走らなかった");
        let records = excluded_records(&transport);
        assert_eq!(records.len(), 2, "やり直しの取得で除外が積まれなかった");
        let mut ids: Vec<_> = records
            .iter()
            .map(|r| r["external_id"].as_str().unwrap().to_string())
            .collect();
        ids.dedup();
        assert_eq!(ids.len(), 1, "やり直しで別の識別子になった");
        assert!(records.iter().all(|r| r["payload"]["excluded_count"] == 1));
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// Scenario: 登録を後から足すと、既に送った訪問の変わった内容は送られない
    #[test]
    fn history_exclusion_added_later() {
        let cfg = cfg();
        let reader = FakeReader::new(vec![read_visit(1, "元の題名")]);
        let (first, _) = fetch_and_send(&cfg, &reader);
        assert_eq!(history_urls_of_kind(&first, "visit").len(), 1);

        write_rules(
            &cfg,
            r#"[{"match":"url-contains","value":"example.test/1"}]"#,
        );
        reader.visits.lock().unwrap()[0].title = Some("変わった題名".into());
        let second = fetch_twice_from_second(&cfg, &reader);
        let all = second
            .bodies
            .borrow()
            .iter()
            .map(|(_, b)| b.clone())
            .collect::<String>();
        assert!(!all.contains("変わった題名"), "登録の後の内容が送られた");
        assert!(history_urls_of_kind(&second, "visit").is_empty());
        // 既に格納された訪問は、除外されても消えた（vanished）ことにはならない
        assert!(
            history_records(&second)
                .iter()
                .all(|r| r["payload"]["kind"] != "vanished"),
            "除外した訪問が履歴から消えたものとして送られた"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 前回の成功を 1 日前に置いて、同じ状態の置き場でもう 1 回取得する。
    fn fetch_twice_from_second(cfg: &Config, reader: &Arc<FakeReader>) -> AcceptAll {
        write_last_success(cfg, t(0) - Duration::hours(24));
        fetch_and_send(cfg, reader).0
    }

    /// Scenario: 登録を外すと、まだ履歴にある除外済みの訪問が次の取得で送られる
    #[test]
    fn history_exclusion_removed_later() {
        let cfg = cfg();
        write_rules(
            &cfg,
            r#"[{"match":"url-contains","value":"example.test/1"}]"#,
        );
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let (first, _) = fetch_and_send(&cfg, &reader);
        assert!(history_urls_of_kind(&first, "visit").is_empty());

        write_rules(&cfg, "[]");
        let second = fetch_twice_from_second(&cfg, &reader);
        assert_eq!(
            history_urls_of_kind(&second, "visit"),
            vec!["https://example.test/1"]
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }
}
