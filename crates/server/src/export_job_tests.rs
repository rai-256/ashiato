// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの口（`POST /export`・`GET /export`）と背景の仕事と台帳（ST33 tasks 6.1〜6.4 / design D1 / D2 / D10 / D11 / D16 / D17）。
//!
//! 書き出しの台帳・状態・置き場は全利用者で 1 つなので、走らせるテストは `EXPORT_SERIAL` で直列にする。
//! 記録と消す滞在は `testdb::unique_day()` の日に置く（時刻でも隔離する。R27）。
//! 後に消した記録の件数は共有の DB 全体の数なので、**消す前の値との差**で確かめる。
#![allow(clippy::unwrap_used)]

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use serde_json::{json, Value};
use tower::ServiceExt as _;
use uuid::Uuid;

use crate::export::{self, ExportConfig, ExportError, FailKind, Hooks, Stage};
use crate::export_excluded_tests::World;
use crate::export_tests::{names, tmp_dir, EXPORT_SERIAL};
use crate::{testdb, App};

const TOKEN: &str = "test-token-0123456789abcdef";
const PASS: &str = "job-test-passphrase-0123";

// ---------------------------------------------------------------------------
// 道具
// ---------------------------------------------------------------------------

fn cfg(dir: &Path, hooks: Hooks) -> ExportConfig {
    let mut c = ExportConfig::with_dir(dir.to_path_buf());
    c.hooks = hooks;
    c
}

fn app_with(pool: &sqlx::PgPool, dir: &Path, hooks: Hooks) -> App {
    App {
        // 滞在を消したときの作り直しは走らせない（書き出しの外の振る舞い）
        stays: crate::StayRebuilder::from_fn(|_, _, _| Box::pin(async { Ok(()) })),
        ..App::for_test(pool.clone(), TOKEN).with_export(cfg(dir, hooks))
    }
}

async fn app_in(dir: &Path) -> App {
    app_with(&testdb::pool().await, dir, Hooks::none())
}

fn request(method: Method, uri: &str, auth: bool, body: Option<String>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri);
    if auth {
        b = b.header(header::AUTHORIZATION, format!("Bearer {TOKEN}"));
    }
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    b.body(body.map_or_else(Body::empty, Body::from)).unwrap()
}

async fn call(app: &App, req: Request<Body>) -> (StatusCode, String) {
    let res = crate::router(app.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

fn post(password: &str) -> Request<Body> {
    request(
        Method::POST,
        "/export",
        true,
        Some(json!({ "password": password }).to_string()),
    )
}

async fn post_status(app: &App, password: &str) -> (StatusCode, String) {
    call(app, post(password)).await
}

async fn wait_idle(app: &App) {
    for _ in 0..3000 {
        if app.exports.running_view().is_none() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("書き出しが終わらない");
}

/// 書き出しを求め（202）、終わるまで待って、その識別子を返す。
async fn export_and_wait(app: &App, password: &str) -> Uuid {
    let (code, body) = post_status(app, password).await;
    assert_eq!(code, StatusCode::ACCEPTED, "{body}");
    wait_idle(app).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    v["export_id"].as_str().unwrap().parse().unwrap()
}

async fn get_status(app: &App) -> Value {
    let (code, body) = call(app, request(Method::GET, "/export", true, None)).await;
    assert_eq!(code, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}

async fn ledger_of(pool: &sqlx::PgPool, id: Uuid) -> Vec<export::LedgerRow> {
    export::read_ledger(pool)
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.export_id == id)
        .collect()
}

/// 置き場の下の書き終えた書き出し（`.` で始まらない `.zip`）。
fn zips(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = names(dir)
        .into_iter()
        .filter(|n| !n.starts_with('.') && n.ends_with(".zip"))
        .map(|n| dir.join(n))
        .collect();
    v.sort();
    v
}

fn sha(path: &Path) -> String {
    export::sha256_of(path).unwrap()
}

/// 項目 `name` の行の数（合言葉で開く）。
fn line_count(path: &Path, name: &str) -> u64 {
    let mut z = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut f = z.by_name_decrypt(name, PASS.as_bytes()).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    s.lines().count() as u64
}

fn auth_headers() -> axum::http::HeaderMap {
    let mut h = axum::http::HeaderMap::new();
    h.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    h
}

/// 本人が消した滞在を、`/stays/erase` の口で消す。
async fn erase(app: &App, stay: Uuid) {
    let _ = crate::stays_erase(
        axum::extract::State(app.clone()),
        auth_headers(),
        axum::Json(crate::EraseRequest {
            stay_id: stay,
            user_id: None,
        }),
    )
    .await
    .unwrap();
}

/// 登録簿の基準のソースの位置を、滞在の中に 1 件置く。
async fn location_at(w: &World, hhmm: &str) -> Uuid {
    w.put(
        "c01-location",
        &w.at(hhmm),
        json!({"lat": 35.0, "lon": 139.0}),
        r#"{"lat":35.0,"lon":139.0}"#,
    )
    .await
}

/// 最初の項目（記録）を書き始める前に、許可が出るまで止める。
fn hold_at(file: &'static str, gate: Arc<tokio::sync::Semaphore>) -> Hooks {
    Hooks::none().with_before_file(move |name| {
        let gate = gate.clone();
        Box::pin(async move {
            if name == file {
                gate.acquire().await.unwrap().forget();
            }
        })
    })
}

fn fail_at(stage: Stage, kind: FailKind) -> Hooks {
    Hooks::none().with_fail(move |s| (s == stage).then(|| ExportError::new(kind, "差し込んだ失敗")))
}

async fn wait_until_blocked(app: &App) -> Value {
    for _ in 0..600 {
        let st = get_status(app).await;
        if st["running"]["total"].as_u64().unwrap_or(0) > 0 {
            return st;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("書き出しが走り始めない");
}

// ---------------------------------------------------------------------------
// 6.1 口
// ---------------------------------------------------------------------------

// Scenario: 範囲を指定した書き出しの求めは断られる
#[tokio::test]
async fn export_endpoint_range_arguments_are_refused() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("range");
    let app = app_in(&dir).await;
    let body = json!({ "password": PASS }).to_string();
    let (code, res) = call(
        &app,
        request(
            Method::POST,
            "/export?from=2020-01-01&source=c01-location",
            true,
            Some(body),
        ),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST, "{res}");
    assert_eq!(
        serde_json::from_str::<Value>(&res).unwrap()["error"],
        "unexpected_field"
    );

    let (code, res) = call(
        &app,
        request(
            Method::POST,
            "/export",
            true,
            Some(json!({ "password": PASS, "from": "2020-01-01" }).to_string()),
        ),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST, "{res}");
    assert_eq!(
        serde_json::from_str::<Value>(&res).unwrap()["error"],
        "unexpected_field"
    );
    assert!(names(&dir).is_empty(), "置き場に新しいファイルができた");
    assert!(app.exports.running_view().is_none());
}

// Scenario: 書き出し中にもう一度求めると断られる
#[tokio::test]
async fn export_endpoint_a_second_request_while_running_is_refused() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("second");
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let app = app_with(
        &testdb::pool().await,
        &dir,
        hold_at("events.jsonl", gate.clone()),
    );
    let (code, _) = post_status(&app, PASS).await;
    assert_eq!(code, StatusCode::ACCEPTED);
    wait_until_blocked(&app).await;
    let before = names(&dir);

    let (code, res) = post_status(&app, PASS).await;
    assert_eq!(code, StatusCode::CONFLICT, "{res}");
    assert_eq!(
        serde_json::from_str::<Value>(&res).unwrap()["error"],
        "export_running"
    );
    assert_eq!(names(&dir), before, "2 度目の求めで置き場が変わった");

    gate.add_permits(1);
    wait_idle(&app).await;
    assert_eq!(zips(&dir).len(), 1, "書き出しは 1 つだけ書かれる");
}

// Scenario: 資格情報の無い書き出しの求めは断られる
#[tokio::test]
async fn export_endpoint_without_credentials_is_unauthorized() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("noauth");
    let app = app_in(&dir).await;
    let (code, _) = call(
        &app,
        request(
            Method::POST,
            "/export",
            false,
            Some(json!({ "password": PASS }).to_string()),
        ),
    )
    .await;
    assert_eq!(code, StatusCode::UNAUTHORIZED);
    assert!(names(&dir).is_empty());
    assert!(app.exports.running_view().is_none());
}

// Scenario: 合言葉の無い・短い書き出しの求めは断られる
#[tokio::test]
async fn export_endpoint_missing_or_short_password_is_refused() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("short");
    let app = app_in(&dir).await;
    let error_of = |res: &str| serde_json::from_str::<Value>(res).unwrap()["error"].clone();

    let (code, res) = call(&app, request(Method::POST, "/export", true, None)).await;
    assert_eq!(
        (code, error_of(&res)),
        (StatusCode::BAD_REQUEST, json!("password_missing"))
    );
    let (code, res) = call(
        &app,
        request(Method::POST, "/export", true, Some("{}".into())),
    )
    .await;
    assert_eq!(
        (code, error_of(&res)),
        (StatusCode::BAD_REQUEST, json!("password_missing"))
    );
    let (code, res) = post_status(&app, "01234567890").await; // 11 文字
    assert_eq!(
        (code, error_of(&res)),
        (StatusCode::BAD_REQUEST, json!("password_too_short"))
    );
    assert!(!res.contains("01234567890"), "応答に合言葉が出た");
    assert!(names(&dir).is_empty(), "置き場に新しいファイルができた");

    // 境界: 12 文字ちょうどは受け付けられる
    let (code, res) = post_status(&app, "012345678901").await;
    assert_eq!(code, StatusCode::ACCEPTED, "{res}");
    wait_idle(&app).await;
    assert_eq!(zips(&dir).len(), 1);
}

#[tokio::test]
async fn export_endpoint_counts_code_points_not_bytes() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("emoji");
    let app = app_in(&dir).await;
    // 基本多言語面の外の文字 1 つは UTF-16 では 2 単位、UTF-8 では 4 バイト。符号位置で 11 個は短い
    let (code, res) = post_status(&app, &"🔑".repeat(11)).await;
    assert_eq!(code, StatusCode::BAD_REQUEST, "{res}");
    assert_eq!(
        serde_json::from_str::<Value>(&res).unwrap()["error"],
        "password_too_short"
    );
    assert!(names(&dir).is_empty());
    let (code, res) = post_status(&app, &"🔑".repeat(12)).await;
    assert_eq!(code, StatusCode::ACCEPTED, "{res}");
    wait_idle(&app).await;
    assert_eq!(zips(&dir).len(), 1);
}

#[tokio::test]
async fn export_endpoint_does_not_trim_the_password() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("trim");
    let app = app_in(&dir).await;
    // 空白を含めて 12 文字。削ると 10 文字になるが、本人が打ったものがそのまま鍵になる
    let (code, res) = post_status(&app, "  abcdefghij  ").await;
    assert_eq!(code, StatusCode::ACCEPTED, "{res}");
    wait_idle(&app).await;
}

// Scenario: 消した時間に後から届いて印の無い位置には、書き出しの前に印が付く
#[tokio::test]
async fn export_endpoint_late_arrivals_are_marked_before_the_dump() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = World::new().await;
    let dir = tmp_dir("late");
    let app = app_in(&dir).await;
    let stay = w.stay(&w.at("10:00"), &w.at("11:00")).await;
    erase(&app, stay).await;
    // 作り直しが走らないまま、消した時間に印の無い位置が入った
    let basis = location_at(&w, "10:30").await;
    let archive = w
        .put(
            "c03-timeline-visit",
            &w.at("10:40"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    let live = |id: Uuid| {
        let pool = w.pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM core.event_live WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    assert_eq!(
        (live(basis).await, live(archive).await),
        (1, 1),
        "前提: 印は無い"
    );

    export_and_wait(&app, PASS).await;

    let (code, events) = call(&app, request(Method::GET, "/events", true, None)).await;
    assert_eq!(code, StatusCode::OK);
    for id in [basis, archive] {
        assert!(
            !events.contains(&id.to_string()),
            "印の付いた記録が読み出しに出た"
        );
        let marks: Vec<String> =
            sqlx::query_scalar("SELECT mark FROM core.deletion_ledger WHERE event_id = $1")
                .bind(id)
                .fetch_all(&w.pool)
                .await
                .unwrap();
        assert_eq!(
            marks,
            vec!["user:late".to_string()],
            "削除の台帳に user:late の行が 1 つ"
        );
    }
}

// ---------------------------------------------------------------------------
// 6.2 台帳と前のファイル
// ---------------------------------------------------------------------------

// Scenario: 書き出すと台帳に始まりと終わりの行が積まれる
#[tokio::test]
async fn export_ledger_rows_start_and_finish_are_appended() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = World::new().await;
    location_at(&w, "09:00").await;
    let dir = tmp_dir("ledger");
    let app = app_in(&dir).await;
    let id = export_and_wait(&app, PASS).await;

    let rows = ledger_of(&w.pool, id).await;
    assert_eq!(rows.len(), 2, "{rows:?}");
    let (start, finish) = (&rows[0], &rows[1]);
    assert_eq!(
        (start.kind.as_str(), finish.kind.as_str()),
        ("start", "finish")
    );
    assert!(start.result.is_none());
    assert_eq!(finish.result.as_deref(), Some("done"));
    let file = zips(&dir).pop().unwrap();
    assert_eq!(finish.file, file.to_string_lossy());
    assert_eq!(
        start.file, finish.file,
        "始まりの行の名前は実際に付けた名前と同じ"
    );
    let sizes = finish.rows.as_ref().unwrap().as_object().unwrap();
    assert_eq!(sizes.len(), 10, "表ごとの行の数: {sizes:?}");
    assert_eq!(
        finish.bytes,
        Some(std::fs::metadata(&file).unwrap().len() as i64)
    );
    assert_eq!(finish.digest.as_deref(), Some(sha(&file).as_str()));
    let (snapshot, written): (Option<chrono::DateTime<chrono::Utc>>, Option<Vec<u8>>) =
        sqlx::query_as(
            "SELECT snapshot_at, written FROM core.export_ledger
              WHERE export_id = $1 AND kind = 'finish'",
        )
        .bind(id)
        .fetch_one(&w.pool)
        .await
        .unwrap();
    assert!(snapshot.is_some() && written.is_some_and(|b| !b.is_empty()));
    let start_snapshot: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT snapshot_at FROM core.export_ledger WHERE export_id = $1 AND kind = 'start'",
    )
    .bind(id)
    .fetch_one(&w.pool)
    .await
    .unwrap();
    assert!(start_snapshot.is_some());
}

// Scenario: 台帳の行は経路を持ち、記録の中身を持たない
#[tokio::test]
async fn export_ledger_rows_have_a_route_and_no_content() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = World::new().await;
    let marker = format!("LEDGER-RAW-{}", Uuid::new_v4());
    w.put(
        "c02-window",
        &w.at("09:00"),
        json!({"title": marker, "lat": 35.6891234, "lon": 139.7012345}),
        &format!(r#"{{"title":"{marker}","lat":35.6891234}}"#),
    )
    .await;
    let dir = tmp_dir("ledger-content");
    let app = app_in(&dir).await;
    let phrase = format!("ledger-secret-{}", Uuid::new_v4());
    let id = export_and_wait(&app, &phrase).await;

    let texts: Vec<String> = sqlx::query_scalar(
        "SELECT l::text FROM core.export_ledger l WHERE export_id = $1 ORDER BY seq",
    )
    .bind(id)
    .fetch_all(&w.pool)
    .await
    .unwrap();
    assert_eq!(texts.len(), 2);
    let route: String = sqlx::query_scalar(
        "SELECT route FROM core.export_ledger WHERE export_id = $1 AND kind = 'start'",
    )
    .bind(id)
    .fetch_one(&w.pool)
    .await
    .unwrap();
    assert_eq!(route, "pc-encrypted-zip");
    for t in &texts {
        for bad in [
            marker.as_str(),
            "35.6891234",
            "139.7012345",
            phrase.as_str(),
        ] {
            assert!(!t.contains(bad), "台帳の行に {bad} が含まれる");
        }
    }
}

// Scenario: 失敗した書き出しも台帳に残る
#[tokio::test]
async fn export_ledger_rows_a_failure_at_the_late_mark_stage_is_recorded() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("fail-late");
    let pool = testdb::pool().await;
    let app = app_with(&pool, &dir, fail_at(Stage::LateMark, FailKind::LateMark));
    let id = export_and_wait(&app, PASS).await;
    let rows = ledger_of(&pool, id).await;
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0].kind, "start");
    assert_eq!(
        (rows[1].kind.as_str(), rows[1].result.as_deref()),
        ("finish", Some("failed"))
    );
    assert!(rows[1].reason.as_deref().unwrap().starts_with("late_mark"));
    assert!(names(&dir).is_empty(), "失敗で書き出しのファイルは残らない");
}

#[tokio::test]
async fn export_ledger_rows_a_failure_at_the_classification_stage_names_the_table() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("fail-class");
    let pool = testdb::pool().await;
    let hooks = Hooks::none().with_inventory(|inv| inv.tables.push("zzz_unclassified".into()));
    let app = app_with(&pool, &dir, hooks);
    let id = export_and_wait(&app, PASS).await;
    let rows = ledger_of(&pool, id).await;
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[1].result.as_deref(), Some("failed"));
    let reason = rows[1].reason.as_deref().unwrap();
    assert!(
        reason.starts_with("unclassified_table") && reason.contains("zzz_unclassified"),
        "{reason}"
    );
    assert!(names(&dir).is_empty());
}

// Scenario: 書き終えると前の書き出しのファイルは消え、1 つだけ残る
#[tokio::test]
async fn export_ledger_rows_only_the_latest_export_file_remains() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("one-left");
    let app = app_in(&dir).await;
    export_and_wait(&app, PASS).await;
    let first = zips(&dir).pop().unwrap();
    // 名前の日時は秒まで。同じ名前でも `-2` が付く
    let second_id = export_and_wait(&app, PASS).await;
    let left = zips(&dir);
    assert_eq!(left.len(), 1, "{left:?}");
    let rows = ledger_of(&testdb::pool().await, second_id).await;
    assert_eq!(rows[1].file, left[0].to_string_lossy());
    assert!(!first.exists(), "1 回目のファイルが残った");
}

// Scenario: 書き出しが失敗すると前の書き出しのファイルは残る
#[tokio::test]
async fn export_ledger_rows_a_failed_export_keeps_the_previous_file() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("keep-prev");
    let pool = testdb::pool().await;
    let ok = app_with(&pool, &dir, Hooks::none());
    export_and_wait(&ok, PASS).await;
    let first = zips(&dir).pop().unwrap();
    let (size, digest) = (std::fs::metadata(&first).unwrap().len(), sha(&first));

    let failing = app_with(&pool, &dir, fail_at(Stage::Write, FailKind::Io));
    export_and_wait(&failing, PASS).await;

    assert!(first.exists(), "前のファイルが消えた");
    assert_eq!(std::fs::metadata(&first).unwrap().len(), size);
    assert_eq!(sha(&first), digest);
    assert_eq!(zips(&dir), vec![first]);
}

// Scenario: このシステムが書き出していないファイルは消さない
#[tokio::test]
async fn export_ledger_rows_files_the_system_did_not_write_are_kept() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("not-ours");
    std::fs::write(dir.join("ashiato-export-19990101-000000.zip"), b"mine-zip").unwrap();
    std::fs::write(dir.join("my-notes.txt"), b"mine-notes").unwrap();
    let app = app_in(&dir).await;
    export_and_wait(&app, PASS).await;
    let first = zips(&dir)
        .into_iter()
        .find(|p| !p.ends_with("ashiato-export-19990101-000000.zip"))
        .unwrap();
    let copy = dir.join("copy-of-export.zip");
    std::fs::copy(&first, &copy).unwrap();
    let copy_digest = sha(&copy);

    export_and_wait(&app, PASS).await;

    assert_eq!(
        std::fs::read(dir.join("ashiato-export-19990101-000000.zip")).unwrap(),
        b"mine-zip"
    );
    assert_eq!(
        std::fs::read(dir.join("my-notes.txt")).unwrap(),
        b"mine-notes"
    );
    assert_eq!(
        sha(&copy),
        copy_digest,
        "本人が写したファイルは中身ごと残る"
    );
}

fn replace_with_foreign_zip(path: &Path) {
    let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    w.set_comment("ashiato-export someone-else");
    w.start_file("a.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    w.write_all(b"mine").unwrap();
    w.finish().unwrap();
}

// Scenario: 前の書き出しのファイルを本人が置き換えていれば消さない
#[tokio::test]
async fn export_ledger_rows_a_file_the_user_replaced_is_kept() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("replaced");
    let app = app_in(&dir).await;
    export_and_wait(&app, PASS).await;
    let first = zips(&dir).pop().unwrap();
    replace_with_foreign_zip(&first);
    let digest = sha(&first);

    export_and_wait(&app, PASS).await;

    assert!(first.exists(), "置き換えたファイルが消えた");
    assert_eq!(sha(&first), digest);
    let st = get_status(&app).await;
    assert!(
        !st["leftover"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == first.to_str().unwrap()),
        "本人が置き換えたファイルが消せなかったファイルに出た"
    );
}

// Scenario: 前の書き出しのファイルに本人が項目を足していれば消さない
#[tokio::test]
async fn export_ledger_rows_a_file_the_user_appended_to_is_kept() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("appended");
    let app = app_in(&dir).await;
    export_and_wait(&app, PASS).await;
    let first = zips(&dir).pop().unwrap();
    {
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&first)
            .unwrap();
        let mut w = zip::ZipWriter::new_append(f).unwrap();
        w.start_file("note.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(b"added").unwrap();
        w.finish().unwrap();
    }
    let z = zip::ZipArchive::new(std::fs::File::open(&first).unwrap()).unwrap();
    assert!(
        z.comment().starts_with(b"ashiato-export "),
        "前提: 項目を足しても注記は残る"
    );
    let digest = sha(&first);

    export_and_wait(&app, PASS).await;

    assert!(first.exists(), "項目を足したファイルが消えた");
    assert_eq!(sha(&first), digest);
    let st = get_status(&app).await;
    assert!(!st["leftover"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f == first.to_str().unwrap()));
}

// ---------------------------------------------------------------------------
// 6.3 状態
// ---------------------------------------------------------------------------

// Scenario: 書き出しの間は進み具合が行の数で読める
#[tokio::test]
async fn export_status_progress_is_readable_in_rows_while_running() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = World::new().await;
    location_at(&w, "09:00").await;
    let dir = tmp_dir("progress");
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let app = app_with(
        &testdb::pool().await,
        &dir,
        hold_at("event_versions.jsonl", gate.clone()),
    );
    let (code, _) = post_status(&app, PASS).await;
    assert_eq!(code, StatusCode::ACCEPTED);
    let st = wait_until_blocked(&app).await;
    let running = &st["running"];
    let (written, total) = (
        running["written"].as_u64().unwrap(),
        running["total"].as_u64().unwrap(),
    );
    assert!(
        total > 0 && written <= total,
        "written={written} total={total}"
    );
    assert!(running["export_id"].is_string() && running["started_at"].is_string());
    assert!(st["last_failure"].is_null() || st["last_failure"].is_object());

    gate.add_permits(1);
    wait_idle(&app).await;
    assert!(get_status(&app).await["running"].is_null());
}

// Scenario: 書き終えると前回の書き出しが読める
#[tokio::test]
async fn export_status_last_export_is_readable_after_it_finishes() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = World::new().await;
    location_at(&w, "09:00").await;
    let dir = tmp_dir("last");
    let app = app_in(&dir).await;
    export_and_wait(&app, PASS).await;
    let file = zips(&dir).pop().unwrap();

    let last = &get_status(&app).await["last"];
    assert!(last["finished_at"].is_string());
    assert_eq!(last["file"], file.to_str().unwrap());
    assert_eq!(
        last["bytes"].as_u64().unwrap(),
        std::fs::metadata(&file).unwrap().len()
    );
    let rows: u64 = export::CATALOG
        .iter()
        .filter_map(|(_, c)| match c {
            export::Class::Include { file, .. } => Some(*file),
            export::Class::Exclude { .. } => None,
        })
        .map(|name| line_count(&file, name))
        .sum();
    assert_eq!(
        last["rows"].as_u64().unwrap(),
        rows,
        "rows は zip の中の各ファイルの行の和"
    );
}

// Scenario: 1 度も書き出していなければそう返る
#[test]
fn export_status_never_exported_is_reported_by_the_pure_function() {
    let status = export::derive(&[], None).into_status(None, 0, |_| true, |_| true);
    assert!(status.last.is_none() && status.last_failure.is_none());
    assert!(status.interrupted.is_none() && status.running.is_none());
    assert!(status.leftover.is_empty());
}

// Scenario: 失敗しても前回の書き出しの値は変わらない
#[tokio::test]
async fn export_status_a_failure_does_not_change_the_last_values() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("fail-last");
    let pool = testdb::pool().await;
    let ok = app_with(&pool, &dir, Hooks::none());
    export_and_wait(&ok, PASS).await;
    let before = get_status(&ok).await;

    let failing = app_with(&pool, &dir, fail_at(Stage::Write, FailKind::Io));
    export_and_wait(&failing, PASS).await;
    let after = get_status(&failing).await;

    assert_eq!(after["last_failure"]["kind"], "io");
    assert!(after["last_failure"]["at"].is_string());
    assert_eq!(after["last"]["finished_at"], before["last"]["finished_at"]);
    assert_eq!(after["last"]["rows"], before["last"]["rows"]);
    assert_eq!(after["last"]["bytes"], before["last"]["bytes"]);
    assert_eq!(after["last"]["file"], before["last"]["file"]);
    assert!(after["running"].is_null());
    assert!(
        before["last_failure"].is_null()
            || before["last_failure"]["at"] != after["last_failure"]["at"]
    );
}

// Scenario: 途中で止まった書き出しはそう返り、次の書き出しを始められる
#[tokio::test]
async fn export_status_an_interrupted_export_is_reported_and_the_next_can_start() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("interrupted");
    let pool = testdb::pool().await;
    export_and_wait(&app_with(&pool, &dir, Hooks::none()), PASS).await;
    let first = zips(&dir).pop().unwrap();

    // 2 回目は `rename` の後・台帳の終わりの行の前に止まる
    let halting = app_with(&pool, &dir, Hooks::none().with_halt(|s| s == Stage::Finish));
    let (code, _) = post_status(&halting, PASS).await;
    assert_eq!(code, StatusCode::ACCEPTED);
    wait_idle(&halting).await;
    let stopped = zips(&dir)
        .into_iter()
        .find(|p| *p != first)
        .expect("止まった書き出しのファイル");

    // 新しい App（再起動）で読む
    let fresh = app_with(&pool, &dir, Hooks::none());
    let st = get_status(&fresh).await;
    assert_eq!(st["interrupted"]["file"], stopped.to_str().unwrap());
    assert!(st["interrupted"]["started_at"].is_string());
    assert_eq!(st["last"]["file"], first.to_str().unwrap());
    let leftover = st["leftover"].as_array().unwrap();
    assert!(
        !leftover.iter().any(|f| f == stopped.to_str().unwrap()),
        "止まったファイルが消せなかったファイルに出た"
    );

    // 次の書き出しを始められ、書き終えても止まったファイルは残る
    export_and_wait(&fresh, PASS).await;
    assert!(stopped.exists(), "止まった書き出しのファイルが消えた");
    assert!(
        !first.exists(),
        "書き終えた 1 回目のファイルは次の書き出しが消す"
    );
    let st = get_status(&fresh).await;
    assert!(st["interrupted"].is_null());
}

/// 滞在とその中の位置を置き、書き出した後に滞在を消す前の `erased_after`、消す関数を返す。
struct Erasure {
    w: World,
    app: App,
    stay: Uuid,
    before: Value,
}

async fn exported_stay_world(dir: &Path, with_photo: bool) -> (Erasure, Option<Uuid>) {
    let w = World::new().await;
    let stay = w.stay(&w.at("10:00"), &w.at("11:00")).await;
    location_at(&w, "10:30").await;
    let photo = if with_photo {
        Some(
            w.put(
                "c01-photo",
                &w.at("10:45"),
                json!({"lat": 35.0, "lon": 139.0}),
                "{}",
            )
            .await,
        )
    } else {
        None
    };
    let app = app_with(&w.pool, dir, Hooks::none());
    export_and_wait(&app, PASS).await;
    let before = get_status(&app).await;
    (
        Erasure {
            w,
            app,
            stay,
            before,
        },
        photo,
    )
}

// Scenario: 書き出した後に記録を消すと、状態に後に消した記録の件数が出る
#[tokio::test]
async fn export_status_erasing_after_the_export_shows_the_erased_count() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("erased");
    let (e, _) = exported_stay_world(&dir, false).await;
    erase(&e.app, e.stay).await;
    let after = get_status(&e.app).await;
    let delta = after["last"]["erased_after"].as_i64().unwrap()
        - e.before["last"]["erased_after"].as_i64().unwrap();
    assert_eq!(delta, 2, "消した滞在と、連鎖で印の付いた位置");
    for k in ["finished_at", "rows", "bytes", "file"] {
        assert_eq!(after["last"][k], e.before["last"][k], "{k} は消す前と同じ");
    }
}

// Scenario: 後に消した記録の件数は、印の無い別のソースの行も数える
#[tokio::test]
async fn export_status_erased_count_includes_unmarked_rows_of_another_source() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("erased-photo");
    let (e, photo) = exported_stay_world(&dir, true).await;
    let photo = photo.unwrap();
    erase(&e.app, e.stay).await;
    let (marked,): (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT deleted_at FROM core.event WHERE id = $1")
            .bind(photo)
            .fetch_one(&e.w.pool)
            .await
            .unwrap();
    assert!(marked.is_none(), "前提: 写真の位置は連鎖で印が付かない");
    let after = get_status(&e.app).await;
    let delta = after["last"]["erased_after"].as_i64().unwrap()
        - e.before["last"]["erased_after"].as_i64().unwrap();
    assert_eq!(
        delta, 3,
        "消した滞在と連鎖で印が付いた位置に、印の無い写真の位置を足した数"
    );
}

// Scenario: 作り直しで置き換わった滞在は後に消した記録に数えない
#[tokio::test]
async fn export_status_rebuild_replacements_are_not_counted_as_erased() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("rebuilt");
    let (e, _) = exported_stay_world(&dir, false).await;
    // 作り直しが置き換えた滞在は作り直しの印（`rebuild:`）で消える。本人は何も消していない
    e.w.mark_deleted(
        e.stay,
        &format!("{}test", crate::stay_store::REBUILD_PREFIX),
    )
    .await;
    let after = get_status(&e.app).await;
    assert_eq!(
        after["last"]["erased_after"],
        e.before["last"]["erased_after"]
    );
}

// Scenario: 書き出し直すと後に消した記録の件数は 0 に戻る
#[tokio::test]
async fn export_status_exporting_again_resets_the_erased_count() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("reset");
    let (e, _) = exported_stay_world(&dir, false).await;
    erase(&e.app, e.stay).await;
    export_and_wait(&e.app, PASS).await;
    assert_eq!(get_status(&e.app).await["last"]["erased_after"], 0);
}

// Scenario: 前のファイルを消せなければ、書き出しは書き終えたまま、状態にそのファイルが出る
#[tokio::test]
async fn export_status_a_previous_file_that_cannot_be_deleted_is_listed() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("leftover");
    let pool = testdb::pool().await;
    export_and_wait(&app_with(&pool, &dir, Hooks::none()), PASS).await;
    let first = zips(&dir).pop().unwrap();

    let stuck = app_with(&pool, &dir, fail_at(Stage::DeletePrevious, FailKind::Io));
    let second_id = export_and_wait(&stuck, PASS).await;
    let rows = ledger_of(&pool, second_id).await;
    assert_eq!(
        rows[1].result.as_deref(),
        Some("done"),
        "書き出しは書き終えたまま"
    );
    let st = get_status(&stuck).await;
    assert_eq!(st["leftover"], json!([first.to_str().unwrap()]));
    assert!(first.exists());
    assert_ne!(st["last"]["file"], first.to_str().unwrap());

    // 次の書き出しが書き終えたときに、もう一度消す
    let ok = app_with(&pool, &dir, Hooks::none());
    export_and_wait(&ok, PASS).await;
    assert!(!first.exists(), "次の書き出しで消える");
    assert_eq!(get_status(&ok).await["leftover"], json!([]));
    assert_eq!(zips(&dir).len(), 1);
}

// ---------------------------------------------------------------------------
// 6.4 応答・合言葉・資格情報
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Scenario: 書き出しの口は記録の中身を返さない
#[tokio::test]
async fn export_responses_carry_no_record_content() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = World::new().await;
    let marker = format!("RESP-RAW-{}", Uuid::new_v4());
    w.put(
        "c02-window",
        &w.at("09:00"),
        json!({"title": marker, "lat": 35.6891234, "lon": 139.7012345}),
        &format!(r#"{{"title":"{marker}","lat":35.6891234,"lon":139.7012345}}"#),
    )
    .await;
    let dir = tmp_dir("resp");
    let app = app_in(&dir).await;
    let mut bodies = Vec::new();
    let (code, post_body) = post_status(&app, PASS).await;
    assert_eq!(code, StatusCode::ACCEPTED);
    bodies.push(post_body);
    wait_idle(&app).await;
    for uri in ["/export", "/export/summary"] {
        let (code, body) = call(&app, request(Method::GET, uri, true, None)).await;
        assert_eq!(code, StatusCode::OK);
        bodies.push(body);
    }
    for body in &bodies {
        for bad in [marker.as_str(), "35.6891234", "139.7012345"] {
            assert!(!body.contains(bad), "応答に {bad} が含まれる: {body}");
        }
    }
}

// Scenario: 合言葉はどこにも残らない
#[tokio::test]
async fn export_responses_never_keep_the_passphrase() {
    let _g = EXPORT_SERIAL.lock().await;
    let pool = testdb::pool().await;
    let dir = tmp_dir("secret");
    let app = app_with(&pool, &dir, Hooks::none());
    let phrase = format!("SECRET-{}", Uuid::new_v4());
    let short = &phrase[..11];

    let captured = Captured::default();
    let sink = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || sink.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);

    let mut bodies = Vec::new();
    let (code, body) = post_status(&app, short).await; // 400 の応答
    assert_eq!(code, StatusCode::BAD_REQUEST);
    bodies.push(body);
    let (code, body) = post_status(&app, &phrase).await;
    assert_eq!(code, StatusCode::ACCEPTED);
    bodies.push(body);
    let (code, body) = post_status(&app, &phrase).await; // 409 か 202 のどちらでも応答に出ない
    bodies.push(body);
    let _ = code;
    wait_idle(&app).await;
    for uri in ["/export", "/export/summary"] {
        bodies.push(call(&app, request(Method::GET, uri, true, None)).await.1);
    }
    wait_idle(&app).await;
    drop(guard);

    let ledger: Vec<String> =
        sqlx::query_scalar("SELECT l::text FROM core.export_ledger l ORDER BY seq DESC LIMIT 50")
            .fetch_all(&pool)
            .await
            .unwrap();
    let access: Vec<String> = sqlx::query_scalar(
        "SELECT a::text FROM core.access_log a WHERE a.at > now() - interval '10 minutes'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let logs = String::from_utf8_lossy(&captured.0.lock().unwrap()).into_owned();
    let mut comments = Vec::new();
    for z in zips(&dir) {
        let a = zip::ZipArchive::new(std::fs::File::open(&z).unwrap()).unwrap();
        comments.push(String::from_utf8_lossy(a.comment()).into_owned());
    }
    let places: Vec<(&str, String)> = vec![
        ("応答", bodies.join("\n")),
        ("台帳", ledger.join("\n")),
        ("読み出しの記録", access.join("\n")),
        ("ログ", logs),
        ("置き場のファイル名", names(&dir).join("\n")),
        ("zip の注記", comments.join("\n")),
    ];
    for (where_, text) in places {
        assert!(!text.contains(&phrase), "{where_} に合言葉が含まれる");
        assert!(!text.contains(short), "{where_} に合言葉の一部が含まれる");
    }
    assert!(!zips(&dir).is_empty());
}

// 資格情報の無い件数と状態の求めは断られる（状態の側。件数の側は export_excluded_tests.rs が印を持つ）
#[tokio::test]
async fn export_responses_status_without_credentials_is_unauthorized() {
    let app = app_in(&tmp_dir("noauth-status")).await;
    for uri in ["/export", "/export/summary"] {
        let (code, _) = call(&app, request(Method::GET, uri, false, None)).await;
        assert_eq!(code, StatusCode::UNAUTHORIZED, "{uri}");
    }
}

// 除いた行の件数はどこにも出ない（応答の側。ファイルの側は export_file_tests.rs が印を持つ）
#[tokio::test]
async fn export_responses_do_not_carry_the_count_of_excluded_rows() {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("no-excluded-count");
    let w = World::new().await;
    let stay = w.stay(&w.at("10:00"), &w.at("11:00")).await;
    location_at(&w, "10:30").await;
    let app = app_with(&w.pool, &dir, Hooks::none());
    erase(&app, stay).await;
    export_and_wait(&app, PASS).await;
    let st = get_status(&app).await;

    let top: std::collections::BTreeSet<&str> =
        st.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        top,
        ["interrupted", "last", "last_failure", "leftover", "running"]
            .into_iter()
            .collect()
    );
    let last: std::collections::BTreeSet<&str> = st["last"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        last,
        ["bytes", "erased_after", "file", "finished_at", "rows"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        st["last"]["erased_after"], 0,
        "書き出した時点で除いた行は数えない"
    );
}
