// SPDX-License-Identifier: AGPL-3.0-only
//! ログオン時の自動起動（design D7・**仮**）。
//!
//! NFR-12 は「収集に要する手作業は **Google 系のみ** 2 か月に 1 回まで。
//! **他のすべてのソースは手作業を要さない**」。自動起動しなければ再起動のたびに
//! 手で起動することになり、**起動を忘れた期間の記録は後から作れない。**
//!
//! # なぜ「スタートアップ」フォルダか
//!
//! レジストリでも走らせられるが、そちらは**別の crate（`winreg`）か unsafe** を要する。
//! スタートアップフォルダへ 1 つ置くのは**ただのファイル書き込み**で、
//! 利用者が中身を読んで消せる（何が仕込まれたかが見える側）。
#[cfg(windows)]
use anyhow::Context as _;

/// 置くファイルの名前。
pub const ENTRY_NAME: &str = "ashiato-collector.cmd";

/// スタートアップに置く中身。
///
/// **窓を出さずに起動する**（`start "" /min`）—— 常駐するものが毎回窓を開くと、
/// 利用者が閉じてしまい、閉じた期間の記録が後から作れない。
pub fn startup_script(exe: &std::path::Path, state_dir: &std::path::Path) -> String {
    format!(
        "@echo off\r\n\
         rem ashiato C-02（ST07 / design D7）。消せば自動起動は止まる。\r\n\
         set ASHIATO_STATE_DIR={}\r\n\
         start \"\" /min \"{}\"\r\n",
        state_dir.display(),
        exe.display()
    )
}

/// スタートアップフォルダ（Windows）。
#[cfg(windows)]
pub fn startup_dir() -> anyhow::Result<std::path::PathBuf> {
    let appdata = std::env::var("APPDATA").context("APPDATA が無い")?;
    Ok(std::path::PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup"))
}

/// 自動起動を仕込む（`--install-autostart`）。**冪等**（同じ内容で上書きする）。
#[cfg(windows)]
pub fn install(state_dir: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let exe = std::env::current_exe().context("自分の場所が分からない")?;
    let path = startup_dir()?.join(ENTRY_NAME);
    std::fs::write(&path, startup_script(&exe, state_dir)).context("スタートアップへ書けない")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// 中身に**置き場と実行ファイル**が入り、窓を出さない。
    #[test]
    fn startup_script_has_exe_and_state_dir() {
        let s = startup_script(
            Path::new(r"C:\apps\ashiato-collector-windows.exe"),
            Path::new(r"C:\Users\me\.ashiato"),
        );
        assert!(s.contains(r"C:\apps\ashiato-collector-windows.exe"));
        assert!(s.contains(r"ASHIATO_STATE_DIR=C:\Users\me\.ashiato"));
        assert!(s.contains("/min"), "窓を開いて起動している");
        // cmd は CRLF で書く（LF だけだと古い cmd が 1 行として読む）
        assert!(s.contains("\r\n"));
    }
}
