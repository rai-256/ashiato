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
use chrono::{DateTime, Duration, Utc};

use crate::contract::{AwayReason, RecordKind, Transition, WindowPayload};
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// アドレスバーを読めたか（design D4）。
///
/// **「読めなかった」と「ブラウザではない」を区別する。** 混ぜると、
/// UI Automation が死んでいても生存信号が「取得できる状態」と報告する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlRead {
    /// 前景がブラウザではない（読む必要が無い）
    NotBrowser,
    /// 読めた。**見えている文字列そのまま**（深掘り Q4）
    Read(String),
    /// 前景はブラウザだが、UI Automation が応答しない
    Unavailable,
}

impl UrlRead {
    fn value(&self) -> Option<&str> {
        match self {
            Self::Read(u) => Some(u),
            Self::NotBrowser | Self::Unavailable => None,
        }
    }
}

/// 最後の入力からの経過時間を読めたか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleRead {
    /// 読めた
    Elapsed(Duration),
    /// 読めない。**離席の判定を動かさない**（読めないことを「入力があった」と読むと、
    /// 離席が黙って消える）
    Unavailable,
}

/// 1 回の見回りで見えたもの（design D5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// 見た時刻
    pub at: DateTime<Utc>,
    /// 前景。**`None` は「読めなかった」**（前景の無い瞬間ではなく、権限や
    /// 別の卓面で読めない状態。生存信号がこれを `blockers` に載せる）
    pub foreground: Option<Foreground>,
    /// 最後の入力からの経過時間
    pub idle: IdleRead,
    /// 画面がロックされているか
    pub locked: bool,
}

/// 記録の元になる状態。**前景の「いま」と、まだ記録していない題名**を持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shown {
    app_name: String,
    exe_path: String,
    process_name: String,
    title: String,
    url: Option<String>,
}

impl Shown {
    fn of(fg: &Foreground) -> Self {
        Self {
            app_name: fg.app_name.clone(),
            exe_path: fg.exe_path.clone(),
            process_name: fg.process_name.clone(),
            title: fg.title.clone(),
            url: fg.url.value().map(str::to_string),
        }
    }

    /// 題名以外が同じか。**ここが「題名だけの変化」の定義**（design D8）。
    fn same_except_title(&self, other: &Self) -> bool {
        self.app_name == other.app_name
            && self.exe_path == other.exe_path
            && self.process_name == other.process_name
            && self.url == other.url
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Away {
    since: DateTime<Utc>,
    reason: AwayReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExcludedSpan {
    since: DateTime<Utc>,
    count: u32,
    /// いま除外している対象の見分け（**本文は記録に出さないが、
    /// 変化の回数を数えるために手元では持つ**）
    key: (String, String, Option<String>),
}

/// 観測を記録に変える。**状態を持つ**（前の観測との差が記録になるため）。
#[derive(Debug)]
pub struct Engine {
    exclusions: Exclusions,
    min_dwell: Duration,
    idle_threshold: Duration,
    shown: Option<Shown>,
    /// 滞留を待っている題名だけの変化と、それが前景に現れた時刻
    pending: Option<(Shown, DateTime<Utc>)>,
    away: Option<Away>,
    excluded: Option<ExcludedSpan>,
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
        }
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
            IdleRead::Elapsed(d) => Some(d),
            // **読めないときは判定を動かさない。** 入力があったと読むと離席が消え、
            // 無かったと読むと動いている最中に離席が立つ
            IdleRead::Unavailable => None,
        };
        let now_away = if obs.locked {
            Some(AwayReason::Locked)
        } else {
            match elapsed {
                Some(d) if d >= self.idle_threshold => Some(AwayReason::Idle),
                Some(_) => None,
                None => return out,
            }
        };

        match (&self.away, now_away) {
            (None, Some(reason)) => {
                // 入った。**入力が止まった時刻まで戻す** —— 見つけた時刻にすると
                // 閾値のぶん（5 分）だけ遅れ、離席の長さが後から引けない
                let since = match (reason, elapsed) {
                    (AwayReason::Idle, Some(d)) => obs.at - d,
                    // ロックは見つけた時刻が入った時刻（ロックした瞬間に入力は止まる）
                    _ => obs.at,
                };
                let mut p = WindowPayload::new(RecordKind::Idle, since);
                p.transition = Some(Transition::Enter);
                p.reason = Some(reason);
                p.idle_ms = elapsed.map(|d| d.num_milliseconds());
                out.push(p);
                self.away = Some(Away { since, reason });
            }
            (Some(away), None) => {
                // 出た。**入力が戻った時刻まで戻す**
                let resumed = elapsed.map_or(obs.at, |d| obs.at - d);
                out.push(leave_record(away.since, resumed, away.reason));
                self.away = None;
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
            // 除外。**本文はここから先へ 1 文字も出さない**（FR-83 / design D11）
            self.pending = None;
            self.shown = None;
            let key = (
                fg.exe_path.clone(),
                fg.title.clone(),
                fg.url.value().map(str::to_string),
            );
            match &mut self.excluded {
                Some(span) if span.key == key => {}
                Some(span) => {
                    span.count += 1;
                    span.key = key;
                }
                None => {
                    self.excluded = Some(ExcludedSpan {
                        since: obs.at,
                        count: 1,
                        key,
                    })
                }
            }
            return out;
        }

        // 除外の対象から出た。**数えを記録にする**（除外された時間帯と
        // 触っていなかった時間帯を区別できるようにするため）
        out.extend(self.close_excluded(obs.at));

        let next = Shown::of(fg);
        match &self.shown {
            None => {
                out.push(foreground_record(&next, obs.at));
                self.shown = Some(next);
                self.pending = None;
            }
            Some(cur) if *cur == next => {
                // 何も変わっていない。**待っていた題名が元に戻ったら捨てる**
                self.pending = None;
            }
            Some(cur) if cur.same_except_title(&next) => {
                // 題名だけの変化。滞留を待つ（design D8）
                match &self.pending {
                    Some((p, since)) if *p == next => {
                        if obs.at - *since >= self.min_dwell {
                            // **現れた時刻で記録する** —— 見つけた時刻にすると
                            // 滞留のぶん記録が後ろへずれる
                            out.push(foreground_record(&next, *since));
                            self.shown = Some(next);
                            self.pending = None;
                        }
                    }
                    // 別の題名に変わった。**前の題名は間引かれる**（Q6 の決定）
                    _ => self.pending = Some((next, obs.at)),
                }
            }
            Some(_) => {
                // アプリか URL の変化。**滞留に関わらず必ず 1 件**（Q6 の決定）
                out.push(foreground_record(&next, obs.at));
                self.shown = Some(next);
                self.pending = None;
            }
        }
        out
    }

    /// 見回りの時刻が飛んだ（PC が眠っていた。design D19）。
    ///
    /// **`from` は最後に見回れた時刻**で、`to` は目覚めた時刻。
    /// 眠っている間の入力は無いので、区間そのものを離席と同じ形で残す。
    pub fn report_suspend(&mut self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<WindowPayload> {
        let mut out = Vec::new();
        if let Some(away) = self.away.take() {
            // 既に離席していた区間は、眠りに入った時刻で閉じる
            out.push(leave_record(away.since, from, away.reason));
        }
        let mut enter = WindowPayload::new(RecordKind::Idle, from);
        enter.transition = Some(Transition::Enter);
        enter.reason = Some(AwayReason::Suspended);
        out.push(enter);
        out.push(leave_record(from, to, AwayReason::Suspended));
        out
    }

    /// 手元に抱えているものを記録にする（送信の契機ごと・終了時）。
    ///
    /// **除外の数えを抱えたまま落ちると、その件数は後から作れない。**
    pub fn flush(&mut self, now: DateTime<Utc>) -> Vec<WindowPayload> {
        let mut out = self.close_excluded(now);
        // 除外の対象を見続けている間も数えは続く（次の区間として数え直す）
        if let Some(span) = &mut self.excluded {
            span.since = now;
            span.count = 0;
        }
        if let Some((p, since)) = self.pending.clone() {
            if now - since >= self.min_dwell {
                out.push(foreground_record(&p, since));
                self.shown = Some(p);
                self.pending = None;
            }
        }
        out
    }

    fn close_excluded(&mut self, at: DateTime<Utc>) -> Vec<WindowPayload> {
        let Some(span) = self.excluded.take() else {
            return Vec::new();
        };
        if span.count == 0 {
            return Vec::new();
        }
        let mut p = WindowPayload::new(RecordKind::Excluded, span.since);
        p.excluded_count = Some(span.count);
        p.range_end = Some(crate::contract::rfc3339(at));
        vec![p]
    }
}

fn foreground_record(s: &Shown, at: DateTime<Utc>) -> WindowPayload {
    let mut p = WindowPayload::new(RecordKind::Foreground, at);
    p.app_name = Some(s.app_name.clone());
    p.exe_path = Some(s.exe_path.clone());
    p.process_name = Some(s.process_name.clone());
    p.title = Some(s.title.clone());
    p.url = s.url.clone();
    p
}

/// 出た側の記録。**区間（`range_end`）を持つのはこちら**（design D14）——
/// 入った側だけでは、いつ戻ったかが分からない。
fn leave_record(since: DateTime<Utc>, resumed: DateTime<Utc>, reason: AwayReason) -> WindowPayload {
    let mut p = WindowPayload::new(RecordKind::Idle, since);
    p.transition = Some(Transition::Leave);
    p.reason = Some(reason);
    p.range_end = Some(crate::contract::rfc3339(resumed));
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
        assert_eq!(out[0].at, crate::contract::rfc3339(at(10)));
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
            crate::contract::rfc3339(at(1)),
            "題名が現れた時刻ではなく、見つけた時刻で記録している"
        );
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
        // アドレスバーが `https://` と末尾のスラッシュを隠した表示
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
        // 省略された表示もそのまま
        let elided = "example.com/very/long/…/tail";
        let out = e.observe(obs(
            1,
            Some(fg("browser", "頁", UrlRead::Read(elided.into()))),
        ));
        assert_eq!(out[0].url.as_deref(), Some(elided));
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
        // 最後の題名だけが滞留を超えてとどまる
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
        // 1 秒だけ別のアプリを前景にして、すぐ戻す（どちらも滞留 5 秒に届かない）
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
        // 閾値（5 分）を超えて入力が無い
        let enter = e.observe(Observation {
            at: at(400),
            foreground: Some(fg("editor", "文書", UrlRead::NotBrowser)),
            idle: IdleRead::Elapsed(Duration::seconds(340)),
            locked: false,
        });
        assert_eq!(kinds(&enter), ["idle"]);
        assert_eq!(enter[0].transition, Some(Transition::Enter));
        assert_eq!(
            enter[0].at,
            crate::contract::rfc3339(at(60)),
            "入った時刻が「入力が止まった時刻」に戻っていない"
        );
        assert_eq!(enter[0].reason, Some(AwayReason::Idle));

        // 入力が戻る（見回りの 1 秒前に触った）
        let leave = e.observe(Observation {
            at: at(500),
            foreground: Some(fg("editor", "文書", UrlRead::NotBrowser)),
            idle: IdleRead::Elapsed(Duration::seconds(1)),
            locked: false,
        });
        assert_eq!(kinds(&leave), ["idle"]);
        assert_eq!(leave[0].transition, Some(Transition::Leave));
        assert_eq!(leave[0].at, crate::contract::rfc3339(at(60)));
        assert_eq!(
            leave[0].range_end.as_deref(),
            Some(crate::contract::rfc3339(at(499)).as_str()),
            "戻った時刻が残っていない"
        );
        // 出入りで 2 件（人間の確認: 5 分離れて戻ると 2 件）
        assert_eq!(enter.len() + leave.len(), 2);
    }

    /// Scenario: 画面ロックとスリープも残る
    #[test]
    fn lock_and_suspend_are_recorded() {
        let mut e = engine();
        e.observe(obs(0, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        // ロックは閾値を待たずに入る（ロックした瞬間に入力は止まる）
        let lock = e.observe(Observation {
            at: at(10),
            foreground: None,
            idle: IdleRead::Elapsed(Duration::seconds(2)),
            locked: true,
        });
        assert_eq!(lock[0].reason, Some(AwayReason::Locked));
        assert_eq!(lock[0].transition, Some(Transition::Enter));
        let unlock = e.observe(Observation {
            at: at(70),
            foreground: Some(fg("editor", "文書", UrlRead::NotBrowser)),
            idle: IdleRead::Elapsed(Duration::zero()),
            locked: false,
        });
        assert_eq!(unlock[0].transition, Some(Transition::Leave));
        assert_eq!(unlock[0].reason, Some(AwayReason::Locked));

        // スリープは見回りの時刻の飛びで拾う（design D19）
        let slept = e.report_suspend(at(100), at(4000));
        assert_eq!(kinds(&slept), ["idle", "idle"]);
        assert_eq!(slept[0].reason, Some(AwayReason::Suspended));
        assert_eq!(
            slept[1].range_end.as_deref(),
            Some(crate::contract::rfc3339(at(4000)).as_str())
        );
    }

    /// **閾値を変えたときに引き直せる**（design D9 の仮決めが可逆であることの根拠）。
    ///
    /// Scenario: 閾値を後から引き直せる形で残る
    #[test]
    fn idle_records_carry_elapsed_for_rethreshold() {
        let mut e = engine();
        let enter = e.observe(Observation {
            at: at(400),
            foreground: Some(fg("editor", "文書", UrlRead::NotBrowser)),
            idle: IdleRead::Elapsed(Duration::seconds(340)),
            locked: false,
        });
        assert_eq!(enter[0].idle_ms, Some(340_000));
        let leave = e.observe(Observation {
            at: at(500),
            foreground: Some(fg("editor", "文書", UrlRead::NotBrowser)),
            idle: IdleRead::Elapsed(Duration::zero()),
            locked: false,
        });
        // 区間の長さが載るので、**閾値を変えたときの判定を後から引ける** ——
        // 10 分で切るなら「この区間は離席に数えない」と、記録だけから決まる
        let span_ms = leave[0].idle_ms.expect("区間の長さ");
        assert_eq!(span_ms, 440_000);
        assert!(span_ms < 600_000, "区間の長さから閾値の判定を引けない");
    }

    /// Scenario: 離席の記録は前景の記録と区別できる
    #[test]
    fn record_kind_is_distinguishable() {
        let mut e = engine();
        let fgr = e.observe(obs(0, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        let idle = e.observe(Observation {
            at: at(400),
            foreground: Some(fg("editor", "文書", UrlRead::NotBrowser)),
            idle: IdleRead::Elapsed(Duration::seconds(340)),
            locked: false,
        });
        assert_eq!(fgr[0].kind, RecordKind::Foreground);
        assert_eq!(idle[0].kind, RecordKind::Idle);
        assert_ne!(fgr[0].kind.as_str(), idle[0].kind.as_str());
    }

    /// 経過時間が読めないときは**離席の判定を動かさない**。
    #[test]
    fn unreadable_idle_does_not_move_the_state() {
        let mut e = engine();
        let out = e.observe(Observation {
            at: at(0),
            foreground: Some(fg("editor", "文書", UrlRead::NotBrowser)),
            idle: IdleRead::Unavailable,
            locked: false,
        });
        assert_eq!(kinds(&out), ["foreground"], "離席が勝手に立っている");
    }

    // ----------------------------------------------------------------- 除外

    fn with_exclusion() -> Engine {
        Engine::new(Exclusions {
            rules: vec![Rule::ProcessName {
                value: "vault.exe".into(),
            }],
        })
    }

    /// **本文が 1 文字も残らない**（FR-83 / design D11）。
    ///
    /// Scenario: 除外に登録した対象の本文は残らない
    /// Scenario: 除外した本文は取り込み口へ送られない
    #[test]
    fn exclusion() {
        let mut e = with_exclusion();
        let secret = Foreground {
            app_name: "金庫".into(),
            exe_path: r"C:\apps\vault.exe".into(),
            process_name: "vault.exe".into(),
            title: "銀行 / 本人の口座".into(),
            url: UrlRead::Read("https://vault.example/item/42?key=abcdef".into()),
        };
        let out = e.observe(obs(0, Some(secret.clone())));
        assert!(out.is_empty(), "除外の対象が記録になっている");

        // 除外の中で 3 回変化する（題名と URL が変わる）
        for i in 1..=2 {
            let mut s = secret.clone();
            s.title = format!("銀行 / 口座 {i}");
            assert!(e.observe(obs(i, Some(s))).is_empty());
        }
        let mut s = secret.clone();
        s.url = UrlRead::Read("https://vault.example/item/43".into());
        assert!(e.observe(obs(3, Some(s))).is_empty());

        // 除外の対象から出ると、件数だけが記録になる
        let out = e.observe(obs(10, Some(fg("editor", "文書", UrlRead::NotBrowser))));
        assert_eq!(kinds(&out), ["excluded", "foreground"]);
        let sent = serde_json::to_string(&out).expect("直列化");
        for leaked in [
            "金庫",
            "vault.exe",
            "銀行",
            "vault.example",
            "abcdef",
            r"C:\apps\vault.exe",
        ] {
            assert!(
                !sent.contains(leaked),
                "除外した本文が送る形に残っている: {leaked}"
            );
        }
    }

    /// Scenario: 除外した件数が残る
    #[test]
    fn excluded_count_is_kept() {
        let mut e = with_exclusion();
        let secret = |title: &str| {
            Some(Foreground {
                app_name: "金庫".into(),
                exe_path: r"C:\apps\vault.exe".into(),
                process_name: "vault.exe".into(),
                title: title.into(),
                url: UrlRead::NotBrowser,
            })
        };
        // 除外の中で前景の変化が 3 回起きる
        for (i, t) in ["項目 1", "項目 2", "項目 3"].iter().enumerate() {
            e.observe(obs(i as i64, secret(t)));
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
        assert_eq!(excluded.at, crate::contract::rfc3339(at(0)));
        assert!(excluded.title.is_none() && excluded.app_name.is_none());
    }

    /// 除外の対象を見続けている間も**数えを抱えたままにしない**（落ちると作れない）。
    #[test]
    fn excluded_count_is_flushed_while_still_foreground() {
        let mut e = with_exclusion();
        let secret = Foreground {
            app_name: "金庫".into(),
            exe_path: r"C:\apps\vault.exe".into(),
            process_name: "vault.exe".into(),
            title: "項目".into(),
            url: UrlRead::NotBrowser,
        };
        e.observe(obs(0, Some(secret.clone())));
        let out = e.flush(at(300));
        assert_eq!(kinds(&out), ["excluded"]);
        assert_eq!(out[0].excluded_count, Some(1));
        // 2 度目の契機では、新しい変化が無ければ何も出ない
        assert!(e.flush(at(600)).is_empty());
    }

    /// Scenario: 除外の登録が空なら何も除外されない
    #[test]
    fn nothing_is_excluded_when_empty() {
        let mut e = engine();
        let out = e.observe(obs(
            0,
            Some(Foreground {
                app_name: "金庫".into(),
                exe_path: r"C:\apps\vault.exe".into(),
                process_name: "vault.exe".into(),
                title: "項目".into(),
                url: UrlRead::NotBrowser,
            }),
        ));
        assert_eq!(kinds(&out), ["foreground"]);
        assert_eq!(out[0].title.as_deref(), Some("項目"));
    }
}
