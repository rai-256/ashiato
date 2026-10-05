// SPDX-License-Identifier: AGPL-3.0-only
//! 滞在を消すときの判定と SQL（ST22 / design D2〜D5）。

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashSet;

use crate::{stay::Criteria, stay_store};

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
                ) AS blocked_by_other_cause
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
    let (points, intervals) = stay_store::split_by_span(&sources);
    let overlaps = stay_store::overlaps_erased_sql("core.event", "$2", "$5", "$3", "$4");
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
          WHERE user_id = $1 AND {overlaps} AND deleted_at IS NULL
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
    let already_deleted: Vec<ActiveDeletionRow> = sqlx::query_as(&format!(
        "SELECT e.id AS event_id, e.logical_source, e.deleted_by AS mark
           FROM core.event e
          WHERE e.user_id = $1 AND {overlaps_e}
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
