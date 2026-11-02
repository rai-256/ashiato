//! 収集アプリの資格情報で到達できる口の一覧（ST44 / design D4）。

use axum::http::Method;

/// 収集アプリに公開する方法と `MatchedPath` の組。
pub const COLLECTOR_ALLOWED: &[(Method, &str)] = &[
    (Method::POST, "/ingest"),
    (Method::POST, "/heartbeat"),
    (Method::POST, "/drops"),
    (Method::POST, "/media/missing"),
    (Method::PUT, "/media/originals/{sha256}"),
    (Method::GET, "/contract"),
    (Method::HEAD, "/media/originals/{sha256}"),
    (Method::GET, "/feelings/reminder"),
    (Method::GET, "/outages"),
];

pub fn collector_allows(method: &Method, matched_path: &str) -> bool {
    COLLECTOR_ALLOWED
        .iter()
        .any(|(allowed_method, allowed_path)| {
            allowed_method == method && *allowed_path == matched_path
        })
}

/// 資格情報を求めない口（ST44 D9）。API の合言葉でも 403 にならない。
/// `bin/openapi.rs` が契約の 403 を足さない口をここから引く（写しを持たない。/code-review R24）。
/// `access_log::middleware` が `/session` を丸ごと通すのは「ハンドラが自分で行を書く」からで、
/// `GET /session` は資格情報を求め、`session_get` が自分で 403 を返す（D12（仮））。
pub fn credential_exempt(method: &Method, matched_path: &str) -> bool {
    matched_path == "/healthz"
        || (matched_path == "/session" && (*method == Method::POST || *method == Method::DELETE))
}
