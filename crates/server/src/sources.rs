// SPDX-License-Identifier: AGPL-3.0-only
//! `GET /sources` —— 登録簿の取り方・契機・取得の間隔と、書庫の通知までの残り日数（ST13 / design D10）。
//!
//! 最後の書庫の導き方は途切れの判定と同じ式（`outage_store::LAST_ARCHIVE_AT_SQL`）。
//! 鳴る時刻 = 最後の書庫の時刻 + 登録簿の日数。途切れの判定の「超える」と同じ境目（ちょうどは過ぎていない）。
use crate::coverage::DAY_TZ;
use crate::outage_store::LAST_ARCHIVE_AT_SQL;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SourceRow {
    pub logical_source: String,
    pub display_name: String,
    /// `device` / `pc` / `archive` / `server` / `scheduled`
    pub acquisition: String,
    /// `interval` / `on_change` / `authored` / `derived` / `once` / `manual`
    pub fetch_trigger: String,
    pub fetch_interval_sec: Option<i32>,
    pub retired_on: Option<NaiveDate>,
    /// 書庫のソースだけ。最後の書庫からこの日数で通知が鳴る
    pub alert_after_days: Option<i32>,
    /// 最後の書庫の作られた日（`Asia/Tokyo`）。退役した行・書庫の無い行は `null`
    pub last_archive_on: Option<NaiveDate>,
    /// 通知までの残り日数（切り上げ）。通知の日を過ぎた・書庫が無い・退役した行は `null`（負の値は出さない）
    pub alert_days_left: Option<i64>,
    pub alert_passed: bool,
    /// 書庫のソースで、書庫も記録も無い
    pub no_archive: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DeviceNotice {
    /// `c01-location` の最新の生存信号の原文の `unused_app_restrictions`（無ければ `null`）
    pub unused_app_restrictions: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SourcesResponse {
    pub sources: Vec<SourceRow>,
    pub device_notice: DeviceNotice,
}

/// 通知までの状態。`alert_passed` は `now - last > days`（途切れの判定と同じ「超える」）。
#[derive(Debug)]
pub struct Alert {
    pub days_left: Option<i64>,
    pub passed: bool,
}

pub fn alert_of(last_archive_at: DateTime<Utc>, days: i32, now: DateTime<Utc>) -> Alert {
    let remaining = last_archive_at + Duration::days(i64::from(days)) - now;
    if remaining < Duration::zero() {
        return Alert {
            days_left: None,
            passed: true,
        };
    }
    let day_ms = Duration::days(1).num_milliseconds();
    Alert {
        days_left: Some((remaining.num_milliseconds() + day_ms - 1) / day_ms),
        passed: false,
    }
}

type Row = (
    String,
    String,
    String,
    String,
    Option<i32>,
    Option<NaiveDate>,
    Option<i32>,
    Option<DateTime<Utc>>,
    Option<NaiveDate>,
);

pub async fn read(
    db: &sqlx::PgPool,
    user: uuid::Uuid,
    now: DateTime<Utc>,
) -> Result<SourcesResponse, sqlx::Error> {
    let sql = format!(
        "SELECT s.logical_source, s.display_name, s.acquisition, s.fetch_trigger, s.fetch_interval_sec,
                s.retired_on, s.alert_after_days, {LAST_ARCHIVE_AT_SQL} AS last_archive_at,
                ({LAST_ARCHIVE_AT_SQL} AT TIME ZONE '{DAY_TZ}')::date AS last_archive_on
           FROM core.source s ORDER BY s.logical_source"
    );
    let rows: Vec<Row> = sqlx::query_as(&sql).bind(user).fetch_all(db).await?;
    let sources = rows
        .into_iter()
        .map(
            |(
                logical_source,
                display_name,
                acquisition,
                fetch_trigger,
                fetch_interval_sec,
                retired_on,
                alert_after_days,
                last,
                last_on,
            )| {
                let live_days = alert_after_days.filter(|_| retired_on.is_none());
                let alert = live_days.zip(last).map(|(d, l)| alert_of(l, d, now));
                SourceRow {
                    logical_source,
                    display_name,
                    acquisition,
                    fetch_trigger,
                    fetch_interval_sec,
                    retired_on,
                    alert_after_days,
                    last_archive_on: live_days.and(last_on),
                    alert_passed: alert.as_ref().is_some_and(|a| a.passed),
                    alert_days_left: alert.and_then(|a| a.days_left),
                    no_archive: live_days.is_some() && last.is_none(),
                }
            },
        )
        .collect();
    // 原文は text の素通しで JSON とは限らない。SQL で cast せず Rust で読み、読めなければ無しにする
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT raw FROM core.heartbeat
          WHERE user_id = $1 AND logical_source = 'c01-location'
          ORDER BY emitted_at DESC LIMIT 1",
    )
    .bind(user)
    .fetch_optional(db)
    .await?;
    Ok(SourcesResponse {
        sources,
        device_notice: DeviceNotice {
            unused_app_restrictions: raw
                .and_then(|r| serde_json::from_str::<serde_json::Value>(&r).ok())
                .and_then(|v| v.get("unused_app_restrictions")?.as_str().map(str::to_owned)),
        },
    })
}
