// SPDX-License-Identifier: AGPL-3.0-only
//! 途切れの判定の材料を引く SQL と台帳（ST14 / design D1・D4・D7）。判定そのものは `outage::judge`。
//!
//! 読むのは `core.event`（削除済みも含む。削除は観測の事実を消さない）・`core.heartbeat`・
//! `core.coverage_span`（停止）。**破棄の報告（`core.drop_report`）は材料に入れない**（C8）。
use crate::outage::{judge, Outage, SourceInput, SourceKind, NON_OBSERVATION_ALERT};
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
    Option<i32>,
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
    Option<DateTime<Utc>>,
);

/// 登録簿の全ソースの材料を引く（端末・書庫・取り込み器。種類は `SourceKind::of`）。
///
/// - 最後の記録: 出来事の時刻の新しい順に後ろから読み、観測ではない種類を飛ばす。
///   **未来の時刻**（`event_time > now`）の記録は受け取った時刻（`ingest_time`）で数える
/// - 生存信号: 取得できる状態のものだけ。未来の `emitted_at` は `received_at`
/// - 収集開始日: 引き継ぎの鎖の根から継ぐ（鎖は `UNION` で重複を落として、輪になっても止まる）
/// - 引き継いだソースは、行ができた時刻を最後に生きていた時刻の下限にする
/// - 書庫（日数を持つソース）: 読み終えた台帳で、そのソースを運んだ最後の書庫の作られた時刻（無ければ置き場で見つけた時刻）。
///   台帳が 1 件も無ければ記録の届いた最後の時刻。形の確認待ち・項目が 0 件の書庫は台帳の側で数えない
pub async fn inputs<'e>(
    db: impl sqlx::PgExecutor<'e>,
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
         SELECT s.logical_source, s.display_name, s.expected_gap_sec, s.alert_after_days, s.outage_target,
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
                CASE WHEN s.alert_after_days IS NOT NULL THEN coalesce(
                  (SELECT max(coalesce(l.created_at, l.discovered_at))
                     FROM core.archive_ledger l
                     JOIN core.archive_ledger_source ls ON ls.ledger_id = l.id
                    WHERE l.user_id = $1 AND ls.logical_source = s.logical_source
                      AND l.outcome = 'read'),
                  (SELECT max(e.ingest_time) FROM core.event e
                    WHERE e.user_id = $1 AND e.logical_source = s.logical_source)
                ) END AS last_archive_at,
                CASE WHEN s.succeeds IS NOT NULL THEN s.registered_at END AS alive_floor
           FROM core.source s
           JOIN started st ON st.name = s.logical_source
          ORDER BY s.logical_source"
    );
    let rows: Vec<Row> = sqlx::query_as(&sql)
        .bind(user)
        .bind(now)
        .fetch_all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                logical_source,
                display_name,
                expected_gap_sec,
                alert_after_days,
                target,
                has_collected,
                has_non_collected,
                retired_on,
                collection_started_on,
                stopped_now,
                last_stop_end,
                last_record_at,
                last_capturable_heartbeat_at,
                last_archive_at,
                alive_floor,
            )| SourceInput {
                kind: SourceKind::of(&logical_source, alert_after_days),
                logical_source,
                display_name,
                expected_gap_sec,
                alert_after_days,
                target,
                has_collected,
                has_non_collected,
                retired_on,
                collection_started_on,
                stopped_now,
                last_stop_end,
                last_record_at,
                last_capturable_heartbeat_at,
                last_archive_at,
                alive_floor,
            },
        )
        .collect())
}

/// 台帳に残った途切れ 1 件（応答の 1 行。識別子は `opened` の行の `id`）。
#[derive(Debug, Clone)]
pub struct Episode {
    pub episode_id: i64,
    pub outage: Outage,
}

/// 判定して台帳に書く（design D7）。**1 回の取引**:
/// 助言ロック → **判定の材料を読んで判定する**（`inputs` → `outage::judge`。表示名の順に並べる）→
/// 開いている途切れ（`closed` の無い `opened`）を読む → 判定と突き合わせ、
/// 新しい途切れに `opened`・戻ったものに `closed` → `device_id` があれば `delivered`（端末ごとに 1 行）。
/// 同時に 2 本が走っても、ロックで直列になるので `opened` は 1 行。材料もロックの後に読むので、
/// 先に抜けた問い合わせが書いた「戻った」を古い判定で開き直さない（review/code.md R2）。返すのは判定どおりの途切れと識別子。
pub async fn judge_and_record(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    now: DateTime<Utc>,
    device_id: Option<&str>,
) -> Result<Vec<Episode>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('outage_ledger:' || $1::text, 0))")
        .bind(user)
        .execute(&mut *tx)
        .await?;
    let found = inputs(&mut *tx, user, now).await?;
    let mut outages = judge(&found, now);
    outages.sort_by(|a, b| {
        (&a.display_name, &a.logical_source).cmp(&(&b.display_name, &b.logical_source))
    });
    let open: Vec<(i64, String)> = sqlx::query_as(
        "SELECT o.id, o.logical_source FROM core.outage_ledger o
          WHERE o.user_id = $1 AND o.kind = 'opened'
            AND NOT EXISTS (SELECT 1 FROM core.outage_ledger c
                             WHERE c.kind = 'closed' AND c.episode_id = o.id)
          ORDER BY o.id",
    )
    .bind(user)
    .fetch_all(&mut *tx)
    .await?;
    for (id, source) in &open {
        if outages.iter().all(|o| &o.logical_source != source) {
            sqlx::query(
                "INSERT INTO core.outage_ledger (user_id, logical_source, kind, episode_id, judged_at)
                 VALUES ($1, $2, 'closed', $3, $4)",
            )
            .bind(user)
            .bind(source)
            .bind(id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
    }
    let mut episodes = Vec::with_capacity(outages.len());
    for outage in outages {
        let episode_id = match open.iter().find(|(_, s)| *s == outage.logical_source) {
            Some((id, _)) => *id,
            None => {
                sqlx::query_scalar(
                    "INSERT INTO core.outage_ledger (user_id, logical_source, kind, last_alive_at, judged_at)
                     VALUES ($1, $2, 'opened', $3, $4) RETURNING id",
                )
                .bind(user)
                .bind(&outage.logical_source)
                .bind(outage.last_alive_at)
                .bind(now)
                .fetch_one(&mut *tx)
                .await?
            }
        };
        if let Some(device) = device_id {
            sqlx::query(
                "INSERT INTO core.outage_ledger (user_id, logical_source, kind, episode_id, device_id, judged_at)
                 VALUES ($1, $2, 'delivered', $3, $4, $5)
                 ON CONFLICT DO NOTHING",
            )
            .bind(user)
            .bind(&outage.logical_source)
            .bind(episode_id)
            .bind(device)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        episodes.push(Episode { episode_id, outage });
    }
    tx.commit().await?;
    Ok(episodes)
}
