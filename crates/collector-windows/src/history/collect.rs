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
    apply_history_exclusions, queue_then_save, select_new_or_changed, HistorySchedule,
    HistoryWorker,
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

/// 履歴 DB を読む側。**実機では [`FsHistoryReader`]、試験では差し替える。**
pub trait HistoryReader: Send + Sync + 'static {
    fn read(&self) -> anyhow::Result<ReadOutcome>;
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
}

/// 取得の契機・別スレッドの読み・未送信への積み込みを持つ。
pub struct HistoryCollector {
    reader: Arc<dyn HistoryReader>,
    dir: PathBuf,
    exclusions_path: PathBuf,
    schedule: HistorySchedule,
    worker: Option<HistoryWorker<ReadOutcome>>,
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
        }
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
        for profile in outcome.profiles {
            let Ok(read) = profile.visits else {
                unreadable += 1;
                continue;
            };
            let mut store = self.open_ledger(profile.browser, &profile.directory)?;
            let visits = read
                .iter()
                .map(|v| Visit::from_read(profile.browser, &profile.directory, v))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let (kept, _) = apply_history_exclusions(store.ledger_mut(), &exclusions, &visits);
            let fresh = select_new_or_changed(store.ledger(), &kept);
            store.ledger_mut().max_visit_id = read.iter().map(|v| v.id).max();
            queue_then_save(&mut store, &fresh, &mut *queue)?;
            queued += fresh.len();
        }
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
