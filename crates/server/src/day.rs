// SPDX-License-Identifier: AGPL-3.0-only
//! 1 日の並び（ST25 / design D1 / D6 / D7）。`stay_store::day_view` の行に、行ごとの要約と
//! 「ほかの記録」の行を足す。`day_view` 自身は変えない（ST22 の契約と試験がそのまま残る）。
//!
//! 要約の中身は `day_records` の正規化（D2〜D6）を**そのまま**読む。`GET /day/records` と同じ関数を通すので、
//! 要約と開いたときの中身がずれない。記録の値（アプリ名・ドメイン・URL）はログに出さない。

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::day_records::{self, Kind, Norm, Origin};
use crate::stay_store::{self, day_bounds, jst_date, CriteriaTag, DayEntry, EntryKind};

/// 要約に出すアプリ・ドメインの数（deep.md Q1。本人の決定）。
pub const SUMMARY_TOP: usize = 3;

/// 離席を行にする下限（分。deep.md Q1。本人の決定）。要約の差し引きは 10 分未満も含める（D6）。
pub const AWAY_ROW_MIN_MINUTES: i64 = 10;

/// スマホのアプリを前に出していた時間の打ち切り（D6（仮））。
const PHONE_USE_CAP_MINUTES: i64 = 10;

const BROWSER_SOURCE: &str = "c02-browser-history";

// ------------------------------------------------------------------ 応答の型

/// 並びの 1 行の種類。`day_view` の種類に「ほかの記録」を足したもの（並びは `day_view` の順のまま）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RowKind {
    Stay,
    Move,
    NoRecord,
    Erased,
    /// どの行にも入らない時間（消した時間・今日の最後の位置からいままで）の位置以外の記録
    Other,
}

impl From<EntryKind> for RowKind {
    fn from(k: EntryKind) -> Self {
        match k {
            EntryKind::Stay => RowKind::Stay,
            EntryKind::Move => RowKind::Move,
            EntryKind::NoRecord => RowKind::NoRecord,
            EntryKind::Erased => RowKind::Erased,
        }
    }
}

/// アプリ 1 つの使った時間。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct AppSeconds {
    pub app: String,
    pub seconds: i64,
}

/// PC / スマホの要約。`seconds` は全アプリの合計、`top` は長い順に最大 `SUMMARY_TOP`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct AppSummary {
    pub seconds: i64,
    pub top: Vec<AppSeconds>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct DomainCount {
    pub domain: String,
    pub count: i64,
}

/// ブラウザの要約。`count` は訪問の総数、`top` はドメインごとの件数の多い順。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct BrowserSummary {
    pub count: i64,
    pub top: Vec<DomainCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ArchiveCount {
    pub display_name: String,
    pub count: i64,
}

/// 行の `[start, end)` の要約。**記録の無いソースは鍵ごと出さない。**
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct Summary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pc: Option<AppSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<AppSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<BrowserSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<Vec<ArchiveCount>>,
}

/// 並びの 1 行。`day_view` の行の欄に `summary` を足したもの（「消した」の行には付けない）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct DayRow {
    pub kind: RowKind,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stay_ids: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Summary>,
}

/// 取れていない時間の種類（D9）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GapKind {
    Away,
    Stopped,
    Excluded,
    Unavailable,
}

/// 取れていない時間 1 つ。置く位置は画面が決める（D9）。日の端といまで切ってある。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct Gap {
    pub kind: GapKind,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// 除外した変化の件数（`excluded` だけ。本文は持たない）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// スマホの取れていない理由（`unavailable` だけ。`retention` / `clock_skew_abandoned`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// `GET /day` の応答。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct DayResponse {
    pub date: NaiveDate,
    pub criteria: Vec<CriteriaTag>,
    pub entries: Vec<DayRow>,
    pub gaps: Vec<Gap>,
    /// ブラウザ履歴が最後に届いた時刻。**「まだ届いていない」を出すべきときだけ**ある（D10）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_through: Option<DateTime<Utc>>,
}

// ------------------------------------------------------------------ 使った時間（D6）

/// アプリを使った 1 区間（差し引き済み）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Seg {
    origin: Origin,
    app: String,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

type Span = (DateTime<Utc>, DateTime<Utc>);

/// `pieces` から `blocks` と重なる部分を取り除く。
fn subtract(
    pieces: Vec<(DateTime<Utc>, DateTime<Utc>)>,
    blocks: &[(DateTime<Utc>, DateTime<Utc>)],
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let mut sorted = blocks.to_vec();
    sorted.sort();
    let mut out = Vec::new();
    for (mut s, e) in pieces {
        for &(bs, be) in &sorted {
            if be <= s || bs >= e {
                continue;
            }
            if bs > s {
                out.push((s, bs));
            }
            s = s.max(be);
        }
        if s < e {
            out.push((s, e));
        }
    }
    out
}

fn by_device(norms: &[Norm], origin: Origin) -> BTreeMap<Option<&str>, Vec<&Norm>> {
    let mut m: BTreeMap<Option<&str>, Vec<&Norm>> = BTreeMap::new();
    for n in norms.iter().filter(|n| n.origin == origin) {
        m.entry(n.device_id.as_deref()).or_default().push(n);
    }
    m
}

/// 同じアプリかを決める鍵。前景は表示名で出るので、パッケージ名があればそれで揃える。
fn identity(n: &Norm) -> Option<&str> {
    n.package.as_deref().or(n.app.as_deref())
}

/// 記録（時刻順）から、アプリを使った区間を作る（D6）。`now` は「次が無い」ときの今日の終わり。
pub(crate) fn usage_segments(norms: &[Norm], now: DateTime<Utc>) -> Vec<Seg> {
    let mut out = Vec::new();

    // PC: 前景から次の前景の変化（次の前景・除外の始まり）まで。間の離席・除外・停止を差し引く。
    for list in by_device(norms, Origin::Pc).values() {
        let blocks: Vec<_> = list
            .iter()
            .filter(|n| matches!(n.kind, Kind::Away | Kind::Stopped | Kind::Excluded))
            .map(|n| (n.start, n.end.unwrap_or(n.start)))
            .collect();
        for (i, n) in list.iter().enumerate() {
            let (Kind::Foreground, Some(app)) = (n.kind, n.app.as_ref()) else {
                continue;
            };
            let end = list[i + 1..]
                .iter()
                .find(|m| matches!(m.kind, Kind::Foreground | Kind::Excluded))
                .map_or_else(|| day_bounds(jst_date(n.start)).1.min(now), |m| m.start)
                .max(n.start);
            for (start, end) in subtract(vec![(n.start, end)], &blocks) {
                out.push(Seg {
                    origin: Origin::Pc,
                    app: app.clone(),
                    start,
                    end,
                });
            }
        }
    }

    // スマホ: 前に出た時刻から、次に別のアプリが前に出た時刻か同じアプリが後ろに下がった時刻まで。1 回 10 分で打ち切る。
    // 後ろに下がった記録が無いまま同じアプリの前景が続くと区間が重なるので、端末とアプリごとに和集合をとる。
    for list in by_device(norms, Origin::Phone).values() {
        let mut by_app: BTreeMap<Option<&str>, (&str, Vec<Span>)> = BTreeMap::new();
        for (i, n) in list.iter().enumerate() {
            let (Kind::Foreground, Some(app)) = (n.kind, n.app.as_ref()) else {
                continue;
            };
            let key = identity(n);
            let cap = n.start + Duration::minutes(PHONE_USE_CAP_MINUTES);
            let end = list[i + 1..]
                .iter()
                .find(|m| match m.kind {
                    Kind::Foreground => identity(m) != key,
                    Kind::Background => identity(m) == key,
                    _ => false,
                })
                .map_or(cap, |m| m.start.min(cap));
            if end > n.start {
                by_app
                    .entry(key)
                    .or_insert_with(|| (app.as_str(), Vec::new()))
                    .1
                    .push((n.start, end));
            }
        }
        for (app, mut spans) in by_app.into_values() {
            spans.sort();
            let mut merged: Vec<Span> = Vec::new();
            for (start, end) in spans {
                match merged.last_mut() {
                    Some(last) if start <= last.1 => last.1 = last.1.max(end),
                    _ => merged.push((start, end)),
                }
            }
            out.extend(merged.into_iter().map(|(start, end)| Seg {
                origin: Origin::Phone,
                app: app.to_owned(),
                start,
                end,
            }));
        }
    }
    out
}

// ------------------------------------------------------------------ 要約（D7）

/// 値の大きい順、同点はキーの昇順。
fn ranked<K: Ord + Clone>(m: &BTreeMap<K, i64>) -> Vec<(K, i64)> {
    let mut v: Vec<(K, i64)> = m.iter().map(|(k, c)| (k.clone(), *c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

/// ドメインを束ねる形: 小文字（`host_of` が済ませている）で先頭の `www.` を外す。
fn bundled(domain: &str) -> String {
    let d = domain.to_lowercase();
    d.strip_prefix("www.")
        .map_or_else(|| d.clone(), str::to_owned)
}

fn app_summary(
    segs: &[Seg],
    origin: Origin,
    s: DateTime<Utc>,
    e: DateTime<Utc>,
) -> Option<AppSummary> {
    let mut by_app: BTreeMap<&str, i64> = BTreeMap::new();
    for g in segs.iter().filter(|g| g.origin == origin) {
        let secs = (g.end.min(e) - g.start.max(s)).num_seconds();
        if secs > 0 {
            *by_app.entry(g.app.as_str()).or_default() += secs;
        }
    }
    let seconds: i64 = by_app.values().sum();
    (seconds > 0).then(|| AppSummary {
        seconds,
        top: ranked(&by_app)
            .into_iter()
            .take(SUMMARY_TOP)
            .map(|(app, seconds)| AppSeconds {
                app: app.to_owned(),
                seconds,
            })
            .collect(),
    })
}

/// 行の `[s, e)` の要約。`segs` は `usage_segments(norms, …)` の結果。
pub(crate) fn summarize(
    norms: &[Norm],
    segs: &[Seg],
    s: DateTime<Utc>,
    e: DateTime<Utc>,
) -> Summary {
    let in_row = |kind: Kind| {
        norms
            .iter()
            .filter(move |n| n.kind == kind && n.in_range(s, e))
    };

    let visits: Vec<&Norm> = in_row(Kind::Visit).collect();
    let mut domains: BTreeMap<String, i64> = BTreeMap::new();
    for d in visits.iter().filter_map(|n| n.domain.as_deref()) {
        *domains.entry(bundled(d)).or_default() += 1;
    }
    let browser = (!visits.is_empty()).then(|| BrowserSummary {
        count: i64::try_from(visits.len()).unwrap_or(i64::MAX),
        top: ranked(&domains)
            .into_iter()
            .take(SUMMARY_TOP)
            .map(|(domain, count)| DomainCount { domain, count })
            .collect(),
    });

    let mut archive: BTreeMap<String, i64> = BTreeMap::new();
    for n in in_row(Kind::Archive) {
        *archive
            .entry(n.source_label.clone().unwrap_or_default())
            .or_default() += 1;
    }
    let archive = (!archive.is_empty()).then(|| {
        ranked(&archive)
            .into_iter()
            .map(|(display_name, count)| ArchiveCount {
                display_name,
                count,
            })
            .collect()
    });

    Summary {
        pc: app_summary(segs, Origin::Pc, s, e),
        phone: app_summary(segs, Origin::Phone, s, e),
        browser,
        archive,
    }
}

// ------------------------------------------------------------------ ほかの記録の行（D7）

/// どの行にも入らない時間: 消した時間と、並びに覆われない時間（今日の最後の位置からいままで）。
pub(crate) fn other_candidates(
    entries: &[DayEntry],
    d0: DateTime<Utc>,
    upper: DateTime<Utc>,
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let mut out: Vec<_> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Erased)
        .map(|e| (e.start, e.end))
        .collect();
    let mut covered: Vec<_> = entries
        .iter()
        .map(|e| (e.start.max(d0), e.end.min(upper)))
        .collect();
    covered.sort();
    let mut cursor = d0;
    for (s, e) in covered {
        if s > cursor && cursor < upper {
            out.push((cursor, s.min(upper)));
        }
        cursor = cursor.max(e);
    }
    if cursor < upper {
        out.push((cursor, upper));
    }
    out
}

// ------------------------------------------------------------------ 取れていない時間（D9）

fn clip(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    d0: DateTime<Utc>,
    upper: DateTime<Utc>,
) -> Option<Span> {
    let (s, e) = (start.max(d0), end.min(upper));
    (s < e).then_some((s, e))
}

/// 除外の記録を、続くものを 1 つにまとめた塊にする（PC の端末ごと）。
/// 塊の終わりは「次の前景の記録」（無ければ次の停止の始まり）。`range_end` は下限（D9）。
/// 前景を挟まず、前の `range_end` までに始まる記録は同じ塊（送る契機の区間切り。数えが 0 の続きは記録にならない）。
fn excluded_gaps(norms: &[Norm], d0: DateTime<Utc>, upper: DateTime<Utc>) -> Vec<Gap> {
    let mut out = Vec::new();
    for list in by_device(norms, Origin::Pc).values() {
        // (始まり, range_end の最大, 件数)
        let mut groups: Vec<(DateTime<Utc>, DateTime<Utc>, i64)> = Vec::new();
        for (i, n) in list.iter().enumerate() {
            if n.kind != Kind::Excluded {
                continue;
            }
            let range_end = n.end.unwrap_or(n.start);
            let count = n.excluded_count.unwrap_or(0);
            let continues = groups.last().is_some_and(|g| {
                n.start <= g.1
                    && !list[..i]
                        .iter()
                        .any(|m| m.kind == Kind::Foreground && m.start > g.0 && m.start <= n.start)
            });
            match groups.last_mut() {
                Some(g) if continues => {
                    g.1 = g.1.max(range_end);
                    g.2 += count;
                }
                _ => groups.push((n.start, range_end, count)),
            }
        }
        for (start, range_end, count) in groups {
            let boundary = list
                .iter()
                .find(|m| matches!(m.kind, Kind::Foreground | Kind::Stopped) && m.start > start)
                .map_or(upper, |m| m.start);
            if let Some((s, e)) = clip(start, boundary.max(range_end), d0, upper) {
                out.push(Gap {
                    kind: GapKind::Excluded,
                    start: s,
                    end: e,
                    count: Some(count),
                    reason: None,
                });
            }
        }
    }
    out
}

/// その日の取れていない時間（離席・PC の停止・除外・スマホ）。`[d0, upper)` で切る。始まりの時刻順。
pub(crate) fn gaps(norms: &[Norm], d0: DateTime<Utc>, upper: DateTime<Utc>) -> Vec<Gap> {
    let mut out = excluded_gaps(norms, d0, upper);
    for n in norms {
        let end = n.end.unwrap_or(n.start);
        let (kind, reason) = match n.kind {
            // 10 分未満は行にしない。長さは記録そのものの長さで見る（日の端で切る前）。
            Kind::Away if end - n.start >= Duration::minutes(AWAY_ROW_MIN_MINUTES) => {
                (GapKind::Away, None)
            }
            Kind::Stopped => (GapKind::Stopped, None),
            Kind::Unavailable => (GapKind::Unavailable, n.reason.clone()),
            _ => continue,
        };
        if let Some((start, end)) = clip(n.start, end, d0, upper) {
            out.push(Gap {
                kind,
                start,
                end,
                count: None,
                reason,
            });
        }
    }
    out.sort_by_key(|g| (g.start, g.kind as u8, g.end));
    out
}

// ------------------------------------------------------------------ ブラウザの最後に届いた時刻（D10（仮））

/// `c02-browser-history` の `ingest_time` の最大（削除済みを含む）を、「まだ届いていない」を出すべきときだけ返す。
/// 収集を始めた日より前の日と、その日の終わり（今日はいま）以後に届いている日には出さない。
async fn browser_through(
    pool: &PgPool,
    user: Uuid,
    date: NaiveDate,
    upper: DateTime<Utc>,
) -> sqlx::Result<Option<DateTime<Utc>>> {
    let started: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT collection_started_on FROM core.source WHERE logical_source = $1",
    )
    .bind(BROWSER_SOURCE)
    .fetch_optional(pool)
    .await?
    .flatten();
    if started.is_none_or(|d| d > date) {
        return Ok(None);
    }
    let through: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT max(ingest_time) FROM core.event WHERE user_id = $1 AND logical_source = $2",
    )
    .bind(user)
    .bind(BROWSER_SOURCE)
    .fetch_one(pool)
    .await?;
    Ok(through.filter(|t| *t < upper))
}

/// その日（`Asia/Tokyo`）の並びに、行ごとの要約とほかの記録の行を足して返す。
pub async fn day(
    pool: &PgPool,
    user: Uuid,
    date: NaiveDate,
    now: DateTime<Utc>,
) -> sqlx::Result<DayResponse> {
    let view = stay_store::day_view(pool, user, date, now).await?;
    let (d0, d1) = day_bounds(date);
    let upper = d1.min(now).max(d0);
    let others = other_candidates(&view.entries, d0, upper);

    // 全部の行の記録を 1 回で読む。前景は同じ日の前から続くので、窓は最も早い始まりのその日の頭から。
    let spans = view
        .entries
        .iter()
        .filter(|e| e.kind != EntryKind::Erased)
        .map(|e| (e.start, e.end))
        .chain(others.iter().copied());
    // 取れていない時間（D9）は行に覆われない日にもあるので、窓はその日の全体を含める。
    let (from, to) = spans.fold((d0, d1), |(ws, we), (s, e)| (ws.min(s), we.max(e)));
    let norms = day_records::read(pool, user, day_bounds(jst_date(from)).0, to, now).await?;
    let segs = usage_segments(&norms, now);

    let mut entries: Vec<DayRow> = view
        .entries
        .into_iter()
        .map(|e| DayRow {
            summary: (e.kind != EntryKind::Erased)
                .then(|| summarize(&norms, &segs, e.start, e.end)),
            kind: e.kind.into(),
            start: e.start,
            end: e.end,
            id: e.id,
            criteria_id: e.criteria_id,
            stay_ids: e.stay_ids,
        })
        .collect();
    for (s, e) in others {
        // 位置以外の記録があるときだけ行にする（開くとその記録に届く）。
        if norms.iter().any(|n| n.is_listed() && n.in_range(s, e)) {
            entries.push(DayRow {
                kind: RowKind::Other,
                start: s,
                end: e,
                id: None,
                criteria_id: None,
                stay_ids: None,
                summary: Some(summarize(&norms, &segs, s, e)),
            });
        }
    }
    entries.sort_by_key(|r| (r.start, r.kind));
    Ok(DayResponse {
        date,
        criteria: view.criteria,
        entries,
        gaps: gaps(&norms, d0, upper),
        browser_through: browser_through(pool, user, date, upper).await?,
    })
}
