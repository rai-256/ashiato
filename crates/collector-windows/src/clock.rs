// SPDX-License-Identifier: AGPL-3.0-only
//! 時計のずれを測って残す（FR-7 / 扉 #5 / design D6）。
//!
//! 記録の時刻（`event_time`）は PC の時計そのもので、**冪等キーの入力でもある**。
//! 扉 #5 は「常に正しい時刻を確保するはオフライン時に必ず破れ、**破れたことを
//! 後から知る手段が無い**」と決着している。**測らなかった期間のずれは後から作れない。**
use chrono::{DateTime, Duration, Utc};

use crate::contract::{RecordKind, WindowPayload};

/// 測る間隔（spec「1 時間ごと」）。
pub const SKEW_INTERVAL_SEC: i64 = 3_600;

/// 基準の時刻をくれるもの。**PC の外から**取る（PC の時計と比べるため）。
pub trait ReferenceClock: std::fmt::Debug {
    /// 基準時刻。取れなければ `Err`（**推測で埋めない**）。
    fn now(&self) -> anyhow::Result<DateTime<Utc>>;

    /// 基準の出どころ（記録に載せる。R15）。
    fn source(&self) -> String;
}

/// 測れなかったときに、次に測り直すまでの間隔（R16 / I8）。
/// **毎秒叩かない** —— 取り込み口が止まっている間、毎秒ログが 1 行ずつ出ていた。
pub const SKEW_RETRY_SEC: i64 = 60;

/// 測る契機。
#[derive(Debug, Clone)]
pub struct SkewSchedule {
    interval: Duration,
    last: Option<DateTime<Utc>>,
}

impl SkewSchedule {
    /// 1 時間ごと。
    pub fn new() -> Self {
        Self {
            interval: Duration::seconds(SKEW_INTERVAL_SEC),
            last: None,
        }
    }

    /// 間隔を明示して作る（テスト用）。
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            interval,
            last: None,
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

    /// 測れなかった。**`SKEW_RETRY_SEC` 後にもう一度**測る。
    pub fn failed(&mut self, now: DateTime<Utc>) {
        self.last = Some(now - self.interval + Duration::seconds(SKEW_RETRY_SEC));
    }
}

impl Default for SkewSchedule {
    fn default() -> Self {
        Self::new()
    }
}

/// 測定の記録。**正なら PC の時計が進んでいる。**
///
/// `source` は基準の出どころ（`host:port`）。ループバックなら「自分の時計と比べた 0」だと
/// 後から分かる（R15）。HTTP の日付は**秒で切り捨て**なので、同じ PC でも
/// 0〜+999 ms に偏る（design D17）。
pub fn measure(local: DateTime<Utc>, reference: DateTime<Utc>, source: &str) -> WindowPayload {
    let mut p = WindowPayload::new(RecordKind::ClockSkew, local);
    p.skew_ms = Some((local - reference).num_milliseconds());
    p.skew_reference = Some(source.to_string());
    p
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
    pub fn new(base_url: &str) -> Self {
        Self {
            url: format!("{}/healthz", base_url.trim_end_matches('/')),
            source: host_of(base_url),
            // **timeout を持つ**（見回りの輪の中で呼ぶ。R22）
            agent: crate::sender::agent(),
        }
    }
}

impl ReferenceClock for HttpDateClock {
    fn now(&self) -> anyhow::Result<DateTime<Utc>> {
        let res = self
            .agent
            .get(&self.url)
            .call()
            .map_err(|e| anyhow::anyhow!("基準時刻を取れない: {}", e))?;
        let date = res
            .headers()
            .get("date")
            .ok_or_else(|| anyhow::anyhow!("応答に date が無い"))?
            .to_str()?
            .to_string();
        parse_http_date(&date)
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
        let mut s = SkewSchedule::new();
        assert!(s.due(t(0)), "起動直後に 1 回も測らない");
        s.mark(t(0));
        assert!(!s.due(t(3_599)), "1 時間より早く測っている");
        assert!(s.due(t(3_600)), "1 時間経っても測らない");

        // PC の時計が 1.2 秒進んでいる
        let p = measure(
            t(3_600),
            t(3_600) - Duration::milliseconds(1_200),
            "127.0.0.1:8787",
        );
        assert_eq!(p.kind, RecordKind::ClockSkew);
        assert_eq!(p.skew_ms, Some(1_200));
        assert_eq!(p.skew_reference.as_deref(), Some("127.0.0.1:8787"));
        assert_eq!(p.at, crate::contract::rfc3339(t(3_600)));
        // 遅れている側も測れる（符号で向きが分かる）
        assert_eq!(
            measure(t(0), t(0) + Duration::milliseconds(500), "h").skew_ms,
            Some(-500)
        );

        // 1 日動かし続けると 24 件（起動直後の 1 件 + 1 時間ごと。24 時間目は翌日に入る）
        let mut s = SkewSchedule::new();
        let mut n = 0;
        for sec in (0..86_400).step_by(60) {
            if s.due(t(sec)) {
                s.mark(t(sec));
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
}
