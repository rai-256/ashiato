// SPDX-License-Identifier: AGPL-3.0-only
//! 写しを zip に流す（ST33 / design D4 / D7 / D19）。
//!
//! 1 つの `REPEATABLE READ READ ONLY` のトランザクションで全表を読み、行は 1 行ずつ zip の暗号化の層へ直に流す。
//! `fetch_all` も、表ごとの平文の一時ファイルも使わない（C24）。記録の値はログにもエラーにも出さない。

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::atomic::Ordering;

use futures_util::TryStreamExt;
use sqlx::PgPool;
use zip::write::SimpleFileOptions;
use zip::{AesMode, CompressionMethod, ZipWriter};

use super::doc::{self, FileStat};
use super::{
    excluded_sql, Bloom, Class, ExcludedSql, ExportConfig, ExportError, FailKind, Passphrase,
    CATALOG,
};

const TIME_FORMAT: &str = "YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"";

/// 写しの時点と、書いたファイルごとの実績。
#[derive(Debug, Clone)]
pub struct Dumped {
    pub snapshot_at: chrono::DateTime<chrono::Utc>,
    pub files: Vec<FileStat>,
    /// 記録のファイルに書いた `id` のブルームフィルタ（台帳の `written`。D17）。
    pub written: Vec<u8>,
}

/// 時刻の書き方（マイクロ秒・`Z`）。目録・説明書の時刻もこれで書く。
pub fn format_time(t: chrono::DateTime<chrono::Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

fn db_error(what: &str, e: &sqlx::Error) -> ExportError {
    // 記録の値が入りうる文言は出さず、段と SQLSTATE だけを書く。
    let code = e
        .as_database_error()
        .and_then(|d| d.code().map(|c| c.into_owned()))
        .unwrap_or_default();
    ExportError::new(FailKind::Db, format!("{what}: {code}"))
}

fn zip_error(what: &str, e: &zip::result::ZipError) -> ExportError {
    match e {
        zip::result::ZipError::Io(io) => {
            ExportError::new(FailKind::Io, format!("{what}: {:?}", io.kind()))
        }
        other => ExportError::new(FailKind::Io, format!("{what}: {other}")),
    }
}

fn io_error(what: &str, e: &std::io::Error) -> ExportError {
    ExportError::new(FailKind::Io, format!("{what}: {:?}", e.kind()))
}

/// 1 行を作る SQL の式。列は `CATALOG` の並びで、時刻は UTC の文字列、`raw` は文字列のまま入る。
fn row_sql(
    alias: &str,
    columns: &[(&str, &str)],
    types: &BTreeMap<(String, String), String>,
    view: &str,
) -> String {
    let parts: Vec<String> = columns
        .iter()
        .map(|(col, _)| {
            let expr = match types
                .get(&(view.to_string(), (*col).to_string()))
                .map(String::as_str)
            {
                Some("timestamp with time zone") => {
                    format!("to_char({alias}.\"{col}\" AT TIME ZONE 'UTC', '{TIME_FORMAT}')")
                }
                _ => format!("{alias}.\"{col}\""),
            };
            format!("'{col}', {expr}")
        })
        .collect();
    format!("json_build_object({})::text", parts.join(", "))
}

/// 並び。記録と前の版は `event_time, id`、それ以外は主キー。
fn order_sql(file: &str) -> &'static str {
    match file {
        "events.jsonl" | "event_versions.jsonl" => "t.event_time, t.id",
        "sources.jsonl" => "t.logical_source",
        "drop_report_hours.jsonl" => "t.report_id, t.hour",
        _ => "t.id",
    }
}

struct Plan {
    file: &'static str,
    view: &'static str,
    columns: &'static [(&'static str, &'static str)],
    select: String,
    count: Option<String>,
}

/// 写しのトランザクションを始める（D4）。分類・列の突き合わせもこの中で読み、検査と写しの間に表の形が変わっても気づく。
pub(super) async fn begin(pool: &PgPool) -> Result<Tx, ExportError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| db_error("写しを始められない", &e))?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|e| db_error("写しを始められない", &e))?;
    sqlx::query("SET LOCAL TimeZone = 'UTC'")
        .execute(&mut *tx)
        .await
        .map_err(|e| db_error("写しを始められない", &e))?;
    Ok(tx)
}

pub(super) type Tx = sqlx::Transaction<'static, sqlx::Postgres>;

/// 写し（`tx`）を読んで zip（`file`）に書き、閉じて返す。合言葉は項目ごとの暗号化にだけ渡す。
pub(super) async fn dump(
    mut tx: Tx,
    cfg: &ExportConfig,
    passphrase: &Passphrase,
    export_id: &str,
    file: std::fs::File,
) -> Result<(std::fs::File, Dumped), ExportError> {
    let snapshot_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT transaction_timestamp()")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| db_error("写しの時点を読めない", &e))?;

    let types = column_types(&mut tx).await?;
    let ex = excluded_sql("transaction_timestamp()");
    let plans = plans(&ex, &types);

    // 表ごとの件数（進み具合の分母）。記録と前の版は除く範囲の CTE を 1 回で済ませる。
    let (event_total, version_total) = count_filtered(&mut tx, &ex).await?;
    let mut totals = Vec::new();
    for p in &plans {
        let n = match &p.count {
            Some(sql) => sqlx::query_scalar::<_, i64>(sql)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| db_error("件数を数えられない", &e))?,
            None if p.file == "events.jsonl" => event_total,
            None => version_total,
        };
        totals.push(n);
    }
    cfg.progress.total.store(
        totals.iter().map(|n| (*n).max(0).cast_unsigned()).sum(),
        Ordering::Relaxed,
    );

    let mut zip = ZipWriter::new(file);
    let options = || {
        SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .with_aes_encryption(AesMode::Aes256, passphrase.expose())
            .large_file(true)
    };
    let mut stats = Vec::new();
    let mut written = Bloom::with_capacity(event_total.max(0).cast_unsigned());
    for p in &plans {
        (cfg.hooks.before_file)(p.file).await;
        if let Some(e) = (cfg.hooks.fail)(super::Stage::Item) {
            return Err(e);
        }
        zip.start_file(p.file, options())
            .map_err(|e| zip_error("項目を始められない", &e))?;
        let (mut rows, mut bytes) = (0u64, 0u64);
        {
            let mut stream =
                sqlx::query_as::<_, (String, Option<uuid::Uuid>)>(&p.select).fetch(&mut *tx);
            while let Some((line, id)) = stream
                .try_next()
                .await
                .map_err(|e| db_error("行を読めない", &e))?
            {
                zip.write_all(line.as_bytes())
                    .map_err(|e| io_error("書けない", &e))?;
                zip.write_all(b"\n").map_err(|e| io_error("書けない", &e))?;
                if let (true, Some(id)) = (p.file == "events.jsonl", id) {
                    written.insert(id);
                }
                rows += 1;
                bytes += line.len() as u64 + 1;
                cfg.progress.written.fetch_add(1, Ordering::Relaxed);
            }
        }
        if !cfg.delay.is_zero() {
            tokio::time::sleep(cfg.delay).await;
        }
        stats.push(FileStat {
            name: p.file,
            source: format!("core.{}", p.view),
            rows,
            bytes,
            columns: p.columns,
        });
    }
    tx.commit()
        .await
        .map_err(|e| db_error("写しを閉じられない", &e))?;

    let schema_version = crate::MIGRATIONS.last().map_or("", |m| m.0);
    let (snap, now) = (format_time(snapshot_at), format_time(chrono::Utc::now()));
    let readme = doc::readme(export_id, &snap, &now, schema_version, &stats);
    let manifest = doc::manifest(export_id, &snap, &now, schema_version, &stats);
    let manifest = serde_json::to_string_pretty(&manifest)
        .map_err(|e| ExportError::new(FailKind::Io, format!("目録を作れない: {e}")))?;
    for (name, body) in [("README.txt", readme), ("manifest.json", manifest)] {
        zip.start_file(name, options())
            .map_err(|e| zip_error("項目を始められない", &e))?;
        zip.write_all(body.as_bytes())
            .map_err(|e| io_error("書けない", &e))?;
    }
    zip.set_comment(format!("ashiato-export {export_id}"));
    let file = zip
        .finish()
        .map_err(|e| zip_error("zip を閉じられない", &e))?;
    Ok((
        file,
        Dumped {
            snapshot_at,
            files: stats,
            written: written.to_bytes(),
        },
    ))
}

/// 出す表のビューの列の型（`information_schema`）。時刻の列だけ文字列に直す。
async fn column_types(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<BTreeMap<(String, String), String>, ExportError> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text, data_type::text
           FROM information_schema.columns WHERE table_schema = 'core'",
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| db_error("列の型を読めない", &e))?;
    Ok(rows.into_iter().map(|(t, c, ty)| ((t, c), ty)).collect())
}

fn plans(ex: &ExcludedSql, types: &BTreeMap<(String, String), String>) -> Vec<Plan> {
    let mut out = Vec::new();
    for (_, class) in CATALOG {
        let Class::Include {
            file,
            view,
            columns,
            ..
        } = class
        else {
            continue;
        };
        let obj = row_sql("t", columns, types, view);
        let (select, count) = match *file {
            "events.jsonl" => (
                format!(
                    "WITH {} SELECT {obj}, t.id FROM core.{view} t WHERE {} ORDER BY {}",
                    ex.ctes(),
                    ex.event_kept("t"),
                    order_sql(file)
                ),
                None,
            ),
            "event_versions.jsonl" => (
                format!(
                    "WITH {} SELECT {obj}, NULL::uuid FROM core.{view} t WHERE {} ORDER BY {}",
                    ex.ctes(),
                    ex.version_kept("t"),
                    order_sql(file)
                ),
                None,
            ),
            _ => (
                format!(
                    "SELECT {obj}, NULL::uuid FROM core.{view} t ORDER BY {}",
                    order_sql(file)
                ),
                Some(format!("SELECT count(*) FROM core.{view}")),
            ),
        };
        out.push(Plan {
            file,
            view,
            columns,
            select,
            count,
        });
    }
    out
}

/// 記録と前の版の、除いた後の件数。除く範囲の CTE は 1 回だけ評価される。
async fn count_filtered(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ex: &ExcludedSql,
) -> Result<(i64, i64), ExportError> {
    let sql = format!(
        "WITH {} SELECT (SELECT count(*) FROM core.event_live t WHERE {}),
                        (SELECT count(*) FROM core.event_version_live t WHERE {})",
        ex.ctes(),
        ex.event_kept("t"),
        ex.version_kept("t")
    );
    sqlx::query_as(&sql)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| db_error("件数を数えられない", &e))
}
