// SPDX-License-Identifier: AGPL-3.0-only
//! 置き場の一覧と、2 回連続で変わらないファイルの選別（ST12 / D8）。

use super::config::ArchiveConfig;
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

/// 置き場を 1 回見て、前回から大きさと更新時刻が変わらないファイルだけ返す。
///
/// sighting は書き換えてよいキャッシュなので、一覧から消えたファイルの行もここで消す。
pub async fn scan_once(
    pool: &sqlx::PgPool,
    config: &ArchiveConfig,
    user_id: uuid::Uuid,
) -> anyhow::Result<Vec<ScanCandidate>> {
    scan_once_with_hasher(pool, config, user_id, &hash_file).await
}

/// `hash_file` を差し替えられる走査。ハッシュを毎回取り直さない性質を実測で守る。
pub async fn scan_once_with_hasher(
    pool: &sqlx::PgPool,
    config: &ArchiveConfig,
    user_id: uuid::Uuid,
    hash: &impl Fn(&Path) -> anyhow::Result<String>,
) -> anyhow::Result<Vec<ScanCandidate>> {
    let mut found = list_dir(&config.inbox_dir, false)?;
    found.extend(list_dir(&config.downloads_dir, true)?);
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
        let hash = if stable {
            match previous.as_ref().and_then(|(_, _, hash, _)| hash.clone()) {
                Some(hash) => hash,
                None => hash(&file.path)?,
            }
        } else {
            hash(&file.path)?
        };
        sqlx::query(
            "INSERT INTO core.archive_sighting (user_id, path, size_bytes, modified_at, sha256)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id, path) DO UPDATE
               SET size_bytes = EXCLUDED.size_bytes, modified_at = EXCLUDED.modified_at,
                   sha256 = EXCLUDED.sha256, seen_at = now()",
        )
        .bind(user_id)
        .bind(&path)
        .bind(file.size_bytes)
        .bind(file.modified_at)
        .bind(&hash)
        .execute(pool)
        .await?;
        if previous
            .as_ref()
            .and_then(|(_, _, _, retry_after)| *retry_after)
            .is_some_and(|retry_after| retry_after > Utc::now())
        {
            continue;
        }
        if stable {
            let already_read: bool = sqlx::query_scalar(
                "SELECT EXISTS(
                   SELECT 1 FROM core.archive_ledger
                    WHERE user_id = $1 AND sha256 = $2 AND parser_version = $3
                      AND outcome IN ('read', 'unreadable')
                 )",
            )
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

    if paths.is_empty() {
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
    Ok(candidates)
}

fn list_dir(dir: &Path, from_downloads: bool) -> anyhow::Result<Vec<FoundFile>> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("書庫の置き場を読めない: {}", dir.display()))?;
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file()
            || is_temporary(&path)
            || !is_candidate(&path, from_downloads)
        {
            continue;
        }
        let metadata = entry.metadata()?;
        // PostgreSQL の timestamptz はマイクロ秒精度。ここも同じ精度へ丸めないと、
        // 同じファイルを毎回「更新された」と誤認する。
        let modified: DateTime<Utc> = metadata.modified()?.into();
        let modified_at = DateTime::from_timestamp_micros(modified.timestamp_micros())
            .context("書庫の更新時刻が範囲外")?;
        found.push(FoundFile {
            path,
            from_downloads,
            size_bytes: i64::try_from(metadata.len()).context("書庫が大きすぎる")?,
            modified_at,
        });
    }
    Ok(found)
}

fn is_candidate(path: &Path, from_downloads: bool) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if from_downloads {
        return name.starts_with("takeout-") && name.ends_with(".zip");
    }
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("zip" | "json")
    )
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
