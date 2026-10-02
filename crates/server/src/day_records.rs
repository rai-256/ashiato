// SPDX-License-Identifier: AGPL-3.0-only
//! 1 日の記録の読み出し（ST25 / design D2〜D5）。
//!
//! 何を記録として出すかは **許可リスト 1 つ**（`ALLOW`）で決める。ST06 は種別をふるい落とさずに積むので、
//! 表に無いものを出す側に倒すと新しい種別が黙って行に混ざる（既定は厳しい側）。
//!
//! 正規化は 2 段。1 段目（`Norm`）は D6 が区切りに使う `Background` を持つ内部の形、
//! 2 段目（`Norm::is_listed`）が `/day/records` に出すものを選ぶ。要約（D7）は 1 段目を読む。
//!
//! 感度では外さない（C5）。記録の値（題名・URL・アプリ名）はログに出さない。

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

use crate::stay_store::{day_bounds, jst_date};

/// 区間の記録の始まり・終わりを、`event_time` の置き方がソースで違っても拾える幅（D5）。
const RANGE_MARGIN_DAYS: i64 = 40;

/// 記録の種類（D2 の表の「種類」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Foreground,
    Away,
    Stopped,
    Excluded,
    Visit,
    /// スマホで前景から後ろに下がった。**内部だけ**（D6 の区切り）。
    Background,
    Unavailable,
    Archive,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Foreground => "foreground",
            Kind::Away => "away",
            Kind::Stopped => "stopped",
            Kind::Excluded => "excluded",
            Kind::Visit => "visit",
            Kind::Background => "background",
            Kind::Unavailable => "unavailable",
            Kind::Archive => "archive",
        }
    }
}

/// 記録の出どころ（D2 の表の `source`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Pc,
    Phone,
    Browser,
    Archive,
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Pc => "pc",
            Origin::Phone => "phone",
            Origin::Browser => "browser",
            Origin::Archive => "archive",
        }
    }
}

/// 正規化した記録 1 件（1 段目の形）。
#[derive(Debug, Clone, PartialEq)]
pub struct Norm {
    pub id: Uuid,
    pub kind: Kind,
    pub origin: Origin,
    pub logical_source: String,
    /// 書庫のときだけ、登録簿の表示名（C1: 画面は「書庫: <表示名>」と書く）。
    pub source_label: Option<String>,
    pub device_id: Option<String>,
    pub tz_id: String,
    pub start: DateTime<Utc>,
    /// 区間の記録の終わり。点の記録は `None`。
    pub end: Option<DateTime<Utc>>,
    pub app: Option<String>,
    pub title: Option<String>,
    pub domain: Option<String>,
    /// マイアクティビティの製品名。
    pub product: Option<String>,
    /// 取れていない理由（`unavailable` だけ）。
    pub reason: Option<String>,
    /// 除外した変化の件数（`excluded` だけ。本文は持たない）。
    pub excluded_count: Option<i64>,
}

impl Norm {
    /// 2 段目: `/day/records` に出すか。`Background` は区切りにだけ使う。
    pub fn is_listed(&self) -> bool {
        self.kind != Kind::Background
    }

    /// 日の範囲との重なり（D5）。点は出来事の時刻、区間は重なりで判定する。
    pub fn in_range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> bool {
        match self.end {
            None => self.start >= from && self.start < to,
            Some(end) => self.start < to && end > from,
        }
    }
}

/// 取り込み済みの 1 行（`core.event_live` の読み出し）。
#[derive(Debug, Clone)]
pub(crate) struct StoredEvent {
    pub(crate) id: Uuid,
    pub(crate) logical_source: String,
    pub(crate) display_name: String,
    pub(crate) event_time: DateTime<Utc>,
    pub(crate) ingest_time: DateTime<Utc>,
    pub(crate) tz_id: String,
    pub(crate) device_id: Option<String>,
    pub(crate) payload: Value,
}

/// 区間の置き方（D5: 始まりと終わりを種類ごとに読む）。
#[derive(Debug, Clone, Copy)]
enum Span {
    Point,
    /// 始まり = `event_time`、終わり = payload のこの鍵。
    EndKey(&'static str),
    /// 始まり・終わりとも payload の鍵（`event_time` は終わりに置かれる）。
    BeginEnd(&'static str, &'static str),
}

/// 許可リストの 1 行（D2 の表の 1 行）。
#[derive(Debug)]
struct Allow {
    /// 論理ソース。末尾が `*` なら前方一致（`c03-myactivity-*`）。
    sources: &'static [&'static str],
    /// payload の条件。
    when: fn(&Value) -> bool,
    kind: Kind,
    origin: Origin,
    span: Span,
    shape: fn(&Value) -> Shape,
}

/// 種類ごとの表示の欄。**`excluded` は本文を写さない**ので `Shape::default()`。
#[derive(Debug, Default)]
struct Shape {
    app: Option<String>,
    title: Option<String>,
    domain: Option<String>,
    product: Option<String>,
    reason: Option<String>,
    excluded_count: Option<i64>,
}

fn s(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

fn kind_is(v: &Value, k: &str) -> bool {
    v.get("kind").and_then(Value::as_str) == Some(k)
}

fn transition_is(v: &Value, t: &str) -> bool {
    v.get("transition").and_then(Value::as_str) == Some(t)
}

fn event_type_in(v: &Value, types: &[i64]) -> bool {
    v.get("kind").is_none()
        && v.get("event_type")
            .and_then(Value::as_i64)
            .is_some_and(|t| types.contains(&t))
}

/// URL のホスト。ユーザ情報・ポート・パスを落とし、小文字にする。
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?;
    let host = match host.strip_prefix('[') {
        Some(v6) => v6.split(']').next()?,
        None => host.split(':').next()?,
    };
    (!host.is_empty()).then(|| host.to_lowercase())
}

fn domain(v: &Value) -> Option<String> {
    s(v, "url").as_deref().and_then(host_of)
}

/// **許可リスト。** D2 の表の 1 行 = この配列の 1 要素。ここに無い記録は出ない。
const ALLOW: &[Allow] = &[
    Allow {
        sources: &["c02-window"],
        when: |v| kind_is(v, "foreground"),
        kind: Kind::Foreground,
        origin: Origin::Pc,
        span: Span::Point,
        shape: |v| Shape {
            app: s(v, "app_name").or_else(|| s(v, "process_name")),
            title: s(v, "title"),
            domain: domain(v),
            ..Shape::default()
        },
    },
    Allow {
        sources: &["c02-window"],
        when: |v| kind_is(v, "idle") && transition_is(v, "leave"),
        kind: Kind::Away,
        origin: Origin::Pc,
        span: Span::EndKey("range_end"),
        shape: |_| Shape::default(),
    },
    Allow {
        sources: &["c02-window"],
        when: |v| kind_is(v, "powered-off"),
        kind: Kind::Stopped,
        origin: Origin::Pc,
        span: Span::EndKey("range_end"),
        shape: |_| Shape::default(),
    },
    Allow {
        sources: &["c02-window"],
        when: |v| kind_is(v, "excluded"),
        kind: Kind::Excluded,
        origin: Origin::Pc,
        span: Span::EndKey("range_end"),
        shape: |v| Shape {
            excluded_count: v.get("excluded_count").and_then(Value::as_i64),
            ..Shape::default()
        },
    },
    Allow {
        sources: &["c02-browser-history"],
        when: |v| kind_is(v, "visit"),
        kind: Kind::Visit,
        origin: Origin::Browser,
        span: Span::Point,
        shape: |v| Shape {
            app: s(v, "browser"),
            title: s(v, "title"),
            domain: domain(v),
            ..Shape::default()
        },
    },
    Allow {
        sources: &["c01-app-usage"],
        when: |v| event_type_in(v, &[1]),
        kind: Kind::Foreground,
        origin: Origin::Phone,
        span: Span::Point,
        shape: |v| Shape {
            app: s(v, "app_label").or_else(|| s(v, "package")),
            ..Shape::default()
        },
    },
    Allow {
        sources: &["c01-app-usage"],
        when: |v| event_type_in(v, &[2, 23]),
        kind: Kind::Background,
        origin: Origin::Phone,
        span: Span::Point,
        shape: |v| Shape {
            app: s(v, "package"),
            ..Shape::default()
        },
    },
    Allow {
        sources: &["c01-app-usage"],
        when: |v| kind_is(v, "gap"),
        kind: Kind::Unavailable,
        origin: Origin::Phone,
        span: Span::BeginEnd("begin", "end"),
        shape: |v| Shape {
            reason: s(v, "reason"),
            ..Shape::default()
        },
    },
    Allow {
        sources: ARCHIVE_SOURCES,
        when: |_| true,
        kind: Kind::Archive,
        origin: Origin::Archive,
        span: Span::Point,
        shape: |v| Shape {
            product: s(v, "product"),
            title: s(v, "title")
                .or_else(|| s(v, "query"))
                .or_else(|| s(v, "semantic_type"))
                .or_else(|| s(v, "activity_type")),
            domain: domain(v),
            ..Shape::default()
        },
    },
];

/// 書庫の論理ソース（D2 の最後の行）。
pub const ARCHIVE_SOURCES: &[&str] = &[
    "c03-chrome-history",
    "c03-youtube-watch",
    "c03-youtube-search",
    "c03-myactivity-*",
    "c03-timeline-visit",
    "c03-timeline-move",
    "c03-legacy-visit",
    "c03-legacy-activity",
];

/// 書庫の位置の点（D11 が使う。ここでは記録として出さない）。
pub const ARCHIVE_LOCATION_SOURCES: &[&str] = &[
    "c03-timeline-route",
    "c03-timeline-signal",
    "c03-legacy-location",
];

/// 論理ソースが表（`pattern` は末尾 `*` で前方一致）に当たるか。
pub fn source_matches(pattern: &str, name: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => pattern == name,
    }
}

/// 許可リストが読む論理ソースの SQL の `LIKE` 形（`*` → `%`）。
pub(crate) fn like_patterns() -> Vec<String> {
    let mut out: Vec<String> = ALLOW
        .iter()
        .flat_map(|a| a.sources.iter())
        .map(|p| p.replace('*', "%"))
        .collect();
    out.sort();
    out.dedup();
    out
}

fn parse_time(v: &Value, key: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(v.get(key)?.as_str()?)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

/// 1 行を許可リストで正規化する（1 段目）。表に無ければ `None`。
/// 区間の鍵が読めない行も `None`（置き場所が決まらない記録は出さない）。
pub(crate) fn normalize(row: &StoredEvent) -> Option<Norm> {
    let allow = ALLOW.iter().find(|a| {
        a.sources
            .iter()
            .any(|p| source_matches(p, &row.logical_source))
            && (a.when)(&row.payload)
    })?;
    let (start, end) = match allow.span {
        Span::Point => (row.event_time, None),
        Span::EndKey(k) => (row.event_time, Some(parse_time(&row.payload, k)?)),
        Span::BeginEnd(b, e) => (
            parse_time(&row.payload, b)?,
            Some(parse_time(&row.payload, e)?),
        ),
    };
    let shape = (allow.shape)(&row.payload);
    Some(Norm {
        id: row.id,
        kind: allow.kind,
        origin: allow.origin,
        logical_source: row.logical_source.clone(),
        source_label: (allow.origin == Origin::Archive).then(|| row.display_name.clone()),
        device_id: row.device_id.clone(),
        tz_id: row.tz_id.clone(),
        start,
        end,
        app: shape.app,
        title: shape.title,
        domain: shape.domain,
        product: shape.product,
        reason: shape.reason,
        excluded_count: shape.excluded_count,
    })
}

fn is_enter(row: &StoredEvent) -> bool {
    row.logical_source == "c02-window"
        && kind_is(&row.payload, "idle")
        && transition_is(&row.payload, "enter")
}

fn is_leave(row: &StoredEvent) -> bool {
    row.logical_source == "c02-window"
        && kind_is(&row.payload, "idle")
        && transition_is(&row.payload, "leave")
}

/// 行の並びを正規化する（`now` は離席中の判定用）。
///
/// 離席は leave の記録だけを使う（区間は leave が持つ）。leave の無い enter は、
/// **今日に限り** enter の時刻からいままでの離席にする。leave は enter と同じ時刻・同じ端末に積まれる
/// （`engine.rs` の `leave_record(since, …)`）ので、その組で対にする。
pub(crate) fn normalize_all(rows: &[StoredEvent], now: DateTime<Utc>) -> Vec<Norm> {
    let left: HashSet<(Option<&str>, DateTime<Utc>)> = rows
        .iter()
        .filter(|r| is_leave(r))
        .map(|r| (r.device_id.as_deref(), r.event_time))
        .collect();
    let mut out = Vec::new();
    for row in rows {
        if is_enter(row) {
            let open = !left.contains(&(row.device_id.as_deref(), row.event_time))
                && row.event_time <= now
                && jst_date(row.event_time) == jst_date(now);
            if open {
                out.push(Norm {
                    id: row.id,
                    kind: Kind::Away,
                    origin: Origin::Pc,
                    logical_source: row.logical_source.clone(),
                    source_label: None,
                    device_id: row.device_id.clone(),
                    tz_id: row.tz_id.clone(),
                    start: row.event_time,
                    end: Some(now),
                    app: None,
                    title: None,
                    domain: None,
                    product: None,
                    reason: None,
                    excluded_count: None,
                });
            }
        } else if let Some(n) = normalize(row) {
            out.push(n);
        }
    }
    out
}

/// 2 台の PC から届いた同じ訪問を `(event_time, url)` で畳む（D4）。`ingest_time` の最も古い行が代表。
fn fold_visits(mut rows: Vec<StoredEvent>) -> Vec<StoredEvent> {
    rows.sort_by_key(|r| (r.ingest_time, r.id));
    let mut seen: HashSet<(DateTime<Utc>, String)> = HashSet::new();
    rows.into_iter()
        .filter(|r| {
            if r.logical_source != "c02-browser-history" || !kind_is(&r.payload, "visit") {
                return true;
            }
            match s(&r.payload, "url") {
                Some(url) => seen.insert((r.event_time, url)),
                None => true,
            }
        })
        .collect()
}

/// 許可リストの前置き（`c03-myactivity-*`）を、登録簿 `core.source` の具体名に開く SQL。
pub(crate) const SOURCES_SQL: &str =
    "SELECT logical_source FROM core.source WHERE logical_source LIKE ANY($1)";

/// `fetch` の SQL。論理ソースは具体名の `= ANY` で引く（D3。`LIKE ANY` だと
/// `event_by_source_time` が使われず、位置の記録も含めて ±40 日を全部走る。
/// サブクエリで開いても計画時に値が無く、同じく索引に乗らない）。
pub(crate) const FETCH_SQL: &str = "SELECT DISTINCT ON (e.logical_source, e.content_hash)
                e.id, e.logical_source, e.event_time, e.ingest_time, e.tz_id, e.device_id,
                e.payload, s.display_name
           FROM core.event_live e JOIN core.source s USING (logical_source)
          WHERE e.user_id = $1 AND e.logical_source = ANY($2)
            AND e.event_time >= $3 AND e.event_time < $4
          ORDER BY e.logical_source, e.content_hash, e.ingest_time, e.id";

/// 内容ハッシュで畳んだ行を日の範囲（±40 日）で読む（D3）。
/// `core.event_live` を日付で先に絞ってから畳む。感度は読まない（C5）。
async fn fetch(
    pool: &PgPool,
    user: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<StoredEvent>, sqlx::Error> {
    let margin = Duration::days(RANGE_MARGIN_DAYS);
    let sources: Vec<String> = sqlx::query_scalar(SOURCES_SQL)
        .bind(like_patterns())
        .fetch_all(pool)
        .await?;
    let rows = sqlx::query(FETCH_SQL)
        .bind(user)
        .bind(sources)
        .bind(from - margin)
        .bind(to + margin)
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|r| {
            Ok(StoredEvent {
                id: r.try_get("id")?,
                logical_source: r.try_get("logical_source")?,
                display_name: r.try_get("display_name")?,
                event_time: r.try_get("event_time")?,
                ingest_time: r.try_get("ingest_time")?,
                tz_id: r.try_get("tz_id")?,
                device_id: r.try_get("device_id")?,
                payload: r.try_get("payload")?,
            })
        })
        .collect()
}

/// `[from, to)` に当たる記録を、1 段目の形で時刻順に返す（D3〜D5）。`now` は離席中の判定用。
pub async fn read(
    pool: &PgPool,
    user: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Vec<Norm>, sqlx::Error> {
    let rows = fold_visits(fetch(pool, user, from, to).await?);
    let mut out: Vec<Norm> = normalize_all(&rows, now)
        .into_iter()
        .filter(|n| n.in_range(from, to))
        .collect();
    out.sort_by_key(|n| (n.start, n.id));
    Ok(out)
}

/// `Asia/Tokyo` のその日に当たる記録（1 段目）。
pub async fn read_day(
    pool: &PgPool,
    user: Uuid,
    day: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Vec<Norm>, sqlx::Error> {
    let (d0, d1) = day_bounds(day);
    read(pool, user, d0, d1, now).await
}
