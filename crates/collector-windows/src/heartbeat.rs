// SPDX-License-Identifier: AGPL-3.0-only
//! 生存信号（FR-78 / design D4）。**記録が 0 件の期間の意味を残す唯一の材料。**
//!
//! FR-33 は記録を生成したときにしか稼働記録を書かないので、記録が 0 件の日は
//! 行が無いだけになり、扉 #14 が求める「動きが無かったのか / 壊れていたのか」を
//! 区別できない。**区別は遡って作れない。**
//!
//! 間隔は**登録簿の想定間隔（`EXPECTED_GAP_SEC` = 6 時間）**に合わせる ——
//! ずらすと受け手の判定窓とずれ、正常な運用が「途絶」に見える（FR-80）。
use anyhow::Context as _;
use chrono::{DateTime, Duration, Utc};

use crate::contract::{rfc3339, HeartbeatRequest};
use crate::{EXPECTED_GAP_SEC, LOGICAL_SOURCE};

/// 満たされていないものの名前（design D4）。**PC 側は 3 つ。**
pub mod blocker {
    /// 前景のウィンドウが読めない
    pub const FOREGROUND: &str = "foreground";
    /// URL を読み取る経路（UI Automation）が応答しない
    pub const UIAUTOMATION: &str = "uiautomation";
    /// 最後の入力からの経過時間が読めない（**離席・ロック・スリープが残らない**。R24）
    pub const IDLE: &str = "idle";
}

/// 取得できる状態かと、その理由。
///
/// **理由の無い「取れない」は組み立てられない** —— 受け口が
/// `missing_blockers` で断るので、ここで作れてしまうと未送信に居座り続ける。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    /// 取得できる状態か
    pub capturable: bool,
    /// 満たされていないもの
    pub blockers: Vec<String>,
}

impl Capability {
    /// 2 つの状態から組み立てる（design D4）。**1 つでも欠けていれば「取れない」。**
    ///
    /// URL の経路を取得可否に含めるのは、深掘り Q4 で
    /// 「URL はアドレスバーから読む」と決めたから —— **UI Automation が
    /// 応答しなければ URL が取れず、取れなかった URL は後から作れない。**
    pub fn of(foreground_readable: bool, url_path_responds: bool) -> Self {
        let mut blockers = Vec::new();
        if !foreground_readable {
            blockers.push(blocker::FOREGROUND.to_string());
        }
        if !url_path_responds {
            blockers.push(blocker::UIAUTOMATION.to_string());
        }
        Self {
            capturable: blockers.is_empty(),
            blockers,
        }
    }
}

impl Capability {
    /// 区間の間に一度でも満たされなかったものの**和**から組み立てる（R26）。
    ///
    /// 信号を出す瞬間の 1 観測で決めると、6 時間ずっと UI Automation が死んでいても
    /// その 1 秒だけ読めれば「取得できる状態」と報告される —— NFR-13 の訂正 (2) が
    /// 名指しした「壊れているのに動いていたと残る」と同じ型。
    pub fn from_blockers(blockers: &std::collections::BTreeSet<String>) -> Self {
        Self {
            capturable: blockers.is_empty(),
            blockers: blockers.iter().cloned().collect(),
        }
    }
}

/// 前回の信号からの取得の試行と成功の数え。
///
/// **プロセスの立て直しをまたいで残す**（ST01 の review R16 / C-2 / I8）——
/// 一緒に新品になると、**死んでいた区間そのものが観測から落ちる**
/// （6 時間のうち 5 時間 50 分死んで 10 分前に立て直されると、
/// 次の信号は `10 / 10` で「取得率 100 %」になる）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Counters {
    /// この数えが始まった時刻
    pub since: DateTime<Utc>,
    /// 取得を試みた回数
    pub attempts: i32,
    /// そのうち成功した回数
    pub successes: i32,
}

impl Counters {
    /// 数え始める。
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            since: now,
            attempts: 0,
            successes: 0,
        }
    }

    /// 1 回の見回りを数える。**成功は試行を超えない**（構造で守る）。
    pub fn record(&mut self, succeeded: bool) {
        self.attempts += 1;
        if succeeded {
            self.successes += 1;
        }
    }

    /// 数えを取り出して**戻す**（区間ごとの取得率にするため）。
    ///
    /// 戻さないと「導入以来の累計」になり、眠っていた区間が薄まって見えなくなる。
    pub fn take(&mut self, now: DateTime<Utc>) -> (i32, i32) {
        let v = (self.attempts, self.successes);
        *self = Self::new(now);
        v
    }
}

/// 数えの置き場。**起動をまたいで残す。**
#[derive(Debug, Clone)]
pub struct CounterStore {
    path: std::path::PathBuf,
}

impl CounterStore {
    /// 置き場を決める。
    pub fn new(state_dir: &std::path::Path) -> Self {
        Self {
            path: state_dir.join("counters.json"),
        }
    }

    /// 読む。**無ければ `None`**（初回起動）。
    pub fn load(&self) -> anyhow::Result<Option<Counters>> {
        match std::fs::read_to_string(&self.path) {
            Ok(t) => Ok(Some(serde_json::from_str(&t).context("数えが読めない")?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context("数えを読めない"),
        }
    }

    /// 読む。**壊れていたら退避して `None` から始める**（R18 / I7）。
    ///
    /// 印（`Marker`）と違って空に倒してよい —— 失うのは 1 区間の取得率だけで、
    /// 起動を止めると**以後の全部**を失う。退避したかどうかを返す（ログに出す）。
    pub fn load_or_quarantine(&self) -> (Option<Counters>, bool) {
        match self.load() {
            Ok(c) => (c, false),
            Err(_) => {
                let _ = std::fs::rename(&self.path, self.path.with_extension("broken.json"));
                (None, true)
            }
        }
    }

    /// 書く。**一時ファイル + 置き換え**（書きかけで電源が落ちても空にならない。R18）。
    pub fn save(&self, c: &Counters) -> anyhow::Result<()> {
        crate::fsutil::atomic_write(&self.path, serde_json::to_string(c)?.as_bytes())
            .context("数えを書けない")
    }
}

/// 信号を出す契機（想定間隔ごと）。
#[derive(Debug, Clone)]
pub struct Schedule {
    interval: Duration,
    last: Option<DateTime<Utc>>,
}

impl Schedule {
    /// 登録簿の想定間隔で作る。**起動直後は 1 回出す**（動き始めたことを残すため）。
    pub fn new() -> Self {
        Self {
            interval: Duration::seconds(EXPECTED_GAP_SEC),
            last: None,
        }
    }

    /// 間隔を明示して作る（テスト用。**本番では登録簿の値から動かさない**）。
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            interval,
            last: None,
        }
    }

    /// いま出す契機か。
    pub fn due(&self, now: DateTime<Utc>) -> bool {
        match self.last {
            None => true,
            Some(last) => now - last >= self.interval,
        }
    }

    /// 出したことを覚える。
    pub fn mark(&mut self, now: DateTime<Utc>) {
        self.last = Some(now);
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

/// 信号 1 件を組み立てる。
///
/// **原文に私的な内容を入れない**（載るのは稼働・取得可否・回数だけ）。
pub fn signal(
    user_id: uuid::Uuid,
    device_id: &str,
    at: DateTime<Utc>,
    cap: &Capability,
    attempts: i32,
    successes: i32,
) -> anyhow::Result<HeartbeatRequest> {
    let raw = serde_json::json!({
        "alive": true,
        "capturable": cap.capturable,
        "blockers": cap.blockers,
        "attempts": attempts,
        "successes": successes,
    });
    Ok(HeartbeatRequest {
        id: uuid::Uuid::new_v4(),
        user_id,
        logical_source: LOGICAL_SOURCE.to_string(),
        device_id: device_id.to_string(),
        emitted_at: rfc3339(at),
        capturable: cap.capturable,
        blockers: cap.blockers.clone(),
        attempts,
        successes,
        raw: serde_json::to_string(&raw)?,
    })
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

    /// **間隔は登録簿の想定間隔と同じ**（tasks 7.1）。
    ///
    /// **注入した時計で進める** —— smoke は数十秒で終わるので、
    /// 間隔を守らない実装でも必ず緑になる（spec-review R8）。
    ///
    /// Scenario: 想定間隔ごとに生存信号が届く
    #[test]
    fn heartbeat_interval_is_expected_gap() {
        assert_eq!(
            EXPECTED_GAP_SEC, 21_600,
            "登録簿の c02-window.expected_gap_sec と違う（FR-80 の判定窓とずれる）"
        );
        let mut s = Schedule::new();
        assert!(s.due(t(0)), "起動直後に 1 件も出ない");
        s.mark(t(0));
        assert!(!s.due(t(EXPECTED_GAP_SEC - 1)), "想定間隔より早く出ている");
        assert!(s.due(t(EXPECTED_GAP_SEC)), "想定間隔が来ても出ない");
        s.mark(t(EXPECTED_GAP_SEC));
        assert!(!s.due(t(EXPECTED_GAP_SEC + 1)));
        // 1 日動かし続けると 4 件（起動直後の 1 件 + 6 時間ごとに 3 件）
        let mut s = Schedule::new();
        let mut sent = 0;
        for sec in (0..86_400).step_by(60) {
            if s.due(t(sec)) {
                s.mark(t(sec));
                sent += 1;
            }
        }
        assert_eq!(sent, 4, "1 日で出る件数が想定間隔と合わない");
    }

    /// **取得できない理由を名前で載せる**（design D4）。
    ///
    /// Scenario: 前景を読めない状態は取得できないとして報告される
    /// Scenario: URL を読めない状態も取得できないとして報告される
    #[test]
    fn capturable_blockers() {
        let ok = Capability::of(true, true);
        assert!(ok.capturable && ok.blockers.is_empty());

        let no_fg = Capability::of(false, true);
        assert!(!no_fg.capturable);
        assert_eq!(no_fg.blockers, [blocker::FOREGROUND]);

        let no_uia = Capability::of(true, false);
        assert!(!no_uia.capturable);
        assert_eq!(
            no_uia.blockers,
            [blocker::UIAUTOMATION],
            "URL の読み取り経路が挙がっていない"
        );

        // **理由の無い「取れない」は作れない**（受け口が断る形を作らせない）
        let both = Capability::of(false, false);
        assert!(!both.capturable && both.blockers.len() == 2);
        let sig = signal(uuid::Uuid::nil(), "dev", t(0), &both, 10, 3).unwrap();
        assert!(!sig.capturable && !sig.blockers.is_empty());
    }

    /// **試行回数と成功回数が載り、成功は試行を超えない**（受け口が断る条件）。
    ///
    /// Scenario: 試行回数と成功回数が載る
    /// Scenario: 成功回数は試行回数を超えない
    #[test]
    fn attempts_successes() {
        let mut c = Counters::new(t(0));
        for i in 0..10 {
            c.record(i % 3 != 0);
        }
        assert_eq!((c.attempts, c.successes), (10, 6));
        assert!(c.successes <= c.attempts);

        let sig = signal(
            uuid::Uuid::nil(),
            "dev",
            t(EXPECTED_GAP_SEC),
            &Capability::of(true, true),
            c.attempts,
            c.successes,
        )
        .unwrap();
        assert_eq!((sig.attempts, sig.successes), (10, 6));
        assert!(sig.successes <= sig.attempts, "成功が試行を超えている");

        // 数えは信号ごとに戻る（区間の取得率にするため）
        let taken = c.take(t(EXPECTED_GAP_SEC));
        assert_eq!(taken, (10, 6));
        assert_eq!((c.attempts, c.successes), (0, 0));
        assert_eq!(c.since, t(EXPECTED_GAP_SEC));
    }

    /// 壊れた数えは**退避して起動を続ける**（R18）。
    #[test]
    fn broken_counters_are_quarantined() {
        let dir = std::env::temp_dir().join(format!("ashiato-cnt-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("counters.json"), "").unwrap();
        let store = CounterStore::new(&dir);
        assert_eq!(store.load_or_quarantine(), (None, true));
        assert!(dir.join("counters.broken.json").exists());
        assert_eq!(
            store.load_or_quarantine(),
            (None, false),
            "退避したのにまだ読んでいる"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 区間の間に一度でも欠けたものが残る（R26）。
    #[test]
    fn capability_from_interval_blockers() {
        let mut set = std::collections::BTreeSet::new();
        assert!(Capability::from_blockers(&set).capturable);
        set.insert(blocker::UIAUTOMATION.to_string());
        set.insert(blocker::IDLE.to_string());
        let c = Capability::from_blockers(&set);
        assert!(!c.capturable);
        assert_eq!(c.blockers, [blocker::IDLE, blocker::UIAUTOMATION]);
    }

    /// 数えは**起動をまたいで残る**（ST01 の review R16 と同じ理由）。
    #[test]
    fn counters_survive_restart() {
        let dir = std::env::temp_dir().join(format!("ashiato-cnt-{}", uuid::Uuid::new_v4()));
        let store = CounterStore::new(&dir);
        assert_eq!(store.load().unwrap(), None);
        let mut c = Counters::new(t(0));
        c.record(true);
        c.record(false);
        store.save(&c).unwrap();
        assert_eq!(store.load().unwrap(), Some(c));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 原文に私的な内容が入らない（載るのは稼働・取得可否・回数だけ）。
    #[test]
    fn heartbeat_raw_has_no_private_content() {
        let sig = signal(
            uuid::Uuid::nil(),
            "dev",
            t(0),
            &Capability::of(true, true),
            360,
            359,
        )
        .unwrap();
        let raw: serde_json::Value = serde_json::from_str(&sig.raw).unwrap();
        let keys: Vec<&String> = raw.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            ["alive", "attempts", "blockers", "capturable", "successes"]
                .iter()
                .collect::<Vec<_>>(),
            "生存信号の原文に余計な項目がある"
        );
    }
}
