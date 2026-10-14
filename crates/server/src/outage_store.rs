// SPDX-License-Identifier: AGPL-3.0-only
//! 途切れの判定の材料を引く SQL（ST14 / design D1・D4）。判定そのものは `outage::judge`。
//!
//! 読むのは `core.event`（削除済みも含む。削除は観測の事実を消さない）・`core.heartbeat`・
//! `core.coverage_span`（停止）。**破棄の報告（`core.drop_report`）は材料に入れない**（C8）。
use crate::outage::{SourceInput, NON_OBSERVATION_ALERT};
use chrono::{DateTime, NaiveDate, Utc};

/// `NOT ((logical_source, kind) IN (…))` の括弧の中。定数から組み立てる（値は定数だけなので埋め込んでよい）。
fn non_observation_sql() -> String {
    NON_OBSERVATION_ALERT
        .iter()
        .map(|(s, k)| format!("('{s}', '{k}')"))
        .collect::<Vec<_>>()
        .join(", ")
}

type Row = (
    String,
    String,
    i32,
    bool,
    bool,
    bool,
    Option<NaiveDate>,
    Option<NaiveDate>,
    bool,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

/// 端末のソース（登録簿のうち、書庫でもサーバ自身の生存信号でもないもの）の材料を引く。
///
/// - 最後の記録: 出来事の時刻の新しい順に後ろから読み、観測ではない種類を飛ばす。
///   **未来の時刻**（`event_time > now`）の記録は受け取った時刻（`ingest_time`）で数える
/// - 生存信号: 取得できる状態のものだけ。未来の `emitted_at` は `received_at`
/// - 収集開始日: 引き継ぎの鎖の根から継ぐ（鎖は `UNION` で重複を落として、輪になっても止まる）
/// - 引き継いだソースは、行ができた時刻を最後に生きていた時刻の下限にする
pub async fn inputs(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    now: DateTime<Utc>,
) -> Result<Vec<SourceInput>, sqlx::Error> {
    let non_obs = non_observation_sql();
    let sql = format!(
        "WITH RECURSIVE anc(name, cur) AS (
             SELECT logical_source, logical_source FROM core.source
             UNION
             SELECT a.name, p.succeeds
               FROM anc a JOIN core.source p ON p.logical_source = a.cur
              WHERE p.succeeds IS NOT NULL),
         started AS (
             SELECT a.name, min(s.collection_started_on) AS started_on
               FROM anc a JOIN core.source s ON s.logical_source = a.cur
              GROUP BY a.name)
         SELECT s.logical_source, s.display_name, s.expected_gap_sec, s.outage_target,
                EXISTS (SELECT 1 FROM core.event e
                         WHERE e.user_id = $1 AND e.logical_source = s.logical_source
                           AND e.origin = 'collected') AS has_collected,
                EXISTS (SELECT 1 FROM core.event e
                         WHERE e.user_id = $1 AND e.logical_source = s.logical_source
                           AND e.origin <> 'collected') AS has_non_collected,
                s.retired_on,
                st.started_on,
                EXISTS (SELECT 1 FROM core.coverage_span c
                         WHERE c.user_id = $1 AND c.logical_source = s.logical_source
                           AND c.kind = 'stopped' AND c.started_at <= $2
                           AND (c.ended_at IS NULL OR c.ended_at > $2)) AS stopped_now,
                (SELECT max(c.ended_at) FROM core.coverage_span c
                  WHERE c.user_id = $1 AND c.logical_source = s.logical_source
                    AND c.kind = 'stopped' AND c.ended_at <= $2) AS last_stop_end,
                greatest(
                  (SELECT e.event_time FROM core.event e
                    WHERE e.user_id = $1 AND e.logical_source = s.logical_source
                      AND e.event_time <= $2
                      AND NOT ((e.logical_source, coalesce(e.payload->>'kind', '')) IN ({non_obs}))
                    ORDER BY e.event_time DESC LIMIT 1),
                  (SELECT max(e.ingest_time) FROM core.event e
                    WHERE e.user_id = $1 AND e.logical_source = s.logical_source
                      AND e.event_time > $2
                      AND NOT ((e.logical_source, coalesce(e.payload->>'kind', '')) IN ({non_obs})))
                ) AS last_record_at,
                greatest(
                  (SELECT h.emitted_at FROM core.heartbeat h
                    WHERE h.user_id = $1 AND h.logical_source = s.logical_source
                      AND h.capturable AND h.emitted_at <= $2
                    ORDER BY h.emitted_at DESC LIMIT 1),
                  (SELECT max(h.received_at) FROM core.heartbeat h
                    WHERE h.user_id = $1 AND h.logical_source = s.logical_source
                      AND h.capturable AND h.emitted_at > $2)
                ) AS last_capturable_heartbeat_at,
                CASE WHEN s.succeeds IS NOT NULL THEN s.registered_at END AS alive_floor
           FROM core.source s
           JOIN started st ON st.name = s.logical_source
          -- 書庫（日数を持つ）とサーバ自身の生存信号は別の判定（Task 6）
          WHERE s.alert_after_days IS NULL AND s.logical_source <> 's01-archive-inbox'
          ORDER BY s.logical_source"
    );
    let rows: Vec<Row> = sqlx::query_as(&sql)
        .bind(user)
        .bind(now)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                logical_source,
                display_name,
                expected_gap_sec,
                target,
                has_collected,
                has_non_collected,
                retired_on,
                collection_started_on,
                stopped_now,
                last_stop_end,
                last_record_at,
                last_capturable_heartbeat_at,
                alive_floor,
            )| SourceInput {
                logical_source,
                display_name,
                expected_gap_sec,
                target,
                has_collected,
                has_non_collected,
                retired_on,
                collection_started_on,
                stopped_now,
                last_stop_end,
                last_record_at,
                last_capturable_heartbeat_at,
                alive_floor,
            },
        )
        .collect())
}
