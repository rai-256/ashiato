// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの台帳 `core.export_ledger` の読み書きと、状態（`GET /export`）の組み立て（ST33 / design D10 / D11 / D16 / D17）。
//!
//! 台帳は追記のみ。1 回の書き出しは始まりの行と終わりの行の 2 行。**記録の値も合言葉も持たない**。
//! 状態を作る部分（`derive` と `Derived::into_status`）は純関数で、ファイルの有無と件数は引数で受ける。

use std::io::Read as _;
use std::path::Path;

use chrono::{DateTime, Utc};
use futures_util::TryStreamExt as _;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use super::{excluded_sql, Bloom, ExportError, FailKind};

/// 始まりの行の経路（Q1 / Q9）。
pub const ROUTE: &str = "pc-encrypted-zip";

/// 台帳の 1 行（大きい `written` は読まない）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LedgerRow {
    pub seq: i64,
    pub export_id: Uuid,
    pub kind: String,
    pub at: DateTime<Utc>,
    pub file: String,
    pub result: Option<String>,
    pub rows: Option<serde_json::Value>,
    pub bytes: Option<i64>,
    pub digest: Option<String>,
    pub reason: Option<String>,
}

impl LedgerRow {
    fn is_start(&self) -> bool {
        self.kind == "start"
    }

    fn is_done(&self) -> bool {
        self.kind == "finish" && self.result.as_deref() == Some("done")
    }

    fn is_failed(&self) -> bool {
        self.kind == "finish" && self.result.as_deref() == Some("failed")
    }

    /// 表ごとの行の数の和（`last.rows`）。
    fn row_total(&self) -> i64 {
        self.rows
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .map_or(0, |m| {
                m.values().filter_map(serde_json::Value::as_i64).sum()
            })
    }
}

fn db(op: &str, e: &sqlx::Error) -> ExportError {
    // 値を含みうる本文は載せず、SQLSTATE だけ。
    let code = e
        .as_database_error()
        .and_then(|d| d.code())
        .map_or_else(|| "unknown".to_string(), |c| c.into_owned());
    ExportError::new(FailKind::Db, format!("{op}: sqlstate {code}"))
}

pub async fn read_ledger(pool: &sqlx::PgPool) -> Result<Vec<LedgerRow>, ExportError> {
    sqlx::query_as(
        "SELECT seq, export_id, kind, at, file, result, rows, bytes, digest, reason
           FROM core.export_ledger ORDER BY seq",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| db("台帳を読めない", &e))
}

/// 始まりの行。`snapshot_at` は「写しを読み始める直前」の時刻（D4）。
pub async fn insert_start(
    pool: &sqlx::PgPool,
    export_id: Uuid,
    file: &str,
    snapshot_at: DateTime<Utc>,
) -> Result<(), ExportError> {
    sqlx::query(
        "INSERT INTO core.export_ledger (export_id, kind, route, file, snapshot_at)
         VALUES ($1, 'start', $2, $3, $4)",
    )
    .bind(export_id)
    .bind(ROUTE)
    .bind(file)
    .bind(snapshot_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| db("台帳に始まりを書けない", &e))
}

/// 書き終えた書き出しの終わりの行。
#[derive(Debug)]
pub struct Done<'a> {
    pub file: &'a str,
    pub snapshot_at: DateTime<Utc>,
    pub rows: serde_json::Value,
    pub bytes: i64,
    pub written: &'a [u8],
    pub digest: &'a str,
}

pub async fn insert_done(
    pool: &sqlx::PgPool,
    export_id: Uuid,
    d: &Done<'_>,
) -> Result<(), ExportError> {
    sqlx::query(
        "INSERT INTO core.export_ledger
           (export_id, kind, route, file, snapshot_at, result, rows, bytes, written, digest)
         VALUES ($1, 'finish', $2, $3, $4, 'done', $5, $6, $7, $8)",
    )
    .bind(export_id)
    .bind(ROUTE)
    .bind(d.file)
    .bind(d.snapshot_at)
    .bind(&d.rows)
    .bind(d.bytes)
    .bind(d.written)
    .bind(d.digest)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| db("台帳に終わりを書けない", &e))
}

/// 失敗した書き出しの終わりの行。`reason` は `ExportError` の `Display`（種別と表・列・段の名前だけ）。
/// 状態はこれを `FailKind::of_reason` で種別に戻す（作る側と読む側が同じ形を使う）。
pub async fn insert_failed(
    pool: &sqlx::PgPool,
    export_id: Uuid,
    file: &str,
    error: &ExportError,
) -> Result<(), ExportError> {
    let reason = error.to_string();
    sqlx::query(
        "INSERT INTO core.export_ledger (export_id, kind, route, file, result, reason)
         VALUES ($1, 'finish', $2, $3, 'failed', $4)",
    )
    .bind(export_id)
    .bind(ROUTE)
    .bind(file)
    .bind(&reason)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| db("台帳に失敗を書けない", &e))
}

// ---------------------------------------------------------------------------
// 前のファイルを見分ける（D16）
// ---------------------------------------------------------------------------

/// 台帳の `done` の行が指すファイルが、**書いたその物のまま**か。
/// 大きさ → zip の注記 `ashiato-export <export_id>` → SHA-256（`digest`）の順に確かめ、
/// どれかが合わない・zip として読めない・無いなら `false`（消さない側に倒す）。
pub fn is_untouched(row: &LedgerRow) -> bool {
    is_untouched_with(row, |p, _| sha256_of(p).ok())
}

/// `is_untouched` の SHA-256 の取り方を受けるもの（状態の読み出しが（パス・大きさ・更新時刻）で覚えた値を使う。R13）。
pub fn is_untouched_with(
    row: &LedgerRow,
    digest_of: impl Fn(&Path, &std::fs::Metadata) -> Option<String>,
) -> bool {
    let (Some(bytes), Some(digest)) = (row.bytes, row.digest.as_deref()) else {
        return false;
    };
    let path = Path::new(&row.file);
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() || i64::try_from(meta.len()).ok() != Some(bytes) {
        return false;
    }
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(zip) = zip::ZipArchive::new(file) else {
        return false;
    };
    if zip.comment() != format!("ashiato-export {}", row.export_id).as_bytes() {
        return false;
    }
    digest_of(path, &meta).is_some_and(|d| d == digest)
}

/// ファイルの SHA-256（16 進）。
pub fn sha256_of(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    use std::fmt::Write as _;
    Ok(h.finalize().iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    }))
}

/// 台帳に `done` の終わりの行を持つ書き出しのうち、`except` 以外で、ファイルが書いたその物のまま残っているもの。
/// 前のファイルを消す対象（D16）であり、消せなかったものは `GET /export` の `leftover` に出る（D11）。
pub fn removable_previous(rows: &[LedgerRow], except: Option<Uuid>) -> Vec<&LedgerRow> {
    rows.iter()
        .filter(|r| r.is_done() && Some(r.export_id) != except && is_untouched(r))
        .collect()
}

// ---------------------------------------------------------------------------
// 状態（D11）
// ---------------------------------------------------------------------------

/// 走っている書き出し（メモリ）。
#[derive(Debug, Clone, Serialize, PartialEq, Eq, utoipa::ToSchema)]
pub struct RunningView {
    pub export_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub written: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, utoipa::ToSchema)]
pub struct LastView {
    pub finished_at: DateTime<Utc>,
    pub rows: i64,
    pub bytes: i64,
    pub file: String,
    /// 後に消した記録の件数（D17）。`null` は数えられない —— 台帳の書いた記録が読めない形だった（deep Q12）。
    pub erased_after: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, utoipa::ToSchema)]
pub struct FailureView {
    pub at: DateTime<Utc>,
    /// 種別だけ（`unclassified_table` / `missing_column` / `io` / `db` / `late_mark`）。
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, utoipa::ToSchema)]
pub struct InterruptedView {
    pub started_at: DateTime<Utc>,
    /// 書き出しの名前のファイルがあるときだけ。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// `GET /export` の応答。**記録の値を持たない**。
#[derive(Debug, Clone, Serialize, PartialEq, Eq, utoipa::ToSchema)]
pub struct Status {
    pub running: Option<RunningView>,
    pub last: Option<LastView>,
    pub last_failure: Option<FailureView>,
    pub interrupted: Option<InterruptedView>,
    pub leftover: Vec<String>,
}

/// 台帳の行から導いたもの（ファイルと件数を見る前）。
#[derive(Debug, Clone)]
pub struct Derived {
    /// 最後の `done` の終わりの行。
    pub last: Option<LedgerRow>,
    last_failure: Option<FailureView>,
    /// 終わりの行が無く、走ってもいない始まりの行（新しい順）と、それが最後の始まりの行か。
    interrupted: Vec<(DateTime<Utc>, String, bool)>,
    /// `last` の始まりより前に始まった、`done` の書き出しの終わりの行（`leftover` の候補）。
    earlier_done: Vec<LedgerRow>,
}

/// 台帳の行（`seq` の順）と、走っている書き出しの識別子から導く。
pub fn derive(rows: &[LedgerRow], running: Option<Uuid>) -> Derived {
    let last = rows.iter().rfind(|r| r.is_done()).cloned();
    let start_seq = |id: Uuid| {
        rows.iter()
            .find(|r| r.is_start() && r.export_id == id)
            .map(|r| r.seq)
    };
    let last_failure = rows
        .iter()
        .rfind(|r| r.is_failed())
        .filter(|f| last.as_ref().is_none_or(|l| f.seq > l.seq))
        .map(|f| FailureView {
            at: f.at,
            kind: f
                .reason
                .as_deref()
                .and_then(FailKind::of_reason)
                .map_or("unknown", FailKind::as_str)
                .to_string(),
        });
    let latest_start = rows.iter().rfind(|r| r.is_start()).map(|r| r.seq);
    let interrupted = rows
        .iter()
        .rev()
        .filter(|s| {
            s.is_start()
                && Some(s.export_id) != running
                && !rows
                    .iter()
                    .any(|r| r.kind == "finish" && r.export_id == s.export_id)
        })
        .map(|s| (s.at, s.file.clone(), Some(s.seq) == latest_start))
        .collect();
    let earlier_done = match last.as_ref().and_then(|l| start_seq(l.export_id)) {
        Some(last_start) => rows
            .iter()
            .filter(|r| {
                r.is_done()
                    && Some(r.export_id) != last.as_ref().map(|l| l.export_id)
                    && start_seq(r.export_id).is_some_and(|s| s < last_start)
            })
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    Derived {
        last,
        last_failure,
        interrupted,
        earlier_done,
    }
}

impl Derived {
    /// 状態にする。`file_exists` は止まった書き出しのファイルの有無、`untouched` は前のファイルが書いたその物のままか。
    pub fn into_status(
        self,
        running: Option<RunningView>,
        erased_after: Option<i64>,
        file_exists: impl Fn(&str) -> bool,
        untouched: impl Fn(&LedgerRow) -> bool,
    ) -> Status {
        Status {
            running,
            last: self.last.map(|l| LastView {
                finished_at: l.at,
                rows: l.row_total(),
                bytes: l.bytes.unwrap_or(0),
                file: l.file,
                erased_after,
            }),
            last_failure: self.last_failure,
            // 最後の始まりの行なら、ファイルが無くても止まったと返す。それより前に止まったものは、名前のファイルが
            // 残っている間だけ返す（次の書き出しが書き終えても、置き場に残るファイルを状態から消さない。R18）。
            interrupted: self
                .interrupted
                .into_iter()
                .find_map(|(started_at, file, latest)| {
                    let exists = file_exists(&file);
                    (latest || exists).then(|| InterruptedView {
                        started_at,
                        file: exists.then_some(file),
                    })
                }),
            leftover: self
                .earlier_done
                .iter()
                .filter(|r| untouched(r))
                .map(|r| r.file.clone())
                .collect(),
        }
    }
}

/// 後に消した記録の件数の鍵（D17 / R1）。**件数を増やしうる書き込みは、どれもここのどれかを動かす**:
///
/// - 前回の書き出し（`last`）が変わる → `last`
/// - 本人の削除の印（消す・連鎖・遅着の印）と戻す → `core.deletion_ledger` に同じまとまりで 1 行（`deletion.rs` / `stay_store::mark_late_arrivals`）
/// - 記録の内容・時刻の更新（取り込みの更新・滞在の作り直しの `push_version`）→ `core.event_version` に 1 行
/// - 作り直しの吸収（`rebuild:absorbed`）→ `core.stay_absorbed` に 1 行
///
/// 3 つの表はどれも追記のみ（行の書き換え・削除をトリガが断る）なので、件数は新しい行がコミットされるたびに必ず増える
/// （`max(seq)` は、先に番号を取って後にコミットした行を見落としうるので使わない）。
/// 鍵に無い書き込み —— 新しい記録の取り込み（次の記録が手前に来て幅が縮む）、作り直しが付ける `rebuild:erased-range`
/// の印（`s01-stay` だけで、滞在は `end` で幅を読むので他の行の幅を延ばさない）、消去（本文を空にする）—— は件数を減らす側か
/// 動かさない側にしか働かず、覚えた値は多く数える側にだけずれる（D17「誤りは多く数える側にしか出ない」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErasedKey {
    last: Uuid,
    deletions: i64,
    versions: i64,
    absorbed: i64,
}

pub async fn erased_key(pool: &sqlx::PgPool, last: Uuid) -> Result<ErasedKey, ExportError> {
    let (deletions, versions, absorbed): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM core.deletion_ledger),
                (SELECT count(*) FROM core.event_version),
                (SELECT count(*) FROM core.stay_absorbed)",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| db("後に消した記録の鍵を読めない", &e))?;
    Ok(ErasedKey {
        last,
        deletions,
        versions,
        absorbed,
    })
}

/// 後に消した記録の件数（D17）。前回の記録のファイルに書いた `id` のうち、いま除かれるもの。
/// いま除かれる = 本人の削除の印がある（作り直しの印は除く）か、D5 の範囲に当たる。偽陽性は多く数える側にだけ出る。
/// 書いた記録が無い・読めない形（知らない版）なら `None`（数えられない）。0 にすると少なく数える側に倒れる（deep Q12 / R20）。
pub async fn erased_after(pool: &sqlx::PgPool, last: Uuid) -> Result<Option<i64>, ExportError> {
    let written: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT written FROM core.export_ledger
          WHERE export_id = $1 AND kind = 'finish' AND result = 'done'",
    )
    .bind(last)
    .fetch_optional(pool)
    .await
    .map_err(|e| db("書いた記録を読めない", &e))?
    .flatten();
    let Some(bloom) = written.as_deref().and_then(Bloom::from_bytes) else {
        return Ok(None);
    };
    let x = excluded_sql("now()");
    let sql = format!(
        "WITH {ctes}
         SELECT id FROM (
           SELECT e.id FROM core.event e
            WHERE e.deleted_at IS NOT NULL
              AND (e.deleted_by IS NULL OR e.deleted_by NOT LIKE '{prefix}%')
           UNION
           SELECT id FROM {excluded}
         ) u",
        ctes = x.ctes(),
        prefix = crate::stay_store::REBUILD_PREFIX,
        excluded = super::EXCLUDED_EVENT,
    );
    let mut ids = sqlx::query_scalar::<_, Uuid>(&sql).fetch(pool);
    let mut n = 0i64;
    while let Some(id) = ids
        .try_next()
        .await
        .map_err(|e| db("除かれる記録を読めない", &e))?
    {
        if bloom.contains(id) {
            n += 1;
        }
    }
    Ok(Some(n))
}
