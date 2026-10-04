// SPDX-License-Identifier: AGPL-3.0-only
//! 取り込み器の環境変数。解釈をここへ集め、綴り違いで写しの扱いを変えない。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    // **既定は本人のホームから作り、相対パスは断る**（code-verify R67）。作業ディレクトリからの
    // 相対だと、サービスやタスクスケジューラから起動したとき写しが `C:\Windows\System32` の下に作られ、
    // 別の場所から起動し直すと目録の写しを読めず、印を置いた後の読み直しが永久に止まる。
    let home = get("USERPROFILE")
        .or_else(|| get("HOME"))
        .map(PathBuf::from);
    let local = get("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|h| h.join("AppData").join("Local")));
    let place = |name: &str, base: Option<&PathBuf>, default: &[&str]| -> anyhow::Result<PathBuf> {
        let path = match get(name) {
            Some(given) => PathBuf::from(given),
            None => {
                let Some(base) = base else {
                    // 取り込み器を起こさない（利用者が未設定）なら置き場は使わない。HTTP は止めない（D1）
                    if user_id.is_none() {
                        return Ok(PathBuf::new());
                    }
                    anyhow::bail!("{name} が無く、既定を作るホーム（USERPROFILE / HOME）も無い");
                };
                default.iter().fold(base.clone(), |p, part| p.join(part))
            }
        };
        absolute(name, path)
    };
    Ok(ArchiveConfig {
        inbox_dir: place(
            "ASHIATO_INBOX_DIR",
            home.as_ref(),
            &["Documents", "ashiato", "取り込み待ち"],
        )?,
        downloads_dir: place("ASHIATO_DOWNLOADS_DIR", home.as_ref(), &["Downloads"])?,
        copy_dir: place(
            "ASHIATO_ARCHIVE_COPY_DIR",
            local.as_ref(),
            &["ashiato", "archive-copies"],
        )?,
        keep_copies,
        user_id,
        scan_sec,
    })
}

fn absolute(name: &str, path: PathBuf) -> anyhow::Result<PathBuf> {
    if !Path::new(&path).is_absolute() {
        anyhow::bail!(
            "{name} は絶対パスで指定する（{}）。相対だと起動した場所によって写しを見失う",
            path.display()
        );
    }
    Ok(path)
}

/// プロセス環境を読む。
pub fn from_env() -> anyhow::Result<ArchiveConfig> {
    from_values(&std::env::vars().collect())
}
