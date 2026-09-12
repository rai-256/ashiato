// SPDX-License-Identifier: AGPL-3.0-only
//! 部品を噛み合わせる。**時計と OS を外から渡す**ので、実機を待たずに確かめられる。
//!
//! 1 回の見回り（`tick`）でやること:
//!
//! 1. 見回りの時刻が飛んでいたら「眠っていた」として残す（design D19）
//! 2. 前景と入力の状態を見て、記録になったものを未送信へ積む
//! 3. 「ここまで動いていた」の印を更新する（FR-82 の材料。design D10）
//! 4. 契機が来ていれば 時計のずれ / 生存信号 / 送信 を行う
use chrono::{DateTime, Duration, Utc};

use crate::clock::{self, ReferenceClock, SkewSchedule};
use crate::config::{Config, Zone};
use crate::contract::{HeartbeatRequest, IngestRequest, WindowPayload};
use crate::engine::{Engine, Observation, UrlRead};
use crate::heartbeat::{self, Capability, CounterStore, Counters};
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
///
/// 見回りは 1 秒間隔なので、2 分の飛びは**止まっていたこと以外では起きない**。
pub const SUSPEND_GAP_SEC: i64 = 120;

/// OS を触る側。**実機では `platform::WindowsSource`、試験では偽物。**
pub trait Source: std::fmt::Debug {
    /// いまの前景と入力の状態を読む。
    fn observe(&mut self, at: DateTime<Utc>) -> Observation;
}

/// 収集の本体。
#[derive(Debug)]
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
    counters: Counters,
    counter_store: CounterStore,
    beat_schedule: heartbeat::Schedule,
    skew_schedule: SkewSchedule,
    last_tick: Option<DateTime<Utc>>,
    last_touch: Option<DateTime<Utc>>,
    last_send: Option<DateTime<Utc>>,
    capability: Capability,
}

impl<'a> Runtime<'a> {
    /// 置き場を開き、数えを読み戻す。
    pub fn new(
        cfg: &Config,
        zone: Zone,
        engine: Engine,
        transport: &'a dyn Transport,
        reference: &'a dyn ReferenceClock,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let counter_store = CounterStore::new(&cfg.state_dir);
        let counters = counter_store.load()?.unwrap_or_else(|| Counters::new(now));
        Ok(Self {
            user_id: cfg.user_id,
            device_id: cfg.device_id.clone(),
            zone,
            engine,
            events: Outbox::open(cfg.state_dir.join("outbox.jsonl"))?,
            // **生存信号は別のファイル**（同じ JSONL に混ぜると読み戻しで
            // 片方が「壊れた行」に見えて退避に回る。ST02 の契約）
            beats: Outbox::open(cfg.state_dir.join("heartbeat.jsonl"))?,
            ingest: Sender::ingest(),
            beat_sender: Sender::heartbeat(),
            transport,
            reference,
            marker: Marker::new(&cfg.state_dir),
            counters,
            counter_store,
            beat_schedule: heartbeat::Schedule::new(),
            skew_schedule: SkewSchedule::new(),
            last_tick: None,
            last_touch: None,
            last_send: None,
            capability: Capability::of(true, true),
        })
    }

    /// 起動時にやること。**前回の停止から今回の起動までを 1 件残す**（FR-82）。
    ///
    /// 印を読むのは**この 1 回だけ** —— 読んでから上書きするので、
    /// 順序を逆にすると停止期間が永久に消える。
    pub fn start(&mut self, now: DateTime<Utc>) -> anyhow::Result<()> {
        let last_seen = self.marker.read()?;
        if let Some(p) = marker::powered_off_span(last_seen, now) {
            self.push(&p, now)?;
        }
        self.marker.touch(now)?;
        self.last_touch = Some(now);
        Ok(())
    }

    /// 1 回の見回り。
    pub fn tick(&mut self, source: &mut dyn Source, now: DateTime<Utc>) -> anyhow::Result<()> {
        // 1. 見回りが飛んでいたら「眠っていた」（design D19）
        if let Some(last) = self.last_tick {
            if now - last >= Duration::seconds(SUSPEND_GAP_SEC) {
                for p in self.engine.report_suspend(last, now) {
                    self.push(&p, now)?;
                }
            }
        }
        self.last_tick = Some(now);

        // 2. 観測 → 記録
        let obs = source.observe(now);
        self.capability = capability_of(&obs);
        self.counters.record(obs.foreground.is_some());
        self.counter_store.save(&self.counters)?;
        for p in self.engine.observe(obs) {
            self.push(&p, now)?;
        }

        // 3. 「ここまで動いていた」の印（FR-82 の材料）
        if self
            .last_touch
            .is_none_or(|t| now - t >= Duration::seconds(TOUCH_INTERVAL_SEC))
        {
            self.marker.touch(now)?;
            self.last_touch = Some(now);
        }

        // 4. 契機
        self.maybe_measure_skew(now)?;
        self.maybe_beat(now)?;
        self.maybe_send(now)?;
        Ok(())
    }

    /// 終わるときに手元のものを吐き出す（除外の数えを抱えたまま消さない）。
    pub fn stop(&mut self, now: DateTime<Utc>) -> anyhow::Result<()> {
        let flushed = self.engine.flush(now);
        for p in &flushed {
            self.push(p, now)?;
        }
        self.send(now)
    }

    /// 溜まっている件数（記録・生存信号）。
    pub fn pending(&self) -> (usize, usize) {
        (self.events.len(), self.beats.len())
    }

    fn push(&mut self, p: &WindowPayload, _now: DateTime<Utc>) -> anyhow::Result<()> {
        let at = DateTime::parse_from_rfc3339(&p.at)?.with_timezone(&Utc);
        let req = IngestRequest::of(p, self.user_id, &self.device_id, at, &self.zone)?;
        self.events.add(req)
    }

    fn maybe_measure_skew(&mut self, now: DateTime<Utc>) -> anyhow::Result<()> {
        if !self.skew_schedule.due(now) {
            return Ok(());
        }
        match self.reference.now() {
            Ok(reference) => {
                let p = clock::measure(now, reference);
                self.push(&p, now)?;
                self.skew_schedule.mark(now);
            }
            // **推測で埋めない。** 測れなかった契機は次の契機で測り直す
            Err(e) => tracing::info!(
                "{}",
                telemetry::line(
                    "clock_skew_unavailable",
                    None,
                    None,
                    Some(telemetry::error_kind(&e))
                )
            ),
        }
        Ok(())
    }

    fn maybe_beat(&mut self, now: DateTime<Utc>) -> anyhow::Result<()> {
        if !self.beat_schedule.due(now) {
            return Ok(());
        }
        let (attempts, successes) = self.counters.take(now);
        self.counter_store.save(&self.counters)?;
        let sig = heartbeat::signal(
            self.user_id,
            &self.device_id,
            now,
            &self.capability,
            attempts,
            successes,
        )?;
        self.beats.add(sig)?;
        self.beat_schedule.mark(now);
        Ok(())
    }

    fn maybe_send(&mut self, now: DateTime<Utc>) -> anyhow::Result<()> {
        if self
            .last_send
            .is_none_or(|t| now - t >= Duration::seconds(SEND_INTERVAL_SEC))
        {
            // 除外の数えは送信の契機ごとに吐き出す（design D18）
            let flushed = self.engine.flush(now);
            for p in &flushed {
                self.push(p, now)?;
            }
            self.send(now)?;
            self.last_send = Some(now);
        }
        Ok(())
    }

    fn send(&mut self, _now: DateTime<Utc>) -> anyhow::Result<()> {
        let mut log = |line: String| tracing::info!("{line}");
        self.ingest
            .flush(&mut self.events, self.transport, &mut log)?;
        self.beat_sender
            .flush(&mut self.beats, self.transport, &mut log)?;
        Ok(())
    }
}

/// 取得できる状態かを観測から決める（design D4）。
///
/// **URL の経路は「前景がブラウザのとき」しか判らない** ——
/// ブラウザでない前景を見ている間に「応答しない」と報告すると、
/// 取得率が生活の都合（どのアプリを使ったか）で上下する。
fn capability_of(obs: &Observation) -> Capability {
    // **ロック中は「取れない」ではない。** 前景が無いのは画面が閉じているからで、
    // 収集は壊れていない —— ここを取得不能に数えると、成功条件 1（NFR-13）の
    // 分子が「席にいた日」に化ける
    if obs.locked {
        return Capability::of(true, true);
    }
    let foreground = obs.foreground.is_some();
    let url_ok = !matches!(
        obs.foreground.as_ref().map(|f| &f.url),
        Some(UrlRead::Unavailable)
    );
    Capability::of(foreground, url_ok)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::engine::{Foreground, IdleRead};
    use crate::exclusion::Exclusions;
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

    /// 前景をそのまま返す偽の OS。
    #[derive(Debug)]
    struct FakeSource {
        app: String,
    }

    impl Source for FakeSource {
        fn observe(&mut self, at: DateTime<Utc>) -> Observation {
            Observation {
                at,
                foreground: Some(Foreground {
                    app_name: self.app.clone(),
                    exe_path: format!(r"C:\apps\{}.exe", self.app),
                    process_name: format!("{}.exe", self.app),
                    title: "題名".into(),
                    url: UrlRead::NotBrowser,
                }),
                idle: IdleRead::Elapsed(Duration::zero()),
                locked: false,
            }
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

    /// 偽の取り込み口が受け取った**記録**（`/ingest` の分だけ）。
    fn sent_records(t: &AcceptAll) -> Vec<serde_json::Value> {
        t.bodies
            .borrow()
            .iter()
            .filter(|(p, _)| p == "/ingest")
            .flat_map(|(_, b)| serde_json::from_str::<Vec<serde_json::Value>>(b).unwrap())
            .collect()
    }

    #[derive(Debug)]
    struct FixedReference(DateTime<Utc>);

    impl ReferenceClock for FixedReference {
        fn now(&self) -> anyhow::Result<DateTime<Utc>> {
            Ok(self.0)
        }
    }

    /// **起動時に「PC が止まっていた」が 1 件積まれる**（FR-82）。
    #[test]
    fn start_records_powered_off_span() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        // 前回の停止の印を置く
        std::fs::create_dir_all(&cfg.state_dir).unwrap();
        Marker::new(&cfg.state_dir).touch(t(-50_000)).unwrap();

        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
            t(0),
        )
        .unwrap();
        rt.start(t(0)).unwrap();
        assert_eq!(rt.pending().0, 1, "止まっていた期間が積まれていない");
        let raw = rt.events.snapshot()[0].raw.clone();
        assert!(
            raw.contains("powered-off") && raw.contains("range_end"),
            "{raw}"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// **見回りの時刻が飛んだら「眠っていた」として残す**（design D19）。
    #[test]
    fn suspend_gap_is_recorded() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
            t(0),
        )
        .unwrap();
        let mut src = FakeSource {
            app: "editor".into(),
        };
        rt.start(t(0)).unwrap();
        rt.tick(&mut src, t(0)).unwrap();
        // 1 時間ぶん見回りが飛ぶ（PC が眠っていた）
        rt.tick(&mut src, t(3_600)).unwrap();
        // **送った本文を見る** —— 送信の契機（5 分）を越えているので、
        // 未送信は受理された分だけ取り除かれて空になっている
        let suspended: Vec<serde_json::Value> = sent_records(&transport)
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
            serde_json::json!(crate::contract::rfc3339(t(3_600))),
            "目覚めた時刻が残っていない"
        );
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// 見回りを続けると、**記録・生存信号・時計のずれ**が契機どおりに送られる。
    #[test]
    fn ticks_send_records_beats_and_skew() {
        let cfg = cfg();
        let transport = AcceptAll::default();
        let reference = FixedReference(t(0));
        let mut rt = Runtime::new(
            &cfg,
            zone(),
            Engine::new(Exclusions::default()),
            &transport,
            &reference,
            t(0),
        )
        .unwrap();
        let mut src = FakeSource {
            app: "editor".into(),
        };
        rt.start(t(0)).unwrap();
        // 10 分ぶん、1 分刻みで見回る（送信の契機は 5 分）
        for m in 0..=10 {
            rt.tick(&mut src, t(m * 60)).unwrap();
        }
        let bodies = transport.bodies.borrow();
        let paths: Vec<&str> = bodies.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"/ingest"), "記録が送られていない");
        assert!(paths.contains(&"/heartbeat"), "生存信号が送られていない");
        // 送り切ったら未送信は空（受理された分だけ取り除く）
        assert_eq!(rt.pending(), (0, 0));
        let sent: String = bodies.iter().map(|(_, b)| b.clone()).collect();
        assert!(sent.contains("clock-skew"), "時計のずれが送られていない");
        assert!(sent.contains("foreground"), "前景の記録が送られていない");
        std::fs::remove_dir_all(&cfg.state_dir).ok();
    }

    /// URL の経路が応答しないときだけ、生存信号が `uiautomation` を挙げる。
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
        assert!(!bad.capturable);
        assert_eq!(bad.blockers, [heartbeat::blocker::UIAUTOMATION]);
        let none = capability_of(&Observation {
            at: t(0),
            foreground: None,
            idle: IdleRead::Unavailable,
            locked: false,
        });
        assert_eq!(none.blockers, [heartbeat::blocker::FOREGROUND]);
    }
}
