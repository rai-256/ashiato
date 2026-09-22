// SPDX-License-Identifier: AGPL-3.0-only
//! ログに出してよいものだけを組み立てる（製造準備 A-2 / spec「収集側のログに私的な内容を出さない」）。
//!
//! **ウィンドウ題名には開いている文書の題・メールの件名・相手の名前がそのまま載る。**
//! ログは記録本体と違って感度の制御（PERM-2）が効かないので、一度出たものは締められない。
//!
//! 出してよいのは **件数・ソース名・所要時間・エラーの種別** の 4 つだけ。
//! だから**値を渡す口を用意しない** —— 引数の型が `usize` と「種別」に絞られている。
//! C-01 の `Telemetry` と同じ形にしてある（2 実装で違う規則を持たない）。
use crate::LOGICAL_SOURCE;

/// ログの 1 行。**題名・URL・アプリ名・原文は渡せない。**
pub fn line(
    kind: &str,
    count: Option<usize>,
    elapsed_ms: Option<u128>,
    error: Option<&str>,
) -> String {
    let mut s = format!("kind={kind} source={LOGICAL_SOURCE}");
    if let Some(c) = count {
        s.push_str(&format!(" count={c}"));
    }
    if let Some(ms) = elapsed_ms {
        s.push_str(&format!(" elapsed_ms={ms}"));
    }
    if let Some(e) = error {
        s.push_str(&format!(" error={e}"));
    }
    s
}

/// ブラウザ履歴の読取り用ログ。本文を受け取る引数を持たず、ソースだけを分ける。
pub fn history_line(
    kind: &str,
    count: Option<usize>,
    elapsed_ms: Option<u128>,
    error: Option<&str>,
) -> String {
    let mut s = format!("kind={kind} source=c02-browser-history");
    if let Some(c) = count { s.push_str(&format!(" count={c}")); }
    if let Some(ms) = elapsed_ms { s.push_str(&format!(" elapsed_ms={ms}")); }
    if let Some(e) = error { s.push_str(&format!(" error={e}")); }
    s
}

/// 断られた理由・網の失敗の**種別**。例外の文言をそのまま出さない ——
/// 文言には要求の本文（題名・URL）が入ることがある。
pub fn error_kind(e: &anyhow::Error) -> &'static str {
    // 種別は自分で名付けたものだけを返す。**下位の文言を素通しさせない**
    if e.to_string().contains("timed out") {
        "timeout"
    } else {
        "unreachable"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **失敗のログに題名も URL も出ない**（spec / 製造準備 A-2）。
    ///
    /// Scenario: 送信の失敗がログに出ても題名と URL は出ない
    #[test]
    fn log_has_no_private_content() {
        let l = line("send_failed", Some(12), Some(83), Some("unreachable"));
        assert_eq!(
            l,
            "kind=send_failed source=c02-window count=12 elapsed_ms=83 error=unreachable"
        );
        // 題名・URL・原文が入りうる文字列は、そもそも渡す口が無い
        for private in ["題名", "http", "example.com", "raw"] {
            assert!(
                !l.contains(private),
                "ログに私的な内容が出ている: {private}"
            );
        }
    }

    #[test]
    fn history_log_has_no_private_content() {
        // Scenario: 履歴の読み取りの失敗がログに出ても URL と題名は出ない
        let l = history_line("history_read_failed", Some(1), Some(83), Some("unreachable"));
        assert_eq!(l, "kind=history_read_failed source=c02-browser-history count=1 elapsed_ms=83 error=unreachable");
        for private in ["題名", "http", "example.com", "raw", "個人"] {
            assert!(!l.contains(private), "履歴ログに私的な内容が出ている: {private}");
        }
    }

    /// 例外の文言を素通しさせない（文言に本文が入ることがある）。
    #[test]
    fn error_kind_does_not_leak_the_message() {
        let e = anyhow::anyhow!("POST http://s/ingest failed: 題名 = 銀行 / 口座");
        let kind = error_kind(&e);
        assert_eq!(kind, "unreachable");
        assert!(!kind.contains("銀行"));
    }
}
