// SPDX-License-Identifier: AGPL-3.0-only
//! 起動時の手当て（本文ごと消した後に作り直しで隠れたのに本文が残る重なる滞在の本文を消す）を本物の PostgreSQL で確かめる
//! （CH08 Task 2 / design D5）。「起動する」は `run()` が呼ぶ `purge_hidden_overlap_remedy` と同じ処理を、
//! 並走する試験の行に触れないよう利用者に絞って呼ぶ。
#![allow(clippy::unwrap_used)]

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::deletion;
use crate::purge_tests::{
    at, emptied, kept, purge_now, scene_basic, scene_erase, scene_mark_rebuild, scene_stay,
    scene_version,
};
use crate::testdb;

const DAY: &str = "2026-10-01";

fn t(hhmm: &str) -> DateTime<Utc> {
    at(&format!("{DAY}T{hhmm}:00+09:00"))
}

async fn stay(pool: &PgPool, user: Uuid, from: &str, to: &str) -> Uuid {
    scene_stay(pool, user, &t(from).to_rfc3339(), &t(to).to_rfc3339()).await
}

/// 10:00–11:00 の滞在を消して本文ごと消す。（滞在の識別子, 操作の識別子）
async fn purged(pool: &PgPool, user: Uuid) -> (Uuid, Uuid) {
    let (p, _) = scene_basic(pool, user).await;
    scene_erase(pool, p).await;
    let operation = purge_now(pool, p).await.operation_id.unwrap();
    (p, operation)
}

/// CH08 の前の作り直しを再現する: 一覧に出ている L に、本文を残したまま作り直しの印を直に付ける。
async fn absorbed_overlap(pool: &PgPool, user: Uuid) -> Uuid {
    let l = stay(pool, user, "10:30", "11:30").await;
    for (no, from, to) in [(1, "10:20", "11:30"), (2, "10:30", "11:00")] {
        scene_version(
            pool,
            l,
            user,
            no,
            &t(from).to_rfc3339(),
            &t(to).to_rfc3339(),
        )
        .await;
    }
    scene_mark_rebuild(pool, l, "rebuild:absorbed").await;
    l
}

async fn remedy(pool: &PgPool, user: Uuid) -> usize {
    deletion::purge_hidden_overlap_remedy_for_user(pool, user, "event").await
}

/// (消去の操作, 記録ごとの消去, 削除) の台帳の行の数
async fn ledger_counts(pool: &PgPool, user: Uuid) -> (i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM core.erasure_operation WHERE user_id = $1),
                (SELECT count(*) FROM core.erasure_ledger l
                   JOIN core.erasure_operation o ON o.id = l.operation_id WHERE o.user_id = $1),
                (SELECT count(*) FROM core.deletion_ledger WHERE user_id = $1)",
    )
    .bind(user)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[derive(Clone, Default)]
struct Captured(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Scenario: 起動すると本文ごと消した後に隠れて本文が残った重なる滞在の本文が消える
#[tokio::test]
async fn purge_remedy_empties_the_overlapping_stay_hidden_after_the_purge() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (p, operation) = purged(&pool, user).await;
    let l = absorbed_overlap(&pool, user).await;
    assert!(kept(&pool, l).await, "前の状態で本文が残っていない");

    assert_eq!(remedy(&pool, user).await, 0);

    assert!(emptied(&pool, l).await, "原文と解析済みが空でない");
    let versions: (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE raw = '' AND payload = '{}'::jsonb)
           FROM core.event_version WHERE event_id = $1",
    )
    .bind(l)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(versions, (2, 2), "前の版の本文が空でない");
    let late: Vec<(Uuid, Option<Uuid>, Uuid, i32)> = sqlx::query_as(
        "SELECT id, parent_id, stay_id, stays FROM core.erasure_operation
          WHERE user_id = $1 AND kind = 'late'",
    )
    .bind(user)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(late.len(), 1, "後からの操作の行が 1 行でない");
    assert_eq!(late[0].1, Some(operation), "元の操作に結ばれていない");
    assert_eq!(late[0].2, p);
    assert_eq!(late[0].3, 1, "滞在の件数");
    let span: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.erasure_span
          WHERE operation_id = $1 AND event_id = $2 AND scope = 'event'",
    )
    .bind(late[0].0)
    .bind(l)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(span, 1, "滞在の時間の範囲の行が無い");
}

// Scenario: 起動を繰り返しても台帳の行は増えない
#[tokio::test]
async fn purge_remedy_repeated_startups_add_no_ledger_rows() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    absorbed_overlap(&pool, user).await;

    remedy(&pool, user).await;
    let first = ledger_counts(&pool, user).await;
    remedy(&pool, user).await;

    assert_eq!(ledger_counts(&pool, user).await, first);
}

// Scenario: 本文ごと消す前から隠れていた端の滞在は起動しても本文が残る
#[tokio::test]
async fn purge_remedy_edge_stay_hidden_before_the_purge_keeps_its_body() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (p, _) = scene_basic(&pool, user).await;
    scene_erase(&pool, p).await;
    let edge = stay(&pool, user, "11:00", "12:00").await;
    scene_mark_rebuild(&pool, edge, "rebuild:absorbed").await;
    purge_now(&pool, p).await;

    remedy(&pool, user).await;

    assert!(kept(&pool, edge).await, "端の滞在の本文が消えた");
}

// Scenario: 本文ごと消した後に初めから隠して作られた滞在は起動しても本文が残る
#[tokio::test]
async fn purge_remedy_stay_created_hidden_after_the_purge_keeps_its_body() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let born = stay(&pool, user, "10:30", "11:30").await;
    // `insert_stay` の隠して作る行と同じ: 作った時刻と隠した時刻が同じ
    sqlx::query(
        "UPDATE core.event SET deleted_at = ingest_time, deleted_by = 'rebuild:erased-range' WHERE id = $1",
    )
    .bind(born)
    .execute(&pool)
    .await
    .unwrap();

    remedy(&pool, user).await;

    assert!(
        kept(&pool, born).await,
        "初めから隠して作られた滞在の本文が消えた"
    );
}

// Scenario: 本文ごと消した後に戻されて作り直しで隠れた消した滞在は起動しても本文が残る
#[tokio::test]
async fn purge_remedy_restored_then_hidden_stay_keeps_its_body() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    // 本文ごと消した後に、本人が消して（台帳に erase）戻して（台帳に restore）、作り直しで吸収された
    let b = stay(&pool, user, "10:15", "10:45").await;
    scene_erase(&pool, b).await;
    deletion::restore(&pool, &[b], None).await.unwrap();
    scene_mark_rebuild(&pool, b, "rebuild:absorbed").await;
    assert!(kept(&pool, b).await, "前の状態で本文が残っていない");

    remedy(&pool, user).await;

    assert!(kept(&pool, b).await, "戻された滞在の本文が消えた");
}

// Scenario: 起動の手当てのログは件数だけを持つ
#[tokio::test]
async fn purge_remedy_log_carries_only_the_count() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let (p, _) = purged(&pool, user).await;
    let l = absorbed_overlap(&pool, user).await;
    let captured = Captured::default();
    let sink = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || sink.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);

    let left = deletion::purge_hidden_overlap_remedy_for_user(&pool, user, "not-a-scope").await;

    drop(guard);
    assert_eq!(left, 1);
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("kind=\"purge_remedy\""), "{log}");
    assert!(log.contains("count=1"), "{log}");
    for secret in [
        l.to_string(),
        p.to_string(),
        user.to_string(),
        DAY.to_string(),
        "35.68".into(),
        "139.76".into(),
    ] {
        assert!(!log.contains(&secret), "{secret} がログに出た: {log}");
    }
}

// Scenario: 対象が無ければ起動の手当ては何も書かない
#[tokio::test]
async fn purge_remedy_without_targets_writes_nothing() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let before = ledger_counts(&pool, user).await;

    assert_eq!(remedy(&pool, user).await, 0);

    assert_eq!(ledger_counts(&pool, user).await, before);
}

// Scenario: 手当てが失敗しても起動は止まらず、次の起動で消える
#[tokio::test]
async fn purge_remedy_failure_leaves_the_body_and_the_next_startup_empties_it() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    purged(&pool, user).await;
    let l = absorbed_overlap(&pool, user).await;
    let before = ledger_counts(&pool, user).await;

    let left = deletion::purge_hidden_overlap_remedy_for_user(&pool, user, "not-a-scope").await;

    assert_eq!(left, 1, "消せずに残った件数");
    assert!(kept(&pool, l).await, "失敗したのに本文が消えた");
    assert_eq!(
        ledger_counts(&pool, user).await,
        before,
        "台帳が巻き戻っていない"
    );
    assert_eq!(remedy(&pool, user).await, 0);
    assert!(emptied(&pool, l).await, "次の起動で本文が空にならない");
}
