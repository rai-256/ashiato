// SPDX-License-Identifier: AGPL-3.0-only
//! 読取り結果を未送信と帳面へ反映する取得。
use sha2::{Digest as _, Sha256};

use crate::history::contract::Visit;
use crate::history::ledger::{Ledger, LedgerStore};

pub const HISTORY_INTERVAL: chrono::Duration = chrono::Duration::hours(24);
pub const HISTORY_RETRY_INTERVAL: chrono::Duration = chrono::Duration::minutes(1);

/// 履歴取得の契機。成功だけが24時間の基準を進め、失敗は1分後に再試行する。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HistorySchedule {
    last_success: Option<chrono::DateTime<chrono::Utc>>,
    retry_after: Option<chrono::DateTime<chrono::Utc>>,
}

/// 時間のかかる DB 読取りを見回りのスレッドから切り離す受け皿。
#[derive(Debug)]
pub struct HistoryWorker<T> {
    handle: std::thread::JoinHandle<anyhow::Result<T>>,
}

impl<T: Send + 'static> HistoryWorker<T> {
    pub fn spawn(read: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Self {
        Self { handle: std::thread::spawn(read) }
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub fn join(self) -> anyhow::Result<T> {
        self.handle.join().map_err(|_| anyhow::anyhow!("履歴読取りworkerがpanicした"))?
    }
}

impl HistorySchedule {
    pub fn with_last_success(last_success: Option<chrono::DateTime<chrono::Utc>>) -> Self {
        Self { last_success, retry_after: None }
    }

    pub fn due(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        if let Some(retry) = self.retry_after {
            return now >= retry;
        }
        self.last_success.is_none_or(|last| now - last >= HISTORY_INTERVAL)
    }

    pub fn succeeded(&mut self, now: chrono::DateTime<chrono::Utc>) {
        self.last_success = Some(now);
        self.retry_after = None;
    }

    pub fn failed(&mut self, now: chrono::DateTime<chrono::Utc>) {
        self.retry_after = Some(now + HISTORY_RETRY_INTERVAL);
    }

    pub fn last_success(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.last_success
    }
}

/// 全履歴と帳面を比べ、未送信または本文が変わった訪問だけを返す。
///
/// 訪問時刻で切らない。同期は過去の時刻の訪問を後から加えるため、全件との比較が
/// 唯一「まだ送っていない」を保てる。
pub fn select_new_or_changed(ledger: &Ledger, visits: &[Visit]) -> Vec<Visit> {
    visits
        .iter()
        .filter(|visit| {
            ledger
                .visits
                .get(&visit.external_id)
                .is_none_or(|saved| saved.content_hash != content_hash(visit))
        })
        .cloned()
        .collect()
}

/// 未送信への追記が成功した訪問を帳面へ反映する。
///
/// 呼ぶ側は outbox への全件追記に成功してから `LedgerStore::save` する。そうしないと
/// 送る前に落ちた訪問を「送った」と誤認して次回の取得で失う。
pub fn mark_queued(ledger: &mut Ledger, visits: &[Visit]) {
    for visit in visits {
        let at = chrono::DateTime::parse_from_rfc3339(&visit.payload.at)
            .expect("Visit::new が RFC3339 マイクロ秒を作る")
            .with_timezone(&chrono::Utc);
        ledger.record_visit(
            &visit.external_id,
            &content_hash(visit),
            at,
            visit.payload.originator_cache_guid.is_some(),
            false,
        );
    }
}

/// 未送信への追記を完了してから帳面を更新する。
///
/// 取り込み口が止まっていても outbox はローカルに積める。ここで失敗した場合は
/// 帳面を進めず、次の取得で同じ訪問を再び差分として扱う。
pub fn queue_then_save(
    store: &mut LedgerStore,
    visits: &[Visit],
    mut queue: impl FnMut(&Visit) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    for visit in visits {
        queue(visit)?;
    }
    mark_queued(store.ledger_mut(), visits);
    store.save()
}

fn content_hash(visit: &Visit) -> String {
    let bytes = serde_json::to_vec(&visit.payload).expect("VisitPayload は直列化できる");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod schedule_tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn at(seconds: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.timestamp_opt(seconds, 0).unwrap()
    }

    /// Scenario: 起動時に前回の成功から 24 時間以上経っていれば取得する
    /// Scenario: 前回の成功から 24 時間経たないうちは取得しない
    /// Scenario: 動作中に前回の成功から 24 時間経つと取得する
    #[test]
    fn history_schedule_uses_success_and_24_hours() {
        let mut schedule = HistorySchedule::with_last_success(Some(at(0)));
        assert!(!schedule.due(at(86_399)));
        assert!(schedule.due(at(86_400)));
        schedule.succeeded(at(86_400));
        assert!(!schedule.due(at(86_400 + 60)));
        assert!(schedule.due(at(86_400 + Duration::days(1).num_seconds())));
    }

    #[test]
    fn history_schedule_starts_when_no_success_was_saved() {
        assert!(HistorySchedule::default().due(at(0)));
    }

    #[test]
    fn history_schedule_retries_failure_after_one_minute_without_advancing_success() {
        let mut schedule = HistorySchedule::with_last_success(Some(at(0)));
        schedule.failed(at(86_400));
        assert!(!schedule.due(at(86_459)));
        assert!(schedule.due(at(86_460)));
        assert_eq!(schedule.last_success(), Some(at(0)));
    }
}

#[cfg(test)]
mod outbox_tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::history::ledger::LedgerStore;

    /// Scenario: 取り込み口が止まっている間に取得した履歴が後から届く
    #[test]
    fn history_success_only_after_outbox() {
        let dir = std::env::temp_dir().join(format!("ashiato-history-fetch-{}", uuid::Uuid::new_v4()));
        let path = dir.join("ledger");
        let mut store = LedgerStore::open(path.clone()).unwrap();
        let visit = Visit::new("chrome", "Default", 1, chrono::Utc::now(), "https://example.test", "題名");

        assert!(queue_then_save(&mut store, &[visit.clone()], |_| anyhow::bail!("取り込み口が止まっている")).is_err());
        assert!(store.ledger().visits.is_empty(), "未送信へ積めないのに成功扱いにしている");

        queue_then_save(&mut store, &[visit.clone()], |_| Ok(())).unwrap();
        let reopened = LedgerStore::open(path).unwrap();
        assert!(reopened.ledger().visits.contains_key(&visit.external_id));
        std::fs::remove_dir_all(dir).ok();
    }
}

#[cfg(test)]
mod worker_tests {
    use super::*;
    use std::sync::mpsc;

    /// Scenario: 履歴の取得でウィンドウのソースの記録は増えない
    #[test]
    fn history_slow_read_does_not_disturb_window() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = HistoryWorker::spawn(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok::<_, anyhow::Error>(42)
        });
        started_rx.recv().unwrap();

        // 読み手が止まっていても、見回り側は自分の仕事を続けられる。
        let mut polls = 0;
        for _ in 0..3 {
            polls += 1;
            assert!(!worker.is_finished());
        }
        assert_eq!(polls, 3);
        release_tx.send(()).unwrap();
        assert_eq!(worker.join().unwrap(), 42);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::history::contract::Visit;
    use crate::history::ledger::Ledger;

    fn visit(id: i64, at: &str, title: &str) -> Visit {
        Visit::new(
            "chrome",
            "Default",
            id,
            chrono::DateTime::parse_from_rfc3339(at).unwrap().with_timezone(&chrono::Utc),
            "https://example.test/page",
            title,
        )
    }

    /// Scenario: 初回の取得で過去の履歴が入る
    /// Scenario: 前回の取得の後に古い時刻で入った訪問も取り込まれる
    #[test]
    fn history_fetch_sends_only_new_or_changed() {
        let mut ledger = Ledger::default();
        let old = visit(1, "2026-09-01T00:00:00Z", "古い訪問");
        assert_eq!(select_new_or_changed(&ledger, &[old.clone()]), vec![old.clone()]);
        mark_queued(&mut ledger, &[old.clone()]);
        assert!(select_new_or_changed(&ledger, &[old.clone()]).is_empty());

        // 取得時刻ではなく、全履歴と帳面を比べる。同期で古い訪問が後から来ても落とさない。
        let late_old = visit(2, "2025-01-01T00:00:00Z", "後から同期された訪問");
        assert_eq!(select_new_or_changed(&ledger, &[old.clone(), late_old.clone()]), vec![late_old]);

        let changed = visit(1, "2026-09-01T00:00:00Z", "後から変わった題名");
        assert_eq!(select_new_or_changed(&ledger, &[changed.clone()]), vec![changed]);
    }
}
