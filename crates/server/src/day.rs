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

/// スマホのアプリを前に出していた時間の打ち切り（D6（仮））。
const PHONE_USE_CAP_MINUTES: i64 = 10;

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

/// `GET /day` の応答。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct DayResponse {
    pub date: NaiveDate,
    pub criteria: Vec<CriteriaTag>,
    pub entries: Vec<DayRow>,
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
    let window = spans.fold(None, |w: Option<(DateTime<Utc>, DateTime<Utc>)>, (s, e)| {
        Some(w.map_or((s, e), |(ws, we)| (ws.min(s), we.max(e))))
    });
    let (norms, segs) = match window {
        Some((from, to)) => {
            let norms =
                day_records::read(pool, user, day_bounds(jst_date(from)).0, to, now).await?;
            let segs = usage_segments(&norms, now);
            (norms, segs)
        }
        None => (Vec::new(), Vec::new()),
    };

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
    })
}
