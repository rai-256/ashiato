// SPDX-License-Identifier: AGPL-3.0-only
//! 本文ごと消しても残るもの（書庫の写し・書き出したファイル）と、変わらない数え方（稼働状況・書き出しの「後に消した記録」）
//! を HTTP の口と置き場のファイルで見る（ST23 Task 8 / design D10 / D18。申し送り R11）。
//! 写しと書き出したファイルは、置き場のファイルの SHA-256 を消す前後で比べる。
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use axum::http::{Method, StatusCode};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::export::Hooks;
use crate::export_excluded_tests::World;
use crate::export_job_tests::{app_with, call, get_status, post_status, request, wait_idle, PASS};
use crate::export_tests::EXPORT_SERIAL;
use crate::purge_tests::{purge_now, scene_erase, scene_event, scene_stay};
use crate::{testdb, App};

const TOKEN: &str = "test-token-0123456789abcdef";
const VISIT: &str = "c03-timeline-visit";
const WATCH: &str = "c03-youtube-watch";

/// 落ちても消える一時フォルダ。
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("ashiato-keep-{name}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn app() -> App {
    App {
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..App::for_test(testdb::pool().await, TOKEN)
    }
}

async fn get(app: &App, uri: &str) -> (StatusCode, Value) {
    let (code, body) = call(app, request(Method::GET, uri, true, None)).await;
    (code, serde_json::from_str(&body).unwrap_or(Value::Null))
}

fn sha_of(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

/// 消した滞在 1 つ（10:05〜11:48）と、その中の書庫の位置 3 件。
async fn scene_erased_visits(pool: &PgPool, user: Uuid) -> Uuid {
    let stay = scene_stay(
        pool,
        user,
        "2026-10-01T10:05:00+09:00",
        "2026-10-01T11:48:00+09:00",
    )
    .await;
    for minute in [10, 20, 30] {
        let when = format!("2026-10-01T10:{minute}:00+09:00");
        scene_event(pool, user, VISIT, &when, r#"{"lat":35.68,"lon":139.76}"#).await;
    }
    // 記録を直に置くので、取り込みが付ける収集開始日も付ける（共有の登録簿。ほかの試験が先に付けたかに依らない）
    crate::coverage::touch_started_on(
        pool,
        VISIT,
        "2026-10-01T10:10:00+09:00".parse().unwrap(),
        crate::coverage::Arrival::Record,
    )
    .await
    .unwrap();
    scene_erase(pool, stay).await;
    stay
}

/// 書庫の写し 1 個（ファイルを置き、台帳に「その書庫が `source` を読んだ」行と目録を足す）。
async fn put_copy(
    pool: &PgPool,
    user: Uuid,
    dir: &Path,
    source: &str,
    inserted: i32,
    duplicate: i32,
) -> PathBuf {
    let archive_sha = Uuid::new_v4().to_string();
    let file = dir.join(format!("copy-{}.json", Uuid::new_v4()));
    std::fs::write(&file, format!("写しの中身 {archive_sha}")).unwrap();
    let ledger: i64 = sqlx::query_scalar(
        "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome)
         VALUES ($1, $2, 'keep-test', 'read') RETURNING id",
    )
    .bind(user)
    .bind(&archive_sha)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.archive_ledger_source (ledger_id, logical_source, inserted_count, duplicate_count)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(ledger)
    .bind(source)
    .bind(inserted)
    .bind(duplicate)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.archive_file (sha256, user_id, inner_path, stored_path, archive_sha256)
         VALUES ($1, $2, 'Records.json', $3, $4)",
    )
    .bind(sha_of(&file))
    .bind(user)
    .bind(file.to_string_lossy().as_ref())
    .bind(&archive_sha)
    .execute(pool)
    .await
    .unwrap();
    file
}

async fn preview(app: &App, stay: Uuid) -> Value {
    let (code, body) = get(app, &format!("/stays/purge/preview?stay_id={stay}")).await;
    assert_eq!(code, StatusCode::OK, "{body}");
    body
}

// Scenario: 確認は書庫の写しが残ることを返す
#[tokio::test]
async fn purge_keep_preview_reports_the_archive_copies_that_remain() {
    let (app, user, dir) = (app().await, testdb::user(), Scratch::new("preview"));
    let stay = scene_erased_visits(&app.pool, user).await;
    put_copy(&app.pool, user, &dir.0, VISIT, 3, 0).await;
    put_copy(&app.pool, user, &dir.0, VISIT, 3, 0).await;

    let v = preview(&app, stay).await;

    assert_eq!(v["copies"], 2);
    let remains: Vec<&str> = v["remains"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect();
    assert_eq!(
        remains,
        [
            "archive_copies",
            "user_folder_archives",
            "export_files",
            "nightly_rewrite"
        ]
    );
}

// Scenario: 記録の入っていない写しも同じソースなら数える
#[tokio::test]
async fn purge_keep_counts_a_copy_without_the_erased_records_of_the_same_source() {
    let (app, user, dir) = (app().await, testdb::user(), Scratch::new("same-source"));
    let stay = scene_erased_visits(&app.pool, user).await;
    // 消した時間帯の位置が入っている写しと、同じソースで 1 件も新しく入れていない（重複だけの）写し
    put_copy(&app.pool, user, &dir.0, VISIT, 3, 0).await;
    put_copy(&app.pool, user, &dir.0, VISIT, 0, 5).await;
    // 数えないもの: 範囲に無いソースだけの写し・置き場から消えた写し・別の利用者の写し
    put_copy(&app.pool, user, &dir.0, WATCH, 4, 0).await;
    let gone = put_copy(&app.pool, user, &dir.0, VISIT, 1, 0).await;
    std::fs::remove_file(gone).unwrap();
    put_copy(&app.pool, testdb::user(), &dir.0, VISIT, 1, 0).await;

    let v = preview(&app, stay).await;

    assert_eq!(
        v["copies"], 2,
        "1 と数えない。別のソース・消えた写し・他人の写しは数えない"
    );
}

// Scenario: 本文ごと消しても書庫の写しは残る
#[tokio::test]
async fn purge_keep_archive_copies_stay_byte_for_byte() {
    let (app, user, dir) = (app().await, testdb::user(), Scratch::new("copies"));
    let stay = scene_erased_visits(&app.pool, user).await;
    let files = [
        put_copy(&app.pool, user, &dir.0, VISIT, 3, 0).await,
        put_copy(&app.pool, user, &dir.0, VISIT, 0, 5).await,
    ];
    let before: Vec<String> = files.iter().map(|f| sha_of(f)).collect();

    let outcome = purge_now(&app.pool, stay).await;

    assert_eq!(
        outcome.counts.archive_locations, 3,
        "前提: 書庫の位置を消した"
    );
    for (file, sha) in files.iter().zip(&before) {
        assert!(file.exists(), "書庫の写しが置き場から消えた");
        assert_eq!(&sha_of(file), sha, "書庫の写しの中身が変わった");
    }
}

/// 全記録を書き出した後の世界。書き出しに入る滞在 1 つと位置 1 件を置いてある。
struct Exported {
    w: World,
    app: App,
    stay: Uuid,
    before: Value,
}

/// 全記録の書き出しは DB 全体を読むので、共有の DB ではなく使い捨ての DB（`World::new`）で走らせる。
async fn exported(dir: &Path) -> Exported {
    let w = World::new().await;
    let stay = w.stay(&w.at("10:00"), &w.at("11:00")).await;
    w.put(
        "c01-location",
        &w.at("10:30"),
        json!({"lat": 35.0, "lon": 139.0}),
        r#"{"lat":35.0,"lon":139.0}"#,
    )
    .await;
    let app = app_with(&w.pool, dir, Hooks::none());
    let (code, body) = post_status(&app, PASS).await;
    assert_eq!(code, StatusCode::ACCEPTED, "{body}");
    wait_idle(&app).await;
    let before = get_status(&app).await;
    Exported {
        w,
        app,
        stay,
        before,
    }
}

/// 置き場の下の書き終えた書き出し。
fn export_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            !n.starts_with('.') && n.ends_with(".zip")
        })
        .collect();
    v.sort();
    v
}

// Scenario: 本文ごと消しても書き出したファイルは残る
#[tokio::test]
async fn purge_keep_export_file_stays_byte_for_byte() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = Scratch::new("export-file");
    let e = exported(&dir.0).await;
    let files = export_files(&dir.0);
    assert_eq!(files.len(), 1, "前提: 書き出したファイルが 1 つ");
    let before = sha_of(&files[0]);
    scene_erase(&e.w.pool, e.stay).await;

    let outcome = purge_now(&e.w.pool, e.stay).await;

    assert_eq!(outcome.counts.stays, 1, "前提: 本文を消した");
    assert_eq!(
        export_files(&dir.0),
        files,
        "書き出したファイルが置き場から消えた"
    );
    assert_eq!(
        sha_of(&files[0]),
        before,
        "書き出したファイルの中身が変わった"
    );
}

// Scenario: 書き出しの後に本文ごと消した記録は後に消した数に入る
#[tokio::test]
async fn purge_keep_erased_after_the_export_counts_the_purged_records() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = Scratch::new("export-count");
    let e = exported(&dir.0).await;
    scene_erase(&e.w.pool, e.stay).await;
    let erased = get_status(&e.app).await["last"]["erased_after"]
        .as_i64()
        .unwrap();

    let outcome = purge_now(&e.w.pool, e.stay).await;

    let after = get_status(&e.app).await["last"]["erased_after"]
        .as_i64()
        .unwrap();
    let before = e.before["last"]["erased_after"].as_i64().unwrap();
    assert_eq!(
        outcome.counts.stays + outcome.counts.locations,
        2,
        "前提: 滞在と位置の本文を消した"
    );
    assert_eq!(
        after - before,
        2,
        "本文を消した滞在と位置が後に消した数に入っている"
    );
    assert_eq!(after, erased, "本文を消しても数は変わらない");
}

/// 書庫の位置のソースの日ごとの状態 `(日, 状態)`。
async fn visit_states(app: &App, user: Uuid) -> Vec<(String, String)> {
    let (code, body) = get(
        app,
        &format!("/coverage?from=2026-09-30&to=2026-10-03&user_id={user}"),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{body}");
    let source = body
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["logical_source"] == VISIT)
        .expect("書庫の位置のソースが稼働状況に出る");
    assert!(
        source["expected_gap_sec"].as_i64().unwrap() >= 86_400,
        "前提: 想定間隔は 1 日以上"
    );
    source["days"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["day"].as_str().unwrap().to_owned(),
                d["state"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn state_of<'a>(states: &'a [(String, String)], day: &str) -> &'a str {
    &states.iter().find(|(d, _)| d == day).unwrap().1
}

// Scenario: 本文ごと消しても稼働状況は記録ありのまま
#[tokio::test]
async fn purge_keep_coverage_stays_recorded_after_the_purge() {
    let (app, user) = (app().await, testdb::user());
    let stay = scene_erased_visits(&app.pool, user).await;
    let before = visit_states(&app, user).await;
    assert_eq!(
        state_of(&before, "2026-10-01"),
        "recorded",
        "前提: 消した後も記録あり"
    );

    purge_now(&app.pool, stay).await;

    let after = visit_states(&app, user).await;
    assert_eq!(state_of(&after, "2026-10-01"), "recorded");
    assert_eq!(after, before, "本文を消しても稼働状況は 1 日も変わらない");
}

// Scenario: 記録を消しても隣の記録の無い日の状態は変わらない
#[tokio::test]
async fn purge_keep_coverage_of_the_next_empty_day_does_not_change() {
    let (app, user) = (app().await, testdb::user());
    let stay = scene_erased_visits(&app.pool, user).await;
    let before = visit_states(&app, user).await;
    assert_ne!(
        state_of(&before, "2026-10-02"),
        state_of(&before, "2026-10-01"),
        "前提: 隣の日には記録が無い"
    );

    purge_now(&app.pool, stay).await;

    let after = visit_states(&app, user).await;
    assert_eq!(
        state_of(&after, "2026-10-02"),
        state_of(&before, "2026-10-02")
    );
    assert_eq!(
        state_of(&after, "2026-09-30"),
        state_of(&before, "2026-09-30")
    );
}
