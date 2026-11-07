// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
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
pub const fn entry_name(channel: ashiato_build_stamp::Channel) -> &'static str {
    match channel {
        ashiato_build_stamp::Channel::Dev => "ashiato-collector-dev.cmd",
        ashiato_build_stamp::Channel::Official => "ashiato-collector.cmd",
    }
}

/// スタートアップに置く中身。
///
/// **窓を出さずに起動する**（`start "" /min`）—— 常駐するものが毎回窓を開くと、
/// 利用者が閉じてしまい、閉じた期間の記録が後から作れない。
///
/// 接続先・資格情報・利用者は置き場のファイルから読む。台本には秘密を書かない。
pub fn startup_script(exe: &std::path::Path, vars: &[(&str, String)]) -> String {
    let mut s = String::from(
        "@echo off\r\nrem ashiato C-02（ST07 / design D7）。消せば自動起動は止まる。\r\n",
    );
    for (k, v) in vars {
        if matches!(*k, "ASHIATO_STATE_DIR" | "ASHIATO_DEVICE_ID") {
            s.push_str(&format!("set \"{k}={v}\"\r\n"));
        }
    }
    s.push_str(&format!("start \"\" /min \"{}\"\r\n", exe.display()));
    s
}

/// 新しい設定の自動起動台本。秘密と接続先はファイルへ移し、互換のため
/// 置き場と端末識別子だけは前の台本から引き継ぐ。
pub fn rewrite_for_setup(old: &str, exe: &std::path::Path) -> String {
    // 名前は固定の 2 つだけなので、読んだ文字列を寿命の長い名前へ移し替える（漏らさない。review/code.md R20）
    let vars: Vec<(&str, String)> = read_vars(old)
        .into_iter()
        .filter_map(|(name, value)| match name.as_str() {
            "ASHIATO_STATE_DIR" => Some(("ASHIATO_STATE_DIR", value)),
            "ASHIATO_DEVICE_ID" => Some(("ASHIATO_DEVICE_ID", value)),
            _ => None,
        })
        .collect();
    startup_script(exe, &vars)
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

/// 既存の自動起動台本を読み、前の版の秘密でない設定を引き継ぐ。
pub fn read_startup_script(
    channel: ashiato_build_stamp::Channel,
) -> anyhow::Result<Option<String>> {
    let Ok(appdata) = std::env::var("APPDATA") else {
        return Ok(None);
    };
    let path = std::path::PathBuf::from(appdata)
        .join(r"Microsoft\Windows\Start Menu\Programs\Startup")
        .join(entry_name(channel));
    match std::fs::read_to_string(path) {
        Ok(script) => Ok(Some(script)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn rewrite_existing_for_setup(
    exe: &std::path::Path,
    channel: ashiato_build_stamp::Channel,
) -> anyhow::Result<()> {
    let Some(old) = read_startup_script(channel)? else {
        return Ok(());
    };
    let appdata = std::env::var("APPDATA").map_err(|_| anyhow::anyhow!("APPDATA が無い"))?;
    let path = std::path::PathBuf::from(appdata)
        .join(r"Microsoft\Windows\Start Menu\Programs\Startup")
        .join(entry_name(channel));
    // 途中で落ちても台本が半分にならないよう、一時ファイルに書いて置き換える（review/code.md R20）
    crate::fsutil::atomic_write(&path, rewrite_for_setup(&old, exe).as_bytes())?;
    Ok(())
}

/// スタートアップフォルダ（Windows）。
#[cfg(windows)]
pub fn startup_dir() -> anyhow::Result<std::path::PathBuf> {
    let appdata = std::env::var("APPDATA").context("APPDATA が無い")?;
    Ok(std::path::PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup"))
}

/// 自動起動を仕込む（`--install-autostart`）。**冪等**（同じ内容で上書きする）。
#[cfg(windows)]
pub fn install(
    cfg: &crate::config::Config,
    channel: ashiato_build_stamp::Channel,
) -> anyhow::Result<std::path::PathBuf> {
    let exe = std::env::current_exe().context("自分の場所が分からない")?;
    let path = startup_dir()?.join(entry_name(channel));
    // 起動に渡された置き場と端末識別子を書く（開発版の `dev`・`-dev` を重ねない。ST54）。秘密は書かない（ST56）
    let vars = [
        (
            "ASHIATO_STATE_DIR",
            cfg.launch_state_dir.display().to_string(),
        ),
        ("ASHIATO_DEVICE_ID", cfg.launch_device_id.clone()),
    ];
    std::fs::write(&path, startup_script(&exe, &vars)).context("スタートアップへ書けない")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn st54_entry_name_uses_channel() {
        // Scenario: 開発版の PC の収集アプリは、開発版の自動起動の項目と差出人を使う
        assert_eq!(
            entry_name(ashiato_build_stamp::Channel::Dev),
            "ashiato-collector-dev.cmd"
        );
        assert_eq!(
            entry_name(ashiato_build_stamp::Channel::Official),
            "ashiato-collector.cmd"
        );
    }

    /// **書いた中身だけで起動の設定が揃う**（R3）。窓を出さない。
    #[test]
    fn startup_script_is_enough_to_start() {
        let tmp = tempfile::Builder::new()
            .prefix("ashiato-auto-")
            .tempdir()
            .expect("tempdir");
        let dir = tmp.path().join("ashiato").display().to_string();
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

        let read = read_vars(&s);
        assert_eq!(
            read.iter()
                .find(|(n, _)| n == "ASHIATO_DEVICE_ID")
                .map(|(_, v)| v.as_str()),
            Some("pc-01")
        );
        assert_eq!(
            read.iter()
                .find(|(n, _)| n == "ASHIATO_STATE_DIR")
                .map(|(_, v)| v.as_str()),
            Some(dir.as_str())
        );
        assert_eq!(read.len(), 2, "台本には秘密でない変数だけを書く");
    }

    // Scenario: 自動起動の台本は実行ファイルの場所だけを持つ
    #[test]
    fn st56_setupcmd_startup_script_does_not_contain_credentials() {
        let s = startup_script(
            Path::new(r"C:\apps\ashiato-collector-windows.exe"),
            &[
                ("ASHIATO_STATE_DIR", r"C:\state".into()),
                ("ASHIATO_DEVICE_ID", "pc-01".into()),
            ],
        );
        assert!(s.contains(r#"start "" /min "C:\apps\ashiato-collector-windows.exe""#));
        assert!(!s.contains("ASHIATO_BASE_URL"));
        assert!(!s.contains("ASHIATO_API_TOKEN"));
        assert!(!s.contains("ASHIATO_USER_ID"));
    }

    // Scenario: 前の形の台本は、設定を保存すると書き換わる
    #[test]
    fn st56_setupcmd_startup_script_keeps_only_portable_state_values() {
        let old = startup_script(
            Path::new(r"C:\apps\ashiato-collector-windows.exe"),
            &[
                ("ASHIATO_BASE_URL", "https://old.example".into()),
                ("ASHIATO_API_TOKEN", "old-token".into()),
                ("ASHIATO_USER_ID", uuid::Uuid::nil().to_string()),
                ("ASHIATO_STATE_DIR", r"C:\state".into()),
                ("ASHIATO_DEVICE_ID", "pc-01".into()),
            ],
        );
        let rewritten =
            rewrite_for_setup(&old, Path::new(r"C:\apps\ashiato-collector-windows.exe"));
        assert!(rewritten.contains(r#"ASHIATO_STATE_DIR=C:\state"#));
        assert!(rewritten.contains(r#"ASHIATO_DEVICE_ID=pc-01"#));
        assert!(!rewritten.contains("old-token"));
        assert!(!rewritten.contains("ASHIATO_BASE_URL"));
        assert!(!rewritten.contains("ASHIATO_API_TOKEN"));
        assert!(!rewritten.contains("ASHIATO_USER_ID"));
    }
}
