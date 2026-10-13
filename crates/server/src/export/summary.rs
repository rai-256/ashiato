// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出す前の件数と見込み（ST33 / design D1 / D12 / D18）。`GET /export/summary` の中身。
//!
//! 件数は除く範囲（`excluded_sql`。D5）で除いた残りを、全利用者の和でソースごとに数える。
//! 件数を別の式で数えない —— 書き出した行の数との一致は、式が 1 つであることで担保する。

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::{excluded_sql, ExportError, FailKind};

/// ソースごとの件数。件数 0 のソースは出さない。
#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SummarySource {
    pub logical_source: String,
    /// 登録簿の表示名。登録簿に無ければソース名。
    pub display_name: String,
    pub count: i64,
    /// いちばん古い記録の時刻。
    pub first: DateTime<Utc>,
    /// いちばん新しい記録の時刻。
    pub last: DateTime<Utc>,
}

/// 書き出しの見込み（D18）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct Estimate {
    /// ソースごとの件数の和（記録の行の数。前の版と辞書の行は含めない）。
    pub rows: i64,
    /// 圧縮した後の zip の大きさの見込み（バイト）。
    pub bytes: i64,
    /// かかる時間の見込み（秒。1 未満は 1）。
    pub seconds: i64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Summary {
    pub sources: Vec<SummarySource>,
    pub estimate: Estimate,
}

/// 前回の書き出しの実績（台帳の最後の `done`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Previous {
    /// 記録のファイル（`events.jsonl`）の行の数。
    pub rows: i64,
    pub bytes: i64,
    /// 始まりの行から終わりの行までの秒。
    pub seconds: f64,
}

/// 前回が無いときの 1 行あたりの大きさ（deep.md 手順 4 の平文 900 バイトに、JSON の Deflate の仮の圧縮率 0.2）。
/// Task 5.6 の量のテストで実測して置き換える。
pub const DEFAULT_BYTES_PER_ROW: i64 = 180;
/// 前回が無いときの 1 秒あたりの行の数（proto の見積もり）。
pub const DEFAULT_ROWS_PER_SECOND: f64 = 20_000.0;

/// 見込み（D18）。前回の実績があればその比で、無ければ既定で見積もる。
pub fn estimate(rows: i64, previous: Option<Previous>) -> Estimate {
    let rows = rows.max(0);
    let prev = previous.filter(|p| p.rows > 0);
    let bytes = match prev {
        Some(p) => i64::try_from(i128::from(p.bytes) * i128::from(rows) / i128::from(p.rows))
            .unwrap_or(i64::MAX),
        None => rows.saturating_mul(DEFAULT_BYTES_PER_ROW),
    };
    let seconds = match prev {
        Some(p) => p.seconds / p.rows as f64 * rows as f64,
        None => rows as f64 / DEFAULT_ROWS_PER_SECOND,
    };
    Estimate {
        rows,
        bytes,
        // 約束でなく見込みなので、切り上げて 1 未満は 1 にする
        seconds: (seconds.ceil() as i64).max(1),
    }
}

fn db(op: &str, e: &sqlx::Error) -> ExportError {
    // 値を含みうる本文は載せず、SQLSTATE だけ（記録の値をエラーに載せない）
    let code = e
        .as_database_error()
        .and_then(|d| d.code())
        .map_or_else(|| "unknown".to_string(), |c| c.into_owned());
    ExportError::new(FailKind::Db, format!("{op}: sqlstate {code}"))
}

/// 書き出す前の件数と見込みを読む。
pub async fn read_summary(pool: &sqlx::PgPool) -> Result<Summary, ExportError> {
    let x = excluded_sql("now()", None);
    let sql = format!(
        "WITH {ctes}
         SELECT e.logical_source,
                coalesce(s.display_name, e.logical_source) AS display_name,
                count(*) AS count, min(e.event_time) AS first, max(e.event_time) AS last
           FROM core.event_live e
           LEFT JOIN core.source s ON s.logical_source = e.logical_source
          WHERE {kept}
          GROUP BY e.logical_source, s.display_name
          ORDER BY e.logical_source",
        ctes = x.ctes(),
        kept = x.event_kept("e"),
    );
    let sources: Vec<SummarySource> = sqlx::query_as(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| db("件数を数えられない", &e))?;
    let total = sources.iter().map(|s| s.count).sum();
    let previous = read_previous(pool).await?;
    Ok(Summary {
        estimate: estimate(total, previous),
        sources,
    })
}

/// 台帳の最後の `done` の終わりの行と、その始まりの行から実績を読む。
async fn read_previous(pool: &sqlx::PgPool) -> Result<Option<Previous>, ExportError> {
    let row: Option<(Option<i64>, Option<i64>, Option<f64>)> = sqlx::query_as(
        "SELECT CASE WHEN f.rows->>'events.jsonl' ~ '^[0-9]{1,15}$'
                     THEN (f.rows->>'events.jsonl')::bigint END,
                f.bytes,
                extract(epoch FROM f.at - s.at)::float8
           FROM core.export_ledger f
           JOIN core.export_ledger s ON s.export_id = f.export_id AND s.kind = 'start'
          WHERE f.kind = 'finish' AND f.result = 'done'
          ORDER BY f.seq DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| db("前回の書き出しを読めない", &e))?;
    Ok(row.and_then(|(rows, bytes, seconds)| {
        Some(Previous {
            rows: rows?,
            bytes: bytes?,
            seconds: seconds?.max(0.0),
        })
    }))
}
