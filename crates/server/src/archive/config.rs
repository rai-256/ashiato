// SPDX-License-Identifier: AGPL-3.0-only
//! 取り込み器の環境変数。解釈をここへ集め、綴り違いで写しの扱いを変えない。

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ArchiveConfig {
    pub inbox_dir: PathBuf,
    pub downloads_dir: PathBuf,
    pub copy_dir: PathBuf,
    pub keep_copies: bool,
    pub user_id: Option<uuid::Uuid>,
    pub scan_sec: u64,
}

/// テストと起動時で同じ規則を使う。
pub fn from_values(values: &BTreeMap<String, String>) -> anyhow::Result<ArchiveConfig> {
    let get = |name: &str| values.get(name).map(String::as_str);
    let keep_copies = match get("ASHIATO_ARCHIVE_KEEP_COPIES") {
        None | Some("true") => true,
        Some("false") => false,
        Some(_) => anyhow::bail!("ASHIATO_ARCHIVE_KEEP_COPIES は true または false で指定する"),
    };
    let user_id = get("ASHIATO_ARCHIVE_USER_ID")
        .map(uuid::Uuid::parse_str)
        .transpose()?;
    let scan_sec = get("ASHIATO_ARCHIVE_SCAN_SEC")
        .map(str::parse)
        .transpose()?
        .unwrap_or(120);
    if scan_sec == 0 {
        anyhow::bail!("ASHIATO_ARCHIVE_SCAN_SEC は 1 以上で指定する");
    }
    Ok(ArchiveConfig {
        inbox_dir: get("ASHIATO_INBOX_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("Documents/ashiato/取り込み待ち")),
        downloads_dir: get("ASHIATO_DOWNLOADS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("Downloads")),
        copy_dir: get("ASHIATO_ARCHIVE_COPY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("AppData/Local/ashiato/archive-copies")),
        keep_copies,
        user_id,
        scan_sec,
    })
}

/// プロセス環境を読む。
pub fn from_env() -> anyhow::Result<ArchiveConfig> {
    from_values(&std::env::vars().collect())
}
