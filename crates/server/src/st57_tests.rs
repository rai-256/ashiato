#![allow(clippy::unwrap_used)]

use super::update_status::{classify, read_check_settings, read_status, CheckSettings};
use ashiato_update_check::{CheckReason, Status};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use chrono::{DateTime, Duration, Utc};
use http_body_util::BodyExt as _;
use serde_json::json;
use tower::ServiceExt as _;

const NOW: &str = "2026-11-12T00:00:00Z";
const INSTALLED: &str = "0.4.0";

fn now() -> DateTime<Utc> {
    NOW.parse().unwrap()
}
fn settings(enabled: bool, since: DateTime<Utc>) -> CheckSettings {
    CheckSettings {
        enabled,
        since: Some(since),
    }
}
fn status(
    latest: Option<&str>,
    checked_at: DateTime<Utc>,
    last_ok_at: Option<DateTime<Utc>>,
    reason: Option<CheckReason>,
) -> Status {
    Status {
        latest: latest.map(str::to_owned),
        found_at: latest.map(|_| checked_at),
        checked_at: Some(checked_at),
        last_ok_at,
        last_reason: reason,
        notified_version: None,
        stale_notified: false,
    }
}

async fn login_cookie(app: &crate::App) -> String {
    let response = crate::router(app.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/session")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"password": app.web_password_for_test()}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}
#[tokio::test]
async fn st57_update_status_requires_login_and_rejects_api_token() {
    // Scenario: 更新の状態の口はログインが要り、API の合言葉では答えない
    let app = crate::App::for_test(crate::testdb::pool().await, "token-for-test-0123456789");
    let unauthorized = crate::router(app.clone())
        .oneshot(
            Request::builder()
                .uri("/update-status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    let forbidden = crate::router(app)
        .oneshot(
            Request::builder()
                .uri("/update-status")
                .header(header::AUTHORIZATION, "Bearer token-for-test-0123456789")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[test]
fn st57_unreadable_result_is_not_exposed() {
    // Scenario: 読めない結果のファイルは画面に出さない
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("settings")).unwrap();
    std::fs::write(
        dir.path().join("settings/check.json"),
        json!({"enabled":true,"since":NOW}).to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("status.json"),
        json!({"latest":"<script>"}).to_string(),
    )
    .unwrap();
    assert_eq!(
        read_status(dir.path(), INSTALLED, now()).state,
        "unreadable"
    );
    std::fs::write(dir.path().join("status.json"), vec![b' '; 65 * 1024]).unwrap();
    assert_eq!(
        read_status(dir.path(), INSTALLED, now()).state,
        "unreadable"
    );
}

#[test]
fn st57_small_valid_result_is_read() {
    // Scenario: 64 KB より小さい正しい結果のファイルは読む
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("settings")).unwrap();
    std::fs::write(
        dir.path().join("settings/check.json"),
        json!({"enabled":true,"since":NOW}).to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("status.json"),
        json!({"latest":"0.5.0","found_at":NOW,"checked_at":NOW,"last_ok_at":NOW}).to_string(),
    )
    .unwrap();
    assert_eq!(read_status(dir.path(), INSTALLED, now()).state, "available");
}

#[test]
fn st57_old_updater_has_not_running_reason() {
    // Scenario: 更新係の時刻が古ければ、理由は更新係が動いていない
    let s = status(
        None,
        now() - Duration::hours(49),
        Some(now() - Duration::days(9)),
        Some(CheckReason::Unreachable),
    );
    let result = classify(
        Some(settings(true, now() - Duration::days(10))),
        Some(s),
        None,
        INSTALLED,
        now(),
    );
    assert_eq!(result.reason.as_deref(), Some("not_running"));
}

#[test]
fn st57_recent_updater_keeps_last_reason() {
    // Scenario: 更新係の時刻が 48 時間たたなければ、理由は最後の回のまま
    let s = status(
        None,
        now() - Duration::hours(47),
        Some(now() - Duration::days(9)),
        Some(CheckReason::Unreachable),
    );
    let result = classify(
        Some(settings(true, now() - Duration::days(10))),
        Some(s),
        None,
        INSTALLED,
        now(),
    );
    assert_eq!(result.reason.as_deref(), Some("unreachable"));
}

#[tokio::test]
async fn st57_update_status_is_not_cached() {
    // Scenario: 更新の状態の口は応答を残させない
    let app = crate::App::for_test(crate::testdb::pool().await, "token-for-test-0123456789");
    let cookie = login_cookie(&app).await;
    let response = crate::router(app)
        .oneshot(
            Request::builder()
                .uri("/update-status")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        7
    );
}

#[test]
fn st57_state_off() {
    assert_eq!(
        classify(Some(settings(false, now())), None, None, INSTALLED, now()).state,
        "off"
    );
}
#[test]
fn st57_state_not_running() {
    assert_eq!(
        classify(None, None, None, INSTALLED, now()).state,
        "not_running"
    );
}
#[test]
fn st57_state_pending() {
    assert_eq!(
        classify(
            Some(settings(true, now() - Duration::days(2))),
            None,
            None,
            INSTALLED,
            now()
        )
        .state,
        "pending"
    );
}
#[test]
fn st57_state_latest() {
    let t = now() - Duration::hours(1);
    assert_eq!(
        classify(
            Some(settings(true, t)),
            Some(status(Some(INSTALLED), t, Some(t), None)),
            None,
            INSTALLED,
            now()
        )
        .state,
        "latest"
    );
}
#[test]
fn st57_state_available() {
    let t = now() - Duration::hours(1);
    assert_eq!(
        classify(
            Some(settings(true, t)),
            Some(status(Some("0.5.0"), t, Some(t), None)),
            None,
            INSTALLED,
            now()
        )
        .state,
        "available"
    );
}
#[test]
fn st57_state_stale() {
    let t = now() - Duration::days(8);
    assert_eq!(
        classify(
            Some(settings(true, t)),
            Some(status(Some(INSTALLED), t, Some(t), None)),
            None,
            INSTALLED,
            now()
        )
        .state,
        "stale"
    );
}
#[test]
fn st57_state_available_stale() {
    let t = now() - Duration::days(8);
    assert_eq!(
        classify(
            Some(settings(true, t)),
            Some(status(Some("0.5.0"), t, Some(t), None)),
            None,
            INSTALLED,
            now()
        )
        .state,
        "available_stale"
    );
}
#[test]
fn st57_state_unreadable() {
    assert_eq!(
        classify(
            Some(settings(true, now())),
            None,
            Some("bad"),
            INSTALLED,
            now()
        )
        .state,
        "unreadable"
    );
}

#[test]
fn st57_check_settings_reads_rfc3339() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("settings")).unwrap();
    std::fs::write(
        dir.path().join("settings/check.json"),
        json!({"enabled":true,"since":NOW}).to_string(),
    )
    .unwrap();
    assert_eq!(
        read_check_settings(dir.path()).unwrap(),
        settings(true, now())
    );
}
