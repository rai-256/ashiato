// SPDX-License-Identifier: AGPL-3.0-only
//! 実機の OS から読んだ値を**どう解釈するか**。OS を触らないので Linux でも確かめられる
//! （review/code.md R9 —— `platform.rs` は 0 テストで、判断まで実機にしか無かった）。
use crate::engine::{IdleRead, UrlRead};

/// ロック画面を出しているプロセス（Windows 10 / 11）。
pub const LOCK_SCREEN: [&str; 2] = ["lockapp.exe", "logonui.exe"];

/// このプロセス名はロック画面か。**大文字小文字を無視する。**
pub fn is_lock_screen(process_name: &str) -> bool {
    LOCK_SCREEN
        .iter()
        .any(|l| l.eq_ignore_ascii_case(process_name))
}

/// 実行ファイルのパスからプロセス名を取る。**取れなければ `None`**（空文字にしない）。
///
/// 空文字にすると、プロセス名で指した除外の規則が当たらず、**除外したかった窓の
/// 題名が読み取り失敗を理由に記録に載る**（review/code.md R29）。
pub fn process_name_of(path: &std::path::Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().to_string();
    (!name.trim().is_empty()).then_some(name)
}

/// アドレスバーの読み取り結果を解釈する（design D12 / review/code.md R14）。
///
/// **空文字は「読めなかった」**。アドレスバーが本当に空の瞬間は短く、
/// 空を「読めた」に倒すと、UI Automation が壊れていても生存信号が満点のまま
/// URL だけが消える（取れなかった URL は後から作れない）。
pub fn url_from_value<E>(value: Result<String, E>) -> UrlRead {
    match value {
        Ok(v) if !v.is_empty() => UrlRead::Read(v),
        _ => UrlRead::Unavailable,
    }
}

/// 最後の入力からの経過時間を解釈する。**変換できない値は「読めなかった」**
/// （0 秒に倒すと「ずっと操作していた」になり、離席が黙って立たない。R24）。
pub fn idle_from<E>(value: Result<std::time::Duration, E>) -> IdleRead {
    match value.ok().and_then(|d| chrono::Duration::from_std(d).ok()) {
        Some(d) => IdleRead::Elapsed(d),
        None => IdleRead::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ロック画面の判定（実機の前景がこの名前で見えるかは確認バッチで見る）。
    #[test]
    fn lock_screen_names_match_ignoring_case() {
        assert!(is_lock_screen("LockApp.exe"));
        assert!(is_lock_screen("LogonUI.exe"));
        assert!(!is_lock_screen("explorer.exe"));
        assert!(!is_lock_screen(""));
    }

    /// プロセス名が取れないときは空文字にしない。
    #[test]
    fn process_name_is_none_not_empty() {
        assert_eq!(process_name_of(std::path::Path::new("")), None);
        assert_eq!(
            process_name_of(std::path::Path::new("/opt/b.exe")).as_deref(),
            Some("b.exe")
        );
    }

    /// 空文字の URL は「読めなかった」（R14）。
    #[test]
    fn empty_url_is_unavailable() {
        assert_eq!(
            url_from_value::<()>(Ok("example.com/a".into())),
            UrlRead::Read("example.com/a".into())
        );
        assert_eq!(
            url_from_value::<()>(Ok(String::new())),
            UrlRead::Unavailable
        );
        assert_eq!(url_from_value(Err(())), UrlRead::Unavailable);
    }

    /// 経過時間が読めない・変換できないときは 0 秒に倒さない（R24）。
    #[test]
    fn unreadable_idle_is_not_zero() {
        assert_eq!(
            idle_from::<()>(Ok(std::time::Duration::from_secs(7))),
            IdleRead::Elapsed(chrono::Duration::seconds(7))
        );
        assert_eq!(idle_from(Err(())), IdleRead::Unavailable);
        assert_eq!(
            idle_from::<()>(Ok(std::time::Duration::MAX)),
            IdleRead::Unavailable
        );
    }
}
