//! 移行を当てる直前の退避（製造準備 A-2「適用の直前に自動でバックアップを取る」。2026-10-11 の整合の確認 K2）。
//!
//! `ashiato-server migrate` は全部の移行を毎回当て直す（当てた版を数えていない）ので、**毎回**退避する。
//! 退避に失敗したら移行を当てない —— 個人のデータは復旧できない（A-2 の可逆性「低」）。
//!
//! - `MIGRATE_BACKUP_DIR` 退避の置き場（既定 `backups/pre-migrate`。作業ディレクトリからの相対）。拠点の外へは出さない（NFR-15）
//! - `PG_DUMP`            `pg_dump` の実行ファイル（既定は PATH の `pg_dump`）
//! - `MIGRATE_BACKUP=off` 退避しない。**使い捨ての DB（縦串・CI・確認バッチ）だけ**が明示して付ける
//!
//! 合言葉は `pg_dump` の引数に載せず `PGPASSWORD` で渡す（同じ PC のプロセス一覧に出さない）。

use anyhow::{bail, Context};
use sqlx::postgres::PgConnectOptions;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// 退避の設定。環境変数から読む（`from_env`）か、試験が組み立てる。
#[derive(Clone, Debug)]
pub struct BackupConfig {
    pub enabled: bool,
    pub dir: PathBuf,
    pub pg_dump: PathBuf,
}

impl BackupConfig {
    pub fn from_env() -> Self {
        let off = std::env::var("MIGRATE_BACKUP").is_ok_and(|v| v.eq_ignore_ascii_case("off"));
        Self {
            enabled: !off,
            dir: std::env::var("MIGRATE_BACKUP_DIR")
                .map_or_else(|_| PathBuf::from("backups/pre-migrate"), PathBuf::from),
            pg_dump: std::env::var("PG_DUMP")
                .map_or_else(|_| PathBuf::from("pg_dump"), PathBuf::from),
        }
    }
}

/// `pg_dump` に渡す接続の部品。合言葉は引数ではなく環境変数に分ける。
#[derive(Debug, PartialEq, Eq)]
pub struct DumpTarget {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
    pub password: Option<String>,
}

/// 接続の URL を部品に分ける。合言葉は sqlx が返さないので URL の userinfo から読む（`%` の符号化を戻す）。
pub fn dump_target(url: &str) -> anyhow::Result<DumpTarget> {
    let opts = PgConnectOptions::from_str(url).context("接続の URL を読めない")?;
    let userinfo = url
        .split_once("://")
        .map(|(_, rest)| rest.split(['/', '?']).next().unwrap_or(""))
        .and_then(|authority| authority.rsplit_once('@').map(|(u, _)| u));
    let password = userinfo
        .and_then(|u| u.split_once(':'))
        .map(|(_, p)| percent_decode(p));
    Ok(DumpTarget {
        host: opts.get_host().to_string(),
        port: opts.get_port(),
        user: opts.get_username().to_string(),
        database: opts.get_database().unwrap_or("postgres").to_string(),
        password,
    })
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 退避して、置いたファイルを返す。無効なら `None`。失敗したら Err（呼び出し側は移行を当てない）。
pub fn backup_before_migrate(
    url: &str,
    cfg: &BackupConfig,
    stamp: &str,
) -> anyhow::Result<Option<PathBuf>> {
    if !cfg.enabled {
        return Ok(None);
    }
    let t = dump_target(url)?;
    std::fs::create_dir_all(&cfg.dir)
        .with_context(|| format!("退避の置き場を作れない: {}", cfg.dir.display()))?;
    let file = cfg.dir.join(format!("pre-migrate-{stamp}.dump"));
    let mut cmd = std::process::Command::new(&cfg.pg_dump);
    cmd.args([
        "--no-password",
        "--format=custom",
        "--host",
        &t.host,
        "--port",
        &t.port.to_string(),
    ])
    .args(["--username", &t.user, "--dbname", &t.database, "--file"])
    .arg(&file)
    .env_remove("PGPASSWORD");
    if let Some(p) = &t.password {
        cmd.env("PGPASSWORD", p);
    }
    let out = cmd.output().with_context(|| {
        format!(
            "{} を起動できない（PG_DUMP で場所を指すか、使い捨ての DB なら MIGRATE_BACKUP=off）",
            cfg.pg_dump.display()
        )
    })?;
    if !out.status.success() || !non_empty(&file) {
        let _ = std::fs::remove_file(&file); // 途中の空の退避を残さない（退避があると読み違える）
        bail!(
            "移行の前の退避に失敗した（rc={:?}）。移行は当てていない",
            out.status.code()
        );
    }
    Ok(Some(file))
}

fn non_empty(p: &Path) -> bool {
    std::fs::metadata(p).is_ok_and(|m| m.len() > 0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn url_splits_into_parts_and_keeps_the_password_out_of_args() {
        let t = dump_target("postgres://ashiato_owner:p%40ss%3Aw@127.0.0.1:55432/ashiato").unwrap();
        assert_eq!(
            t,
            DumpTarget {
                host: "127.0.0.1".into(),
                port: 55432,
                user: "ashiato_owner".into(),
                database: "ashiato".into(),
                password: Some("p@ss:w".into()),
            }
        );
    }

    #[cfg(unix)]
    fn fake_pg_dump(dir: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join("pg_dump");
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ashiato-mb-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // Scenario: 移行を当てる前に退避が置かれ、合言葉は引数でなく環境変数で渡る
    #[cfg(unix)]
    #[test]
    fn backup_writes_a_dump_and_passes_the_password_by_env() {
        let d = tmp("ok");
        // 引数をそのまま記録し、--file の次の引数へ PGPASSWORD を書く
        let fake = fake_pg_dump(
            &d,
            r#"echo "$@" > "$(dirname "$0")/args"; while [ "$1" != "--file" ]; do shift; done; printf '%s' "$PGPASSWORD" > "$2""#,
        );
        let cfg = BackupConfig {
            enabled: true,
            dir: d.join("out"),
            pg_dump: fake,
        };
        let f = backup_before_migrate(
            "postgres://o:secret@127.0.0.1:5432/a",
            &cfg,
            "20261011T000000Z",
        )
        .unwrap()
        .unwrap();
        assert_eq!(f, d.join("out/pre-migrate-20261011T000000Z.dump"));
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "secret");
        let args = std::fs::read_to_string(d.join("args")).unwrap();
        assert!(!args.contains("secret"), "合言葉が引数に載っている: {args}");
        assert!(args.contains("--format=custom") && args.contains("--dbname a"));
    }

    // Scenario: 退避に失敗したら移行を当てない
    #[cfg(unix)]
    #[test]
    fn failed_or_empty_backup_is_an_error() {
        let d = tmp("ng");
        let cfg = BackupConfig {
            enabled: true,
            dir: d.join("out"),
            pg_dump: fake_pg_dump(&d, "exit 1"),
        };
        assert!(backup_before_migrate("postgres://o:x@127.0.0.1/a", &cfg, "s").is_err());
        let cfg = BackupConfig {
            enabled: true,
            dir: d.join("out"),
            pg_dump: fake_pg_dump(&d, "exit 0"),
        };
        assert!(
            backup_before_migrate("postgres://o:x@127.0.0.1/a", &cfg, "s").is_err(),
            "空の退避を成功と読んだ"
        );
        assert!(
            !d.join("out/pre-migrate-s.dump").exists(),
            "失敗した退避のファイルが残っている"
        );
        let cfg = BackupConfig {
            enabled: true,
            dir: d.join("out"),
            pg_dump: d.join("no-such-pg_dump"),
        };
        assert!(backup_before_migrate("postgres://o:x@127.0.0.1/a", &cfg, "s").is_err());
    }

    #[test]
    fn off_skips_without_running_anything() {
        let cfg = BackupConfig {
            enabled: false,
            dir: PathBuf::from("/nonexistent"),
            pg_dump: PathBuf::from("/nonexistent"),
        };
        assert!(
            backup_before_migrate("postgres://o:x@127.0.0.1/a", &cfg, "s")
                .unwrap()
                .is_none()
        );
    }
}
