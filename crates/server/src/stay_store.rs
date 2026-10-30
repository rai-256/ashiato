// SPDX-License-Identifier: AGPL-3.0-only
//! 滞在の作り直しと、基準の台帳（ST16 / FR-31 / FR-50 / FR-76。design D3 / D4 / D5 / D10）。
//!
//! 判定そのものは `stay`（DB に触らない）。ここが持つのは、**何を読んで判定に渡し、
//! 結果をどの行へどう書くか** —— 識別子の引き継ぎ（D3）と、本人が消した時間帯（D4）と、錠と範囲（D5）。
use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};

use crate::ingest;
use crate::stay::{self, Criteria, Point, Stay};

/// 作り直しの錠の名前空間（`pg_advisory_xact_lock(key, hashtext(user))` の 1 つ目）。
/// `testdb` のマイグレーションの錠（4820251。1 引数の形）とは別の空間。
pub(crate) const LOCK_KEY: i32 = 4_816_016;

/// 作り直しが付ける削除の印。**これ以外（`NULL` を含む）はすべて本人が消したもの**（design D4 / R14）。
pub const REBUILD_PREFIX: &str = "rebuild:";
/// どの新しい滞在にも割り当てられなかった（design D3）。
pub const ABSORBED: &str = "rebuild:absorbed";
/// 本人が消した時間帯と重なった（design D4）。
pub const ERASED_RANGE: &str = "rebuild:erased-range";
/// 本人が消した滞在の時間帯へ、消した後に届いた基準ソースの記録。
pub const USER_LATE: &str = "user:late";

/// 日の区切り（`Asia/Tokyo`）。JST は固定の +09:00（`lib.rs` の `today_jst` と同じ前提）。
pub fn day_bounds(day: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
    let start = day.and_time(NaiveTime::MIN).and_utc() - Duration::hours(9);
    (start, start + Duration::days(1))
}

/// その時刻が属する `Asia/Tokyo` の日。
pub fn jst_date(t: DateTime<Utc>) -> NaiveDate {
    (t + Duration::hours(9)).date_naive()
}

// ------------------------------------------------------------------ 基準（design D10）

/// 基準の版 1 つ（`GET /stays/criteria` の 1 行）。
#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CriteriaVersion {
    pub id: i64,
    pub radius_m: i32,
    pub min_minutes: i32,
    pub gap_minutes: i32,
    pub sources: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl From<CriteriaVersion> for Criteria {
    fn from(v: CriteriaVersion) -> Self {
        Self {
            id: v.id,
            radius_m: v.radius_m,
            min_minutes: v.min_minutes,
            gap_minutes: v.gap_minutes,
            sources: v.sources,
        }
    }
}

const CRITERIA_COLUMNS: &str = "id, radius_m, min_minutes, gap_minutes, sources, created_at";

/// 利用者のいまの基準。**行が 1 本も無ければ `None`**（呼び出し側が既定を使うか書くかを決める）。
pub async fn current_criteria<'e, E>(ex: E, user: uuid::Uuid) -> sqlx::Result<Option<Criteria>>
where
    E: sqlx::PgExecutor<'e>,
{
    let row: Option<CriteriaVersion> = sqlx::query_as(&format!(
        "SELECT {CRITERIA_COLUMNS} FROM core.stay_criteria
          WHERE user_id = $1 ORDER BY id DESC LIMIT 1"
    ))
    .bind(user)
    .fetch_optional(ex)
    .await?;
    Ok(row.map(Criteria::from))
}

/// 利用者のいまの基準。行が無ければ既定（**書かない**。読むだけの経路用）。
pub async fn criteria_or_default(pool: &PgPool, user: uuid::Uuid) -> sqlx::Result<Criteria> {
    Ok(current_criteria(pool, user)
        .await?
        .unwrap_or_else(Criteria::default_values))
}

/// 利用者の基準の版を古い順に返す。
pub async fn criteria_versions(
    pool: &PgPool,
    user: uuid::Uuid,
) -> sqlx::Result<Vec<CriteriaVersion>> {
    sqlx::query_as(&format!(
        "SELECT {CRITERIA_COLUMNS} FROM core.stay_criteria WHERE user_id = $1 ORDER BY id"
    ))
    .bind(user)
    .fetch_all(pool)
    .await
}

/// いまの基準を返す。**行が無ければ既定を最初の版として書く**（design D10）。錠の中で呼ぶ。
async fn ensure_criteria(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
) -> sqlx::Result<Criteria> {
    if let Some(c) = current_criteria(&mut **tx, user).await? {
        return Ok(c);
    }
    insert_criteria(tx, user, &Criteria::default_values()).await
}

async fn insert_criteria(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    c: &Criteria,
) -> sqlx::Result<Criteria> {
    let row: CriteriaVersion = sqlx::query_as(&format!(
        "INSERT INTO core.stay_criteria
           (user_id, radius_m, min_minutes, gap_minutes, sources, server_version)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING {CRITERIA_COLUMNS}"
    ))
    .bind(user)
    .bind(c.radius_m)
    .bind(c.min_minutes)
    .bind(c.gap_minutes)
    .bind(&c.sources)
    .bind(crate::BUILD_STAMP)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.into())
}

/// 基準の変更。**省いた値はいまの値のまま。** いまと同じなら版を足さない（spec「いまと同じ基準を添えても基準の版は増えない」）。
///
/// 範囲の検査は呼び出し側（API が 400 にする）。ここへ範囲外が来たら DB の `CHECK` が落とす。
pub async fn set_criteria(
    pool: &PgPool,
    user: uuid::Uuid,
    radius_m: Option<i32>,
    min_minutes: Option<i32>,
    gap_minutes: Option<i32>,
) -> sqlx::Result<Criteria> {
    let mut tx = pool.begin().await?;
    lock(&mut tx, user).await?;
    let now = ensure_criteria(&mut tx, user).await?;
    let wanted = Criteria {
        radius_m: radius_m.unwrap_or(now.radius_m),
        min_minutes: min_minutes.unwrap_or(now.min_minutes),
        gap_minutes: gap_minutes.unwrap_or(now.gap_minutes),
        ..now.clone()
    };
    let got = if wanted == now {
        now
    } else {
        insert_criteria(&mut tx, user, &wanted).await?
    };
    tx.commit().await?;
    Ok(got)
}

/// 利用者の基準を、書く前提で揃える（既定の版が無ければ書く）。
pub async fn ensure_criteria_committed(pool: &PgPool, user: uuid::Uuid) -> sqlx::Result<Criteria> {
    let mut tx = pool.begin().await?;
    lock(&mut tx, user).await?;
    let c = ensure_criteria(&mut tx, user).await?;
    tx.commit().await?;
    Ok(c)
}

/// **利用者ごとに直列化する**（design D5 / R16）。トランザクションが終わると外れる。
async fn lock(tx: &mut Transaction<'_, Postgres>, user: uuid::Uuid) -> sqlx::Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text))")
        .bind(LOCK_KEY)
        .bind(user)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

// ------------------------------------------------------------------ 読むもの

/// 判定の入力にする位置を引く文。**`event_by_source_time` に乗る形**（`stay_day_query_uses_index` が見る）。
/// 論理削除された記録は `core.event_live` が外す。
pub(crate) const POINTS_SQL: &str = "SELECT event_time, tz_offset_min, tz_id,
        payload->'lat' AS lat, payload->'lon' AS lon, payload->'acc_m' AS acc_m
   FROM core.event_live
  WHERE logical_source = ANY($1) AND event_time >= $2 AND event_time < $3 AND user_id = $4
  ORDER BY event_time";

#[derive(sqlx::FromRow)]
struct PointRow {
    event_time: DateTime<Utc>,
    tz_offset_min: i32,
    tz_id: String,
    lat: Option<serde_json::Value>,
    lon: Option<serde_json::Value>,
    acc_m: Option<serde_json::Value>,
}

pub(crate) async fn load_points<'e, E>(
    ex: E,
    user: uuid::Uuid,
    sources: &[String],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> sqlx::Result<Vec<Point>>
where
    E: sqlx::PgExecutor<'e>,
{
    let rows: Vec<PointRow> = sqlx::query_as(POINTS_SQL)
        .bind(sources)
        .bind(from)
        .bind(to)
        .bind(user)
        .fetch_all(ex)
        .await?;
    // **数値でない緯度経度を数えて残す**（R43）。本文を消去した記録（`'{}'`）と、形の壊れた記録を
    // 同じ「無い」に畳むと、壊れた記録の日が黙って「記録なし」になる。出すのは件数だけ（値は出さない）
    let malformed = rows
        .iter()
        .filter(|r| {
            [&r.lat, &r.lon, &r.acc_m]
                .into_iter()
                .any(|v| v.as_ref().is_some_and(|v| !v.is_number()))
        })
        .count();
    if malformed > 0 {
        tracing::warn!(
            kind = "stay.points_malformed",
            count = malformed,
            "緯度・経度・精度が数値でない位置の記録を判定に使わなかった"
        );
    }
    Ok(rows
        .into_iter()
        .map(|r| Point {
            at: r.event_time,
            // **数値でない値は「無い」と読む** —— 文字列の緯度で判定を落とすと、その日の滞在が作れなくなる
            lat: r.lat.as_ref().and_then(serde_json::Value::as_f64),
            lon: r.lon.as_ref().and_then(serde_json::Value::as_f64),
            acc_m: r.acc_m.as_ref().and_then(serde_json::Value::as_f64),
            tz_offset_min: r.tz_offset_min,
            tz_id: r.tz_id,
        })
        .collect())
}

/// 既存の滞在の状態（design D3 / D4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    /// 読み出しに出ている
    Live,
    /// 作り直しで吸収された（`rebuild:absorbed`。`rebuild:` で始まる未知の印もここ）
    Absorbed,
    /// 本人が消した時間帯と重なって隠れている（`rebuild:erased-range`）
    ErasedRange,
    /// 本人が消した。**作り直しは触らない**
    UserDeleted,
}

impl Mark {
    pub(crate) fn of(deleted_at: Option<DateTime<Utc>>, deleted_by: Option<&str>) -> Self {
        match (deleted_at, deleted_by) {
            (None, _) => Self::Live,
            (Some(_), Some(ERASED_RANGE)) => Self::ErasedRange,
            (Some(_), Some(by)) if by.starts_with(REBUILD_PREFIX) => Self::Absorbed,
            // **`NULL` も本人の削除**（R14。`NOT LIKE` だけで絞ると三値論理で落ちる）
            (Some(_), _) => Self::UserDeleted,
        }
    }
}

/// 既存の滞在 1 行。
#[derive(Debug, Clone)]
pub(crate) struct Existing {
    pub id: uuid::Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub raw: String,
    pub mark: Mark,
    /// 読んだときの `deleted_by`（書き換えるときに、読んだ後に変わっていないことを確かめる。R44）
    pub deleted_by: Option<String>,
    /// 本文ごと消した滞在の、本文ごと消した操作（`core.erasure_operation.id`）。
    /// **範囲の行が無く、その日全体として読む窓（ST23 D14）には無い**。
    pub operation_id: Option<uuid::Uuid>,
}

#[derive(sqlx::FromRow)]
struct ExistingRow {
    id: uuid::Uuid,
    event_time: DateTime<Utc>,
    raw: String,
    payload: serde_json::Value,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<String>,
}

/// 期間の読み方。**取り込みの口から入った滞在は形が崩れていることがある**ので、
/// 読めない終わりは始まりと同じと読む（その行は作り直しで置き換わるか吸収される。design D2）。
pub(crate) fn span_of(
    event_time: DateTime<Utc>,
    payload: &serde_json::Value,
) -> (DateTime<Utc>, DateTime<Utc>) {
    let end = payload
        .get("end")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc))
        .filter(|t| *t >= event_time)
        .unwrap_or(event_time);
    (event_time, end)
}

/// 本文ごと消した滞在の 1 行（識別子・始まり・終わり・消した印）。
type PurgedStay = (
    uuid::Uuid,
    DateTime<Utc>,
    DateTime<Utc>,
    Option<String>,
    Option<uuid::Uuid>,
);

/// `[from, to)` と時間が重なる（閉区間で触れるものを含む）既存の滞在。**候補は `origin='derived'` の `s01-stay` だけ**（D2 / D3）。
///
/// 滞在の長さに上限は無いので、始まりだけでは絞れない。`payload->>'end'` の文字列比較で粗く絞ってから
/// 読んだ終わりで正確に絞る（2 日の余裕を取るので、秒の端数の書き方の違いは効かない）。
///
/// **本文を消した滞在（`raw = ''`）は候補に入れない**（作り直しが本文を戻さないため。design D6）。
/// その時間は `core.stay_erased`（台帳の範囲。範囲の行が無ければその日全体。D14）から読み、
/// 本人が消した時間帯（`Mark::UserDeleted`）として足す。重なる新しい滞在はこれで `rebuild:erased-range` に隠れる。
async fn load_existing(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> sqlx::Result<Vec<Existing>> {
    let rough = from - Duration::days(2);
    let rows: Vec<ExistingRow> = sqlx::query_as(
        "SELECT id, event_time, raw, payload, deleted_at, deleted_by
           FROM core.event
          WHERE logical_source = $1 AND origin = 'derived' AND user_id = $2
            AND raw <> ''
            AND event_time < $3
            AND (event_time >= $4 OR payload->>'end' >= $5)
          ORDER BY event_time, id",
    )
    .bind(stay::SOURCE)
    .bind(user)
    .bind(to)
    .bind(rough)
    .bind(rough.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    .fetch_all(&mut **tx)
    .await?;
    let purged: Vec<PurgedStay> = sqlx::query_as(
        "SELECT id, start_at, end_at, deleted_by, operation_id FROM core.stay_erased_purge
              WHERE user_id = $1 AND purged AND start_at < $2 AND end_at >= $3
              ORDER BY start_at, id",
    )
    .bind(user)
    .bind(to)
    .bind(from)
    .fetch_all(&mut **tx)
    .await?;
    let mut existing: Vec<Existing> = rows
        .into_iter()
        .filter_map(|r| {
            let (start, end) = span_of(r.event_time, &r.payload);
            (end >= from).then(|| Existing {
                id: r.id,
                start,
                end,
                lat: r.payload.get("lat").and_then(serde_json::Value::as_f64),
                lon: r.payload.get("lon").and_then(serde_json::Value::as_f64),
                raw: r.raw,
                mark: Mark::of(r.deleted_at, r.deleted_by.as_deref()),
                deleted_by: r.deleted_by,
                operation_id: None,
            })
        })
        .collect();
    existing.extend(purged.into_iter().map(
        |(id, start, end, deleted_by, operation_id): PurgedStay| Existing {
            id,
            start,
            end,
            lat: None,
            lon: None,
            raw: String::new(),
            mark: Mark::UserDeleted,
            deleted_by,
            operation_id,
        },
    ));
    existing.sort_by(|a, b| a.start.cmp(&b.start).then(a.id.cmp(&b.id)));
    Ok(existing)
}

// ------------------------------------------------------------------ 範囲（design D5）

/// 求められた日から作り直しの範囲を広げる（design D5 / R15）。**変わらなくなるまで繰り返す。**
///
/// 1. 範囲の端を**またぐ**、読み出しに出ている（または消した時間帯で隠れている）既存の滞在を含むまで広げる
/// 2. 範囲の端をまたいで続いている集まり（最短のとどまりに満たないものも含む）を含むまで広げる
///
/// **1 は端の「近く」の滞在では広げない**（R40）。近くで広げていたときは、短い移動でつながった滞在を
/// 1 件ずつたどり、数日で 64 回の上限に当たって、その利用者の作り直しが毎回落ちた（code-reviewer の実測）。
/// 範囲と重ならない既存の滞在には範囲の中の新しい滞在が重ならないので、引き継ぎにも消した時間帯にも効かない。
/// 0 時の前後で別々に届いた位置は 2 が拾う。
///
/// 2 で見る集まりは、範囲の前後を広めに読んで判定する。**広げた端では集まりが切れている**ので、
/// 最後に `[lo, hi)` だけで判定しても、全期間で判定したときと同じ滞在になる。
async fn settle_range(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    c: &Criteria,
    (mut lo, mut hi): (DateTime<Utc>, DateTime<Utc>),
) -> anyhow::Result<(DateTime<Utc>, DateTime<Utc>)> {
    let tick = Duration::microseconds(1);
    // 読む幅は範囲の長さに合わせて伸ばす —— 1 日ずつだと、何日も続くとどまりで回数が日数に比例する
    for _ in 0..64 {
        let (mut nlo, mut nhi) = (lo, hi);
        for e in load_existing(tx, user, lo, hi).await? {
            if matches!(e.mark, Mark::Live | Mark::ErasedRange) {
                if e.start < lo && e.end >= lo {
                    nlo = nlo.min(e.start);
                }
                if e.start < hi && e.end >= hi {
                    nhi = nhi.max(e.end + tick);
                }
            }
        }
        let look = (nhi - nlo).max(Duration::days(1));
        let points = load_points(&mut **tx, user, &c.sources, nlo - look, nhi + look).await?;
        for k in stay::clusters(&points, c.radius_m, c.gap_minutes) {
            if k.start < nlo && k.end >= nlo {
                nlo = k.start;
            }
            if k.start < nhi && k.end >= nhi {
                nhi = k.end + tick;
            }
        }
        if (nlo, nhi) == (lo, hi) {
            return Ok((lo, hi));
        }
        (lo, hi) = (nlo, nhi);
    }
    Err(RangeUnsettled.into())
}

/// 作り直しの範囲が定まらなかった（`settle_range` の上限）。**値を含まない**のでログの種別に使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeUnsettled;

impl std::fmt::Display for RangeUnsettled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("作り直しの範囲が 64 回で定まらない")
    }
}

impl std::error::Error for RangeUnsettled {}

/// 読んだ後に削除の印が変わっていた（R44）。**書き換えずに作り直しを巻き戻す**（次の作り直しで読み直す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkChanged;

impl std::fmt::Display for MarkChanged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("読んだ後に滞在の削除の印が変わっていた")
    }
}

impl std::error::Error for MarkChanged {}

/// 作り直しの失敗の種別（ログ用。**値を含まない**）。
pub fn failure_kind(e: &anyhow::Error) -> String {
    if e.downcast_ref::<RangeUnsettled>().is_some() {
        "range_unsettled".into()
    } else if e.downcast_ref::<MarkChanged>().is_some() {
        "mark_changed".into()
    } else if let Some(db) = e.downcast_ref::<sqlx::Error>() {
        match db.as_database_error().and_then(|d| d.code()) {
            Some(code) => format!("sqlstate:{code}"),
            // 変種の名前は値を含まない（`PoolTimedOut` / `Io` / `Decode` …）
            None => format!("sqlx:{}", sqlx_variant(db)),
        }
    } else {
        "other".into()
    }
}

fn sqlx_variant(e: &sqlx::Error) -> &'static str {
    match e {
        sqlx::Error::PoolTimedOut => "pool_timed_out",
        sqlx::Error::PoolClosed => "pool_closed",
        sqlx::Error::Io(_) => "io",
        sqlx::Error::Tls(_) => "tls",
        sqlx::Error::Protocol(_) => "protocol",
        sqlx::Error::RowNotFound => "row_not_found",
        sqlx::Error::ColumnDecode { .. } | sqlx::Error::Decode(_) => "decode",
        _ => "other",
    }
}

// ------------------------------------------------------------------ 割り当て（design D3）

/// 時間の重なり（正のときだけ）。
fn overlap(a: (DateTime<Utc>, DateTime<Utc>), b: (DateTime<Utc>, DateTime<Utc>)) -> Duration {
    (a.1.min(b.1) - a.0.max(b.0)).max(Duration::zero())
}

/// 割り当ての結果。
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Assignment {
    /// 新しい滞在 `i` が引き継ぐ既存の滞在の位置
    pub inherits: Vec<Option<usize>>,
    /// 吸収される既存の滞在の位置と、吸収先の新しい滞在の位置
    pub absorbed: Vec<(usize, Option<usize>)>,
}

/// 識別子の割り当て（design D3。**本人の決定なので規則を変えない**）。
///
/// 1. 本人が消した滞在は対象にしない
/// 2. 代表点どうしが半径の 2 倍より離れている組を捨てる（Q11）。代表点の無い既存の滞在は継がせない
/// 3. 残った組を時間の重なりの大きい順に、どちらもまだ使われていない組から割り当てる（Q10: 1 対 1）。
///    同じ重なりなら読み出しに出ている既存の滞在 → 始まりの早い既存の滞在 → 始まりの早い新しい滞在 → 識別子の順
/// 4. 割り当てのない既存の滞在のうち、読み出しに出ているものと消した時間帯で隠れているものを吸収する。
///    吸収先は時間の重なりがいちばん大きい新しい滞在（**距離は見ない**。無ければ `None`）
pub(crate) fn assign(fresh: &[Stay], existing: &[Existing], radius_m: i32) -> Assignment {
    let limit = f64::from(radius_m) * 2.0;
    let mut pairs = Vec::new();
    for (i, f) in fresh.iter().enumerate() {
        for (j, e) in existing.iter().enumerate() {
            if e.mark == Mark::UserDeleted {
                continue;
            }
            let ov = overlap((f.start, f.end), (e.start, e.end));
            if ov <= Duration::zero() {
                continue;
            }
            let (Some(lat), Some(lon)) = (e.lat, e.lon) else {
                continue;
            };
            if stay::distance_m(f.lat, f.lon, lat, lon) > limit {
                continue;
            }
            pairs.push((ov, e.mark != Mark::Live, e.start, f.start, e.id, i, j));
        }
    }
    pairs.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
            .then(a.3.cmp(&b.3))
            .then(a.4.cmp(&b.4))
    });

    let mut inherits = vec![None; fresh.len()];
    let mut taken = vec![false; existing.len()];
    for (_, _, _, _, _, i, j) in pairs {
        if inherits[i].is_none() && !taken[j] {
            inherits[i] = Some(j);
            taken[j] = true;
        }
    }

    let absorbed = existing
        .iter()
        .enumerate()
        .filter(|(j, e)| !taken[*j] && matches!(e.mark, Mark::Live | Mark::ErasedRange))
        .map(|(j, e)| {
            let into = fresh
                .iter()
                .enumerate()
                .map(|(i, f)| (overlap((f.start, f.end), (e.start, e.end)), i))
                .filter(|(ov, _)| *ov > Duration::zero())
                // 重なりの大きい順、同じなら早い方（`max_by` は後勝ちなので位置を逆に比べる）
                .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)))
                .map(|(_, i)| i);
            (j, into)
        })
        .collect();
    Assignment { inherits, absorbed }
}

/// 本人が消した時間帯と**少しでも**重なるか（Q12。触れるだけでも重なりとする —— 既定は厳しい側）。
fn touches_erased(f: &Stay, erased: &[(DateTime<Utc>, DateTime<Utc>)]) -> bool {
    erased.iter().any(|(s, e)| f.start <= *e && *s <= f.end)
}

/// 作り直しが隠す、本文ごと消した時間帯と重なる既存の滞在を選び、`plan` から外す（CH08 design D2 / D3）。
/// 返すのは (本文を消す滞在の位置, 元の操作を持つ窓の位置)。
///
/// 本文を消す滞在は、作り直しの前に一覧に出ていて（`Mark::Live`）、時間が本文ごと消した時間帯と**重なり**（端が触れるだけは含めない）、
/// この作り直しが隠すもの —— 吸収される、または引き継ぐ新しい滞在が消した時間帯に触れて隠れる。
/// 後者は引き継がせず（`inherits` を外す）、新しい滞在を隠して作らせる。
/// 操作の識別子の無い窓（範囲の行が無く、その日全体として読むもの）は使わない。
/// 窓が複数なら重なりの大きいもの → 始まりの早いもの → 操作の識別子の順に選ぶ。
fn take_hidden_overlaps(
    fresh: &[Stay],
    existing: &[Existing],
    erased: &[(DateTime<Utc>, DateTime<Utc>)],
    plan: &mut Assignment,
) -> Vec<(usize, usize)> {
    let mut hidden = Vec::new();
    for (j, l) in existing.iter().enumerate() {
        if l.mark != Mark::Live {
            continue;
        }
        let parent = existing
            .iter()
            .enumerate()
            .filter(|(_, w)| w.operation_id.is_some() && l.start < w.end && w.start < l.end)
            .min_by_key(|(_, w)| {
                (
                    std::cmp::Reverse(overlap((l.start, l.end), (w.start, w.end))),
                    w.start,
                    w.operation_id,
                )
            })
            .map(|(k, _)| k);
        let Some(parent) = parent else {
            continue;
        };
        let absorbed = plan.absorbed.iter().any(|(k, _)| *k == j);
        let hiding = plan
            .inherits
            .iter()
            .zip(fresh)
            .position(|(inherit, f)| *inherit == Some(j) && touches_erased(f, erased));
        if absorbed {
            plan.absorbed.retain(|(k, _)| *k != j);
        } else if let Some(i) = hiding {
            plan.inherits[i] = None;
        } else {
            continue;
        }
        hidden.push((j, parent));
    }
    hidden
}

// ------------------------------------------------------------------ 作り直し

/// 1 日ぶん（広げた範囲）の作り直しの結果。**値を含まない**のでログに出してよい。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DayOutcome {
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub absorbed: usize,
}

/// その日（`Asia/Tokyo`）の滞在を、いまの基準で作り直す（design D3 / D4 / D5）。
///
/// **1 トランザクション**。先頭で利用者の錠を取るので、同じ利用者の作り直しは同時に 1 つしか走らない。
pub async fn rebuild_day(
    pool: &PgPool,
    user: uuid::Uuid,
    day: NaiveDate,
) -> anyhow::Result<DayOutcome> {
    rebuild_day_using(pool, user, day, "event").await
}

/// 作り直しが隠した重なる滞在の消去の台帳（最後の書き込みの手前）の CHECK 違反を起こし、
/// 本文が空になった後でも作り直しごと rollback されることを試す口。
#[cfg(test)]
pub(crate) async fn rebuild_day_with_ledger_scope(
    pool: &PgPool,
    user: uuid::Uuid,
    day: NaiveDate,
    ledger_scope: &str,
) -> anyhow::Result<DayOutcome> {
    rebuild_day_using(pool, user, day, ledger_scope).await
}

async fn rebuild_day_using(
    pool: &PgPool,
    user: uuid::Uuid,
    day: NaiveDate,
    ledger_scope: &str,
) -> anyhow::Result<DayOutcome> {
    let mut tx = pool.begin().await?;
    let out = rebuild_day_in(&mut tx, user, day, ledger_scope).await?;
    tx.commit().await?;
    Ok(out)
}

/// `rebuild_day` の本体。呼び出し側のトランザクションで走り、commit も rollback もしない
/// （戻す口が、戻した結果を同じまとまりの中で作り直して確かめる。CH08 design D4）。錠は同じ鍵を取り直すだけ。
pub(crate) async fn rebuild_day_in(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    day: NaiveDate,
    ledger_scope: &str,
) -> anyhow::Result<DayOutcome> {
    lock(tx, user).await?;
    let c = ensure_criteria(tx, user).await?;
    let bounds = day_bounds(day);
    mark_late_arrivals(tx, user, &with_archive_sources(&c.sources), bounds, false).await?;
    let (lo, hi) = settle_range(tx, user, &c, bounds).await?;

    let points = load_points(&mut **tx, user, &c.sources, lo, hi).await?;
    let fresh = stay::detect(&points, &c);
    let existing = load_existing(tx, user, lo, hi).await?;
    let erased: Vec<_> = existing
        .iter()
        .filter(|e| e.mark == Mark::UserDeleted)
        .map(|e| (e.start, e.end))
        .collect();
    let mut plan = assign(&fresh, &existing, c.radius_m);
    let hidden = take_hidden_overlaps(&fresh, &existing, &erased, &mut plan);

    let mut out = DayOutcome::default();
    let mut ids = Vec::with_capacity(fresh.len());
    for (f, inherit) in fresh.iter().zip(&plan.inherits) {
        let hide = touches_erased(f, &erased);
        let raw = stay::raw(f, &c);
        match inherit.map(|j| &existing[j]) {
            Some(e) => {
                ids.push(e.id);
                let mark_ok = if hide {
                    e.mark == Mark::ErasedRange
                } else {
                    e.mark == Mark::Live
                };
                if e.raw == raw && mark_ok {
                    out.unchanged += 1;
                    continue;
                }
                if e.raw != raw {
                    push_version(tx, e.id).await?;
                }
                update_stay(tx, e, f, &raw, hide).await?;
                out.updated += 1;
            }
            None => {
                let id = uuid::Uuid::new_v4();
                ids.push(id);
                insert_stay(tx, user, id, f, &raw, hide).await?;
                out.created += 1;
            }
        }
    }
    for (j, into) in &plan.absorbed {
        absorb(tx, user, &existing[*j], into.map(|i| ids[i]), c.id).await?;
        out.absorbed += 1;
    }
    for (j, parent) in hidden {
        let (l, p) = (&existing[j], &existing[parent]);
        let Some(parent_id) = p.operation_id else {
            continue;
        };
        crate::deletion::purge_hidden_overlap(
            tx,
            crate::deletion::HiddenOverlap {
                user_id: user,
                event_id: l.id,
                deleted_by: l.deleted_by.clone(),
                start: l.start,
                end: l.end,
                parent_id,
                stay_id: p.id,
                parent_start: p.start,
                parent_end: p.end,
            },
            ledger_scope,
        )
        .await?;
    }
    Ok(out)
}

/// 本人が消した滞在のある利用者に、遅れて届いた位置の印を付ける（ST33 design D6）。
///
/// `rebuild_day` と同じ順（transaction → 錠 → 基準）で、`mark_late_arrivals` を `rebuild_day` と同じ並びで 1 回呼ぶだけ
/// （判定は書き写さない）。範囲は本人が消した滞在の最小の始まりの日（`Asia/Tokyo`）の始まり〜（最大の終わり＋1 マイクロ秒）。
/// 下端を日の始まりにするのは、消した滞在より前に始まって消した時間にかかる書庫の区間を候補に入れるため
/// （`rebuild_day` が 1 日の範囲を渡すのと同じ）。上端を足すのは `mark_late_arrivals` が終わりを含まないため。
/// 本人が消した滞在が無ければ何もしない。何度呼んでも印と台帳の行は増えない。
pub async fn mark_late_for_user(pool: &PgPool, user: uuid::Uuid) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    lock(&mut tx, user).await?;
    let c = ensure_criteria(&mut tx, user).await?;
    let span: (Option<DateTime<Utc>>, Option<DateTime<Utc>>) = sqlx::query_as(&format!(
        "SELECT min(s.start_at), max(s.end_at)
           FROM core.stay_erased s
          WHERE s.user_id = $1
            AND {erased}",
        erased = user_erased_stay_sql("s", 2)
    ))
    .bind(user)
    .bind(format!("{REBUILD_PREFIX}%"))
    .fetch_one(&mut *tx)
    .await?;
    if let (Some(first), Some(last)) = span {
        let from = day_bounds(jst_date(first)).0;
        let to = last + Duration::microseconds(1);
        mark_late_arrivals(
            &mut tx,
            user,
            &with_archive_sources(&c.sources),
            (from, to),
            false,
        )
        .await?;
    }
    tx.commit().await
}

/// 本人が消した滞在を持つ利用者（書き出しが利用者ごとに遅着の印を付ける対象。ST33 design D6）。
pub(crate) async fn users_with_erased_stays(pool: &PgPool) -> sqlx::Result<Vec<uuid::Uuid>> {
    sqlx::query_scalar(&format!(
        "SELECT DISTINCT s.user_id FROM core.stay_erased s WHERE {erased} ORDER BY s.user_id",
        erased = user_erased_stay_sql("s", 1)
    ))
    .bind(format!("{REBUILD_PREFIX}%"))
    .fetch_all(pool)
    .await
}

/// 基準のソースに書庫の位置の論理ソースを足した集合（design D22）。滞在の判定（`Criteria::sources`）は変えない。
pub(crate) fn with_archive_sources(base: &[String]) -> Vec<String> {
    let mut all = base.to_vec();
    for source in crate::archive::LOCATION_SOURCES {
        if !all.iter().any(|s| s == source) {
            all.push(source.to_owned());
        }
    }
    all
}

/// 「本人が消した滞在」の SQL 条件（`alias` は `core.stay_erased` の別名、`prefix` は作り直しの印の接頭辞を束ねる `$n` の番号）。
/// `mark_late_for_user` の範囲と `mark_late_arrivals` の候補が同じ判定を使う（D6）。
/// ビューが派生の滞在・削除済み・`user:purged` 除外を持つので、ここは台帳の最新の行が erase で、
/// かつ作り直しの置き換え（`REBUILD_PREFIX`）で消えたものではない、だけを見る。時間もビューの `start_at` / `end_at`（D5）。
fn user_erased_stay_sql(alias: &str, prefix: u8) -> String {
    format!(
        "({a}.deleted_by IS NULL OR {a}.deleted_by NOT LIKE ${prefix})
            AND EXISTS (
              SELECT 1 FROM core.deletion_ledger d
               WHERE d.event_id = {a}.id
                 AND d.action = 'erase'
                 AND d.seq = (
                   SELECT max(last.seq) FROM core.deletion_ledger last
                    WHERE last.event_id = {a}.id
                 )
            )",
        a = alias
    )
}

/// 記録の終わりの SQL 式。区間の記録（`end_time`）は終わりを、点は始まりを返す
/// （`end_time` はオフセット付きの時刻か、移行前のミリ秒）。重なりは端が触れるだけでも成り立つ（Q12）。
/// 終わりの列（ST41）が始まりより後ならそれを先に読む —— 書き出し（`export::excluded`）と同じ順（D11。review/code.md R18）。
pub(crate) fn event_end_sql(alias: &str) -> String {
    format!(
        "coalesce(CASE WHEN {a}.end_time > {a}.event_time THEN {a}.end_time END,
                  core.try_timestamptz({a}.payload->>'end_time'),
                  CASE WHEN {a}.payload->>'end_time' ~ '^[0-9]{{1,15}}$'
                       THEN to_timestamp(({a}.payload->>'end_time')::numeric / 1000.0) END,
                  {a}.event_time)",
        a = alias
    )
}

/// 消した時間帯 `[start, end]`（SQL の束縛の番号）と重なる記録の条件（design D22）。
///
/// 区間を持つ書庫のソース（`archive::INTERVAL_SOURCES`。束縛 `intervals`）だけを終わりで見て、
/// それ以外（束縛 `points`。基準のソースと点のソース）は始まりの時刻が時間帯の中にあるかで見る。
/// 点の終わりは始まりと同じなので意味は 1 本の式と変わらず、点の側には索引
/// `event_by_source_time (logical_source, event_time)` の上下限が効く（final review 第 2 回 R73）。
/// 端が触れるだけでも重なりとする（Q12）。
pub(crate) fn overlaps_erased_sql(
    alias: &str,
    points: &str,
    intervals: &str,
    start: &str,
    end: &str,
) -> String {
    format!(
        "(({a}.logical_source = ANY({points}) AND {a}.event_time >= {start} AND {a}.event_time <= {end})
          OR ({a}.logical_source = ANY({intervals}) AND {a}.event_time <= {end} AND {e} >= {start}))",
        a = alias,
        e = event_end_sql(alias),
    )
}

/// マイアクティビティの論理ソースの前置き（`archive::myactivity::source_name`）。
const MYACTIVITY_PREFIX: &str = "c03-myactivity-";

/// 位置の欄を持てば印付けの対象になる項目の論理ソース（design D22-b / D22-d）。
/// 登録済みのマイアクティビティ（製品ごとに 1 本）に、固定名の `archive::ITEM_SOURCES` を足したもの。
pub(crate) async fn item_sources(executor: impl sqlx::PgExecutor<'_>) -> sqlx::Result<Vec<String>> {
    let mut sources: Vec<String> =
        sqlx::query_scalar("SELECT logical_source FROM core.source WHERE logical_source LIKE $1")
            .bind(format!("{MYACTIVITY_PREFIX}%"))
            .fetch_all(executor)
            .await?;
    sources.extend(crate::archive::ITEM_SOURCES.iter().map(|s| (*s).to_owned()));
    Ok(sources)
}

/// 論理ソースが、位置の欄を持てば印付けの対象になる項目のものか（`item_sources` と `item_located_sql` と同じ範囲）。
fn is_item_source(logical_source: &str) -> bool {
    logical_source.starts_with(MYACTIVITY_PREFIX)
        || crate::archive::ITEM_SOURCES.contains(&logical_source)
}

/// 原文の項目が位置（`locationInfos`）の欄を空でない値で持つか（design D22-b / 第 5 回 Q14）。
///
/// **中身の形は見ない**（C: 既定は厳しい側。code-verify 第 5 回 R91）。実物の形はこの repo の材料に無いので、
/// 配列でなくても（1 件だけのときのオブジェクト・文字列など）印を付ける。外すのは `null` と空の配列・
/// オブジェクト・文字列だけ。
///
/// **`item_located_sql` と同じ判定にする**（片方だけを直すと、印付けの範囲と印を付ける条件が黙ってずれる。
/// `archive_myactivity_location_rust_and_sql_agree` が固定する）。欄の名前が原文に無ければ解析しない（同じ前置き）。
pub(crate) fn carries_location(raw: &str) -> bool {
    if !raw.contains(LOCATION_KEY) {
        return false;
    }
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|item| {
            Some(match item.get("locationInfos")? {
                serde_json::Value::Null => false,
                serde_json::Value::Array(values) => !values.is_empty(),
                serde_json::Value::Object(fields) => !fields.is_empty(),
                serde_json::Value::String(text) => !text.is_empty(),
                serde_json::Value::Bool(_) | serde_json::Value::Number(_) => true,
            })
        })
        .unwrap_or(false)
}

/// 原文に位置の欄の名前があるかの安い前置き（引用符ごと見る。`carries_location` と SQL で共有する）。
const LOCATION_KEY: &str = "\"locationInfos\"";

/// 位置を持ちうる項目（マイアクティビティ・YouTube・Chrome）の行は、原文に位置を持つものだけを通す SQL の条件
/// （他のソースは常に通す）。範囲は `is_item_source` と同じ。
/// `raw` は text なので、JSON として読めるときだけ中を見る（入れ子の CASE で評価の順を固定する）。
///
/// **欄の名前が原文に無い行は jsonb として読まない**（final review 第 3 回 R85）。助言ロックを握ったまま
/// 全期間の Takeout の範囲を評価するので、位置を持たない大多数の行の解析を文字列の照合 1 回で畳む。
/// 前置きが偽で中を見ると真になるのは、欄の名前を `\u` でエスケープした原文だけ（Takeout はそう書かない）。
/// `carries_location` も同じ前置きを置くので、両者の判定は一致したまま。
pub(crate) fn item_located_sql(alias: &str) -> String {
    format!(
        "(CASE WHEN ({a}.logical_source LIKE '{MYACTIVITY_PREFIX}%'
                    OR {a}.logical_source IN ({fixed}))
               THEN CASE WHEN strpos({a}.raw, '{LOCATION_KEY}') = 0 THEN false
                         WHEN pg_input_is_valid({a}.raw, 'jsonb')
                         THEN COALESCE({a}.raw::jsonb->'locationInfos'
                                       NOT IN ('null', '[]', '{{}}', '\"\"'), false)
                         ELSE false END
               ELSE true END)",
        a = alias,
        fixed = crate::archive::ITEM_SOURCES
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// 消す側の論理ソースの集合を、点で見るもの（基準のソースと点のソース）と区間で見るもの
/// （`archive::INTERVAL_SOURCES`）に分ける（`overlaps_erased_sql` の 2 つの束縛）。
pub(crate) fn split_by_span(sources: &[String]) -> (Vec<String>, Vec<String>) {
    sources
        .iter()
        .cloned()
        .partition(|s| !crate::archive::INTERVAL_SOURCES.contains(&s.as_str()))
}

/// 書庫 1 冊の格納を commit した直後に、消した滞在の時間帯と重なる書庫の位置へ印を付ける（design D22 (1)）。
///
/// **見る範囲はその書庫が入れた位置の始まりの時刻の範囲だけ**（final review 第 2 回 R73）。この書庫の行は
/// すべてその範囲にあり、それより前に格納された行は、その書庫の印付けか消すときの連鎖が既に見ている。
/// **位置を持たない項目だけの書庫では何もしない**（`locationInfos` を持たない YouTube・Chrome・
/// マイアクティビティだけの書庫。R78）。位置を持つ項目は位置と同じく範囲に入る（D22-b / D22-d）。
/// **本文を消した時間帯に入る行は、同じまとまりで本文も消す**（ST23 / design D9（仮））。落ちたら全部巻き戻り、
/// 取り込み器は `read` の台帳の行を書かない —— 次の見回りが書庫を読み直して、この関数を呼び直す。
/// **滞在の作り直しを呼ばない・待たない**。錠と transaction は作り直しと同じ。
pub async fn mark_archive_arrivals(
    pool: &PgPool,
    user: uuid::Uuid,
    stored: &[crate::IngestRequest],
) -> sqlx::Result<()> {
    let Some((first, last)) = stored
        .iter()
        .filter(|r| {
            crate::archive::LOCATION_SOURCES.contains(&r.logical_source.as_str())
                || (is_item_source(&r.logical_source) && carries_location(&r.raw))
        })
        .map(|r| r.event_time)
        .fold(None, |span: Option<(DateTime<Utc>, DateTime<Utc>)>, t| {
            Some(span.map_or((t, t), |(lo, hi)| (lo.min(t), hi.max(t))))
        })
    else {
        return Ok(());
    };
    let mut tx = pool.begin().await?;
    lock(&mut tx, user).await?;
    let sources: Vec<String> = crate::archive::LOCATION_SOURCES
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    // `mark_late_arrivals` の範囲は終わりを含まない。DB の時刻はマイクロ秒に丸まるので、両端を 1 ms 広げる
    // （広げても見る行が増えるだけで、印を付ける条件は変わらない）。
    let margin = Duration::milliseconds(1);
    mark_late_arrivals(
        &mut tx,
        user,
        &sources,
        (first - margin, last + margin),
        true,
    )
    .await?;
    tx.commit().await
}

/// 本人が消した滞在の時間帯へ後から届いた、まだ印の無い基準ソースと書庫の位置の記録を隠す。
/// 位置（`locationInfos`）を持つ項目（マイアクティビティ・YouTube・Chrome）も対象（design D22-b / D22-d。持たない項目は外れる）。
///
/// 作り直しと同じ錠・transaction の中で、滞在の判定用の位置を読む前に呼ぶ。
/// すでに印のある行は候補にも更新にも含めないので、何度呼んでも台帳は一度だけ増える。
/// 消す transaction が見えた範囲内の位置にはすべて印を付けるため、ここで未印の行は
/// 消す側から未 commit で見えなかったか、その commit 後に届いた行である。
///
/// `purge_bodies` が真なら（書庫の印付け。design D9（仮））、本文を消した滞在（範囲の行がある。D14）と重なる行の本文も
/// 同じ transaction で消す。作り直しの拾い直しは印だけ（`false`）。印が `user:late` で本文がまだ残る行
/// （作り直しの拾い直しが先に印を付けた行）も、本文を消す側の候補に入れる。
async fn mark_late_arrivals(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    sources: &[String],
    (from, to): (DateTime<Utc>, DateTime<Utc>),
    purge_bodies: bool,
) -> sqlx::Result<()> {
    // 位置を持つ項目（マイアクティビティ・YouTube・Chrome）も同じ印の対象（design D22-b / D22-d。位置を持たない項目は下の条件で外れる）
    let mut sources = sources.to_vec();
    sources.extend(item_sources(&mut **tx).await?);
    let live = if purge_bodies {
        format!("(e.deleted_at IS NULL OR (e.deleted_by = '{USER_LATE}' AND e.raw <> ''))")
    } else {
        "e.deleted_at IS NULL".to_owned()
    };
    // 重なる滞在が複数あっても、本文を消す側を原因にする（`settle_arrival` と同じ順）
    let candidates: Vec<(uuid::Uuid, String, LateCause)> = sqlx::query(&format!(
        "SELECT DISTINCT ON (e.id) e.id AS event_id, e.logical_source,
                s.id, s.start_at, s.end_at, s.purged, s.operation_id
           FROM core.event e
           JOIN core.stay_erased_purge s
             ON s.user_id = e.user_id
            AND {erased}
            AND e.event_time <= s.end_at
            AND {end} >= s.start_at
          WHERE e.user_id = $1
            AND e.logical_source = ANY($2)
            AND e.event_time >= $3 AND e.event_time < $4
            AND {live}
            AND {located}
          ORDER BY e.id, s.purged DESC, s.start_at, s.id",
        erased = user_erased_stay_sql("s", 5),
        end = event_end_sql("e"),
        located = item_located_sql("e")
    ))
    .bind(user)
    .bind(&sources)
    .bind(from)
    .bind(to)
    .bind(format!("{REBUILD_PREFIX}%"))
    .try_map(|row: sqlx::postgres::PgRow| {
        use sqlx::Row;
        Ok((
            row.try_get("event_id")?,
            row.try_get("logical_source")?,
            sqlx::FromRow::from_row(&row)?,
        ))
    })
    .fetch_all(&mut **tx)
    .await?;

    for (event_id, logical_source, cause) in candidates {
        let marked = mark_late(tx, user, event_id, &logical_source, cause.stay_id).await?;
        if purge_bodies
            && purge_late_body(tx, user, event_id, &logical_source, &cause).await?
            && !marked
        {
            // すでに別の消した滞在の後着として隠れていた行を、本文を消した滞在の後着として消した。
            // その滞在を原因として台帳に足す —— 足さないと、本文の無いこの行が前の原因だけを持ち、
            // 前の原因の滞在を戻す口が「本文を消した記録は戻せない」で断り続ける（/code-review R24）
            ledger_late(tx, user, event_id, &logical_source, cause.stay_id).await?;
        }
    }
    Ok(())
}

/// 後着の 1 件の本文を、原因の滞在が本文を消した滞在（範囲の行がある）なら消す（`kind = 'late'`）。消したら `true`。
/// 範囲の行が無い本文を消した滞在（D14）は印だけ —— 消す側に倒すと、見せていない記録の本文が戻らない。
async fn purge_late_body(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    event_id: uuid::Uuid,
    logical_source: &str,
    cause: &LateCause,
) -> sqlx::Result<bool> {
    let Some(parent_id) = cause.operation_id.filter(|_| cause.purged) else {
        return Ok(false);
    };
    let mut counts = crate::deletion::PurgeCounts::default();
    if crate::archive::LOCATION_SOURCES.contains(&logical_source) {
        counts.archive_locations = 1;
    } else if is_item_source(logical_source) {
        counts.archive_items = 1;
    } else {
        counts.locations = 1;
    }
    crate::deletion::purge_late_arrival(
        tx,
        crate::deletion::LateArrival {
            user_id: user,
            event_id,
            logical_source,
            parent_id,
            stay_id: cause.stay_id,
            start: cause.start,
            end: cause.end,
            counts,
        },
    )
    .await?;
    Ok(true)
}

/// 1 件に `user:late` を付けて削除の台帳に 1 行（原因 = 消した滞在）。すでに隠れている行には何もしない。付けたら `true`。
async fn mark_late(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    event_id: uuid::Uuid,
    logical_source: &str,
    cause_event_id: uuid::Uuid,
) -> sqlx::Result<bool> {
    let changed = sqlx::query(
        "UPDATE core.event SET deleted_at = now(), deleted_by = $2
          WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(event_id)
    .bind(USER_LATE)
    .execute(&mut **tx)
    .await?;
    if changed.rows_affected() == 0 {
        return Ok(false);
    }
    ledger_late(tx, user, event_id, logical_source, cause_event_id).await?;
    Ok(true)
}

/// 削除の台帳に後着の 1 行（原因 = 消した滞在）。その原因の最後の行がすでに `erase` なら足さない。
async fn ledger_late(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    event_id: uuid::Uuid,
    logical_source: &str,
    cause_event_id: uuid::Uuid,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO core.deletion_ledger
           (event_id, user_id, logical_source, action, cause_event_id, mark, server_version)
         SELECT $1, $2, $3, 'erase', $4, $5, $6
          WHERE NOT EXISTS (
            SELECT 1 FROM core.deletion_ledger d
             WHERE d.event_id = $1 AND d.cause_event_id = $4 AND d.action = 'erase'
               AND d.seq = (SELECT max(l.seq) FROM core.deletion_ledger l
                             WHERE l.event_id = $1 AND l.cause_event_id = $4))",
    )
    .bind(event_id)
    .bind(user)
    .bind(logical_source)
    .bind(cause_event_id)
    .bind(USER_LATE)
    .bind(crate::BUILD_STAMP)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct LateCause {
    #[sqlx(rename = "id")]
    stay_id: uuid::Uuid,
    #[sqlx(rename = "start_at")]
    start: DateTime<Utc>,
    #[sqlx(rename = "end_at")]
    end: DateTime<Utc>,
    purged: bool,
    operation_id: Option<uuid::Uuid>,
}

/// 取り込みで新しく入った記録 1 件を、記録を入れたトランザクションの中（commit の前）で片付ける（ST23 / design D8（仮））。
///
/// 位置（基準のソース ∪ 書庫の位置）と、位置を持つ書庫の項目だけが対象で、それ以外は何もしない（錠も取らない）。
/// 錠を取ってから、消した滞在の時間帯（`core.stay_erased`。D5）に入るかを見る。入れば `user:late` を付けて削除の台帳に 1 行。
/// さらにその滞在が本文を消した滞在で、範囲の行があれば（無ければ D14 で印だけ）、同じまとまりで本文を消し、
/// `kind = 'late'` の操作の行と記録ごとの台帳の行を積む。`rebuild_day` の `mark_late_arrivals` は残す（取り込み以外の経路の拾い直し）。
pub(crate) async fn settle_arrival(
    tx: &mut Transaction<'_, Postgres>,
    stored: &crate::IngestRequest,
) -> sqlx::Result<()> {
    let source = stored.logical_source.as_str();
    let located = if is_item_source(source) {
        carries_location(&stored.raw)
    } else {
        let criteria = current_criteria(&mut **tx, stored.user_id)
            .await?
            .unwrap_or_else(Criteria::default_values);
        with_archive_sources(&criteria.sources)
            .iter()
            .any(|s| s == source)
    };
    if !located {
        return Ok(());
    }
    lock(tx, stored.user_id).await?;
    // 本文を消した滞在を先に選ぶ（重なる滞在が複数あっても、本文を消す側を原因にする）
    let cause: Option<LateCause> = sqlx::query_as(&format!(
        "SELECT s.id, s.start_at, s.end_at, s.purged, s.operation_id
               FROM core.event e
               JOIN core.stay_erased_purge s
                 ON s.user_id = e.user_id
                AND {erased}
                AND e.event_time <= s.end_at
                AND {end} >= s.start_at
              WHERE e.id = $1
                AND e.deleted_at IS NULL
                AND {located}
              ORDER BY s.purged DESC, s.start_at, s.id
              LIMIT 1",
        erased = user_erased_stay_sql("s", 2),
        end = event_end_sql("e"),
        located = item_located_sql("e")
    ))
    .bind(stored.id)
    .bind(format!("{REBUILD_PREFIX}%"))
    .fetch_optional(&mut **tx)
    .await?;
    let Some(cause) = cause else {
        return Ok(());
    };
    if !mark_late(tx, stored.user_id, stored.id, source, cause.stay_id).await? {
        return Ok(());
    }
    purge_late_body(tx, stored.user_id, stored.id, source, &cause).await?;
    Ok(())
}

/// 前の版を履歴へ積む（`core.event_version`）。`version_no` は続き番号 —— 錠の中なので衝突しない。
async fn push_version(tx: &mut Transaction<'_, Postgres>, id: uuid::Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash,
            raw, payload, source_updated_at, external_ref,
            tz_offset_min, tz_id, schema_version, unit_system, crs,
            collector_version, web_version, server_version)
         SELECT e.id, e.user_id, e.logical_source,
                coalesce((SELECT max(v.version_no) FROM core.event_version v WHERE v.event_id = e.id), 0) + 1,
                e.event_time, e.content_hash, e.raw, e.payload, e.source_updated_at, e.external_ref,
                e.tz_offset_min, e.tz_id, e.schema_version, e.unit_system, e.crs,
                e.collector_version, e.web_version, e.server_version
           FROM core.event e WHERE e.id = $1",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn payload_of(raw: &str) -> sqlx::Result<serde_json::Value> {
    serde_json::from_str(raw).map_err(|e| sqlx::Error::Decode(Box::new(e)))
}

/// **読んだときと削除の印が同じ行だけを書き換える**（R44）。
///
/// 作り直しの錠を取らない書き手（ST22 の「消す」）が、読んだ後に本人の削除を付けていたら、
/// そのまま書くと本人の削除が黙って取り消される。1 行も当たらなければ作り直しごと巻き戻す。
const SAME_MARK: &str = "(deleted_at IS NULL) = $10 AND deleted_by IS NOT DISTINCT FROM $11";

fn ensure_one(done: sqlx::postgres::PgQueryResult) -> anyhow::Result<()> {
    if done.rows_affected() == 1 {
        Ok(())
    } else {
        Err(MarkChanged.into())
    }
}

async fn update_stay(
    tx: &mut Transaction<'_, Postgres>,
    e: &Existing,
    f: &Stay,
    raw: &str,
    hide: bool,
) -> anyhow::Result<()> {
    let done = sqlx::query(&format!(
        "UPDATE core.event
            SET raw = $2, payload = $3, content_hash = $4, event_time = $5,
                tz_offset_min = $6, tz_id = $7,
                deleted_at = CASE WHEN $8 THEN coalesce(deleted_at, now()) END,
                deleted_by = CASE WHEN $8 THEN $9 END,
                server_version = CASE WHEN raw IS DISTINCT FROM $2 THEN $12 ELSE server_version END
          WHERE id = $1 AND {SAME_MARK}"
    ))
    .bind(e.id)
    .bind(raw)
    .bind(payload_of(raw)?)
    .bind(ingest::content_hash_of(stay::SOURCE, f.start, raw))
    .bind(f.start)
    .bind(f.tz_offset_min)
    .bind(&f.tz_id)
    .bind(hide)
    .bind(ERASED_RANGE)
    .bind(e.mark == Mark::Live)
    .bind(&e.deleted_by)
    .bind(crate::BUILD_STAMP)
    .execute(&mut **tx)
    .await?;
    ensure_one(done)
}

/// 新しい滞在を足す（design D1 の列の表）。**感度には何も書かない**（D9。列の既定に乗る）。
async fn insert_stay(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    id: uuid::Uuid,
    f: &Stay,
    raw: &str,
    hide: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, device_id, origin,
            event_time, tz_offset_min, tz_id, schema_version, unit_system, crs,
            content_hash, raw, payload, deleted_at, deleted_by, server_version)
         VALUES ($1, $2, $3, $4, NULL, 'derived', $5, $6, $7, 1, $8, $9, $10, $11, $12,
                 CASE WHEN $13 THEN now() END, CASE WHEN $13 THEN $14 END, $15)",
    )
    .bind(id)
    .bind(user)
    .bind(stay::SOURCE)
    .bind(id.to_string())
    .bind(f.start)
    .bind(f.tz_offset_min)
    .bind(&f.tz_id)
    .bind(ingest::DEFAULT_UNIT_SYSTEM)
    .bind(ingest::DEFAULT_CRS)
    .bind(ingest::content_hash_of(stay::SOURCE, f.start, raw))
    .bind(raw)
    .bind(payload_of(raw)?)
    .bind(hide)
    .bind(ERASED_RANGE)
    .bind(crate::BUILD_STAMP)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// 吸収する。**行は消さない**（D3）—— 印を付け、吸収の台帳に 1 行。
async fn absorb(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    e: &Existing,
    into: Option<uuid::Uuid>,
    criteria_id: i64,
) -> anyhow::Result<()> {
    let id = e.id;
    let done = sqlx::query(&format!(
        "UPDATE core.event SET deleted_at = now(), deleted_by = $2
          WHERE id = $1 AND {}",
        SAME_MARK.replace("$10", "$3").replace("$11", "$4")
    ))
    .bind(id)
    .bind(ABSORBED)
    .bind(e.mark == Mark::Live)
    .bind(&e.deleted_by)
    .execute(&mut **tx)
    .await?;
    ensure_one(done)?;
    sqlx::query(
        "INSERT INTO core.stay_absorbed (event_id, user_id, into_event_id, criteria_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(user)
    .bind(into)
    .bind(criteria_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// 全期間の作り直しの結果（`POST /stays/rebuild` の応答の材料）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllOutcome {
    pub criteria_id: i64,
    pub days: i64,
    pub stays_before: i64,
    pub stays_after: i64,
}

/// 読み出しに出ている滞在の件数。
pub async fn live_stay_count(pool: &PgPool, user: uuid::Uuid) -> sqlx::Result<i64> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM core.event_live
          WHERE logical_source = $1 AND origin = 'derived' AND user_id = $2",
    )
    .bind(stay::SOURCE)
    .bind(user)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// 利用者の全期間を作り直す（Q6 / C10）。**範囲は位置の記録と既存の滞在がある最初の日から最後の日まで**、
/// 日ごとにトランザクションと錠を取り直す（design D5 —— 自動の作り直しを長く待たせない）。
pub async fn rebuild_all(pool: &PgPool, user: uuid::Uuid) -> anyhow::Result<AllOutcome> {
    let c = ensure_criteria_committed(pool, user).await?;
    let stays_before = live_stay_count(pool, user).await?;
    let (first, last): (Option<DateTime<Utc>>, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT min(t), max(t) FROM (
           SELECT event_time AS t FROM core.event_live
            WHERE logical_source = ANY($1) AND user_id = $2
           UNION ALL
           SELECT event_time FROM core.event
            WHERE logical_source = $3 AND origin = 'derived' AND user_id = $2
         ) s",
    )
    .bind(&c.sources)
    .bind(user)
    .bind(stay::SOURCE)
    .fetch_one(pool)
    .await?;
    let mut days = 0;
    if let (Some(first), Some(last)) = (first, last) {
        let mut day = jst_date(first);
        while day <= jst_date(last) {
            if let Err(e) = rebuild_day(pool, user, day).await {
                // **どこまで済んだかを残す**（R42）。前の日は新しい基準、この日から後は古い基準のまま
                tracing::error!(
                    kind = "stays.rebuild_all",
                    %user,
                    %day,
                    days_done = days,
                    failure = %failure_kind(&e),
                    "全期間の作り直しが途中の日で止まった"
                );
                return Err(e);
            }
            days += 1;
            day += Duration::days(1);
        }
    }
    Ok(AllOutcome {
        criteria_id: c.id,
        days,
        stays_before,
        stays_after: live_stay_count(pool, user).await?,
    })
}

// ------------------------------------------------------------------ 1 日の並び（design D8）

/// 並びの 1 行の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    /// 滞在
    Stay,
    /// 滞在と滞在の間（位置の記録はある）
    Move,
    /// 位置の記録が無い時間（隣り合う記録の間隔が「記録が無い」とみなす間隔以上）
    NoRecord,
    /// 本人が消した滞在を含む時間
    Erased,
}

/// 並びの 1 行。**滞在は実際の始まりと終わり**（日をまたいでも切らない）、ほかはその日の中に切る。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct DayEntry {
    pub kind: EntryKind,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// 滞在の識別子（滞在の行だけ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    /// 滞在を作った基準の版（滞在の行だけ。取り込みの口から入った滞在には無いことがある）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria_id: Option<i64>,
    /// この「消した」区間から戻せる、本人が消した滞在の識別子
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stay_ids: Option<Vec<uuid::Uuid>>,
    /// 本文ごと消した区間（「消した」の行だけ。`stay_ids` は持たず、戻せない。design D12）
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub purged: bool,
    /// 本文ごと消すときに指定した滞在の識別子（本文ごと消した区間だけ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purge_stay_id: Option<uuid::Uuid>,
    /// 後から届いて本文を消した件数（本文ごと消した区間だけ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub late_purged: Option<u32>,
    /// 本文ごと消した区間にすっぽり入る消した滞在（本文ごと消した区間だけ。空なら出さない。CH08 design D6）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nested: Option<Vec<NestedEntry>>,
}

/// 本文ごと消した区間の入れ子の項目。別の区間にはならず、親の区間の下に添える。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct NestedEntry {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub stay_ids: Vec<uuid::Uuid>,
    /// 本文ごと消した滞在か（戻す口が無い）
    pub purged: bool,
}

impl DayEntry {
    fn plain(kind: EntryKind, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            kind,
            start,
            end,
            id: None,
            criteria_id: None,
            stay_ids: None,
            purged: false,
            purge_stay_id: None,
            late_purged: None,
            nested: None,
        }
    }
}

#[derive(Debug)]
struct ErasedSpan {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    stay_ids: Vec<uuid::Uuid>,
}

/// 入れ子の項目の親: 重なりのいちばん大きい本文ごと消した区間 → 始まりの早い区間（design D6 の 3）。
fn parent_of(parents: &[DayEntry], start: DateTime<Utc>, end: DateTime<Utc>) -> Option<usize> {
    parents
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let overlap = p.end.min(end) - p.start.max(start);
            (overlap > Duration::zero()).then_some((i, overlap, p.start))
        })
        .min_by_key(|(_, overlap, pstart)| (std::cmp::Reverse(*overlap), *pstart))
        .map(|(i, _, _)| i)
}

fn subtract_spans(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    erased: &[ErasedSpan],
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let mut fragments = Vec::new();
    let mut cursor = start;
    for span in erased {
        if span.end <= cursor || span.start >= end {
            continue;
        }
        if cursor < span.start {
            fragments.push((cursor, span.start.min(end)));
        }
        cursor = cursor.max(span.end);
        if cursor >= end {
            break;
        }
    }
    if cursor < end {
        fragments.push((cursor, end));
    }
    fragments
}

/// 並んだ滞在を作った基準（一覧の上に出す）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct CriteriaTag {
    pub criteria_id: i64,
    pub radius_m: i32,
    pub min_minutes: i32,
}

/// `GET /stays` の応答。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct DayView {
    pub date: NaiveDate,
    /// その日に並んだ滞在を作った基準。**新しい版から順に**、重複なし（design D8。先頭が一覧の上の基準）
    pub criteria: Vec<CriteriaTag>,
    /// 始まりの時刻順
    pub entries: Vec<DayEntry>,
}

/// その日（`Asia/Tokyo`）の滞在・移動・記録なし・消した区間を並べる（design D7 / D8 / R23）。`now` は差し替えられる。
///
/// - 滞在: その日と時間が重なる、読み出しに出ている `origin='derived'` の `s01-stay`
/// - 記録なし: 緯度経度を持つ位置の記録（精度を問わない）の間隔が `gap_minutes` 以上の区間。
///   **前後の日の記録も含めて測る**（日の頭と尻を、隣の日から続く記録で記録なしにしない）。
///   今日は、最後の位置から `now` までが `gap_minutes` 以上ならそこまでを記録なしにし、`now` より後は並べない
/// - 消した: 本人が消した滞在と、その時間帯で隠れた滞在の重なる・隣り合う区間をつないだもの
/// - 移動: 滞在にも記録なしにも入らない時間（今日は最後の位置まで）
pub async fn day_view(
    pool: &PgPool,
    user: uuid::Uuid,
    date: NaiveDate,
    now: DateTime<Utc>,
) -> sqlx::Result<DayView> {
    let (d0, d1) = day_bounds(date);
    let c = criteria_or_default(pool, user).await?;
    let gap = Duration::minutes(i64::from(c.gap_minutes));
    // 並べてよい上限。まだ起きていない時間は並べない
    let upper = d1.min(now).max(d0);

    let rows: Vec<ExistingRow> = sqlx::query_as(
        "SELECT id, event_time, raw, payload, deleted_at, deleted_by
           FROM core.event_live
          WHERE logical_source = $1 AND origin = 'derived' AND user_id = $2
            AND event_time < $3
            AND (event_time >= $4 OR payload->>'end' >= $5)
          ORDER BY event_time, id",
    )
    .bind(stay::SOURCE)
    .bind(user)
    .bind(d1)
    .bind(d0 - Duration::days(2))
    .bind((d0 - Duration::days(2)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    .fetch_all(pool)
    .await?;

    let mut entries = Vec::new();
    let mut tags: Vec<CriteriaTag> = Vec::new();
    for r in rows {
        let (start, end) = span_of(r.event_time, &r.payload);
        if !(start < d1 && (end > d0 || start >= d0)) {
            continue;
        }
        let tag = r.payload.get("criteria").and_then(|v| {
            Some(CriteriaTag {
                criteria_id: v.get("id")?.as_i64()?,
                radius_m: i32::try_from(v.get("radius_m")?.as_i64()?).ok()?,
                min_minutes: i32::try_from(v.get("min_minutes")?.as_i64()?).ok()?,
            })
        });
        if let Some(tag) = &tag {
            if !tags.iter().any(|t| t.criteria_id == tag.criteria_id) {
                tags.push(tag.clone());
            }
        }
        entries.push(DayEntry {
            id: Some(r.id),
            criteria_id: tag.map(|t| t.criteria_id),
            ..DayEntry::plain(EntryKind::Stay, start, end)
        });
    }
    tags.sort_by_key(|t| std::cmp::Reverse(t.criteria_id));

    // **本人が消した滞在と、消した時間帯で隠した滞在を「消した」の行にする**（design D7）。
    // 吸収された滞在は吸収先が同じ時間を持つので数えない。読み出しは座標を持たない専用ビュー越しに限る（D12）。
    // **本文ごと消した滞在は別の行にする**（design D12）。指定した滞在の時間（D13）をそのまま持ち、
    // 本文を消していない「消した」とはまとめず、「消した」の区間はその分を差し引く。
    #[derive(sqlx::FromRow)]
    struct HiddenRow {
        id: uuid::Uuid,
        start_at: DateTime<Utc>,
        end_at: DateTime<Utc>,
        deleted_by: Option<String>,
        purged: bool,
        operation_id: Option<uuid::Uuid>,
    }
    let hidden_rows: Vec<HiddenRow> = sqlx::query_as(
        "SELECT id, start_at, end_at, deleted_by, purged, operation_id FROM core.stay_erased_purge
          WHERE user_id = $1 AND deleted_by IS DISTINCT FROM $4
            AND start_at < $2 AND end_at > $3
          ORDER BY start_at, id",
    )
    .bind(user)
    .bind(d1)
    .bind(d0)
    .bind(ABSORBED)
    .fetch_all(pool)
    .await?;
    // 後から本文を消した件数を、本文を消した滞在の操作ごとに 1 回の問い合わせで数える（/code-review R27）
    let purged_ops: Vec<uuid::Uuid> = hidden_rows
        .iter()
        .filter(|r| r.purged)
        .filter_map(|r| r.operation_id)
        .collect();
    let late_by_op: HashMap<uuid::Uuid, i64> = if purged_ops.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_as::<_, (uuid::Uuid, i64)>(
            "SELECT parent_id, sum(stays + locations + archive_locations + archive_items)::bigint
               FROM core.erasure_operation
              WHERE kind = 'late' AND parent_id = ANY($1)
              GROUP BY parent_id",
        )
        .bind(&purged_ops)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect()
    };
    // 操作の順（本文ごと消す口は最初の書き込みの前に利用者の錠を取るので、`txid` の順が消した順）
    let op_rank: HashMap<uuid::Uuid, usize> = if purged_ops.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT id FROM core.erasure_operation WHERE id = ANY($1) ORDER BY txid, id",
        )
        .bind(&purged_ops)
        .fetch_all(pool)
        .await?
        .into_iter()
        .enumerate()
        .map(|(rank, id)| (id, rank))
        .collect()
    };
    let late_of = |row: &HiddenRow| -> u32 {
        let late = row
            .operation_id
            .and_then(|op| late_by_op.get(&op).copied())
            .unwrap_or(0);
        u32::try_from(late).unwrap_or(u32::MAX)
    };

    // 本文ごと消した滞在のうち、ほかの本文ごと消した区間の和に覆われるものは区間にせず入れ子にする（D6 の 2）。
    // 長いものから（同じ長さなら先に消した方から）区間に残し、覆われたものを入れ子にする。
    // 時間が同じなら先に消した方が親になり、互いに覆い合っても親が 1 つ残る。
    let mut purged_rows: Vec<(&HiddenRow, DateTime<Utc>, DateTime<Utc>)> = hidden_rows
        .iter()
        .filter(|r| r.purged)
        .map(|r| (r, r.start_at.max(d0), r.end_at.min(upper)))
        .filter(|(_, s, e)| s < e)
        .collect();
    purged_rows.sort_by_key(|(r, s, e)| {
        (
            std::cmp::Reverse(*e - *s),
            r.operation_id.and_then(|op| op_rank.get(&op).copied()),
            r.id,
        )
    });
    let mut purged_spans: Vec<ErasedSpan> = Vec::new();
    let mut parents: Vec<DayEntry> = Vec::new();
    let mut nested_purged: Vec<(&HiddenRow, DateTime<Utc>, DateTime<Utc>)> = Vec::new();
    for (row, start, end) in purged_rows {
        if subtract_spans(start, end, &purged_spans).is_empty() {
            nested_purged.push((row, start, end));
            continue;
        }
        parents.push(DayEntry {
            purged: true,
            purge_stay_id: Some(row.id),
            late_purged: Some(late_of(row)),
            ..DayEntry::plain(EntryKind::Erased, start, end)
        });
        purged_spans.push(ErasedSpan {
            start,
            end,
            stay_ids: Vec::new(),
        });
        purged_spans.sort_by_key(|s| s.start);
    }
    // 親は重なりの大きい区間 → 始まりの早い区間（D6 の 3）
    for (row, start, end) in nested_purged {
        if let Some(i) = parent_of(&parents, start, end) {
            let parent = &mut parents[i];
            parent.late_purged = Some(parent.late_purged.unwrap_or(0).saturating_add(late_of(row)));
            parent
                .nested
                .get_or_insert_with(Vec::new)
                .push(NestedEntry {
                    start,
                    end,
                    stay_ids: vec![row.id],
                    purged: true,
                });
        }
    }

    let mut fragments: Vec<ErasedSpan> = Vec::new();
    // 差し引いて空になる、本文を消していない滞在（日ごとに判定する。D6 の 1）
    let mut covered_erased: Vec<ErasedSpan> = Vec::new();
    for row in hidden_rows.into_iter().filter(|r| !r.purged) {
        let HiddenRow {
            id,
            start_at: start,
            end_at: end,
            deleted_by,
            ..
        } = row;
        let start = start.max(d0);
        let end = end.min(upper);
        if start >= end {
            continue;
        }
        let mut causes = if deleted_by.as_deref() == Some(ABSORBED) {
            Vec::new()
        } else if deleted_by
            .as_deref()
            .is_some_and(|by| by.starts_with(REBUILD_PREFIX))
        {
            sqlx::query_scalar(
                "SELECT id FROM core.stay_erased_purge
                   WHERE user_id = $1 AND deleted_by IS DISTINCT FROM $2
                     AND start_at < $4 AND end_at > $3
                     AND NOT purged
                     AND (deleted_by IS NULL OR deleted_by NOT LIKE $5)
                   ORDER BY start_at, id",
            )
            .bind(user)
            .bind(ABSORBED)
            .bind(start)
            .bind(end)
            .bind(format!("{REBUILD_PREFIX}%"))
            .fetch_all(pool)
            .await?
        } else {
            vec![id]
        };
        causes.sort_unstable();
        causes.dedup();
        let left = subtract_spans(start, end, &purged_spans);
        if left.is_empty() {
            covered_erased.push(ErasedSpan {
                start,
                end,
                stay_ids: causes.clone(),
            });
        }
        for (start, end) in left {
            fragments.push(ErasedSpan {
                start,
                end,
                stay_ids: causes.clone(),
            });
        }
    }
    // 親ごとに、始まりの順に、重なるか隣り合う本文を消していない滞在を 1 つにまとめる（D6 の 4）
    covered_erased.sort_by_key(|f| f.start);
    let mut grouped: Vec<(usize, ErasedSpan)> = Vec::new();
    for span in covered_erased {
        let Some(parent) = parent_of(&parents, span.start, span.end) else {
            continue;
        };
        if let Some((_, last)) = grouped
            .iter_mut()
            .rev()
            .find(|(p, _)| *p == parent)
            .filter(|(_, last)| span.start <= last.end)
        {
            last.end = last.end.max(span.end);
            last.stay_ids.extend(span.stay_ids);
            last.stay_ids.sort_unstable();
            last.stay_ids.dedup();
        } else {
            grouped.push((parent, span));
        }
    }
    for (parent, span) in grouped {
        parents[parent]
            .nested
            .get_or_insert_with(Vec::new)
            .push(NestedEntry {
                start: span.start,
                end: span.end,
                stay_ids: span.stay_ids,
                purged: false,
            });
    }
    for nested in parents.iter_mut().filter_map(|p| p.nested.as_mut()) {
        nested.sort_by_key(|n| (n.start, n.end));
    }
    entries.extend(parents);
    fragments.sort_by_key(|f| f.start);
    let mut hidden: Vec<ErasedSpan> = Vec::new();
    for span in fragments {
        if let Some(last) = hidden.last_mut().filter(|last| span.start <= last.end) {
            last.end = last.end.max(span.end);
            last.stay_ids.extend(span.stay_ids);
            last.stay_ids.sort_unstable();
            last.stay_ids.dedup();
        } else {
            hidden.push(span);
        }
    }
    entries.extend(hidden.iter().map(|span| DayEntry {
        stay_ids: Some(span.stay_ids.clone()),
        ..DayEntry::plain(EntryKind::Erased, span.start, span.end)
    }));
    // 記録なしを測るときは、本文ごと消した時間も消した時間と同じに「記録なし」にしない
    hidden.extend(purged_spans);
    hidden.sort_by_key(|s| s.start);

    // 記録なし。読む窓の端を「その外側に記録がある」とみなさない仮の点にして、頭と尻も同じ規則で測る
    let observed: Vec<DateTime<Utc>> = load_points(pool, user, &c.sources, d0 - gap, d1 + gap)
        .await?
        .into_iter()
        .filter(|p| p.lat.is_some() && p.lon.is_some())
        .map(|p| p.at)
        .collect();
    let today = now < d1;
    let tail = if today { now } else { d1 + gap };
    let mut edges = Vec::with_capacity(observed.len() + 2);
    edges.push(d0 - gap);
    edges.extend(observed.iter().copied().filter(|t| *t <= tail));
    edges.push(tail);
    let mut covered: Vec<(DateTime<Utc>, DateTime<Utc>)> = entries
        .iter()
        .map(|e| (e.start.max(d0), e.end.min(upper)))
        .collect();
    for w in edges.windows(2) {
        let (a, b) = (w[0], w[1]);
        for (s, e) in subtract_spans(a, b, &hidden)
            .into_iter()
            .filter(|(s, e)| *e - *s >= gap)
        {
            let (s, e) = (s.max(d0), e.min(upper));
            if s >= e {
                continue;
            }
            entries.push(DayEntry::plain(EntryKind::NoRecord, s, e));
            covered.push((s, e));
        }
    }

    // 移動。今日は最後の位置までしか並べない（その先は、まだ記録なしと決まっていない時間）
    let move_upper = if today {
        observed
            .iter()
            .copied()
            .filter(|t| *t <= now)
            .max()
            .map_or(d0, |last| last.max(d0))
            .max(covered.iter().map(|c| c.1).max().unwrap_or(d0))
            .min(upper)
    } else {
        upper
    };
    covered.sort();
    let mut cursor = d0;
    for (s, e) in covered {
        if s > cursor && cursor < move_upper {
            entries.push(DayEntry::plain(EntryKind::Move, cursor, s.min(move_upper)));
        }
        cursor = cursor.max(e);
    }
    if cursor < move_upper {
        entries.push(DayEntry::plain(EntryKind::Move, cursor, move_upper));
    }

    entries.sort_by(|a, b| a.start.cmp(&b.start).then(a.kind.cmp(&b.kind)));
    Ok(DayView {
        date,
        criteria: tags,
        entries,
    })
}

/// テスト用: 「読み出しに出ている」と読んだ滞在を吸収する（R44）。
#[cfg(test)]
pub(crate) async fn absorb_for_test(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    id: uuid::Uuid,
    into: Option<uuid::Uuid>,
) -> anyhow::Result<()> {
    let c = ensure_criteria(tx, user).await?;
    let read_as_live = Existing {
        id,
        start: Utc::now(),
        end: Utc::now(),
        lat: None,
        lon: None,
        raw: String::new(),
        mark: Mark::Live,
        deleted_by: None,
        operation_id: None,
    };
    absorb(tx, user, &read_as_live, into, c.id).await
}
