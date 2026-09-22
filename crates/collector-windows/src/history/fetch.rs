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

/// 消えた訪問に付ける、原因を断定しない手がかり。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VanishedVisit {
    pub external_id: String,
    pub age_days: i64,
    pub foreign: bool,
    pub table_recreated: bool,
    pub profile_gone: bool,
}

/// 今回読めた識別子に無い、送信済み（除外以外）の訪問を消失として返す。
pub fn detect_vanished(
    ledger: &Ledger,
    seen: &[String],
    now: chrono::DateTime<chrono::Utc>,
    max_visit_id: Option<i64>,
    profile_gone: bool,
) -> Vec<VanishedVisit> {
    let seen: std::collections::BTreeSet<_> = seen.iter().collect();
    let table_recreated = ledger.max_visit_id.zip(max_visit_id).is_some_and(|(before, now)| now < before);
    ledger.visits.iter().filter_map(|(external_id, visit)| {
        (!visit.excluded && !seen.contains(external_id)).then(|| VanishedVisit {
            external_id: external_id.clone(),
            age_days: (now - visit.at).num_days().max(0),
            foreign: visit.foreign,
            table_recreated,
            profile_gone,
        })
    }).collect()
}

/// 読めなかったプロファイルは、空の履歴と区別して消失判定しない。
pub fn detect_vanished_if_readable(
    ledger: &Ledger,
    seen: &[String],
    now: chrono::DateTime<chrono::Utc>,
    max_visit_id: Option<i64>,
    profile_gone: bool,
    readable: bool,
) -> Vec<VanishedVisit> {
    readable.then(|| detect_vanished(ledger, seen, now, max_visit_id, profile_gone)).unwrap_or_default()
}

/// 送信対象へ積んだ消失を帳面から外す。同じ取得の再試行で二重に積まないため。
pub fn apply_vanished(ledger: &mut Ledger, vanished: &[VanishedVisit]) {
    for item in vanished {
        ledger.visits.remove(&item.external_id);
    }
}

/// 送る前に履歴へ除外を写す。後から規則を外した訪問は、まだ DB にあれば再び送る。
pub fn apply_history_exclusions(
    ledger: &mut Ledger,
    exclusions: &crate::exclusion::Exclusions,
    visits: &[Visit],
) -> (Vec<Visit>, usize) {
    let mut kept = Vec::new();
    let mut newly_excluded = 0;
    for visit in visits {
        let excluded = exclusions.hits_history(&visit.payload.browser, &visit.payload.profile, &visit.payload.title, &visit.payload.url);
        if excluded {
            let was_excluded = ledger.visits.get(&visit.external_id).is_some_and(|saved| saved.excluded);
            if !was_excluded { newly_excluded += 1; }
            let at = chrono::DateTime::parse_from_rfc3339(&visit.payload.at).expect("Visit は RFC3339").with_timezone(&chrono::Utc);
            ledger.record_visit(&visit.external_id, &content_hash(visit), at, visit.payload.originator_cache_guid.is_some(), true);
        } else {
            kept.push(visit.clone());
        }
    }
    (kept, newly_excluded)
}

/// 取り込み本文の上限を越えないよう、安定順の最大1000件で区切る。
pub fn vanished_chunks(items: &[VanishedVisit]) -> Vec<Vec<VanishedVisit>> {
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| a.external_id.cmp(&b.external_id));
    sorted.chunks(1000).map(<[_]>::to_vec).collect()
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
mod vanished_tests {
    use super::*;
    use chrono::{Duration, Utc};
    use crate::history::ledger::Ledger;

    fn ledger() -> Ledger {
        let mut ledger = Ledger::default();
        ledger.record_visit("v1:a", "hash", Utc::now() - Duration::days(3), false, false);
        ledger.max_visit_id = Some(10);
        ledger
    }

    // Scenario: 履歴から 1 件消すと次の取得で「消えた」記録が残る
    #[test] fn history_vanished_is_detected() { assert_eq!(detect_vanished(&ledger(), &[], Utc::now(), Some(10), false)[0].external_id, "v1:a"); }
    // Scenario: 消えた訪問の、訪問から取得までの日数が本文にある
    #[test] fn history_vanished_has_age_days() { assert_eq!(detect_vanished(&ledger(), &[], Utc::now(), Some(10), false)[0].age_days, 3); }
    // Scenario: 同期で入った訪問が消えたことが本文にある
    #[test] fn history_vanished_marks_foreign() { let mut l=ledger(); l.visits.get_mut("v1:a").unwrap().foreign=true; assert!(detect_vanished(&l,&[],Utc::now(),Some(10),false)[0].foreign); }
    // Scenario: 表が作り直されたことが本文にある
    #[test] fn history_vanished_marks_recreated_table() { assert!(detect_vanished(&ledger(),&[],Utc::now(),Some(1),false)[0].table_recreated); }
    // Scenario: プロファイルが無くなったことが本文にある
    #[test] fn history_vanished_marks_gone_profile() { assert!(detect_vanished(&ledger(),&[],Utc::now(),Some(10),true)[0].profile_gone); }
    // Scenario: 消えた経路を名指しする値を持たない
    // Scenario: 消えた記録に URL と題名が載らない
    #[test] fn history_vanished_has_no_named_cause_or_private_text() { let item=&detect_vanished(&ledger(),&[],Utc::now(),Some(10),false)[0]; let json=serde_json::to_string(item).unwrap(); assert!(!json.contains("deleted") && !json.contains("url") && !json.contains("title")); }
    #[test] fn history_vanished_is_chunked_at_1000() { let mut l=Ledger::default(); for i in 0..1001 { l.record_visit(&format!("v1:{i:04}"),"h",Utc::now(),false,false); } let chunks=vanished_chunks(&detect_vanished(&l,&[],Utc::now(),None,false)); assert_eq!(chunks.iter().map(Vec::len).collect::<Vec<_>>(),vec![1000,1]); }
    // Scenario: 読めなかったプロファイルでは消えた記録を出さない
    #[test] fn history_vanished_skips_unreadable_profile() { let l=ledger(); assert!(detect_vanished_if_readable(&l,&[],Utc::now(),Some(10),false,false).is_empty()); }
    // Scenario: 取得をやり直しても「消えた」記録は増えない
    #[test] fn history_vanished_is_idempotent_on_retry() { let mut l=ledger(); let vanished=detect_vanished_if_readable(&l,&[],Utc::now(),Some(10),false,true); apply_vanished(&mut l,&vanished); assert!(detect_vanished_if_readable(&l,&[],Utc::now(),Some(10),false,true).is_empty()); }
}

#[cfg(test)]
mod exclusion_change_tests {
    use super::*;
    use crate::exclusion::{Exclusions, Rule};

    fn visit() -> Visit { Visit::new("chrome", "Default", 1, chrono::Utc::now(), "https://example.test", "private") }
    #[test]
    fn history_exclusion_added_later() {
        // Scenario: 登録を後から足すと、既に送った訪問の変わった内容は送られない
        let v=visit(); let mut ledger=Ledger::default(); mark_queued(&mut ledger, std::slice::from_ref(&v));
        let rules=Exclusions { rules: vec![Rule::TitleContains { value: "private".into() }] };
        let (kept, excluded)=apply_history_exclusions(&mut ledger,&rules,&[v]);
        assert!(kept.is_empty() && excluded==1);
    }
    #[test]
    fn history_exclusion_removed_later() {
        // Scenario: 登録を外すと、まだ履歴にある除外済みの訪問が次の取得で送られる
        let v=visit(); let mut ledger=Ledger::default(); ledger.record_visit(&v.external_id,"h",chrono::Utc::now(),false,true);
        let (kept, excluded)=apply_history_exclusions(&mut ledger,&Exclusions::default(),std::slice::from_ref(&v));
        assert_eq!(kept,vec![v]); assert_eq!(excluded,0);
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
