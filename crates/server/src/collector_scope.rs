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
