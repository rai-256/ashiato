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
///
/// **読む変数を全部書く**（R3）。置き場だけを書いていたので、次のログオンで
/// 接続先が無いまま即終了し、**自動起動が 1 度も収集しなかった**。
/// 合言葉も平文で載る —— 置き場の未送信（題名と URL を含む）と同じ利用者の
/// プロファイルの中で、同じ読み手にしか読めない（design D7・仮）。
pub fn startup_script(exe: &std::path::Path, vars: &[(&str, String)]) -> String {
    let mut s = String::from(
        "@echo off\r\nrem ashiato C-02（ST07 / design D7）。消せば自動起動は止まる。\r\n\
         rem 合言葉を含む。このファイルを他人に渡さない。\r\n",
    );
    for (k, v) in vars {
        s.push_str(&format!("set \"{k}={v}\"\r\n"));
    }
    s.push_str(&format!("start \"\" /min \"{}\"\r\n", exe.display()));
    s
}

/// `.cmd` の `set "K=V"` 行を読み戻す（**書いた中身で起動できるか**を確かめるため）。
pub fn read_vars(script: &str) -> Vec<(String, String)> {
    script
        .lines()
        .filter_map(|l| l.trim().strip_prefix("set \"")?.strip_suffix('"'))
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// スタートアップフォルダ（Windows）。
#[cfg(windows)]
pub fn startup_dir() -> anyhow::Result<std::path::PathBuf> {
    let appdata = std::env::var("APPDATA").context("APPDATA が無い")?;
    Ok(std::path::PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup"))
}

/// 自動起動を仕込む（`--install-autostart`）。**冪等**（同じ内容で上書きする）。
#[cfg(windows)]
pub fn install(cfg: &crate::config::Config) -> anyhow::Result<std::path::PathBuf> {
    let exe = std::env::current_exe().context("自分の場所が分からない")?;
    let path = startup_dir()?.join(ENTRY_NAME);
    std::fs::write(&path, startup_script(&exe, &cfg.as_vars()))
        .context("スタートアップへ書けない")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// **書いた中身だけで起動の設定が揃う**（R3）。窓を出さない。
    #[test]
    fn startup_script_is_enough_to_start() {
        let dir = std::env::temp_dir().join("ashiato").display().to_string();
        let vars = [
            ("ASHIATO_BASE_URL", "http://127.0.0.1:8787".to_string()),
            ("ASHIATO_API_TOKEN", "tok-0123456789".to_string()),
            ("ASHIATO_USER_ID", uuid::Uuid::nil().to_string()),
            ("ASHIATO_DEVICE_ID", "pc-01".to_string()),
            ("ASHIATO_STATE_DIR", dir.clone()),
        ];
        let s = startup_script(Path::new(r"C:\apps\ashiato-collector-windows.exe"), &vars);
        assert!(s.contains(r"C:\apps\ashiato-collector-windows.exe"));
        assert!(s.contains("/min"), "窓を開いて起動している");
        assert!(s.contains("\r\n"), "cmd は CRLF で書く");

        // **素の環境で、この中身だけから設定が組める**
        let read = read_vars(&s);
        let cfg = crate::config::Config::from_lookup(|k| {
            read.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
        })
        .expect("自動起動の中身だけでは起動できない");
        assert_eq!(cfg.device_id, "pc-01");
        assert_eq!(cfg.state_dir.display().to_string(), dir);
        assert_eq!(
            read.len(),
            crate::config::VARS.len(),
            "書き漏れた変数がある"
        );
    }
}
