// SPDX-License-Identifier: AGPL-3.0-only
//! どのプロセスを「ブラウザ」として扱うか。
//!
//! **URL を読むのは前景がブラウザのときだけ**（design D5 の反転条件 /
//! Risks の「UI Automation が重い」）。すべての前景で UI Automation を叩くと
//! 描画が重くなる報告がある。
//!
//! **足せるようにしてある**（`ASHIATO_BROWSERS`）—— 一覧に無いブラウザを
//! 使っている間、URL は「取れない」ではなく**そもそも読まれない**ので、
//! 取っていない URL は後から作れない。
/// 既定で URL を読む対象。
pub const DEFAULT_BROWSERS: [&str; 8] = [
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "brave.exe",
    "vivaldi.exe",
    "opera.exe",
    "iexplore.exe",
    "chromium.exe",
];

/// URL を読む対象の一覧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Browsers {
    names: Vec<String>,
}

impl Browsers {
    /// 既定の一覧。
    pub fn new() -> Self {
        Self {
            names: DEFAULT_BROWSERS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// `ASHIATO_BROWSERS`（コンマ区切り）を**既定に足す**。
    ///
    /// 置き換えではなく足すのは、書いた瞬間に既定のブラウザの URL が
    /// 黙って取れなくなるのを防ぐため。
    pub fn from_env() -> Self {
        let mut b = Self::new();
        if let Ok(extra) = std::env::var("ASHIATO_BROWSERS") {
            for name in extra.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                b.names.push(name.to_lowercase());
            }
        }
        b
    }

    /// このプロセス名はブラウザか。**大文字小文字を無視する**（Windows の規則）。
    pub fn contains(&self, process_name: &str) -> bool {
        self.names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(process_name))
    }
}

impl Default for Browsers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 既定の一覧が効き、大文字小文字を無視する。
    #[test]
    fn default_browsers_match_ignoring_case() {
        let b = Browsers::new();
        assert!(b.contains("chrome.exe"));
        assert!(b.contains("MSEDGE.EXE"));
        assert!(!b.contains("editor.exe"));
    }

    /// 環境変数は**既定に足す**（置き換えない）。
    #[test]
    fn env_adds_to_the_defaults() {
        // 環境変数はプロセス全体で共有なので、この試験だけの名前を使う
        std::env::set_var("ASHIATO_BROWSERS", "waterfox.exe, my-browser.exe");
        let b = Browsers::from_env();
        assert!(b.contains("waterfox.exe"));
        assert!(b.contains("my-browser.exe"));
        assert!(b.contains("chrome.exe"), "既定が置き換えられている");
        std::env::remove_var("ASHIATO_BROWSERS");
    }
}
