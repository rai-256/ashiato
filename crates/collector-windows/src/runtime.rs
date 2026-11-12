// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
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

use crate::channel_notice::ChannelNotice;
use crate::clock::{ReferenceClock, SkewSchedule, Uptime};
use crate::clock_record::{skew_record, RecordContext};
use crate::clock_worker::{ClockReading, ClockWorker, WorkerPoll};
use crate::config::{Config, Zone};
use crate::contract::{ClockTrigger, HeartbeatRequest, IngestRequest, WindowPayload};
use crate::contract_gate::ContractGate;
use crate::contract_notice::ContractNotice;
use crate::destination_gate::{self, State as DestinationState};
use crate::engine::{Engine, EngineState, IdleRead, Observation, UrlRead};
use crate::heartbeat::{self, blocker, Capability, CounterStore, Counters};
use crate::history::collect::{HistoryCollector, HistoryReader, ProfileHealth};
use crate::history::contract::Visit;
use crate::marker::{self, Marker, TOUCH_INTERVAL_SEC};
use crate::outage_notice::OutagePoller;
use crate::outbox::Outbox;
use crate::sender::{read_contract_answer, ContractAnswer, Sender, Transport};
use crate::telemetry;
use crate::time_sync::TimeSyncSource;
use ashiato_build_stamp::Channel;

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
    destination_enabled: bool,
    destination_state: DestinationState,
    /// 印（`--config-from-env`）で起動した。送り先は承認したものとして門を開け、「届かない」は鳴らさない（D13。R11）
    config_from_env: bool,
    /// 「送り先が設定されていません」の知らせ（24 時間ごとに出し直し、設定したら消す。R7）
    setup_notice: SetupNotice,
    user_id: uuid::Uuid,
    device_id: String,
    zone: Zone,
    engine: Engine,
    events: Outbox<IngestRequest>,
    beats: Outbox<HeartbeatRequest>,
    ingest: Sender,
    beat_sender: Sender,
    /// 「S-01 の更新を待っている」状態（ST50 / design D9）
    contract_gate: ContractGate,
    transport: &'a dyn Transport,
    reference: Arc<dyn ReferenceClock>,
    time_sync: Arc<dyn TimeSyncSource>,
    /// 起動からの経過時間（記録に載せる。design D9）
    uptime: Arc<dyn Uptime>,
    /// いま読んでいる測定の契機（記録に載せる）
    skew_trigger: ClockTrigger,
    /// 基準を読んでいる作業スレッド。**走っている間は次の測定を始めない**（design D7）
    clock_worker: Option<ClockWorker>,
    /// 途切れの問い合わせ係。**時計の測定の契機で 1 回呼ぶ**（ST14 / design D11）。通知を出せない構成では持たない
    outage: Option<OutagePoller>,
    /// 「サーバを更新してください」の通知係（ST50 / design D5）。付けなければ通知は出ない
    contract_notice: Option<ContractNotice>,
    destination_toaster: Option<Arc<dyn destination_gate::DestinationToaster>>,
    /// 系統の違いの通知係（D12（仮））。契約の版の通知とは別の札を使う。
    channel_notice: Option<ChannelNotice>,
    update_checker: Option<crate::update_check::UpdateChecker>,
    update_periodic_at: Option<DateTime<Utc>>,
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
    exclusions_path: std::path::PathBuf,
    channel: Channel,
    /// 系統の違いのログ（`channel_mismatch`）を出した相手の系統。違う状態に入った契機で 1 行だけ出し、
    /// 同じ系統のサーバが答えたら戻す（送る契機のたびに出さない。final review R12）
    channel_mismatch_logged: Option<Channel>,
    /// 単調時計の起点（`tick` が使う）
    anchor: Option<(std::time::Instant, DateTime<Utc>)>,
    log: fn(String),
    /// 記録を積むときに書く、この組み立ての印（ST49 / design D13）。試験は差し替えて「更新後の組み立て」を作る
    build_stamp: String,
}

/// **合言葉も本文も出さない**（R11 / R35）。件数と状態の有無だけ。
impl std::fmt::Debug for Runtime<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("events", &self.events.len())
            .field("beats", &self.beats.len())
            .field("unsaved", &self.unsaved.len())
            .field("engine", &self.engine)
            .field("channel", &self.channel)
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
    ///
    /// `queued` が偽（読めたが送る準備を終えられなかった取得）なら、どのプロファイルも成功に数えず、
    /// 種別だけを満たされていないものに載せる（deep.md 第 5 回 Q10）。
    fn record(&mut self, profiles: &[ProfileHealth], queued: bool) {
        self.read_seen = true;
        // 積めなかった取得の空は「見つからない」と言えない（読みのスレッドが落ちた。K4）
        if profiles.is_empty() && queued {
            self.blockers
                .insert(heartbeat::history_blocker::NONE_FOUND.to_string());
        }
        if !queued {
            self.blockers
                .insert(heartbeat::history_blocker::NOT_QUEUED.to_string());
        }
        for p in profiles {
            self.counters.record(p.readable && queued);
            if !p.readable {
                self.blockers.insert(heartbeat::history_blocker::unreadable(
                    p.browser.name(),
                    &p.directory,
                ));
            }
        }
    }
}

/// 「送り先が設定されていません」の知らせの状態（プロセスの中だけ。起動したら 1 度出す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetupNotice {
    /// 設定済みで、知らせは出ていない
    Configured,
    /// 設定済みになったが、前に出した知らせが残っている（次の見回りで消す）
    ConfiguredWhileShown,
    /// 未設定。最後に出した時刻
    Unset(Option<DateTime<Utc>>),
}

impl SetupNotice {
    /// 読み直した設定で状態を進める。
    fn with_setup(self, needs_setup: bool) -> Self {
        match (needs_setup, self) {
            (true, Self::Unset(last)) => Self::Unset(last),
            (true, _) => Self::Unset(None),
            (false, Self::Unset(Some(_)) | Self::ConfiguredWhileShown) => {
                Self::ConfiguredWhileShown
            }
            (false, _) => Self::Configured,
        }
    }
}

/// 送ってよいか。印で渡した送り先は承認したものとして扱う（D13。review/code.md R11）。
fn destination_open(
    cfg: &Config,
    store: &destination_gate::Store,
    state: &DestinationState,
    pending: usize,
) -> bool {
    (cfg.config_from_env
        || !store.exists()
        || destination_gate::allows(state, &cfg.base_url, pending))
        && !cfg.base_url.is_empty()
        && !cfg.api_token.is_empty()
}

fn info(line: String) {
    // log-ok: 呼び出し元は telemetry::line の行だけを渡す（件数・所要時間・種別しか受け取らない組み立て）
    tracing::info!("{line}");
}

impl<'a> Runtime<'a> {
    /// `setup` の別プロセス保存を次の送信契機へ反映する。
    pub fn reload_config(&mut self, cfg: &Config) -> anyhow::Result<()> {
        let store = destination_gate::Store::new(&cfg.state_dir);
        self.destination_state = store.load()?;
        let pending = self.pending_count();
        self.destination_enabled = destination_open(cfg, &store, &self.destination_state, pending);
        self.config_from_env = cfg.config_from_env;
        self.setup_notice = self.setup_notice.with_setup(cfg.needs_setup());
        self.user_id = cfg.user_id;
        self.device_id = cfg.device_id.clone();
        Ok(())
    }
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
        channel: Channel,
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
        let destination_store = destination_gate::Store::new(&cfg.state_dir);
        let (destination_state, broken_destination) = destination_store.load_or_quarantine()?;
        if broken_destination {
            info(telemetry::line(
                "destination_state_quarantined",
                Some(1),
                None,
                None,
            ));
        }
        let pending = events.len() + beats.len() + destination_gate::pending_drops(&cfg.state_dir);
        let destination_enabled =
            destination_open(cfg, &destination_store, &destination_state, pending);
        Ok(Self {
            destination_enabled,
            destination_state,
            config_from_env: cfg.config_from_env,
            setup_notice: SetupNotice::Configured.with_setup(cfg.needs_setup()),
            user_id: cfg.user_id,
            device_id: cfg.device_id.clone(),
            zone,
            engine,
            events,
            beats,
            ingest: Sender::ingest(),
            beat_sender: Sender::heartbeat(),
            contract_gate: ContractGate::open(&cfg.state_dir, &mut info),
            transport,
            reference,
            time_sync,
            uptime,
            skew_trigger: ClockTrigger::Start,
            clock_worker: None,
            outage: None,
            contract_notice: None,
            destination_toaster: None,
            channel_notice: None,
            update_checker: None,
            update_periodic_at: Some(now),
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
            exclusions_path: cfg.exclusions_path.clone(),
            channel,
            channel_mismatch_logged: None,
            anchor: None,
            log: info,
            build_stamp: env!("ASHIATO_BUILD_STAMP").to_string(),
        })
    }

    /// 試験用: 積むときに書く印を差し替える（更新後の組み立てを作る）。
    #[cfg(test)]
    fn set_build_stamp(&mut self, stamp: &str) {
        self.build_stamp = stamp.to_string();
    }

    /// 途切れの問い合わせ係を付ける。**付けなければ問い合わせない**（通知を出せない構成）。
    pub fn set_outage_poller(&mut self, poller: OutagePoller) {
        self.outage = Some(poller);
    }

    /// 「サーバを更新してください」の通知係を付ける。**付けなければ通知は出ない**（収集は続く）。
    pub fn set_contract_notice(&mut self, notice: ContractNotice) {
        self.contract_notice = Some(notice);
    }

    pub fn set_destination_toaster(
        &mut self,
        toaster: Arc<dyn destination_gate::DestinationToaster>,
    ) {
        self.destination_toaster = Some(toaster);
    }

    /// 系統の違いの通知係を付ける。
    pub fn set_channel_notice(&mut self, notice: ChannelNotice) {
        self.channel_notice = Some(notice);
    }

    pub fn set_update_checker(&mut self, checker: crate::update_check::UpdateChecker) {
        self.update_checker = Some(checker);
    }

    /// ブラウザ履歴の取得を有効にする。**読み手を差し込めるようにしてある**（試験は偽の読み手を渡す）。
    #[must_use]
    pub fn with_history(mut self, reader: std::sync::Arc<dyn HistoryReader>) -> Self {
        let history = HistoryCollector::new(reader, &self.state_dir, &self.exclusions_path);
        // 前のプロセスが落ちて残した写しを、読みを始める前に消す（design D2。R60）。文言にはパスが入るので種別だけ
        if history.sweep_copies().is_err() {
            (self.log)(telemetry::history_line(
                "history_copy_sweep_failed",
                Some(1),
                None,
                Some("io"),
            ));
        }
        self.history = Some(history);
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
        if let Some(checker) = &self.update_checker {
            let checker = checker.clone();
            let log = self.log;
            std::thread::spawn(move || {
                if let Err(error) = checker.start(wall) {
                    log(telemetry::line(
                        "update_check_failed",
                        Some(1),
                        None,
                        Some(telemetry::error_kind(&error)),
                    ));
                }
            });
        }
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
        if self
            .update_periodic_at
            .is_none_or(|at| mono - at >= Duration::hours(1))
        {
            self.update_periodic_at = Some(mono);
            if let Some(checker) = &self.update_checker {
                let idle = match obs.idle {
                    IdleRead::Elapsed(value) => Some(value.num_seconds()),
                    IdleRead::Unavailable => None,
                };
                let checker = checker.clone();
                let log = self.log;
                std::thread::spawn(move || {
                    if let Err(error) = checker.periodic(wall, obs.locked, idle) {
                        log(telemetry::line(
                            "update_check_failed",
                            Some(1),
                            None,
                            Some(telemetry::error_kind(&error)),
                        ));
                    }
                });
            }
        }
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
        if self.destination_enabled {
            self.maybe_measure_skew(wall, mono, &*source);
        }
        self.maybe_beat(wall, mono);
        self.maybe_history(wall);
        self.maybe_history_beat(wall, mono);
        self.maybe_setup_notice(wall);
        if self.destination_enabled {
            self.maybe_send(wall, mono);
        } else {
            let mut log = self.log;
            self.emit_destination_notice(wall, &mut log);
        }
    }

    /// 未設定の間は「送り先が設定されていません」を 24 時間ごとに出し直し、設定したら消す（spec。review/code.md R7）。
    fn maybe_setup_notice(&mut self, now: DateTime<Utc>) {
        match self.setup_notice {
            SetupNotice::Configured => {}
            SetupNotice::ConfiguredWhileShown => {
                self.setup_notice = SetupNotice::Configured;
                if let Some(toaster) = &self.destination_toaster {
                    if toaster.dismiss_setup().is_err() {
                        (self.log)(telemetry::line(
                            "setup_notice_dismiss_failed",
                            None,
                            None,
                            Some("setup"),
                        ));
                    }
                }
            }
            SetupNotice::Unset(last) => {
                if last.is_some_and(|at| now - at < destination_gate::RENOTIFY_AFTER) {
                    return;
                }
                let shown = self
                    .destination_toaster
                    .as_ref()
                    .is_none_or(|toaster| toaster.show_setup().is_ok());
                if shown {
                    (self.log)(telemetry::line("setup_notice", None, None, Some("setup")));
                    self.setup_notice = SetupNotice::Unset(Some(now));
                }
            }
        }
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
        if self.destination_enabled {
            self.send();
        }
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
        let mut req = IngestRequest::of(p, self.user_id, &self.device_id, at, &self.zone)?;
        req.client_version = Some(self.build_stamp.clone());
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
            self.poll_outages();
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

    /// 時計の測定の契機で途切れを問い合わせる（測り直しは契機ではない）。待たない。
    fn poll_outages(&mut self) {
        if !self.destination_enabled {
            return;
        }
        if self.skew_trigger == ClockTrigger::Retry {
            return;
        }
        if let Some(poller) = self.outage.as_mut() {
            poller.poll(&self.device_id);
            #[cfg(test)]
            poller.wait();
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
            &self.build_stamp,
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
        let (user_id, device_id, zone, events, stamp) = (
            self.user_id,
            &self.device_id,
            &self.zone,
            &mut self.events,
            &self.build_stamp,
        );
        // まとめて積み、同期は 1 回（見回りを止めない。deep.md 第 5 回 Q9）。`source_updated_at` は読んだ時刻。
        // ただし前に送った版より前にしない（D15 / 第 6 回 Q11。決めるのは取得の側）
        let mut queue = |visits: &[Visit], version: DateTime<Utc>| {
            let reqs = visits
                .iter()
                .map(|v| {
                    let mut req = IngestRequest::of_visit(v, user_id, device_id, version, zone)?;
                    req.client_version = Some(stamp.clone());
                    Ok(req)
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            events.add_many(reqs)
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
        // 飛ばした行・退避した帳面・消せなかった写しは、件数だけ（R59 / R60 / R63）
        for (kind, count) in history.take_log_counts() {
            (self.log)(telemetry::history_line(kind, Some(count), None, None));
        }
    }

    /// 履歴のソースの生存信号。区間に読みが 1 回も無いときは、**別スレッドで開けるかを確かめてから**出す（design D12）。
    fn maybe_history_beat(&mut self, wall: DateTime<Utc>, mono: DateTime<Utc>) {
        let (Some(history), beat) = (self.history.as_mut(), &mut self.history_beat) else {
            return;
        };
        let mut counted = false;
        if let Some(report) = history.take_report() {
            beat.record(&report.profiles, report.queued);
            // 読めた訪問は送れているので成功の数えは変えず、種別だけを載せる（「取得できない」になる。D17（仮））
            if report.sent_version_unreadable {
                beat.blockers
                    .insert(heartbeat::history_blocker::SENT_VERSION_UNREADABLE.to_string());
            }
            counted = true;
        }
        if history.is_probing() {
            // 確かめの結果が出たら、それを区間の読みとして数えて出す
            let Some(profiles) = history.poll_probe() else {
                return self.save_history_counters_if(counted);
            };
            // 開けても、直近の取得が積めなかったなら取得できるにしない（起動直後の信号も。K1）
            beat.record(&profiles, history.last_fetch_queued());
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
            &self.build_stamp,
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
            self.send_at(wall);
            self.last_send = Some(mono);
        }
    }

    /// 直前の見回りの壁時計で送る（見回りの前なら今の時刻）。
    fn send(&mut self) {
        let now = self.last_tick.map_or_else(Utc::now, |(wall, _)| wall);
        self.send_at(now);
    }

    /// 送る契機の 1 回分。**頭で 1 回だけ `GET /contract` を引き**、記録と生存信号の両方に同じ答えを渡す
    /// （design D18（仮）。答えは覚えない）。
    fn send_at(&mut self, now: DateTime<Utc>) {
        let log = self.log;
        let mut log = |line: String| log(line);
        let pending = self.pending_count();
        if !self.destination_enabled {
            log(telemetry::line(
                "send_held",
                Some(pending),
                None,
                Some("destination"),
            ));
            return;
        }
        let contract_reply = self.transport.get("/contract");
        destination_gate::observe_response(
            &mut self.destination_state,
            contract_reply.as_ref().ok().map(|reply| reply.status),
            now,
        );
        self.save_destination_state();
        let channel = crate::sender::read_contract_channel(&contract_reply);
        let answer = read_contract_answer(&contract_reply);
        if let Some(server) = channel {
            if server != self.channel {
                if self.channel_mismatch_logged != Some(server) {
                    log(telemetry::line(
                        "channel_mismatch",
                        None,
                        None,
                        Some(&format!(
                            "server={} self={}",
                            server.as_str(),
                            self.channel.as_str()
                        )),
                    ));
                    self.channel_mismatch_logged = Some(server);
                }
                if let Some(notice) = &self.channel_notice {
                    notice.after_send(Some(server), self.channel, now, &mut log);
                }
                return;
            }
            self.channel_mismatch_logged = None;
        }
        if let Some(notice) = &self.channel_notice {
            notice.after_send(channel, self.channel, now, &mut log);
        }
        let accepted = match answer {
            ContractAnswer::Version(a) => Some(a),
            held => {
                // 送らずに残した契機を必ず 1 行残す（/code-review R22）。合言葉の誤り（401）も到達できないのも
                // ここでは一時的な失敗になり、送る口の `send_failed` が出なくなる。携帯の `send_held` と同じ形
                log(telemetry::line(
                    "send_held",
                    None,
                    None,
                    Some(contract_answer_kind(held)),
                ));
                None
            }
        };
        let sent = self
            .ingest
            .flush(&mut self.events, self.transport, accepted, &mut log);
        if let Err(e) = sent {
            log(telemetry::line(
                "send_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
        }
        let ingest_outcome = self.ingest.last_outcome();
        if ingest_outcome.posted {
            destination_gate::observe_response(
                &mut self.destination_state,
                ingest_outcome.response_status,
                now,
            );
        }
        let sent = self
            .beat_sender
            .flush(&mut self.beats, self.transport, accepted, &mut log);
        if let Err(e) = sent {
            log(telemetry::line(
                "send_failed",
                None,
                None,
                Some(telemetry::error_kind(&e)),
            ));
        }
        let beat_outcome = self.beat_sender.last_outcome();
        if beat_outcome.posted {
            destination_gate::observe_response(
                &mut self.destination_state,
                beat_outcome.response_status,
                now,
            );
        }
        self.save_destination_state();
        self.emit_destination_notice(now, &mut log);
        let outcomes = [self.ingest.last_outcome(), self.beat_sender.last_outcome()];
        if self.contract_gate.observe(answer, now, &outcomes, &mut log) {
            // 待っている状態を抜けた。断られた印のせいで先頭に居座らせない（design D14）
            self.ingest.clear_skipped();
            self.beat_sender.clear_skipped();
        }
        if let Some(notice) = &self.contract_notice {
            // 手元に残した最も古い行を**積んだ時刻**（design D5。置き場の控え）。控えに無い行（控えを書く前に
            // 積んだ・書けなかった）だけ出来事の時刻で代える —— ブラウザの履歴は古い訪問の時刻を持つので、
            // 出来事の時刻の最小で代えると日数が大きく出る（final review R10）
            // 通知を出す契機でだけ数える（未送信を全部なめるので。/code-review R25）
            let events = &self.events;
            let oldest = || {
                events
                    .snapshot()
                    .iter()
                    .filter_map(|r| {
                        events.enqueued_at(&r.id).or_else(|| {
                            DateTime::parse_from_rfc3339(&r.event_time)
                                .ok()
                                .map(|t| t.with_timezone(&Utc))
                        })
                    })
                    .min()
            };
            notice.after_send(&mut self.contract_gate, now, oldest, &mut log);
        }
    }

    fn save_destination_state(&mut self) {
        let store = destination_gate::Store::new(&self.state_dir);
        // `setup` は別プロセスで承認 URL を保存する。送信中にその保存が
        // 起きても、巡回開始時の古い state 全体で上書きしない。
        if let Ok(persisted) = store.load() {
            if persisted.setup_saved_at > self.destination_state.setup_saved_at {
                // 見回りの間に設定が保存し直された。保存は数えを消して起点を置き直すので、この契機の
                // 観測で書き戻さない（届かない・断られの数えを復活させない。review/code.md R10）
                self.destination_state = persisted;
            } else {
                // 承認と保留は `setup` の持ち物。見回りが握っている古い値で書き戻さない —— y の後に
                // 「確認を待っています」が出続ける（review/code.md R36）
                if persisted.approved_url.is_some() {
                    self.destination_state.approved_url = persisted.approved_url;
                }
                self.destination_state.destination_notified_at = persisted.destination_notified_at;
                // `setup` の確かめが書いた「届いた」を、見回りの古い値で消さない（review/code.md R44）
                self.destination_state.first_response_at = self
                    .destination_state
                    .first_response_at
                    .or(persisted.first_response_at);
                if persisted.destination_notified_at.is_none() {
                    self.destination_state.destination_notice_at = None;
                }
            }
        }
        // 環境変数起動や初回起動の一時的な失敗だけで「承認済み送り先なし」の
        // 門を作らない。設定副命令が作った state、または承認済み送り先がある
        // state だけを再起動越しに保持する。
        if self.destination_state.approved_url.is_none() && !store.exists() {
            return;
        }
        if let Err(error) = store.save(&self.destination_state) {
            (self.log)(telemetry::line(
                "destination_state_save_failed",
                None,
                None,
                Some(telemetry::error_kind(&error)),
            ));
        }
    }

    fn emit_destination_notice(&mut self, now: DateTime<Utc>, log: &mut dyn FnMut(String)) {
        if destination_gate::destination_notice(&self.destination_state, now) {
            let shown = self.destination_toaster.as_ref().is_none_or(|toaster| {
                toaster
                    .show_destination_hold(destination_gate::HOLD_TEXT)
                    .is_ok()
            });
            if shown {
                log(telemetry::line(
                    "destination_notice",
                    None,
                    None,
                    Some("destination"),
                ));
                self.destination_state.destination_notice_at = Some(now);
                self.save_destination_state();
            }
        }
        // 印で渡した送り先には「届かない」を当てない（spec。前の `setup` の状態が残っていても。R10）。
        // 門が閉じている間（保留・未設定）は送らず答えも受けないので、「届かない」を数えない（review/code.md R21・R33）
        let kind = destination_gate::notice_kind(&self.destination_state, now).filter(|kind| {
            !((self.config_from_env || !self.destination_enabled)
                && *kind == destination_gate::NoticeKind::Unreachable)
        });
        if let Some(kind) = kind {
            let kind_name = match kind {
                destination_gate::NoticeKind::Auth => "auth",
                destination_gate::NoticeKind::Unreachable => "unreachable",
            };
            let message = match kind {
                destination_gate::NoticeKind::Auth => destination_gate::AUTH_TEXT,
                destination_gate::NoticeKind::Unreachable => destination_gate::UNREACHABLE_TEXT,
            };
            let shown = self
                .destination_toaster
                .as_ref()
                .is_none_or(|toaster| toaster.show_destination(kind, message).is_ok());
            if shown {
                log(telemetry::line(
                    "destination_notice",
                    None,
                    None,
                    Some(kind_name),
                ));
                match kind {
                    destination_gate::NoticeKind::Auth => {
                        self.destination_state.auth_notified_at = Some(now)
                    }
                    destination_gate::NoticeKind::Unreachable => {
                        self.destination_state.unreachable_notified_at = Some(now)
                    }
                }
                self.save_destination_state();
            }
        }
    }

    /// 未送信の数。記録と生存信号は手元の待ち行列から数え、破棄の報告だけを置き場から数える
    /// （見回りのたびに未送信の置き場を全部読み直さない。review/code.md R38・R39）。
    fn pending_count(&self) -> usize {
        self.events.len()
            + self.beats.len()
            + self.unsaved.len()
            + destination_gate::pending_drops(&self.state_dir)
    }
}

/// 送らずに残した契機のログの `error`（携帯の `send_held` と同じ語）。
fn contract_answer_kind(answer: ContractAnswer) -> &'static str {
    match answer {
        ContractAnswer::Version(_) => "version",
        ContractAnswer::NoEndpoint => "noendpoint",
        ContractAnswer::Unreadable => "unreadable",
        ContractAnswer::Transient => "transient",
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

    thread_local! {
        static CAPTURED_LOGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn capture_log(line: String) {
        CAPTURED_LOGS.with(|logs| logs.borrow_mut().push(line));
    }

    fn take_captured_logs() -> Vec<String> {
        CAPTURED_LOGS.with(|logs| std::mem::take(&mut *logs.borrow_mut()))
    }

    fn t(sec: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-13T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::seconds(sec)
    }

    /// 設定と置き場の `TempDir`。**試験の最後まで持つ**（落ちても消える）。
    fn cfg() -> (Config, tempfile::TempDir) {
        let tmp = tempfile::Builder::new()
            .prefix("ashiato-rt-")
            .tempdir()
            .unwrap();
        let cfg = Config {
            base_url: "http://127.0.0.1:1".into(),
            api_token: "t".into(),
            user_id: uuid::Uuid::nil(),
            device_id: "dev-1".into(),
            state_dir: tmp.path().join("state"),
            settings_mtime: None,
            credential_mtime: None,
            config_from_env: false,
            exclusions_path: tmp.path().join("state/exclusions.json"),
            launch_state_dir: tmp.path().join("state"),
            launch_device_id: "dev-1".into(),
        };
        (cfg, tmp)
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
        fn get(&self, _path: &str) -> anyhow::Result<Reply> {
            Ok(Reply {
                status: 200,
                body: format!(
                    r#"{{"contract_version":{}}}"#,
                    crate::contract::CONTRACT_VERSION
                ),
                headers: Default::default(),
            })
        }

        fn post(&self, path: &str, body: &str, _version: u32) -> anyhow::Result<Reply> {
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
                headers: Default::default(),
            })
        }
    }

    #[derive(Debug, Default)]
    struct Unauthorized {
        attempts: RefCell<usize>,
    }

    impl Transport for Unauthorized {
        fn get(&self, _path: &str) -> anyhow::Result<Reply> {
            self.attempts.replace_with(|n| *n + 1);
            Ok(Reply {
                status: 401,
                body: String::new(),
                headers: Default::default(),
            })
        }

        fn post(&self, _path: &str, _body: &str, _version: u32) -> anyhow::Result<Reply> {
            self.attempts.replace_with(|n| *n + 1);
            Ok(Reply {
                status: 401,
                body: String::new(),
                headers: Default::default(),
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
        transport: &'a dyn Transport,
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
            Channel::Official,
        )
        .unwrap()
    }

    /// 壁時計と単調時計を揃えて `sec` 秒ぶん回す。
    fn run(rt: &mut Runtime, src: &mut FakeSource, from: i64, to: i64, step: usize) {
        for sec in (from..=to).step_by(step) {
            rt.tick_at(src, t(sec), t(sec));
        }
    }

    #[test]
    fn st57_runtime_runs_update_check_at_start_and_hourly_tick() {
        // Scenario: PC の収集アプリは起動の時と実行時の 1 時間ごとの仕事で更新を確かめる
        let (cfg, _tmp) = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let checker = crate::update_check::UpdateChecker::new(&cfg.state_dir);
        // 新規導入直後の「示す」門ではなく Runtime が起こす check を観測する。
        checker.set_enabled(true).unwrap();
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        rt.set_update_checker(checker);
        let mut source = FakeSource::new("editor");
        rt.start_at(&source, t(0), t(0));
        let records = cfg.state_dir.join("update-checks.jsonl");
        for _ in 0..100 {
            if records.exists() { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(records.exists(), "起動時の更新確認が走らない");
        // runtime の 1 時間の契機と、更新係自身の 24 時間の間引きを別々に満たす。
        rt.tick_at(&mut source, t(86_400), t(3600));
        for _ in 0..100 {
            if std::fs::read_to_string(&records).unwrap_or_default().lines().count() >= 2 { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(std::fs::read_to_string(records).unwrap().lines().count(), 2);
    }

    // Scenario: 設定が無くても起動して前景の窓を積む
    // Scenario: 設定の無い起動は窓で貼らせ、待つ間も収集する
    #[test]
    fn st56_runtime_collects_without_destination() {
        let (mut cfg, tmp) = cfg();
        cfg.base_url.clear();
        cfg.api_token.clear();
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut source = FakeSource::new("editor");

        rt.start(&source);
        rt.tick_at(&mut source, t(0), t(0));

        assert_eq!(
            crate::SETUP_PROMPT,
            "画面の設定の『収集アプリの設定』で出した文字列を貼ってください"
        );
        assert!(rt.pending().0 >= 1, "前景の窓が未送信に積まれていない");
        assert!(transport.bodies.borrow().is_empty(), "送り先なしで送信した");
        let pending_before_paste = rt.pending().0;

        let setup = "ashiato-collector:1:eyJ1cmwiOiJodHRwczovL3BjLmV4YW1wbGUudHMubmV0L2FwaSIsInRva2VuIjoidG9rZW4tZm9yLXRlc3QtMDEyMzQ1Njc4OSJ9";
        cfg.save_setup_string(setup).expect("貼った設定を保存");
        let loaded = Config::from_sources(
            &[],
            |key| {
                (key == "ASHIATO_STATE_DIR")
                    .then(|| tmp.path().join("state").to_string_lossy().into_owned())
            },
            None,
            ashiato_build_stamp::channel(env!("ASHIATO_VERSION")),
        )
        .expect("貼った設定を再読込");
        assert_eq!(loaded.base_url, "https://pc.example.ts.net/api");
        assert_eq!(loaded.api_token, "token-for-test-0123456789");
        assert!(
            rt.pending().0 >= pending_before_paste,
            "設定を待つ間の記録を保持する"
        );
    }

    /// 受けた知らせを覚える偽の差出人。
    #[derive(Debug, Default)]
    struct RecordingToaster {
        shown: std::sync::Mutex<Vec<String>>,
        setup_shown: std::sync::atomic::AtomicUsize,
        setup_dismissed: std::sync::atomic::AtomicUsize,
    }

    impl destination_gate::DestinationToaster for RecordingToaster {
        fn show_destination(
            &self,
            _kind: destination_gate::NoticeKind,
            message: &str,
        ) -> anyhow::Result<()> {
            self.shown.lock().unwrap().push(message.to_owned());
            Ok(())
        }
        fn show_destination_hold(&self, message: &str) -> anyhow::Result<()> {
            self.shown.lock().unwrap().push(message.to_owned());
            Ok(())
        }
        fn show_setup(&self) -> anyhow::Result<()> {
            self.setup_shown
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
        fn dismiss_setup(&self) -> anyhow::Result<()> {
            self.setup_dismissed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
    }

    /// どこにも届かない口。
    #[derive(Debug, Default)]
    struct Offline;

    impl Transport for Offline {
        fn get(&self, _path: &str) -> anyhow::Result<Reply> {
            anyhow::bail!("offline")
        }
        fn post(&self, _path: &str, _body: &str, _version: u32) -> anyhow::Result<Reply> {
            anyhow::bail!("offline")
        }
    }

    #[test]
    fn st56_runtime_setup_notice_repeats_daily_and_clears_after_setup() {
        // review/code.md R7: 未設定の知らせは 24 時間ごと、設定したら消す
        let (mut cfg, _tmp) = cfg();
        cfg.base_url.clear();
        cfg.api_token.clear();
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let toaster = Arc::new(RecordingToaster::default());
        rt.set_destination_toaster(toaster.clone());
        let mut source = FakeSource::new("editor");
        let shown = || {
            toaster
                .setup_shown
                .load(std::sync::atomic::Ordering::Relaxed)
        };
        let day = 24 * 60 * 60;
        rt.tick_at(&mut source, t(0), t(0));
        assert_eq!(shown(), 1);
        rt.tick_at(&mut source, t(day - 60), t(day - 60));
        assert_eq!(shown(), 1, "24 時間より前に出し直した");
        rt.tick_at(&mut source, t(day + 60), t(day + 60));
        assert_eq!(shown(), 2, "24 時間で出し直していない");

        cfg.base_url = "https://b.example".into();
        cfg.api_token = "token-for-test-0123456789".into();
        rt.reload_config(&cfg).unwrap();
        rt.tick_at(&mut source, t(day + 120), t(day + 120));
        assert_eq!(
            toaster
                .setup_dismissed
                .load(std::sync::atomic::Ordering::Relaxed),
            1,
            "設定した後も未設定の知らせが残る"
        );
        rt.tick_at(&mut source, t(3 * day), t(3 * day));
        assert_eq!(shown(), 2);
    }

    #[test]
    fn st56_runtime_config_from_env_is_not_held_by_previous_setup_state() {
        // review/code.md R11: 印で渡した送り先は承認したものとして扱う（前の setup の状態と未送信があっても）
        for marked in [false, true] {
            let (mut cfg, _tmp) = cfg();
            cfg.config_from_env = marked;
            std::fs::create_dir_all(&cfg.state_dir).unwrap();
            destination_gate::Store::new(&cfg.state_dir)
                .save(&destination_gate::State {
                    approved_url: Some("https://approved-by-setup.example".into()),
                    ..Default::default()
                })
                .unwrap();
            let transport = AcceptAll::default();
            let reference = FixedReference::new(t(0));
            let mut source = FakeSource::new("editor");
            // 前の起動（送り先なし）が積んだ未送信を残す
            {
                let mut unset = cfg.clone();
                unset.base_url.clear();
                let mut before = runtime(
                    &unset,
                    Engine::new(Exclusions::default()),
                    &transport,
                    &reference,
                );
                before.start_at(&source, t(0), t(0));
                before.tick_at(&mut source, t(0), t(0));
                before.stop_at(t(1));
            }
            assert!(destination_gate::pending_in(&cfg.state_dir) > 0);
            let mut rt = runtime(
                &cfg,
                Engine::new(Exclusions::default()),
                &transport,
                &reference,
            );
            rt.reload_config(&cfg).unwrap();
            rt.send();
            assert_eq!(
                !sent(&transport, "/ingest").is_empty(),
                marked,
                "印 {marked} で送った/送らなかったが逆"
            );
        }
    }

    #[test]
    fn st56_runtime_unreachable_notice_uses_spec_text_and_skips_marked_destination() {
        // review/code.md R10: 文は spec のとおり。起点は保存の時刻。印の送り先には鳴らさない
        for marked in [false, true] {
            let (mut cfg, _tmp) = cfg();
            cfg.config_from_env = marked;
            std::fs::create_dir_all(&cfg.state_dir).unwrap();
            destination_gate::Store::new(&cfg.state_dir)
                .save(&destination_gate::State {
                    approved_url: Some(cfg.base_url.clone()),
                    setup_saved_at: Some(t(0)),
                    ..Default::default()
                })
                .unwrap();
            let transport = Offline;
            let reference = FixedReference::new(t(0));
            let mut rt = runtime(
                &cfg,
                Engine::new(Exclusions::default()),
                &transport,
                &reference,
            );
            let toaster = Arc::new(RecordingToaster::default());
            rt.set_destination_toaster(toaster.clone());
            rt.send_at(t(24 * 60 * 60 - 60));
            assert!(toaster.shown.lock().unwrap().is_empty());
            rt.send_at(t(24 * 60 * 60 + 60));
            let shown = toaster.shown.lock().unwrap().clone();
            if marked {
                assert!(
                    shown.is_empty(),
                    "印の送り先で「届かない」が鳴った: {shown:?}"
                );
            } else {
                assert_eq!(shown, vec![destination_gate::UNREACHABLE_TEXT.to_owned()]);
            }
        }
    }

    #[test]
    fn st56_runtime_does_not_overwrite_a_setup_saved_during_send() {
        // 見回りの最中に setup が保存し直しても、古い数えで書き戻さない（review/code.md R10）
        let (cfg, _tmp) = cfg();
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        let store = destination_gate::Store::new(&cfg.state_dir);
        store
            .save(&destination_gate::State {
                approved_url: Some(cfg.base_url.clone()),
                ..Default::default()
            })
            .unwrap();
        let transport = Unauthorized::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        rt.send_at(t(0));
        assert!(store.load().unwrap().auth_since.is_some());
        // 別のプロセスの setup が保存し直した
        store
            .save(&destination_gate::State {
                approved_url: Some(cfg.base_url.clone()),
                setup_saved_at: Some(t(10)),
                ..Default::default()
            })
            .unwrap();
        rt.send_at(t(20));
        let state = store.load().unwrap();
        assert_eq!(state.setup_saved_at, Some(t(10)));
        assert_eq!(state.auth_since, None, "保存で消した数えを書き戻した");
    }

    // Scenario: 設定を保存すると、起動し直さずに送り始める
    #[test]
    fn st56_runtime_reload_uses_saved_destination_and_sends_pending_record() {
        let (mut cfg, _tmp) = cfg();
        cfg.base_url.clear();
        cfg.api_token.clear();
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut source = FakeSource::new("editor");
        rt.start_at(&source, t(0), t(0));
        rt.tick_at(&mut source, t(0), t(0));
        assert_eq!(rt.pending().0, 1, "送信前の記録が積まれていない");

        let destination = "https://b.example";
        let store = crate::destination_gate::Store::new(&cfg.state_dir);
        store
            .save(&crate::destination_gate::State {
                approved_url: Some(destination.into()),
                ..Default::default()
            })
            .unwrap();
        cfg.base_url = destination.into();
        cfg.api_token = "token".into();
        rt.reload_config(&cfg).unwrap();
        rt.send();

        assert_eq!(rt.pending().0, 0, "設定保存後の送信で記録が残っている");
        assert!(
            !sent(&transport, "/ingest").is_empty(),
            "実ランタイムが記録を送っていない"
        );
    }

    // Scenario: PC は 401 で断られた記録を取り除かない
    #[test]
    fn st56_runtime_unauthorized_keeps_pending_records() {
        let (cfg, _tmp) = cfg();
        let transport = Unauthorized::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let mut source = FakeSource::new("editor");
        rt.start_at(&source, t(0), t(0));
        rt.tick_at(&mut source, t(0), t(0));
        assert_eq!(rt.pending().0, 1, "送信前の記録が積まれていない");

        rt.send();
        assert_eq!(rt.pending().0, 1, "401 で未送信の記録を取り除いた");
        assert!(
            *transport.attempts.borrow() > 0,
            "実ランタイムの送信契機を通っていない"
        );
    }

    /// 承認した送り先 A・未送信あり・送り先 B の置き場（`setup` で B を貼り、y と答えていない状態）を作る。
    fn held_at_b(cfg: &mut Config) {
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut source = FakeSource::new("editor");
        {
            let mut unset = cfg.clone();
            unset.base_url.clear();
            let mut before = runtime(
                &unset,
                Engine::new(Exclusions::default()),
                &transport,
                &reference,
            );
            before.start_at(&source, t(0), t(0));
            before.tick_at(&mut source, t(0), t(0));
            before.stop_at(t(1));
        }
        assert!(destination_gate::pending_in(&cfg.state_dir) > 0);
        destination_gate::Store::new(&cfg.state_dir)
            .save(&destination_gate::State {
                approved_url: Some("https://a.example".into()),
                destination_notified_at: Some(t(0)),
                setup_saved_at: Some(t(0)),
                ..Default::default()
            })
            .unwrap();
        cfg.base_url = "https://b.example".into();
    }

    // Scenario: 未送信があるまま送り先を変えると、答えるまで送らない
    #[test]
    fn st56_runtime_held_destination_sends_nothing_and_asks_no_outages_until_yes() {
        // review/code.md R24: 門を runtime の配線ごと見る（記録・生存信号・時計の記録・途切れの問い合わせが 0 件、y の後は届く）
        let (mut cfg, _tmp) = cfg();
        held_at_b(&mut cfg);
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let (asked, _) = with_outage_poller(&mut rt, &cfg, true);
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(2), t(2));
        for sec in (2..=2 * 60 * 60).step_by(10) {
            rt.reload_config(&cfg).unwrap();
            rt.tick_at(&mut src, t(sec), t(sec));
        }
        assert!(
            transport.bodies.borrow().is_empty(),
            "保留の間に送った: {:?}",
            transport
                .bodies
                .borrow()
                .iter()
                .map(|(p, _)| p.clone())
                .collect::<Vec<_>>()
        );
        assert!(
            asked.0.lock().unwrap().is_empty(),
            "保留の間に途切れを問い合わせた"
        );

        // y と答えた（`setup` が承認を書いた）
        crate::setup::approve_destination(&cfg, "https://b.example").unwrap();
        for sec in (2 * 60 * 60..=4 * 60 * 60).step_by(10) {
            rt.reload_config(&cfg).unwrap();
            rt.tick_at(&mut src, t(sec), t(sec));
        }
        assert!(
            !sent(&transport, "/ingest").is_empty(),
            "y の後も記録を送らない"
        );
        assert!(
            !sent(&transport, "/heartbeat").is_empty(),
            "y の後も生存信号を送らない"
        );
        assert!(
            !asked.0.lock().unwrap().is_empty(),
            "y の後も途切れを問い合わせない"
        );
    }

    #[test]
    fn st56_runtime_held_destination_does_not_report_unreachable() {
        // review/code.md R21・R33: 保留の間は送らず答えも受けない。保存から 24 時間たっても「届かない」を出さない
        let (mut cfg, _tmp) = cfg();
        held_at_b(&mut cfg);
        let transport = Offline;
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let toaster = Arc::new(RecordingToaster::default());
        rt.set_destination_toaster(toaster.clone());
        let mut src = FakeSource::new("editor");
        for sec in (0..=26 * 60 * 60).step_by(600) {
            rt.reload_config(&cfg).unwrap();
            rt.tick_at(&mut src, t(sec), t(sec));
        }
        let shown = toaster.shown.lock().unwrap().clone();
        assert!(
            !shown
                .iter()
                .any(|m| m == destination_gate::UNREACHABLE_TEXT),
            "保留の間に「届かない」を出した: {shown:?}"
        );
        assert!(shown.iter().any(|m| m == destination_gate::HOLD_TEXT));
    }

    #[test]
    fn st56_runtime_does_not_write_back_a_hold_answered_in_setup() {
        // review/code.md R36: setup で y と答えた後、見回りが古い保留を書き戻さない
        let (mut cfg, _tmp) = cfg();
        held_at_b(&mut cfg);
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let toaster = Arc::new(RecordingToaster::default());
        rt.set_destination_toaster(toaster.clone());
        let mut src = FakeSource::new("editor");
        rt.reload_config(&cfg).unwrap();
        rt.tick_at(&mut src, t(10), t(10));
        assert_eq!(
            toaster.shown.lock().unwrap().len(),
            1,
            "保留を知らせていない"
        );
        // 見回りが読み直す前に y が書かれ、24 時間後の見回りが知らせを出し直して書く
        crate::setup::approve_destination(&cfg, "https://b.example").unwrap();
        rt.tick_at(&mut src, t(24 * 60 * 60 + 20), t(24 * 60 * 60 + 20));
        let state = destination_gate::Store::new(&cfg.state_dir).load().unwrap();
        assert_eq!(
            state.destination_notified_at, None,
            "y で消した保留を書き戻した"
        );
        assert_eq!(state.approved_url.as_deref(), Some("https://b.example"));
        rt.reload_config(&cfg).unwrap();
        let before = toaster.shown.lock().unwrap().len();
        rt.tick_at(&mut src, t(3 * 24 * 60 * 60), t(3 * 24 * 60 * 60));
        let shown = toaster.shown.lock().unwrap().clone();
        assert!(
            !shown[before..]
                .iter()
                .any(|m| m == destination_gate::HOLD_TEXT),
            "承認の後も「確認を待っています」を出した: {shown:?}"
        );
    }

    #[test]
    fn st56_runtime_starts_with_a_broken_setup_state_and_holds() {
        // review/code.md R26: 壊れた setup-state.json で起動を止めず、退避して閉じた門で進み、setup で直せる
        let (mut cfg, _tmp) = cfg();
        held_at_b(&mut cfg);
        std::fs::write(cfg.state_dir.join("setup-state.json"), "{").unwrap();
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(reference.0)),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            Channel::Official,
        )
        .expect("壊れた setup-state.json で起動が止まった");
        assert!(cfg.state_dir.join("setup-state.broken.json").exists());
        let toaster = Arc::new(RecordingToaster::default());
        rt.set_destination_toaster(toaster.clone());
        let mut src = FakeSource::new("editor");
        rt.reload_config(&cfg).unwrap();
        rt.tick_at(&mut src, t(10), t(10));
        assert_eq!(
            toaster.shown.lock().unwrap().clone(),
            vec![destination_gate::HOLD_TEXT.to_owned()],
            "退避した後、黙って止まった（R43）"
        );
        rt.reload_config(&cfg).unwrap();
        rt.send();
        assert!(
            transport.bodies.borrow().is_empty(),
            "承認の無いまま未送信を送った"
        );
        cfg.save_setup_string(
            "ashiato-collector:1:eyJ1cmwiOiJodHRwczovL3BjLmV4YW1wbGUudHMubmV0L2FwaSIsInRva2VuIjoidG9rZW4tZm9yLXRlc3QtMDEyMzQ1Njc4OSJ9",
        )
        .expect("setup で保存し直せない");
    }

    #[test]
    fn st56_runtime_save_preserves_approval_written_by_setup() {
        let (cfg, _tmp) = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let destination = "https://approved-by-setup.example";
        crate::destination_gate::Store::new(&cfg.state_dir)
            .save(&crate::destination_gate::State {
                approved_url: Some(destination.into()),
                ..Default::default()
            })
            .unwrap();

        rt.send();

        let state = crate::destination_gate::Store::new(&cfg.state_dir)
            .load()
            .unwrap();
        assert_eq!(state.approved_url.as_deref(), Some(destination));
    }

    #[derive(Debug)]
    struct ChannelTransport {
        contract: std::sync::Mutex<Reply>,
        posts: std::sync::atomic::AtomicUsize,
        /// 真の間、送る要求を 426 で断る（契約の版の待ちを続けさせる）
        refuse_posts: std::sync::atomic::AtomicBool,
    }

    impl ChannelTransport {
        fn new(contract: Reply) -> Self {
            Self {
                contract: std::sync::Mutex::new(contract),
                posts: std::sync::atomic::AtomicUsize::new(0),
                refuse_posts: std::sync::atomic::AtomicBool::new(false),
            }
        }

        fn set_contract(&self, reply: Reply) {
            *self.contract.lock().unwrap() = reply;
        }
    }

    impl Transport for ChannelTransport {
        fn get(&self, path: &str) -> anyhow::Result<Reply> {
            if path == "/contract" {
                Ok(self.contract.lock().unwrap().clone())
            } else {
                Ok(Reply {
                    status: 200,
                    body: r#"{"status":"ready","outages":[]}"#.into(),
                    headers: Default::default(),
                })
            }
        }

        fn post(&self, _: &str, body: &str, _: u32) -> anyhow::Result<Reply> {
            self.posts
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if self.refuse_posts.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(Reply {
                    status: 426,
                    body: String::new(),
                    headers: Default::default(),
                });
            }
            let n = serde_json::from_str::<Vec<serde_json::Value>>(body).map_or(0, |v| v.len());
            Ok(Reply {
                status: 200,
                body: serde_json::to_string(
                    &(0..n)
                        .map(|_| serde_json::json!({"accepted":true,"error":null}))
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
                headers: Default::default(),
            })
        }
    }

    #[derive(Debug, Default)]
    struct ChannelToasts {
        channel: std::sync::Mutex<Vec<String>>,
        channel_dismissed: std::sync::atomic::AtomicUsize,
        contract: std::sync::Mutex<Vec<String>>,
        contract_dismissed: std::sync::atomic::AtomicUsize,
    }

    impl crate::channel_notice::ChannelToaster for ChannelToasts {
        fn show_channel(&self, xml: &str) -> anyhow::Result<()> {
            self.channel.lock().unwrap().push(xml.into());
            Ok(())
        }
        fn dismiss_channel(&self) -> anyhow::Result<()> {
            self.channel_dismissed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
    }
    impl crate::contract_notice::ContractToaster for ChannelToasts {
        fn show_contract(&self, xml: &str) -> anyhow::Result<()> {
            self.contract.lock().unwrap().push(xml.into());
            Ok(())
        }
        fn dismiss_contract(&self) -> anyhow::Result<()> {
            self.contract_dismissed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
    }

    fn contract_reply(status: u16, channel: Option<&str>, readable: bool) -> Reply {
        let mut headers = std::collections::BTreeMap::new();
        if let Some(channel) = channel {
            headers.insert("Ashiato-Channel".into(), channel.into());
        }
        Reply {
            status,
            body: if readable {
                r#"{"contract_version":1}"#.into()
            } else {
                "<html>入口の画面</html>".into()
            },
            headers,
        }
    }

    fn exercise_channel(
        app: Channel,
        reply: Reply,
        with_outages: bool,
    ) -> (usize, usize, usize, usize, usize, usize, usize, Vec<String>) {
        let (cfg, _tmp) = cfg();
        let _ = take_captured_logs();
        let transport = ChannelTransport::new(reply);
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(t(0))),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            app,
        )
        .unwrap();
        rt.log = capture_log;
        let toasts = Arc::new(ChannelToasts::default());
        rt.set_channel_notice(crate::channel_notice::ChannelNotice::new(
            toasts.clone(),
            &cfg.state_dir,
        ));
        rt.set_contract_notice(ContractNotice::new(toasts.clone()));
        let query = with_outages.then(|| with_outage_poller(&mut rt, &cfg, true).0);
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 700, 10);
        rt.send_at(t(710));
        let channel_shown = toasts.channel.lock().unwrap().len();
        let contract_shown = toasts.contract.lock().unwrap().len();
        let outage_calls = query.map_or(0, |q| q.0.lock().unwrap().len());
        (
            transport.posts.load(std::sync::atomic::Ordering::Relaxed),
            rt.events.len(),
            rt.beats.len(),
            channel_shown,
            toasts
                .channel_dismissed
                .load(std::sync::atomic::Ordering::Relaxed),
            contract_shown,
            outage_calls,
            take_captured_logs(),
        )
    }

    fn exercise_daily_notice() -> usize {
        let (cfg, _tmp) = cfg();
        let transport = ChannelTransport::new(contract_reply(200, Some("official"), true));
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(t(0))),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            Channel::Dev,
        )
        .unwrap();
        let toasts = Arc::new(ChannelToasts::default());
        rt.set_channel_notice(crate::channel_notice::ChannelNotice::new(
            toasts.clone(),
            &cfg.state_dir,
        ));
        let mut src = FakeSource::new("editor");
        run(&mut rt, &mut src, 0, 700, 10);
        rt.send_at(t(0));
        rt.send_at(t(23 * 3600));
        rt.send_at(t(24 * 3600));
        let shown = toasts.channel.lock().unwrap().len();
        shown
    }

    fn exercise_recovery() -> (usize, usize) {
        let (cfg, _tmp) = cfg();
        let transport = ChannelTransport::new(contract_reply(200, Some("official"), true));
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(t(0))),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            Channel::Dev,
        )
        .unwrap();
        let toasts = Arc::new(ChannelToasts::default());
        rt.set_channel_notice(crate::channel_notice::ChannelNotice::new(
            toasts.clone(),
            &cfg.state_dir,
        ));
        let mut src = FakeSource::new("editor");
        run(&mut rt, &mut src, 0, 700, 10);
        rt.send_at(t(0));
        transport.set_contract(contract_reply(200, Some("dev"), true));
        rt.send_at(t(1));
        (
            transport.posts.load(std::sync::atomic::Ordering::Relaxed),
            toasts
                .channel_dismissed
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    // Scenario: 開発版の PC の収集アプリは、正式版のサーバに何も送らず手元に残す
    #[test]
    fn st54_channel_runtime_dev_holds_official() {
        let r = exercise_channel(
            Channel::Dev,
            contract_reply(200, Some("official"), true),
            false,
        );
        assert_eq!(r.0, 0);
        assert!(r.1 > 0 && r.2 > 0);
    }
    // Scenario: 正式版の PC の収集アプリは、開発版のサーバに何も送らず手元に残す
    #[test]
    fn st54_channel_runtime_official_holds_dev() {
        let r = exercise_channel(
            Channel::Official,
            contract_reply(200, Some("dev"), true),
            false,
        );
        assert_eq!(r.0, 0);
        assert!(r.1 > 0 && r.2 > 0);
    }
    // Scenario: 開発版の PC の収集アプリは、開発版のサーバには今どおり送る
    #[test]
    fn st54_channel_runtime_dev_sends_dev() {
        let r = exercise_channel(Channel::Dev, contract_reply(200, Some("dev"), true), false);
        assert!(r.0 > 0);
        assert_eq!(r.3, 0);
    }
    // Scenario: 系統を答えない古いサーバは、正式版のサーバとみなす
    #[test]
    fn st54_channel_runtime_old_server_is_official() {
        let r = exercise_channel(Channel::Official, contract_reply(200, None, true), false);
        assert!(r.0 > 0);
    }
    // Scenario: 系統が違って送らない間は、サーバを更新してくださいの通知を出さない
    #[test]
    fn st54_channel_runtime_mismatch_does_not_show_contract() {
        let r = exercise_channel(
            Channel::Dev,
            contract_reply(200, Some("official"), true),
            false,
        );
        assert_eq!(r.5, 0);
    }
    // Scenario: 系統が違って送らない間、PC の収集アプリは送り先を確かめるよう知らせる
    #[test]
    fn st54_channel_runtime_mismatch_shows_channel() {
        let r = exercise_channel(
            Channel::Dev,
            contract_reply(200, Some("official"), true),
            false,
        );
        assert_eq!(r.3, 1);
        assert!(r.7.iter().any(|line| {
            line.contains("kind=channel_mismatch")
                && line.contains("error=server=official self=dev")
        }));
    }
    // Scenario: 系統の違いの知らせは、続く間 24 時間ごとに出し直す
    #[test]
    fn st54_channel_runtime_notice_is_daily() {
        assert_eq!(exercise_daily_notice(), 2);
    }
    // Scenario: 送り先を直すと、PC の収集アプリは手元に残した記録を送り、系統の違いの知らせを消す
    #[test]
    fn st54_channel_runtime_recovery_sends_and_dismisses() {
        let (posts, dismissed) = exercise_recovery();
        assert!(posts > 0 && dismissed == 1);
    }
    // Scenario: 契約の問い合わせが 404 なら、系統の違いの知らせでなく更新を促す
    #[test]
    fn st54_channel_runtime_404_is_contract_notice() {
        let r = exercise_channel(Channel::Dev, contract_reply(404, None, true), false);
        assert_eq!(r.3, 0);
        assert!(r.5 > 0);
    }
    // Scenario: 契約の問い合わせが読めない 200 なら、系統の違いの知らせを出さない
    #[test]
    fn st54_channel_runtime_unreadable_is_not_channel() {
        let r = exercise_channel(
            Channel::Dev,
            contract_reply(200, Some("official"), false),
            false,
        );
        assert_eq!(r.3, 0);
    }
    // Scenario: 契約の問い合わせが 401 なら、系統の違いの知らせを出さない
    #[test]
    fn st54_channel_runtime_401_is_not_channel() {
        let r = exercise_channel(
            Channel::Dev,
            contract_reply(401, Some("official"), true),
            false,
        );
        assert_eq!(r.3, 0);
    }
    // Scenario: 系統の違いの知らせの時刻は、収集アプリを起こし直しても引き継ぐ
    #[test]
    fn st54_channel_runtime_notice_state_is_persistent() {
        let (cfg, _tmp) = cfg();
        let transport = ChannelTransport::new(contract_reply(200, Some("official"), true));
        let toasts = Arc::new(ChannelToasts::default());
        {
            let mut rt = Runtime::new(
                &cfg,
                zone(),
                Engine::new(Exclusions::default()),
                &transport,
                ClockInputs {
                    reference: Arc::new(FixedReference::new(t(0))),
                    time_sync: Arc::new(NoTimeSync),
                    uptime: Arc::new(FixedUptime),
                },
                t(0),
                Channel::Dev,
            )
            .unwrap();
            rt.set_channel_notice(crate::channel_notice::ChannelNotice::new(
                toasts.clone(),
                &cfg.state_dir,
            ));
            rt.send_at(t(0));
        }
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(t(0))),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            Channel::Dev,
        )
        .unwrap();
        rt.set_channel_notice(crate::channel_notice::ChannelNotice::new(
            toasts.clone(),
            &cfg.state_dir,
        ));
        rt.send_at(t(23 * 3600));
        assert_eq!(toasts.channel.lock().unwrap().len(), 1);
    }
    // Scenario: 系統の違いの知らせを消しても、サーバを更新してくださいの知らせは消えない
    #[test]
    fn st54_channel_runtime_notices_are_independent() {
        let (cfg, _tmp) = cfg();
        let transport = ChannelTransport::new(contract_reply(200, Some("dev"), true));
        transport
            .refuse_posts
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(t(0))),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            Channel::Dev,
        )
        .unwrap();
        let toasts = Arc::new(ChannelToasts::default());
        rt.set_channel_notice(crate::channel_notice::ChannelNotice::new(
            toasts.clone(),
            &cfg.state_dir,
        ));
        rt.set_contract_notice(ContractNotice::new(toasts.clone()));
        let mut src = FakeSource::new("editor");
        run(&mut rt, &mut src, 0, 700, 10);
        // 同じ系統のサーバが 426 で断る: 札 contract の「サーバを更新してください」が出る
        rt.send_at(t(710));
        assert!(!toasts.contract.lock().unwrap().is_empty());
        // 系統の違うサーバが答える: 札 channel の知らせも出る（両方が出ている）
        transport.set_contract(contract_reply(200, Some("official"), true));
        rt.send_at(t(720));
        assert_eq!(toasts.channel.lock().unwrap().len(), 1);
        // 系統の違いだけが抜ける（同じ系統のサーバが答え、426 の待ちは続く）
        transport.set_contract(contract_reply(200, Some("dev"), true));
        rt.send_at(t(730));
        let dismissed =
            |n: &std::sync::atomic::AtomicUsize| n.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            dismissed(&toasts.channel_dismissed),
            1,
            "札 channel が消えていない"
        );
        assert_eq!(
            dismissed(&toasts.contract_dismissed),
            0,
            "札 contract まで消えた"
        );
    }

    /// 系統の違いのログは違う状態に入った契機で 1 行だけ出し、送る契機のたびに出さない（final review R12）。
    #[test]
    fn st54_channel_mismatch_log_is_not_repeated() {
        let (cfg, _tmp) = cfg();
        let _ = take_captured_logs();
        let transport = ChannelTransport::new(contract_reply(200, Some("official"), true));
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(t(0))),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            Channel::Dev,
        )
        .unwrap();
        rt.log = capture_log;
        let mismatch_lines = || {
            take_captured_logs()
                .iter()
                .filter(|l| l.contains("kind=channel_mismatch"))
                .count()
        };
        rt.send_at(t(0));
        rt.send_at(t(60));
        rt.send_at(t(120));
        assert_eq!(mismatch_lines(), 1);
        transport.set_contract(contract_reply(200, Some("dev"), true));
        rt.send_at(t(180));
        transport.set_contract(contract_reply(200, Some("official"), true));
        rt.send_at(t(240));
        assert_eq!(mismatch_lines(), 1, "直した後にまた違えば、もう 1 行出す");
    }
    // Scenario: 系統の違うサーバからも、PC の収集アプリは途絶の判定を今どおり引く
    #[test]
    fn st54_channel_runtime_mismatch_still_polls_outages() {
        let r = exercise_channel(
            Channel::Dev,
            contract_reply(200, Some("official"), true),
            true,
        );
        assert_eq!(r.6, 1);
    }

    /// **起動時に「PC が止まっていた」が 1 件積まれ、OS の起動時刻が載る**（FR-82 / D23）。
    #[test]
    fn start_records_powered_off_span() {
        let (cfg, _tmp) = cfg();
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
    }

    /// **自分で止まった回の次は `clean_stop` が付く**（R16 / D23）。
    #[test]
    fn clean_stop_is_carried_to_next_start() {
        let (cfg, _tmp) = cfg();
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
    }

    /// **見回りの時刻が飛んだら「眠っていた」として残し、単調時計の進みも載せる**（D19）。
    #[test]
    fn suspend_gap_is_recorded() {
        let (cfg, _tmp) = cfg();
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
    }

    /// **1 日回すと、生存信号 4 件・時計のずれ 24 件**（R19。Runtime の層で契機を守る）。
    ///
    /// Scenario: 想定間隔ごとに生存信号が届く
    /// Scenario: 1 時間ごとにずれの測定記録が残る
    #[test]
    fn ticks_keep_heartbeat_and_skew_intervals_for_a_day() {
        let (cfg, _tmp) = cfg();
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
    }

    /// 偽の途切れの問い合わせ。呼ばれた端末識別子を覚え、いつも位置の途切れを返す。
    #[derive(Debug, Default)]
    struct AskedOutages(std::sync::Mutex<Vec<String>>);

    impl crate::outage_notice::OutageQuery for AskedOutages {
        fn fetch(&self, device_id: &str) -> crate::outage_notice::OutageResult {
            self.0.lock().unwrap().push(device_id.into());
            crate::outage_notice::OutageResult::Ready(vec![crate::outage_notice::Outage {
                episode_id: 1,
                logical_source: "c01-location".into(),
                display_name: "位置".into(),
                last_alive_at: t(0),
            }])
        }
    }

    /// 偽の通知の口。`ok = false` は出せない。
    #[derive(Debug)]
    struct CountingToaster {
        ok: bool,
        shown: std::sync::atomic::AtomicUsize,
    }

    impl crate::outage_notice::Toaster for CountingToaster {
        fn show(&self, _xml: &str, _sound: bool) -> anyhow::Result<()> {
            self.shown
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if self.ok {
                Ok(())
            } else {
                anyhow::bail!("unreachable: 通知を出せない")
            }
        }
        fn dismiss(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn with_outage_poller(
        rt: &mut Runtime,
        cfg: &Config,
        ok: bool,
    ) -> (Arc<AskedOutages>, Arc<CountingToaster>) {
        let query = Arc::new(AskedOutages::default());
        let toaster = Arc::new(CountingToaster {
            ok,
            shown: std::sync::atomic::AtomicUsize::new(0),
        });
        rt.set_outage_poller(OutagePoller::new(
            query.clone(),
            toaster.clone(),
            crate::outage_notice::NoticeStore::new(&cfg.state_dir),
            |_| {},
        ));
        (query, toaster)
    }

    /// Scenario: PC は時計の測定の契機で問い合わせる
    #[test]
    fn outage_notice_poll_runs_once_per_clock_trigger_with_the_device_id() {
        let (cfg, _tmp) = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let (query, _) = with_outage_poller(&mut rt, &cfg, true);
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 86_399, 10);
        let asked = query.0.lock().unwrap().clone();
        assert_eq!(
            of_kind(&transport, "clock-skew").len(),
            24,
            "測定の契機は 1 時間ごと"
        );
        assert_eq!(asked, vec!["dev-1"; 24], "契機ごとに 1 回・端末識別子つき");
    }

    /// 測り直し（60 秒後）は契機ではない。
    #[test]
    fn outage_notice_poll_is_not_repeated_by_clock_retries() {
        let (cfg, _tmp) = cfg();
        let transport = AcceptAll::default();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut rt = runtime_with(
            &cfg,
            &transport,
            Arc::new(BrokenReference {
                panics: false,
                calls,
            }),
        );
        let (query, _) = with_outage_poller(&mut rt, &cfg, true);
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 3_000, 10);
        assert_eq!(
            query.0.lock().unwrap().len(),
            1,
            "測り直しでは問い合わせない"
        );
    }

    /// Scenario: 契約の版で止まると PC に通知が 1 回出る
    /// 送る契機の後に通知係が呼ばれ、待っている間は 2 回目の契機でも出し直さない。
    #[test]
    fn st50_runtime_notifies_once_when_waiting_for_server_update() {
        #[derive(Debug, Default)]
        struct Toast(std::sync::Mutex<Vec<String>>);
        impl crate::contract_notice::ContractToaster for Toast {
            fn show_contract(&self, xml: &str) -> anyhow::Result<()> {
                self.0.lock().unwrap().push(xml.to_string());
                Ok(())
            }
            fn dismiss_contract(&self) -> anyhow::Result<()> {
                Ok(())
            }
        }
        // 版 1 までしか受け付けない。送りは通らず、行は残る
        #[derive(Debug)]
        struct Old;
        impl Transport for Old {
            fn get(&self, _: &str) -> anyhow::Result<Reply> {
                Ok(Reply {
                    status: 200,
                    body: r#"{"contract_version":1}"#.to_string(),
                    headers: Default::default(),
                })
            }
            fn post(&self, _: &str, _: &str, _: u32) -> anyhow::Result<Reply> {
                anyhow::bail!("止まっている")
            }
        }
        let (cfg, _tmp) = cfg();
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &Old,
            ClockInputs {
                reference: Arc::new(FixedReference::new(t(0))),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            Channel::Official,
        )
        .unwrap();
        let toast = Arc::new(Toast::default());
        rt.set_contract_notice(ContractNotice::new(toast.clone()));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 400, 10);
        assert!(!rt.events.is_empty(), "送れない行が残っている");
        let ahead = rt.events.snapshot()[0].clone();
        rt.events.add_many_at(vec![ahead], 2).unwrap();
        rt.send_at(t(10));
        rt.send_at(t(20));
        let shown = toast.0.lock().unwrap().clone();
        assert_eq!(shown.len(), 1, "待っている間は 1 回だけ出す");
        assert!(shown[0].contains("サーバを更新してください"));
    }

    /// 送る契機の頭の答えが口の無い 404・読めない 200 なら、記録も生存信号も 1 件も送らず残す（code-verify R17）。
    /// 送り手に答えを手で渡す試験だけでは、契機が答えを送り手へ渡すこと（`send_at`）を見ていなかった
    /// Scenario: 問い合わせの口が無い S-01 には PC も何も送らない
    #[test]
    fn st50_runtime_send_holds_everything_without_a_readable_answer() {
        #[derive(Debug)]
        struct Answers {
            reply: Reply,
            posts: std::sync::Mutex<usize>,
        }
        impl Transport for Answers {
            fn get(&self, _: &str) -> anyhow::Result<Reply> {
                Ok(self.reply.clone())
            }
            fn post(&self, _: &str, _: &str, _: u32) -> anyhow::Result<Reply> {
                *self.posts.lock().unwrap() += 1;
                anyhow::bail!("送ってはいけない")
            }
        }
        for reply in [
            Reply {
                status: 404,
                body: String::new(),
                headers: Default::default(),
            },
            Reply {
                status: 200,
                body: "<html>入口の画面</html>".to_string(),
                headers: Default::default(),
            },
        ] {
            let transport = Answers {
                reply: reply.clone(),
                posts: std::sync::Mutex::new(0),
            };
            let (cfg, _tmp) = cfg();
            let mut rt = Runtime::new(
                &cfg,
                zone(),
                Engine::new(Exclusions::default()),
                &transport,
                ClockInputs {
                    reference: Arc::new(FixedReference::new(t(0))),
                    time_sync: Arc::new(NoTimeSync),
                    uptime: Arc::new(FixedUptime),
                },
                t(0),
                Channel::Official,
            )
            .unwrap();
            let mut src = FakeSource::new("editor");
            rt.start_at(&src, t(0), t(0));
            run(&mut rt, &mut src, 0, 700, 10);
            rt.send_at(t(710));
            assert!(
                !rt.events.is_empty() && !rt.beats.is_empty(),
                "{}: 記録と生存信号が積まれていない",
                reply.status
            );
            assert_eq!(
                *transport.posts.lock().unwrap(),
                0,
                "{} {:?} の答えで送った",
                reply.status,
                reply.body
            );
        }
    }

    /// Scenario: 通知を出せなくても収集は止まらない
    #[test]
    fn outage_notice_poll_with_a_failing_toaster_keeps_collecting() {
        let (cfg, _tmp) = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let mut rt = runtime(
            &cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        let (query, toaster) = with_outage_poller(&mut rt, &cfg, false);
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 86_399, 10);
        assert_eq!(
            query.0.lock().unwrap().len(),
            24,
            "問い合わせは契機ごとに続く"
        );
        assert_eq!(
            toaster.shown.load(std::sync::atomic::Ordering::Relaxed),
            24,
            "出せないので覚えが進まず、契機ごとに出し直そうとする"
        );
        assert_eq!(
            sent(&transport, "/heartbeat").len(),
            4,
            "生存信号の送信は続く"
        );
        assert_eq!(of_kind(&transport, "clock-skew").len(), 24);
        assert_eq!(rt.pending(), (0, 0), "記録の送信は続く");
    }

    /// **動作中は 1 分ごとに印が進む**（R8 / tasks 5.1）。
    #[test]
    fn marker_advances_every_minute_while_running() {
        let (cfg, _tmp) = cfg();
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
    }

    /// **時計が戻っても印と送信は止まらない。時計だけが進んだことも後から分かる**（R17）。
    #[test]
    fn clock_jumps_do_not_stop_the_intervals() {
        let (cfg, _tmp) = cfg();
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
    }

    /// **6 時間ロックしたままでも取得率は壊れない**（R6）。
    #[test]
    fn locked_hours_are_not_failures() {
        let (cfg, _tmp) = cfg();
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
    }

    const EXPECTED_GAP: i64 = crate::EXPECTED_GAP_SEC;

    /// **区間の間に一度でも欠けたものが生存信号に残る**（R26）。
    #[test]
    fn blockers_are_sticky_within_the_interval() {
        let (cfg, _tmp) = cfg();
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
    }

    /// **書き込みが失敗しても見回りを続け、記録は抱えて積み直す**（R16 / D22）。
    #[test]
    fn write_failures_do_not_stop_collection() {
        let (cfg, _tmp) = cfg();
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
    }

    /// **落ちる前に開いていた離席と除外の数えが、次の起動で閉じられる**（R1 / R4）。
    #[test]
    fn restart_closes_previous_away_and_excluded() {
        let (cfg, _tmp) = cfg();
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
    }

    /// **除外した本文は、Runtime を通って取り込み口へ送られる本文にも現れない**（FR-83）。
    #[test]
    fn excluded_body_never_reaches_the_transport() {
        let (cfg, _tmp) = cfg();
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

    /// 基準を読むのに `gate` が閉じるまで（最長 60 秒）かかる偽物。打ち切りを長く取るのは、
    /// 見回りが読み取りを待ったときの経過（60 秒以上）と、混んだ runner の遅さ（数秒）を取り違えないため。
    #[derive(Debug)]
    struct SlowReference(std::sync::Mutex<std::sync::mpsc::Receiver<()>>);

    impl ReferenceClock for SlowReference {
        fn now(&self) -> anyhow::Result<clock::ReferenceReading> {
            let _ = self
                .0
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(60));
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
            Channel::Official,
        )
        .unwrap()
    }

    fn patrol_watchdog(
        release: std::sync::mpsc::Sender<()>,
        patrols: usize,
    ) -> (
        std::sync::mpsc::Sender<()>,
        Arc<std::sync::atomic::AtomicBool>,
        std::thread::JoinHandle<()>,
    ) {
        let (patrol_returned, patrol_returns) = mpsc::channel();
        let released = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let released_by_watchdog = Arc::clone(&released);
        let watchdog = std::thread::spawn(move || {
            for _ in 0..patrols {
                if patrol_returns
                    .recv_timeout(std::time::Duration::from_secs(30))
                    .is_err()
                {
                    released_by_watchdog.store(true, std::sync::atomic::Ordering::Relaxed);
                    let _ = release.send(());
                    break;
                }
            }
        });
        (patrol_returned, released, watchdog)
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
        let (cfg, _tmp) = cfg();
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
    }

    /// 基準の読み取りが打ち切りまでかかる間も、1 秒ごとの見回りは止まらず、切り替えは残る。
    ///
    /// Scenario: 基準の読み取りが長引いても前景の切り替えは記録に残る
    #[test]
    fn clock_worker_slow_read_does_not_hold_the_patrol() {
        let (cfg, _tmp) = cfg();
        let transport = AcceptAll::default();
        let (gate, slow) = std::sync::mpsc::channel();
        // 見張りは 9 回の見回り（0〜8 秒）を全部見る。前景が変わる 4〜8 秒の見回りも読みを待てば落ちる（review/code.md R21）
        let (patrol_returned, released_by_watchdog, watchdog) = patrol_watchdog(gate.clone(), 9);
        let mut rt = runtime_with(
            &cfg,
            &transport,
            Arc::new(SlowReference(std::sync::Mutex::new(slow))),
        );
        rt.settle_clock_worker = false;
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        // timing-ok: t(sec) を進める時刻を兼ねるループ（回数の待ちではない）
        for sec in 0..=8 {
            if sec == 4 {
                assert!(rt.clock_worker.is_some(), "読み取りが走っていない");
                src.app = "browser".into();
            }
            rt.tick_at(&mut src, t(sec), t(sec));
            patrol_returned.send(()).ok();
            assert!(
                !released_by_watchdog.load(std::sync::atomic::Ordering::Relaxed),
                "見張りが読みを放した: {sec} 秒の見回りが基準の読み取りを待った"
            );
        }
        watchdog.join().unwrap();
        assert!(
            !released_by_watchdog.load(std::sync::atomic::Ordering::Relaxed),
            "見張りが読みを放した: 見回りが基準の読み取りを待った"
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
        let (cfg, _tmp) = cfg();
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
    }

    /// 作業スレッドが panic しても見回りは止まらず、取れなかったとして 60 秒後に測り直す。
    #[test]
    fn clock_worker_panic_is_unavailable_and_retried_after_a_minute() {
        let (cfg, _tmp) = cfg();
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
        let (cfg, _tmp) = cfg();
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
    }

    /// 壁時計が飛ぶと、次の 1 時間の契機を待たずに、飛んだ後の差を持つ記録が残る。
    ///
    /// Scenario: 壁時計が飛ぶとその場で測る
    /// Scenario: 時計が飛んだ直後の測定は飛んだ後の差を持つ
    /// Scenario: PC の測定記録に測った契機が残る
    #[test]
    fn clock_skew_record_is_taken_at_once_after_the_clock_jumps() {
        let (cfg, _tmp) = cfg();
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
    }

    /// 基準が 1 つも取れないと、取れなかった印の付いた記録が 1 件残り、基準ごとの理由を持つ。
    ///
    /// Scenario: PC で基準が 1 つも取れないと取れなかった印の付いた記録が残る
    /// Scenario: PC の取れなかった記録は基準ごとの理由を持つ
    #[test]
    fn clock_skew_record_unavailable_has_a_reason_per_source() {
        let (cfg, _tmp) = cfg();
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
    }

    /// 取れない状態が 1 時間続いても、その契機の取れなかった記録は 1 件（測り直しのたびには増えない）。
    /// 次の 1 時間の契機では改めて 1 件残る。
    ///
    /// Scenario: PC の測り直しのたびには記録を増やさない
    #[test]
    fn clock_skew_record_unavailable_is_not_repeated_by_retries() {
        let (cfg, _tmp) = cfg();
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
    }

    /// 取れなかった契機のあと、測り直しで取れたら、別の 1 件が `retry` で残る。
    ///
    /// Scenario: PC の測り直しで取れたら別の 1 件が残る
    #[test]
    fn clock_skew_record_retry_that_succeeds_leaves_another_one() {
        let (cfg, _tmp) = cfg();
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
    }

    /// 取れた測定のあとは、測り直しの記録は増えない（次は 1 時間後）。
    #[test]
    fn clock_skew_record_success_stops_the_retries() {
        let (cfg, _tmp) = cfg();
        let transport = AcceptAll::default();
        let reference = SwitchReference::new(false);
        let mut rt = runtime_with(&cfg, &transport, Arc::new(reference.clone()));
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 100, 1);
        reference.set_ok(true);
        run(&mut rt, &mut src, 101, 1_000, 1);
        assert_eq!(skew_records(&mut rt, &transport, 1_001).len(), 2);
    }

    /// PC の時計が進んでいても、測定記録も前景の記録も観測したときの PC の時計のまま。
    ///
    /// Scenario: PC の記録の時刻は補正されない
    #[test]
    fn clock_skew_record_time_is_not_corrected() {
        let (cfg, _tmp) = cfg();
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
    }

    /// 起動したとき壁時計が前回の印より戻っていたら、契機は `jump`（戻り）で残る。
    ///
    /// Scenario: PC の測定記録に測った契機が残る
    #[test]
    fn clock_skew_record_after_the_clock_went_back_has_jump_trigger() {
        let (cfg, _tmp) = cfg();
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
            url: Some(format!("https://example.test/{id}")),
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
        /// 読みのスレッドが落ちる（panic）を再現する
        panics: Mutex<bool>,
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
                panics: Mutex::new(false),
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
                panics: Mutex::new(false),
            });
            (reader, tx)
        }

        fn reads(&self) -> usize {
            *self.reads.lock().unwrap()
        }

        fn panic_on_read(&self) {
            *self.panics.lock().unwrap() = true;
        }
    }

    impl HistoryReader for FakeReader {
        fn read(&self, _tmp: &std::path::Path) -> anyhow::Result<ReadOutcome> {
            if let Some(gate) = &self.gate {
                gate.lock().unwrap().recv().ok();
            }
            *self.reads.lock().unwrap() += 1;
            assert!(!*self.panics.lock().unwrap(), "読みのスレッドが落ちた");
            let visits = match *self.profile.lock().unwrap() {
                ProfileState::Gone | ProfileState::Missing => {
                    return Ok(ReadOutcome::default());
                }
                ProfileState::Unreadable => Err(anyhow::anyhow!("履歴 DB を開けない")),
                ProfileState::Readable => Ok(self.visits.lock().unwrap().clone().into()),
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
                    visits: Ok(visits.clone().into()),
                });
            }
            Ok(ReadOutcome {
                profiles,
                names: Vec::new(),
            })
        }

        fn probe(&self, _tmp: &std::path::Path) -> Vec<crate::history::collect::ProfileHealth> {
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
    ) -> HistoryRuntime<'a> {
        HistoryRuntime(
            runtime(
                cfg,
                Engine::new(Exclusions::default()),
                transport,
                reference,
            )
            .with_history(reader),
        )
    }

    /// 履歴の取得を持つ `Runtime`。**落とす前に、取得の別スレッドが終わるまで待つ**（CH03 R2）。
    /// `HistoryWorker` は drop で join しないので、試験が取得の最中に終わると、`cfg()` の `TempDir` が消えた後に
    /// スレッドが置き場（`browser-history/`）を作り直し、一時フォルダが漏れる。`TempDir` は `rt` より先に
    /// 宣言されている（後に落ちる）ので、ここで待てば消える前にスレッドが終わる。
    struct HistoryRuntime<'a>(Runtime<'a>);

    impl<'a> std::ops::Deref for HistoryRuntime<'a> {
        type Target = Runtime<'a>;
        fn deref(&self) -> &Runtime<'a> {
            &self.0
        }
    }

    impl<'a> std::ops::DerefMut for HistoryRuntime<'a> {
        fn deref_mut(&mut self) -> &mut Runtime<'a> {
            &mut self.0
        }
    }

    impl Drop for HistoryRuntime<'_> {
        fn drop(&mut self) {
            let Some(history) = self.0.history.as_ref() else {
                return;
            };
            // 30 秒で諦める（放されない gate で止まった読みなど）。drop の中では panic しない
            // （試験が落ちて巻き戻している最中だと abort になり、落ちた理由が見えなくなる）
            let started = std::time::Instant::now();
            let deadline = started + std::time::Duration::from_secs(30);
            while std::time::Instant::now() < deadline {
                if history.threads_finished() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            eprintln!(
                "履歴の取得の別スレッドが終わらない: 待った時間 {:?}、最後に見た値 threads_finished={}",
                started.elapsed(),
                history.threads_finished()
            );
        }
    }

    /// 起動の経路（`with_history`）で、前のプロセスが残した写しが消える（design D2。R60 / R72）。
    #[test]
    fn history_copy_leftovers_are_swept_at_startup() {
        let (cfg, _tmp) = cfg();
        let tmp = cfg.state_dir.join("browser-history/tmp");
        std::fs::create_dir_all(tmp.join("left-over")).unwrap();
        std::fs::write(tmp.join("left-over/History"), b"private").unwrap();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let _rt = history_runtime(&cfg, &transport, &reference, FakeReader::new(Vec::new()));
        assert!(!tmp.join("left-over").exists(), "起動時に写しを掃除しない");
    }

    fn write_last_success(cfg: &Config, at: DateTime<Utc>) {
        let path = cfg.state_dir.join("browser-history/last_success.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!(r#"{{"last_success":"{}"}}"#, at.to_rfc3339())).unwrap();
    }

    /// 読みが終わるまで（実時間で）見回りを回す。
    fn tick_until_read(rt: &mut Runtime, src: &mut FakeSource, reader: &FakeReader, from: i64) {
        // timing-ok: t(sec) を進める時刻を兼ねるループのため、回数を維持する。
        for sec in from..from + 2000 {
            rt.tick_at(src, t(sec), t(sec));
            if reader.reads() > 0 && rt.history.as_ref().is_some_and(|h| !h.is_reading()) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("履歴の取得が終わらない");
    }

    /// 取得の別スレッドが置き場へ書き終えるまで見回りを回す。
    /// 終わった後に書かれると、消えた置き場を作り直して一時フォルダが漏れる。
    fn tick_until_history_idle(rt: &mut Runtime, src: &mut FakeSource, sec: i64) {
        let started = std::time::Instant::now();
        let deadline = started + std::time::Duration::from_secs(30);
        loop {
            let last_reading = rt.history.as_ref().is_some_and(|h| h.is_reading());
            if !last_reading {
                return;
            }
            if std::time::Instant::now() >= deadline {
                panic!(
                    "履歴の取得が終わらない: 待った時間 {:?}、最後に見た値 is_reading={last_reading}",
                    started.elapsed()
                );
            }
            rt.tick_at(src, t(sec), t(sec));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    fn history_records(t: &AcceptAll) -> Vec<serde_json::Value> {
        sent(t, "/ingest")
            .into_iter()
            .filter(|r| r["logical_source"] == "c02-browser-history")
            .collect()
    }

    #[test]
    fn st54_dev_history_reads_exclusions_from_launch_directory() {
        // Scenario: 開発版は、ブラウザの履歴の取り込みでも渡された置き場の除外の登録を読む
        let (mut cfg, tmp) = cfg();
        cfg.state_dir = tmp.path().join("dev");
        cfg.exclusions_path = tmp.path().join("exclusions.json");
        std::fs::write(
            &cfg.exclusions_path,
            r#"{"rules":[{"match":"url-contains","value":"example.test/1"}]}"#,
        )
        .unwrap();
        assert!(!cfg.state_dir.join("exclusions.json").exists());
        let transport = AcceptAll::default();
        let reference = FixedReference::new(t(0));
        let reader = FakeReader::new(vec![read_visit(1, "one"), read_visit(2, "two")]);
        let exclusions = Exclusions::load(&cfg.exclusions_path).unwrap();
        let mut rt = HistoryRuntime(
            Runtime::new(
                &cfg,
                zone(),
                Engine::new(exclusions),
                &transport,
                ClockInputs {
                    reference: Arc::new(FixedReference::new(reference.0)),
                    time_sync: Arc::new(NoTimeSync),
                    uptime: Arc::new(FixedUptime),
                },
                t(0),
                Channel::Official,
            )
            .unwrap()
            .with_history(reader.clone()),
        );
        let mut src = FakeSource::new("browser");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        rt.send();
        let records = history_records(&transport);
        assert!(records
            .iter()
            .all(|r| r["payload"]["url"] != "https://example.test/1"));
        assert!(records
            .iter()
            .any(|r| r["payload"]["url"] == "https://example.test/2"));
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
        let started = std::time::Instant::now();
        let deadline = started + std::time::Duration::from_secs(30);
        loop {
            rt.tick_at(src, t(sec), t(sec));
            rt.send();
            let last_beats = history_beats(transport).len();
            if last_beats >= want {
                return;
            }
            if std::time::Instant::now() >= deadline {
                panic!(
                    "履歴の生存信号が届かない: 待った時間 {:?}、最後に見た値 beats={last_beats}",
                    started.elapsed()
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Scenario: ブラウザ履歴のソースにも想定間隔ごとに生存信号が届く
    /// Scenario: ブラウザ履歴の生存信号はウィンドウの生存信号と別の件である
    #[test]
    fn history_heartbeat_every_day_and_separate_from_window_at_runtime() {
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
    }

    /// Scenario: 読めないプロファイルが 1 つでもあれば取得できないとして報告される
    /// Scenario: 読めなかったブラウザとプロファイルが満たされていないものに挙がる
    #[test]
    fn history_heartbeat_reports_unreadable_profile_at_runtime() {
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
    }

    /// Scenario: 履歴が 1 つも見つからなければ取得できないとして報告される
    #[test]
    fn history_heartbeat_reports_none_found_at_runtime() {
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(Vec::new());
        *reader.profile.lock().unwrap() = ProfileState::Missing;
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        let beat = &history_beats(&transport)[0];
        assert_eq!(beat["capturable"], false);
        assert_eq!(beat["blockers"], serde_json::json!(["history-none-found"]));
    }

    /// 区間に読みが 1 回も無い（直近の取得が 24 時間以内で読みを始めない）起動直後でも、開けるかを確かめて報告する。
    ///
    /// Scenario: 区間に読みが無くても、開けるかを確かめてから報告する
    #[test]
    fn history_heartbeat_probes_when_not_read_at_runtime() {
        let (cfg, _tmp) = cfg();
        write_last_success(&cfg, t(0));
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
    }

    /// 起動直後に 1 件出る（読みも契機も待たない）。
    ///
    /// Scenario: ブラウザ履歴のソースにも想定間隔ごとに生存信号が届く
    #[test]
    fn history_heartbeat_on_start_at_runtime() {
        let (cfg, _tmp) = cfg();
        write_last_success(&cfg, t(0));
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        assert_eq!(history_beats(&transport)[0]["capturable"], true);
        assert!(cfg.state_dir.join("counters-browser-history.json").exists());
    }

    /// Scenario: 起動時に前回の成功から 24 時間以上経っていれば取得する
    #[test]
    fn history_schedule_runtime_fetches_on_first_tick_after_24h() {
        let (cfg, _tmp) = cfg();
        write_last_success(&cfg, t(0) - Duration::hours(24));
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        assert_eq!(reader.reads(), 1);
        rt.send();
        assert_eq!(history_records(&transport).len(), 1);
    }

    /// Scenario: 前回の成功から 24 時間経たないうちは取得しない
    #[test]
    fn history_schedule_runtime_skips_within_24h() {
        let (cfg, _tmp) = cfg();
        write_last_success(&cfg, t(0) - Duration::hours(24) + Duration::seconds(60));
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 30, 1);
        std::thread::sleep(std::time::Duration::from_millis(100));
        run(&mut rt, &mut src, 31, 40, 1);
        assert_eq!(reader.reads(), 0, "24 時間経たないのに読んだ");
    }

    /// Scenario: 動作中に前回の成功から 24 時間経つと取得する
    #[test]
    fn history_schedule_runtime_fetches_when_24h_pass_while_running() {
        let (cfg, _tmp) = cfg();
        write_last_success(&cfg, t(0));
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
    }

    /// 時計が一度先に進んでから戻ると、前回の成功が今より先になる。そのときは 24 時間を待たずにすぐ取得し、
    /// 今の時刻を成功として書き直す（deep.md 第 4 回 Q7 / design D3。R56）。
    #[test]
    fn history_schedule_runtime_fetches_at_once_when_last_success_is_ahead() {
        let (cfg, _tmp) = cfg();
        write_last_success(&cfg, t(0) + Duration::days(400));
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        assert_eq!(reader.reads(), 1, "前回の成功が先にあるのに待った");
        let saved: serde_json::Value = serde_json::from_slice(
            &std::fs::read(cfg.state_dir.join("browser-history/last_success.json")).unwrap(),
        )
        .unwrap();
        let saved = DateTime::parse_from_rfc3339(saved["last_success"].as_str().unwrap()).unwrap();
        assert!(
            saved < t(0) + Duration::days(1),
            "成功を今の時刻に書き直していない: {saved}"
        );
        // 書き直した後は、また 24 時間ごと
        run(&mut rt, &mut src, 2000, 2040, 1);
        std::thread::sleep(std::time::Duration::from_millis(100));
        run(&mut rt, &mut src, 2041, 2050, 1);
        assert_eq!(reader.reads(), 1);
    }

    /// 取得の成功は置き場へ書かれ、次の起動では 24 時間経つまで読まない。
    #[test]
    fn history_schedule_runtime_persists_success_across_restart() {
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
    }

    /// 1 回目で送った訪問は、2 回目に読んでも積まれない。変わった訪問と新しい訪問だけが積まれる。
    #[test]
    fn history_runtime_second_fetch_queues_only_the_difference() {
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
    }

    /// 題名 `title` の訪問の記録が載せた版（`source_updated_at`）。
    fn history_version_of(t: &AcceptAll, title: &str) -> DateTime<Utc> {
        let record = history_records(t)
            .into_iter()
            .find(|r| r["payload"]["title"] == title)
            .unwrap_or_else(|| panic!("題名 {title} の訪問が届いていない"));
        DateTime::parse_from_rfc3339(record["source_updated_at"].as_str().unwrap())
            .unwrap()
            .with_timezone(&Utc)
    }

    /// 時計が先に進んだ時刻で 1 回送り、時計を戻して題名を変えた訪問を取得する（間に `damage` が
    /// `state_dir/browser-history/` を傷める）。返すのは（前に送った版, 戻った後に送った版）。
    fn history_versions_across_clock_back(
        damage: impl FnOnce(&std::path::Path),
    ) -> (DateTime<Utc>, DateTime<Utc>) {
        let (cfg, _tmp) = cfg();
        let ahead = Duration::days(400).num_seconds();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(ahead)));
        let reader = FakeReader::new(vec![read_visit(1, "前")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(ahead), t(ahead));
        tick_until_read(&mut rt, &mut src, &reader, ahead);
        rt.send();
        let before = history_version_of(&transport, "前");
        damage(&cfg.state_dir.join("browser-history"));
        *reader.visits.lock().unwrap() = vec![read_visit(1, "後")];
        // 前回の成功が今より先なので、24 時間を待たずに取得する（Q7）
        tick_until_read_again(&mut rt, &mut src, &reader, 0);
        rt.send();
        let after = history_version_of(&transport, "後");
        (before, after)
    }

    /// 時計が戻った後に書き換わった題名も、前に送った版より新しい版で届く（取り込み口で古い到着として捨てられない）。
    /// deep.md 第 6 回 Q11 / design D15。
    ///
    /// Scenario: 時計が戻った後に書き換わった題名も、前に送った版より新しい版で届く
    #[test]
    fn history_runtime_clock_back_update_has_newer_version() {
        let (before, after) = history_versions_across_clock_back(|_| {});
        assert!(
            after > before,
            "戻った後の版 {after} が前に送った版 {before} より新しくない"
        );
    }

    /// 帳面を壊して退避した後でも、前に送った版の最大値（退避で消えない）より前にしない（design D16（仮）。C4）。
    ///
    /// Scenario: 帳面を作り直した後も、時計が戻った後の更新は前に送った版より新しい版で届く
    #[test]
    fn history_runtime_clock_back_update_after_quarantine_has_newer_version() {
        let (before, after) = history_versions_across_clock_back(|dir| {
            std::fs::write(dir.join("chrome/Default.ledger"), b"{broken").unwrap();
        });
        assert!(
            after > before,
            "帳面の退避の後、戻った後の版 {after} が前に送った版 {before} より新しくない"
        );
    }

    /// 送った版の最大値が失われても、帳面の各訪問にある前に送った版より前にしない（deep.md 第 6 回 Q11 の
    /// 「帳面の各訪問に前に送った版を持つ」。review/code.md R91）。
    ///
    /// Scenario: 時計が戻った後に書き換わった題名も、前に送った版より新しい版で届く
    #[test]
    fn history_runtime_clock_back_update_without_sent_version_has_newer_version() {
        let (before, after) = history_versions_across_clock_back(|dir| {
            std::fs::remove_file(dir.join("sent_version.json")).unwrap();
        });
        assert!(
            after > before,
            "送った版の最大値を失った後、戻った後の版 {after} が前に送った版 {before} より新しくない"
        );
    }

    // ---- 組み立ての印の焼き込み（ST49 / design D13） ----

    /// `cfg()` の返り値から `Config` を取り出す。`cfg()` は CH03 で `(Config, TempDir)` に変わる（一時フォルダを試験の
    /// 終わりに消すため）。ST49 の試験はどちらの形でも通るようにここを通す（CH03 が main に入ったら外してよい）。
    trait HeldConfig {
        fn config(&self) -> &Config;
    }

    impl HeldConfig for Config {
        fn config(&self) -> &Config {
            self
        }
    }

    impl<T> HeldConfig for (Config, T) {
        fn config(&self) -> &Config {
            &self.0
        }
    }

    const STAMP_A: &str = "0.1.0+aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const STAMP_D: &str = "0.1.0+dddddddddddddddddddddddddddddddddddddddd";

    /// 印 A で積んだ未送信を、印 D の組み立てで開き直して送る。送った要求の版は A のまま。
    ///
    /// Scenario: PC の収集アプリを更新した後に送った未送信は、積んだときの版で届く
    #[test]
    fn st49_c02_pending_keeps_stamp_from_add_time() {
        let held = cfg();
        let cfg = held.config();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        Marker::new(&cfg.state_dir).touch(t(-50_000)).unwrap();
        {
            let mut rt = runtime(
                cfg,
                Engine::new(Exclusions::default()),
                &transport,
                &reference,
            );
            rt.set_build_stamp(STAMP_A);
            let mut src = FakeSource::new("editor");
            src.boot = Some(t(-60));
            rt.start_at(&src, t(0), t(0));
            assert!(rt.pending().0 >= 1, "積まれていない");
        }
        let mut rt = runtime(
            cfg,
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
        );
        rt.set_build_stamp(STAMP_D);
        rt.send();
        let records = sent(&transport, "/ingest");
        assert!(!records.is_empty(), "送られていない");
        for r in &records {
            assert_eq!(r["client_version"], STAMP_A, "{r}");
        }
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 止めている間は送れない取り込み口（見回りの中の送信で置き場が空にならないように）
    #[derive(Debug)]
    struct Switch {
        up: std::cell::Cell<bool>,
        inner: AcceptAll,
    }

    impl Switch {
        fn down() -> Self {
            Self {
                up: std::cell::Cell::new(false),
                inner: AcceptAll::default(),
            }
        }
    }

    impl Transport for Switch {
        fn post(&self, path: &str, body: &str, contract_version: u32) -> anyhow::Result<Reply> {
            if self.up.get() {
                self.inner.post(path, body, contract_version)
            } else {
                anyhow::bail!("取り込み口が止まっている")
            }
        }

        fn get(&self, path: &str) -> anyhow::Result<Reply> {
            self.inner.get(path)
        }
    }

    /// 止めた取り込み口で 1 時間回し、生存信号を未送信に積んだままの組み立てを返す。
    fn runtime_with_queued_beats<'a>(
        cfg: &Config,
        transport: &'a Switch,
        stamp: &str,
    ) -> Runtime<'a> {
        let mut rt = Runtime::new(
            cfg,
            zone(),
            Engine::new(Exclusions::default()),
            transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(t(0))),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            Channel::Official,
        )
        .unwrap();
        rt.set_build_stamp(stamp);
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        run(&mut rt, &mut src, 0, 3600, 60);
        rt
    }

    fn queued_beat_lines(cfg: &Config) -> Vec<serde_json::Value> {
        std::fs::read_to_string(cfg.state_dir.join("heartbeat.jsonl"))
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// 印 A で積んだ生存信号を、印 D の組み立てで開き直して送る。欄も原文も A のまま。
    ///
    /// Scenario: PC の生存信号は、積んだときの版を欄と原文に持つ
    #[test]
    fn st51_c02_heartbeat_keeps_enqueued_version() {
        let held = cfg();
        let cfg = held.config();
        let transport = Switch::down();
        {
            let rt = runtime_with_queued_beats(cfg, &transport, STAMP_A);
            assert!(rt.pending().1 >= 1, "生存信号が積まれていない");
        }
        let lines = queued_beat_lines(cfg);
        assert!(!lines.is_empty(), "生存信号が置き場に積まれていない");
        for l in &lines {
            assert_eq!(l["client_version"], STAMP_A, "{l}");
            let raw: serde_json::Value = serde_json::from_str(l["raw"].as_str().unwrap()).unwrap();
            assert_eq!(raw["client_version"], STAMP_A, "{l}");
        }
        let (accept, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let mut rt = runtime(cfg, Engine::new(Exclusions::default()), &accept, &reference);
        rt.set_build_stamp(STAMP_D);
        rt.send();
        let beats = sent(&accept, "/heartbeat");
        assert!(beats.len() >= lines.len(), "積んだ生存信号が送られていない");
        for b in &beats {
            assert_eq!(b["client_version"], STAMP_A, "{b}");
            let raw: serde_json::Value = serde_json::from_str(b["raw"].as_str().unwrap()).unwrap();
            assert_eq!(raw["client_version"], STAMP_A, "{b}");
        }
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 版の欄を持たない溜めた行は、今の版を足さずに送る。
    ///
    /// Scenario: PC の版の欄を持たない溜めた生存信号は、版の無いまま送られる
    #[test]
    fn st51_c02_old_heartbeat_sent_without_version() {
        let held = cfg();
        let cfg = held.config();
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        let line = r#"{"id":"00000000-0000-0000-0000-0000000000a1","user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c02-window","device_id":"dev-1","emitted_at":"2026-09-13T00:00:00.000Z","capturable":true,"blockers":[],"attempts":1,"successes":1,"raw":"{\"alive\":true}"}"#;
        std::fs::write(cfg.state_dir.join("heartbeat.jsonl"), format!("{line}\n")).unwrap();
        let (accept, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let mut rt = runtime(cfg, Engine::new(Exclusions::default()), &accept, &reference);
        rt.set_build_stamp(STAMP_D);
        rt.send();
        let beats = sent(&accept, "/heartbeat");
        let old = beats
            .iter()
            .find(|b| b["id"] == "00000000-0000-0000-0000-0000000000a1")
            .expect("古い行が送られていない");
        assert!(old.get("client_version").is_none(), "{old}");
        assert_eq!(old["raw"], r#"{"alive":true}"#);
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 起動の直後に、生存信号が 1 件積まれる（`Schedule::new()` の最初の `due`）。
    #[test]
    fn st51_c02_heartbeat_first_after_start() {
        let held = cfg();
        let cfg = held.config();
        let transport = Switch::down();
        let mut rt = Runtime::new(
            cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            ClockInputs {
                reference: Arc::new(FixedReference::new(t(0))),
                time_sync: Arc::new(NoTimeSync),
                uptime: Arc::new(FixedUptime),
            },
            t(0),
            Channel::Official,
        )
        .unwrap();
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        assert_eq!(rt.pending().1, 0, "起動だけでは積まれない");
        rt.tick_at(&mut src, t(0), t(0));
        assert_eq!(rt.pending().1, 1, "起動の直後の 1 件が積まれない");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 生存信号は未送信に溜まり、送る前に落とした後も残る。
    #[test]
    fn st51_c02_heartbeat_is_queued() {
        let held = cfg();
        let cfg = held.config();
        let transport = Switch::down();
        {
            let rt = runtime_with_queued_beats(cfg, &transport, STAMP_A);
            assert!(rt.pending().1 >= 1, "生存信号が溜まらない");
        }
        let (accept, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let rt = runtime(cfg, Engine::new(Exclusions::default()), &accept, &reference);
        assert!(rt.pending().1 >= 1, "落とした後に生存信号が残っていない");
        assert!(
            sent(&accept, "/heartbeat").is_empty(),
            "開いただけで送られた"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 印 A で送った訪問を、印 D の組み立てが読み直して題名の変わった記録を積む。版は D。
    ///
    /// Scenario: ブラウザ履歴を読み直して送る記録は、読み直した時点の版を持つ
    #[test]
    fn st49_c02_history_reread_gets_current_stamp() {
        let held = cfg();
        let cfg = held.config();
        let ahead = Duration::days(400).num_seconds();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(ahead)));
        let reader = FakeReader::new(vec![read_visit(1, "前")]);
        let mut rt = history_runtime(cfg, &transport, &reference, reader.clone());
        rt.set_build_stamp(STAMP_A);
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(ahead), t(ahead));
        tick_until_read(&mut rt, &mut src, &reader, ahead);
        rt.send();
        let first = history_records(&transport)
            .into_iter()
            .find(|r| r["payload"]["title"] == "前")
            .unwrap();
        assert_eq!(first["client_version"], STAMP_A);
        rt.set_build_stamp(STAMP_D);
        *reader.visits.lock().unwrap() = vec![read_visit(1, "後")];
        tick_until_read_again(&mut rt, &mut src, &reader, 0);
        rt.send();
        let second = history_records(&transport)
            .into_iter()
            .find(|r| r["payload"]["title"] == "後")
            .unwrap();
        assert_eq!(second["client_version"], STAMP_D);
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
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
    }

    /// Scenario: 前回の取得の後に古い時刻で入った訪問も取り込まれる
    #[test]
    fn history_runtime_late_arriving_old_visit_is_sent() {
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
        let vanished = vanished_records(&transport);
        drop(rt); // 取得のスレッドを待ってから transport を返す（HistoryRuntime）
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

    /// URL の行を失った訪問は、URL と題名を省いた `visit` として送られ、「消えた」に出ない
    /// （deep.md 第 4 回 Q8 / design D2 / D10。R57）。時刻・滞在時間・遷移の種類は残る。
    ///
    /// Scenario: URL の行だけを失った訪問は「消えた」にならない
    #[test]
    fn history_visit_whose_url_row_is_missing_is_sent_without_url_and_not_vanished() {
        let mut lost = read_visit(2, "消える題名");
        lost.duration_us = Some(1_500_000);
        lost.transition = 1;
        let (transport, vanished) = fetch_then_change(vec![read_visit(1, "a"), lost], |r| {
            let mut visits = r.visits.lock().unwrap();
            visits[1].url = None;
            visits[1].title = None;
        });
        assert!(
            vanished.is_empty(),
            "URL の行が無い訪問を消えたとした: {vanished:?}"
        );
        let visits: Vec<_> = history_records(&transport)
            .into_iter()
            .filter(|r| r["payload"]["kind"] == "visit")
            .collect();
        let without_url: Vec<_> = visits
            .iter()
            .filter(|r| r["payload"].get("url").is_none())
            .collect();
        assert_eq!(
            without_url.len(),
            1,
            "URL 無しの訪問が送られない: {visits:?}"
        );
        let p = &without_url[0]["payload"];
        assert!(p.get("title").is_none());
        assert_eq!(p["visit_id"], 2);
        assert_eq!(p["visit_duration_us"], 1_500_000);
        assert_eq!(p["transition_core"], "typed");
        assert_eq!(without_url[0]["event_time"], p["at"]);
    }

    /// 初めて読んだ時点で URL の行が無い訪問も、捨てずに URL 無しで入る（deep.md 第 4 回 Q8。R57）。
    ///
    /// Scenario: URL の行が無い訪問は URL と題名の無い記録として入る
    #[test]
    fn history_visit_without_url_row_is_sent_on_first_fetch() {
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let mut orphan = read_visit(2, "");
        orphan.url = None;
        orphan.title = None;
        let reader = FakeReader::new(vec![read_visit(1, "a"), orphan]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        rt.send();
        let records = history_records(&transport);
        assert_eq!(records.len(), 2, "URL の行が無い訪問を捨てた");
        assert!(records.iter().any(|r| r["payload"].get("url").is_none()));
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
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
    }

    /// 次の読みが終わるまで（`reads` が増えるまで）見回しを回す。
    fn tick_until_read_again(
        rt: &mut Runtime,
        src: &mut FakeSource,
        reader: &FakeReader,
        from: i64,
    ) {
        let before = reader.reads();
        // timing-ok: t(sec) を進める時刻を兼ねるループのため、回数を維持する。
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
            fn get(&self, path: &str) -> anyhow::Result<Reply> {
                if self.up.get() {
                    self.inner.get(path)
                } else {
                    anyhow::bail!("取り込み口が止まっている")
                }
            }

            fn post(&self, path: &str, body: &str, version: u32) -> anyhow::Result<Reply> {
                if self.up.get() {
                    self.inner.post(path, body, version)
                } else {
                    anyhow::bail!("取り込み口が止まっている")
                }
            }
        }
        let (cfg, _tmp) = cfg();
        let transport = Switch {
            up: std::cell::Cell::new(false),
            inner: AcceptAll::default(),
        };
        let reference = FixedReference::new(t(0));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        // 途中の assert で落ちても、取得の別スレッドを待ってから一時フォルダを消す（CH03 R27）
        let mut rt = HistoryRuntime(
            Runtime::new(
                &cfg,
                zone(),
                Engine::new(Exclusions::default()),
                &transport,
                ClockInputs {
                    reference: Arc::new(FixedReference::new(reference.0)),
                    time_sync: Arc::new(NoTimeSync),
                    uptime: Arc::new(FixedUptime),
                },
                t(0),
                Channel::Official,
            )
            .unwrap()
            .with_history(reader.clone()),
        );
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
        tick_until_history_idle(&mut rt, &mut src, 100);
    }

    /// 読みが数分かかっても見回りは続き、ウィンドウのソースに眠りも履歴の記録も入らない。
    ///
    /// Scenario: 履歴の取得でウィンドウのソースの記録は増えない
    #[test]
    fn history_slow_read_does_not_disturb_window() {
        let window_records = |with_history: bool| {
            let (cfg, _tmp) = cfg();
            let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
            let (reader, release) = FakeReader::gated(vec![read_visit(1, "a")]);
            let mut rt = Runtime::new(
                &cfg,
                zone(),
                Engine::new(Exclusions::default()),
                &transport,
                ClockInputs {
                    reference: Arc::new(FixedReference::new(reference.0)),
                    time_sync: Arc::new(NoTimeSync),
                    uptime: Arc::new(FixedUptime),
                },
                t(0),
                Channel::Official,
            )
            .unwrap();
            if with_history {
                rt = rt.with_history(reader.clone());
            }
            // 途中の assert で落ちても、取得の別スレッドを待ってから一時フォルダを消す（CH03 R27）。
            // gate は rt より後に束縛し直し、先に放す（rt の Drop が止まった読みを待ち続けない。R28）
            let mut rt = HistoryRuntime(rt);
            let release = release;
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
            tick_until_history_idle(&mut rt, &mut src, 200);
            window.len()
        };
        assert_eq!(window_records(true), window_records(false));
    }

    /// 訪問の多い取得（初回に限らない。帳面がある状態で数万件が新しく入った取得）でも、積み終えるまでの間
    /// 見回りは毎回すぐ戻り、その間の前景の変化が積まれる（deep.md 第 5 回 Q9。review/code.md R68）。
    ///
    /// Scenario: 訪問の多い取得でも前景の観測は止まらない
    /// Scenario: 履歴の読みを止めている間も、見回りは返り、前景の変化が積まれる
    /// Scenario: 読みを放した後、見回り 1 回で積む履歴の記録は 2,000 件以下
    #[test]
    fn history_large_fetch_keeps_observing_foreground() {
        use crate::history::collect::QUEUE_PER_TICK;
        // 下の比べは製品の定数に合わせて緩むので、値そのものを固定する（design D18（仮）。review/code.md R17）
        assert_eq!(QUEUE_PER_TICK, 2_000, "見回り 1 回に積む上限は 2,000 件");
        const N: i64 = 30_000;
        // 送らない（未送信に積んだ順をそのまま見る）
        #[derive(Debug)]
        struct Down;
        impl Transport for Down {
            fn get(&self, _: &str) -> anyhow::Result<Reply> {
                anyhow::bail!("取り込み口が止まっている")
            }

            fn post(&self, _: &str, _: &str, _: u32) -> anyhow::Result<Reply> {
                anyhow::bail!("取り込み口が止まっている")
            }
        }
        let (cfg, _tmp) = cfg();
        let reference = FixedReference::new(t(0));
        let (reader, release) = FakeReader::gated((1..=10).map(|i| read_visit(i, "a")).collect());
        // 途中の assert で落ちても、取得の別スレッドを待ってから一時フォルダを消す（CH03 R27）
        let mut rt = HistoryRuntime(
            Runtime::new(
                &cfg,
                zone(),
                Engine::new(Exclusions::default()),
                &Down,
                ClockInputs {
                    reference: Arc::new(FixedReference::new(reference.0)),
                    time_sync: Arc::new(NoTimeSync),
                    uptime: Arc::new(FixedUptime),
                },
                t(0),
                Channel::Official,
            )
            .unwrap()
            .with_history(reader.clone()),
        );
        // 放す口は rt より後に置き、rt より先に落とす。途中の assert で落ちたとき、gate で止まった読みが
        // 放されてから rt の drop が取得の別スレッドを待つ（review/code.md R23）
        let release = release;
        let mut src = FakeSource::new("editor");
        let (initial_patrol_returned, initial_released_by_watchdog, initial_watchdog) =
            patrol_watchdog(release.clone(), 1);
        rt.start_at(&src, t(0), t(0));
        release.send(()).unwrap();
        tick_until_read(&mut rt, &mut src, &reader, 0);
        initial_patrol_returned.send(()).ok();
        initial_watchdog.join().unwrap();
        assert!(
            !initial_released_by_watchdog.load(std::sync::atomic::Ordering::Relaxed),
            "見張りが読みを放した: 初回の履歴取得を待った"
        );
        // 帳面がある状態で、同期などで新しい訪問がまとめて入る
        *reader.visits.lock().unwrap() = (1..=N).map(|i| read_visit(i, "a")).collect();
        let history = |rt: &Runtime| {
            rt.events
                .snapshot()
                .iter()
                .filter(|r| r.logical_source == "c02-browser-history")
                .count()
        };
        let before = history(&rt);
        // 24 時間後へ飛ぶ（この 1 回の見回りの「眠っていた」は時刻を飛ばしたことによる）
        let mut sec = 86_400;
        let (patrol_returned, released_by_watchdog, watchdog) = patrol_watchdog(release.clone(), 5);
        rt.tick_at(&mut src, t(sec), t(sec));
        let from = rt.events.len();
        let mut queueing_ticks = 0;
        let mut switched_at = None;
        for _ in 0..5 {
            sec += 1;
            rt.tick_at(&mut src, t(sec), t(sec));
            patrol_returned.send(()).ok();
            assert!(
                !released_by_watchdog.load(std::sync::atomic::Ordering::Relaxed),
                "見張りが読みを放した: 見回りが履歴の取得を待った"
            );
            assert_eq!(history(&rt), before, "止めた読みの間に履歴を積んだ");
            if src.app == "editor" {
                src.app = "browser".into();
            }
        }
        watchdog.join().unwrap();
        assert!(
            !released_by_watchdog.load(std::sync::atomic::Ordering::Relaxed),
            "見張りが読みを放した: 見回りが履歴の取得を待った"
        );
        // 止めた読みが本当に始まって gate で止まっていたか（始まっていなければ、上の 5 回は読みと関係なく通る。R22）
        assert!(
            rt.history.as_ref().unwrap().is_reading() && reader.reads() == 1,
            "止めた読みが始まっていない: 読んだ回数 {}・読み中 {}",
            reader.reads(),
            rt.history.as_ref().unwrap().is_reading()
        );
        let stopped_read_events = rt.events.snapshot();
        assert!(
            stopped_read_events
                .iter()
                .any(|r| r.logical_source == "c02-window" && r.raw.contains("browser")),
            "読みを止めている間の前景の切り替えが積まれていない"
        );
        release.send(()).unwrap();
        src.app = "editor".into();
        let mut max_queued_per_tick = 0;
        // 時刻を兼ねるループ（1 回ごとに t(sec) を進める）。実時間の上限 30 秒も持つ（R22）
        let real_started = std::time::Instant::now();
        let real_deadline = real_started + std::time::Duration::from_secs(30);
        while reader.reads() < 2 || rt.history.as_ref().unwrap().is_reading() {
            assert!(
                std::time::Instant::now() < real_deadline,
                "放した読みが終わらない: 読んだ回数 {}・積んだ履歴 {}・待った時間 {:?}",
                reader.reads(),
                history(&rt),
                real_started.elapsed()
            );
            sec += 1;
            let previous = history(&rt);
            rt.tick_at(&mut src, t(sec), t(sec));
            let queued = history(&rt).saturating_sub(previous);
            max_queued_per_tick = max_queued_per_tick.max(queued);
            if queued > 0 {
                queueing_ticks += 1;
                if switched_at.is_none() {
                    src.app = "browser".into();
                    switched_at = Some(rt.events.len());
                }
            }
            assert!(sec < 86_400 + 60_000, "取得が終わらない");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(
            max_queued_per_tick <= QUEUE_PER_TICK,
            "見回り 1 回で積みすぎた: {max_queued_per_tick} 件"
        );
        assert!(
            queueing_ticks >= (N as usize - 10) / QUEUE_PER_TICK,
            "積み込みを見回りに分けていない: {queueing_ticks} 回"
        );
        let events = rt.events.snapshot();
        let switched_at = switched_at.expect("積んでいる途中が無かった");
        let window_at = (switched_at..events.len())
            .find(|&i| events[i].logical_source == "c02-window")
            .expect("積んでいる間の前景の変化が積まれていない");
        assert!(
            events[window_at..]
                .iter()
                .any(|r| r.logical_source == "c02-browser-history"),
            "前景の変化が積み込みの後になった（見回りが止まっていた）"
        );
        assert!(
            events[from..].iter().all(|r| !r.raw.contains("suspended")),
            "積み込みの間に眠りが入った"
        );
        assert_eq!(
            events
                .iter()
                .filter(|r| r.logical_source == "c02-browser-history"
                    && r.raw.contains(r#""kind":"visit""#))
                .count(),
            N as usize,
            "積みこぼした"
        );
        tick_until_history_idle(&mut rt, &mut src, sec + 1);
    }

    /// 除外の登録が壊れて、読めても積めない取得が続く間は、履歴の生存信号が「取得できない」を報告し、
    /// 理由に種別だけを載せる（deep.md 第 5 回 Q10。review/code.md R70）。
    ///
    /// Scenario: 送る準備を終えられない取得は取得できないとして報告される
    #[test]
    fn history_heartbeat_reports_not_queued_at_runtime() {
        let (cfg, _tmp) = cfg();
        write_rules(&cfg, r#"[{"kind":"no-such-kind","value":"x"}]"#);
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        // 3 日回す（失敗した取得は 1 分から 1 時間まで間を空けて試し直す）
        for day in 1..=3 {
            for sec in ((day - 1) * 86_400 + 60..day * 86_400).step_by(60) {
                rt.tick_at(&mut src, t(sec), t(sec));
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            tick_until_history_beats(
                &mut rt,
                &mut src,
                &transport,
                day * 86_400,
                day as usize + 1,
            );
        }
        assert!(reader.reads() >= 3, "読みが試し直されていない");
        assert!(history_records(&transport).is_empty(), "壊れた登録で送った");
        let beats = history_beats(&transport);
        for beat in &beats[1..] {
            assert_eq!(
                beat["capturable"], false,
                "積めないのに取得できるとした: {beat}"
            );
            assert_eq!(beat["blockers"], serde_json::json!(["history-not-queued"]));
            assert!(beat["successes"].as_i64() < beat["attempts"].as_i64());
            assert!(!beat["raw"].as_str().unwrap().contains("example.test"));
        }
    }

    /// 前のプロセスの最後の取得が積めなかったなら、起動し直した直後の生存信号（確かめで出るもの）も
    /// 「取得できない」を報告する。毎晩切る PC では毎日の信号が起動直後の 1 本になる（2026-10-12 の整合の確認 K1。Q10）。
    ///
    /// Scenario: 前の起動の取得が送る準備を終えられなかったなら、起動直後の生存信号も取得できない
    #[test]
    fn history_heartbeat_at_startup_reports_previous_not_queued() {
        let (cfg, _tmp) = cfg();
        write_rules(&cfg, r#"[{"kind":"no-such-kind","value":"x"}]"#);
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        {
            let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
            let mut src = FakeSource::new("editor");
            rt.start_at(&src, t(0), t(0));
            tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
            tick_until_read(&mut rt, &mut src, &reader, 0);
            // 前のプロセスの区間の信号まで送り切る（次のプロセスが送り残しを送ると、見たい信号と混ざる）
            rt.send();
        }
        // 起動し直す。読みは確かめより後に終わる（本物の読みは全行、確かめは開けるかまで）
        let (gated, release) = FakeReader::gated(vec![read_visit(1, "a")]);
        let transport = AcceptAll::default();
        let mut rt = history_runtime(&cfg, &transport, &reference, gated);
        // rt より後に束縛し直す: 宣言の逆順に落ちるので、assert で落ちたとき gate が先に放され、
        // rt の Drop が止まった読みを 20 秒待たずに済む（CH03 R28）
        let release = release;
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(60), t(60));
        tick_until_history_beats(&mut rt, &mut src, &transport, 60, 1);
        let beats = history_beats(&transport);
        assert_eq!(
            beats[0]["capturable"], false,
            "前の取得が積めなかったのに、起動直後に取得できるとした: {}",
            beats[0]
        );
        assert_eq!(
            beats[0]["blockers"],
            serde_json::json!(["history-not-queued"])
        );
        release.send(()).ok();
        tick_until_history_idle(&mut rt, &mut src, 60);
    }

    /// 送った版の最大値を書けない取得は積まず、「取得できない」に数える（design D16（仮）の「書けなければ
    /// 送る準備の失敗にする」。review/code.md R91）。
    ///
    /// Scenario: 送る準備を終えられない取得は取得できないとして報告される
    #[test]
    fn history_heartbeat_counts_unwritable_sent_version_as_not_queued() {
        let (cfg, _tmp) = cfg();
        // 一時ファイルと同名のディレクトリがあると、送った版の最大値のファイルを書けない
        // （本体と同名だと読めないファイルとして退避される。deep.md 第 8 回 Q13）
        std::fs::create_dir_all(cfg.state_dir.join("browser-history/sent_version.tmp")).unwrap();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        let day = 86_400;
        for sec in (60..day).step_by(600) {
            rt.tick_at(&mut src, t(sec), t(sec));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        tick_until_history_beats(&mut rt, &mut src, &transport, day, 2);
        rt.send();
        assert!(reader.reads() >= 1, "読みが走っていない");
        assert!(
            history_records(&transport).is_empty(),
            "送った版の最大値を書けないのに送った"
        );
        let beat = &history_beats(&transport)[1];
        assert_eq!(
            beat["capturable"], false,
            "書けないのに取得できるとした: {beat}"
        );
        assert_eq!(beat["blockers"], serde_json::json!(["history-not-queued"]));
    }

    /// 送った版の最大値のファイルがあるのに読めなければ、低い版で上書きせずに退避し、その区間の生存信号に
    /// 種別を載せる（「取得できない」。D17（仮））。読めた訪問は送る。次の取得はファイルが無いだけなので載せない
    /// （deep.md 第 8 回 Q13。review/code.md R90）。
    ///
    /// Scenario: 送った版の最大値が読めなかった取得は、生存信号に種別で報告される
    #[test]
    fn history_heartbeat_reports_unreadable_sent_version() {
        let (cfg, _tmp) = cfg();
        let dir = cfg.state_dir.join("browser-history");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sent_version.json"), b"{broken").unwrap();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        let day = 86_400;
        for sec in (60..day).step_by(600) {
            rt.tick_at(&mut src, t(sec), t(sec));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        tick_until_history_beats(&mut rt, &mut src, &transport, day, 2);
        rt.send();
        assert_eq!(history_records(&transport).len(), 1, "読めた訪問を送らない");
        assert_eq!(
            std::fs::read(dir.join("sent_version.broken.json")).unwrap(),
            b"{broken",
            "読めないファイルを退避せずに上書きした"
        );
        let beat = &history_beats(&transport)[1];
        assert_eq!(
            beat["capturable"], false,
            "読めないのに取得できるとした: {beat}"
        );
        assert_eq!(
            beat["blockers"],
            serde_json::json!(["history-sent-version-unreadable"])
        );
        assert!(
            !beat.to_string().contains("sent_version.json"),
            "パスを載せた: {beat}"
        );
        // 次の取得（ファイルが無いだけ）では載せない
        tick_until_read_again(&mut rt, &mut src, &reader, 2 * day + 10);
        for sec in (2 * day + 10..3 * day).step_by(600) {
            rt.tick_at(&mut src, t(sec), t(sec));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        tick_until_history_beats(&mut rt, &mut src, &transport, 3 * day, 3);
        let beat = &history_beats(&transport)[2];
        assert_eq!(
            beat["blockers"],
            serde_json::json!([]),
            "無いだけで載せた: {beat}"
        );
    }

    /// 読み（と積む分の組み立て）のスレッドが落ちた取得も「取得できない」に数え、
    /// 確かめの「開ける」で取得できるにしない（2026-10-12 の整合の確認 K4。Q10）。
    ///
    /// Scenario: 送る準備を終えられない取得は取得できないとして報告される
    #[test]
    fn history_heartbeat_counts_panicked_read_as_not_queued() {
        let (cfg, _tmp) = cfg();
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        reader.panic_on_read();
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_history_beats(&mut rt, &mut src, &transport, 0, 1);
        // 区間の間に落ちた読みを経てから、次の信号を待つ
        let day = 86_400;
        for sec in (60..day).step_by(600) {
            rt.tick_at(&mut src, t(sec), t(sec));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        tick_until_history_beats(&mut rt, &mut src, &transport, day, 2);
        assert!(reader.reads() >= 1, "読みが走っていない");
        let beat = &history_beats(&transport)[1];
        assert_eq!(beat["capturable"], false, "落ちた読みを数えない: {beat}");
        assert_eq!(beat["blockers"], serde_json::json!(["history-not-queued"]));
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
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
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
        drop(rt); // 取得のスレッドを待ってから transport を返す（HistoryRuntime）
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
        let (cfg, _tmp) = cfg();
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
    }

    /// `exe-path` は実行ファイルのフルパスで登録する（README の手順）。履歴には、そのファイル名部分で当てる（design D11。R58）。
    #[test]
    fn history_exclusion_exe_path_covers_all_profiles() {
        let (cfg, _tmp) = cfg();
        write_rules(
            &cfg,
            r#"[{"match":"exe-path","value":"C:\\Program Files\\Google\\Chrome\\Application\\CHROME.EXE"}]"#,
        );
        let reader = FakeReader::new(vec![read_visit(1, "ひとつめの題名")]);
        with_other_profile(&reader, "Profile 2", vec![read_visit(2, "ふたつめの題名")]);
        let (transport, all) = fetch_and_send(&cfg, &reader);
        for leaked in ["example.test/1", "example.test/2", "ひとつめの題名"] {
            assert!(
                !all.contains(leaked),
                "除外したはずの本文が送られた: {leaked}"
            );
        }
        assert!(history_urls_of_kind(&transport, "visit").is_empty());
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
        let (cfg, _tmp) = cfg();
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
    }

    /// 2 回目の取得（1 日後）まで回して送る。
    fn fetch_twice(cfg: &Config, reader: &Arc<FakeReader>) -> AcceptAll {
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let mut rt = history_runtime(cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, reader, 0);
        rt.send();
        tick_until_read_again(&mut rt, &mut src, reader, Duration::hours(24).num_seconds());
        rt.send();
        drop(rt); // 取得のスレッドを待ってから transport を返す（HistoryRuntime）
        transport
    }

    /// Scenario: 除外した訪問は取得のたびに数え直されない
    #[test]
    fn history_exclusion_is_not_recounted_each_fetch() {
        let (cfg, _tmp) = cfg();
        write_rules(&cfg, r#"[{"match":"process-name","value":"chrome.exe"}]"#);
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let transport = fetch_twice(&cfg, &reader);
        let counted: u64 = excluded_records(&transport)
            .iter()
            .map(|r| r["payload"]["excluded_count"].as_u64().unwrap())
            .sum();
        assert_eq!(counted, 1, "2 回目の取得で数え直した");
    }

    /// Scenario: 題名の部分一致の登録はページの題名に当たる
    #[test]
    fn history_exclusion_title_matches_page_title() {
        let (cfg, _tmp) = cfg();
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
    }

    /// Scenario: URL の部分一致の登録は履歴にも効く
    #[test]
    fn history_exclusion_url_matches_history() {
        let (cfg, _tmp) = cfg();
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
    }

    /// Scenario: プロファイルを指す登録はそのプロファイルの履歴だけを除く
    #[test]
    fn history_exclusion_profile_is_specific() {
        let (cfg, _tmp) = cfg();
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
    }

    /// Scenario: 取得をやり直しても除外の件数は増えない
    #[test]
    fn history_exclusion_retry_counts_once() {
        let (cfg, _tmp) = cfg();
        write_rules(&cfg, r#"[{"match":"process-name","value":"chrome.exe"}]"#);
        let reader = FakeReader::new(vec![read_visit(1, "a")]);
        let (transport, reference) = (AcceptAll::default(), FixedReference::new(t(0)));
        let mut rt = history_runtime(&cfg, &transport, &reference, reader.clone());
        let mut src = FakeSource::new("editor");
        rt.start_at(&src, t(0), t(0));
        tick_until_read(&mut rt, &mut src, &reader, 0);
        drop(rt);
        // 除外の記録を積んだ後・取得の成功を書く前に止まった（帳面は書く前のまま）。起動し直してやり直す
        std::fs::remove_dir_all(cfg.state_dir.join("browser-history")).unwrap();
        let mut again = history_runtime(&cfg, &transport, &reference, reader.clone());
        let before = reader.reads();
        // timing-ok: t(sec) を進める時刻を兼ねるループのため、回数を維持する（design D4 の除く・Q5）。
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
    }

    /// Scenario: 登録を後から足すと、既に送った訪問の変わった内容は送られない
    #[test]
    fn history_exclusion_added_later() {
        let (cfg, _tmp) = cfg();
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
    }

    /// URL で除外した訪問が URL の行を失っても、URL 無しの訪問として送られない（除外を引き継ぐ。D11 / deep.md 第 4 回 Q8）。
    #[test]
    fn history_exclusion_survives_lost_url_row() {
        let (cfg, _tmp) = cfg();
        write_rules(
            &cfg,
            r#"[{"match":"url-contains","value":"example.test/2"}]"#,
        );
        let reader = FakeReader::new(vec![read_visit(1, "a"), read_visit(2, "伏せた")]);
        let (first, _) = fetch_and_send(&cfg, &reader);
        assert_eq!(history_urls_of_kind(&first, "visit").len(), 1);
        {
            let mut visits = reader.visits.lock().unwrap();
            visits[1].url = None;
            visits[1].title = None;
        }
        let second = fetch_twice_from_second(&cfg, &reader);
        assert!(
            history_records(&second)
                .iter()
                .all(|r| r["payload"]["visit_id"] != 2),
            "除外した訪問が URL を失って送られた"
        );
    }

    /// 前回の成功を 1 日前に置いて、同じ状態の置き場でもう 1 回取得する。
    fn fetch_twice_from_second(cfg: &Config, reader: &Arc<FakeReader>) -> AcceptAll {
        write_last_success(cfg, t(0) - Duration::hours(24));
        fetch_and_send(cfg, reader).0
    }

    /// Scenario: 登録を外すと、まだ履歴にある除外済みの訪問が次の取得で送られる
    #[test]
    fn history_exclusion_removed_later() {
        let (cfg, _tmp) = cfg();
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
    }
}
