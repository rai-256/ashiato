// SPDX-License-Identifier: AGPL-3.0-only
//! 時計のずれを測って残す（FR-7 / 扉 #5 / design D6）。
//!
//! 記録の時刻（`event_time`）は PC の時計そのもので、**冪等キーの入力でもある**。
//! 扉 #5 は「常に正しい時刻を確保するはオフライン時に必ず破れ、**破れたことを
//! 後から知る手段が無い**」と決着している。**測らなかった期間のずれは後から作れない。**
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::contract::ClockTrigger;

/// 測る間隔（spec「1 時間ごと」）。
pub const SKEW_INTERVAL_SEC: i64 = 3_600;

/// OS が起動してからの経過時間（design D9）。スリープの間も進む。
pub trait Uptime: std::fmt::Debug + Send + Sync {
    /// 起動からの経過時間（ミリ秒）。
    fn millis(&self) -> u64;

    /// 刻み（ミリ秒）。読んだ値は本当の値より最大 `刻み - 1` 小さい（切り捨て）。
    fn resolution_ms(&self) -> u64 {
        1
    }
}

/// 本番: Windows の起動からの経過時間（sysinfo が安全に包んでいる。**unsafe を書かない**ため
/// `GetTickCount64` は直接呼ばない）。**刻みは秒**（D9（仮）: ミリ秒でなく、幅を上限側に寄せる）。
#[cfg(windows)]
#[derive(Debug, Default)]
pub struct SystemUptime;

#[cfg(windows)]
impl Uptime for SystemUptime {
    fn millis(&self) -> u64 {
        sysinfo::System::uptime().saturating_mul(1000)
    }

    fn resolution_ms(&self) -> u64 {
        1000
    }
}

/// 基準を 1 回読んだ結果（design D7）。**差は `wall_after` で計算する**（見回りの先頭の壁時計は使わない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceReading {
    /// 基準の時刻。
    pub time: DateTime<Utc>,
    /// 読む直前の起動からの経過時間（ミリ秒）。
    pub uptime_before_ms: u64,
    /// 読んだ直後の起動からの経過時間（ミリ秒）。
    pub uptime_after_ms: u64,
    /// 応答を受け取った直後の PC の壁時計。
    pub wall_after: DateTime<Utc>,
}

/// 基準の時刻をくれるもの。**PC の外から**取る（PC の時計と比べるため）。
/// 作業スレッドへ渡す（design D7）ので `Send + Sync`。
pub trait ReferenceClock: std::fmt::Debug + Send + Sync {
    /// 基準を読む。取れなければ `Err`（**推測で埋めない**）。
    fn now(&self) -> anyhow::Result<ReferenceReading>;

    /// 基準の出どころ（記録に載せる。R15）。
    fn source(&self) -> String;
}

/// 測れなかったときに、次に測り直すまでの間隔（R16 / I8）。
/// **毎秒叩かない** —— 取り込み口が止まっている間、毎秒ログが 1 行ずつ出ていた。
pub const SKEW_RETRY_SEC: i64 = 60;

/// 測る契機。
///
/// 1 時間ごとの契機を **1 周期**とし、周期の中で取れなかった記録は 1 件までにする（design D4 / D10）。
/// 測り直し（60 秒）は周期の中で続き、取れたら `retry` の記録を 1 件残す。周期が 1 時間を過ぎたら新しい周期にする。
#[derive(Debug, Clone)]
pub struct SkewSchedule {
    interval: Duration,
    last: Option<DateTime<Utc>>,
    /// 次に測るときの契機（起動・飛び・戻りで立て、測り始めたら `Hourly` に戻る）
    pending: ClockTrigger,
    /// いまの周期の始まり
    cycle_start: Option<DateTime<Utc>>,
    /// 取れなかった契機のあと、測り直している最中
    retrying: bool,
    /// いまの周期で、取れなかった記録を残したか
    unavailable_recorded: bool,
}

impl SkewSchedule {
    /// 1 時間ごと。最初の契機は起動。
    pub fn new() -> Self {
        Self::with_interval(Duration::seconds(SKEW_INTERVAL_SEC))
    }

    /// 間隔を明示して作る（テスト用）。
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            interval,
            last: None,
            pending: ClockTrigger::Start,
            cycle_start: None,
            retrying: false,
            unavailable_recorded: false,
        }
    }

    /// いま測る契機か。
    pub fn due(&self, now: DateTime<Utc>) -> bool {
        self.last.is_none_or(|last| now - last >= self.interval)
    }

    /// 測ったことを覚える。
    pub fn mark(&mut self, now: DateTime<Utc>) {
        self.last = Some(now);
    }

    /// 測り始める。**どの契機の測定かを返す**（測り直しの間は `Retry`、周期が変われば起動・飛び・1 時間ごと）。
    pub fn begin(&mut self, now: DateTime<Utc>) -> ClockTrigger {
        self.mark(now);
        let in_cycle = self.cycle_start.is_some_and(|s| now - s < self.interval);
        if self.retrying && in_cycle {
            return ClockTrigger::Retry;
        }
        self.cycle_start = Some(now);
        self.retrying = false;
        self.unavailable_recorded = false;
        std::mem::replace(&mut self.pending, ClockTrigger::Hourly)
    }

    /// 起動・壁時計の飛び・戻りで、次の見回りにすぐ測らせる。
    pub fn reset(&mut self, trigger: ClockTrigger) {
        self.last = None;
        self.pending = trigger;
        self.cycle_start = None;
        self.retrying = false;
        self.unavailable_recorded = false;
    }

    /// 測れなかった。**`SKEW_RETRY_SEC` 後にもう一度**測る。
    pub fn failed(&mut self, now: DateTime<Utc>) {
        self.last = Some(now - self.interval + Duration::seconds(SKEW_RETRY_SEC));
        self.retrying = true;
    }

    /// 取れた。測り直しを終える。
    pub fn succeeded(&mut self) {
        self.retrying = false;
    }

    /// 取れなかった記録をいま残すか。**周期に 1 件まで**（測り直しのたびには増やさない）。
    pub fn record_unavailable(&mut self) -> bool {
        !std::mem::replace(&mut self.unavailable_recorded, true)
    }
}

impl Default for SkewSchedule {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP の `date` ヘッダを読む。
pub fn parse_http_date(value: &str) -> anyhow::Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc2822(value)?.with_timezone(&Utc))
}

/// 基点 URL から `host:port` を取り出す（記録に載せる基準の出どころ）。
pub fn host_of(base_url: &str) -> String {
    let rest = base_url.split_once("://").map_or(base_url, |(_, r)| r);
    rest.split('/').next().unwrap_or(rest).to_string()
}

/// 取り込み口の HTTP 応答の `date` ヘッダを基準にする（design D17）。
///
/// **刻みは 1 秒**（HTTP の日付の形がそれしか持たない）。扉 #5 が求めるのは
/// 「破れたことを後から知る」ことなので、秒の分解能で足りる。
pub struct HttpDateClock {
    url: String,
    source: String,
    agent: ureq::Agent,
    uptime: Arc<dyn Uptime>,
}

impl std::fmt::Debug for HttpDateClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpDateClock")
            .field("url", &self.url)
            .finish()
    }
}

impl HttpDateClock {
    /// 取り込み口の生存確認の口（`/healthz`）を叩く。**合言葉を要しない口**を使う。
    pub fn new(base_url: &str, uptime: Arc<dyn Uptime>) -> Self {
        Self {
            url: format!("{}/healthz", base_url.trim_end_matches('/')),
            source: host_of(base_url),
            // **timeout を持つ**（見回りの輪の中で呼ぶ。R22）
            agent: crate::sender::agent(),
            uptime,
        }
    }
}

impl ReferenceClock for HttpDateClock {
    fn now(&self) -> anyhow::Result<ReferenceReading> {
        let uptime_before_ms = self.uptime.millis();
        let res = self
            .agent
            .get(&self.url)
            .call()
            .map_err(|e| anyhow::anyhow!("基準時刻を取れない: {}", e))?;
        // **応答を受け取った直後**に読む（Q3 ②）。差にはこの壁時計を使う
        let wall_after = Utc::now();
        // 刻みで切り捨てた分を直後の側へ足し、幅を**実際の読み取り時間の上限**にする（D9（仮））
        let uptime_after_ms = self.uptime.millis() + self.uptime.resolution_ms() - 1;
        let date = res
            .headers()
            .get("date")
            .ok_or_else(|| anyhow::anyhow!("応答に date が無い"))?
            .to_str()?
            .to_string();
        Ok(ReferenceReading {
            time: parse_http_date(&date)?,
            uptime_before_ms,
            uptime_after_ms,
            wall_after,
        })
    }

    fn source(&self) -> String {
        self.source.clone()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn t(sec: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-13T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::seconds(sec)
    }

    /// **1 時間ごとに測り、ずれを記録に残す**（tasks 8b.2 / design D6）。
    ///
    /// Scenario: 1 時間ごとにずれの測定記録が残る
    #[test]
    fn clock_skew_is_measured() {
        use crate::clock_record::{skew_record, RecordContext};
        use crate::clock_worker::ClockReading;
        use crate::contract::{rfc3339, RecordKind};

        let mut s = SkewSchedule::new();
        assert!(s.due(t(0)), "起動直後に 1 回も測らない");
        assert_eq!(s.begin(t(0)), ClockTrigger::Start, "最初の契機は起動");
        assert!(!s.due(t(3_599)), "1 時間より早く測っている");
        assert!(s.due(t(3_600)), "1 時間経っても測らない");
        assert_eq!(s.begin(t(3_600)), ClockTrigger::Hourly);

        // PC の時計が 1.2 秒進んでいる
        let reading = ClockReading {
            source: "127.0.0.1:8787".into(),
            reference: Ok(ReferenceReading {
                time: t(3_600) - Duration::milliseconds(1_200),
                uptime_before_ms: 10,
                uptime_after_ms: 11,
                wall_after: t(3_600),
            }),
            time_sync: Err("spawn_failed".into()),
        };
        let ctx = RecordContext {
            trigger: ClockTrigger::Hourly,
            at: t(3_600),
            uptime_ms: 3_600_000,
            boot_at: None,
        };
        let p = skew_record(&ctx, reading);
        assert_eq!(p.kind, RecordKind::ClockSkew);
        assert_eq!(p.skew_ms, Some(1_200));
        assert_eq!(p.skew_reference.as_deref(), Some("127.0.0.1:8787"));
        assert_eq!(p.at, rfc3339(t(3_600)));

        // 1 日動かし続けると 24 件（起動直後の 1 件 + 1 時間ごと。24 時間目は翌日に入る）
        let mut s = SkewSchedule::new();
        let mut n = 0;
        for sec in (0..86_400).step_by(60) {
            if s.due(t(sec)) {
                s.begin(t(sec));
                n += 1;
            }
        }
        assert_eq!(n, 24);
    }

    /// 測れなかったら 1 分後に測り直す（毎秒叩かない。I8）。
    #[test]
    fn skew_failure_backs_off() {
        let mut s = SkewSchedule::new();
        s.failed(t(0));
        assert!(
            !s.due(t(SKEW_RETRY_SEC - 1)),
            "測れなかった直後にまた叩いている"
        );
        assert!(s.due(t(SKEW_RETRY_SEC)));
    }

    /// HTTP の日付と基点 URL の読み方。
    #[test]
    fn http_date_and_host_are_parsed() {
        assert_eq!(
            parse_http_date("Sat, 12 Sep 2026 16:34:04 GMT").unwrap(),
            DateTime::parse_from_rfc3339("2026-09-12T16:34:04Z")
                .unwrap()
                .with_timezone(&Utc)
        );
        assert!(parse_http_date("きのう").is_err());
        assert_eq!(host_of("http://127.0.0.1:8787/"), "127.0.0.1:8787");
        assert_eq!(host_of("http://s01.lan:8787/base"), "s01.lan:8787");
    }

    // ---- 基準を読む（clock_reference_*） ----

    use std::io::{Read, Write};
    use std::sync::Mutex;
    use std::time::Instant;

    /// 起動からの経過時間の偽物。作ってからの実時間を返す。
    #[derive(Debug)]
    struct ElapsedUptime(Instant);

    impl Uptime for ElapsedUptime {
        fn millis(&self) -> u64 {
            u64::try_from(self.0.elapsed().as_millis()).unwrap()
        }
    }

    /// `date` に指定の時刻を入れて返す偽の取り込み口。受けた要求の 1 行目を数える。
    fn fake_ingest(date: DateTime<Utc>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&requests);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let mut head = Vec::new();
                let mut buf = [0u8; 512];
                while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => head.extend_from_slice(&buf[..n]),
                    }
                }
                let first = String::from_utf8_lossy(&head)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                seen.lock().unwrap().push(first);
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nDate: {}\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                    date.format("%a, %d %b %Y %H:%M:%S GMT")
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        (base, requests)
    }

    /// 基準を 1 回読んで、PC の差（ミリ秒）と、読む直前と直後の経過時間の幅を返す。
    fn skew_against(reference: DateTime<Utc>) -> (i64, u64) {
        let (base, _) = fake_ingest(reference);
        let clock = HttpDateClock::new(&base, Arc::new(ElapsedUptime(Instant::now())));
        let r = clock.now().unwrap();
        (
            (r.wall_after - r.time).num_milliseconds(),
            r.uptime_after_ms - r.uptime_before_ms,
        )
    }

    /// 刻みが粗い経過時間でも、幅は実際の読み取り時間を下回らない。
    #[test]
    fn clock_reference_width_covers_coarse_uptime_resolution() {
        #[derive(Debug)]
        struct Coarse;
        impl Uptime for Coarse {
            fn millis(&self) -> u64 {
                5_000
            }
            fn resolution_ms(&self) -> u64 {
                1_000
            }
        }
        let (base, _) = fake_ingest(Utc::now());
        let r = HttpDateClock::new(&base, Arc::new(Coarse)).now().unwrap();
        assert_eq!(r.uptime_before_ms, 5_000);
        assert_eq!(r.uptime_after_ms, 5_999);
    }

    /// Scenario: PC の時計が進んでいると差が正で残る
    #[test]
    fn clock_reference_pc_ahead_gives_positive_difference() {
        let (skew, width) = skew_against(Utc::now() - Duration::minutes(5));
        assert!(
            (300_000..301_000 + i64::try_from(width).unwrap()).contains(&skew),
            "skew={skew} width={width}"
        );
    }

    /// Scenario: PC の時計が遅れていると差が負で残る
    #[test]
    fn clock_reference_pc_behind_gives_negative_difference() {
        let (skew, width) = skew_against(Utc::now() + Duration::minutes(5));
        assert!(
            (-300_000 - i64::try_from(width).unwrap()..-299_000 + i64::try_from(width).unwrap())
                .contains(&skew),
            "skew={skew} width={width}"
        );
    }

    /// 壁時計は応答を受け取った直後に読み、経過時間はその前後を挟む。
    #[test]
    fn clock_reference_reads_wall_between_the_uptime_readings() {
        let (base, _) = fake_ingest(Utc::now());
        let uptime = Arc::new(ElapsedUptime(Instant::now()));
        let clock = HttpDateClock::new(&base, uptime.clone());
        let (before, started) = (Utc::now(), uptime.millis());
        let r = clock.now().unwrap();
        let (after, finished) = (Utc::now(), uptime.millis());
        assert!(r.wall_after >= before && r.wall_after <= after);
        assert!(started <= r.uptime_before_ms && r.uptime_before_ms <= r.uptime_after_ms);
        assert!(r.uptime_after_ms <= finished);
    }

    /// Scenario: 測るための要求は取り込み口の生存確認の 1 本だけ
    #[test]
    fn clock_reference_asks_the_ingest_health_check_once() {
        let (base, requests) = fake_ingest(Utc::now());
        let clock = HttpDateClock::new(&base, Arc::new(ElapsedUptime(Instant::now())));
        clock.now().unwrap();
        let seen = requests.lock().unwrap().clone();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(seen[0].starts_with("GET /healthz "), "{seen:?}");
    }
}
