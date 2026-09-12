// SPDX-License-Identifier: AGPL-3.0-only
//! 観測を記録に変える規則。**OS を触らない** —— だから実機を待たずに確かめられる。
//!
//! ここにあるのは深掘りで本人が決めたことそのもの:
//!
//! | | |
//! |---|---|
//! | Q4 / Q6 | アプリと URL の変化は必ず 1 件。題名だけの変化に最小滞留（design D8・仮） |
//! | Q7 | 入力が無い状態への出入りを 1 件として残す（design D9 / D14） |
//! | Q5 | 除外された対象は本文を残さず、除外した件数だけを残す（design D11 / D18） |
//!
//! **間引くのは題名だけ。** 短時間しか前景に無かったアプリを落とすと
//! 「使っていない」と読めてしまう（spec）。
//!
//! **プロセスをまたぐ状態（離席の区間・除外の数え）は `EngineState` として外へ出す**
//! （design D21）。メモリだけに持つと、落ちた瞬間に「入った」だけが残って永久に閉じない。
use chrono::{DateTime, Duration, Utc};

use crate::contract::{rfc3339, AwayReason, EndedBy, RecordKind, Transition, WindowPayload};
use crate::exclusion::Exclusions;

/// 題名だけの変化の最小滞留（design D8・**仮**）。
///
/// **反転条件**: 実測した年間の容量が NFR-5 の枠（1 日 5,990〜15,200 件）に対して
/// 大きく余る / 足りないと分かったら変える。
pub const MIN_DWELL_SEC: i64 = 5;

/// 入力が無いとみなす閾値（design D9・**仮**）。
///
/// **反転条件**: 短い離席が記録を埋めると分かったら延ばす。
/// 出入りの時刻と経過時間が記録に載るので、**何分で切るかは後から引き直せる**。
pub const IDLE_THRESHOLD_SEC: i64 = 300;

/// 前景の 1 つの状態。**OS から取れたものをそのまま持つ**（補正しない。design D12）。
#[derive(Clone, PartialEq, Eq)]
pub struct Foreground {
    /// アプリの表示名
    pub app_name: String,
    /// 実行ファイルのパス
    pub exe_path: String,
    /// プロセス名
    pub process_name: String,
    /// ウィンドウ題名
    pub title: String,
    /// アドレスバーの読み取り結果
    pub url: UrlRead,
}

/// **本文を出さない**（review/code.md R35）。`{:?}` 1 つで題名と URL がログに落ちる。
impl std::fmt::Debug for Foreground {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Foreground { .. }")
    }
}

/// アドレスバーを読めたか（design D4）。
///
/// **「読めなかった」と「ブラウザではない」を区別する。** 混ぜると、
/// UI Automation が死んでいても生存信号が「取得できる状態」と報告する。
#[derive(Clone, PartialEq, Eq)]
pub enum UrlRead {
    /// 前景がブラウザではない（読む必要が無い）
    NotBrowser,
    /// 読めた。**見えている文字列そのまま**（深掘り Q4）
    Read(String),
    /// 前景はブラウザだが、UI Automation が応答しない（または空を返した）
    Unavailable,
}

impl std::fmt::Debug for UrlRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotBrowser => "NotBrowser",
            Self::Read(_) => "Read(..)",
            Self::Unavailable => "Unavailable",
        })
    }
}

/// 最後の入力からの経過時間を読めたか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleRead {
    /// 読めた
    Elapsed(Duration),
    /// 読めない。**入力があったとも無かったとも読まない**
    Unavailable,
}

/// 1 回の見回りで見えたもの（design D5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// 見た時刻
    pub at: DateTime<Utc>,
    /// 前景。**`None` は「読めなかった」**（ロック中は `locked` が立つ）
    pub foreground: Option<Foreground>,
    /// 最後の入力からの経過時間
    pub idle: IdleRead,
    /// 画面がロックされているか
    pub locked: bool,
}

/// いま前景として記録してある状態。
#[derive(Clone, PartialEq, Eq)]
struct Shown {
    app_name: String,
    exe_path: String,
    process_name: String,
    title: String,
    url: Option<String>,
    /// **比較に使わない**（読めたり読めなかったりの揺れを「変化」にしないため。R25）
    url_unavailable: bool,
}

impl Shown {
    /// 観測から作る。**URL が読めなかったときは、同じアプリの間だけ前の URL を持ち越す**
    /// —— 持ち越さないと、同じページを見ているだけで「URL が変わった」記録が出る（R25）。
    fn of(fg: &Foreground, prev: Option<&Shown>) -> Self {
        let same_app = prev.is_some_and(|p| {
            p.app_name == fg.app_name
                && p.exe_path == fg.exe_path
                && p.process_name == fg.process_name
        });
        let (url, url_unavailable) = match &fg.url {
            UrlRead::Read(u) => (Some(u.clone()), false),
            UrlRead::NotBrowser => (None, false),
            UrlRead::Unavailable => (prev.filter(|_| same_app).and_then(|p| p.url.clone()), true),
        };
        Self {
            app_name: fg.app_name.clone(),
            exe_path: fg.exe_path.clone(),
            process_name: fg.process_name.clone(),
            title: fg.title.clone(),
            url,
            url_unavailable,
        }
    }

    fn key(&self) -> (&str, &str, &str, &str, Option<&str>) {
        (
            &self.app_name,
            &self.exe_path,
            &self.process_name,
            &self.title,
            self.url.as_deref(),
        )
    }

    /// 題名以外が同じか。**ここが「題名だけの変化」の定義**（design D8）。
    fn same_except_title(&self, other: &Self) -> bool {
        let (a, b) = (self.key(), other.key());
        a.0 == b.0 && a.1 == b.1 && a.2 == b.2 && a.4 == b.4
    }
}

/// 離席の区間。**プロセスをまたいで残す**（design D21）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Away {
    /// 入った時刻
    pub since: DateTime<Utc>,
    /// 理由
    pub reason: AwayReason,
}

/// 除外の数え。**本文（題名・URL）は持たない**ので、そのまま置き場に落としてよい。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExcludedCount {
    /// 数え始めた時刻
    pub since: DateTime<Utc>,
    /// まだ記録にしていない変化の回数
    pub count: u32,
}

/// プロセスをまたいで残す状態（design D21）。**本文を 1 文字も含まない。**
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EngineState {
    /// 開いている離席の区間
    pub away: Option<Away>,
    /// 数え途中の除外
    pub excluded: Option<ExcludedCount>,
}

/// 観測を記録に変える。**状態を持つ**（前の観測との差が記録になるため）。
pub struct Engine {
    exclusions: Exclusions,
    min_dwell: Duration,
    idle_threshold: Duration,
    shown: Option<Shown>,
    /// 滞留を待っている題名だけの変化と、それが前景に現れた時刻
    pending: Option<(Shown, DateTime<Utc>)>,
    away: Option<Away>,
    excluded: Option<ExcludedCount>,
    /// いま除外している対象の見分け。**手元でだけ持ち、置き場にも記録にも出さない**
    excluded_key: Option<(String, String, Option<String>)>,
    /// 最後に経過時間が読めた時刻と、そのときの経過時間
    last_idle: Option<(DateTime<Utc>, Duration)>,
    /// 経過時間が読めなくなった時刻
    idle_unreadable_since: Option<DateTime<Utc>>,
}

/// **本文を出さない**（R35）。状態の有無だけを出す。
impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("rules", &self.exclusions.rules.len())
            .field("shown", &self.shown.is_some())
            .field("pending", &self.pending.is_some())
            .field("away", &self.away)
            .field("excluded", &self.excluded)
            .finish()
    }
}

impl Engine {
    /// 既定の値（design D8 / D9 の**仮**）で作る。
    pub fn new(exclusions: Exclusions) -> Self {
        Self::with_thresholds(
            exclusions,
            Duration::seconds(MIN_DWELL_SEC),
            Duration::seconds(IDLE_THRESHOLD_SEC),
        )
    }

    /// 値を明示して作る（テストと、**反転条件が満たされたとき**のため）。
    pub fn with_thresholds(
        exclusions: Exclusions,
        min_dwell: Duration,
        idle_threshold: Duration,
    ) -> Self {
        Self {
            exclusions,
            min_dwell,
            idle_threshold,
            shown: None,
            pending: None,
            away: None,
            excluded: None,
            excluded_key: None,
            last_idle: None,
            idle_unreadable_since: None,
        }
    }

    /// 置き場に落とす状態。
    pub fn state(&self) -> EngineState {
        EngineState {
            away: self.away,
            excluded: self.excluded,
        }
    }

    /// 前回のプロセスが残した状態を**閉じる**（design D21 / review/code.md R1 / R4）。
    ///
    /// `last_seen` は「ここまで動いていた」の印。**開いていた離席はそこで閉じ、
    /// 数え途中の除外はそこまでの件数として残す** —— 閉じ方が違うことは
    /// `ended_by: restart` で読める（「いつ戻ったか」として読ませない）。
    pub fn close_previous(
        &self,
        previous: EngineState,
        last_seen: Option<DateTime<Utc>>,
    ) -> Vec<WindowPayload> {
        let mut out = Vec::new();
        let Some(end) = last_seen else {
            return out;
        };
        if let Some(away) = previous.away {
            let end = end.max(away.since);
            let mut p = leave_record(away.since, end, away.reason);
            p.ended_by = Some(EndedBy::Restart);
            out.push(p);
        }
        if let Some(ex) = previous.excluded.filter(|e| e.count > 0) {
            let mut p = WindowPayload::new(RecordKind::Excluded, ex.since);
            p.excluded_count = Some(ex.count);
            p.range_end = Some(rfc3339(end.max(ex.since)));
            out.push(p);
        }
        out
    }

    /// 1 回の観測を食わせ、生まれた記録を返す。
    ///
    /// **順序が意味を持つ** —— 離席の出入りを先に見る（前景が変わらないまま
    /// 席を離れた場合に記録が 1 件も出ないのを防ぐ）。
    pub fn observe(&mut self, obs: Observation) -> Vec<WindowPayload> {
        let mut out = self.observe_away(&obs);
        out.extend(self.observe_foreground(&obs));
        out
    }

    /// 入力が無い状態への出入り（FR-81 / design D14）。
    fn observe_away(&mut self, obs: &Observation) -> Vec<WindowPayload> {
        let mut out = Vec::new();
        let elapsed = match obs.idle {
            IdleRead::Elapsed(d) => {
                self.last_idle = Some((obs.at, d));
                self.idle_unreadable_since = None;
                Some(d)
            }
            IdleRead::Unavailable => {
                self.idle_unreadable_since.get_or_insert(obs.at);
                None
            }
        };

        let now_away = if obs.locked {
            Some(AwayReason::Locked)
        } else {
            match (elapsed, self.away) {
                (Some(d), _) if d >= self.idle_threshold => Some(AwayReason::Idle),
                (Some(_), _) => None,
                // ロックは読めている（`locked == false`）ので、ロックの区間は閉じてよい
                (None, Some(a)) if a.reason == AwayReason::Locked => None,
                (None, Some(a)) => {
                    // **読めない状態が閾値ぶん続いたら、最後に読めた時刻で閉じる**（R24）。
                    // 開いたままにすると、読めないまま落ちた区間が永久に閉じない
                    let since = self.idle_unreadable_since.unwrap_or(obs.at);
                    if obs.at - since >= self.idle_threshold {
                        let end = self.last_idle.map_or(since, |(t, _)| t).max(a.since);
                        let mut p = leave_record(a.since, end, a.reason);
                        p.ended_by = Some(EndedBy::Unreadable);
                        out.push(p);
                        self.away = None;
                    }
                    return out;
                }
                (None, None) => return out,
            }
        };

        match (self.away, now_away) {
            (None, Some(reason)) => {
                let since = enter_time(reason, elapsed, obs.at);
                out.push(enter_record(since, reason, elapsed));
                self.away = Some(Away { since, reason });
            }
            (Some(away), None) => {
                // 出た。**入力が戻った時刻まで戻す**
                let resumed = elapsed.map_or(obs.at, |d| obs.at - d).max(away.since);
                out.push(leave_record(away.since, resumed, away.reason));
                self.away = None;
            }
            (Some(away), Some(reason)) if away.reason != reason => {
                // **理由が入れ替わった**（離席のまま画面が自動でロックされた、など。R23）。
                // 前の区間をここで閉じて、新しい理由で入り直す —— 黙って続けると
                // ロックした時刻がどこにも残らない
                let mut p = leave_record(away.since, obs.at.max(away.since), away.reason);
                p.ended_by = Some(EndedBy::Superseded);
                out.push(p);
                out.push(enter_record(obs.at, reason, elapsed));
                self.away = Some(Away {
                    since: obs.at,
                    reason,
                });
            }
            _ => {}
        }
        out
    }

    /// 前景の変化（FR-12）と除外（FR-83）。
    fn observe_foreground(&mut self, obs: &Observation) -> Vec<WindowPayload> {
        let mut out = Vec::new();
        let Some(fg) = obs.foreground.as_ref() else {
            // 読めなかった。**記録を作らない**（作ると「前景が無かった」ことになる）
            return out;
        };

        if self.exclusions.hits(fg) {
            // 除外。**本文はここから先へ 1 文字も出さない**（FR-83 / design D11）。
            // 滞留を満たしていた題名は、除外に入る前に記録する（取りこぼさない側。R40）
            out.extend(self.take_dwelled_pending(obs.at));
            self.pending = None;
            self.shown = None;
            let key = (
                fg.exe_path.clone(),
                fg.title.clone(),
                match &fg.url {
                    UrlRead::Read(u) => Some(u.clone()),
                    _ => None,
                },
            );
            // **除外の対象を前景にしたこと自体を 1 回と数える**（design D18 / R37）
            if self.excluded_key.as_ref() != Some(&key) {
                let span = self.excluded.get_or_insert(ExcludedCount {
                    since: obs.at,
                    count: 0,
                });
                span.count += 1;
                self.excluded_key = Some(key);
            }
            return out;
        }

        // 除外の対象から出た。**数えを記録にする**（除外された時間帯と
        // 触っていなかった時間帯を区別できるようにするため）
        out.extend(self.close_excluded(obs.at, false));

        let next = Shown::of(fg, self.shown.as_ref());
        match self.shown.clone() {
            None => {
                out.push(foreground_record(&next, obs.at));
                self.shown = Some(next);
                self.pending = None;
            }
            Some(cur) if cur.key() == next.key() => {
                // 何も変わっていない。**待っていた題名が元に戻ったら捨てる**
                self.pending = None;
            }
            Some(cur) if cur.same_except_title(&next) => {
                // 題名だけの変化。滞留を待つ（design D8）
                match self.pending.clone() {
                    Some((p, since)) if p.key() == next.key() => {
                        if obs.at - since >= self.min_dwell {
                            // **現れた時刻で記録する** —— 見つけた時刻にすると
                            // 滞留のぶん記録が後ろへずれる
                            out.push(foreground_record(&next, since));
                            self.shown = Some(next);
                            self.pending = None;
                        }
                    }
                    // 別の題名に変わった。**滞留を満たしていた題名は残し**、
                    // 満たしていなかった題名は間引く（Q6 の決定）
                    _ => {
                        out.extend(self.take_dwelled_pending(obs.at));
                        self.pending = Some((next, obs.at));
                    }
                }
            }
            Some(_) => {
                // アプリか URL の変化。**滞留に関わらず必ず 1 件**（Q6 の決定）
                out.extend(self.take_dwelled_pending(obs.at));
                out.push(foreground_record(&next, obs.at));
                self.shown = Some(next);
                self.pending = None;
            }
        }
        out
    }

    /// 滞留を満たしていた題名があれば記録にする（R40）。
    fn take_dwelled_pending(&mut self, now: DateTime<Utc>) -> Vec<WindowPayload> {
        match self.pending.take() {
            Some((p, since)) if now - since >= self.min_dwell => {
                let r = foreground_record(&p, since);
                self.shown = Some(p);
                vec![r]
            }
            _ => Vec::new(),
        }
    }

    /// 見回りの時刻が飛んだ（PC が眠っていた。design D19）。
    ///
    /// **`from` は最後に見回れた時刻**で、`to` は目覚めた時刻。`mono_gap` はその間に
    /// 単調時計が進んだ長さ —— 壁時計の飛びと比べれば「止まっていた」と
    /// 「時計だけが進んだ」を後から分けられる（review/code.md R17）。
    pub fn report_suspend(
        &mut self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        mono_gap: Option<Duration>,
    ) -> Vec<WindowPayload> {
        let mut out = Vec::new();
        if let Some(away) = self.away.take() {
            // 既に離席していた区間は、眠りに入った時刻で閉じる
            let mut p = leave_record(away.since, from.max(away.since), away.reason);
            p.ended_by = Some(EndedBy::Superseded);
            out.push(p);
        }
        // **眠っていた間を滞留に数えない**（R40）
        self.pending = None;
        // 入った側にも最後に読めた経過時間を載せる（spec の SHALL。R13）
        let elapsed = self
            .last_idle
            .map(|(t, d)| d + (from - t).max(Duration::zero()));
        let mut enter = enter_record(from, AwayReason::Suspended, elapsed);
        enter.mono_gap_ms = mono_gap.map(|g| g.num_milliseconds());
        out.push(enter);
        let mut leave = leave_record(from, to.max(from), AwayReason::Suspended);
        leave.mono_gap_ms = mono_gap.map(|g| g.num_milliseconds());
        out.push(leave);
        out
    }

    /// 手元に抱えているものを記録にする（送信の契機ごと・終了時）。
    ///
    /// **除外の数えを抱えたまま落ちると、その件数は後から作れない。**
    /// 除外の対象を見続けている間は、**区間を閉じずに数えだけを 0 から数え直す**（R2）。
    pub fn flush(&mut self, now: DateTime<Utc>) -> Vec<WindowPayload> {
        let mut out = self.close_excluded(now, true);
        if let Some((p, since)) = self.pending.clone() {
            if now - since >= self.min_dwell {
                out.push(foreground_record(&p, since));
                self.shown = Some(p);
                self.pending = None;
            }
        }
        out
    }

    /// 除外の数えを記録にする。`keep_open` なら区間を続ける（見続けている間の吐き出し）。
    fn close_excluded(&mut self, at: DateTime<Utc>, keep_open: bool) -> Vec<WindowPayload> {
        let Some(span) = self.excluded else {
            return Vec::new();
        };
        let emit = span.count > 0 && at > span.since;
        let mut out = Vec::new();
        if emit {
            let mut p = WindowPayload::new(RecordKind::Excluded, span.since);
            p.excluded_count = Some(span.count);
            p.range_end = Some(rfc3339(at));
            out.push(p);
        }
        if keep_open {
            if emit {
                self.excluded = Some(ExcludedCount {
                    since: at,
                    count: 0,
                });
            }
            // 長さ 0 の区間は作らない —— 数えは次の契機まで持ち越す
        } else if emit || span.count == 0 {
            self.excluded = None;
            self.excluded_key = None;
        } else {
            // 入った瞬間に出た（長さ 0）。**数えは捨てず**、出た時刻を 1 ミリ秒後ろにして残す
            let mut p = WindowPayload::new(RecordKind::Excluded, span.since);
            p.excluded_count = Some(span.count);
            p.range_end = Some(rfc3339(span.since + Duration::milliseconds(1)));
            out.push(p);
            self.excluded = None;
            self.excluded_key = None;
        }
        out
    }
}

fn enter_time(reason: AwayReason, elapsed: Option<Duration>, at: DateTime<Utc>) -> DateTime<Utc> {
    match (reason, elapsed) {
        // **入力が止まった時刻まで戻す** —— 見つけた時刻にすると閾値のぶん遅れる
        (AwayReason::Idle, Some(d)) => at - d,
        // ロックは見つけた時刻が入った時刻（ロックした瞬間に入力は止まる）
        _ => at,
    }
}

fn enter_record(
    since: DateTime<Utc>,
    reason: AwayReason,
    elapsed: Option<Duration>,
) -> WindowPayload {
    let mut p = WindowPayload::new(RecordKind::Idle, since);
    p.transition = Some(Transition::Enter);
    p.reason = Some(reason);
    p.idle_ms = elapsed.map(|d| d.num_milliseconds());
    p
}

fn foreground_record(s: &Shown, at: DateTime<Utc>) -> WindowPayload {
    let mut p = WindowPayload::new(RecordKind::Foreground, at);
    p.app_name = Some(s.app_name.clone());
    p.exe_path = Some(s.exe_path.clone());
    p.process_name = Some(s.process_name.clone());
    p.title = Some(s.title.clone());
    p.url = s.url.clone();
    p.url_unavailable = s.url_unavailable.then_some(true);
    p
}

/// 出た側の記録。**区間（`range_end`）を持つのはこちら**（design D14）——
/// 入った側だけでは、いつ戻ったかが分からない。
fn leave_record(since: DateTime<Utc>, resumed: DateTime<Utc>, reason: AwayReason) -> WindowPayload {
    let mut p = WindowPayload::new(RecordKind::Idle, since);
    p.transition = Some(Transition::Leave);
    p.reason = Some(reason);
    p.range_end = Some(rfc3339(resumed));
    p.idle_ms = Some((resumed - since).num_milliseconds());
    p
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::exclusion::Rule;

    fn base() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-13T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn at(sec: i64) -> DateTime<Utc> {
        base() + Duration::seconds(sec)
    }

    fn fg(app: &str, title: &str, url: UrlRead) -> Foreground {
        Foreground {
            app_name: app.into(),
            exe_path: format!(r"C:\apps\{app}.exe"),
            process_name: format!("{app}.exe"),
            title: title.into(),
            url,
        }
    }

    /// 席にいて前景が読めている観測。
    fn obs(sec: i64, fg: Option<Foreground>) -> Observation {
        Observation {
            at: at(sec),
            foreground: fg,
            idle: IdleRead::Elapsed(Duration::zero()),
            locked: false,
        }
    }

    fn idle_obs(sec: i64, idle_sec: Option<i64>, locked: bool) -> Observation {
        Observation {
            at: at(sec),
            foreground: Some(fg("editor", "文書", UrlRead::NotBrowser)),
            idle: idle_sec.map_or(IdleRead::Unavailable, |s| {
                IdleRead::Elapsed(Duration::seconds(s))
            }),
            locked,
        }
    }

    fn engine() -> Engine {
        Engine::new(Exclusions::default())
    }

    fn kinds(records: &[WindowPayload]) -> Vec<&'static str> {
        records.iter().map(|r| r.kind.as_str()).collect()
    }

    // ------------------------------------------------------------- 前景の変化

    /// Scenario: アプリを切り替えると 1 件増える
    #[test]
    fn foreground_switch_makes_one_record() {
        let mut e = engine();
        assert_eq!(
            e.observe(obs(0, Some(fg("editor", "文書", UrlRead::NotBrowser))))
                .len(),
            1
        );
        let out = e.observe(obs(10, Some(fg("mail", "受信箱", UrlRead::NotBrowser))));
        assert_eq!(out.len(), 1, "切り替えで 1 件にならない");
        assert_eq!(out[0].app_name.as_deref(), Some("mail"));
        assert_eq!(out[0].kind, RecordKind::Foreground);
        assert_eq!(out[0].at, rfc3339(at(10)));
    }

    /// 題名だけの変化は**滞留を超えてから** 1 件になる（design D8）。
    ///
    /// Scenario: 同じアプリの中で題名が変われば 1 件増える
    #[test]
    fn title_change_makes_one_record_after_dwell() {
        let mut e = engine();
        e.observe(obs(0, Some(fg("editor", "文書 A", UrlRead::NotBrowser))));
        let out = e.observe(obs(1, Some(fg("editor", "文書 B", UrlRead::NotBrowser))));
        assert!(out.is_empty(), "滞留を待たずに記録している");
        let out = e.observe(obs(7, Some(fg("editor", "文書 B", UrlRead::NotBrowser))));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title.as_deref(), Some("文書 B"));
        assert_eq!(
            out[0].at,
            rfc3339(at(1)),
            "題名が現れた時刻ではなく、見つけた時刻で記録している"
        );
    }

    /// **本人が決めた「5 秒」を下からも縛る**（review/code.md R7）。
    /// 4 秒とどまった題名は落ち、5 秒とどまった題名は残る。
    #[test]
    fn min_dwell_is_five_seconds_from_both_sides() {
        assert_eq!(MIN_DWELL_SEC, 5, "深掘り Q6 / design D8 の値が変わっている");
        for (stay, kept) in [(4, false), (5, true)] {
            let mut e = engine();
            e.observe(obs(0, Some(fg("editor", "A", UrlRead::NotBrowser))));
            e.observe(obs(10, Some(fg("editor", "B", UrlRead::NotBrowser))));
            let out = e.observe(obs(10 + stay, Some(fg("editor", "B", UrlRead::NotBrowser))));
            assert_eq!(!out.is_empty(), kept, "{stay} 秒とどまった題名の扱いが違う");
        }
    }

    /// Scenario: URL だけが変われば 1 件増える
    #[test]
    fn url_change_makes_one_record() {
        let mut e = engine();
        let page = |u: &str| Some(fg("browser", "同じ題名", UrlRead::Read(u.into())));
        e.observe(obs(0, page("example.com/a")));
        let out = e.observe(obs(1, page("example.com/b")));
        assert_eq!(out.len(), 1, "URL だけの変化が落ちている");
        assert_eq!(out[0].url.as_deref(), Some("example.com/b"));
    }

    /// URL は**クエリもフラグメントも落とさない**（深掘り Q4）。
    ///
    /// Scenario: クエリとフラグメントが残る
    #[test]
    fn url_keeps_query_and_fragment() {
        let mut e = engine();
        let url = "https://example.com/search?q=%E8%B6%B3%E8%B7%A1&page=2#results";
        let out = e.observe(obs(
            0,
            Some(fg("browser", "検索", UrlRead::Read(url.into()))),
        ));
        assert_eq!(out[0].url.as_deref(), Some(url));
    }

    /// **補正しない**（design D12 / 扉 #7）—— `https://` を補わず、省略も展開しない。
    ///
    /// Scenario: 表示されている文字列を補正しない
    #[test]
    fn url_is_not_normalized() {
        let mut e = engine();
        let shown = "example.com";
        let out = e.observe(obs(
            0,
            Some(fg("browser", "頁", UrlRead::Read(shown.into()))),
        ));
        assert_eq!(
            out[0].url.as_deref(),
            Some(shown),
            "見えていない文字を補っている"
        );
        let elided = "example.com/very/long/…/tail";
        let out = e.observe(obs(
            1,
            Some(fg("browser", "頁", UrlRead::Read(elided.into()))),
        ));
        assert_eq!(out[0].url.as_deref(), Some(elided));
    }

    /// **URL の読み取りが揺れても「変化」にしない**。読めなかった記録にはその印が残る（R25）。
    #[test]
    fn url_flapping_is_not_a_change() {
        let mut e = engine();
        let page = |u: UrlRead| Some(fg("browser", "同じ題名", u));
        e.observe(obs(0, page(UrlRead::Read("example.com/a".into()))));
        assert!(
            e.observe(obs(1, page(UrlRead::Unavailable))).is_empty(),
            "読めなかっただけで記録が増えた"
        );
        assert!(e
            .observe(obs(2, page(UrlRead::Read("example.com/a".into()))))
            .is_empty());

        // 最初から読めないブラウザは「URL が無い理由」を印で残す
        let mut e = engine();
        let out = e.observe(obs(0, page(UrlRead::Unavailable)));
        assert_eq!(out[0].url, None);
        assert_eq!(out[0].url_unavailable, Some(true));
        // 読めたら URL の変化として記録する
        let out = e.observe(obs(1, page(UrlRead::Read("example.com/b".into()))));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].url_unavailable, None);
    }

    /// **題名が流れ続けても記録は増えない**（design D8。動画プレイヤー・端末の進捗表示）。
    ///
    /// Scenario: 題名が最小滞留より短く変わり続けても記録は増えない
    #[test]
    fn min_dwell_drops_title_only_churn() {
        let mut e = engine();
        e.observe(obs(0, Some(fg("player", "再生 0:00", UrlRead::NotBrowser))));
        let mut made = 0;
        for i in 1..=10 {
            made += e
                .observe(obs(
                    i,
                    Some(fg("player", &format!("再生 0:{i:02}"), UrlRead::NotBrowser)),
                ))
                .len();
        }
        assert_eq!(made, 0, "題名の流れが記録になっている");
        let out = e.observe(obs(
            20,
            Some(fg("player", "再生 0:10", UrlRead::NotBrowser)),
        ));
        assert_eq!(out.len(), 1, "とどまった題名が記録になっていない");
        assert_eq!(out[0].title.as_deref(), Some("再生 0:10"));
    }

    /// Scenario: アプリの切り替えは滞留時間に関わらず記録される
    #[test]
    fn app_switch_ignores_min_dwell() {
        let mut e = engine();
        e.observe(obs(0, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        let a = e.observe(obs(1, Some(fg("chat", "通知", UrlRead::NotBrowser))));
        let b = e.observe(obs(2, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        assert_eq!((a.len(), b.len()), (1, 1), "短い切り替えが落ちている");
        assert_eq!(a[0].app_name.as_deref(), Some("chat"));
    }

    /// Scenario: URL の変化は滞留時間に関わらず記録される
    #[test]
    fn url_change_ignores_min_dwell() {
        let mut e = engine();
        let page = |u: &str| Some(fg("browser", "題名", UrlRead::Read(u.into())));
        e.observe(obs(0, page("example.com/1")));
        let mut made = 0;
        for i in 1..=3 {
            made += e
                .observe(obs(i, page(&format!("example.com/{}", i + 1))))
                .len();
        }
        assert_eq!(made, 3, "滞留より短い間隔の URL の変化が落ちている");
    }

    /// **滞留を満たした題名は、見回りの前にアプリが変わっても残る**（R40）。
    #[test]
    fn dwelled_title_survives_app_switch() {
        let mut e = engine();
        e.observe(obs(0, Some(fg("editor", "A", UrlRead::NotBrowser))));
        e.observe(obs(1, Some(fg("editor", "B", UrlRead::NotBrowser))));
        // 題名 B は 6 秒前景にあったが、同じ題名のままの見回りが来る前にアプリが変わった
        let out = e.observe(obs(7, Some(fg("mail", "受信箱", UrlRead::NotBrowser))));
        let titles: Vec<_> = out.iter().map(|r| r.title.as_deref()).collect();
        assert_eq!(titles, [Some("B"), Some("受信箱")]);
    }

    /// 前景が読めない観測は**記録を作らない**（生存信号がこれを報告する。design D4）。
    #[test]
    fn unreadable_foreground_makes_no_record() {
        let mut e = engine();
        e.observe(obs(0, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        assert!(e.observe(obs(1, None)).is_empty());
    }

    // ----------------------------------------------------------------- 離席

    /// Scenario: 離席の始まりと終わりが残る
    #[test]
    fn idle_transitions() {
        let mut e = engine();
        e.observe(obs(0, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        let enter = e.observe(idle_obs(400, Some(340), false));
        assert_eq!(kinds(&enter), ["idle"]);
        assert_eq!(enter[0].transition, Some(Transition::Enter));
        assert_eq!(
            enter[0].at,
            rfc3339(at(60)),
            "入った時刻が「入力が止まった時刻」に戻っていない"
        );
        assert_eq!(enter[0].reason, Some(AwayReason::Idle));

        let leave = e.observe(idle_obs(500, Some(1), false));
        assert_eq!(kinds(&leave), ["idle"]);
        assert_eq!(leave[0].transition, Some(Transition::Leave));
        assert_eq!(leave[0].at, rfc3339(at(60)));
        assert_eq!(
            leave[0].range_end.as_deref(),
            Some(rfc3339(at(499)).as_str()),
            "戻った時刻が残っていない"
        );
        assert_eq!(
            leave[0].ended_by, None,
            "入力で戻った区間に閉じ方が付いている"
        );
        assert_eq!(enter.len() + leave.len(), 2);
    }

    /// **閾値ちょうどで入る**（review/code.md I6）。299 秒では入らない。
    #[test]
    fn idle_threshold_boundary() {
        assert_eq!(IDLE_THRESHOLD_SEC, 300);
        let mut e = engine();
        assert!(e.observe_away(&idle_obs(1000, Some(299), false)).is_empty());
        assert_eq!(e.observe_away(&idle_obs(1001, Some(300), false)).len(), 1);
    }

    /// Scenario: 画面ロックとスリープも残る
    #[test]
    fn lock_and_suspend_are_recorded() {
        let mut e = engine();
        e.observe(obs(0, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        let lock = e.observe(Observation {
            at: at(10),
            foreground: None,
            idle: IdleRead::Elapsed(Duration::seconds(2)),
            locked: true,
        });
        assert_eq!(lock[0].reason, Some(AwayReason::Locked));
        assert_eq!(lock[0].transition, Some(Transition::Enter));
        let unlock = e.observe(idle_obs(70, Some(0), false));
        assert_eq!(unlock[0].transition, Some(Transition::Leave));
        assert_eq!(unlock[0].reason, Some(AwayReason::Locked));

        let slept = e.report_suspend(at(100), at(4000), Some(Duration::seconds(3900)));
        assert_eq!(kinds(&slept), ["idle", "idle"]);
        assert_eq!(slept[0].reason, Some(AwayReason::Suspended));
        assert_eq!(
            slept[1].range_end.as_deref(),
            Some(rfc3339(at(4000)).as_str())
        );
        assert_eq!(slept[1].mono_gap_ms, Some(3_900_000));
    }

    /// **離席のまま自動でロックされても、ロックした時刻が残る**（R23）。
    #[test]
    fn idle_then_lock_records_the_lock() {
        let mut e = engine();
        e.observe(idle_obs(400, Some(340), false)); // 離席に入る
        let out = e.observe(idle_obs(900, Some(840), true)); // 画面オフ → ロック
        assert_eq!(out.len(), 2, "ロックへの入れ替わりが記録になっていない");
        assert_eq!(out[0].transition, Some(Transition::Leave));
        assert_eq!(out[0].reason, Some(AwayReason::Idle));
        assert_eq!(out[0].ended_by, Some(EndedBy::Superseded));
        assert_eq!(out[1].transition, Some(Transition::Enter));
        assert_eq!(out[1].reason, Some(AwayReason::Locked));
        assert_eq!(out[1].at, rfc3339(at(900)));
    }

    /// **経過時間が読めないまま閾値ぶん経ったら、最後に読めた時刻で閉じる**（R24）。
    #[test]
    fn unreadable_idle_closes_the_span_eventually() {
        let mut e = engine();
        e.observe(idle_obs(400, Some(340), false)); // 入る
        assert!(
            e.observe(idle_obs(410, None, false)).is_empty(),
            "すぐ閉じている"
        );
        let out = e.observe(idle_obs(710, None, false));
        assert_eq!(out.len(), 1, "読めないまま離席が閉じない");
        assert_eq!(out[0].ended_by, Some(EndedBy::Unreadable));
        assert_eq!(out[0].range_end.as_deref(), Some(rfc3339(at(400)).as_str()));

        // ロックの区間は、ロックが解けたと読めた時点で閉じる（経過時間が読めなくても）
        let mut e = engine();
        e.observe(idle_obs(0, Some(0), true));
        let out = e.observe(idle_obs(60, None, false));
        assert_eq!(out[0].transition, Some(Transition::Leave));
        assert_eq!(out[0].reason, Some(AwayReason::Locked));
    }

    /// **閾値を変えたときに引き直せる**（design D9 の仮決めが可逆であることの根拠）。
    /// 3 種類の理由すべてで「入った」側が経過時間を持つ（R13）。
    ///
    /// Scenario: 閾値を後から引き直せる形で残る
    #[test]
    fn idle_records_carry_elapsed_for_rethreshold() {
        let mut e = engine();
        let enter = e.observe(idle_obs(400, Some(340), false));
        assert_eq!(enter[0].idle_ms, Some(340_000));
        let leave = e.observe(idle_obs(500, Some(0), false));
        // 区間の長さから、閾値を 10 分にした場合の判定を記録だけで引ける
        assert_eq!(leave[0].idle_ms, Some(440_000));

        let mut e = engine();
        let lock = e.observe(idle_obs(10, Some(3), true));
        assert_eq!(lock[0].idle_ms, Some(3_000));

        let mut e = engine();
        e.observe(idle_obs(100, Some(20), false));
        let slept = e.report_suspend(at(110), at(5000), None);
        assert_eq!(
            slept[0].idle_ms,
            Some(30_000),
            "眠りに入った側に経過時間が無い"
        );
    }

    /// Scenario: 離席の記録は前景の記録と区別できる
    #[test]
    fn record_kind_is_distinguishable() {
        let mut e = engine();
        let fgr = e.observe(obs(0, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        let idle = e.observe(idle_obs(400, Some(340), false));
        assert_eq!(fgr[0].kind, RecordKind::Foreground);
        assert_eq!(idle[0].kind, RecordKind::Idle);
    }

    /// 経過時間が読めないときは**離席を勝手に立てない**。
    #[test]
    fn unreadable_idle_does_not_move_the_state() {
        let mut e = engine();
        let out = e.observe(idle_obs(0, None, false));
        assert_eq!(kinds(&out), ["foreground"], "離席が勝手に立っている");
    }

    /// **眠っていた間を題名の滞留に数えない**（R40）。
    #[test]
    fn suspend_does_not_count_as_dwell() {
        let mut e = engine();
        e.observe(obs(0, Some(fg("editor", "A", UrlRead::NotBrowser))));
        e.observe(obs(1, Some(fg("editor", "B", UrlRead::NotBrowser))));
        e.report_suspend(at(2), at(3600), None);
        let out = e.observe(obs(3600, Some(fg("editor", "B", UrlRead::NotBrowser))));
        assert!(
            out.iter().all(|r| r.kind != RecordKind::Foreground),
            "眠っていた時間で題名の滞留を満たした"
        );
    }

    // ----------------------------------------------------------------- 除外

    fn with_exclusion() -> Engine {
        Engine::new(Exclusions {
            rules: vec![Rule::ProcessName {
                value: "vault.exe".into(),
            }],
        })
    }

    fn secret(title: &str, url: UrlRead) -> Foreground {
        Foreground {
            app_name: "金庫".into(),
            exe_path: r"C:\apps\vault.exe".into(),
            process_name: "vault.exe".into(),
            title: title.into(),
            url,
        }
    }

    /// **本文が 1 文字も残らない**（FR-83 / design D11）。
    ///
    /// Scenario: 除外に登録した対象の本文は残らない
    /// Scenario: 除外した本文は取り込み口へ送られない
    #[test]
    fn exclusion() {
        let mut e = with_exclusion();
        let url = UrlRead::Read("https://vault.example/item/42?key=abcdef".into());
        assert!(
            e.observe(obs(0, Some(secret("銀行 / 本人の口座", url.clone()))))
                .is_empty(),
            "除外の対象が記録になっている"
        );
        for i in 1..=2 {
            assert!(e
                .observe(obs(
                    i,
                    Some(secret(&format!("銀行 / 口座 {i}"), url.clone()))
                ))
                .is_empty());
        }
        assert!(e
            .observe(obs(
                3,
                Some(secret(
                    "銀行 / 口座 2",
                    UrlRead::Read("https://vault.example/item/43".into())
                ))
            ))
            .is_empty());

        let out = e.observe(obs(10, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        assert_eq!(kinds(&out), ["excluded", "foreground"]);
        // **入ったこと 1 回 + 題名 2 回 + URL 1 回 = 4 回**（design D18 の数え方）
        assert_eq!(out[0].excluded_count, Some(4));
        let sent = serde_json::to_string(&out).expect("直列化");
        for leaked in ["金庫", "vault.exe", "銀行", "vault.example", "abcdef"] {
            assert!(
                !sent.contains(leaked),
                "除外した本文が送る形に残っている: {leaked}"
            );
        }
        // 置き場に落とす状態にも本文が無い
        let state = serde_json::to_string(&e.state()).unwrap();
        assert!(!state.contains("銀行") && !state.contains("vault"));
    }

    /// Scenario: 除外した件数が残る
    #[test]
    fn excluded_count_is_kept() {
        let mut e = with_exclusion();
        for (i, t) in ["項目 1", "項目 2", "項目 3"].iter().enumerate() {
            e.observe(obs(i as i64, Some(secret(t, UrlRead::NotBrowser))));
        }
        let out = e.observe(obs(9, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        let excluded = out
            .iter()
            .find(|r| r.kind == RecordKind::Excluded)
            .expect("除外の記録");
        assert_eq!(
            excluded.excluded_count,
            Some(3),
            "件数から 3 回を読み取れない"
        );
        assert_eq!(excluded.at, rfc3339(at(0)));
        assert!(excluded.title.is_none() && excluded.app_name.is_none());
    }

    /// **見続けている間の吐き出しで、変化していないのに数えが増えない**（R2）。
    #[test]
    fn excluded_count_is_not_inflated_by_flush() {
        let mut e = with_exclusion();
        let mut total = 0;
        // 30 分ずっと同じ除外の窓。5 分ごとに吐き出す
        for sec in 0..=1800 {
            e.observe(obs(sec, Some(secret("項目", UrlRead::NotBrowser))));
            if sec % 300 == 0 {
                for r in e.flush(at(sec)) {
                    total += r.excluded_count.unwrap_or(0);
                    assert_ne!(Some(r.at.clone()), r.range_end, "長さ 0 の区間を作った");
                }
            }
        }
        for r in e.observe(obs(1801, Some(fg("editor", "文書", UrlRead::NotBrowser)))) {
            total += r.excluded_count.unwrap_or(0);
        }
        assert_eq!(total, 1, "変化は入った 1 回だけなのに数えが水増しされた");
    }

    /// Scenario: 除外の登録が空なら何も除外されない
    #[test]
    fn nothing_is_excluded_when_empty() {
        let mut e = engine();
        let out = e.observe(obs(0, Some(secret("項目", UrlRead::NotBrowser))));
        assert_eq!(kinds(&out), ["foreground"]);
        assert_eq!(out[0].title.as_deref(), Some("項目"));
    }

    // ------------------------------------------------------ プロセスをまたぐ状態

    /// **落ちる前に開いていた離席と除外の数えを、次の起動で閉じる**（R1 / R4）。
    #[test]
    fn previous_state_is_closed_on_restart() {
        let mut e = with_exclusion();
        e.observe(idle_obs(400, Some(340), false));
        // 離席のまま、除外の窓が前景に出る（通知で前面に来た、など）
        e.observe(Observation {
            idle: IdleRead::Elapsed(Duration::seconds(341)),
            ..obs(401, Some(secret("項目", UrlRead::NotBrowser)))
        });
        let state = e.state();
        assert!(state.away.is_some() && state.excluded.is_some());

        let fresh = with_exclusion();
        let out = fresh.close_previous(state, Some(at(900)));
        assert_eq!(kinds(&out), ["idle", "excluded"]);
        assert_eq!(out[0].transition, Some(Transition::Leave));
        assert_eq!(out[0].ended_by, Some(EndedBy::Restart));
        assert_eq!(out[0].range_end.as_deref(), Some(rfc3339(at(900)).as_str()));
        assert_eq!(out[1].excluded_count, Some(1));
        // 印が無ければ閉じる時刻が無いので作らない
        assert!(fresh.close_previous(state, None).is_empty());
    }

    /// `Debug` に題名も URL も出ない（R35）。
    #[test]
    fn debug_does_not_leak_private_content() {
        let mut e = with_exclusion();
        e.observe(obs(
            0,
            Some(fg(
                "editor",
                "秘密の文書",
                UrlRead::Read("x.example".into()),
            )),
        ));
        e.observe(obs(1, Some(secret("銀行", UrlRead::NotBrowser))));
        let dbg = format!(
            "{e:?} {:?}",
            fg("editor", "秘密の文書", UrlRead::Read("x.example".into()))
        );
        for leaked in ["秘密", "x.example", "銀行", "editor"] {
            assert!(
                !dbg.contains(leaked),
                "Debug に本文が出た: {leaked} / {dbg}"
            );
        }
    }
}
