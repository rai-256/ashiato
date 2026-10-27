// SPDX-License-Identifier: AGPL-3.0-only
//! 原本の受け口と D-02（ST09 tasks 6 / design D10 / D10a）を、本物の PostgreSQL と本物の置き場（一時ディレクトリ）に対して確かめる。
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Method, Request, StatusCode};
use axum::{extract::State, Json};
use sha2::{Digest as _, Sha256};
use tower::ServiceExt as _;

use crate::media::{self, MediaConfig};
use crate::testdb;
use crate::{ingest, router, App};

const TOKEN: &str = "test-token-0123456789abcdef";

/// 一時の置き場。落とすと消す（`TempDir` が消す）。
pub(crate) struct Tmp(tempfile::TempDir);

impl Tmp {
    pub(crate) fn new() -> Self {
        Self(
            tempfile::Builder::new()
                .prefix("ashiato-media-")
                .tempdir()
                .unwrap(),
        )
    }
    /// 置き場（箱の中の 1 段下。外に書かれたかを箱の直下で見られる）
    pub(crate) fn root(&self) -> PathBuf {
        self.0.path().join("media")
    }
    pub(crate) fn made(&self) -> PathBuf {
        let root = self.root();
        std::fs::create_dir_all(&root).unwrap();
        root
    }
}

pub(crate) async fn app_with(root: PathBuf) -> App {
    App::for_test(testdb::pool().await, TOKEN).with_media(MediaConfig::with_dir(root))
}

pub(crate) fn sha_of(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// 試験ごとに固有のバイト（記録の外部識別子が全体で一意なため）。
pub(crate) fn unique_bytes(tag: &str) -> Vec<u8> {
    format!("{tag}-{}", uuid::Uuid::new_v4()).into_bytes()
}

pub(crate) fn record(
    user: uuid::Uuid,
    source: &str,
    sha: &str,
    name: &str,
    event_time: &str,
    extra: serde_json::Value,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "kind": "media", "sha256": sha, "media_id": 7, "display_name": name,
    });
    for (k, v) in extra.as_object().unwrap() {
        payload[k] = v.clone();
    }
    serde_json::json!({
        "id": uuid::Uuid::new_v4(), "user_id": user,
        "logical_source": source, "external_id": sha,
        "device_id": "phone-01", "origin": "collected",
        "event_time": event_time, "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo", "schema_version": 1, "raw": sha, "payload": payload,
    })
}

pub(crate) async fn store(app: &App, items: Vec<serde_json::Value>) {
    let mut h = HeaderMap::new();
    h.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    let (_, Json(res)) = ingest(State(app.clone()), h, Json(serde_json::Value::Array(items)))
        .await
        .unwrap();
    assert!(res.iter().all(|r| r.accepted), "断られた: {res:?}");
}

pub(crate) async fn photo(
    app: &App,
    user: uuid::Uuid,
    bytes: &[u8],
    name: &str,
    event_time: &str,
) -> String {
    let sha = sha_of(bytes);
    store(
        app,
        vec![record(
            user,
            "c01-photo",
            &sha,
            name,
            event_time,
            serde_json::json!({}),
        )],
    )
    .await;
    sha
}

fn put_req(sha: &str, user: uuid::Uuid, body: Body, auth: bool) -> Request<Body> {
    let mut b = Request::builder()
        .method(Method::PUT)
        .uri(format!("/media/originals/{sha}?user_id={user}"));
    if auth {
        b = b.header("authorization", format!("Bearer {TOKEN}"));
    }
    b.body(body).unwrap()
}

pub(crate) async fn put(
    app: &App,
    sha: &str,
    user: uuid::Uuid,
    bytes: &[u8],
) -> (StatusCode, String) {
    let res = router(app.clone())
        .oneshot(put_req(sha, user, Body::from(bytes.to_vec()), true))
        .await
        .unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&body).into_owned())
}

async fn head(app: &App, sha: &str, user: uuid::Uuid) -> StatusCode {
    let req = Request::builder()
        .method(Method::HEAD)
        .uri(format!("/media/originals/{sha}?user_id={user}"))
        .header("authorization", format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap();
    router(app.clone()).oneshot(req).await.unwrap().status()
}

/// 置き場の下の通常のファイル全部（置き場からの相対。`.incoming/` は除く）。
fn placed(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, root, out);
            } else {
                out.push(
                    p.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else {
        return out;
    };
    for e in rd {
        let p = e.unwrap().path();
        if p.file_name().is_some_and(|n| n == ".incoming") {
            continue;
        }
        if p.is_dir() {
            walk(&p, root, &mut out);
        } else {
            out.push(p.strip_prefix(root).unwrap().to_string_lossy().into_owned());
        }
    }
    out.sort();
    out
}

fn incoming_files(root: &Path) -> usize {
    std::fs::read_dir(root.join(".incoming")).map_or(0, Iterator::count)
}

async fn receipts(app: &App, user: uuid::Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM core.media_original WHERE user_id = $1")
        .bind(user)
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

const TAKEN: &str = "2026-10-17T05:30:12Z";

// ------------------------------------------------------------------ 6.1 置き場の設定

fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |k| {
        pairs
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| (*v).to_string())
    }
}

/// Scenario: 置き場の設定が無ければ既定の置き場になる
#[test]
fn st09_up_dir_defaults_when_unset() {
    let at = |pairs: &[(&str, &str)]| media::media_dir(env_of(pairs)).map(|(d, _)| d);
    assert_eq!(
        at(&[
            ("LOCALAPPDATA", r"C:\Users\me\AppData\Local"),
            ("HOME", "/h")
        ])
        .unwrap(),
        PathBuf::from(r"C:\Users\me\AppData\Local")
            .join("ashiato")
            .join("media")
    );
    assert_eq!(
        at(&[("XDG_DATA_HOME", "/x"), ("HOME", "/h")]).unwrap(),
        PathBuf::from("/x/ashiato/media")
    );
    assert_eq!(
        at(&[("HOME", "/h")]).unwrap(),
        PathBuf::from("/h/.local/share/ashiato/media")
    );
    // 空の値は無いものとして次へ進む
    assert_eq!(
        at(&[("ASHIATO_MEDIA_DIR", ""), ("HOME", "/h")]).unwrap(),
        PathBuf::from("/h/.local/share/ashiato/media")
    );
    assert!(at(&[]).is_none());
    let config = MediaConfig::from_env(env_of(&[("XDG_DATA_HOME", "/x")])).unwrap();
    assert_eq!(config.dir().unwrap(), Path::new("/x/ashiato/media"));
}

/// Scenario: 置き場を設定で変えられる
#[tokio::test]
async fn st09_up_dir_setting_changes_the_place() {
    let tmp = Tmp::new();
    let other = tmp.made();
    let config = MediaConfig::from_env(env_of(&[
        ("ASHIATO_MEDIA_DIR", other.to_str().unwrap()),
        ("HOME", "/h"),
    ]))
    .unwrap();
    assert_eq!(config.dir().unwrap(), other);
    let app = App::for_test(testdb::pool().await, TOKEN).with_media(config);
    let user = testdb::user();
    let bytes = unique_bytes("setting");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    assert_eq!(placed(&other).len(), 1);
}

/// Scenario: 相対の置き場では起動しない
#[test]
fn st09_up_dir_relative_refuses_to_start() {
    let refusal =
        MediaConfig::from_env(env_of(&[("ASHIATO_MEDIA_DIR", "media/here")])).unwrap_err();
    assert_eq!(refusal.setting, "ASHIATO_MEDIA_DIR");
    assert!(refusal.to_string().contains("ASHIATO_MEDIA_DIR"));
}

/// Scenario: 設定した置き場が無ければ作らずに書けない失敗にする
#[tokio::test]
async fn st09_up_dir_missing_is_not_created_and_fails_retriably() {
    let tmp = Tmp::new();
    let missing = tmp.root(); // 作っていない
    let app = app_with(missing.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("missing-dir");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    let (status, body) = put(&app, &sha, user, &bytes).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains("storage_unavailable"), "{body}");
    assert!(!missing.exists(), "設定された置き場を作った");
    // prepare も設定された場所は作らない
    MediaConfig::with_dir(missing.clone()).prepare().unwrap();
    assert!(!missing.exists());
}

/// Scenario: 起動すると書きかけの一時のファイルが消える
#[test]
fn st09_up_dir_startup_removes_part_files() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let incoming = root.join(".incoming");
    std::fs::create_dir_all(&incoming).unwrap();
    std::fs::write(incoming.join("a.part"), b"half").unwrap();
    std::fs::write(incoming.join("b.part"), b"half").unwrap();
    MediaConfig::with_dir(root.clone()).prepare().unwrap();
    assert_eq!(incoming_files(&root), 0);
    // 既定の置き場だけは起動時に作る
    let home = tmp.0.path().join("home");
    let config = MediaConfig::from_env(env_of(&[("HOME", home.to_str().unwrap())])).unwrap();
    config.prepare().unwrap();
    assert!(config.dir().unwrap().is_dir());
}

// ------------------------------------------------------------------ 6.2 受け口

/// Scenario: ハッシュの合う原本は受け取られる
#[tokio::test]
async fn st09_up_put_matching_hash_is_stored() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("ok");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    let (status, _) = put(&app, &sha, user, &bytes).await;
    assert_eq!(status, StatusCode::CREATED);
    let files = placed(&root);
    assert_eq!(files.len(), 1, "{files:?}");
    assert_eq!(std::fs::read(root.join(&files[0])).unwrap(), bytes);
    assert_eq!(receipts(&app, user).await, 1);
    assert_eq!(incoming_files(&root), 0);
}

/// Scenario: ハッシュの合わない原本は置かれない
#[tokio::test]
async fn st09_up_put_mismatched_hash_is_not_placed() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("mismatch");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    let mut other = bytes.clone();
    other.push(b'x');
    let (status, body) = put(&app, &sha, user, &other).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body.contains("hash_mismatch"), "{body}");
    assert!(placed(&root).is_empty());
    assert_eq!(incoming_files(&root), 0);
    assert_eq!(receipts(&app, user).await, 0);
}

/// Scenario: 記録より先に届いた原本は送り直せる失敗になる
#[tokio::test]
async fn st09_up_put_before_the_record_is_retriable() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("early");
    let (status, body) = put(&app, &sha_of(&bytes), user, &bytes).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body.contains("record_missing"), "{body}");
    assert!(placed(&root).is_empty());
    assert_eq!(incoming_files(&root), 0);
}

/// Scenario: 動画の原本は受け取られない
#[tokio::test]
async fn st09_up_put_video_is_not_accepted() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("video");
    let sha = sha_of(&bytes);
    store(
        &app,
        vec![record(
            user,
            "c01-video",
            &sha,
            "v.mp4",
            TAKEN,
            serde_json::json!({}),
        )],
    )
    .await;
    let (status, body) = put(&app, &sha, user, &bytes).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body.contains("video_not_accepted"), "{body}");
    assert!(placed(&root).is_empty());
}

/// Scenario: 同じ原本が 2 度届いても 1 つだけ置かれる
#[tokio::test]
async fn st09_up_put_twice_places_one_file() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("twice");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::OK);
    assert_eq!(placed(&root).len(), 1);
    assert_eq!(receipts(&app, user).await, 1);
    assert_eq!(incoming_files(&root), 0);
}

/// Scenario: 書いている途中で止まっても壊れたファイルは並びに残らない
#[tokio::test]
async fn st09_up_put_cut_midway_leaves_no_broken_file() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("cut");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    let half = Bytes::copy_from_slice(&bytes[..bytes.len() / 2]);
    let stream = futures_util::stream::iter(vec![
        Ok::<_, std::io::Error>(half),
        Err(std::io::Error::other("接続が切れた")),
    ]);
    let res = router(app.clone())
        .oneshot(put_req(&sha, user, Body::from_stream(stream), true))
        .await
        .unwrap();
    assert!(!res.status().is_success());
    assert!(placed(&root).is_empty(), "並びに途中までのファイルがある");
    assert_eq!(incoming_files(&root), 0);
    assert_eq!(receipts(&app, user).await, 0);
    // 送り直すと置かれる
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    assert_eq!(placed(&root).len(), 1);
}

/// Scenario: D-02 に書けなければ送り直せる失敗になり、端末に送り待ちが残る
#[tokio::test]
async fn st09_up_put_unwritable_is_retriable_and_noted() {
    let tmp = Tmp::new();
    let app = app_with(tmp.root()).await; // 置き場が無い
    let user = testdb::user();
    let bytes = unique_bytes("unwritable");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    let before = chrono::Utc::now();
    let (status, _) = put(&app, &sha, user, &bytes).await;
    // 503 は端末の送り手が行を残して試し直す形（OriginalUploader の「そのほか」）
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let last = app.media.last_storage_error().expect("直近の失敗");
    assert_eq!(last.kind, "dir_missing");
    assert!(last.at >= before);
    assert_eq!(receipts(&app, user).await, 0);
}

/// Scenario: 資格情報の無い原本の要求は断られる
#[tokio::test]
async fn st09_up_put_without_credentials_is_refused() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("noauth");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    let res = router(app.clone())
        .oneshot(put_req(&sha, user, Body::from(bytes), false))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(placed(&root).is_empty());
    assert_eq!(incoming_files(&root), 0);
}

/// Scenario: D-01 は原本のファイルの場所を持たない
#[tokio::test]
async fn st09_up_put_d01_holds_no_file_location() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("d01");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'media_original' ORDER BY ordinal_position",
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(
        columns,
        [
            "user_id",
            "sha256",
            "logical_source",
            "size_bytes",
            "received_at"
        ]
    );
    let (stored_sha, size): (String, i64) =
        sqlx::query_as("SELECT sha256, size_bytes FROM core.media_original WHERE user_id = $1")
            .bind(user)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(stored_sha, sha);
    assert_eq!(size, bytes.len() as i64);
    // 記録のどこにも置き場の場所が入っていない
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT payload::text || raw FROM core.event WHERE user_id = $1")
            .bind(user)
            .fetch_all(&app.pool)
            .await
            .unwrap();
    assert!(rows.iter().all(|r| !r.contains(root.to_str().unwrap())));
}

/// Scenario: 原本は撮影の年/月のフォルダに撮影日時と元の名前で置かれる
#[tokio::test]
async fn st09_up_put_lands_under_year_month_with_time_and_name() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("layout");
    // 2026-10-17T14:30:12+09:00
    let sha = photo(
        &app,
        user,
        &bytes,
        "PXL_20261017_053012345.jpg",
        "2026-10-17T05:30:12Z",
    )
    .await;
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    assert_eq!(
        placed(&root),
        ["2026/10/20261017-143012_PXL_20261017_053012345.jpg"]
    );
}

/// Scenario: 同じ名前の別の中身にはハッシュの頭が足される
#[tokio::test]
async fn st09_up_put_same_name_other_content_gets_hash_prefix() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let (a, b) = (unique_bytes("same-a"), unique_bytes("same-b"));
    let sha_a = photo(&app, user, &a, "IMG.jpg", TAKEN).await;
    let sha_b = photo(&app, user, &b, "IMG.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha_a, user, &a).await.0, StatusCode::CREATED);
    assert_eq!(put(&app, &sha_b, user, &b).await.0, StatusCode::CREATED);
    let files = placed(&root);
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(files.contains(&"2026/10/20261017-143012_IMG.jpg".to_string()));
    assert!(files.contains(&format!("2026/10/20261017-143012_IMG_{}.jpg", &sha_b[..8])));
}

/// Scenario: 元の名前に区切りがあっても置き場の外に書かない
#[tokio::test]
async fn st09_up_put_name_with_separators_stays_inside() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let (a, b) = (unique_bytes("evil-a"), unique_bytes("evil-b"));
    let sha_a = photo(&app, user, &a, r"..\..\evil.jpg", TAKEN).await;
    let sha_b = photo(&app, user, &b, "../../evil2.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha_a, user, &a).await.0, StatusCode::CREATED);
    assert_eq!(put(&app, &sha_b, user, &b).await.0, StatusCode::CREATED);
    assert_eq!(
        placed(&root),
        [
            "2026/10/20261017-143012_evil.jpg",
            "2026/10/20261017-143012_evil2.jpg"
        ]
    );
    // 箱（置き場の外）には置き場のほか何も無い
    let outside: Vec<_> = std::fs::read_dir(&tmp.0)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(outside, ["media"]);
    // 名前の整え方（D10a）
    assert_eq!(media::sanitize_name(r"C:\x\a:b?.jpg"), "a_b_.jpg");
    assert_eq!(media::sanitize_name("..."), "media");
    assert_eq!(media::sanitize_name(""), "media");
    assert_eq!(media::sanitize_name("dir/"), "media");
    assert_eq!(media::sanitize_name("CON.txt"), "_CON.txt");
    assert_eq!(media::sanitize_name("a.jpg  "), "a.jpg");
    // 上限は本人の決定ではない仮の値（D10a）。書き換えたら気付くよう名指しで固定する（review/code.md R17）
    assert_eq!(media::NAME_MAX_CHARS, 120);
    assert_eq!(media::NAME_MAX_BYTES, 170);
    let long = format!("{}.jpeg", "a".repeat(300));
    let cut = media::sanitize_name(&long);
    assert_eq!(cut.chars().count(), media::NAME_MAX_CHARS);
    assert!(cut.ends_with(".jpeg"));
    // 日本語は文字数より先にバイト数で切れる（1 文字 3 バイト。review/code.md R21）
    let wide = media::sanitize_name(&format!("{}.jpeg", "あ".repeat(300)));
    assert!(wide.len() <= media::NAME_MAX_BYTES, "{}", wide.len());
    assert_eq!(wide, format!("{}.jpeg", "あ".repeat(55)));
}

/// 日本語の長い名前でも、時刻とハッシュを足した名前がファイルの名前の上限（255 バイト）を超えず、置ける（review/code.md R21）。
#[tokio::test]
async fn st09_up_put_long_japanese_name_fits_the_file_name_limit() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let name = format!("{}.jpg", "写真".repeat(60));
    let (a, b) = (unique_bytes("wide-a"), unique_bytes("wide-b"));
    let sha_a = photo(&app, user, &a, &name, TAKEN).await;
    let sha_b = photo(&app, user, &b, &name, TAKEN).await;
    assert_eq!(put(&app, &sha_a, user, &a).await.0, StatusCode::CREATED);
    assert_eq!(put(&app, &sha_b, user, &b).await.0, StatusCode::CREATED);
    let files = placed(&root);
    assert_eq!(files.len(), 2, "{files:?}");
    for f in &files {
        let file_name = f.rsplit('/').next().unwrap();
        assert!(file_name.len() <= 255, "{}", file_name.len());
        assert!(file_name.ends_with(".jpg"), "{file_name}");
    }
    assert_eq!(app.media.last_storage_error(), None);
}

/// 名前を決められない（どの名前も別の中身が取っている）のは行ごとの欠陥。`503` にせず `422 name_refused`、
/// 直近の書き込みの失敗にも残さない（design Risks「名前の整え方の取りこぼし」。R8）。
#[tokio::test]
async fn st09_up_put_name_taken_everywhere_is_refused_without_a_storage_error() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("taken");
    let sha = photo(&app, user, &bytes, "IMG.jpg", TAKEN).await;
    let month = root.join("2026/10");
    std::fs::create_dir_all(&month).unwrap();
    std::fs::write(month.join("20261017-143012_IMG.jpg"), b"other 1").unwrap();
    std::fs::write(
        month.join(format!("20261017-143012_IMG_{}.jpg", &sha[..8])),
        b"other 2",
    )
    .unwrap();
    std::fs::write(
        month.join(format!("20261017-143012_IMG_{sha}.jpg")),
        b"other 3",
    )
    .unwrap();
    let (status, body) = put(&app, &sha, user, &bytes).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(body.contains("name_refused"), "{body}");
    assert_eq!(app.media.last_storage_error(), None);
    assert_eq!(receipts(&app, user).await, 0);
    assert_eq!(incoming_files(&root), 0);
    assert_eq!(placed(&root).len(), 3);
}

/// 元の名前とハッシュの頭の名前の両方を別の中身が取っていても、全ハッシュの名前に置く
/// （頭の 8 文字の重なり・手で置かれたファイルで、いつまでも `422` にしない。review/code.md R22）。
#[tokio::test]
async fn st09_up_put_both_names_taken_lands_under_the_full_hash_name() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("taken-twice");
    let sha = photo(&app, user, &bytes, "IMG.jpg", TAKEN).await;
    let month = root.join("2026/10");
    std::fs::create_dir_all(&month).unwrap();
    std::fs::write(month.join("20261017-143012_IMG.jpg"), b"other 1").unwrap();
    std::fs::write(
        month.join(format!("20261017-143012_IMG_{}.jpg", &sha[..8])),
        b"other 2",
    )
    .unwrap();
    let (status, body) = put(&app, &sha, user, &bytes).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let full = month.join(format!("20261017-143012_IMG_{sha}.jpg"));
    assert_eq!(std::fs::read(&full).unwrap(), bytes);
    assert_eq!(receipts(&app, user).await, 1);
}

/// Scenario: 原本は形式を変えずに送られる
/// D-02 の側: HEIC の原本は拡張子も中身のバイトも変えずに置かれる（review/code.md R16）。
#[tokio::test]
async fn st09_up_put_heic_original_keeps_its_format_and_bytes() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    // HEIC の頭（ftyp heic）に固有のバイトを足したもの
    let mut bytes = b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00mif1heic".to_vec();
    bytes.extend(unique_bytes("heic"));
    let sha = photo(&app, user, &bytes, "IMG_0001.HEIC", TAKEN).await;
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    assert_eq!(placed(&root), ["2026/10/20261017-143012_IMG_0001.HEIC"]);
    assert_eq!(
        std::fs::read(root.join("2026/10/20261017-143012_IMG_0001.HEIC")).unwrap(),
        bytes
    );
}

/// D-02 のファイルが後から無くなった（本人が動かした・消した）なら、問い合わせは受け取っていないと答え、
/// 送り直しで置き直す（受け取りの行は 1 つのまま。review/code.md R15 / D25（仮））。
#[tokio::test]
async fn st09_up_put_lost_original_is_asked_again_and_placed_again() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("lost");
    let sha = photo(&app, user, &bytes, "lost.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    let file = root.join("2026/10/20261017-143012_lost.jpg");
    assert_eq!(head(&app, &sha, user).await, StatusCode::OK);
    std::fs::remove_file(&file).unwrap();
    assert_eq!(head(&app, &sha, user).await, StatusCode::NOT_FOUND);
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    assert_eq!(head(&app, &sha, user).await, StatusCode::OK);
    assert_eq!(receipts(&app, user).await, 1);
    // 置いたままなら送り直しは置き直さない（同じ 1 つ）
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::OK);
    assert_eq!(placed(&root).len(), 1);
}

/// 置いた後に記録が更新されて名前が変わっても（端末で名前を変えた）、置いた時の名前のファイルを見つけ、
/// 同じ原本を 2 つ目として置かない（scoped re-review の後退の指摘）。
#[tokio::test]
async fn st09_up_put_renamed_record_still_finds_the_placed_original() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("renamed");
    let sha = photo(&app, user, &bytes, "IMG_1.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    let mut renamed = record(
        user,
        "c01-photo",
        &sha,
        "trip.jpg",
        TAKEN,
        serde_json::json!({}),
    );
    renamed["raw"] = serde_json::json!(format!("{sha}-renamed"));
    store(&app, vec![renamed]).await;
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT payload->>'display_name' FROM core.event WHERE user_id = $1 AND external_id = $2",
    )
    .bind(user)
    .bind(&sha)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(names, ["trip.jpg"], "記録が更新されていない");
    assert_eq!(head(&app, &sha, user).await, StatusCode::OK);
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::OK);
    assert_eq!(placed(&root), ["2026/10/20261017-143012_IMG_1.jpg"]);
}

/// 置いた原本（撮影位置を持つ）と作ったフォルダは持ち主だけが読める（Unix。review/code.md R19 / D26（仮））。
#[cfg(unix)]
#[tokio::test]
async fn st09_up_put_original_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt as _;
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let bytes = unique_bytes("private");
    let sha = photo(&app, user, &bytes, "private.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha, user, &bytes).await.0, StatusCode::CREATED);
    let mode = |p: PathBuf| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(root.join("2026")), 0o700);
    assert_eq!(mode(root.join("2026/10")), 0o700);
    assert_eq!(mode(root.join(".incoming")), 0o700);
    assert_eq!(
        mode(root.join("2026/10/20261017-143012_private.jpg")),
        0o600
    );
    // 既定の置き場（起動時に作る）も
    let xdg = tmp.0.join("xdg");
    let xdg_s = xdg.to_str().unwrap().to_string();
    let config = MediaConfig::from_env(env_of(&[("XDG_DATA_HOME", xdg_s.as_str())])).unwrap();
    config.prepare().unwrap();
    assert_eq!(mode(xdg.join("ashiato/media")), 0o700);
}

/// 置き場が設定されていない（既定も決められない）ときも、書けない失敗を要約の直近の失敗に残す（review/code.md R24）。
#[tokio::test]
async fn st09_up_put_without_any_place_notes_the_storage_error() {
    let app = App::for_test(testdb::pool().await, TOKEN).with_media(MediaConfig::none());
    let user = testdb::user();
    let bytes = unique_bytes("no-place");
    let sha = photo(&app, user, &bytes, "a.jpg", TAKEN).await;
    let (status, body) = put(&app, &sha, user, &bytes).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains("storage_unavailable"), "{body}");
    assert_eq!(app.media.last_storage_error().unwrap().kind, "dir_missing");
    // 問い合わせは受け取っていないと答える（ファイルを確かめられない）
    assert_eq!(head(&app, &sha, user).await, StatusCode::NOT_FOUND);
}

/// 経路の上限（1 GiB）が既定の 2 MB を上書きしている: 3 MB の本文が置かれる（R9）。
#[tokio::test]
async fn st09_up_put_three_mb_body_passes_the_default_two_mb_limit() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let mut bytes = unique_bytes("three-mb");
    bytes.resize(3 * 1024 * 1024, 0x5a);
    let sha = photo(&app, user, &bytes, "big.jpg", TAKEN).await;
    let (status, body) = put(&app, &sha, user, &bytes).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let files = placed(&root);
    assert_eq!(files.len(), 1, "{files:?}");
    assert_eq!(std::fs::read(root.join(&files[0])).unwrap(), bytes);
    assert_eq!(receipts(&app, user).await, 1);
}

/// Scenario: 受け取り済みかを問い合わせられる
#[tokio::test]
async fn st09_up_put_head_asks_whether_received() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let (a, b) = (unique_bytes("head-a"), unique_bytes("head-b"));
    let sha_a = photo(&app, user, &a, "a.jpg", TAKEN).await;
    let sha_b = photo(&app, user, &b, "b.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha_a, user, &a).await.0, StatusCode::CREATED);
    assert_eq!(head(&app, &sha_a, user).await, StatusCode::OK);
    assert_eq!(head(&app, &sha_b, user).await, StatusCode::NOT_FOUND);
}

/// Scenario: 1 GiB を超える原本は送り直せない失敗になる
#[tokio::test]
async fn st09_up_put_over_one_gib_is_too_large() {
    assert_eq!(media::ORIGINAL_BODY_LIMIT, 1 << 30);
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let sha = sha_of(b"big");
    // 宣言された長さが上限を超える本文は、読み始める前に断る
    let mut req = put_req(&sha, user, Body::empty(), true);
    req.headers_mut().insert(
        "content-length",
        ((1_u64 << 30) + 1).to_string().parse().unwrap(),
    );
    let res = router(app.clone()).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert!(placed(&root).is_empty());
    assert_eq!(incoming_files(&root), 0);
    // 流している途中で超えたものも断り、書きかけを残さない（上限だけを小さくして確かめる）
    let small = App::for_test(testdb::pool().await, TOKEN)
        .with_media(MediaConfig::with_dir(root.clone()).with_body_limit(1024));
    let chunks = futures_util::stream::iter(
        (0..3).map(|_| Ok::<_, std::io::Error>(Bytes::from(vec![0_u8; 600]))),
    );
    let res = router(small.clone())
        .oneshot(put_req(&sha, user, Body::from_stream(chunks), true))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert!(placed(&root).is_empty());
    assert_eq!(incoming_files(&root), 0);
}

/// Scenario: 中身が変わった写真は新旧どちらの原本も D-02 に残る
#[tokio::test]
async fn st09_up_put_edited_photo_keeps_both_originals() {
    let tmp = Tmp::new();
    let root = tmp.made();
    let app = app_with(root.clone()).await;
    let user = testdb::user();
    let (old, new) = (unique_bytes("edit-old"), unique_bytes("edit-new"));
    let sha_old = photo(&app, user, &old, "IMG.jpg", TAKEN).await;
    assert_eq!(put(&app, &sha_old, user, &old).await.0, StatusCode::CREATED);
    let old_file = placed(&root).remove(0);
    // 同じ項目に上書き保存: 新しい内容ハッシュの記録（前の版のハッシュを持つ）
    let sha_new = sha_of(&new);
    store(
        &app,
        vec![record(
            user,
            "c01-photo",
            &sha_new,
            "IMG.jpg",
            TAKEN,
            serde_json::json!({"previous_sha256": sha_old}),
        )],
    )
    .await;
    assert_eq!(put(&app, &sha_new, user, &new).await.0, StatusCode::CREATED);
    assert_eq!(placed(&root).len(), 2);
    assert_eq!(std::fs::read(root.join(old_file)).unwrap(), old);
    assert_eq!(receipts(&app, user).await, 2);
}

// ------------------------------------------------------------------ 6.3 ログ

/// Scenario: 原本の送信の失敗がログに出てもファイル名と位置は出ない
#[tokio::test]
async fn st09_up_log_failures_show_kind_but_no_name_dir_or_hash() {
    // 受け手はプロセス全体（並列で走る他の試験の購読に取り合われない）
    let captured = crate::archive_flow_tests::captured_log().clone();

    let tmp = Tmp::new();
    let secret_dir = tmp.0.join("SECRETDIR-folder");
    let app = app_with(secret_dir.clone()).await; // 置き場が無い → 書けない失敗
    let user = testdb::user();
    let bytes = unique_bytes("log");
    let name = "SECRETNAME-photo.jpg";
    let sha = photo(&app, user, &bytes, name, TAKEN).await;
    assert_eq!(
        put(&app, &sha, user, &bytes).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    // ハッシュの合わない送りも
    let root = tmp.made();
    let app2 = app_with(root).await;
    let mut other = bytes.clone();
    other.push(b'!');
    assert_eq!(
        put(&app2, &sha, user, &other).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("storage_unavailable"), "{log}");
    assert!(log.contains("hash_mismatch"), "{log}");
    for kind in ["storage_unavailable", "hash_mismatch"] {
        let line = log.lines().find(|l| l.contains(kind)).unwrap();
        assert!(line.contains("count=1"), "件数が無い: {line}");
    }
    for secret in [
        "SECRETNAME",
        "SECRETDIR",
        sha.as_str(),
        tmp.0.to_str().unwrap(),
    ] {
        assert!(!log.contains(secret), "ログに出た: {secret}\n{log}");
    }
}
