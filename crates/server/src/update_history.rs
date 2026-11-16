// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 更新の履歴（`core.update_history`）を書く（ST51 / design D7・D8・D9・D15・D16）。
//!
//! 版の変わり目を 1 行ずつ追記する。**前の版**は書いた順（`seq`）で最後の通った行（`applied`・`nothing_to_apply`）の
//! 今の版で、時刻では決めない。比べて書くまでを 1 つの取引にし、取引の頭で更新の履歴の鍵を取る。
//!
//! 書けないとき（表がまだ無い・接続が切れた・当てる処理の錠が取れない）は、種別 `update_history_unwritten` の警告を出して
//! 呼び出し元を止めない（D9）。**DB の誤りの本文は行にも警告にも出さない**（Q1・D8）—— SQLSTATE だけを持つ。
//!
//! 組み立ての印は引数で受ける。実行時に差し替える口は作らない（D15）。

use crate::migration_ledger::{compare, read_ledger_on, Ledger, Verdict, LOCK_KEY};
use serde::{Deserialize, Serialize};
use sqlx::{Connection as _, PgConnection};
use std::collections::BTreeSet;
use std::path::Path;

/// 書けなかったときの警告の種別（D9）。種別の語はここにだけ置く（起動の接続を取れない分岐も同じ口を通る）。
pub const UNWRITTEN_KIND: &str = "update_history_unwritten";

/// 更新の履歴の鍵（取引の助言錠）。当てる処理の錠（`LOCK_KEY`）とは別の数。
const HISTORY_KEY: i64 = 4_820_260;

/// 1 行の結果（`outcome` の列と、それに付く値。design D8 の表）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<'a> {
    /// 版の検査を通った後・退避の前の「試み」。
    Attempted,
    /// 当てた版（ORDER の順）。
    Applied(&'a [String]),
    /// 当てる版が無い。起動の行もこれ。
    NothingToApply,
    /// 落ちた。それまでに当て終えた版と、落ちた版の名前・SQLSTATE（取れなければ空）。
    Failed {
        applied: &'a [String],
        failed_version: Option<&'a str>,
        sqlstate: Option<&'a str>,
    },
    /// 退避に失敗して、何も当てていない。
    BackupFailed,
    /// 退避から戻した。`previous_version` の判定には数えない。
    RolledBack { applied: &'a [String] },
}

impl Outcome<'_> {
    fn name(&self) -> &'static str {
        match self {
            Self::Attempted => "attempted",
            Self::Applied(_) => "applied",
            Self::NothingToApply => "nothing_to_apply",
            Self::Failed { .. } => "failed",
            Self::BackupFailed => "backup_failed",
            Self::RolledBack { .. } => "rolled_back",
        }
    }

    fn applied(&self) -> &[String] {
        match self {
            Self::Applied(v)
            | Self::Failed { applied: v, .. }
            | Self::RolledBack { applied: v } => v,
            _ => &[],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackKind<'a> {
    /// 落ちた版と SQLSTATE。分からなければ `None`（列は NULL。新しい版の名前で埋めない）。
    MigrationFailed {
        failed_version: Option<&'a str>,
        sqlstate: Option<&'a str>,
    },
    StartupFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingRow {
    at: String,
    component: String,
    via: String,
    from_version: Option<String>,
    to_version: String,
    outcome: String,
    applied_versions: Vec<String>,
    failed_version: Option<String>,
    sqlstate: Option<String>,
    #[serde(default)]
    written_at: Option<String>,
}

#[derive(Debug)]
pub enum RollbackError {
    Database { sqlstate: Option<String> },
    PendingWritten,
    PendingIo(std::io::Error),
    PendingJson(serde_json::Error),
}

impl std::fmt::Display for RollbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database { .. } => f.write_str("更新の履歴を書けず、控えにも書けない"),
            Self::PendingWritten => f.write_str("更新の履歴は控えに書いた"),
            Self::PendingIo(_) => f.write_str("控えの読み書きに失敗"),
            Self::PendingJson(_) => f.write_str("控えの JSON が不正"),
        }
    }
}

impl std::error::Error for RollbackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PendingIo(e) => Some(e),
            Self::PendingJson(e) => Some(e),
            _ => None,
        }
    }
}

fn database_error(error: sqlx::Error) -> RollbackError {
    RollbackError::Database {
        sqlstate: error
            .as_database_error()
            .and_then(|e| e.code().as_deref().map(str::to_owned)),
    }
}

fn rollback_row(
    at: &str,
    from: &str,
    to: &str,
    applied: &[String],
    failed: Option<&str>,
    state: Option<&str>,
) -> PendingRow {
    PendingRow {
        at: at.into(),
        component: "S-01".into(),
        via: "migrate".into(),
        from_version: Some(from.into()),
        to_version: to.into(),
        outcome: "rolled_back".into(),
        applied_versions: applied.to_vec(),
        failed_version: failed.map(str::to_owned),
        sqlstate: state.map(str::to_owned),
        written_at: None,
    }
}

async fn now_string(conn: &mut PgConnection) -> Result<String, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')",
    )
    .fetch_one(conn)
    .await
}

async fn insert_pending_row(conn: &mut PgConnection, row: &PendingRow) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO core.update_history (at, component, via, from_version, to_version, outcome, applied_versions, failed_version, sqlstate) VALUES ($1::timestamptz, $2, $3, $4, $5, $6, $7, $8, $9)")
        .bind(&row.at).bind(&row.component).bind(&row.via).bind(&row.from_version).bind(&row.to_version)
        .bind(&row.outcome).bind(&row.applied_versions).bind(&row.failed_version).bind(&row.sqlstate)
        .execute(conn).await?;
    Ok(())
}

/// 戻した DB に履歴を足す。DB へ足す 1 回の取引に失敗した場合だけ控えへ書く。
pub async fn record_rollback(
    conn: &mut PgConnection,
    new_version: &str,
    previous: &str,
    kind: RollbackKind<'_>,
    versions: &[(&str, &str)],
    pending_out: Option<&Path>,
) -> Result<(), RollbackError> {
    let mut tx = conn.begin().await.map_err(database_error)?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(HISTORY_KEY)
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    let at = now_string(&mut tx).await.map_err(database_error)?;
    let applied: Vec<String> = if matches!(kind, RollbackKind::StartupFailed) {
        let known: BTreeSet<&str> = versions.iter().map(|(name, _)| *name).collect();
        let current: Vec<String> =
            sqlx::query_scalar("SELECT name FROM migration_ledger.applied ORDER BY name")
                .fetch_all(&mut *tx)
                .await
                .map_err(database_error)?;
        versions
            .iter()
            .filter(|(name, _)| !current.iter().any(|x| x == name) && known.contains(name))
            .map(|(name, _)| (*name).to_string())
            .collect()
    } else {
        Vec::new()
    };
    let (failed_version, sqlstate) = match kind {
        RollbackKind::MigrationFailed {
            failed_version,
            sqlstate,
        } => (failed_version, sqlstate),
        RollbackKind::StartupFailed => (None, None),
    };
    let rollback = rollback_row(
        &at,
        new_version,
        previous,
        &applied,
        failed_version,
        sqlstate,
    );
    let failed = if let RollbackKind::MigrationFailed {
        failed_version,
        sqlstate,
    } = kind
    {
        let attempted: Option<(Option<String>, String)> = sqlx::query_as(
            "SELECT from_version, to_version FROM core.update_history
             WHERE outcome = 'attempted' ORDER BY seq DESC LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(database_error)?;
        attempted.map(|(from_version, to_version)| PendingRow {
            at: at.clone(),
            component: "S-01".into(),
            via: "migrate".into(),
            from_version,
            to_version,
            outcome: "failed".into(),
            applied_versions: Vec::new(),
            failed_version: failed_version.map(str::to_owned),
            sqlstate: sqlstate.map(str::to_owned),
            written_at: None,
        })
    } else {
        None
    };
    let write_result = async {
        if let Some(failed) = &failed {
            insert_pending_row(&mut tx, failed).await?;
        }
        insert_pending_row(&mut tx, &rollback).await
    }
    .await;
    match write_result {
        Ok(()) => {
            tx.commit().await.map_err(database_error)?;
            Ok(())
        }
        Err(e) => {
            tx.rollback().await.ok();
            if let Some(path) = pending_out {
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(RollbackError::PendingIo)?;
                if let Some(failed) = &failed {
                    serde_json::to_writer(&mut f, failed).map_err(RollbackError::PendingJson)?;
                    use std::io::Write as _;
                    writeln!(f).map_err(RollbackError::PendingIo)?;
                }
                serde_json::to_writer(&mut f, &rollback).map_err(RollbackError::PendingJson)?;
                use std::io::Write as _;
                writeln!(f).map_err(RollbackError::PendingIo)?;
                Err(RollbackError::PendingWritten)
            } else {
                Err(database_error(e))
            }
        }
    }
}

/// 控えの未写し行を時刻を保ったまま書き、成功した行に `written_at` を付ける。
pub async fn replay_pending(conn: &mut PgConnection, path: &Path) -> Result<(), RollbackError> {
    let text = std::fs::read_to_string(path).map_err(RollbackError::PendingIo)?;
    let mut rows: Vec<PendingRow> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(RollbackError::PendingJson))
        .collect::<Result<_, _>>()?;
    let mut tx = conn.begin().await.map_err(database_error)?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(HISTORY_KEY)
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    for row in rows.iter_mut().filter(|r| r.written_at.is_none()) {
        // 写した後・控えを書き直す前に落ちた回は、控えに written_at が無いまま残る。
        // 同じ時刻・結果・版の行が既にあれば写し済みとみなし、2 行にしない（review/code.md R63）
        let already: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM core.update_history
              WHERE at = $1::timestamptz AND component = $2 AND outcome = $3 AND to_version = $4)",
        )
        .bind(&row.at)
        .bind(&row.component)
        .bind(&row.outcome)
        .bind(&row.to_version)
        .fetch_one(&mut *tx)
        .await
        .map_err(database_error)?;
        if !already {
            insert_pending_row(&mut tx, row)
                .await
                .map_err(database_error)?;
        }
        row.written_at = Some(now_string(&mut tx).await.map_err(database_error)?);
    }
    tx.commit().await.map_err(database_error)?;
    let out = rows
        .into_iter()
        .map(|r| serde_json::to_string(&r).map_err(RollbackError::PendingJson))
        .collect::<Result<Vec<_>, _>>()?
        .join("\n")
        + "\n";
    let tmp = path.with_extension(format!("pending.{}.tmp", std::process::id()));
    std::fs::write(&tmp, out).map_err(RollbackError::PendingIo)?;
    std::fs::rename(tmp, path).map_err(RollbackError::PendingIo)
}

/// 試みの行をどうしたか（結果の行を書くかを決める。D16（仮））。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    /// 試みの行を書いた。結果の行は比べ直さずに必ず書く（R17）。
    Written,
    /// 表があり、前の版と同じだったので書かなかった。結果の行も書かない。
    NotNeeded,
    /// 表が無い（または書けなかった）。結果の行は、書けるなら D2 の条件（前の版と違えば）で書く。
    Unwritten,
}

#[cfg(test)]
thread_local! {
    /// 試験が「警告を出したか」を見る口。警告の取り込みは並走する試験と callsite の登録を取り合って落ちるので、
    /// 同じ場所で、この試験のスレッドに理由の語を残す（本番の経路は変えない）。
    static UNWRITTEN: std::cell::RefCell<Vec<&'static str>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 警告を出した理由の語（このスレッドで出した分。試験用）。
#[cfg(test)]
pub(crate) fn unwritten_seen() -> Vec<&'static str> {
    UNWRITTEN.with(|v| v.borrow().clone())
}

/// 書けなかったことを警告に出す。DB の誤りは SQLSTATE だけ（本文は出さない）。
/// `why` は固定の語（`no_table`・`db_error`・`migrate_running`・`db_ahead`・`ledger_unreadable`・`acquire`）。
pub(crate) fn warn_unwritten(why: &'static str, e: Option<&sqlx::Error>) {
    #[cfg(test)]
    UNWRITTEN.with(|v| v.borrow_mut().push(why));
    let sqlstate = e.and_then(sqlstate_of).unwrap_or_default();
    // log-ok: 種別と理由の固定の語・SQLSTATE だけ（DB の誤りの本文と私的データは出さない）
    tracing::warn!(
        kind = UNWRITTEN_KIND,
        why,
        sqlstate = sqlstate.as_str(),
        "更新の履歴を書けなかった（続ける）"
    );
}

/// DB の誤りの SQLSTATE（取れなければ `None`）。誤りの本文は返さない。
pub fn sqlstate_of(e: &sqlx::Error) -> Option<String> {
    e.as_database_error()
        .and_then(|d| d.code())
        .map(|c| c.to_string())
}

async fn table_exists(conn: &mut PgConnection) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT to_regclass('core.update_history') IS NOT NULL")
        .fetch_one(conn)
        .await
}

/// 表があれば `true`。無い・確かめられないときは警告を出して `false`（migrate の試みと結果の口が共有する）。
async fn table_ready(conn: &mut PgConnection) -> bool {
    match table_exists(conn).await {
        Ok(true) => true,
        Ok(false) => {
            warn_unwritten("no_table", None);
            false
        }
        Err(e) => {
            warn_unwritten("db_error", Some(&e));
            false
        }
    }
}

/// 書いた順で最後の通った行の今の版（D7）。
async fn previous_version(conn: &mut PgConnection) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT to_version FROM core.update_history
          WHERE outcome IN ('applied', 'nothing_to_apply')
          ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(conn)
    .await
}

async fn insert_row(
    conn: &mut PgConnection,
    via: &str,
    from: Option<&str>,
    stamp: &str,
    outcome: &Outcome<'_>,
) -> Result<(), sqlx::Error> {
    let (failed_version, sqlstate) = match outcome {
        Outcome::Failed {
            failed_version,
            sqlstate,
            ..
        } => (*failed_version, *sqlstate),
        _ => (None, None),
    };
    sqlx::query(
        "INSERT INTO core.update_history
           (component, via, from_version, to_version, outcome, applied_versions, failed_version, sqlstate)
         VALUES ('S-01', $1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(via)
    .bind(from)
    .bind(stamp)
    .bind(outcome.name())
    .bind(outcome.applied())
    .bind(failed_version)
    .bind(sqlstate)
    .execute(conn)
    .await?;
    Ok(())
}

/// 前の版と比べて、`always` か違うときに 1 行書く。比べて書くまでを 1 つの取引にし、頭で鍵を取る（D7）。
/// 書いたら `true`。
async fn write_in_tx(
    conn: &mut PgConnection,
    via: &str,
    stamp: &str,
    outcome: &Outcome<'_>,
    always: bool,
) -> Result<bool, sqlx::Error> {
    let mut tx = conn.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(HISTORY_KEY)
        .execute(&mut *tx)
        .await?;
    let prev = previous_version(&mut tx).await?;
    if !always && prev.as_deref() == Some(stamp) {
        return Ok(false);
    }
    insert_row(&mut tx, via, prev.as_deref(), stamp, outcome).await?;
    tx.commit().await?;
    Ok(true)
}

/// 版の検査を通った後・退避の前に、試みの行を書いて commit する（Q6・D16（仮））。
/// 当てる処理の錠を持った接続で呼ぶ（版を当てる取引の外）。前の版と違うときだけ書く。
pub async fn record_attempt(conn: &mut PgConnection, stamp: &str) -> Attempt {
    if !table_ready(conn).await {
        return Attempt::Unwritten;
    }
    match write_in_tx(conn, "migrate", stamp, &Outcome::Attempted, false).await {
        Ok(true) => Attempt::Written,
        Ok(false) => Attempt::NotNeeded,
        Err(e) => {
            warn_unwritten("db_error", Some(&e));
            Attempt::Unwritten
        }
    }
}

/// migrate の結果の行を書く。当てる処理の錠を持った接続で、錠を外す前に呼ぶ（R28）。
/// 試みの行を書いた回は比べ直さずに必ず書く（R17）。書かなかった回（表があって同じ版）は書かない。
/// 試みを書けなかった回は、表が今あれば前の版と違うときに書く。
pub async fn record_migrate(
    conn: &mut PgConnection,
    stamp: &str,
    attempt: Attempt,
    outcome: &Outcome<'_>,
) {
    let always = match attempt {
        Attempt::NotNeeded => return,
        Attempt::Written => true,
        Attempt::Unwritten => false,
    };
    if !table_ready(conn).await {
        return;
    }
    if let Err(e) = write_in_tx(conn, "migrate", stamp, outcome, always).await {
        warn_unwritten("db_error", Some(&e));
    }
}

/// 起動の行を書く。版の検査の後・待ち受けの前に、実行時の役割の接続で呼ぶ（D2・D8）。
/// 当てる処理の錠を待たずに共有で取ろうとし、取れなければ（migrate が走っている）書かずに警告する。
/// 共有なので、同時に起動した S-01 どうしは互いを migrate と取り違えない（更新の履歴の鍵で 1 本ずつ書く）。
/// 取れたら版の記録を読み直し、自分の知らない版があれば書かない（R26）。前の版と違うときだけ書く。
/// 書かなかった理由（警告に出した語）を返す。書いた・同じ版で書かなくてよいときは `None`。
pub async fn record_startup(
    conn: &mut PgConnection,
    stamp: &str,
    versions: &[(&str, &str)],
) -> Option<&'static str> {
    let why = match startup_tx(conn, stamp, versions).await {
        Ok(None) => return None,
        Ok(Some(why)) => {
            warn_unwritten(why, None);
            why
        }
        Err(e) => {
            warn_unwritten("db_error", Some(&e));
            "db_error"
        }
    };
    Some(why)
}

/// `Ok(None)` は書いた（または同じ版で書かなくてよい）。`Ok(Some(why))` は書かなかった理由。
async fn startup_tx(
    conn: &mut PgConnection,
    stamp: &str,
    versions: &[(&str, &str)],
) -> Result<Option<&'static str>, sqlx::Error> {
    if !table_exists(conn).await? {
        return Ok(Some("no_table"));
    }
    let mut tx = conn.begin().await?;
    let got: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock_shared($1)")
        .bind(LOCK_KEY)
        .fetch_one(&mut *tx)
        .await?;
    if !got {
        return Ok(Some("migrate_running"));
    }
    let known: BTreeSet<String> = versions.iter().map(|v| v.0.to_string()).collect();
    match read_ledger_on(&mut tx).await? {
        Ledger::Recorded(names) if matches!(compare(&known, &names), Verdict::Ahead { .. }) => {
            return Ok(Some("db_ahead"));
        }
        Ledger::Unreadable(_) => return Ok(Some("ledger_unreadable")),
        _ => {}
    }
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(HISTORY_KEY)
        .execute(&mut *tx)
        .await?;
    let prev = previous_version(&mut tx).await?;
    if prev.as_deref() != Some(stamp) {
        insert_row(
            &mut tx,
            "startup",
            prev.as_deref(),
            stamp,
            &Outcome::NothingToApply,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(None)
}
