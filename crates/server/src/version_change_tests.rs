// SPDX-License-Identifier: AGPL-3.0-only
//! 版の変わり目の受け口（ST51 / design D6 / D13）を、本物の PostgreSQL に対して確かめる。
//!
//! 生存信号の `client_version`（Task 4）。口は `heartbeat_post` を直に呼ぶ（`api_tests.rs` と同じ）。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";
const COMMIT_A: &str = "0.1.0+0123456789abcdef0123456789abcdef01234567";

fn auth() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

/// 生存信号 1 件ぶんの JSON。`version` が `None` なら欄ごと持たない。
fn hb(source: &str, user: uuid::Uuid, version: Option<&str>) -> serde_json::Value {
    let mut v = serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "user_id": user,
        "logical_source": source,
        "device_id": "test-dev",
        "emitted_at": "2026-10-20T03:00:00Z",
        "capturable": true,
        "blockers": [],
        "attempts": 10,
        "successes": 9,
        "raw": r#"{"alive":true}"#,
    });
    if let Some(version) = version {
        v["client_version"] = serde_json::json!(version);
    }
    v
}

/// 1 件送り、受け付けられたことを確かめて、行の `collector_version` を返す。
async fn send(version: Option<&str>) -> Option<String> {
    let pool = testdb::pool().await;
    let source = testdb::source(&pool, "c01-st51hb", 3600).await;
    let user = uuid::Uuid::new_v4();
    let app = App::for_test(pool.clone(), TOKEN);
    let (_, Json(res)) = heartbeat_post(
        State(app),
        auth(),
        Json(serde_json::json!([hb(&source, user, version)])),
    )
    .await
    .expect("生存信号の受け口");
    assert!(res[0].accepted, "受け付けられる: {:?}", res[0].error);
    sqlx::query_scalar("SELECT collector_version FROM core.heartbeat WHERE user_id = $1")
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap()
}

fn base() -> heartbeat::HeartbeatRequest {
    heartbeat::HeartbeatRequest {
        id: uuid::Uuid::nil(),
        user_id: uuid::Uuid::nil(),
        logical_source: "pinned-source".into(),
        device_id: Some("whatever".into()),
        emitted_at: chrono::DateTime::from_timestamp(1_757_000_000, 0).expect("時刻"),
        capturable: true,
        blockers: vec![],
        attempts: 360,
        successes: 230,
        network: None,
        client_version: None,
        raw: r#"{"alive":true}"#.into(),
    }
}

// Scenario: 生存信号の版は、行に残る
#[tokio::test]
async fn st51_hb_version_stored() {
    assert_eq!(send(Some(COMMIT_A)).await.as_deref(), Some(COMMIT_A));
}

// Scenario: 版の欄を持たない生存信号も受け付ける
#[tokio::test]
async fn st51_hb_without_version_accepted() {
    assert_eq!(send(None).await, None);
}

// Scenario: 形の崩れた版でも、生存信号は断られない
#[tokio::test]
async fn st51_hb_malformed_version_accepted() {
    assert_eq!(
        send(Some("not-a-version")).await.as_deref(),
        Some("not-a-version")
    );
}

// Scenario: 空の版は空として残り、U+0000 は取り除かれる
#[tokio::test]
async fn st51_hb_empty_and_nul() {
    assert_eq!(send(Some("")).await, None);
    let with_nul = format!("{}\0{}", &COMMIT_A[..5], &COMMIT_A[5..]);
    assert_eq!(send(Some(&with_nul)).await.as_deref(), Some(COMMIT_A));
    // 前後の空白は除かない（D6）
    assert_eq!(send(Some(" 1 ")).await.as_deref(), Some(" 1 "));
}

/// 欄を足す前の固定値（`api_tests::heartbeat_hash_is_pinned` と同じ入力・同じ値）。
// Scenario: 版の欄を足しても、生存信号の冪等キーは変わらない
#[test]
fn st51_hb_hash_is_pinned() {
    const PINNED: &str = "86330ea52a87d040a1db94ae259aeda9bce72ec3b8eb0f488e0ed31c879061df";
    let without = base();
    let mut with = base();
    with.client_version = Some(COMMIT_A.into());
    assert_eq!(heartbeat::content_hash(&without), PINNED);
    assert_eq!(heartbeat::content_hash(&with), PINNED);
}

// Scenario: 書庫の取り込みが積む生存信号は版を持たない
#[tokio::test]
async fn st51_hb_archive_has_no_version() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    crate::archive::worker::record_archive_heartbeat(
        &pool,
        user,
        chrono::DateTime::parse_from_rfc3339("2026-10-20T03:00:00Z")
            .unwrap()
            .to_utc(),
        true,
        vec![],
    )
    .await
    .unwrap();
    let rows: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT collector_version FROM core.heartbeat
          WHERE user_id = $1 AND logical_source = 's01-archive-inbox'",
    )
    .bind(user)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows, vec![None]);
}

/// 冪等キーはソース・発信の時刻・原文だけから作られる。ほかの欄（版を含む）を変えても動かず、
/// この 3 つは 1 つ変えれば動く。
#[test]
fn st51_hb_hash_inputs_only() {
    let want = heartbeat::content_hash(&base());
    let mut other = base();
    other.id = uuid::Uuid::new_v4();
    other.user_id = uuid::Uuid::new_v4();
    other.device_id = None;
    other.capturable = false;
    other.blockers = vec!["permission".into()];
    other.attempts = 1;
    other.successes = 0;
    other.network = Some(false);
    other.client_version = Some(COMMIT_A.into());
    assert_eq!(heartbeat::content_hash(&other), want);

    let mut s = base();
    s.logical_source = "other-source".into();
    assert_ne!(heartbeat::content_hash(&s), want);
    let mut t = base();
    t.emitted_at = chrono::DateTime::from_timestamp(1_757_000_001, 0).unwrap();
    assert_ne!(heartbeat::content_hash(&t), want);
    let mut r = base();
    r.raw = r#"{"alive":false}"#.into();
    assert_ne!(heartbeat::content_hash(&r), want);
}
