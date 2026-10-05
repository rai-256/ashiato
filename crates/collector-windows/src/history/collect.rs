// SPDX-License-Identifier: AGPL-3.0-only
//! 見つけたプロファイルの履歴を取得し、未送信へ積む（ST08 design D3 / D10）。
//!
//! 読みと積む分の組み立て・帳面の書き込みは別のスレッド（[`HistoryWorker`]）で行い、
//! **積むのは見回りの側**（`Runtime`）で 1 回 [`QUEUE_PER_TICK`] 件まで（deep.md 第 5 回 Q9）。
//! 成功とみなすのは、未送信に積み終えて帳面を書いた**後**だけ。
use std::cell::Cell;
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context as _;
use chrono::{DateTime, Utc};

use crate::exclusion::Exclusions;
use crate::history::contract::Visit;
use crate::history::fetch::{
    apply_history_exclusions, apply_vanished, detect_vanished_if_readable, mark_queued,
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
        // 探す先の根が無いときは、無いとは言えない（環境が違う）
        if ![&self.local, &self.roaming]
            .iter()
            .all(|root| matches!(root.try_exists(), Ok(true)))
        {
            return false;
        }
        // 見つけるときと同じ置き場を候補にする（R71）。置き場ごと無いときは、どの候補も無い
        let base = browser.base(&self.local, &self.roaming);
        let mut candidates = vec![base.join(directory)];
        match browser {
            // Opera の主プロファイルは置き場の直下を `Default` と読む
            Browser::Opera => {
                candidates.push(base.join("_side_profiles").join(directory));
                if directory == "Default" {
                    candidates.push(base.clone());
                }
            }
            // ini に絶対パスで書かれたプロファイルは置き場の外にある
            Browser::Firefox => {
                let ini = self.roaming.join("Mozilla/Firefox/profiles.ini");
                match std::fs::read_to_string(&ini) {
                    Ok(text) => candidates.extend(
                        locate::ini_profile_dirs(&ini, &text)
                            .into_iter()
                            .filter(|d| d.file_name() == Some(std::ffi::OsStr::new(directory))),
                    ),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    // 読めない ini では確かめられない
                    Err(_) => return false,
                }
            }
            _ => {}
        }
        // 在るかどうか確かめられないときは「在る」と読む
        !candidates
            .iter()
            .any(|dir| dir.try_exists().unwrap_or(true))
    }
}

/// 見回り 1 回で未送信に積む履歴の件数の上限。残りは次の見回りへ回す（deep.md 第 5 回 Q9）。
///
/// 積むのは見回りのスレッドなので、1 回の取得の全件を一度に積むと、件数に比例して見回りが戻らない
/// （Windows 実機で 15 万件 148 秒。review/code.md R68）。
pub const QUEUE_PER_TICK: usize = 2_000;

/// 読めなかったプロファイルだけが理由で、取得を成功にしなかったこと。
/// 積み込みと帳面はほかのプロファイルの分まで済んでいるので、「送る準備の失敗」には数えない（Q10）。
#[derive(Debug)]
struct UnreadableProfiles;

impl std::fmt::Display for UnreadableProfiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("読めなかったプロファイルがある")
    }
}

impl std::error::Error for UnreadableProfiles {}

/// 訪問をまとめて未送信へ積む口（2 つめの引数は読んだ時刻。design D15）。
pub type QueueVisits<'q> = dyn FnMut(&[Visit], DateTime<Utc>) -> anyhow::Result<()> + 'q;

/// 取得 1 回の、生存信号の数えへ渡す結果（design D12）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchReport {
    pub profiles: Vec<ProfileHealth>,
    /// 送る準備（除外の登録を読む・未送信に積む・帳面と前回の成功を書く）まで済んだか（deep.md 第 5 回 Q10）
    pub queued: bool,
}

/// 読んだ後、見回りが未送信に積む分と、積み終えてから書く帳面。
struct Plan {
    items: VecDeque<Visit>,
    /// 積み終える前に書くと、落ちたときに積んでいない訪問を「送った」と読む
    stores: Vec<LedgerStore>,
    unreadable: usize,
    /// 読んだ時刻（`source_updated_at` と前回の成功。design D15）
    read_at: DateTime<Utc>,
    queued: usize,
}

/// 別スレッドで読み、積む分を組んだもの。
struct Fetched {
    health: Vec<ProfileHealth>,
    skipped: usize,
    quarantined: usize,
    plan: anyhow::Result<Plan>,
}

/// 取得 1 回の段。**どの段も見回りを待たせない**（積むのだけが見回りの側で、1 回 [`QUEUE_PER_TICK`] 件まで）。
enum Stage {
    Idle,
    /// 読みと、積む分の組み立て（別スレッド）
    Reading(HistoryWorker<Fetched>),
    /// 見回りごとに少しずつ未送信へ積む
    Queueing(Plan, Vec<ProfileHealth>),
    /// 帳面と前回の成功を書く（別スレッド）
    Saving(HistoryWorker<usize>, Vec<ProfileHealth>, DateTime<Utc>),
}

/// 取得の契機・別スレッドの読み・未送信への積み込みを持つ。
pub struct HistoryCollector {
    reader: Arc<dyn HistoryReader>,
    dir: PathBuf,
    exclusions_path: PathBuf,
    schedule: HistorySchedule,
    stage: Stage,
    /// 別スレッドの「開けるかの確かめ」（生存信号の前。読みとは別）
    probe: Option<HistoryWorker<Vec<ProfileHealth>>>,
    /// 終わったが、生存信号の数えへまだ渡していない取得の結果
    report: Option<FetchReport>,
    /// まだログに出していない、飛ばした行の数（R59）
    skipped_rows: usize,
    /// まだログに出していない、壊れていて退避した帳面の数（R63）
    quarantined: usize,
}

impl std::fmt::Debug for HistoryCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryCollector")
            .field("reading", &self.is_reading())
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
            stage: Stage::Idle,
            probe: None,
            report: None,
            skipped_rows: 0,
            quarantined: 0,
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
            (
                "history_ledger_quarantined",
                std::mem::take(&mut self.quarantined),
            ),
            ("history_copy_cleanup_failed", read::take_cleanup_failures()),
        ]
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .collect()
    }

    /// 前回から終わった取得の結果を渡す（読みが終わらなかった取得は無い）。`profiles` が空なら「1 つも見つからない」。
    pub fn take_report(&mut self) -> Option<FetchReport> {
        self.report.take()
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

    /// 取得の最中か（読みを始めて、まだ積み終えて帳面を書き終えていない）。
    pub fn is_reading(&self) -> bool {
        !matches!(self.stage, Stage::Idle)
    }

    /// 見回りごとに呼ぶ。取得を 1 段ずつ進め、終わった回に結果（積んだ件数）を返す。
    /// 契機に達していれば読みを別スレッドで始める。**読みも帳面の書き込みも待たず、積むのは 1 回 [`QUEUE_PER_TICK`] 件まで。**
    ///
    /// `queue` は訪問をまとめて未送信へ積む（2 つめの引数は読んだ時刻）。
    pub fn tick(
        &mut self,
        wall: DateTime<Utc>,
        queue: &mut QueueVisits<'_>,
    ) -> Option<anyhow::Result<usize>> {
        if let Stage::Reading(worker) = &self.stage {
            if !worker.is_finished() {
                return None;
            }
            let Stage::Reading(worker) = std::mem::replace(&mut self.stage, Stage::Idle) else {
                unreachable!()
            };
            let fetched = match worker.join() {
                Ok(f) => f,
                // 読みそのものが落ちた（thread の panic）。読めたプロファイルが分からないので数えない
                Err(e) => {
                    self.schedule.failed(wall);
                    return Some(Err(e));
                }
            };
            self.skipped_rows += fetched.skipped;
            self.quarantined += fetched.quarantined;
            match fetched.plan {
                Ok(plan) => self.stage = Stage::Queueing(plan, fetched.health),
                Err(e) => return Some(self.finish(wall, None, fetched.health, Err(e))),
            }
        }
        if let Stage::Queueing(plan, _) = &mut self.stage {
            let n = plan.items.len().min(QUEUE_PER_TICK);
            let chunk: Vec<Visit> = plan.items.drain(..n).collect();
            let queued = if chunk.is_empty() {
                Ok(())
            } else {
                queue(&chunk, plan.read_at)
            };
            let more = !plan.items.is_empty();
            let Stage::Queueing(plan, health) = std::mem::replace(&mut self.stage, Stage::Idle)
            else {
                unreachable!()
            };
            if let Err(e) = queued {
                return Some(self.finish(wall, None, health, Err(e)));
            }
            if more {
                self.stage = Stage::Queueing(plan, health);
            } else {
                let read_at = plan.read_at;
                let dir = self.dir.clone();
                self.stage = Stage::Saving(
                    HistoryWorker::spawn(move || plan.save(&dir)),
                    health,
                    read_at,
                );
            }
            return None;
        }
        if let Stage::Saving(worker, ..) = &self.stage {
            if !worker.is_finished() {
                return None;
            }
            let Stage::Saving(worker, health, read_at) =
                std::mem::replace(&mut self.stage, Stage::Idle)
            else {
                unreachable!()
            };
            return Some(self.finish(wall, Some(read_at), health, worker.join()));
        }
        if self.schedule.due(wall) {
            let planner = Planner {
                reader: Arc::clone(&self.reader),
                dir: self.dir.clone(),
                exclusions_path: self.exclusions_path.clone(),
                quarantined: Cell::new(0),
            };
            self.stage = Stage::Reading(HistoryWorker::spawn(move || planner.fetch(wall)));
        }
        None
    }

    /// 取得 1 回を閉じる。成功なら前回の成功を読んだ時刻へ進め、生存信号の数えへ結果を渡す。
    fn finish(
        &mut self,
        wall: DateTime<Utc>,
        read_at: Option<DateTime<Utc>>,
        health: Vec<ProfileHealth>,
        result: anyhow::Result<usize>,
    ) -> anyhow::Result<usize> {
        let queued = match &result {
            Ok(_) => true,
            Err(e) => e.downcast_ref::<UnreadableProfiles>().is_some(),
        };
        match (&result, read_at) {
            (Ok(_), Some(at)) => self.schedule.succeeded(at),
            _ => self.schedule.failed(wall),
        }
        self.report = Some(FetchReport {
            profiles: health,
            queued,
        });
        result
    }
}

impl Plan {
    /// 積み終えた後に帳面を書き、読めなかったプロファイルが無ければ前回の成功を書く。
    fn save(self, dir: &Path) -> anyhow::Result<usize> {
        for store in &self.stores {
            store.save()?;
        }
        if self.unreadable > 0 {
            return Err(UnreadableProfiles.into());
        }
        crate::fsutil::atomic_write(
            &dir.join("last_success.json"),
            &serde_json::to_vec(&LastSuccess {
                last_success: self.read_at,
            })?,
        )?;
        Ok(self.queued)
    }
}

/// 読みと、積む分の組み立て。別スレッドで動く（件数に比例する仕事を見回りに残さない。Q9）。
struct Planner {
    reader: Arc<dyn HistoryReader>,
    dir: PathBuf,
    exclusions_path: PathBuf,
    quarantined: Cell<usize>,
}

impl Planner {
    fn fetch(self, wall: DateTime<Utc>) -> anyhow::Result<Fetched> {
        let outcome = self.reader.read(&self.dir.join("tmp"))?;
        let health = outcome
            .profiles
            .iter()
            .map(|p| ProfileHealth {
                browser: p.browser,
                directory: p.directory.clone(),
                readable: p.visits.is_ok(),
            })
            .collect();
        let skipped = outcome
            .profiles
            .iter()
            .filter_map(|p| p.visits.as_ref().ok())
            .map(|r| r.skipped)
            .sum();
        let plan = self.plan(outcome, wall);
        Ok(Fetched {
            health,
            skipped,
            quarantined: self.quarantined.get(),
            plan,
        })
    }

    /// 積む分を順に組み、帳面は書かずに持つ（積み終えてから書く）。
    fn plan(&self, outcome: ReadOutcome, wall: DateTime<Utc>) -> anyhow::Result<Plan> {
        let exclusions = Exclusions::load(&self.exclusions_path)
            .context("除外の登録を読めないので履歴を送らない")?;
        let mut items = Vec::new();
        let mut stores = BTreeMap::new();
        let mut unreadable = 0;
        for profile in &outcome.profiles {
            let Ok(read) = &profile.visits else {
                unreadable += 1;
                continue;
            };
            let store = self.ledger(&mut stores, profile.browser, &profile.directory)?;
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
                items.push(Visit::excluded(
                    profile.browser,
                    &profile.directory,
                    wall,
                    &newly_excluded,
                )?);
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
            queue_vanished(
                store,
                profile.browser,
                &profile.directory,
                &vanished,
                wall,
                &mut items,
            )?;
            // 全件が消えて番号が無いときは、前回の番号を比べる土台として残す
            let ledger = store.ledger_mut();
            ledger.max_visit_id = max_visit_id.or(ledger.max_visit_id);
            ledger.visit_sequence = read.sequence.or(ledger.visit_sequence);
            mark_queued(ledger, &fresh);
            items.extend(fresh);
        }
        self.queue_gone_profiles(&mut stores, &outcome.profiles, wall, &mut items)?;
        for (browser, current) in &outcome.names {
            self.queue_profiles(&mut stores, *browser, current, wall, &mut items)?;
        }
        Ok(Plan {
            queued: items.len(),
            items: items.into(),
            stores: stores.into_values().collect(),
            unreadable,
            read_at: wall,
        })
    }

    /// 帳面があるのに今回見つからず、ディレクトリそのものが無いと確かめられたものを消えたとする。
    fn queue_gone_profiles(
        &self,
        stores: &mut BTreeMap<PathBuf, LedgerStore>,
        found: &[ProfileRead],
        wall: DateTime<Utc>,
        items: &mut Vec<Visit>,
    ) -> anyhow::Result<()> {
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
                let store = self.ledger(stores, browser, &directory)?;
                let vanished =
                    detect_vanished_if_readable(store.ledger(), &[], wall, None, None, true, true);
                queue_vanished(store, browser, &directory, &vanished, wall, items)?;
            }
        }
        Ok(())
    }

    /// 表示名の対応が初回か変わったときだけ `profiles` を積み、帳面に今回の対応を残す。
    fn queue_profiles(
        &self,
        stores: &mut BTreeMap<PathBuf, LedgerStore>,
        browser: Browser,
        current: &BTreeMap<String, Option<String>>,
        wall: DateTime<Utc>,
        items: &mut Vec<Visit>,
    ) -> anyhow::Result<()> {
        let mut previous = BTreeMap::new();
        for directory in current.keys() {
            let store = self.ledger(stores, browser, directory)?;
            previous.extend(store.ledger().profile_names.clone());
        }
        let Some(record) = locate::profiles_record_if_changed(browser, wall, current, &previous)?
        else {
            return Ok(());
        };
        items.push(record);
        for (directory, name) in current {
            self.ledger(stores, browser, directory)?
                .ledger_mut()
                .set_profile_name(directory, name.clone());
        }
        Ok(())
    }

    /// 帳面を開く。**1 回の取得で同じ帳面は 1 度だけ開く** —— 書くのは積み終えた後なので、
    /// 開き直すと前の変更を持たない版で上書きする。
    fn ledger<'s>(
        &self,
        stores: &'s mut BTreeMap<PathBuf, LedgerStore>,
        browser: Browser,
        directory: &str,
    ) -> anyhow::Result<&'s mut LedgerStore> {
        let path = self
            .dir
            .join(browser.name())
            .join(format!("{directory}.ledger"));
        if !stores.contains_key(&path) {
            let store = LedgerStore::open(path.clone())?;
            if store.quarantined() {
                self.quarantined.set(self.quarantined.get() + 1);
            }
            stores.insert(path.clone(), store);
        }
        stores.get_mut(&path).context("開いた帳面が見つからない")
    }
}

/// 消えた訪問を `vanished` にして積む分へ足し、帳面から外す（保存は積み終えた後）。
/// 積んだ後・保存の前に落ちても、識別子が中身から決まるのでやり直しで畳まれる（D6）。
fn queue_vanished(
    store: &mut LedgerStore,
    browser: Browser,
    directory: &str,
    vanished: &[VanishedVisit],
    wall: DateTime<Utc>,
    items: &mut Vec<Visit>,
) -> anyhow::Result<()> {
    for chunk in vanished_chunks(vanished) {
        items.push(Visit::vanished(browser, directory, wall, chunk)?);
    }
    apply_vanished(store.ledger_mut(), vanished);
    Ok(())
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
            let mut queue = |v: &[Visit], _: DateTime<Utc>| {
                queued.extend_from_slice(v);
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

    /// 除外の登録が読めないときは、空の登録で送らない —— 訪問を 1 件も積まず、取得は失敗（D11。R72）。
    #[test]
    fn history_exclusion_broken_registration_sends_nothing() {
        let fx = Fixture::new("INSERT INTO visits VALUES(1,1,13402627200000000,0,1);");
        std::fs::create_dir_all(fx.state()).unwrap();
        std::fs::write(
            fx.state().join("exclusions.json"),
            r#"[{"kind":"no-such-kind","value":"x"}]"#,
        )
        .unwrap();
        let mut collector = fx.collector();
        let (result, queued) = fetch(&mut collector, Utc::now());
        assert!(result.is_err(), "壊れた登録で取得を成功にした");
        assert!(queued.is_empty(), "壊れた登録で積んだ: {}", queued.len());
        assert_eq!(collector.schedule.last_success(), None);
    }

    /// 読めないプロファイルが 1 つでもあれば、取得は成功にならず前回の成功も進まない（design D3。R72）。
    #[test]
    fn history_unreadable_profile_does_not_advance_success() {
        let fx = Fixture::new("INSERT INTO visits VALUES(1,1,13402627200000000,0,1);");
        let other = fx.root.join("local/Google/Chrome/User Data/Profile 2");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("History"), b"not a sqlite file").unwrap();
        let mut collector = fx.collector();
        let (result, _) = fetch(&mut collector, Utc::now());
        assert!(result.is_err(), "読めないプロファイルがあるのに成功にした");
        assert_eq!(collector.schedule.last_success(), None);
        assert!(!fx
            .state()
            .join("browser-history/last_success.json")
            .exists());
    }

    /// 未送信へ積めなかった取得は、帳面も前回の成功も書かず、生存信号には「送る準備を終えられなかった」と渡す。
    /// 読めなかったプロファイルだけが理由の失敗は、送る準備の失敗にしない（deep.md 第 5 回 Q10）。
    #[test]
    fn history_not_queued_writes_no_ledger_and_is_reported() {
        let fx = Fixture::new("INSERT INTO visits VALUES(1,1,13402627200000000,0,1);");
        let mut collector = fx.collector();
        let started = std::time::Instant::now();
        let result = loop {
            let mut queue = |_: &[Visit], _: DateTime<Utc>| anyhow::bail!("置き場へ書けない");
            if let Some(r) = collector.tick(Utc::now(), &mut queue) {
                break r;
            }
            assert!(started.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        assert!(result.is_err());
        assert!(
            !fx.state()
                .join("browser-history/chrome/Default.ledger")
                .exists(),
            "積めないのに帳面を書いた"
        );
        assert!(!fx
            .state()
            .join("browser-history/last_success.json")
            .exists());
        let report = collector.take_report().unwrap();
        assert!(!report.queued);
        assert!(report.profiles.iter().all(|p| p.readable));

        let other = fx.root.join("local/Google/Chrome/User Data/Profile 2");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("History"), b"not a sqlite file").unwrap();
        let (result, _) = fetch(&mut collector, Utc::now() + chrono::Duration::hours(2));
        assert!(result.is_err());
        assert!(
            collector.take_report().unwrap().queued,
            "読めないだけの取得を送る準備の失敗にした"
        );
        assert!(
            fx.state()
                .join("browser-history/chrome/Default.ledger")
                .is_file(),
            "読めたプロファイルの帳面を書いていない"
        );
    }

    fn reader(root: &Path) -> FsHistoryReader {
        FsHistoryReader::new(root.join("local"), root.join("roaming"))
    }

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("ashiato-absent-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("local")).unwrap();
        std::fs::create_dir_all(root.join("roaming")).unwrap();
        root
    }

    /// Opera の主プロファイル（置き場の直下を `Default` と読む）は、置き場が在れば無くなっていない（R71）。
    #[test]
    fn history_vanished_opera_main_profile_is_its_place() {
        let root = temp_root();
        let r = reader(&root);
        let opera = Browser::Opera.base(&root.join("local"), &root.join("roaming"));
        std::fs::create_dir_all(&opera).unwrap();
        assert!(!r.profile_dir_absent(Browser::Opera, "Default"));
        std::fs::remove_dir_all(&opera).unwrap();
        assert!(r.profile_dir_absent(Browser::Opera, "Default"));
        std::fs::remove_dir_all(root).ok();
    }

    /// Firefox の ini に絶対パスで書かれたプロファイルは、そのパスで確かめる（R71）。
    #[test]
    fn history_vanished_firefox_absolute_profile_is_checked_at_its_path() {
        let root = temp_root();
        let r = reader(&root);
        let elsewhere = root.join("D/ffprof.work");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::create_dir_all(root.join("roaming/Mozilla/Firefox/Profiles")).unwrap();
        std::fs::write(
            root.join("roaming/Mozilla/Firefox/profiles.ini"),
            format!(
                "[Profile0]\nName=work\nIsRelative=0\nPath={}\n",
                elsewhere.display()
            ),
        )
        .unwrap();
        assert!(!r.profile_dir_absent(Browser::Firefox, "ffprof.work"));
        std::fs::remove_dir_all(&elsewhere).unwrap();
        assert!(r.profile_dir_absent(Browser::Firefox, "ffprof.work"));
        std::fs::remove_dir_all(root).ok();
    }

    /// 置き場ごと消えた（データも消すアンインストール）ら無くなった。探す先の根が無いときは、無いとは言えない（R71）。
    #[test]
    fn history_vanished_whole_place_removed_is_gone() {
        let root = temp_root();
        let r = reader(&root);
        let base = Browser::Chrome.base(&root.join("local"), &root.join("roaming"));
        std::fs::create_dir_all(base.join("Default")).unwrap();
        assert!(!r.profile_dir_absent(Browser::Chrome, "Default"));
        std::fs::remove_dir_all(&base).unwrap();
        assert!(r.profile_dir_absent(Browser::Chrome, "Default"));
        std::fs::remove_dir_all(root.join("local")).unwrap();
        assert!(
            !r.profile_dir_absent(Browser::Chrome, "Default"),
            "探す先の根が無いのに無くなったとした"
        );
        std::fs::remove_dir_all(root).ok();
    }

    /// 在るかどうか確かめられないときは「在る」と読む（R72）。
    /// 置き場がファイルだと、その下を確かめる問い合わせが「無い」でなく失敗を返す（Linux の ENOTDIR）。
    #[cfg(unix)]
    #[test]
    fn history_vanished_unknown_existence_is_present() {
        let root = temp_root();
        let r = reader(&root);
        let base = Browser::Chrome.base(&root.join("local"), &root.join("roaming"));
        std::fs::create_dir_all(base.parent().unwrap()).unwrap();
        std::fs::write(&base, b"").unwrap();
        assert!(!r.profile_dir_absent(Browser::Chrome, "Default"));
        std::fs::remove_dir_all(root).ok();
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
