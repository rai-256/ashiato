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

// ---- 版の変化のビュー `core.collector_version_change`（Task 5 / design D5・D6） ----
//
// 生存信号は直に INSERT する（発信の時刻と受信の時刻を指定するため）。ビューは `c01-`・`c02-` で
// 始まるソースだけを読むので、`testdb::source`（`t-` で始まる）は使わず、名前を自分で登録する。
// DB は回で共有なので、行は試験ごとの利用者で絞って数える。

const COMMIT_B: &str = "0.1.0+89abcdef0123456789abcdef0123456789abcdef";

/// 登録簿へ置いたソースの名前。`prefix` は `c01-` などビューが読む接頭辞。
async fn view_source(pool: &sqlx::PgPool, prefix: &str) -> String {
    let name = format!("{prefix}st51v-{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO core.source
           (logical_source, display_name, expected_gap_sec, registered_at, external_id_kind)
         VALUES ($1, $1, 3600, ($2::date::timestamp AT TIME ZONE 'Asia/Tokyo'), 'none')",
    )
    .bind(&name)
    .bind("2000-01-01")
    .execute(pool)
    .await
    .unwrap();
    name
}

/// 生存信号 1 件を直に置く。時刻は 2026-10-20 の `hhmm`（UTC）。受信は発信の `recv_min` 分後。
/// `column` は列 `collector_version`、`raw` は原文（`None` なら `{"alive":true}`）。
struct Sig<'a> {
    user: uuid::Uuid,
    source: &'a str,
    device: Option<&'a str>,
    emitted: &'a str,
    received: &'a str,
    column: Option<&'a str>,
    raw: Option<String>,
}

impl<'a> Sig<'a> {
    fn new(user: uuid::Uuid, source: &'a str, device: Option<&'a str>, at: &'a str) -> Self {
        Sig {
            user,
            source,
            device,
            emitted: at,
            received: at,
            column: None,
            raw: None,
        }
    }
    fn col(mut self, v: &'a str) -> Self {
        self.column = Some(v);
        self
    }
    fn received(mut self, at: &'a str) -> Self {
        self.received = at;
        self
    }
    fn raw(mut self, raw: impl Into<String>) -> Self {
        self.raw = Some(raw.into());
        self
    }
    async fn put(self, pool: &sqlx::PgPool) {
        let id = uuid::Uuid::new_v4();
        let raw = self.raw.unwrap_or_else(|| r#"{"alive":true}"#.into());
        sqlx::query(
            "INSERT INTO core.heartbeat
               (id, user_id, logical_source, device_id, emitted_at, received_at,
                capturable, blockers, attempts, successes, content_hash, raw, collector_version)
             VALUES ($1, $2, $3, $4, $5::timestamptz, $6::timestamptz,
                     true, '{}', 1, 1, $7, $8, $9)",
        )
        .bind(id)
        .bind(self.user)
        .bind(self.source)
        .bind(self.device)
        .bind(format!("2026-10-20T{}:00Z", self.emitted))
        .bind(format!("2026-10-20T{}:00Z", self.received))
        .bind(id.to_string())
        .bind(raw)
        .bind(self.column)
        .execute(pool)
        .await
        .unwrap();
    }
}

type Change = (
    Option<String>,
    String,
    Option<String>,
    String,
    chrono::DateTime<chrono::Utc>,
);

/// ビューを利用者で絞って引く（端末・部品・前の版・今の版・発信の時刻）。
async fn changes(pool: &sqlx::PgPool, user: uuid::Uuid) -> Vec<Change> {
    sqlx::query_as(
        "SELECT device_id, component, from_version, to_version, emitted_at
           FROM core.collector_version_change
          WHERE user_id = $1
          ORDER BY emitted_at, component, device_id",
    )
    .bind(user)
    .fetch_all(pool)
    .await
    .unwrap()
}

fn raw_version(v: &str) -> String {
    serde_json::json!({ "alive": true, "client_version": v }).to_string()
}

// Scenario: 収集アプリを更新すると、版の変化が 1 行出る
#[tokio::test]
async fn st51_view_one_change_on_update() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    Sig::new(user, &src, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "11:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "12:00")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    let rows = changes(&pool, user).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    let (device, component, from, to, at) = &rows[0];
    assert_eq!(device.as_deref(), Some("d1"));
    assert_eq!(component, "C-01");
    assert_eq!(from.as_deref(), Some(COMMIT_A));
    assert_eq!(to, COMMIT_B);
    assert_eq!(at.to_rfc3339(), "2026-10-20T12:00:00+00:00");
}

// Scenario: 更新の前に溜まった信号が後から届いても、偽の変化は出ない
#[tokio::test]
async fn st51_view_late_queued_no_false_change() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    // 12:00 の信号（版 B）が先に届き、10:00・11:00 の信号（版 A）は 13:00 以後に届く
    Sig::new(user, &src, Some("d1"), "12:00")
        .col(COMMIT_B)
        .received("12:01")
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "10:00")
        .col(COMMIT_A)
        .received("13:00")
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "11:00")
        .col(COMMIT_A)
        .received("13:01")
        .put(&pool)
        .await;
    let rows = changes(&pool, user).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].2.as_deref(), Some(COMMIT_A));
    assert_eq!(rows[0].3, COMMIT_B);
    assert_eq!(rows[0].4.to_rfc3339(), "2026-10-20T12:00:00+00:00");
}

// Scenario: 最初に版を持った信号は、変化として出ない
#[tokio::test]
async fn st51_view_first_is_start_not_change() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    for at in ["08:00", "09:00", "10:00"] {
        Sig::new(user, &src, Some("d1"), at).put(&pool).await;
    }
    Sig::new(user, &src, Some("d1"), "11:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "12:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    assert!(changes(&pool, user).await.is_empty());
}

// Scenario: 版を持たない信号が挟まっても、変化は版を持つ信号どうしで数える
#[tokio::test]
async fn st51_view_skips_versionless() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    Sig::new(user, &src, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "11:00").put(&pool).await;
    Sig::new(user, &src, Some("d1"), "12:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    assert!(changes(&pool, user).await.is_empty());
}

// Scenario: 版を持たない信号が挟まっても、変化は版を持つ信号どうしで数える
#[tokio::test]
async fn st51_view_versionless_between_different_versions() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    Sig::new(user, &src, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "11:00").put(&pool).await;
    Sig::new(user, &src, Some("d1"), "12:00")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    let rows = changes(&pool, user).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].2.as_deref(), Some(COMMIT_A));
    assert_eq!(rows[0].3, COMMIT_B);
}

// Scenario: 列に版が無い信号は、原文の版で数える
#[tokio::test]
async fn st51_view_reads_raw_when_column_empty() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    Sig::new(user, &src, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "11:00")
        .raw(raw_version(COMMIT_B))
        .put(&pool)
        .await;
    let rows = changes(&pool, user).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].3, COMMIT_B);
}

// Scenario: 原文が JSON でない信号があっても、読み口は失敗しない
#[tokio::test]
async fn st51_view_non_json_raw_ok() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    Sig::new(user, &src, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "11:00")
        .raw("not json {")
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "12:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    // 読み口全体が失敗しない（全利用者ぶんを引く）。不正な原文の信号は並びから外れ、変化は出ない
    let all: i64 = sqlx::query_scalar("SELECT count(*) FROM core.collector_version_change")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(all >= 0);
    assert!(changes(&pool, user).await.is_empty());
}

// Scenario: 同じ端末でも、部品が違う信号は別の並びで数える
#[tokio::test]
async fn st51_view_split_by_component() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let c01 = view_source(&pool, "c01-").await;
    let c02 = view_source(&pool, "c02-").await;
    Sig::new(user, &c01, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &c02, Some("d1"), "11:00")
        .col("0.2.0+b")
        .put(&pool)
        .await;
    Sig::new(user, &c01, Some("d1"), "12:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &c02, Some("d1"), "13:00")
        .col("0.2.0+b")
        .put(&pool)
        .await;
    assert!(changes(&pool, user).await.is_empty());
}

// Scenario: 同じ部品でも、端末が違う信号は別の並びで数える
#[tokio::test]
async fn st51_view_split_by_device() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    Sig::new(user, &src, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d2"), "11:00")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "12:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d2"), "13:00")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    assert!(changes(&pool, user).await.is_empty());
}

// Scenario: 同じ端末と部品でも、利用者が違う信号は別の並びで数える
#[tokio::test]
async fn st51_view_split_by_user() {
    let pool = testdb::pool().await;
    let (u1, u2) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let src = view_source(&pool, "c01-").await;
    Sig::new(u1, &src, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(u2, &src, Some("d1"), "11:00")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    Sig::new(u1, &src, Some("d1"), "12:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(u2, &src, Some("d1"), "13:00")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    assert!(changes(&pool, u1).await.is_empty());
    assert!(changes(&pool, u2).await.is_empty());
}

// Scenario: 収集アプリでないソースの信号は並びに入らない
#[tokio::test]
async fn st51_view_only_collector_sources() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    // 部品の CASE は c01-/c02- 以外を C-02 に倒すので、c02- の並びに挟んで絞り込みを確かめる
    let loc = view_source(&pool, "c02-").await;
    let other = view_source(&pool, "c03-chrome-history-").await;
    Sig::new(user, &loc, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &other, Some("d1"), "11:00")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    Sig::new(user, &loc, Some("d1"), "12:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    assert!(changes(&pool, user).await.is_empty());
}

// Scenario: 同じ端末の C-01 の 2 つのソースが同じ版を申告しても、変化は 1 行
#[tokio::test]
async fn st51_view_two_sources_one_change() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let loc = view_source(&pool, "c01-").await;
    let usage = view_source(&pool, "c01-usage-").await;
    Sig::new(user, &loc, Some("d1"), "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &usage, Some("d1"), "10:30")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &loc, Some("d1"), "12:00")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    Sig::new(user, &usage, Some("d1"), "12:30")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    let rows = changes(&pool, user).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].1, "C-01");
}

/// ST51 より前の S-01 は `client_version` の欄を読み飛ばし、列は空のまま原文だけに版が残る（premises 3）。
#[tokio::test]
async fn st51_view_old_server_drops_field_keeps_raw() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    // 要求の欄を持つ JSON 全体が原文になり、列は書かれない
    Sig::new(user, &src, Some("d1"), "10:00")
        .raw(raw_version(COMMIT_A))
        .put(&pool)
        .await;
    Sig::new(user, &src, Some("d1"), "12:00")
        .raw(raw_version(COMMIT_B))
        .put(&pool)
        .await;
    let rows = changes(&pool, user).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].2.as_deref(), Some(COMMIT_A));
    assert_eq!(rows[0].3, COMMIT_B);
}

/// 端末を持たない信号（NULL・空）は、利用者・部品ごとに 1 つの並びになる。
#[tokio::test]
async fn st51_view_null_device_one_sequence() {
    let pool = testdb::pool().await;
    let user = uuid::Uuid::new_v4();
    let src = view_source(&pool, "c01-").await;
    Sig::new(user, &src, None, "10:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, Some(""), "11:00")
        .col(COMMIT_A)
        .put(&pool)
        .await;
    Sig::new(user, &src, None, "12:00")
        .col(COMMIT_B)
        .put(&pool)
        .await;
    let rows = changes(&pool, user).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].0, None);
    assert_eq!(rows[0].3, COMMIT_B);
}
