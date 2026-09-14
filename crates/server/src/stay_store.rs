// SPDX-License-Identifier: AGPL-3.0-only
//! 滞在の作り直しと、基準の台帳（ST16 / FR-31 / FR-50 / FR-76。design D3 / D4 / D5 / D10）。
//!
//! 判定そのものは `stay`（DB に触らない）。ここが持つのは、**何を読んで判定に渡し、
//! 結果をどの行へどう書くか** —— 識別子の引き継ぎ（D3）と、本人が消した時間帯（D4）と、錠と範囲（D5）。
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};

use crate::ingest;
use crate::stay::{self, Criteria, Point, Stay};

/// 作り直しの錠の名前空間（`pg_advisory_xact_lock(key, hashtext(user))` の 1 つ目）。
/// `testdb` のマイグレーションの錠（4820251。1 引数の形）とは別の空間。
const LOCK_KEY: i32 = 4_816_016;

/// 作り直しが付ける削除の印。**これ以外（`NULL` を含む）はすべて本人が消したもの**（design D4 / R14）。
pub const REBUILD_PREFIX: &str = "rebuild:";
/// どの新しい滞在にも割り当てられなかった（design D3）。
pub const ABSORBED: &str = "rebuild:absorbed";
/// 本人が消した時間帯と重なった（design D4）。
pub const ERASED_RANGE: &str = "rebuild:erased-range";

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
        "INSERT INTO core.stay_criteria (user_id, radius_m, min_minutes, gap_minutes, sources)
         VALUES ($1, $2, $3, $4, $5) RETURNING {CRITERIA_COLUMNS}"
    ))
    .bind(user)
    .bind(c.radius_m)
    .bind(c.min_minutes)
    .bind(c.gap_minutes)
    .bind(&c.sources)
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
    fn of(deleted_at: Option<DateTime<Utc>>, deleted_by: Option<&str>) -> Self {
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

/// `[from, to)` と時間が重なる（閉区間で触れるものを含む）既存の滞在。**候補は `origin='derived'` の `s01-stay` だけ**（D2 / D3）。
///
/// 滞在の長さに上限は無いので、始まりだけでは絞れない。`payload->>'end'` の文字列比較で粗く絞ってから
/// 読んだ終わりで正確に絞る（2 日の余裕を取るので、秒の端数の書き方の違いは効かない）。
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
    Ok(rows
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
            })
        })
        .collect())
}

// ------------------------------------------------------------------ 範囲（design D5）

/// 求められた日から作り直しの範囲を広げる（design D5 / R15）。**変わらなくなるまで繰り返す。**
///
/// 1. 範囲の端から `gap_minutes` 以内にかかる、読み出しに出ている（または消した時間帯で隠れている）既存の滞在を含むまで広げる
/// 2. 範囲の端をまたいで続いている集まり（最短のとどまりに満たないものも含む）を含むまで広げる
///
/// 2 で見る集まりは、範囲の前後を広めに読んで判定する。**広げた端では集まりが切れている**ので、
/// 最後に `[lo, hi)` だけで判定しても、全期間で判定したときと同じ滞在になる。
async fn settle_range(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    c: &Criteria,
    (mut lo, mut hi): (DateTime<Utc>, DateTime<Utc>),
) -> anyhow::Result<(DateTime<Utc>, DateTime<Utc>)> {
    let gap = Duration::minutes(i64::from(c.gap_minutes));
    let tick = Duration::microseconds(1);
    // 読む幅は範囲の長さに合わせて伸ばす —— 1 日ずつだと、何日も続くとどまりで回数が日数に比例する
    for _ in 0..64 {
        let (mut nlo, mut nhi) = (lo, hi);
        for e in load_existing(tx, user, lo - gap, hi + gap).await? {
            if matches!(e.mark, Mark::Live | Mark::ErasedRange) {
                nlo = nlo.min(e.start);
                nhi = nhi.max(e.end + tick);
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
    anyhow::bail!("作り直しの範囲が 64 回で定まらない")
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
    let mut tx = pool.begin().await?;
    lock(&mut tx, user).await?;
    let c = ensure_criteria(&mut tx, user).await?;
    let (lo, hi) = settle_range(&mut tx, user, &c, day_bounds(day)).await?;

    let points = load_points(&mut *tx, user, &c.sources, lo, hi).await?;
    let fresh = stay::detect(&points, &c);
    let existing = load_existing(&mut tx, user, lo, hi).await?;
    let erased: Vec<_> = existing
        .iter()
        .filter(|e| e.mark == Mark::UserDeleted)
        .map(|e| (e.start, e.end))
        .collect();
    let plan = assign(&fresh, &existing, c.radius_m);

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
                    push_version(&mut tx, e.id).await?;
                }
                update_stay(&mut tx, e.id, f, &raw, hide).await?;
                out.updated += 1;
            }
            None => {
                let id = uuid::Uuid::new_v4();
                ids.push(id);
                insert_stay(&mut tx, user, id, f, &raw, hide).await?;
                out.created += 1;
            }
        }
    }
    for (j, into) in &plan.absorbed {
        absorb(&mut tx, user, existing[*j].id, into.map(|i| ids[i]), c.id).await?;
        out.absorbed += 1;
    }
    tx.commit().await?;
    Ok(out)
}

/// 前の版を履歴へ積む（`core.event_version`）。`version_no` は続き番号 —— 錠の中なので衝突しない。
async fn push_version(tx: &mut Transaction<'_, Postgres>, id: uuid::Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash,
            raw, payload, source_updated_at, external_ref,
            tz_offset_min, tz_id, schema_version, unit_system, crs)
         SELECT e.id, e.user_id, e.logical_source,
                coalesce((SELECT max(v.version_no) FROM core.event_version v WHERE v.event_id = e.id), 0) + 1,
                e.event_time, e.content_hash, e.raw, e.payload, e.source_updated_at, e.external_ref,
                e.tz_offset_min, e.tz_id, e.schema_version, e.unit_system, e.crs
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

async fn update_stay(
    tx: &mut Transaction<'_, Postgres>,
    id: uuid::Uuid,
    f: &Stay,
    raw: &str,
    hide: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE core.event
            SET raw = $2, payload = $3, content_hash = $4, event_time = $5,
                tz_offset_min = $6, tz_id = $7,
                deleted_at = CASE WHEN $8 THEN coalesce(deleted_at, now()) END,
                deleted_by = CASE WHEN $8 THEN $9 END
          WHERE id = $1",
    )
    .bind(id)
    .bind(raw)
    .bind(payload_of(raw)?)
    .bind(ingest::content_hash_of(stay::SOURCE, f.start, raw))
    .bind(f.start)
    .bind(f.tz_offset_min)
    .bind(&f.tz_id)
    .bind(hide)
    .bind(ERASED_RANGE)
    .execute(&mut **tx)
    .await?;
    Ok(())
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
            content_hash, raw, payload, deleted_at, deleted_by)
         VALUES ($1, $2, $3, $4, NULL, 'derived', $5, $6, $7, 1, $8, $9, $10, $11, $12,
                 CASE WHEN $13 THEN now() END, CASE WHEN $13 THEN $14 END)",
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
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// 吸収する。**行は消さない**（D3）—— 印を付け、吸収の台帳に 1 行。
async fn absorb(
    tx: &mut Transaction<'_, Postgres>,
    user: uuid::Uuid,
    id: uuid::Uuid,
    into: Option<uuid::Uuid>,
    criteria_id: i64,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = $2 WHERE id = $1")
        .bind(id)
        .bind(ABSORBED)
        .execute(&mut **tx)
        .await?;
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
            rebuild_day(pool, user, day).await?;
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
}

/// 並びの 1 行。**滞在は実際の始まりと終わり**（日をまたいでも切らない）、移動と記録なしはその日の中に切る。
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

/// その日（`Asia/Tokyo`）の滞在・移動・記録なしを並べる（design D8 / R23）。`now` は差し替えられる。
///
/// - 滞在: その日と時間が重なる、読み出しに出ている `origin='derived'` の `s01-stay`
/// - 記録なし: 緯度経度を持つ位置の記録（精度を問わない）の間隔が `gap_minutes` 以上の区間。
///   **前後の日の記録も含めて測る**（日の頭と尻を、隣の日から続く記録で記録なしにしない）。
///   今日は、最後の位置から `now` までが `gap_minutes` 以上ならそこまでを記録なしにし、`now` より後は並べない
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
            kind: EntryKind::Stay,
            start,
            end,
            id: Some(r.id),
            criteria_id: tag.map(|t| t.criteria_id),
        });
    }
    tags.sort_by_key(|t| std::cmp::Reverse(t.criteria_id));

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
        if b - a < gap {
            continue;
        }
        let (s, e) = (a.max(d0), b.min(upper));
        if s < e {
            entries.push(DayEntry {
                kind: EntryKind::NoRecord,
                start: s,
                end: e,
                id: None,
                criteria_id: None,
            });
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
            entries.push(DayEntry {
                kind: EntryKind::Move,
                start: cursor,
                end: s.min(move_upper),
                id: None,
                criteria_id: None,
            });
        }
        cursor = cursor.max(e);
    }
    if cursor < move_upper {
        entries.push(DayEntry {
            kind: EntryKind::Move,
            start: cursor,
            end: move_upper,
            id: None,
            criteria_id: None,
        });
    }

    entries.sort_by(|a, b| a.start.cmp(&b.start).then(a.kind.cmp(&b.kind)));
    Ok(DayView {
        date,
        criteria: tags,
        entries,
    })
}
