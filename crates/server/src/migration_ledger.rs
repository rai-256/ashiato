// SPDX-License-Identifier: AGPL-3.0-only
//! DB に当てた移行の版の記録を読み、自分の知る版と比べる（ST48 / design D1・D2・D3・D5）。
//!
//! 比べ方は**名前の集合**だけ（並びの位置・名前の時刻では比べない。ORDER は時刻順でない）。
//! 読む関数（`read_ledger`）は DB に何も書かない。当てる処理（`apply_migrations`）は記録を書く。

use sha2::{Digest as _, Sha256};
use sqlx::pool::PoolConnection;
use sqlx::{Connection as _, PgConnection, Postgres};
use std::collections::{BTreeMap, BTreeSet};

/// 当てる処理の錠の鍵（セッションの助言錠。試験の足場が取っていた鍵と同じ数。design D11）。
const LOCK_KEY: i64 = 4_820_251;

/// 当てた組み立ての版番号（design D8）。
pub const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 当てた組み立ての commit（`build.rs` が決める。design D8）。
pub const BUILD_COMMIT: &str = env!("ASHIATO_BUILD_COMMIT");

/// 当てる処理が止まった理由。呼び出し元（終了コードを決める側）が区別できる。
#[derive(Debug)]
pub enum MigrateError {
    /// DB にだけある版がある（自分より新しい DB）。何も書いていない。名前順。
    Ahead { unknown: Vec<UnknownVersion> },
    /// 記録が読めない。何も書いていない。
    Unreadable(String),
    /// 版の名前つきで、当てるのに失敗した（その版の取引は戻っている）。
    Version { name: String, source: sqlx::Error },
    /// 当てる直前の仕事（退避）が失敗した。何も当てていない。
    Before(anyhow::Error),
    /// 版を当てた後のアプリの役割への付与が失敗した（版の記録は当てた分だけ残る）。
    Grants(sqlx::Error),
    /// DB の問い合わせそのものの失敗。
    Db(sqlx::Error),
}

/// DB にだけある版と、それを当てた組み立て（design D6 の `db:` の行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVersion {
    pub name: String,
    pub build_version: String,
    pub build_commit: String,
}

/// 標準エラーに並べる `db:` の行の上限（design D6）。
const DB_LINES_MAX: usize = 20;

impl MigrateError {
    /// 版の食い違いで止めた（終了コード 3。design D6）ときの種別と標準エラーの本文。それ以外は `None`。
    /// 1 行目は `error: kind=<種別>`（配った版に焼き付く形）。
    pub fn mismatch_report(&self) -> Option<(&'static str, String)> {
        let selfline = format!("  self: {BUILD_VERSION} {BUILD_COMMIT}");
        match self {
            Self::Ahead { unknown } => {
                let mut out = String::from("error: kind=db_ahead\n");
                for u in unknown.iter().take(DB_LINES_MAX) {
                    out.push_str(&format!(
                        "  db: {} ({} {})\n",
                        u.name, u.build_version, u.build_commit
                    ));
                }
                if unknown.len() > DB_LINES_MAX {
                    // `db:` の頭は 1 版 1 行に取っておく（ST58 が機械で読む）。超えた分は別の頭で件数だけ
                    out.push_str(&format!("  more: {}\n", unknown.len() - DB_LINES_MAX));
                }
                out.push_str(&selfline);
                out.push_str("\n  この DB には、この組み立ての知らない移行が当たっている。この組み立てでは当てない・起動しない\n");
                Some(("db_ahead", out))
            }
            Self::Unreadable(why) => Some((
                "ledger_unreadable",
                format!(
                    "error: kind=ledger_unreadable\n{selfline}\n  この DB の版の記録を読めない（{why}）。何も書かずに止まった\n"
                ),
            )),
            _ => None,
        }
    }
}

impl std::fmt::Display for MigrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ahead { unknown } => write!(
                f,
                "DB に自分の知らない版がある（自分より新しい DB）: {}",
                unknown
                    .iter()
                    .map(|u| u.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Before(e) => write!(f, "{e:#}"),
            Self::Unreadable(why) => write!(f, "版の記録を読めない: {why}"),
            Self::Version { name, source } => {
                write!(f, "マイグレーション {name} の適用に失敗: {source}")
            }
            Self::Grants(e) => write!(f, "アプリの役割への付与に失敗: {e}"),
            Self::Db(e) => write!(f, "版の記録の問い合わせに失敗: {e}"),
        }
    }
}

impl std::error::Error for MigrateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Version { source, .. } | Self::Grants(source) | Self::Db(source) => Some(source),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for MigrateError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e)
    }
}

/// 版の記録の置き場を作る DDL（design D1）。`migrations/ORDER` の外。何度流しても同じ。
pub const BOOTSTRAP: &str = include_str!("migration_ledger.sql");

/// 記録の形の版。後の版が形を変えるときは上げる（古い版は知らない形として止まる。D5）。
pub const FORM: i32 = 1;

/// `migration_ledger.form` の列（名前, 型）。
const FORM_COLUMNS: &[(&str, &str)] = &[("only_row", "boolean"), ("form", "integer")];

/// `migration_ledger.applied` の列（名前, 型）。D5 の「読めない」は名前と型だけを見る。
const APPLIED_COLUMNS: &[(&str, &str)] = &[
    ("name", "text"),
    ("sha256", "text"),
    ("applied_at", "timestamp with time zone"),
    ("build_version", "text"),
    ("build_commit", "text"),
    ("before_ledger", "boolean"),
];

/// DB の版の記録の様子（design D3 の 3 分岐と D5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ledger {
    /// `migration_ledger` が無く、`core` も無い。
    Empty,
    /// `migration_ledger` が無く、`core` がある（ST48 より前の DB）。
    BeforeLedger,
    /// 記録のある DB。当たっている版の名前の集合。
    Recorded(BTreeSet<String>),
    /// 記録はあるが読めない（`form` の表か行が無い・`form` が 1 でない・`applied` の列が違う）。理由つき。
    Unreadable(String),
}

/// 知る版と DB の版の比べ結果（design D2）。`Ahead` を先に返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// DB にだけある名前がある（自分より新しい DB）。
    Ahead { unknown: Vec<String> },
    /// 自分にだけある名前がある（まだ当たっていない版）。
    Behind { missing: Vec<String> },
    /// 同じ集合。
    Even,
}

/// `known`（自分の知る版）と `ledger`（DB の版）の名前の集合を比べる。純粋。
pub fn compare(known: &BTreeSet<String>, ledger: &BTreeSet<String>) -> Verdict {
    let unknown: Vec<String> = ledger.difference(known).cloned().collect();
    if !unknown.is_empty() {
        return Verdict::Ahead { unknown };
    }
    let missing: Vec<String> = known.difference(ledger).cloned().collect();
    if !missing.is_empty() {
        return Verdict::Behind { missing };
    }
    Verdict::Even
}

async fn schema_exists(pool: &mut PgConnection, name: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)")
        .bind(name)
        .fetch_one(&mut *pool)
        .await
}

/// 表の列（名前, 型）を並びの順に引く。
async fn columns(
    pool: &mut PgConnection,
    table: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT column_name::text, data_type::text FROM information_schema.columns
          WHERE table_schema = 'migration_ledger' AND table_name = $1
          ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(&mut *pool)
    .await
}

fn same_columns(found: &[(String, String)], want: &[(&str, &str)]) -> bool {
    found.len() == want.len()
        && found
            .iter()
            .zip(want)
            .all(|(f, w)| f.0 == w.0 && f.1 == w.1)
}

/// DB の版の記録を読む。DB の問い合わせそのものの失敗は `Err`（記録が「読めない」のとは別）。
/// アプリの役割（`SELECT` だけ）でも所有者でも読める。
pub async fn read_ledger(pool: &sqlx::PgPool) -> Result<Ledger, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    read_ledger_on(&mut conn).await
}

/// `read_ledger` を 1 本の接続で行う（当てる処理が錠を持った接続で読む）。
async fn read_ledger_on(pool: &mut PgConnection) -> Result<Ledger, sqlx::Error> {
    if !schema_exists(pool, "migration_ledger").await? {
        return Ok(if schema_exists(pool, "core").await? {
            Ledger::BeforeLedger
        } else {
            Ledger::Empty
        });
    }
    let form_cols = columns(pool, "form").await?;
    if form_cols.is_empty() {
        return Ok(Ledger::Unreadable("form の表が無い".into()));
    }
    if !same_columns(&form_cols, FORM_COLUMNS) {
        return Ok(Ledger::Unreadable("form の列が決めた形と違う".into()));
    }
    let applied_cols = columns(pool, "applied").await?;
    if !same_columns(&applied_cols, APPLIED_COLUMNS) {
        return Ok(Ledger::Unreadable(
            "applied の表が無いか、列が決めた形と違う".into(),
        ));
    }
    let form: Option<i32> = sqlx::query_scalar("SELECT form FROM migration_ledger.form")
        .fetch_optional(&mut *pool)
        .await?;
    match form {
        None => return Ok(Ledger::Unreadable("form の行が無い".into())),
        Some(f) if f != FORM => {
            return Ok(Ledger::Unreadable(format!(
                "記録の形の版 {f} を知らない（知っているのは {FORM}）"
            )))
        }
        Some(_) => {}
    }
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM migration_ledger.applied")
        .fetch_all(&mut *pool)
        .await?;
    Ok(Ledger::Recorded(names.into_iter().collect()))
}

/// サーバの起動の検査（design D6・D17）。アプリの役割で読み、何も書かない。
/// 起動を止めるなら `Some((種別, 標準エラーの本文))`（終了コード 3）。揃っていれば `None`。
/// 記録が無い DB は behind（まだ全部が当たっていない）。指紋の違いは警告だけで止めない（D7）。
pub async fn startup_refusal(
    pool: &sqlx::PgPool,
    versions: &[(&str, &str)],
) -> Result<Option<(&'static str, String)>, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    let known: BTreeSet<String> = versions.iter().map(|v| v.0.to_string()).collect();
    let behind = |count: usize| {
        let selfline = format!("  self: {BUILD_VERSION} {BUILD_COMMIT}");
        Some((
            "db_behind",
            format!(
                "error: kind=db_behind\n{selfline}\n  まだ当たっていない版が {count} 個。先に `ashiato-server migrate` を\n"
            ),
        ))
    };
    match read_ledger_on(&mut conn).await? {
        Ledger::Empty | Ledger::BeforeLedger => Ok(behind(known.len())),
        Ledger::Unreadable(why) => Ok(MigrateError::Unreadable(why).mismatch_report()),
        Ledger::Recorded(names) => match compare(&known, &names) {
            Verdict::Ahead { unknown } => {
                let unknown = describe_unknown(&mut conn, &unknown).await?;
                Ok(MigrateError::Ahead { unknown }.mismatch_report())
            }
            Verdict::Behind { missing } => Ok(behind(missing.len())),
            Verdict::Even => {
                warn_changed_fingerprints(&mut conn, versions).await?;
                Ok(None)
            }
        },
    }
}

/// 中身の sha256（16 進の小文字）。
fn fingerprint(sql: &str) -> String {
    format!("{:x}", Sha256::digest(sql.as_bytes()))
}

/// `versions`（渡された並び）のうち記録に無い版だけを、1 版 1 取引で当てて記録する（design D3・D7・D11）。
///
/// 錠を取る → 記録を読む → ahead・読めないなら何も書かずに返す → bootstrap → 1 版ずつ当てて `INSERT` → 錠を外す。
/// 版が落ちたらその取引ごと戻り、その版も後の版も記録されない。付与は当てない（`apply_migrations_with` が当てる）。
pub async fn apply_migrations(
    conn: PoolConnection<Postgres>,
    versions: &[(&str, &str)],
) -> Result<(), MigrateError> {
    apply_migrations_with(conn, versions, || Ok(()), "").await
}

/// `apply_migrations` に、検査を通った後・何かを書く前に呼ぶ仕事（退避）と、版を当てた後に錠の中で流す
/// 付与の SQL（`grants`。空なら流さない）を足したもの（design D4 の 5〜7）。
/// ahead・読めないなら `before_apply` も `grants` も流さずに返す。`before_apply` が失敗したら何も当てずに `Before` を返す。
/// 錠を外せなかったら接続を pool に返さずに閉じる（セッションの助言錠は接続が閉じるまで残る）。
pub async fn apply_migrations_with(
    mut conn: PoolConnection<Postgres>,
    versions: &[(&str, &str)],
    before_apply: impl FnOnce() -> anyhow::Result<()>,
    grants: &str,
) -> Result<(), MigrateError> {
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(LOCK_KEY)
        .execute(&mut *conn)
        .await?;
    let result = apply_locked(&mut conn, versions, before_apply, grants).await;
    let unlocked: Result<bool, sqlx::Error> = sqlx::query_scalar("SELECT pg_advisory_unlock($1)")
        .bind(LOCK_KEY)
        .fetch_one(&mut *conn)
        .await;
    if !matches!(unlocked, Ok(true)) {
        // pool に戻すと錠を持った接続が生き残り、次の migrate が待ち続ける。pool から外して閉じる。
        // 閉じるのに失敗しても落とせば切れる。元の結果を優先する
        let _ = conn.detach().close().await;
    }
    result
}

async fn apply_locked(
    conn: &mut PgConnection,
    versions: &[(&str, &str)],
    before_apply: impl FnOnce() -> anyhow::Result<()>,
    grants: &str,
) -> Result<(), MigrateError> {
    let ledger = read_ledger_on(conn).await?;
    let (recorded, before_ledger) = match ledger {
        Ledger::Unreadable(why) => return Err(MigrateError::Unreadable(why)),
        Ledger::Empty => (BTreeSet::new(), false),
        Ledger::BeforeLedger => (BTreeSet::new(), true),
        Ledger::Recorded(names) => {
            let known: BTreeSet<String> = versions.iter().map(|v| v.0.to_string()).collect();
            if let Verdict::Ahead { unknown } = compare(&known, &names) {
                let unknown = describe_unknown(conn, &unknown).await?;
                return Err(MigrateError::Ahead { unknown });
            }
            (names, false)
        }
    };
    before_apply().map_err(MigrateError::Before)?;
    sqlx::raw_sql(BOOTSTRAP).execute(&mut *conn).await?;
    warn_changed_fingerprints(conn, versions).await?;
    for (name, sql) in versions {
        if recorded.contains(*name) {
            continue;
        }
        apply_one(conn, name, sql, before_ledger)
            .await
            .map_err(|source| MigrateError::Version {
                name: (*name).to_string(),
                source,
            })?;
    }
    if !grants.is_empty() {
        sqlx::raw_sql(grants)
            .execute(&mut *conn)
            .await
            .map_err(MigrateError::Grants)?;
    }
    Ok(())
}

/// 知らない版の名前に、当てた組み立ての版番号と commit を添える（名前順）。
async fn describe_unknown(
    conn: &mut PgConnection,
    names: &[String],
) -> Result<Vec<UnknownVersion>, sqlx::Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT name, build_version, build_commit FROM migration_ledger.applied
          WHERE name = ANY($1) ORDER BY name",
    )
    .bind(names)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(name, build_version, build_commit)| UnknownVersion {
            name,
            build_version,
            build_commit,
        })
        .collect())
}

/// 記録にある版の指紋が自分の中身と違うなら、当て直さず版の名前だけを警告に出す（design D7）。
async fn warn_changed_fingerprints(
    conn: &mut PgConnection,
    versions: &[(&str, &str)],
) -> Result<(), sqlx::Error> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT name, sha256 FROM migration_ledger.applied")
            .fetch_all(&mut *conn)
            .await?;
    let recorded: BTreeMap<String, String> = rows.into_iter().collect();
    for (name, sql) in versions {
        if recorded
            .get(*name)
            .is_some_and(|sha| *sha != fingerprint(sql))
        {
            // log-ok: 版の名前は配った移行のファイル名（時刻と slug）で、利用者のデータを含まない
            tracing::warn!(
                version = *name,
                "記録した指紋と中身が違う版がある（当て直さない）"
            );
        }
    }
    Ok(())
}

async fn apply_one(
    conn: &mut PgConnection,
    name: &str,
    sql: &str,
    before_ledger: bool,
) -> Result<(), sqlx::Error> {
    let mut tx = conn.begin().await?;
    sqlx::raw_sql(sql).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO migration_ledger.applied (name, sha256, build_version, build_commit, before_ledger)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(name)
    .bind(fingerprint(sql))
    .bind(BUILD_VERSION)
    .bind(BUILD_COMMIT)
    .bind(before_ledger)
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}
