// SPDX-License-Identifier: AGPL-3.0-only
//! ブラウザ履歴を取り込み口へ送る契約。
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

/// 読み手が返す訪問。識別子は本文と分離して受け口の更新キーに使う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    pub external_id: String,
    pub payload: VisitPayload,
}

impl Visit {
    pub fn new(browser: &str, profile: &str, visit_id: i64, at: DateTime<Utc>, url: &str, title: &str) -> Self {
        let payload = VisitPayload {
            kind: "visit",
            at: micros(at),
            tz_basis: "collected-at",
            browser: browser.to_owned(),
            profile: profile.to_owned(),
            url: url.to_owned(),
            title: title.to_owned(),
            duration_ms: None,
            transition: None,
            referrer: None,
            originator_cache_guid: None,
            originator_visit_id: None,
        };
        let mut hasher = Sha256::new();
        let id = visit_id.to_string();
        for part in [browser, profile, id.as_str(), payload.at.as_str(), url] {
            hasher.update((part.len() as u64).to_be_bytes());
            hasher.update(part.as_bytes());
        }
        Self { external_id: format!("v1:{:x}", hasher.finalize()), payload }
    }
}

/// 訪問本文。URL と題名は visit のときだけ送り、消失・除外には含めない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VisitPayload {
    pub kind: &'static str,
    pub at: String,
    pub tz_basis: &'static str,
    pub browser: String,
    pub profile: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub originator_cache_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub originator_visit_id: Option<i64>,
}

/// RFC3339 UTC をマイクロ秒精度で固定する。
pub fn micros(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Micros, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visit_time_keeps_micros() {
        // Scenario: 訪問時刻がマイクロ秒で残る
        let at = chrono::DateTime::parse_from_rfc3339("2026-09-13T01:02:03.456789Z")
            .expect("時刻")
            .with_timezone(&chrono::Utc);
        let visit = Visit::new("chrome", "Default", 7, at, "https://example.test/a", "題名");
        assert_eq!(visit.payload.at, "2026-09-13T01:02:03.456789Z");
    }

    #[test]
    fn visit_external_id_is_pinned() {
        // Scenario: 識別子から URL と訪問時刻とプロファイルが読み取れない
        let at = chrono::DateTime::parse_from_rfc3339("2026-09-13T01:02:03.456789Z")
            .expect("時刻")
            .with_timezone(&chrono::Utc);
        let visit = Visit::new("chrome", "Default", 7, at, "https://example.test/a?q=x", "題名");
        assert!(visit.external_id.starts_with("v1:"));
        assert!(!visit.external_id.contains("example"));
        assert!(!visit.external_id.contains("Default"));
        assert!(!visit.external_id.contains("456789"));
    }

    #[test]
    fn visit_payload_shape_is_pinned() {
        // Scenario: タイムゾーンが取得時のものだと本文から分かる
        // Scenario: 記録の本文にブラウザとプロファイルがある
        let at = chrono::DateTime::parse_from_rfc3339("2026-09-13T01:02:03.456789Z")
            .expect("時刻")
            .with_timezone(&chrono::Utc);
        let visit = Visit::new("chrome", "Default", 7, at, "https://example.test/a", "題名");
        assert_eq!(serde_json::to_string(&visit.payload).expect("JSON"), r#"{"kind":"visit","at":"2026-09-13T01:02:03.456789Z","tz_basis":"collected-at","browser":"chrome","profile":"Default","url":"https://example.test/a","title":"題名"}"#);
    }
}
