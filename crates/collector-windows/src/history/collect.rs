// SPDX-License-Identifier: AGPL-3.0-only
//! 見つけたプロファイルの履歴を取得し、未送信へ積む（ST08 design D3 / D10）。
//!
//! 読みは別のスレッド（[`HistoryWorker`]）で行い、**積むのは見回りの側**（`Runtime`）。
//! 成功とみなすのは、未送信に積み終えて帳面を書いた**後**だけ。
use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context as _;
use chrono::{DateTime, Utc};

use crate::exclusion::Exclusions;
use crate::history::contract::Visit;
use crate::history::fetch::{
    apply_history_exclusions, apply_vanished, detect_vanished_if_readable, queue_then_save,
    select_new_or_changed, still_present, vanished_chunks, HistorySchedule, HistoryWorker,
    VanishedVisit,
};
use crate::history::ledger::LedgerStore;
use crate::history::locate::{self, Browser};
use crate::history::read::{self, ReadHistory};

/// 1 プロファイルの読み結果。読めなかったときは `Err`（空の履歴と区別する）。
#[derive(Debug)]
pub struct ProfileRead {
    pub browser: Browser,
    pub directory: String,
    pub visits: anyhow::Result<ReadHistory>,
}

/// 1 回の取得で読んだもの全部。
#[derive(Debug, Default)]
pub struct ReadOutcome {
    pub profiles: Vec<ProfileRead>,
    /// ブラウザごとの「ディレクトリ名 → 表示名」（design D1）
    pub names: Vec<(Browser, BTreeMap<String, Option<String>>)>,
}

/// 見つかったプロファイル 1 つの、履歴を読めた（開けた）かどうか。生存信号の取得可否の材料（design D12）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileHealth {
    pub browser: Browser,
    pub directory: String,
    pub readable: bool,
}

/// 履歴 DB を読む側。**実機では [`FsHistoryReader`]、試験では差し替える。**
///
/// `tmp` は写しの置き場（`state_dir/browser-history/tmp`。design D2）。
pub trait HistoryReader: Send + Sync + 'static {
    fn read(&self, tmp: &Path) -> anyhow::Result<ReadOutcome>;

    /// 見つかった対象の写しを取って開けるか（`SELECT` まで）だけを確かめる。行は読まない。
    /// 区間に読みが 1 回も無いときの生存信号が使う（design D12）。
    fn probe(&self, _tmp: &Path) -> Vec<ProfileHealth> {
        Vec::new()
    }

    /// プロファイルのディレクトリそのものが無いと**確かめられた**ときだけ `true`。
    /// 読みに出てこない（読めない・DB が一時的に無い）だけでは `true` にしない（D10）。
    fn profile_dir_absent(&self, _browser: Browser, _directory: &str) -> bool {
        false
    }
}

/// 置き場を探して写しから読む、本物の読み手。
#[derive(Debug, Clone)]
pub struct FsHistoryReader {
    local: PathBuf,
    roaming: PathBuf,
}

impl FsHistoryReader {
    pub fn new(local: PathBuf, roaming: PathBuf) -> Self {
        Self { local, roaming }
    }

    /// `LOCALAPPDATA` / `APPDATA` から作る。どちらかが無ければ `None`（探す先が無い）。
    pub fn from_env() -> Option<Self> {
        Some(Self::new(
            std::env::var_os("LOCALAPPDATA")?.into(),
            std::env::var_os("APPDATA")?.into(),
        ))
    }
}

impl HistoryReader for FsHistoryReader {
    fn read(&self, tmp: &Path) -> anyhow::Result<ReadOutcome> {
        let found = locate::locate(&self.local, &self.roaming);
        let names = Browser::ALL
            .iter()
            .filter(|b| found.iter().any(|p| p.browser == **b))
            .map(|b| {
                let base = b.base(&self.local, &self.roaming);
                (*b, locate::current_mapping(*b, &base, &found))
            })
            .collect();
        let profiles = found
            .iter()
            .map(|p| ProfileRead {
                browser: p.browser,
                directory: p.directory.clone(),
                visits: read::read_visits(p.browser, &p.path, tmp),
            })
            .collect();
        Ok(ReadOutcome { profiles, names })
    }

    fn probe(&self, tmp: &Path) -> Vec<ProfileHealth> {
        locate::locate(&self.local, &self.roaming)
            .into_iter()
            .map(|p| ProfileHealth {
                readable: read::probe_readable(&p.path, tmp).is_ok(),
                browser: p.browser,
                directory: p.directory,
            })
            .collect()
    }

    fn profile_dir_absent(&self, browser: Browser, directory: &str) -> bool {
        let base = browser.base(&self.local, &self.roaming);
        // 置き場そのものが読めないときは、無いとは言えない
        if std::fs::read_dir(&base).is_err() {
            return false;
        }
        let parents = [base.clone(), base.join("_side_profiles")];
        // 在るかどうか確かめられないときは「在る」と読む
        let dir_exists = |parent: &PathBuf| parent.join(directory).try_exists().unwrap_or(true);
        !parents.iter().any(dir_exists)
    }
}

/// 取得の契機・別スレッドの読み・未送信への積み込みを持つ。
pub struct HistoryCollector {
    reader: Arc<dyn HistoryReader>,
    dir: PathBuf,
    exclusions_path: PathBuf,
    schedule: HistorySchedule,
    worker: Option<HistoryWorker<ReadOutcome>>,
    /// 別スレッドの「開けるかの確かめ」（生存信号の前。読みとは別）
    probe: Option<HistoryWorker<Vec<ProfileHealth>>>,
    /// 読み終えたが、生存信号の数えへまだ渡していない読みの結果
    finished_reads: Vec<ProfileHealth>,
    /// 読みが 1 回でも終わったか（見つからなかった読みも含む）が `finished_reads` と別に要るので件数でなく印で持つ
    read_finished: bool,
    /// まだログに出していない、飛ばした行の数（R59）
    skipped_rows: usize,
    /// まだログに出していない、壊れていて退避した帳面の数（R63）
    quarantined: Cell<usize>,
}

impl std::fmt::Debug for HistoryCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryCollector")
            .field("reading", &self.worker.is_some())
            .finish()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LastSuccess {
    last_success: DateTime<Utc>,
}

impl HistoryCollector {
    /// `state_dir/browser-history/` を置き場にする。前回の成功が読めなければ「初めて」と同じに扱う。
    pub fn new(reader: Arc<dyn HistoryReader>, state_dir: &std::path::Path) -> Self {
        let dir = state_dir.join("browser-history");
        let last = std::fs::read(dir.join("last_success.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<LastSuccess>(&b).ok())
            .map(|l| l.last_success);
        Self {
            reader,
            dir,
            exclusions_path: state_dir.join("exclusions.json"),
            schedule: HistorySchedule::with_last_success(last),
            worker: None,
            probe: None,
            finished_reads: Vec::new(),
            read_finished: false,
            skipped_rows: 0,
            quarantined: Cell::new(0),
        }
    }

    /// 写しの置き場（design D2）。
    fn tmp(&self) -> PathBuf {
        self.dir.join("tmp")
    }

    /// 前のプロセスが残した写しを消す。起動時に 1 回、読みを始める前に呼ぶ（R60）。
    pub fn sweep_copies(&self) -> std::io::Result<()> {
        read::sweep_copies(&self.tmp())
    }

    /// ログに件数だけ出す出来事（種別, 件数）を返して 0 に戻す。0 件のものは返さない。
    /// URL・パス・表示名は持たない（`telemetry::history_line` に渡す）。
    pub fn take_log_counts(&mut self) -> Vec<(&'static str, usize)> {
        [
            (
                "history_rows_skipped",
                std::mem::take(&mut self.skipped_rows),
            ),
            ("history_ledger_quarantined", self.quarantined.take()),
            ("history_copy_cleanup_failed", read::take_cleanup_failures()),
        ]
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .collect()
    }

    /// 前回から終わった読みの結果を渡す。読みが 1 回でも終わっていたら `Some`（空は「1 つも見つからない」）。
    pub fn take_reads(&mut self) -> Option<Vec<ProfileHealth>> {
        std::mem::take(&mut self.read_finished).then(|| std::mem::take(&mut self.finished_reads))
    }

    /// 開けるかの確かめを別スレッドで始める。
    pub fn start_probe(&mut self) {
        if self.probe.is_none() {
            let (reader, tmp) = (Arc::clone(&self.reader), self.tmp());
            self.probe = Some(HistoryWorker::spawn(move || Ok(reader.probe(&tmp))));
        }
    }

    pub fn is_probing(&self) -> bool {
        self.probe.is_some()
    }

    /// 確かめが終わっていれば結果を返す。終わっていなければ `None`（待たない）。
    pub fn poll_probe(&mut self) -> Option<Vec<ProfileHealth>> {
        if !self.probe.as_ref()?.is_finished() {
            return None;
        }
        // 確かめの thread が落ちたら、見つからなかったと同じ扱い（理由の無い「取れる」にしない）
        Some(self.probe.take()?.join().unwrap_or_default())
    }

    /// 読みの最中か（読みを始めて、まだ積み込んでいない）。
    pub fn is_reading(&self) -> bool {
        self.worker.is_some()
    }

    /// 見回りごとに呼ぶ。読みが終わっていれば積み込んで結果（積んだ件数）を返し、
    /// 契機に達していれば読みを別スレッドで始める。**読みの完了は待たない。**
    pub fn tick(
        &mut self,
        wall: DateTime<Utc>,
        queue: &mut dyn FnMut(&Visit) -> anyhow::Result<()>,
    ) -> Option<anyhow::Result<usize>> {
        if let Some(worker) = &self.worker {
            if !worker.is_finished() {
                return None;
            }
            let outcome = self.worker.take()?.join();
            if let Ok(o) = &outcome {
                self.skipped_rows += o
                    .profiles
                    .iter()
                    .filter_map(|p| p.visits.as_ref().ok())
                    .map(|r| r.skipped)
                    .sum::<usize>();
                self.finished_reads = o
                    .profiles
                    .iter()
                    .map(|p| ProfileHealth {
                        browser: p.browser,
                        directory: p.directory.clone(),
                        readable: p.visits.is_ok(),
                    })
                    .collect();
                self.read_finished = true;
            }
            let result = outcome.and_then(|o| self.apply(o, wall, queue));
            match &result {
                Ok(_) => self.schedule.succeeded(wall),
                Err(_) => self.schedule.failed(wall),
            }
            return Some(result);
        }
        if self.schedule.due(wall) {
            let (reader, tmp) = (Arc::clone(&self.reader), self.tmp());
            self.worker = Some(HistoryWorker::spawn(move || reader.read(&tmp)));
        }
        None
    }

    /// 積み終えて帳面を書いてから、前回の成功を置き場へ書く。途中で落ちたら成功にしない。
    fn apply(
        &self,
        outcome: ReadOutcome,
        wall: DateTime<Utc>,
        queue: &mut dyn FnMut(&Visit) -> anyhow::Result<()>,
    ) -> anyhow::Result<usize> {
        let exclusions = Exclusions::load(&self.exclusions_path)
            .context("除外の登録を読めないので履歴を送らない")?;
        let mut queued = 0;
        let mut unreadable = 0;
        for profile in &outcome.profiles {
            let Ok(read) = &profile.visits else {
                unreadable += 1;
                continue;
            };
            let mut store = self.open_ledger(profile.browser, &profile.directory)?;
            let visits = read
                .visits
                .iter()
                .map(|v| Visit::from_read(profile.browser, &profile.directory, v))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let (kept, newly_excluded) =
                apply_history_exclusions(store.ledger_mut(), &exclusions, &visits);
            // 除外の件数は「その回に新しく除外した数」1 件。0 なら書かない。識別子は中身から決まるので、
            // 積んだ後・帳面の保存の前に落ちたやり直しでも同じ 1 件に畳まれる（D6 / D11）
            if !newly_excluded.is_empty() {
                queue(&Visit::excluded(
                    profile.browser,
                    &profile.directory,
                    wall,
                    &newly_excluded,
                )?)?;
                queued += 1;
            }
            let fresh = select_new_or_changed(store.ledger(), &kept);
            // 帳面の最大の訪問番号と比べるので、書き換える前に消えた訪問を見つける
            let max_visit_id = read.visits.iter().map(|v| v.id).max();
            let seen = still_present(store.ledger(), &visits);
            let vanished = detect_vanished_if_readable(
                store.ledger(),
                &seen,
                wall,
                max_visit_id,
                read.sequence,
                false,
                true,
            );
            queued += self.queue_vanished(
                &mut store,
                profile.browser,
                &profile.directory,
                &vanished,
                wall,
                &mut *queue,
            )?;
            // 全件が消えて番号が無いときは、前回の番号を比べる土台として残す
            let ledger = store.ledger_mut();
            ledger.max_visit_id = max_visit_id.or(ledger.max_visit_id);
            ledger.visit_sequence = read.sequence.or(ledger.visit_sequence);
            queue_then_save(&mut store, &fresh, &mut *queue)?;
            queued += fresh.len();
        }
        queued += self.queue_gone_profiles(&outcome.profiles, wall, &mut *queue)?;
        for (browser, current) in &outcome.names {
            queued += self.queue_profiles(*browser, current, wall, &mut *queue)?;
        }
        anyhow::ensure!(unreadable == 0, "読めなかったプロファイルがある");
        crate::fsutil::atomic_write(
            &self.dir.join("last_success.json"),
            &serde_json::to_vec(&LastSuccess { last_success: wall })?,
        )?;
        Ok(queued)
    }

    /// 消えた訪問を `vanished` にして積み、帳面から外す（保存は呼び出し側）。
    /// 積んだ後・保存の前に落ちても、識別子が中身から決まるのでやり直しで畳まれる（D6）。
    fn queue_vanished(
        &self,
        store: &mut LedgerStore,
        browser: Browser,
        directory: &str,
        vanished: &[VanishedVisit],
        wall: DateTime<Utc>,
        queue: &mut dyn FnMut(&Visit) -> anyhow::Result<()>,
    ) -> anyhow::Result<usize> {
        let mut queued = 0;
        for chunk in vanished_chunks(vanished) {
            queue(&Visit::vanished(browser, directory, wall, chunk)?)?;
            queued += 1;
        }
        apply_vanished(store.ledger_mut(), vanished);
        Ok(queued)
    }

    /// 帳面があるのに今回見つからず、ディレクトリそのものが無いと確かめられたものを消えたとする。
    fn queue_gone_profiles(
        &self,
        found: &[ProfileRead],
        wall: DateTime<Utc>,
        queue: &mut dyn FnMut(&Visit) -> anyhow::Result<()>,
    ) -> anyhow::Result<usize> {
        let mut queued = 0;
        for browser in Browser::ALL {
            let Ok(entries) = std::fs::read_dir(self.dir.join(browser.name())) else {
                continue;
            };
            let mut gone: Vec<String> = entries
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter_map(|n| n.strip_suffix(".ledger").map(str::to_owned))
                .filter(|d| !d.ends_with(".broken"))
                .filter(|d| {
                    !found
                        .iter()
                        .any(|p| p.browser == browser && &p.directory == d)
                })
                .filter(|d| self.reader.profile_dir_absent(browser, d))
                .collect();
            gone.sort();
            for directory in gone {
                let mut store = self.open_ledger(browser, &directory)?;
                let vanished =
                    detect_vanished_if_readable(store.ledger(), &[], wall, None, None, true, true);
                if vanished.is_empty() {
                    continue;
                }
                queued += self.queue_vanished(
                    &mut store,
                    browser,
                    &directory,
                    &vanished,
                    wall,
                    &mut *queue,
                )?;
                store.save()?;
            }
        }
        Ok(queued)
    }

    /// 表示名の対応が初回か変わったときだけ `profiles` を積み、帳面に今回の対応を残す。
    fn queue_profiles(
        &self,
        browser: Browser,
        current: &BTreeMap<String, Option<String>>,
        wall: DateTime<Utc>,
        queue: &mut dyn FnMut(&Visit) -> anyhow::Result<()>,
    ) -> anyhow::Result<usize> {
        let mut stores = Vec::new();
        let mut previous = BTreeMap::new();
        for directory in current.keys() {
            let store = self.open_ledger(browser, directory)?;
            previous.extend(store.ledger().profile_names.clone());
            stores.push((directory, store));
        }
        let record = locate::profiles_record_if_changed(browser, wall, current, &previous)?;
        let Some(record) = record else {
            return Ok(0);
        };
        queue(&record)?;
        for (directory, mut store) in stores {
            store
                .ledger_mut()
                .set_profile_name(directory, current[directory].clone());
            store.save()?;
        }
        Ok(1)
    }

    fn open_ledger(&self, browser: Browser, directory: &str) -> anyhow::Result<LedgerStore> {
        let store = LedgerStore::open(
            self.dir
                .join(browser.name())
                .join(format!("{directory}.ledger")),
        )?;
        if store.quarantined() {
            self.quarantined.set(self.quarantined.get() + 1);
        }
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// 本物の読み手（置き場探し・写し・読み）を、一時の置き場に作った Chromium の履歴 DB で通す。
    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(rows: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("ashiato-collect-{}", uuid::Uuid::new_v4()));
            let profile = root.join("local/Google/Chrome/User Data/Default");
            std::fs::create_dir_all(&profile).unwrap();
            std::fs::create_dir_all(root.join("roaming")).unwrap();
            let conn = rusqlite::Connection::open(profile.join("History")).unwrap();
            conn.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, from_visit INTEGER, transition INTEGER); INSERT INTO urls VALUES(1,'https://example.test/','t');").unwrap();
            conn.execute_batch(rows).unwrap();
            Self { root }
        }

        fn collector(&self) -> HistoryCollector {
            let reader = FsHistoryReader::new(self.root.join("local"), self.root.join("roaming"));
            HistoryCollector::new(Arc::new(reader), &self.state())
        }

        fn state(&self) -> PathBuf {
            self.root.join("state")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).ok();
        }
    }

    /// 取得契機から積み込みまで回し、結果と積んだ記録を返す。
    fn fetch(
        collector: &mut HistoryCollector,
        wall: DateTime<Utc>,
    ) -> (anyhow::Result<usize>, Vec<Visit>) {
        let mut queued = Vec::new();
        let started = std::time::Instant::now();
        loop {
            let mut queue = |v: &Visit| {
                queued.push(v.clone());
                Ok(())
            };
            if let Some(result) = collector.tick(wall, &mut queue) {
                return (result, queued);
            }
            assert!(
                started.elapsed() < std::time::Duration::from_secs(10),
                "読みが終わらない"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// 負の訪問時刻の行が 1 つあっても、そのプロファイルの取得は成功し、前回の成功が進む。
    /// 飛ばした行は件数だけログへ渡る（R59）。
    #[test]
    fn history_bad_row_does_not_fail_the_profile() {
        let fx = Fixture::new("INSERT INTO visits VALUES(1,1,13402627200000000,0,1),(2,1,-5,0,1);");
        let mut collector = fx.collector();
        let now = Utc::now();
        let (result, queued) = fetch(&mut collector, now);
        result.unwrap();
        assert_eq!(
            queued.iter().filter(|v| v.payload.kind == "visit").count(),
            1
        );
        assert_eq!(collector.schedule.last_success(), Some(now));
        assert!(fx
            .state()
            .join("browser-history/last_success.json")
            .is_file());
        assert_eq!(
            collector.take_log_counts(),
            vec![("history_rows_skipped", 1)]
        );
        assert!(
            collector.take_log_counts().is_empty(),
            "数えを 0 に戻さない"
        );
    }

    /// 写しは `state_dir/browser-history/tmp` の下に作られ、読み終えると残らない（design D2。R60）。
    #[test]
    fn history_copies_live_under_state_tmp_and_are_removed() {
        let fx = Fixture::new("INSERT INTO visits VALUES(1,1,13402627200000000,0,1);");
        let mut collector = fx.collector();
        let tmp = fx.state().join("browser-history/tmp");
        std::fs::create_dir_all(tmp.join("left-over")).unwrap();
        std::fs::write(tmp.join("left-over/History"), b"private").unwrap();
        collector.sweep_copies().unwrap();
        assert!(
            !tmp.join("left-over").exists(),
            "前のプロセスの写しが残った"
        );
        let (result, _) = fetch(&mut collector, Utc::now());
        result.unwrap();
        // 写しは tmp の下に作られた（ディレクトリができている）が、中身は残らない
        assert!(tmp.is_dir(), "写しを置き場の tmp の下に作っていない");
        assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0);
    }

    /// 壊れた帳面の退避は、件数だけログへ渡る（R63）。
    #[test]
    fn history_quarantined_ledger_is_counted() {
        let fx = Fixture::new("INSERT INTO visits VALUES(1,1,13402627200000000,0,1);");
        let ledger = fx.state().join("browser-history/chrome/Default.ledger");
        std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
        std::fs::write(&ledger, b"{broken").unwrap();
        let mut collector = fx.collector();
        let (result, _) = fetch(&mut collector, Utc::now());
        result.unwrap();
        assert!(collector
            .take_log_counts()
            .contains(&("history_ledger_quarantined", 1)));
    }
}
