// SPDX-License-Identifier: AGPL-3.0-only
//! 夜の書き直し（ST23 design D11）。本文を消した表を `VACUUM FULL` で書き直し、古い行の痕跡を追い出す。
//!
//! - 毎日 03:30（Asia/Tokyo）に 1 回。時刻は `spawn` に差し込む（壁時計を直に読まない）
//! - 対象は、その表の最後の成功の後に `core.erasure_operation` の行がある表だけ
//! - 書き出しの job が走っている間は始めず、`POLL` ごとに見直す
//! - `pg_class.relfilenode` を前後で比べ、替わらなければ失敗（権限が無いと `VACUUM FULL` は警告だけで成功を返す）
//! - 残すのは表の名前・時刻・結果・SQLSTATE だけ（記録の値は含めない）
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Duration as ChronoDuration, FixedOffset, NaiveTime, TimeZone, Utc};
use sqlx::PgPool;

/// 書き直す表（`core.purge_compaction.table_name` の CHECK と同じ 2 表）。
pub const TABLES: [&str; 2] = ["core.event", "core.event_version"];

/// 書き出しが走っている間に見直す間隔（design D11）。
pub const POLL: Duration = Duration::from_secs(60);

/// 表の排他ロックを待つ上限。超えたら失敗として残す。
const LOCK_TIMEOUT: &str = "5s";

const SQLSTATE_LOCK_NOT_AVAILABLE: &str = "55P03";

/// 1 表の結果。`outcome` は `core.purge_compaction.outcome` の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableResult {
    pub table: &'static str,
    pub outcome: &'static str,
    pub sqlstate: Option<String>,
}

/// 時刻の差し込み口。本番は `Utc::now`。
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// `after` より後の、次の 03:30（Asia/Tokyo。夏時間が無いので +09:00 の固定）。
pub fn next_run_after(after: DateTime<Utc>) -> DateTime<Utc> {
    let tokyo = FixedOffset::east_opt(9 * 3600).expect("+09:00");
    let at_0330 = NaiveTime::from_hms_opt(3, 30, 0).expect("03:30");
    let mut day = after.with_timezone(&tokyo).date_naive();
    loop {
        let candidate = tokyo
            .from_local_datetime(&day.and_time(at_0330))
            .single()
            .expect("固定の差")
            .with_timezone(&Utc);
        if candidate > after {
            return candidate;
        }
        day = day.succ_opt().expect("日付の範囲内");
    }
}

/// 毎日 03:30 に `run_once` を呼ぶ仕事を起こす。握らなくても動き続ける。
pub fn spawn(
    pool: PgPool,
    is_exporting: impl Fn() -> bool + Send + Sync + 'static,
    clock: Clock,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let wait = (next_run_after(clock()) - clock())
                .to_std()
                .unwrap_or(Duration::ZERO);
            tokio::time::sleep(wait).await;
            match run_once(&pool, clock(), &is_exporting, POLL).await {
                Ok(results) => {
                    // ログは表の名前（定数）と結果の種別（自分で名付けた語）だけ
                    for r in &results {
                        tracing::info!(
                            kind = "purge_compaction",
                            op = r.table,
                            reason = r.outcome,
                            sqlstate = r.sqlstate.as_deref().unwrap_or(""),
                            "夜の書き直し"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        kind = "purge_compaction_record",
                        sqlstate = sqlstate_of(&e).as_deref().unwrap_or(""),
                        "夜の書き直しの結果を残せない"
                    );
                }
            }
            // 同じ時刻に続けて起きないよう、次の計算の起点を過ぎるまで待つ
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
}

/// 夜の書き直し 1 回。`now` までに本文を消した操作がある表だけを書き直し、結果を表ごとに 1 行残す。
/// 書き出しが走っている間（`is_exporting`）は始めず、`poll` ごとに見直す。対象が無ければ何も残さない。
pub async fn run_once(
    pool: &PgPool,
    now: DateTime<Utc>,
    is_exporting: &(impl Fn() -> bool + ?Sized),
    poll: Duration,
) -> Result<Vec<TableResult>, sqlx::Error> {
    let mut targets = Vec::new();
    for table in TABLES {
        if has_unrewritten_erasure(pool, table, now).await? {
            targets.push(table);
        }
    }
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    // 書き出しの長い読み出しの後ろで ACCESS EXCLUSIVE を待つと、後続の読み書きがすべて止まる
    while is_exporting() {
        tokio::time::sleep(poll).await;
    }
    let mut results = Vec::new();
    for table in targets {
        let started = Instant::now();
        let (outcome, sqlstate) = rewrite(pool, table).await;
        let finished_at = now + ChronoDuration::from_std(started.elapsed()).unwrap_or_default();
        sqlx::query(
            "INSERT INTO core.purge_compaction (table_name, started_at, finished_at, outcome, sqlstate)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(table)
        .bind(now)
        .bind(finished_at)
        .bind(outcome)
        .bind(&sqlstate)
        .execute(pool)
        .await?;
        results.push(TableResult {
            table,
            outcome,
            sqlstate,
        });
    }
    Ok(results)
}

/// その表の最後の成功の始まりより後、`now` までに消去の操作があるか。
async fn has_unrewritten_erasure(
    pool: &PgPool,
    table: &str,
    now: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (
           SELECT 1 FROM core.erasure_operation op
            WHERE op.at <= $2
              AND op.at > coalesce(
                    (SELECT max(c.started_at) FROM core.purge_compaction c
                      WHERE c.table_name = $1 AND c.outcome = 'rewritten'),
                    '-infinity'::timestamptz))",
    )
    .bind(table)
    .bind(now)
    .fetch_one(pool)
    .await
}

async fn filenode(conn: &mut sqlx::PgConnection, table: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT relfilenode::bigint FROM pg_class WHERE oid = $1::regclass")
        .bind(table)
        .fetch_one(conn)
        .await
}

/// 1 表を書き直し、結果の種別と SQLSTATE を返す。ファイルが替わらなければ書き直せていない。
async fn rewrite(pool: &PgPool, table: &'static str) -> (&'static str, Option<String>) {
    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(e) => return ("failed", sqlstate_of(&e)),
    };
    let result = rewrite_on(&mut conn, table).await;
    // 接続を戻す前に待ち時間の上限を元に戻す（失敗しても接続ごと捨てる）
    if sqlx::query("RESET lock_timeout")
        .execute(&mut *conn)
        .await
        .is_err()
    {
        conn.detach();
    }
    result
}

async fn rewrite_on(
    conn: &mut sqlx::PgConnection,
    table: &'static str,
) -> (&'static str, Option<String>) {
    let fail = |e: &sqlx::Error| {
        let state = sqlstate_of(e);
        let outcome = if state.as_deref() == Some(SQLSTATE_LOCK_NOT_AVAILABLE) {
            "lock_timeout"
        } else {
            "failed"
        };
        (outcome, state)
    };
    let before = match filenode(conn, table).await {
        Ok(n) => n,
        Err(e) => return fail(&e),
    };
    if let Err(e) = sqlx::query(&format!("SET lock_timeout = '{LOCK_TIMEOUT}'"))
        .execute(&mut *conn)
        .await
    {
        return fail(&e);
    }
    // 表の名前は定数（TABLES）だけ。値を差し込まない
    if let Err(e) = sqlx::query(&format!("VACUUM FULL {table}"))
        .execute(&mut *conn)
        .await
    {
        return fail(&e);
    }
    match filenode(conn, table).await {
        Ok(after) if after != before => ("rewritten", None),
        Ok(_) => ("not_rewritten", None),
        Err(e) => fail(&e),
    }
}

fn sqlstate_of(e: &sqlx::Error) -> Option<String> {
    e.as_database_error()
        .and_then(|d| d.code())
        .map(|c| c.into_owned())
}
