// SPDX-License-Identifier: AGPL-3.0-only
//! 置き場の一覧と、2 回連続で変わらないファイルの選別（ST12 / D8）。

use super::config::ArchiveConfig;
use super::worker::{record_unhashable, REMEMBERED_OUTCOME};
use anyhow::Context as _;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// 読み手へ渡す、安定した書庫。置き場の種類は本人のファイルを動かす規則に使う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanCandidate {
    pub path: PathBuf,
    pub from_downloads: bool,
    pub sha256: String,
    pub disposition: ScanDisposition,
}

/// 読み手が中身を開くか、台帳へ既読として残すだけかを分ける。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanDisposition {
    Read,
    AlreadyRead,
}

#[derive(Debug)]
struct FoundFile {
    path: PathBuf,
    from_downloads: bool,
    size_bytes: i64,
    modified_at: DateTime<Utc>,
}

type PreviousSighting = (i64, DateTime<Utc>, Option<String>, Option<DateTime<Utc>>);

/// 読めなかった置き場の、生存信号の `blockers` に載せる名前（D8 / R34）。
pub const DEDICATED_INBOX_UNREADABLE: &str = "dedicated_inbox_unreadable";
pub const DOWNLOADS_UNREADABLE: &str = "downloads_unreadable";

/// 置き場を 1 回見て、前回から大きさと更新時刻が変わらないファイルと、読めなかった置き場を返す。
///
/// **2 つの置き場を別々に読む**（D8）。片方が読めなくても、もう片方の候補は返す。
/// sighting は書き換えてよいキャッシュなので、一覧から消えたファイルの行もここで消す。
pub async fn scan_once(
    pool: &sqlx::PgPool,
    config: &ArchiveConfig,
    user_id: uuid::Uuid,
) -> anyhow::Result<(Vec<ScanCandidate>, Vec<&'static str>)> {
    scan_once_with_hasher(pool, config, user_id, &hash_file).await
}

/// `hash_file` を差し替えられる走査。ハッシュを毎回取り直さない性質を実測で守る。
pub async fn scan_once_with_hasher(
    pool: &sqlx::PgPool,
    config: &ArchiveConfig,
    user_id: uuid::Uuid,
    hash: &impl Fn(&Path) -> anyhow::Result<String>,
) -> anyhow::Result<(Vec<ScanCandidate>, Vec<&'static str>)> {
    let mut unreadable = Vec::new();
    let mut found = Vec::new();
    for (dir, from_downloads, blocker) in [
        (&config.inbox_dir, false, DEDICATED_INBOX_UNREADABLE),
        (&config.downloads_dir, true, DOWNLOADS_UNREADABLE),
    ] {
        match list_dir(dir, from_downloads) {
            Ok((files, failed)) => {
                found.extend(files);
                // **1 ファイルが読めなくても走査を止めない**（final review R2）。そのファイルだけを
                // 台帳に残し、残りのファイルともう片方の置き場は読み続ける。
                for path in failed {
                    note_unhashable(pool, user_id, &path, None, from_downloads).await?;
                }
            }
            Err(error) => {
                tracing::warn!(kind = blocker, error = %error, "書庫の置き場を読めない");
                unreadable.push(blocker);
            }
        }
    }
    let paths: BTreeSet<String> = found
        .iter()
        .map(|file| file.path.to_string_lossy().into_owned())
        .collect();

    let mut candidates = Vec::new();
    for file in found {
        let path = file.path.to_string_lossy().into_owned();
        let previous: Option<PreviousSighting> = sqlx::query_as(
            "SELECT size_bytes, modified_at, sha256, retry_after
               FROM core.archive_sighting WHERE user_id = $1 AND path = $2",
        )
        .bind(user_id)
        .bind(&path)
        .fetch_optional(pool)
        .await?;
        let stable = previous.as_ref().is_some_and(|(size, modified, _, _)| {
            *size == file.size_bytes && *modified == file.modified_at
        });
        let cached = if stable {
            previous.as_ref().and_then(|(_, _, hash, _)| hash.clone())
        } else {
            None
        };
        let hash = match cached {
            Some(hash) => hash,
            None => match hash(&file.path) {
                Ok(hash) => hash,
                // **読み出せない 1 ファイルで走査全体を落とさない**（final review R2）。中身の sha256 が
                // 取れないので、台帳の鍵はパスと大きさと更新時刻から作る（design D20（仮））。
                // **初めて見た・大きさか更新時刻が変わったときは、まだ失敗と決めない**（/code-review R28）。
                // コピーの途中や一時のロックで読めないだけのファイルに、読み出せなかった行を残さない。
                // sighting に大きさと更新時刻だけ（sha256 なし）を書き、次の走査でも変わらず読めなければ台帳に残す。
                Err(_) => {
                    sqlx::query(
                        "INSERT INTO core.archive_sighting (user_id, path, size_bytes, modified_at, sha256)
                         VALUES ($1, $2, $3, $4, NULL)
                         ON CONFLICT (user_id, path) DO UPDATE
                           SET size_bytes = EXCLUDED.size_bytes, modified_at = EXCLUDED.modified_at,
                               sha256 = NULL, seen_at = now()",
                    )
                    .bind(user_id)
                    .bind(&path)
                    .bind(file.size_bytes)
                    .bind(file.modified_at)
                    .execute(pool)
                    .await?;
                    if stable {
                        note_unhashable(
                            pool,
                            user_id,
                            &file.path,
                            Some((file.size_bytes, file.modified_at)),
                            file.from_downloads,
                        )
                        .await?;
                    }
                    continue;
                }
            },
        };
        // 同じ場所に中身の違う書庫が置き直されたら、前の書庫の失敗の回数と待ちを持ち越さない（design D8 / M7）。
        let replaced = previous
            .as_ref()
            .and_then(|(_, _, previous_hash, _)| previous_hash.as_deref())
            .is_some_and(|previous_hash| previous_hash != hash);
        sqlx::query(
            "INSERT INTO core.archive_sighting (user_id, path, size_bytes, modified_at, sha256)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id, path) DO UPDATE
               SET size_bytes = EXCLUDED.size_bytes, modified_at = EXCLUDED.modified_at,
                   sha256 = EXCLUDED.sha256, seen_at = now(),
                   consecutive_failures = CASE WHEN $6 THEN 0
                                               ELSE core.archive_sighting.consecutive_failures END,
                   retry_after = CASE WHEN $6 THEN NULL
                                      ELSE core.archive_sighting.retry_after END",
        )
        .bind(user_id)
        .bind(&path)
        .bind(file.size_bytes)
        .bind(file.modified_at)
        .bind(&hash)
        .bind(replaced)
        .execute(pool)
        .await?;
        if !replaced
            && previous
                .as_ref()
                .and_then(|(_, _, _, retry_after)| *retry_after)
                .is_some_and(|retry_after| retry_after > Utc::now())
        {
            continue;
        }
        if stable {
            // **確認待ちの書庫も覚えている**（D16）。`read` の行を持たない確認待ちだけの書庫を
            // 読みへ回していたときは、120 秒ごとに書庫全体を展開し直していた（final review R52）。
            // **読み出せなかった行（`file_unreadable`）は覚えない**（design D20（仮）。final review R1）。
            let already_read: bool = sqlx::query_scalar(&format!(
                "SELECT EXISTS(
                   SELECT 1 FROM core.archive_ledger
                    WHERE user_id = $1 AND sha256 = $2 AND parser_version = $3
                      AND {REMEMBERED_OUTCOME}
                 )"
            ))
            .bind(user_id)
            .bind(&hash)
            .bind(super::PARSER_VERSION)
            .fetch_one(pool)
            .await?;
            candidates.push(ScanCandidate {
                path: file.path,
                from_downloads: file.from_downloads,
                sha256: hash,
                disposition: if already_read {
                    ScanDisposition::AlreadyRead
                } else {
                    ScanDisposition::Read
                },
            });
        }
    }

    if !unreadable.is_empty() {
        // 読めなかった置き場の行まで消さない（読めるようになった日に覚えが残る）。**読めた置き場の行は消す**
        // （/code-review R32）。残すと、消して置き直した同じ大きさ・更新時刻のファイルが初めての走査で安定に数えられる。
        let readable: Vec<&Path> = [
            (config.inbox_dir.as_path(), DEDICATED_INBOX_UNREADABLE),
            (config.downloads_dir.as_path(), DOWNLOADS_UNREADABLE),
        ]
        .into_iter()
        .filter(|(_, blocker)| !unreadable.contains(blocker))
        .map(|(dir, _)| dir)
        .collect();
        let known: Vec<String> =
            sqlx::query_scalar("SELECT path FROM core.archive_sighting WHERE user_id = $1")
                .bind(user_id)
                .fetch_all(pool)
                .await?;
        let gone: Vec<String> = known
            .into_iter()
            .filter(|path| {
                !paths.contains(path)
                    && Path::new(path)
                        .parent()
                        .is_some_and(|parent| readable.contains(&parent))
            })
            .collect();
        if !gone.is_empty() {
            sqlx::query("DELETE FROM core.archive_sighting WHERE user_id = $1 AND path = ANY($2)")
                .bind(user_id)
                .bind(&gone)
                .execute(pool)
                .await?;
        }
    } else if paths.is_empty() {
        sqlx::query("DELETE FROM core.archive_sighting WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
    } else {
        let kept: Vec<&str> = paths.iter().map(String::as_str).collect();
        sqlx::query(
            "DELETE FROM core.archive_sighting WHERE user_id = $1 AND NOT (path = ANY($2))",
        )
        .bind(user_id)
        .bind(&kept)
        .execute(pool)
        .await?;
    }
    Ok((candidates, unreadable))
}

/// 置き場を 1 つ一覧する。**置き場そのものが読めない（一覧が取れない）ときだけ `Err`**。
///
/// 中の 1 件が読めないときは、そのファイルを 2 つ目の戻り値に積んで残りを続ける（final review R2）。
/// 包括の `From` で `?` を許していたときは、1 件の `metadata` の失敗が走査全体の失敗になり、
/// もう片方の置き場まで止まっていた。名前も取れない項目（`read_dir` の項目そのものの失敗）は台帳に残せないのでログだけ。
fn list_dir(
    dir: &Path,
    from_downloads: bool,
) -> Result<(Vec<FoundFile>, Vec<PathBuf>), std::io::Error> {
    let entries = std::fs::read_dir(dir)?;
    let mut found = Vec::new();
    let mut failed = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            tracing::warn!(kind = "archive_list_entry", "置き場の項目を読めない");
            continue;
        };
        let path = entry.path();
        if is_temporary(&path) || !is_candidate(&path, from_downloads) {
            continue;
        }
        match entry.file_type() {
            Ok(file_type) if !file_type.is_file() => continue,
            Ok(_) => {}
            Err(_) => {
                failed.push(path);
                continue;
            }
        }
        match found_file(&entry, from_downloads) {
            Ok(file) => found.push(file),
            Err(_) => failed.push(path),
        }
    }
    Ok((found, failed))
}

fn found_file(entry: &std::fs::DirEntry, from_downloads: bool) -> anyhow::Result<FoundFile> {
    let metadata = entry.metadata()?;
    // PostgreSQL の timestamptz はマイクロ秒精度。ここも同じ精度へ丸めないと、
    // 同じファイルを毎回「更新された」と誤認する。
    let modified: DateTime<Utc> = metadata.modified()?.into();
    let modified_at = DateTime::from_timestamp_micros(modified.timestamp_micros())
        .context("書庫の更新時刻が範囲外")?;
    Ok(FoundFile {
        path: entry.path(),
        from_downloads,
        size_bytes: i64::try_from(metadata.len()).context("書庫が大きすぎる")?,
        modified_at,
    })
}

/// 読み出せないファイルを台帳に残す。ログは台帳に新しく足した 1 回だけ（走査のたびに出さない）。
async fn note_unhashable(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    path: &Path,
    seen: Option<(i64, DateTime<Utc>)>,
    from_downloads: bool,
) -> anyhow::Result<()> {
    if record_unhashable(pool, user_id, path, seen, from_downloads).await? {
        tracing::warn!(
            kind = super::open::FILE_UNREADABLE,
            "置き場のファイルを読み出せない"
        );
    }
    Ok(())
}

fn is_candidate(path: &Path, from_downloads: bool) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if from_downloads {
        // **ダウンロードのフォルダは本人の他のファイルが混ざる**ので、Takeout の
        // 名前の zip だけを拾う（spec「ダウンロードのフォルダの他のファイルは読まれない」）。
        return name.starts_with("takeout-") && name.ends_with(".zip");
    }
    // **専用のフォルダは本人が書庫だけを置く場所**なので、読める形でなくても拾う
    // —— 拾わないと `.tgz` が走査の対象にすらならず、「読めなかった書庫」として
    // 台帳にも画面にも出ないまま消える（R4）。読めるかどうかは読み手が決める。
    true
}

fn is_temporary(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    name.ends_with(".part")
        || name.ends_with(".tmp")
        || name.starts_with('.')
        || name.ends_with('~')
}

fn hash_file(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("一覧で見つけた書庫を読めない: {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
