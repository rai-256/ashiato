// SPDX-License-Identifier: AGPL-3.0-only
//! 読取り結果を未送信と帳面へ反映する取得。
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::cloned_ref_to_slice_refs))]
use sha2::{Digest as _, Sha256};

use crate::history::contract::Visit;
use crate::history::ledger::{Ledger, LedgerStore};

pub const HISTORY_INTERVAL: chrono::Duration = chrono::Duration::hours(24);
pub const HISTORY_RETRY_INTERVAL: chrono::Duration = chrono::Duration::minutes(1);
/// 続けて失敗したときの試し直しの間隔の上限（design D3（仮））。24 時間の契機より短く保つ
pub const HISTORY_RETRY_INTERVAL_MAX: chrono::Duration = chrono::Duration::hours(1);

/// 履歴取得の契機。成功だけが24時間の基準を進め、失敗は1分後に再試行する。
/// 続けて失敗するたびに間隔を倍にし、1 時間で止める（design D3（仮）。R59 —— 読めないプロファイルが
/// 1 つ残り続けると、全プロファイルの写しと全件読みが 1 日 1,440 回走る）。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HistorySchedule {
    last_success: Option<chrono::DateTime<chrono::Utc>>,
    retry_after: Option<chrono::DateTime<chrono::Utc>>,
    /// 前回の成功から続けて失敗した回数
    #[serde(default)]
    failures: u32,
}

/// 時間のかかる DB 読取りを見回りのスレッドから切り離す受け皿。
#[derive(Debug)]
pub struct HistoryWorker<T> {
    handle: std::thread::JoinHandle<anyhow::Result<T>>,
}

impl<T: Send + 'static> HistoryWorker<T> {
    pub fn spawn(read: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Self {
        Self {
            handle: std::thread::spawn(read),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub fn join(self) -> anyhow::Result<T> {
        self.handle
            .join()
            .map_err(|_| anyhow::anyhow!("履歴読取りworkerがpanicした"))?
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
///
/// 表の作り直しは、最大の訪問番号が前回より小さいか、Chromium の `sqlite_sequence` の値が前回より小さいときに印を付ける
/// （design D10。最大の番号が下がらない作り直しは `sqlite_sequence` でだけ分かる）。
pub fn detect_vanished(
    ledger: &Ledger,
    seen: &[String],
    now: chrono::DateTime<chrono::Utc>,
    max_visit_id: Option<i64>,
    sequence: Option<i64>,
    profile_gone: bool,
) -> Vec<VanishedVisit> {
    let seen: std::collections::BTreeSet<_> = seen.iter().collect();
    let dropped = |before: Option<i64>, now: Option<i64>| {
        before.zip(now).is_some_and(|(before, now)| now < before)
    };
    // 最大の番号が無い（全件が消えた）のは作り直しの印ではない。ディレクトリごと無いときも表の話ではない
    let table_recreated = !profile_gone
        && (dropped(ledger.max_visit_id, max_visit_id) || dropped(ledger.visit_sequence, sequence));
    ledger
        .visits
        .iter()
        .filter(|(external_id, visit)| !visit.excluded && !seen.contains(external_id))
        .map(|(external_id, visit)| VanishedVisit {
            external_id: external_id.clone(),
            age_days: (now - visit.at).num_days().max(0),
            foreign: visit.foreign,
            table_recreated,
            profile_gone,
        })
        .collect()
}

/// 読めなかったプロファイルは、空の履歴と区別して消失判定しない。
pub fn detect_vanished_if_readable(
    ledger: &Ledger,
    seen: &[String],
    now: chrono::DateTime<chrono::Utc>,
    max_visit_id: Option<i64>,
    sequence: Option<i64>,
    profile_gone: bool,
    readable: bool,
) -> Vec<VanishedVisit> {
    if readable {
        detect_vanished(ledger, seen, now, max_visit_id, sequence, profile_gone)
    } else {
        Vec::new()
    }
}

/// URL を除いた訪問の組（family・browser・profile_dir・番号・訪問時刻）のハッシュ。
///
/// 識別子（design D6）は URL を組に含むので、URL の行を失った訪問は別の識別子になる。同じ組の訪問が
/// 今回の読みにあれば、前の識別子の訪問は「消えた」ではない（deep.md 第 4 回 Q8）。
pub fn slot(visit: &Visit) -> String {
    let p = &visit.payload;
    let parts = [
        p.family.to_owned(),
        p.browser.to_owned(),
        p.profile_dir.clone().unwrap_or_default(),
        p.visit_id.map(|v| v.to_string()).unwrap_or_default(),
        p.visit_time_raw.map(|v| v.to_string()).unwrap_or_default(),
    ];
    format!("{:x}", Sha256::digest(parts.join("\x1f").as_bytes()))
}

/// 今回の読みに「まだある」識別子。読んだ訪問の識別子と、帳面にあって**同じ組の訪問が今回の読みにある**識別子
/// （URL の行を失って識別子が変わった訪問。deep.md 第 4 回 Q8 —— 「消えた」とは記録しない）。
pub fn still_present(ledger: &Ledger, visits: &[Visit]) -> Vec<String> {
    let slots: std::collections::BTreeSet<String> = visits.iter().map(slot).collect();
    visits
        .iter()
        .map(|v| v.external_id.clone())
        .chain(
            ledger
                .visits
                .iter()
                .filter(|(_, saved)| saved.slot.as_ref().is_some_and(|s| slots.contains(s)))
                .map(|(id, _)| id.clone()),
        )
        .collect()
}

/// 帳面に 1 件書く。訪問時刻と組のハッシュは `Visit` から取る。
fn record(ledger: &mut Ledger, visit: &Visit, excluded: bool) {
    ledger.record_visit(
        &visit.external_id,
        &content_hash(visit),
        visit.at,
        visit.payload.originator_cache_guid.is_some(),
        excluded,
    );
    if let Some(saved) = ledger.visits.get_mut(&visit.external_id) {
        saved.slot = Some(slot(visit));
    }
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
) -> (Vec<Visit>, Vec<String>) {
    let mut kept = Vec::new();
    let mut newly_excluded = Vec::new();
    for visit in visits {
        let excluded = exclusions.hits_history(
            visit.payload.browser,
            visit.payload.profile_dir.as_deref().unwrap_or_default(),
            visit.payload.title.as_deref().unwrap_or_default(),
            visit.payload.url.as_deref().unwrap_or_default(),
        );
        if excluded {
            let was_excluded = ledger
                .visits
                .get(&visit.external_id)
                .is_some_and(|saved| saved.excluded);
            if !was_excluded {
                newly_excluded.push(visit.external_id.clone());
            }
            record(ledger, visit, true);
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
        Self {
            last_success,
            retry_after: None,
            failures: 0,
        }
    }

    /// 今の失敗の回数での試し直しの間隔（1 分 → 2 分 → 4 分 … 1 時間）。
    fn retry_interval(&self) -> chrono::Duration {
        let doublings = self.failures.saturating_sub(1).min(6);
        (HISTORY_RETRY_INTERVAL * 2_i32.pow(doublings)).min(HISTORY_RETRY_INTERVAL_MAX)
    }

    /// 取得の契機に達したか（design D3）。
    ///
    /// **前回の成功が今より先なら（時計が戻った）、24 時間を待たずにすぐ取得する**（deep.md 第 4 回 Q7）。
    /// 待つと戻った幅だけ取得が止まり、Chromium は 90 日で消すのでその間の訪問を後から取れない。
    /// 試し直しの時刻も同じで、今より試し直しの間隔を超えて先にあれば時計が戻ったと読む。
    pub fn due(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        if let Some(retry) = self.retry_after {
            return now >= retry || retry - now > self.retry_interval();
        }
        self.last_success
            .is_none_or(|last| last > now || now - last >= HISTORY_INTERVAL)
    }

    pub fn succeeded(&mut self, now: chrono::DateTime<chrono::Utc>) {
        self.last_success = Some(now);
        self.retry_after = None;
        self.failures = 0;
    }

    pub fn failed(&mut self, now: chrono::DateTime<chrono::Utc>) {
        self.failures = self.failures.saturating_add(1);
        self.retry_after = Some(now + self.retry_interval());
    }

    pub fn last_success(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.last_success
    }
}

/// 全履歴と帳面を比べ、未送信または本文が変わった訪問だけを返す。
/// 帳面が「除外した」と覚えている訪問がここへ来たなら、除外が外れたので送る。
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
                .is_none_or(|saved| saved.excluded || saved.content_hash != content_hash(visit))
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
        record(ledger, visit, false);
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
    format!("{:x}", Sha256::digest(visit.raw.as_bytes()))
}

#[cfg(test)]
mod schedule_tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn at(seconds: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.timestamp_opt(seconds, 0).unwrap()
    }

    // 契機の Scenario の印は、Runtime を通す `runtime::tests::history_schedule_runtime_*` が持つ。
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
    fn history_schedule_fetches_at_once_when_last_success_is_ahead() {
        // 1 年先の成功の後に時計が戻っても、365 日待たない（deep.md 第 4 回 Q7。R56）
        let mut schedule = HistorySchedule::with_last_success(Some(at(400 * 86_400)));
        assert!(schedule.due(at(0)));
        schedule.succeeded(at(0));
        assert!(!schedule.due(at(60)));
        // 試し直しの時刻が先に残ったまま戻ったときも待たない
        let mut failed = HistorySchedule::with_last_success(Some(at(0)));
        failed.failed(at(400 * 86_400));
        assert!(failed.due(at(86_400)));
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

    #[test]
    fn history_schedule_backs_off_repeated_failures_up_to_one_hour() {
        let mut schedule = HistorySchedule::with_last_success(Some(at(0)));
        let mut now = 86_400;
        for want in [60, 120, 240, 480, 960, 1920, 3600, 3600] {
            schedule.failed(at(now));
            assert!(
                !schedule.due(at(now + want - 1)),
                "{want} 秒より前に試し直した"
            );
            assert!(schedule.due(at(now + want)), "{want} 秒で試し直さない");
            now += want;
        }
        // 成功すると 1 分に戻る
        schedule.succeeded(at(now));
        schedule.failed(at(now + 86_400));
        assert!(schedule.due(at(now + 86_400 + 60)));
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
        let dir =
            std::env::temp_dir().join(format!("ashiato-history-fetch-{}", uuid::Uuid::new_v4()));
        let path = dir.join("ledger");
        let mut store = LedgerStore::open(path.clone()).unwrap();
        let visit = crate::history::contract::sample_visit(
            1,
            chrono::Utc::now(),
            "https://example.test",
            "題名",
        );

        assert!(
            queue_then_save(&mut store, &[visit.clone()], |_| anyhow::bail!(
                "取り込み口が止まっている"
            ))
            .is_err()
        );
        assert!(
            store.ledger().visits.is_empty(),
            "未送信へ積めないのに成功扱いにしている"
        );

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

    // 印は Runtime を通す `runtime::tests::history_slow_read_does_not_disturb_window` が持つ。
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
// 印は Runtime を通す `runtime::tests::history_vanished_*` が持つ。ここは判定関数だけを見る。
mod vanished_tests {
    use super::*;
    use crate::history::ledger::Ledger;
    use chrono::{Duration, Utc};

    fn ledger() -> Ledger {
        let mut ledger = Ledger::default();
        ledger.record_visit("v1:a", "hash", Utc::now() - Duration::days(3), false, false);
        ledger.max_visit_id = Some(10);
        ledger
    }

    #[test]
    fn history_vanished_is_detected() {
        assert_eq!(
            detect_vanished(&ledger(), &[], Utc::now(), Some(10), None, false)[0].external_id,
            "v1:a"
        );
    }
    #[test]
    fn history_vanished_has_age_days() {
        assert_eq!(
            detect_vanished(&ledger(), &[], Utc::now(), Some(10), None, false)[0].age_days,
            3
        );
    }
    #[test]
    fn history_vanished_marks_foreign() {
        let mut l = ledger();
        l.visits.get_mut("v1:a").unwrap().foreign = true;
        assert!(detect_vanished(&l, &[], Utc::now(), Some(10), None, false)[0].foreign);
    }
    #[test]
    fn history_vanished_marks_recreated_table() {
        assert!(
            detect_vanished(&ledger(), &[], Utc::now(), Some(1), None, false)[0].table_recreated
        );
    }
    #[test]
    fn history_vanished_marks_recreated_table_by_sequence() {
        // 最大の番号が下がらない作り直しは `sqlite_sequence` でだけ分かる（design D10。R64）
        let mut l = ledger();
        l.visit_sequence = Some(50);
        assert!(detect_vanished(&l, &[], Utc::now(), Some(10), Some(3), false)[0].table_recreated);
        assert!(
            !detect_vanished(&l, &[], Utc::now(), Some(10), Some(50), false)[0].table_recreated
        );
        // 前回の値が無い（Firefox・古い帳面）なら比べない
        assert!(
            !detect_vanished(&ledger(), &[], Utc::now(), Some(10), Some(3), false)[0]
                .table_recreated
        );
    }
    #[test]
    fn history_vanished_keeps_visit_whose_url_row_went_missing() {
        // URL の行を失って識別子が変わっても、同じ組の訪問が読めていれば「消えた」にしない（deep.md 第 4 回 Q8）
        let at = Utc::now();
        let with_url = crate::history::contract::sample_visit(1, at, "https://example.test/a", "t");
        let mut l = Ledger::default();
        mark_queued(&mut l, std::slice::from_ref(&with_url));
        let mut read = with_url.payload.clone();
        read.url = None;
        let mut without_url = with_url.clone();
        without_url.external_id = "v1:visit:other".into();
        without_url.payload = read;
        let seen = still_present(&l, std::slice::from_ref(&without_url));
        assert!(detect_vanished(&l, &seen, at, Some(1), None, false).is_empty());
        // 別の訪問（番号が違う）しか無ければ消えた
        let other = crate::history::contract::sample_visit(2, at, "https://example.test/a", "t");
        let seen = still_present(&l, &[other]);
        assert_eq!(
            detect_vanished(&l, &seen, at, Some(2), None, false).len(),
            1
        );
    }
    #[test]
    fn history_vanished_all_gone_is_not_recreated_table() {
        assert!(!detect_vanished(&ledger(), &[], Utc::now(), None, None, false)[0].table_recreated);
    }
    #[test]
    fn history_vanished_marks_gone_profile() {
        assert!(detect_vanished(&ledger(), &[], Utc::now(), Some(10), None, true)[0].profile_gone);
    }
    #[test]
    fn history_vanished_has_no_named_cause_or_private_text() {
        let item = &detect_vanished(&ledger(), &[], Utc::now(), Some(10), None, false)[0];
        let json = serde_json::to_string(item).unwrap();
        assert!(!json.contains("deleted") && !json.contains("url") && !json.contains("title"));
    }
    #[test]
    fn history_vanished_is_chunked_at_1000() {
        let mut l = Ledger::default();
        for i in 0..1001 {
            l.record_visit(&format!("v1:{i:04}"), "h", Utc::now(), false, false);
        }
        let chunks = vanished_chunks(&detect_vanished(&l, &[], Utc::now(), None, None, false));
        assert_eq!(
            chunks.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![1000, 1]
        );
    }
    #[test]
    fn history_vanished_skips_unreadable_profile() {
        let l = ledger();
        assert!(
            detect_vanished_if_readable(&l, &[], Utc::now(), Some(10), None, false, false)
                .is_empty()
        );
    }
    #[test]
    fn history_vanished_is_idempotent_on_retry() {
        let mut l = ledger();
        let vanished =
            detect_vanished_if_readable(&l, &[], Utc::now(), Some(10), None, false, true);
        apply_vanished(&mut l, &vanished);
        assert!(
            detect_vanished_if_readable(&l, &[], Utc::now(), Some(10), None, false, true)
                .is_empty()
        );
    }
}

#[cfg(test)]
mod exclusion_change_tests {
    use super::*;
    use crate::exclusion::{Exclusions, Rule};

    fn visit() -> Visit {
        crate::history::contract::sample_visit(
            1,
            chrono::Utc::now(),
            "https://example.test",
            "private",
        )
    }
    #[test]
    fn history_exclusion_added_later() {
        // 印は Runtime を通す `runtime::tests::history_exclusion_added_later` が持つ。
        let v = visit();
        let mut ledger = Ledger::default();
        mark_queued(&mut ledger, std::slice::from_ref(&v));
        let rules = Exclusions {
            rules: vec![Rule::TitleContains {
                value: "private".into(),
            }],
        };
        let (kept, excluded) = apply_history_exclusions(&mut ledger, &rules, &[v]);
        assert!(kept.is_empty() && excluded.len() == 1);
    }
    #[test]
    fn history_exclusion_removed_later() {
        // 印は Runtime を通す `runtime::tests::history_exclusion_removed_later` が持つ。
        let v = visit();
        let mut ledger = Ledger::default();
        ledger.record_visit(&v.external_id, "h", chrono::Utc::now(), false, true);
        let (kept, excluded) = apply_history_exclusions(
            &mut ledger,
            &Exclusions::default(),
            std::slice::from_ref(&v),
        );
        assert_eq!(kept, vec![v]);
        assert!(excluded.is_empty());
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::history::contract::Visit;
    use crate::history::ledger::Ledger;

    fn visit(id: i64, at: &str, title: &str) -> Visit {
        crate::history::contract::sample_visit(
            id,
            chrono::DateTime::parse_from_rfc3339(at)
                .unwrap()
                .with_timezone(&chrono::Utc),
            "https://example.test/page",
            title,
        )
    }

    // 印は Runtime を通す `runtime::tests::history_runtime_first_fetch_sends_past_visits` と
    // `history_runtime_late_arriving_old_visit_is_sent` が持つ。ここは選別関数だけを見る。
    #[test]
    fn history_fetch_sends_only_new_or_changed() {
        let mut ledger = Ledger::default();
        let old = visit(1, "2026-09-01T00:00:00Z", "古い訪問");
        assert_eq!(
            select_new_or_changed(&ledger, &[old.clone()]),
            vec![old.clone()]
        );
        mark_queued(&mut ledger, &[old.clone()]);
        assert!(select_new_or_changed(&ledger, &[old.clone()]).is_empty());

        // 取得時刻ではなく、全履歴と帳面を比べる。同期で古い訪問が後から来ても落とさない。
        let late_old = visit(2, "2025-01-01T00:00:00Z", "後から同期された訪問");
        assert_eq!(
            select_new_or_changed(&ledger, &[old.clone(), late_old.clone()]),
            vec![late_old]
        );

        let changed = visit(1, "2026-09-01T00:00:00Z", "後から変わった題名");
        assert_eq!(
            select_new_or_changed(&ledger, &[changed.clone()]),
            vec![changed]
        );
    }
}
