// SPDX-License-Identifier: AGPL-3.0-only
//! 画面のログイン（ST28 / design D1 / D2 / D3 / D16 / D17 / D18）。
//! 合言葉・印・その SHA-256 はログにも応答にも出さない（製造準備 A-2）。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    body::Bytes,
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse as _, Response},
    Json,
};
use base64::Engine as _;
use hmac::{Hmac, Mac as _};
use rand::RngCore as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::access_log::{via_of, AccessEntry};
use crate::{internal_at, token_matches, App};

/// 印を運ぶ cookie の名前（D3）。
pub const COOKIE_NAME: &str = "ashiato_session";
/// cookie の寿命。ブラウザは 400 日に切り詰める（D18）ので、それを最大として使うたびに出し直す。
const COOKIE_MAX_AGE_SECS: u64 = 400 * 24 * 60 * 60;
/// 画面の合言葉の下限（D16（仮））。
const MIN_WEB_PASSWORD_LEN: usize = 16;
/// 雛形の合言葉の頭（`.env.example`。D16（仮））。
const PLACEHOLDER_PREFIX: &str = "change-me";
/// 失敗を数える窓と上限（D17（仮））。
const THROTTLE_WINDOW: Duration = Duration::from_secs(60);
const THROTTLE_MAX_FAILURES: usize = 10;

/// 呼び出し元の種類（D1）。認められなかった求めは `Err(401)` で返るので、ここに `none` は無い。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Caller {
    WebSession,
    ApiToken,
}

impl Caller {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WebSession => "web_session",
            Self::ApiToken => "api_token",
        }
    }
}

/// 起動を拒む画面の合言葉の理由の種別（D16）。値は出さない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebPasswordRefusal {
    Missing,
    TooShort,
    /// `.env.example` の雛形（`change-me` で始まる）のまま（D16（仮））。
    Placeholder,
    SameAsApiToken,
}

impl WebPasswordRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::TooShort => "too_short",
            Self::Placeholder => "placeholder",
            Self::SameAsApiToken => "same_as_api_token",
        }
    }
}

/// 起動時の画面の合言葉の検査。無い・16 文字未満・雛形のまま（`change-me` で始まる）・
/// API の合言葉と同じなら拒む。雛形の判定は `tools/db-roles.sh` と同じ（D16（仮））。
pub fn check_web_password(
    web_password: Option<&str>,
    api_token: &str,
) -> Result<(), WebPasswordRefusal> {
    match web_password {
        None | Some("") => Err(WebPasswordRefusal::Missing),
        Some(p) if p.chars().count() < MIN_WEB_PASSWORD_LEN => Err(WebPasswordRefusal::TooShort),
        Some(p) if p.starts_with(PLACEHOLDER_PREFIX) => Err(WebPasswordRefusal::Placeholder),
        Some(p) if token_matches(p, api_token) => Err(WebPasswordRefusal::SameAsApiToken),
        Some(_) => Ok(()),
    }
}

/// 合言葉の世代の印（D3）。HMAC-SHA256、鍵は `API_TOKEN`（DB の写しだけでは合言葉の候補を確かめられない）。
pub fn secret_tag(api_token: &str, web_password: &str) -> Vec<u8> {
    // HMAC はどの長さの鍵も受ける
    let mut mac = Hmac::<Sha256>::new_from_slice(api_token.as_bytes())
        .unwrap_or_else(|_| unreachable!("HMAC は任意の長さの鍵を受ける"));
    mac.update(b"ashiato-web-session-v1");
    mac.update(web_password.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn sha256(s: &str) -> Vec<u8> {
    Sha256::digest(s.as_bytes()).to_vec()
}

/// ログインの失敗を数える。数は機械全体で 1 つ（D17。呼び出し元ごとに分けない）。
#[derive(Debug, Default)]
pub struct LoginLimiter(Mutex<VecDeque<Instant>>);

impl LoginLimiter {
    fn recent(q: &mut VecDeque<Instant>) -> usize {
        while q.front().is_some_and(|t| t.elapsed() >= THROTTLE_WINDOW) {
            q.pop_front();
        }
        q.len()
    }

    /// 窓の中の失敗が上限に達しているか。
    fn throttled(&self) -> bool {
        let mut q = self.0.lock().unwrap_or_else(|e| e.into_inner());
        Self::recent(&mut q) >= THROTTLE_MAX_FAILURES
    }

    fn record_failure(&self) {
        let mut q = self.0.lock().unwrap_or_else(|e| e.into_inner());
        Self::recent(&mut q);
        q.push_back(Instant::now());
    }
}

/// ログインの設定と状態。`App` が持つ。
#[derive(Clone, Debug)]
pub struct WebLogin {
    password: String,
    tag: Vec<u8>,
    /// 0 = 期限なし（D18）。
    max_age_days: u32,
    /// 失敗のたびに待たせる長さ（D17。試験は差し替える）。
    pub(crate) failure_delay: Duration,
    limiter: Arc<LoginLimiter>,
}

impl WebLogin {
    pub fn new(api_token: &str, web_password: &str, max_age_days: u32) -> Self {
        Self {
            password: web_password.to_owned(),
            tag: secret_tag(api_token, web_password),
            max_age_days,
            failure_delay: Duration::from_secs(1),
            limiter: Arc::default(),
        }
    }
}

/// cookie ヘッダから印を取り出す。
fn cookie_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find_map(|(k, v)| (k == COOKIE_NAME).then_some(v))
}

fn set_cookie(value: &str, max_age: u64) -> HeaderValue {
    // 印は base64url なので header に載せられる
    HeaderValue::from_str(&format!(
        "{COOKIE_NAME}={value}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={max_age}"
    ))
    .unwrap_or_else(|_| HeaderValue::from_static(""))
}

/// 印が有効か（D3 の有効の条件）。
pub(crate) async fn session_valid(app: &App, headers: &HeaderMap) -> Result<bool, sqlx::Error> {
    let Some(token) = cookie_token(headers) else {
        return Ok(false);
    };
    let issued: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT issued_at FROM core.web_session
          WHERE token_sha256 = $1 AND revoked_at IS NULL AND secret_tag = $2",
    )
    .bind(sha256(token))
    .bind(&app.login.tag)
    .fetch_optional(&app.pool)
    .await?;
    Ok(issued.is_some_and(|at| {
        app.login.max_age_days == 0
            || at + chrono::Duration::days(i64::from(app.login.max_age_days)) > app.now()
    }))
}

/// 印で認めた応答には、寿命を出し直す `Set-Cookie` を付ける（D18）。
pub async fn refresh_cookie(State(app): State<App>, req: Request, next: Next) -> Response {
    let token = cookie_token(req.headers()).map(str::to_owned);
    let headers = req.headers().clone();
    let mut res = next.run(req).await;
    if let Some(token) = token {
        if res.status() != StatusCode::UNAUTHORIZED
            && matches!(session_valid(&app, &headers).await, Ok(true))
        {
            res.headers_mut()
                .append(header::SET_COOKIE, set_cookie(&token, COOKIE_MAX_AGE_SECS));
        }
    }
    res
}

/// `POST /session` の本文。
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    /// 画面の合言葉
    pub password: String,
}

/// `GET /session` の応答。
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SessionState {
    /// `web_session` または `api_token`
    pub credential: String,
}

/// `/session` の求めの 1 行。ログインの結果はハンドラが判定した場で書く（D8）。
fn session_entry(
    headers: &HeaderMap,
    credential: &'static str,
    method: &str,
    outcome: &'static str,
    status: StatusCode,
) -> AccessEntry {
    AccessEntry {
        via: via_of(headers),
        credential,
        route: "/session".into(),
        method: method.into(),
        outcome,
        status: status.as_u16(),
    }
}

fn unauthorized() -> (StatusCode, String) {
    (StatusCode::UNAUTHORIZED, "unauthorized".into())
}

/// ログイン。**画面の合言葉そのものを資格情報として要求する**（`authorize()` は通さない）。
#[utoipa::path(post, path = "/session", request_body = LoginRequest,
    responses((status = 204), (status = 401), (status = 415), (status = 429)))]
pub async fn session_post(
    State(app): State<App>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, String)> {
    let is_json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.trim()
                .to_ascii_lowercase()
                .starts_with("application/json")
        });
    if !is_json {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported media type".into(),
        ));
    }
    if app.login.limiter.throttled() {
        tracing::warn!(kind = "login_throttled", "ログインの失敗が多いので断る");
        app.access
            .write(session_entry(
                &headers,
                "none",
                "POST",
                "login_throttled",
                StatusCode::TOO_MANY_REQUESTS,
            ))
            .await?;
        return Err((StatusCode::TOO_MANY_REQUESTS, "too many requests".into()));
    }
    let given = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("password")
                .and_then(|p| p.as_str().map(str::to_owned))
        })
        .unwrap_or_default();
    if given.is_empty() || !token_matches(&given, &app.login.password) {
        app.login.limiter.record_failure();
        tracing::warn!(kind = "login_failed", "ログインの合言葉が一致しない");
        app.access
            .write(session_entry(
                &headers,
                "none",
                "POST",
                "login_failed",
                StatusCode::UNAUTHORIZED,
            ))
            .await?;
        tokio::time::sleep(app.login.failure_delay).await;
        return Err(unauthorized());
    }
    let mut raw = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
    sqlx::query(
        "INSERT INTO core.web_session (token_sha256, secret_tag, issued_at) VALUES ($1, $2, $3)",
    )
    .bind(sha256(&token))
    .bind(&app.login.tag)
    .bind(app.now())
    .execute(&app.pool)
    .await
    .map_err(|e| internal_at("session.insert", e))?;
    // 書けなければ印を渡さない（渡さない印は使えない）
    app.access
        .write(session_entry(
            &headers,
            "none",
            "POST",
            "login_ok",
            StatusCode::NO_CONTENT,
        ))
        .await?;
    let mut res = StatusCode::NO_CONTENT.into_response();
    res.headers_mut()
        .append(header::SET_COOKIE, set_cookie(&token, COOKIE_MAX_AGE_SECS));
    Ok(res)
}

/// ログアウト。印が無くても 204。行は消さず `revoked_at` を付ける。
#[utoipa::path(delete, path = "/session", responses((status = 204)))]
pub async fn session_delete(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let credential = if session_valid(&app, &headers)
        .await
        .map_err(|e| internal_at("session.lookup", e))?
    {
        "web_session"
    } else {
        "none"
    };
    app.access
        .write(session_entry(
            &headers,
            credential,
            "DELETE",
            "logout",
            StatusCode::NO_CONTENT,
        ))
        .await?;
    if let Some(token) = cookie_token(&headers) {
        sqlx::query(
            "UPDATE core.web_session SET revoked_at = $2
              WHERE token_sha256 = $1 AND revoked_at IS NULL",
        )
        .bind(sha256(token))
        .bind(app.now())
        .execute(&app.pool)
        .await
        .map_err(|e| internal_at("session.revoke", e))?;
    }
    let mut res = StatusCode::NO_CONTENT.into_response();
    res.headers_mut()
        .append(header::SET_COOKIE, set_cookie("", 0));
    Ok(res)
}

/// ログインの状態。印か API の合言葉で認められれば、その種類を返す。
#[utoipa::path(get, path = "/session",
    responses((status = 200, body = SessionState), (status = 401)))]
pub async fn session_get(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<SessionState>, (StatusCode, String)> {
    let caller = match crate::authorize(&app, &headers).await {
        Ok(c) => c,
        Err(e) => {
            // 401 も残す（画面は起動時に必ずここを引く）。判定そのものの失敗（500）は書かない
            if e.0 == StatusCode::UNAUTHORIZED {
                app.access
                    .write(session_entry(
                        &headers,
                        "none",
                        "GET",
                        "unauthorized",
                        StatusCode::UNAUTHORIZED,
                    ))
                    .await?;
            }
            return Err(e);
        }
    };
    app.access
        .write(session_entry(
            &headers,
            caller.as_str(),
            "GET",
            "ok",
            StatusCode::OK,
        ))
        .await?;
    Ok(Json(SessionState {
        credential: caller.as_str().into(),
    }))
}
