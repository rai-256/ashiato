// SPDX-License-Identifier: AGPL-3.0-only
//! 写真の原本の受け口と D-02（ST09 / design D10 / D10a）。
//!
//! `PUT /media/originals/{sha256}` は本文を流しながら `<置き場>/.incoming/<uuid>.part` に書いてハッシュを取り、
//! 判定（ハッシュ → 記録 → 受け取り済み）を通ったものだけを `fsync` → 名前を変えて
//! `<置き場>/<YYYY>/<MM>/<YYYYMMDD-HHMMSS>_<名前>` に置き、`core.media_original` に 1 行積む。
//! `HEAD` は受け取りの行があるかだけを答える。
//!
//! **ファイルの場所は D-01 に書かない**（C2）。ログに出すのは種別と件数だけ（ファイル名・置き場・ハッシュは出さない。D21）。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::extract::{Path as UrlPath, Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, RequestExt as _};
use chrono::{DateTime, FixedOffset, Offset as _, Utc};
use futures_util::StreamExt as _;
use sha2::{Digest as _, Sha256};
use tokio::io::AsyncWriteExt as _;

use crate::{authorize, internal_at, App};

/// 原本の本文の上限（D10。仮）。この経路だけで、ほかの経路の 2 MB は変えない。
pub const ORIGINAL_BODY_LIMIT: usize = 1 << 30;
/// 置くファイルの名前の最大の文字数（拡張子を含む。D10a）。
pub const NAME_MAX_CHARS: usize = 120;
/// 整えた名前の最大のバイト数（UTF-8。拡張子を含む）。ext4 の名前の上限は 255 バイト、NTFS は 255 文字（UTF-16）で、
/// 文字数だけで切ると日本語の名前が時刻とハッシュを足した後に超える（review/code.md R21）。
/// 時刻の頭（`YYYYMMDD-HHMMSS_` 16 バイト）と全ハッシュ（`_` + 64 バイト）を足しても 255 に収まる値。
pub const NAME_MAX_BYTES: usize = 170;
/// 同じ名前の別の中身に足す内容ハッシュの頭の文字数（Q12）。
pub const HASH_SUFFIX_CHARS: usize = 8;

/// 置き場の設定の名前。
pub const DIR_SETTING: &str = "ASHIATO_MEDIA_DIR";
const INCOMING: &str = ".incoming";
/// 作るフォルダとファイルの権限（Unix）。原本は撮影位置と時刻を持つので、持ち主だけが読める
/// （書き出しの `ensure_dir` / `create_private_file` と同じ。既にあるものの権限は変えない。review/code.md R19）。
#[cfg(unix)]
const PRIVATE_DIR_MODE: u32 = 0o700;
#[cfg(unix)]
const PRIVATE_FILE_MODE: u32 = 0o600;
const PART_EXT: &str = "part";
pub(crate) const IMAGE_SOURCES: [&str; 3] = ["c01-photo", "c01-screenshot", "c01-image"];
pub(crate) const VIDEO_SOURCE: &str = "c01-video";

// ---------------------------------------------------------------------------
// 置き場の設定
// ---------------------------------------------------------------------------

/// 置き場を決める。環境は引数で受ける（純関数）。書き出しの `export_dir` と同じ並び:
/// `ASHIATO_MEDIA_DIR` → `LOCALAPPDATA\ashiato\media` → `XDG_DATA_HOME/ashiato/media` → `HOME/.local/share/ashiato/media`。
/// 空の値は無いものとして扱う。返すのは場所と、それを与えた環境変数の名前。どれも無ければ `None`。
pub fn media_dir(env: impl Fn(&str) -> Option<String>) -> Option<(PathBuf, &'static str)> {
    let get = |k: &str| env(k).filter(|v| !v.is_empty());
    if let Some(dir) = get(DIR_SETTING) {
        return Some((PathBuf::from(dir), DIR_SETTING));
    }
    let app = |base: PathBuf| base.join("ashiato").join("media");
    if let Some(local) = get("LOCALAPPDATA") {
        return Some((app(PathBuf::from(local)), "LOCALAPPDATA"));
    }
    if let Some(xdg) = get("XDG_DATA_HOME") {
        return Some((app(PathBuf::from(xdg)), "XDG_DATA_HOME"));
    }
    get("HOME").map(|home| {
        (
            app(PathBuf::from(home).join(".local").join("share")),
            "HOME",
        )
    })
}

/// 置き場の設定が相対の場所だった。起動しない（置き場が起動した場所で変わるのを防ぐ）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelativeDir {
    /// その場所を与えた環境変数の名前（終えるときにこの名前を挙げる）。
    pub setting: &'static str,
}

impl std::fmt::Display for RelativeDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} は絶対の場所で指定する", self.setting)
    }
}

impl std::error::Error for RelativeDir {}

/// 失敗した書き込みの直近の 1 件（量の読み出しが読む。D12）。S-01 を起動し直すと消える。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageError {
    pub kind: &'static str,
    pub at: DateTime<Utc>,
}

/// D-02 の設定と、受け口が共有する状態。
#[derive(Debug, Clone)]
pub struct MediaConfig {
    dir: Option<PathBuf>,
    /// 既定の置き場か（起動時に作ってよいのは既定だけ。設定された場所は作らない）。
    is_default: bool,
    body_limit: usize,
    /// 名前を決めて置く区間を 1 本にする（同じ名前を二人が取らない・同じ原本の二重の行を作らない）。
    place: Arc<tokio::sync::Mutex<()>>,
    last_error: Arc<Mutex<Option<StorageError>>>,
}

impl MediaConfig {
    /// 置き場の無い設定（`App::new` の既定。書く要求は `503 storage_unavailable`）。
    pub fn none() -> Self {
        Self {
            dir: None,
            is_default: false,
            body_limit: ORIGINAL_BODY_LIMIT,
            place: Arc::default(),
            last_error: Arc::default(),
        }
    }

    /// 環境から組む。置き場の設定が相対の場所なら `Err`。
    pub fn from_env(env: impl Fn(&str) -> Option<String>) -> Result<Self, RelativeDir> {
        let Some((dir, setting)) = media_dir(env) else {
            return Ok(Self::none());
        };
        if !dir.is_absolute() {
            return Err(RelativeDir { setting });
        }
        Ok(Self {
            dir: Some(dir),
            is_default: setting != DIR_SETTING,
            ..Self::none()
        })
    }

    /// 設定された置き場を指す（試験・差し替え）。作らない。
    pub fn with_dir(dir: PathBuf) -> Self {
        Self {
            dir: Some(dir),
            ..Self::none()
        }
    }

    /// 本文の上限を差し替えた複製（試験が 1 GiB を送らずに途中の超過を確かめる）。
    #[cfg(test)]
    pub(crate) fn with_body_limit(self, body_limit: usize) -> Self {
        Self { body_limit, ..self }
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    pub fn body_limit(&self) -> usize {
        self.body_limit
    }

    /// 起動時の用意。既定の置き場だけを作り、`.incoming/` の書きかけ（`.part`）を消す（D10 / R28）。
    /// 設定された置き場は作らない（外したドライブの空の差し込み口に書かない）。
    pub fn prepare(&self) -> std::io::Result<()> {
        let Some(dir) = &self.dir else { return Ok(()) };
        if self.is_default {
            let mut b = std::fs::DirBuilder::new();
            b.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt as _;
                b.mode(PRIVATE_DIR_MODE);
            }
            b.create(dir)?;
        }
        let incoming = match std::fs::read_dir(dir.join(INCOMING)) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        for entry in incoming {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == PART_EXT) {
                std::fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    /// 書けなかった直近の失敗（種別と時刻）。
    pub fn last_storage_error(&self) -> Option<StorageError> {
        self.last_error.lock().ok().and_then(|slot| slot.clone())
    }

    fn note_storage_error(&self, kind: &'static str, at: DateTime<Utc>) {
        if let Ok(mut slot) = self.last_error.lock() {
            *slot = Some(StorageError { kind, at });
        }
    }
}

// ---------------------------------------------------------------------------
// 名前（D10a）と並び（Q12）
// ---------------------------------------------------------------------------

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// 元のファイル名を置き場に置ける名前に整える（D10a）。区切りの最後の後ろだけを取り、危ない文字を `_` にし、
/// 末尾の `.` と空白を落とし、空・`.`・`..` は `media`、Windows の予約名は前に `_`、`NAME_MAX_CHARS` 文字と
/// `NAME_MAX_BYTES` バイトの両方に収まるよう切る（拡張子は残す）。
pub fn sanitize_name(original: &str) -> String {
    let last = original.rsplit(['/', '\\']).next().unwrap_or_default();
    let cleaned: String = last
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim_end_matches(|c: char| c == '.' || c.is_whitespace());
    let mut name = if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        "media".to_string()
    } else {
        trimmed.to_string()
    };
    let base = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if RESERVED.contains(&base.as_str()) {
        name.insert(0, '_');
    }
    truncate_keeping_ext(&name, NAME_MAX_CHARS, NAME_MAX_BYTES)
}

/// `(語幹, 拡張子（先頭の . を含む。無ければ空）)`。先頭だけの `.` は拡張子と見ない。
fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => name.split_at(i),
        _ => (name, ""),
    }
}

fn truncate_keeping_ext(name: &str, max_chars: usize, max_bytes: usize) -> String {
    let fits = |chars: usize, bytes: usize| chars <= max_chars && bytes <= max_bytes;
    if fits(name.chars().count(), name.len()) {
        return name.to_string();
    }
    let (stem, ext) = split_ext(name);
    let (head, tail) = if fits(ext.chars().count(), ext.len()) {
        (stem, ext)
    } else {
        (name, "")
    };
    let (mut chars, mut bytes) = (tail.chars().count(), tail.len());
    let mut kept = String::new();
    for c in head.chars() {
        if !fits(chars + 1, bytes + c.len_utf8()) {
            break;
        }
        chars += 1;
        bytes += c.len_utf8();
        kept.push(c);
    }
    format!("{kept}{tail}")
}

/// 置く名前の候補（`(フォルダ, [先に試す名前, ハッシュの頭を足した名前, 全ハッシュを足した名前])`）。
/// 全ハッシュの名前は、頭の 8 文字まで重なる別の中身や手で置かれたファイルがあっても、名前を決められずに
/// いつまでも断る（端末は `422` を試し直し続ける）ことにしないための最後の候補（review/code.md R22）。
/// 時刻は記録の出来事の時刻を記録の地域のずれで読んだもの。
fn layout(
    event_time: DateTime<Utc>,
    tz_offset_min: i32,
    display_name: &str,
    sha256: &str,
) -> (PathBuf, [String; 3]) {
    let offset =
        FixedOffset::east_opt(tz_offset_min.saturating_mul(60)).unwrap_or_else(|| Utc.fix());
    let local = event_time.with_timezone(&offset);
    let folder = PathBuf::from(local.format("%Y").to_string()).join(local.format("%m").to_string());
    let stamp = local.format("%Y%m%d-%H%M%S").to_string();
    let name = sanitize_name(display_name);
    let (stem, ext) = split_ext(&name);
    let suffix: String = sha256.chars().take(HASH_SUFFIX_CHARS).collect();
    let plain = format!("{stamp}_{name}");
    let suffixed = format!("{stamp}_{stem}_{suffix}{ext}");
    let full = format!("{stamp}_{stem}_{sha256}{ext}");
    (folder, [plain, suffixed, full])
}

// ---------------------------------------------------------------------------
// 受け口
// ---------------------------------------------------------------------------

/// 受け口が返す失敗。本文は `{"error": "<種別>"}` で、記録の値は載せない。
#[derive(Debug)]
enum Failure {
    Auth((StatusCode, String)),
    Malformed,
    TooLarge,
    Interrupted,
    HashMismatch,
    VideoNotAccepted,
    RecordMissing,
    /// 名前を決められない（置き場の外を指す・名前に区切りが残る・どの名前も別の中身が取っている）。
    /// 行ごとの欠陥で置き場の障害ではないので `503` にせず、直近の失敗にも残さない（design Risks）。
    NameRefused(&'static str),
    Storage(&'static str),
    Internal((StatusCode, String)),
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::Auth(r) | Self::Internal(r) => return r.into_response(),
            Self::Malformed => (StatusCode::BAD_REQUEST, "malformed"),
            Self::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
            Self::Interrupted => (StatusCode::BAD_REQUEST, "interrupted"),
            Self::HashMismatch => (StatusCode::UNPROCESSABLE_ENTITY, "hash_mismatch"),
            Self::VideoNotAccepted => (StatusCode::UNPROCESSABLE_ENTITY, "video_not_accepted"),
            Self::RecordMissing => (StatusCode::CONFLICT, "record_missing"),
            Self::NameRefused(_) => (StatusCode::UNPROCESSABLE_ENTITY, "name_refused"),
            Self::Storage(_) => (StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"),
        };
        (status, Json(serde_json::json!({ "error": code }))).into_response()
    }
}

/// 書けない失敗の種別（自分で名付けた語。OS の文言やパスを出さない）。
pub(crate) fn storage_kind(e: &std::io::Error) -> &'static str {
    match e.kind() {
        std::io::ErrorKind::NotFound => "dir_missing",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        std::io::ErrorKind::StorageFull => "disk_full",
        _ => "io",
    }
}

/// 持ち主だけが入れるフォルダを作る口（Unix は 0700）。
fn private_dir_builder(recursive: bool) -> tokio::fs::DirBuilder {
    let mut b = tokio::fs::DirBuilder::new();
    b.recursive(recursive);
    #[cfg(unix)]
    b.mode(PRIVATE_DIR_MODE);
    b
}

/// `?user_id=<uuid>`。
#[derive(Debug, serde::Deserialize)]
pub struct OriginalQuery {
    user_id: Option<uuid::Uuid>,
}

fn parse_sha(sha: &str) -> Result<String, Failure> {
    if sha.len() == 64
        && sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(sha.to_string())
    } else {
        Err(Failure::Malformed)
    }
}

/// 書きかけの一時のファイル。置かずに終わるとき（どの失敗でも）消える。
struct Part(Option<PathBuf>);

impl Part {
    fn keep(&mut self) {
        self.0 = None;
    }
}

impl Drop for Part {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn length_limit_hit(e: &axum::Error) -> bool {
    let mut cur: Option<&(dyn std::error::Error + 'static)> = Some(e);
    while let Some(err) = cur {
        if err.is::<http_body_util::LengthLimitError>() {
            return true;
        }
        cur = err.source();
    }
    false
}

/// 原本を受け取る。流しながら `.part` に書いてハッシュを取り、D10 の順で判定して置く。
#[utoipa::path(put, path = "/media/originals/{sha256}",
    params(("sha256" = String, Path, description = "原本の内容ハッシュ"),
           ("user_id" = uuid::Uuid, Query, description = "利用者")),
    request_body(content = Vec<u8>, content_type = "application/octet-stream"),
    responses((status = 201, description = "置いた"), (status = 200, description = "受け取り済み"),
              (status = 400), (status = 401), (status = 409), (status = 413), (status = 422), (status = 503)))]
pub async fn put_original(
    State(app): State<App>,
    UrlPath(sha256): UrlPath<String>,
    Query(q): Query<OriginalQuery>,
    headers: HeaderMap,
    req: Request,
) -> Response {
    match put_inner(&app, &sha256, q, &headers, req).await {
        Ok(status) => status_body(status),
        Err(f) => {
            // 種別だけを出す（ファイル名・置き場・ハッシュは出さない）。401 は `authorize` が出す
            match &f {
                Failure::Storage(kind) => {
                    tracing::error!(
                        kind = "storage_unavailable",
                        reason = *kind,
                        count = 1_u32,
                        "原本を置けなかった"
                    );
                }
                Failure::Auth(_) | Failure::Internal(_) => {}
                other => {
                    tracing::warn!(
                        kind = "original_refused",
                        reason = other.reason(),
                        count = 1_u32,
                        "原本を断った"
                    );
                }
            }
            f.into_response()
        }
    }
}

impl Failure {
    fn reason(&self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::TooLarge => "too_large",
            Self::Interrupted => "interrupted",
            Self::HashMismatch => "hash_mismatch",
            Self::VideoNotAccepted => "video_not_accepted",
            Self::RecordMissing => "record_missing",
            Self::NameRefused(kind) | Self::Storage(kind) => kind,
            Self::Auth(_) | Self::Internal(_) => "other",
        }
    }
}

fn status_body(status: StatusCode) -> Response {
    let word = if status == StatusCode::CREATED {
        "stored"
    } else {
        "already"
    };
    (status, Json(serde_json::json!({ "status": word }))).into_response()
}

async fn put_inner(
    app: &App,
    sha_param: &str,
    q: OriginalQuery,
    headers: &HeaderMap,
    req: Request,
) -> Result<StatusCode, Failure> {
    authorize(app, headers).await.map_err(Failure::Auth)?;
    let sha256 = parse_sha(sha_param)?;
    let user_id = q.user_id.ok_or(Failure::Malformed)?;
    let limit = app.media.body_limit;
    let declared = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    if declared.is_some_and(|n| n > limit as u64) {
        return Err(Failure::TooLarge);
    }
    // 置き場が設定されていない（既定も決められなかった）のも書けない失敗。要約の直近の失敗に残す（review/code.md R24）
    let root = app
        .media
        .dir()
        .ok_or_else(|| {
            app.media.note_storage_error("dir_missing", app.now());
            Failure::Storage("dir_missing")
        })?
        .to_path_buf();
    let stored = |e: &std::io::Error| {
        let kind = storage_kind(e);
        app.media.note_storage_error(kind, app.now());
        Failure::Storage(kind)
    };

    // 置き場の直下にだけ `.incoming/` を作る。**置き場そのものは作らない**（無ければここで落ちる）
    let incoming = root.join(INCOMING);
    match private_dir_builder(false).create(&incoming).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(stored(&e)),
    }
    let part_path = incoming.join(format!("{}.{PART_EXT}", uuid::Uuid::new_v4()));
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(PRIVATE_FILE_MODE);
    let mut file = options
        .open(&part_path)
        .await
        .map_err(|e| stored(&e))?;
    let mut part = Part(Some(part_path.clone()));

    let mut hasher = Sha256::new();
    let mut written = 0_i64;
    let mut body = req.with_limited_body().into_body().into_data_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|e| {
            if length_limit_hit(&e) {
                Failure::TooLarge
            } else {
                Failure::Interrupted
            }
        })?;
        hasher.update(&chunk);
        written += i64::try_from(chunk.len()).unwrap_or(i64::MAX);
        file.write_all(&chunk).await.map_err(|e| stored(&e))?;
    }
    file.flush().await.map_err(|e| stored(&e))?;
    if format!("{:x}", hasher.finalize()) != sha256 {
        return Err(Failure::HashMismatch);
    }

    let record = find_record(app, user_id, &sha256).await?;
    let Some(record) = record else {
        let video = sqlx::query_scalar::<_, i32>(
            "SELECT 1 FROM core.event
              WHERE user_id = $1 AND external_id = $2 AND logical_source = $3 AND deleted_at IS NULL
                AND payload->>'sha256' = $2 LIMIT 1",
        )
        .bind(user_id)
        .bind(&sha256)
        .bind(VIDEO_SOURCE)
        .fetch_optional(&app.pool)
        .await
        .map_err(|e| Failure::Internal(internal_at("media.video", e)))?;
        return Err(if video.is_some() {
            Failure::VideoNotAccepted
        } else {
            Failure::RecordMissing
        });
    };

    let _placing = app.media.place.lock().await;
    // 受け取りの行があっても、置いたファイルが無くなっていれば置き直す（行はそのまま。review/code.md R15）
    if let Some(size) = received(app, user_id, &sha256).await? {
        if on_disk(&root, &record, &sha256, size).await {
            return Ok(StatusCode::OK);
        }
    }
    file.sync_all().await.map_err(|e| stored(&e))?;
    drop(file);
    place(&root, &part_path, &record, &sha256)
        .await
        .map_err(|e| match e {
            PlaceError::Name(kind) => Failure::NameRefused(kind),
            PlaceError::Io(e) => stored(&e),
        })?;
    part.keep();
    sqlx::query(
        "INSERT INTO core.media_original (user_id, sha256, logical_source, size_bytes)
         VALUES ($1, $2, $3, $4) ON CONFLICT (user_id, sha256) DO NOTHING",
    )
    .bind(user_id)
    .bind(&sha256)
    .bind(&record.logical_source)
    .bind(written)
    .execute(&app.pool)
    .await
    .map_err(|e| Failure::Internal(internal_at("media.receive", e)))?;
    tracing::info!(kind = "original_stored", count = 1_u32, "原本を置いた");
    Ok(StatusCode::CREATED)
}

#[derive(Debug)]
struct RecordRow {
    logical_source: String,
    event_time: DateTime<Utc>,
    tz_offset_min: i32,
    display_name: String,
}

/// その利用者の生きている画像の記録のうち、内容ハッシュが合う最も古い受理のもの。
async fn find_record(
    app: &App,
    user_id: uuid::Uuid,
    sha256: &str,
) -> Result<Option<RecordRow>, Failure> {
    let row: Option<(String, DateTime<Utc>, i32, Option<String>)> = sqlx::query_as(
        "SELECT logical_source, event_time, tz_offset_min, payload->>'display_name'
           FROM core.event
          WHERE user_id = $1 AND external_id = $2 AND deleted_at IS NULL
            AND logical_source = ANY($3) AND payload->>'sha256' = $2
          ORDER BY ingest_time, id LIMIT 1",
    )
    .bind(user_id)
    .bind(sha256)
    .bind(&IMAGE_SOURCES[..])
    .fetch_optional(&app.pool)
    .await
    .map_err(|e| Failure::Internal(internal_at("media.record", e)))?;
    Ok(row.map(
        |(logical_source, event_time, tz_offset_min, name)| RecordRow {
            logical_source,
            event_time,
            tz_offset_min,
            display_name: name.unwrap_or_default(),
        },
    ))
}

/// 受け取りの行があれば、受け取った大きさ（バイト）。
async fn received(app: &App, user_id: uuid::Uuid, sha256: &str) -> Result<Option<i64>, Failure> {
    sqlx::query_scalar::<_, i64>(
        "SELECT size_bytes FROM core.media_original WHERE user_id = $1 AND sha256 = $2",
    )
    .bind(user_id)
    .bind(sha256)
    .fetch_optional(&app.pool)
    .await
    .map_err(|e| Failure::Internal(internal_at("media.received", e)))
}

/// 置いたファイルが、並びの規則の名前のどれかに、受け取った大きさのまま残っているか（D25（仮））。
/// 本人が D-02 のフォルダを開いて動かした・消した原本を、端末にまだあるうちに置き直すため（review/code.md R15）。
/// 中身のハッシュは計算し直さない（問い合わせのたびに最大 1 GiB を読まない）—— 大きさの一致で見る。
async fn on_disk(root: &Path, record: &RecordRow, sha256: &str, size: i64) -> bool {
    let (folder, names) = layout(
        record.event_time,
        record.tz_offset_min,
        &record.display_name,
        sha256,
    );
    for name in names {
        if let Ok(meta) = tokio::fs::metadata(root.join(&folder).join(name)).await {
            if meta.is_file() && i64::try_from(meta.len()).is_ok_and(|n| n == size) {
                return true;
            }
        }
    }
    false
}

/// 問い合わせの答え: 受け取りの行があり、置いたファイルも残っているか。
/// 記録が消された（生きている記録が無い）ときは行だけで答える（置き直しの `PUT` は `409` になる）。
/// 置き場が設定されていなければ、ファイルを確かめられないので受け取っていないと答える（端末は行を残して試し直す）。
async fn received_and_on_disk(
    app: &App,
    user_id: uuid::Uuid,
    sha256: &str,
) -> Result<bool, Failure> {
    let Some(size) = received(app, user_id, sha256).await? else {
        return Ok(false);
    };
    let Some(root) = app.media.dir() else {
        return Ok(false);
    };
    match find_record(app, user_id, sha256).await? {
        Some(record) => Ok(on_disk(root, &record, sha256, size).await),
        None => Ok(true),
    }
}

/// 置けなかった理由。名前の側の欠陥（`422`）と、書けない（`503`）を分ける。
#[derive(Debug)]
enum PlaceError {
    Name(&'static str),
    Io(std::io::Error),
}

impl From<std::io::Error> for PlaceError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// 並びに置く。同じ名前に別の中身があれば、拡張子の前にハッシュの頭を足した名前に置く。
/// 同じ中身がもう置かれていれば（名前を変えた後に行を積む前に止まった跡）、置き直さず `.part` を消す。
/// 名前を変えたら、置いたフォルダから置き場までを `fsync` する（電源断の後に「受け取りの行はあるがファイルが無い」を作らない）。
async fn place(
    root: &Path,
    part: &Path,
    record: &RecordRow,
    sha256: &str,
) -> Result<(), PlaceError> {
    let (folder, names) = layout(
        record.event_time,
        record.tz_offset_min,
        &record.display_name,
        sha256,
    );
    let dir = root.join(&folder);
    private_dir_builder(true).create(&dir).await?;
    // 正規化した道筋で、置き先が置き場の下にあることを確かめてから名前を変える
    let canon_dir = tokio::fs::canonicalize(&dir).await?;
    let canon_root = tokio::fs::canonicalize(root).await?;
    if !canon_dir.starts_with(&canon_root) {
        return Err(PlaceError::Name("outside_root"));
    }
    for name in names {
        if Path::new(&name).file_name().and_then(|n| n.to_str()) != Some(name.as_str()) {
            return Err(PlaceError::Name("bad_name"));
        }
        let target = canon_dir.join(&name);
        if tokio::fs::try_exists(&target).await? {
            if file_sha256(&target).await? == sha256 {
                return Ok(tokio::fs::remove_file(part).await?);
            }
            continue;
        }
        tokio::fs::rename(part, target).await?;
        for d in canon_dir
            .ancestors()
            .take_while(|d| d.starts_with(&canon_root))
        {
            sync_dir(d).await?;
        }
        return Ok(());
    }
    Err(PlaceError::Name("name_taken"))
}

/// フォルダの項目（名前を変えた結果）をディスクに書く。Windows はフォルダを開いて `fsync` できないので何もしない。
#[cfg(unix)]
async fn sync_dir(dir: &Path) -> std::io::Result<()> {
    tokio::fs::File::open(dir).await?.sync_all().await
}

#[cfg(not(unix))]
async fn sync_dir(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

async fn file_sha256(path: &Path) -> std::io::Result<String> {
    use tokio::io::AsyncReadExt as _;
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0_u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            return Ok(format!("{:x}", hasher.finalize()));
        }
        hasher.update(&buf[..n]);
    }
}

/// 受け取り済みかを答える。受け取りの行があり、置いたファイルも残っていれば `200`、どちらかが無ければ `404`（本文は受けない）。
#[utoipa::path(head, path = "/media/originals/{sha256}",
    params(("sha256" = String, Path, description = "原本の内容ハッシュ"),
           ("user_id" = uuid::Uuid, Query, description = "利用者")),
    responses((status = 200), (status = 404), (status = 400), (status = 401)))]
pub async fn head_original(
    State(app): State<App>,
    UrlPath(sha256): UrlPath<String>,
    Query(q): Query<OriginalQuery>,
    headers: HeaderMap,
) -> Response {
    let result: Result<bool, Failure> = async {
        authorize(&app, &headers).await.map_err(Failure::Auth)?;
        let sha256 = parse_sha(&sha256)?;
        let user_id = q.user_id.ok_or(Failure::Malformed)?;
        received_and_on_disk(&app, user_id, &sha256).await
    }
    .await;
    match result {
        Ok(true) => StatusCode::OK.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(f) => f.into_response(),
    }
}
