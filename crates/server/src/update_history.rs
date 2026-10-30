// SPDX-License-Identifier: AGPL-3.0-only
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
use sqlx::{Connection as _, PgConnection};
use std::collections::BTreeSet;

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
}

impl Outcome<'_> {
    fn name(&self) -> &'static str {
        match self {
            Self::Attempted => "attempted",
            Self::Applied(_) => "applied",
            Self::NothingToApply => "nothing_to_apply",
            Self::Failed { .. } => "failed",
            Self::BackupFailed => "backup_failed",
        }
    }

    fn applied(&self) -> &[String] {
        match self {
            Self::Applied(v) | Self::Failed { applied: v, .. } => v,
            _ => &[],
        }
    }
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
