// SPDX-License-Identifier: AGPL-3.0-only
//! 見つけたプロファイルの履歴を取得し、未送信へ積む（ST08 design D3 / D10）。
//!
//! 読みは別のスレッド（[`HistoryWorker`]）で行い、**積むのは見回りの側**（`Runtime`）。
//! 成功とみなすのは、未送信に積み終えて帳面を書いた**後**だけ。
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context as _;
use chrono::{DateTime, Utc};

use crate::exclusion::Exclusions;
use crate::history::contract::Visit;
use crate::history::fetch::{
    apply_history_exclusions, apply_vanished, detect_vanished_if_readable, queue_then_save,
    select_new_or_changed, vanished_chunks, HistorySchedule, HistoryWorker, VanishedVisit,
};
use crate::history::ledger::LedgerStore;
use crate::history::locate::{self, Browser};
use crate::history::read::{self, ReadVisit};

/// 1 プロファイルの読み結果。読めなかったときは `Err`（空の履歴と区別する）。
#[derive(Debug)]
pub struct ProfileRead {
    pub browser: Browser,
    pub directory: String,
    pub visits: anyhow::Result<Vec<ReadVisit>>,
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
pub trait HistoryReader: Send + Sync + 'static {
    fn read(&self) -> anyhow::Result<ReadOutcome>;

    /// 見つかった対象の写しを取って開けるか（`SELECT` まで）だけを確かめる。行は読まない。
    /// 区間に読みが 1 回も無いときの生存信号が使う（design D12）。
    fn probe(&self) -> Vec<ProfileHealth> {
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
    fn read(&self) -> anyhow::Result<ReadOutcome> {
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
                visits: read::read_visits(p.browser, &p.path),
            })
            .collect();
        Ok(ReadOutcome { profiles, names })
    }

    fn probe(&self) -> Vec<ProfileHealth> {
        locate::locate(&self.local, &self.roaming)
            .into_iter()
            .map(|p| ProfileHealth {
                readable: read::probe_readable(&p.path).is_ok(),
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
        }
    }

    /// 前回から終わった読みの結果を渡す。読みが 1 回でも終わっていたら `Some`（空は「1 つも見つからない」）。
    pub fn take_reads(&mut self) -> Option<Vec<ProfileHealth>> {
        std::mem::take(&mut self.read_finished).then(|| std::mem::take(&mut self.finished_reads))
    }

    /// 開けるかの確かめを別スレッドで始める。
    pub fn start_probe(&mut self) {
        if self.probe.is_none() {
            let reader = Arc::clone(&self.reader);
            self.probe = Some(HistoryWorker::spawn(move || Ok(reader.probe())));
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
            let reader = Arc::clone(&self.reader);
            self.worker = Some(HistoryWorker::spawn(move || reader.read()));
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
            let max_visit_id = read.iter().map(|v| v.id).max();
            let seen: Vec<String> = visits.iter().map(|v| v.external_id.clone()).collect();
            let vanished =
                detect_vanished_if_readable(store.ledger(), &seen, wall, max_visit_id, false, true);
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
                    detect_vanished_if_readable(store.ledger(), &[], wall, None, true, true);
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
        LedgerStore::open(
            self.dir
                .join(browser.name())
                .join(format!("{directory}.ledger")),
        )
    }
}
