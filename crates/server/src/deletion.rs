// SPDX-License-Identifier: AGPL-3.0-only
//! 滞在を消すときの判定と SQL（ST22 / design D2〜D5）。

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashSet;

use crate::{archive, stay, stay::Criteria, stay_store};

/// 消す操作が成立しなかった理由。404 に畳む 2 種類は、ログの要否だけが違う。
#[derive(Debug)]
pub enum EraseError {
    NotFound,
    UserMismatch,
    Database(sqlx::Error),
}

/// 戻す操作が成立しなかった理由。存在と利用者の不一致はどちらも API では 404 に畳む。
#[derive(Debug)]
pub enum RestoreError {
    NotFound,
    UserMismatch,
    /// 戻すことになる記録に、本文を消したものがある（design D6。何も書かずに断る）。
    Purged,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for RestoreError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<sqlx::Error> for EraseError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

#[derive(sqlx::FromRow)]
struct StayRow {
    user_id: uuid::Uuid,
    event_time: DateTime<Utc>,
    payload: serde_json::Value,
    deleted_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct RestoreStayRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    event_time: DateTime<Utc>,
    payload: serde_json::Value,
}

#[derive(sqlx::FromRow)]
struct LedgerRow {
    event_id: uuid::Uuid,
    user_id: uuid::Uuid,
    logical_source: String,
    cause_event_id: uuid::Uuid,
    mark: String,
    blocked_by_other_cause: bool,
    purged: bool,
}

#[derive(sqlx::FromRow)]
struct ActiveDeletionRow {
    event_id: uuid::Uuid,
    logical_source: String,
    mark: String,
}

/// commit 後の作り直しに必要な範囲と、応答に出す件数。
#[derive(Debug)]
pub struct EraseOutcome {
    pub user_id: uuid::Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    pub stays: u64,
    pub locations: u64,
}

impl EraseOutcome {
    /// 滞在が触れる `Asia/Tokyo` の日を、古い順に一度ずつ返す。
    pub fn days(&self) -> impl Iterator<Item = NaiveDate> {
        let first = stay_store::jst_date(self.start);
        let last = stay_store::jst_date(self.end);
        first.iter_days().take_while(move |day| *day <= last)
    }
}

/// 戻した件数と、commit 後に作り直す利用者・日。
#[derive(Debug)]
pub struct RestoreOutcome {
    pub stays: u64,
    pub locations: u64,
    pub rebuild_days: Vec<(uuid::Uuid, NaiveDate)>,
}

/// 要求内の滞在を原因とする最新の `erase` だけを、一つの transaction で戻す。
pub async fn restore(
    pool: &PgPool,
    stay_ids: &[uuid::Uuid],
    requested_user: Option<uuid::Uuid>,
) -> Result<RestoreOutcome, RestoreError> {
    let requested: HashSet<_> = stay_ids.iter().copied().collect();
    if requested.is_empty() {
        return Err(RestoreError::NotFound);
    }

    let mut tx = pool.begin().await?;
    // 利用者は行から取る。最初の SQL で、決定的な順に対象利用者の錠を取る。
    let stays: Vec<RestoreStayRow> = sqlx::query_as(
        "SELECT id, user_id, event_time, payload,
                pg_advisory_xact_lock($2, hashtext(user_id::text)) AS locked
           FROM core.event
          WHERE id = ANY($1) AND logical_source = 's01-stay' AND origin = 'derived'
          ORDER BY user_id, id
          FOR UPDATE",
    )
    .bind(stay_ids)
    .bind(stay_store::LOCK_KEY)
    .fetch_all(&mut *tx)
    .await?;
    if stays.len() != requested.len() {
        return Err(RestoreError::NotFound);
    }
    if requested_user.is_some_and(|user| stays.iter().any(|stay| stay.user_id != user)) {
        return Err(RestoreError::UserMismatch);
    }

    let rows: Vec<LedgerRow> = sqlx::query_as(
        "SELECT d.event_id, d.user_id, d.logical_source, d.cause_event_id, d.mark,
                EXISTS (
                  SELECT 1
                    FROM core.deletion_ledger other
                   WHERE other.event_id = d.event_id
                     AND other.action = 'erase'
                     AND other.seq = (
                       SELECT max(latest.seq) FROM core.deletion_ledger latest
                        WHERE latest.event_id = other.event_id
                          AND latest.cause_event_id = other.cause_event_id
                     )
                     AND other.cause_event_id <> ALL($1)
                ) AS blocked_by_other_cause,
                (e.raw = '') AS purged
           FROM core.deletion_ledger d
           JOIN core.event e ON e.id = d.event_id AND e.user_id = d.user_id
          WHERE d.cause_event_id = ANY($1)
            AND d.action = 'erase'
            AND d.seq = (
              SELECT max(last.seq) FROM core.deletion_ledger last
               WHERE last.event_id = d.event_id
                 AND last.cause_event_id = d.cause_event_id
            )
            AND e.deleted_at IS NOT NULL
            AND e.deleted_by IS NOT DISTINCT FROM d.mark
          ORDER BY d.event_id",
    )
    .bind(stay_ids)
    .fetch_all(&mut *tx)
    .await?;
    // 本文を消した記録は戻せない。別の原因で消えたまま残る記録は戻さないので数えない（design D6（仮））。
    if rows
        .iter()
        .any(|row| row.purged && !row.blocked_by_other_cause)
    {
        return Err(RestoreError::Purged);
    }
    let mut restored_causes = HashSet::new();
    let mut restored_stays = 0;
    let mut restored_locations = 0;
    for row in rows {
        ledger(
            &mut tx,
            row.event_id,
            row.user_id,
            &row.logical_source,
            "restore",
            row.cause_event_id,
            &row.mark,
        )
        .await?;
        restored_causes.insert(row.cause_event_id);
        if row.blocked_by_other_cause {
            continue;
        }
        let changed = sqlx::query(
            "UPDATE core.event SET deleted_at = NULL, deleted_by = NULL
              WHERE id = $1 AND deleted_at IS NOT NULL
                AND deleted_by IS NOT DISTINCT FROM $2",
        )
        .bind(row.event_id)
        .bind(&row.mark)
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() == 0 {
            continue;
        }
        if row.logical_source == crate::stay::SOURCE {
            restored_stays += 1;
        } else {
            restored_locations += 1;
        }
    }
    tx.commit().await?;

    let mut rebuild_days = Vec::new();
    for stay in stays {
        if !restored_causes.contains(&stay.id) {
            continue;
        }
        let (start, end) = stay_store::span_of(stay.event_time, &stay.payload);
        let first = stay_store::jst_date(start);
        let last = stay_store::jst_date(end);
        for day in first.iter_days().take_while(|day| *day <= last) {
            if !rebuild_days.contains(&(stay.user_id, day)) {
                rebuild_days.push((stay.user_id, day));
            }
        }
    }

    Ok(RestoreOutcome {
        stays: restored_stays,
        locations: restored_locations,
        rebuild_days,
    })
}

/// 滞在と連鎖対象の位置に印を付け、同じ transaction の台帳へ追記する。
pub async fn erase(
    pool: &PgPool,
    stay_id: uuid::Uuid,
    requested_user: Option<uuid::Uuid>,
) -> Result<EraseOutcome, EraseError> {
    erase_using_action(pool, stay_id, requested_user, "erase").await
}

/// 台帳の CHECK 違反を起こし、印と台帳が同時に rollback されることを試す口。
#[cfg(test)]
pub(crate) async fn erase_with_action(
    pool: &PgPool,
    stay_id: uuid::Uuid,
    requested_user: Option<uuid::Uuid>,
    action: &str,
) -> Result<EraseOutcome, EraseError> {
    erase_using_action(pool, stay_id, requested_user, action).await
}

async fn erase_using_action(
    pool: &PgPool,
    stay_id: uuid::Uuid,
    requested_user: Option<uuid::Uuid>,
    action: &str,
) -> Result<EraseOutcome, EraseError> {
    let mut tx = pool.begin().await?;
    // 利用者は行からしか分からないため、最初の SQL で行を読むのと同時にその利用者の錠を取る。
    let Some(identity) = stay_row_and_lock(&mut tx, stay_id).await? else {
        return Err(EraseError::NotFound);
    };
    if requested_user.is_some_and(|user| user != identity.user_id) {
        return Err(EraseError::UserMismatch);
    }
    let row = stay_row(&mut tx, stay_id, true)
        .await?
        .ok_or(EraseError::NotFound)?;
    let (start, end) = stay_store::span_of(row.event_time, &row.payload);
    if row.deleted_at.is_some() {
        tx.commit().await?;
        return Ok(EraseOutcome {
            user_id: row.user_id,
            start,
            end,
            stays: 0,
            locations: 0,
        });
    }

    let sources = stay_store::current_criteria(&mut *tx, row.user_id)
        .await?
        .unwrap_or_else(Criteria::default_values)
        .sources;
    // 基準のソース ∪ 書庫の位置の論理ソース（design D22）。滞在の判定の入力は変えない。
    let sources = stay_store::with_archive_sources(&sources);
    // 点で見るソースと区間で見るソースに分け、点の側は始まりの時刻の上下限で索引を効かせる（R73）。
    let (mut points, intervals) = stay_store::split_by_span(&sources);
    // 位置を持つ項目（マイアクティビティ・YouTube・Chrome）も連鎖の対象（design D22-b / D22-d。持たない項目は `located` で外れる）
    points.extend(stay_store::item_sources(&mut *tx).await?);
    let overlaps = stay_store::overlaps_erased_sql("core.event", "$2", "$5", "$3", "$4");
    let located = stay_store::item_located_sql("core.event");
    let stay_changed: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "UPDATE core.event SET deleted_at = now(), deleted_by = 'user'
          WHERE id = $1 AND deleted_at IS NULL
          RETURNING id, logical_source",
    )
    .bind(stay_id)
    .fetch_all(&mut *tx)
    .await?;
    let locations: Vec<(uuid::Uuid, String)> = sqlx::query_as(&format!(
        "UPDATE core.event SET deleted_at = now(), deleted_by = 'user:cascade'
          WHERE user_id = $1 AND {overlaps} AND {located} AND deleted_at IS NULL
          RETURNING id, logical_source",
    ))
    .bind(row.user_id)
    .bind(&points)
    .bind(start)
    .bind(end)
    .bind(&intervals)
    .fetch_all(&mut *tx)
    .await?;

    // 既に別の消去原因で隠れている位置にも、この操作の原因を追記する。
    // A を戻したとき、重なる B の消去まで戻さないために必要な因果関係である。
    // 新たに消した行はまだ台帳に載せていないため、この検索には含まれない。
    let overlaps_e = stay_store::overlaps_erased_sql("e", "$2", "$5", "$3", "$4");
    let located_e = stay_store::item_located_sql("e");
    let already_deleted: Vec<ActiveDeletionRow> = sqlx::query_as(&format!(
        "SELECT e.id AS event_id, e.logical_source, e.deleted_by AS mark
           FROM core.event e
          WHERE e.user_id = $1 AND {overlaps_e} AND {located_e}
            AND e.deleted_at IS NOT NULL
            AND EXISTS (
              SELECT 1 FROM core.deletion_ledger d
               WHERE d.event_id = e.id AND d.action = 'erase'
                 AND e.deleted_by IS NOT DISTINCT FROM d.mark
                 AND d.seq = (SELECT max(last.seq) FROM core.deletion_ledger last WHERE last.event_id = e.id)
            )"
    ))
    .bind(row.user_id)
    .bind(&points)
    .bind(start)
    .bind(end)
    .bind(&intervals)
    .fetch_all(&mut *tx)
    .await?;

    for (event_id, logical_source) in &stay_changed {
        ledger(
            &mut tx,
            *event_id,
            row.user_id,
            logical_source,
            action,
            stay_id,
            "user",
        )
        .await?;
    }
    for (event_id, logical_source) in &locations {
        ledger(
            &mut tx,
            *event_id,
            row.user_id,
            logical_source,
            action,
            stay_id,
            "user:cascade",
        )
        .await?;
    }
    for row in &already_deleted {
        ledger(
            &mut tx,
            row.event_id,
            identity.user_id,
            &row.logical_source,
            action,
            stay_id,
            &row.mark,
        )
        .await?;
    }
    tx.commit().await?;

    Ok(EraseOutcome {
        user_id: row.user_id,
        start,
        end,
        stays: stay_changed.len() as u64,
        locations: locations.len() as u64,
    })
}

async fn stay_row_and_lock(
    tx: &mut Transaction<'_, Postgres>,
    stay_id: uuid::Uuid,
) -> sqlx::Result<Option<StayRow>> {
    sqlx::query_as(
        "SELECT user_id, event_time, payload, deleted_at,
                pg_advisory_xact_lock($2, hashtext(user_id::text)) AS locked
           FROM core.event
          WHERE id = $1 AND logical_source = 's01-stay' AND origin = 'derived'",
    )
    .bind(stay_id)
    .bind(stay_store::LOCK_KEY)
    .fetch_optional(&mut **tx)
    .await
}

async fn stay_row(
    tx: &mut Transaction<'_, Postgres>,
    stay_id: uuid::Uuid,
    for_update: bool,
) -> sqlx::Result<Option<StayRow>> {
    let suffix = if for_update { " FOR UPDATE" } else { "" };
    sqlx::query_as(&format!(
        "SELECT user_id, event_time, payload, deleted_at
           FROM core.event
          WHERE id = $1 AND logical_source = 's01-stay' AND origin = 'derived'{suffix}"
    ))
    .bind(stay_id)
    .fetch_optional(&mut **tx)
    .await
}

#[allow(clippy::too_many_arguments)]
async fn ledger(
    tx: &mut Transaction<'_, Postgres>,
    event_id: uuid::Uuid,
    user_id: uuid::Uuid,
    logical_source: &str,
    action: &str,
    cause_event_id: uuid::Uuid,
    mark: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO core.deletion_ledger
           (event_id, user_id, logical_source, action, cause_event_id, mark)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(event_id)
    .bind(user_id)
    .bind(logical_source)
    .bind(action)
    .bind(cause_event_id)
    .bind(mark)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

// ------------------------------------------------------------------ 本文ごと消す（ST23 / design D2 / D3 / D6 / D15 / D17）

/// 本文ごと消す範囲を組ごとに数えたもの（design D15）。確認で見せる数と台帳に残す数は同じ。
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
)]
pub struct PurgeCounts {
    pub stays: u32,
    pub locations: u32,
    pub archive_locations: u32,
    pub archive_items: u32,
    pub versions: u32,
}

/// 本文ごと消す操作が成立しなかった理由。
#[derive(Debug)]
pub enum PurgeError {
    NotFound,
    UserMismatch,
    /// 指定した滞在が「消した」になっていない（消えていない・作り直しの印だけ）。
    NotErased,
    /// 確認で見せた件数と、数え直した件数が違う。
    CountsChanged(PurgeCounts),
    /// どの組（design D15）にも入らない論理ソースが範囲に入った。
    UnknownSource,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for PurgeError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

/// 本文ごと消した結果。すでに消していた滞在は件数 0・`operation_id` なし。
#[derive(Debug)]
pub struct PurgeOutcome {
    pub user_id: uuid::Uuid,
    pub counts: PurgeCounts,
    pub operation_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Stay,
    Location,
    ArchiveLocation,
    ArchiveItem,
}

/// 範囲に入った記録 1 件。`span` は滞在だけが持つ（消す前の本文から読んだ時間）。
#[derive(Debug)]
struct Member {
    id: uuid::Uuid,
    logical_source: String,
    group: Group,
    span: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// 一緒に本文を消す重なる滞在（印を `user:purged` に付け替える。design D6）
    swallowed: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct VersionRow {
    event_id: uuid::Uuid,
    version_no: i32,
    event_time: DateTime<Utc>,
    payload: serde_json::Value,
}

/// `purge_scope` が集めた範囲。確認の口（何も書かない）と消す口が同じものを使う。
#[derive(Debug)]
pub struct PurgeScope {
    pub stay_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    /// 指定した滞在の時間（design D13）
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// すでに本文を消した滞在。範囲は空・件数は 0。
    pub already_purged: bool,
    pub counts: PurgeCounts,
    members: Vec<Member>,
    versions: Vec<VersionRow>,
}

#[derive(sqlx::FromRow)]
struct ScopeStayRow {
    user_id: uuid::Uuid,
    event_time: DateTime<Utc>,
    raw: String,
    payload: serde_json::Value,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<String>,
    erased: bool,
}

#[derive(sqlx::FromRow)]
struct ScopeEventRow {
    id: uuid::Uuid,
    logical_source: String,
    event_time: DateTime<Utc>,
    payload: serde_json::Value,
}

/// 本文ごと消す範囲を集めて、組ごとに数える（design D2 / D15）。**書かない。** 錠は呼び出し側が取る。
///
/// 1. 指定した滞在（本人が「消した」にしたもの） 2. それを原因として消えた記録（削除の台帳の原因で引く）
/// 3. 時間が重なる滞在のうち作り直しの印のもの（端が触れるだけは入れない。本人が別に消したものと印の無いものは入れない）
/// 4. 1〜3 の前の版すべて。どの組にも入らない論理ソースが混ざっていたら `UnknownSource`。
pub async fn purge_scope(
    tx: &mut Transaction<'_, Postgres>,
    stay_id: uuid::Uuid,
) -> Result<PurgeScope, PurgeError> {
    let stay: ScopeStayRow = sqlx::query_as(
        "SELECT e.user_id, e.event_time, e.raw, e.payload, e.deleted_at, e.deleted_by,
                EXISTS (
                  SELECT 1 FROM core.deletion_ledger d
                   WHERE d.event_id = e.id AND d.action = 'erase'
                     AND d.seq = (SELECT max(last.seq) FROM core.deletion_ledger last WHERE last.event_id = e.id)
                ) AS erased
           FROM core.event e
          WHERE e.id = $1 AND e.logical_source = 's01-stay' AND e.origin = 'derived'",
    )
    .bind(stay_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(PurgeError::NotFound)?;
    let (start, end) = stay_store::span_of(stay.event_time, &stay.payload);
    let mut scope = PurgeScope {
        stay_id,
        user_id: stay.user_id,
        start,
        end,
        already_purged: stay.raw.is_empty(),
        counts: PurgeCounts::default(),
        members: Vec::new(),
        versions: Vec::new(),
    };
    if scope.already_purged {
        return Ok(scope);
    }
    if stay.deleted_at.is_none() || stay.deleted_by.as_deref() != Some("user") || !stay.erased {
        return Err(PurgeError::NotErased);
    }

    scope.members.push(Member {
        id: stay_id,
        logical_source: stay::SOURCE.to_owned(),
        group: Group::Stay,
        span: Some((start, end)),
        swallowed: false,
    });

    // 2: この滞在を原因として消えた記録（原因ごとの最新の行が erase で、いまも同じ印で消えているもの）
    let caused: Vec<ScopeEventRow> = sqlx::query_as(
        "SELECT e.id, e.logical_source, e.event_time, e.payload
           FROM core.event e
          WHERE e.user_id = $1 AND e.id <> $2 AND e.raw <> '' AND e.deleted_at IS NOT NULL
            AND EXISTS (
              SELECT 1 FROM core.deletion_ledger d
               WHERE d.event_id = e.id AND d.cause_event_id = $2 AND d.action = 'erase'
                 AND e.deleted_by IS NOT DISTINCT FROM d.mark
                 AND d.seq = (
                   SELECT max(last.seq) FROM core.deletion_ledger last
                    WHERE last.event_id = e.id AND last.cause_event_id = d.cause_event_id
                 )
            )
          ORDER BY e.id",
    )
    .bind(scope.user_id)
    .bind(stay_id)
    .fetch_all(&mut **tx)
    .await?;

    let base_sources: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT s FROM core.stay_criteria c, unnest(c.sources) AS s WHERE c.user_id = $1",
    )
    .bind(scope.user_id)
    .fetch_all(&mut **tx)
    .await?;
    let item_sources = stay_store::item_sources(&mut **tx).await?;
    for row in caused {
        let group = if row.logical_source == stay::SOURCE {
            Group::Stay
        } else if archive::LOCATION_SOURCES.contains(&row.logical_source.as_str()) {
            Group::ArchiveLocation
        } else if item_sources.contains(&row.logical_source) {
            Group::ArchiveItem
        } else if row.logical_source == "c01-location" || base_sources.contains(&row.logical_source)
        {
            Group::Location
        } else {
            // ログに出すのは論理ソースの名前だけ
            tracing::warn!(
                kind = "purge_unknown_source",
                logical_source = %row.logical_source,
                "どの組にも入らない論理ソースが本文ごと消す範囲に入ったため断った"
            );
            return Err(PurgeError::UnknownSource);
        };
        let span =
            (group == Group::Stay).then(|| stay_store::span_of(row.event_time, &row.payload));
        scope.members.push(Member {
            id: row.id,
            logical_source: row.logical_source,
            group,
            span,
            swallowed: false,
        });
    }

    // 3: 時間が重なる（端が触れるだけは含めない）作り直しの印の滞在。時間は `core.stay_erased` が持つ 1 か所の読み方
    let swallowed: Vec<ScopeEventRow> = sqlx::query_as(
        "SELECT e.id, e.logical_source, e.event_time, e.payload
           FROM core.stay_erased v
           JOIN core.event e ON e.id = v.id
          WHERE v.user_id = $1 AND v.id <> $2 AND NOT v.purged
            AND v.deleted_by IN ($3, $4)
            AND v.start_at < $6 AND $5 < v.end_at
          ORDER BY e.id",
    )
    .bind(scope.user_id)
    .bind(stay_id)
    .bind(stay_store::ABSORBED)
    .bind(stay_store::ERASED_RANGE)
    .bind(start)
    .bind(end)
    .fetch_all(&mut **tx)
    .await?;
    for row in swallowed {
        if scope.members.iter().any(|m| m.id == row.id) {
            continue;
        }
        let span = stay_store::span_of(row.event_time, &row.payload);
        scope.members.push(Member {
            id: row.id,
            logical_source: row.logical_source,
            group: Group::Stay,
            span: Some(span),
            swallowed: true,
        });
    }

    // 4: 前の版すべて
    let ids: Vec<uuid::Uuid> = scope.members.iter().map(|m| m.id).collect();
    scope.versions = sqlx::query_as(
        "SELECT event_id, version_no, event_time, payload
           FROM core.event_version
          WHERE event_id = ANY($1) AND raw <> ''
          ORDER BY event_id, version_no",
    )
    .bind(&ids)
    .fetch_all(&mut **tx)
    .await?;

    let count = |group| scope.members.iter().filter(|m| m.group == group).count() as u32;
    scope.counts = PurgeCounts {
        stays: count(Group::Stay),
        locations: count(Group::Location),
        archive_locations: count(Group::ArchiveLocation),
        archive_items: count(Group::ArchiveItem),
        versions: scope.versions.len() as u32,
    };
    Ok(scope)
}

/// 本文ごと消す前の確認。範囲を消す口と同じ `purge_scope` で数え、何も書かずに返す（design D1）。
pub async fn purge_preview(
    pool: &PgPool,
    stay_id: uuid::Uuid,
    requested_user: Option<uuid::Uuid>,
) -> Result<PurgeScope, PurgeError> {
    let mut tx = pool.begin().await?;
    // 消す口と同じ順で断る: 滞在でない・無い → 404、利用者違い → 404、その後に 409
    let owner: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM core.event
          WHERE id = $1 AND logical_source = 's01-stay' AND origin = 'derived'",
    )
    .bind(stay_id)
    .fetch_optional(&mut *tx)
    .await?;
    let owner = owner.ok_or(PurgeError::NotFound)?;
    if requested_user.is_some_and(|user| user != owner) {
        return Err(PurgeError::UserMismatch);
    }
    let scope = purge_scope(&mut tx, stay_id).await?;
    // 読むだけ。巻き戻して何も残さない。
    tx.rollback().await?;
    Ok(scope)
}

/// 消した滞在を、その場面の記録と前の版ごと本文を消す（design D3 の順。1 つの transaction）。
///
/// すでに本文を消した滞在は `expected` を比べずに件数 0 で返す（2 度押しは成功）。
/// 確認で見せた `expected` と数え直した件数が 1 つでも違えば、何も書かずに `CountsChanged`。
pub async fn purge(
    pool: &PgPool,
    stay_id: uuid::Uuid,
    requested_user: Option<uuid::Uuid>,
    expected: PurgeCounts,
) -> Result<PurgeOutcome, PurgeError> {
    purge_using_span_scope(pool, stay_id, requested_user, expected, "event").await
}

/// 範囲の行（台帳の最後の書き込み）の CHECK 違反を起こし、本文が空になった後でも全部 rollback されることを試す口。
#[cfg(test)]
pub(crate) async fn purge_with_span_scope(
    pool: &PgPool,
    stay_id: uuid::Uuid,
    requested_user: Option<uuid::Uuid>,
    expected: PurgeCounts,
    span_scope: &str,
) -> Result<PurgeOutcome, PurgeError> {
    purge_using_span_scope(pool, stay_id, requested_user, expected, span_scope).await
}

async fn purge_using_span_scope(
    pool: &PgPool,
    stay_id: uuid::Uuid,
    requested_user: Option<uuid::Uuid>,
    expected: PurgeCounts,
    span_scope: &str,
) -> Result<PurgeOutcome, PurgeError> {
    let mut tx = pool.begin().await?;
    // 1: 錠 → 行を FOR UPDATE → 範囲を集めて数える
    let Some(identity) = stay_row_and_lock(&mut tx, stay_id).await? else {
        return Err(PurgeError::NotFound);
    };
    if requested_user.is_some_and(|user| user != identity.user_id) {
        return Err(PurgeError::UserMismatch);
    }
    sqlx::query("SELECT 1 FROM core.event WHERE id = $1 FOR UPDATE")
        .bind(stay_id)
        .execute(&mut *tx)
        .await?;
    let scope = purge_scope(&mut tx, stay_id).await?;
    if scope.already_purged {
        tx.commit().await?;
        return Ok(PurgeOutcome {
            user_id: scope.user_id,
            counts: PurgeCounts::default(),
            operation_id: None,
        });
    }
    if scope.counts != expected {
        return Err(PurgeError::CountsChanged(scope.counts));
    }

    // 2: 消す前の本文から時間を控える（滞在は範囲の中の `span`・前の版は `scope.versions`）
    let user_id = scope.user_id;
    let ids: Vec<uuid::Uuid> = scope.members.iter().map(|m| m.id).collect();

    // 3: 操作の台帳に 1 行
    let operation_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.erasure_operation
           (id, user_id, kind, stay_id, start_at, end_at,
            stays, locations, archive_locations, archive_items, versions)
         VALUES ($1, $2, 'user', $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(operation_id)
    .bind(user_id)
    .bind(stay_id)
    .bind(scope.start)
    .bind(scope.end)
    .bind(scope.counts.stays as i32)
    .bind(scope.counts.locations as i32)
    .bind(scope.counts.archive_locations as i32)
    .bind(scope.counts.archive_items as i32)
    .bind(scope.counts.versions as i32)
    .execute(&mut *tx)
    .await?;

    // 4: 記録ごと・前の版ごとの消去の台帳（門が同じまとまりの行を見る）
    let sources: Vec<&str> = scope
        .members
        .iter()
        .map(|m| m.logical_source.as_str())
        .collect();
    ledger_erasures(&mut tx, operation_id, user_id, "event", &ids, &sources).await?;
    let version_events: Vec<uuid::Uuid> = scope.versions.iter().map(|v| v.event_id).collect();
    let version_sources: Vec<&str> = scope
        .versions
        .iter()
        .filter_map(|v| {
            scope
                .members
                .iter()
                .find(|m| m.id == v.event_id)
                .map(|m| m.logical_source.as_str())
        })
        .collect();
    ledger_erasures(
        &mut tx,
        operation_id,
        user_id,
        "version",
        &version_events,
        &version_sources,
    )
    .await?;

    // 5: 前の版 → 記録の順に消去の形へ（本文だけ。時刻・内容の鍵・削除の印は触らない）
    erase_bodies(&mut tx, &ids).await?;

    // 6: 一緒に本文を消した滞在の印を本人の「本文ごと消した」に付け替える（作り直しが印を外さないように）
    for member in scope.members.iter().filter(|m| m.swallowed) {
        sqlx::query(
            "UPDATE core.event SET deleted_by = 'user:purged'
              WHERE id = $1 AND deleted_at IS NOT NULL",
        )
        .bind(member.id)
        .execute(&mut *tx)
        .await?;
        ledger(
            &mut tx,
            member.id,
            user_id,
            &member.logical_source,
            "erase",
            stay_id,
            "user:purged",
        )
        .await?;
    }

    // 7: 本文を消した滞在と前の版の時間
    let mut span_events = Vec::new();
    let mut span_scopes: Vec<&str> = Vec::new();
    let mut span_versions: Vec<Option<i32>> = Vec::new();
    let mut span_starts = Vec::new();
    let mut span_ends = Vec::new();
    for member in &scope.members {
        if let Some((from, to)) = member.span {
            span_events.push(member.id);
            span_scopes.push(span_scope);
            span_versions.push(None);
            span_starts.push(from);
            span_ends.push(to);
        }
    }
    for version in &scope.versions {
        let of_stay = scope
            .members
            .iter()
            .any(|m| m.id == version.event_id && m.group == Group::Stay);
        if of_stay {
            let (from, to) = stay_store::span_of(version.event_time, &version.payload);
            span_events.push(version.event_id);
            span_scopes.push("version");
            span_versions.push(Some(version.version_no));
            span_starts.push(from);
            span_ends.push(to);
        }
    }
    sqlx::query(
        "INSERT INTO core.erasure_span (operation_id, event_id, scope, version_no, start_at, end_at)
         SELECT $1, * FROM unnest($2::uuid[], $3::text[], $4::int[], $5::timestamptz[], $6::timestamptz[])",
    )
    .bind(operation_id)
    .bind(&span_events)
    .bind(&span_scopes)
    .bind(&span_versions)
    .bind(&span_starts)
    .bind(&span_ends)
    .execute(&mut *tx)
    .await?;

    // 8: commit の時に門（派生・収集した記録・前の版）が台帳の行と前の版の全消去を確かめる
    tx.commit().await?;
    Ok(PurgeOutcome {
        user_id,
        counts: scope.counts,
        operation_id: Some(operation_id),
    })
}

/// 消去の台帳に、記録（`scope = 'event'`）か前の版（`'version'`。`event_id` は親）ごとの行を足す。
async fn ledger_erasures(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: uuid::Uuid,
    user_id: uuid::Uuid,
    scope: &str,
    event_ids: &[uuid::Uuid],
    sources: &[&str],
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO core.erasure_ledger
           (event_id, user_id, logical_source, scope, erased_by, reason, operation_id)
         SELECT e, $1, s, $2, 'user', 'stay-purge', $3
           FROM unnest($4::uuid[], $5::text[]) AS t(e, s)",
    )
    .bind(user_id)
    .bind(scope)
    .bind(operation_id)
    .bind(event_ids)
    .bind(sources)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// 記録の本文（原文と解析済み）を消去の形にする、**唯一の場所**。前の版 → 記録の順
/// （時刻・内容の鍵・削除の印は触らない。前の版が無い記録では前の版の更新は 0 行）。
async fn erase_bodies(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[uuid::Uuid],
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE core.event_version SET raw = '', payload = '{}'::jsonb
          WHERE event_id = ANY($1) AND raw <> ''",
    )
    .bind(ids)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}'::jsonb WHERE id = ANY($1)")
        .bind(ids)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// 本文ごと消した時間帯に後から届いた記録 1 件（`stay_store::settle_arrival` が渡す）。
pub(crate) struct LateArrival<'a> {
    pub user_id: uuid::Uuid,
    pub event_id: uuid::Uuid,
    pub logical_source: &'a str,
    /// 元の操作（`core.erasure_operation.id`）
    pub parent_id: uuid::Uuid,
    /// 時間帯を持つ、本文を消した滞在
    pub stay_id: uuid::Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// この 1 件が数えられる組（1 件分）
    pub counts: PurgeCounts,
}

/// 後から届いた 1 件の本文を、呼び出し側のトランザクションの中で消す（design D8（仮）。`kind = 'late'`）。
///
/// 操作の行（元の操作へ `parent_id` で結ぶ。元の行は書き換えない）→ 記録ごとの台帳の行 → 本文、の順
/// （門が commit の時に台帳の行を見る）。前の版は無い（いま入ったばかりの記録）。
pub(crate) async fn purge_late_arrival(
    tx: &mut Transaction<'_, Postgres>,
    late: LateArrival<'_>,
) -> sqlx::Result<()> {
    let operation_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.erasure_operation
           (id, user_id, kind, parent_id, stay_id, start_at, end_at,
            stays, locations, archive_locations, archive_items, versions)
         VALUES ($1, $2, 'late', $3, $4, $5, $6, 0, $7, $8, $9, 0)",
    )
    .bind(operation_id)
    .bind(late.user_id)
    .bind(late.parent_id)
    .bind(late.stay_id)
    .bind(late.start)
    .bind(late.end)
    .bind(late.counts.locations as i32)
    .bind(late.counts.archive_locations as i32)
    .bind(late.counts.archive_items as i32)
    .execute(&mut **tx)
    .await?;
    ledger_erasures(
        tx,
        operation_id,
        late.user_id,
        "event",
        &[late.event_id],
        &[late.logical_source],
    )
    .await?;
    erase_bodies(tx, &[late.event_id]).await?;
    Ok(())
}
