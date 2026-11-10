// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 画面のログイン（ST28 / design D1 / D2 / D3 / D16 / D17 / D18）。
//! 合言葉・印・その SHA-256 はログにも応答にも出さない（製造準備 A-2）。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier};
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
use uuid::Uuid;

use crate::access_log::{via_of, AccessEntry};
use crate::{internal_at, token_matches, App};
use ashiato_build_stamp::Channel;

/// 印を運ぶ cookie の名前（D3）。
pub const COOKIE_NAME: &str = "ashiato_session";

/// 系統ごとの画面ログイン用 cookie 名を返す。
pub const fn cookie_name(channel: Channel) -> &'static str {
    match channel {
        Channel::Dev => "ashiato_session_dev",
        Channel::Official => COOKIE_NAME,
    }
}
/// cookie の寿命。ブラウザは 400 日に切り詰める（D18）ので、それを最大として使うたびに出し直す。
const COOKIE_MAX_AGE_SECS: u64 = 400 * 24 * 60 * 60;
/// 画面の合言葉の下限（D16（仮））。
const MIN_WEB_PASSWORD_LEN: usize = 16;
const MIN_WEB_SESSION_KEY_LEN: usize = 16;
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
    /// 衛星の資格情報（ST29 / D8）。番号（uuid）だけを持つ。識別子の文字列は持たない（D4）。
    Satellite {
        satellite: Uuid,
        credential: Uuid,
    },
}

impl Caller {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WebSession => "web_session",
            Self::ApiToken => "api_token",
            Self::Satellite { .. } => "satellite",
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
    Malformed,
    RawPasswordPresent,
    /// 開発の口（環境変数で起こす形）に照らすための値が渡された。開発の口は元の値の形だけ（D8）。
    VerifierInDevelopment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSessionKeyRefusal {
    Missing,
    TooShort,
    SameAsApiToken,
    SameAsWebPassword,
}

impl WebSessionKeyRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::TooShort => "too_short",
            Self::SameAsApiToken => "same_as_api_token",
            Self::SameAsWebPassword => "same_as_web_password",
        }
    }
}

impl WebPasswordRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::TooShort => "too_short",
            Self::Placeholder => "placeholder",
            Self::SameAsApiToken => "same_as_api_token",
            Self::Malformed => "malformed",
            Self::RawPasswordPresent => "raw_password_present",
            Self::VerifierInDevelopment => "verifier_in_development",
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

/// 照らす形の設定にある PHC 文字列を検査し、API の合言葉との一致も拒む。
pub fn check_web_password_verifier(
    verifier: Option<&str>,
    api_token: &str,
) -> Result<(), WebPasswordRefusal> {
    let verifier = verifier
        .filter(|value| !value.is_empty())
        .ok_or(WebPasswordRefusal::Missing)?;
    let hash = PasswordHash::new(verifier).map_err(|_| WebPasswordRefusal::Malformed)?;
    if hash.algorithm != Algorithm::Argon2id.ident() {
        return Err(WebPasswordRefusal::Malformed);
    }
    check_verifier_against_api(verifier, api_token)
}

/// インストールした形では、元の画面の合言葉を設定に置かない。
pub fn check_installed_password_configuration(
    raw_password: Option<&str>,
    verifier: Option<&str>,
) -> Result<(), WebPasswordRefusal> {
    if raw_password.is_some() {
        return Err(WebPasswordRefusal::RawPasswordPresent);
    }
    if verifier.is_none() {
        return Err(WebPasswordRefusal::Missing);
    }
    Ok(())
}

/// PHC 文字列と入力値を Argon2id で照合する。
pub fn verify_password_verifier(verifier: &str, password: &str) -> bool {
    PasswordHash::new(verifier).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

pub fn check_verifier_against_api(
    verifier: &str,
    api_token: &str,
) -> Result<(), WebPasswordRefusal> {
    if verify_password_verifier(verifier, api_token) {
        Err(WebPasswordRefusal::SameAsApiToken)
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn password_verifier_for_test(
    password: &str,
) -> Result<String, argon2::password_hash::Error> {
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::PasswordHasher as _;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

pub fn check_web_session_key(
    key: Option<&str>,
    api_token: &str,
    web_password: &str,
) -> Result<(), WebSessionKeyRefusal> {
    match key {
        None | Some("") => Err(WebSessionKeyRefusal::Missing),
        Some(key) if key.chars().count() < MIN_WEB_SESSION_KEY_LEN => {
            Err(WebSessionKeyRefusal::TooShort)
        }
        Some(key) if token_matches(key, api_token) => Err(WebSessionKeyRefusal::SameAsApiToken),
        Some(key) if token_matches(key, web_password) => {
            Err(WebSessionKeyRefusal::SameAsWebPassword)
        }
        Some(_) => Ok(()),
    }
}

/// 照らす形で `WEB_SESSION_KEY` が画面の合言葉と同じでないかを検査する。
pub fn check_web_session_key_verifier(
    key: Option<&str>,
    api_token: &str,
    verifier: &str,
) -> Result<(), WebSessionKeyRefusal> {
    match key {
        None | Some("") => Err(WebSessionKeyRefusal::Missing),
        Some(key) if key.chars().count() < MIN_WEB_SESSION_KEY_LEN => {
            Err(WebSessionKeyRefusal::TooShort)
        }
        Some(key) if token_matches(key, api_token) => Err(WebSessionKeyRefusal::SameAsApiToken),
        Some(key) if verify_password_verifier(verifier, key) => {
            Err(WebSessionKeyRefusal::SameAsWebPassword)
        }
        Some(_) => Ok(()),
    }
}

/// 合言葉の世代の印（D7）。HMAC-SHA256、鍵は `WEB_SESSION_KEY`。
pub fn secret_tag(session_key: &str, web_password: &str) -> Vec<u8> {
    // HMAC はどの長さの鍵も受ける
    let mut mac = Hmac::<Sha256>::new_from_slice(session_key.as_bytes())
        .unwrap_or_else(|_| unreachable!("HMAC は任意の長さの鍵を受ける"));
    mac.update(b"ashiato-web-session-v1");
    mac.update(web_password.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// 照らす形で使う印の材料。verifier の塩を含め、決め直しで全印を失効させる。
pub fn secret_tag_for_verifier(session_key: &str, verifier: &str) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(session_key.as_bytes())
        .unwrap_or_else(|_| unreachable!("HMAC は任意の長さの鍵を受ける"));
    mac.update(b"ashiato-web-session-v2");
    mac.update(verifier.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

pub(crate) fn sha256(s: &str) -> Vec<u8> {
    Sha256::digest(s.as_bytes()).to_vec()
}

/// ログインの失敗を数える。数は機械全体で 1 つ（D17。呼び出し元ごとに分けない）。
#[derive(Debug, Default)]
pub struct LoginLimiter(Mutex<VecDeque<Instant>>);

impl LoginLimiter {
    fn recent(q: &mut VecDeque<Instant>, now: Instant) -> usize {
        while q
            .front()
            .is_some_and(|t| now.saturating_duration_since(*t) >= THROTTLE_WINDOW)
        {
            q.pop_front();
        }
        q.len()
    }

    /// 窓の中の失敗が上限に達しているか。
    fn throttled(&self) -> bool {
        self.throttled_at(Instant::now())
    }

    fn throttled_at(&self, now: Instant) -> bool {
        let mut q = self.0.lock().unwrap_or_else(|e| e.into_inner());
        Self::recent(&mut q, now) >= THROTTLE_MAX_FAILURES
    }

    fn record_failure(&self) {
        self.record_failure_at(Instant::now());
    }

    fn record_failure_at(&self, now: Instant) {
        let mut q = self.0.lock().unwrap_or_else(|e| e.into_inner());
        Self::recent(&mut q, now);
        q.push_back(now);
    }
}

/// `WEB_SESSION_MAX_AGE_DAYS` の値からログインの期限（日）を決める。**設定が無ければ 0 = 期限なし**（Q6 / D18）。
pub fn session_max_age_days(var: Option<&str>) -> anyhow::Result<u32> {
    match var {
        None => Ok(0),
        Some(v) => v
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("WEB_SESSION_MAX_AGE_DAYS が日数（整数）でない")),
    }
}

/// ログインの合言葉の照合を 1 つずつ通す（ST55 review R23 の security の指摘）。
static LOGIN_CHECK: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

/// 試験が照合の 1 つずつの印を握り、照合を「印を待つ所」で止める（review/code.md R10。
/// 取り消しを照合の途中に、時刻に依らず置くため）。
#[cfg(test)]
pub(crate) async fn hold_login_check_for_test() -> tokio::sync::SemaphorePermit<'static> {
    login_check_permit().await
}

/// 照合の 1 つずつの印を取る。印は static で、誰も閉じない（review/code.md R9）。
async fn login_check_permit() -> tokio::sync::SemaphorePermit<'static> {
    LOGIN_CHECK.acquire().await.expect("1 つずつの印は閉じない")
}

/// ログインの設定と状態。`App` が持つ。
#[derive(Clone, Debug)]
pub struct WebLogin {
    #[cfg(test)]
    session_key: String,
    password: String,
    verifier: Option<String>,
    tag: Vec<u8>,
    /// 0 = 期限なし（D18）。
    max_age_days: u32,
    /// 失敗のたびに待たせる長さ（D17。試験は差し替える）。
    pub(crate) failure_delay: Duration,
    limiter: Arc<LoginLimiter>,
    /// 照合が 1 つずつの印を待ち始めた合図（試験が照合の途中で取り消すため。review/code.md R11・R12）。
    #[cfg(test)]
    check_waiting: Arc<tokio::sync::Notify>,
}

impl WebLogin {
    pub fn new(session_key: &str, web_password: &str, max_age_days: u32) -> Self {
        Self {
            #[cfg(test)]
            session_key: session_key.to_owned(),
            password: web_password.to_owned(),
            verifier: None,
            tag: secret_tag(session_key, web_password),
            max_age_days,
            failure_delay: Duration::from_secs(1),
            limiter: Arc::default(),
            #[cfg(test)]
            check_waiting: Arc::default(),
        }
    }

    /// 照らす形のログイン状態を作る。元の合言葉は保持しない。
    pub fn new_with_verifier(session_key: &str, verifier: &str, max_age_days: u32) -> Self {
        Self {
            #[cfg(test)]
            session_key: session_key.to_owned(),
            password: String::new(),
            verifier: Some(verifier.to_owned()),
            tag: secret_tag_for_verifier(session_key, verifier),
            max_age_days,
            failure_delay: Duration::from_secs(1),
            limiter: Arc::default(),
            #[cfg(test)]
            check_waiting: Arc::default(),
        }
    }

    fn accepts(&self, password: &str) -> bool {
        self.verifier.as_deref().map_or_else(
            || !password.is_empty() && token_matches(password, &self.password),
            |verifier| verify_password_verifier(verifier, password),
        )
    }

    /// この login の照合が 1 つずつの印を待ち始めた合図を待つ（並走する他の試験の照合とは混ざらない）。
    /// 合図は溜まらず、作った後の合図だけを受ける（前のログインの照合の合図を拾わない）ので、求めを出す前に作る。
    #[cfg(test)]
    pub(crate) fn check_waiting_for_test(&self) -> tokio::sync::futures::Notified<'_> {
        self.check_waiting.notified()
    }

    #[cfg(test)]
    pub(crate) fn password_for_test(&self) -> &str {
        &self.password
    }

    #[cfg(test)]
    pub(crate) fn session_key_for_test(&self) -> &str {
        &self.session_key
    }

    /// 鍵だけを差し替えた複製。合言葉・期限・失敗の待ちは保つ（鍵を変えて起動し直したときと同じ）。
    #[cfg(test)]
    pub(crate) fn with_session_key_for_test(&self, session_key: &str) -> Self {
        let mut login = self.verifier.as_deref().map_or_else(
            || Self::new(session_key, &self.password, self.max_age_days),
            |verifier| Self::new_with_verifier(session_key, verifier, self.max_age_days),
        );
        login.failure_delay = self.failure_delay;
        login
    }

    #[cfg(test)]
    pub(crate) fn status_for_tag_for_test(&self, tag: &[u8]) -> StatusCode {
        if self.tag == tag {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::UNAUTHORIZED
        }
    }

    #[cfg(test)]
    pub(crate) fn tag_for_test(&self) -> &[u8] {
        &self.tag
    }
}

/// cookie ヘッダから印を取り出す。
pub(crate) fn cookie_token(headers: &HeaderMap, channel: Channel) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find_map(|(k, v)| (k == cookie_name(channel)).then_some(v))
}

fn set_cookie(value: &str, max_age: u64, channel: Channel) -> HeaderValue {
    // 印は base64url なので header に載せられる
    HeaderValue::from_str(&format!(
        "{}={value}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={max_age}",
        cookie_name(channel)
    ))
    .unwrap_or_else(|_| HeaderValue::from_static(""))
}

/// 印が有効か（D3 の有効の条件）。
pub(crate) async fn session_valid(app: &App, headers: &HeaderMap) -> Result<bool, sqlx::Error> {
    let Some(token) = cookie_token(headers, app.channel) else {
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

/// 最も外側の層。**この求めの認可をここで 1 回だけ判定し**（`crate::AUTHN`。final review R3）、
/// 内側（読み出しの記録・ハンドラ）はその判定を読む。
/// 印で認めた応答には、寿命を出し直す `Set-Cookie` を付ける（D18）。ただし応答がもう
/// `Set-Cookie` を持っているとき（ログイン・ログアウト）は付けない —— 2 本並ぶとブラウザは
/// 後のほう（古い印）を採る（final review R2）。
pub async fn refresh_cookie(State(app): State<App>, req: Request, next: Next) -> Response {
    let token = cookie_token(req.headers(), app.channel).map(str::to_owned);
    let authn = crate::decide(&app, req.headers()).await;
    let mut res = crate::AUTHN.scope(authn, next.run(req)).await;
    if let Some(token) = token {
        if authn == crate::Authn::Decided(Some(Caller::WebSession))
            && res.status() != StatusCode::UNAUTHORIZED
            && !res.headers().contains_key(header::SET_COOKIE)
        {
            res.headers_mut().append(
                header::SET_COOKIE,
                set_cookie(&token, COOKIE_MAX_AGE_SECS, app.channel),
            );
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CollectorSetupResponse {
    pub token: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CollectorSetupError {
    pub error: &'static str,
}

/// `/session` の求めの 1 行。ログインの結果はハンドラが判定した場で書く（D8）。
fn session_entry(
    headers: &HeaderMap,
    caller: Option<Caller>,
    method: &str,
    outcome: &'static str,
    status: StatusCode,
) -> AccessEntry {
    AccessEntry::for_caller(
        caller,
        via_of(headers),
        "/session".into(),
        method.into(),
        outcome,
        status.as_u16(),
    )
}

fn unauthorized() -> (StatusCode, String) {
    (StatusCode::UNAUTHORIZED, "unauthorized".into())
}

fn collector_setup_entry(
    headers: &HeaderMap,
    caller: Option<Caller>,
    outcome: &'static str,
    status: StatusCode,
) -> AccessEntry {
    AccessEntry::for_caller(
        caller,
        via_of(headers),
        "/collector-setup".into(),
        "POST".into(),
        outcome,
        status.as_u16(),
    )
}

/// 本文が JSON と名乗っているか（ログインと収集アプリの設定の口）。
fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.trim()
                .to_ascii_lowercase()
                .starts_with("application/json")
        })
}

/// 収集アプリの資格情報を、画面の有効なログインに一度だけ返す。
#[utoipa::path(post, path = "/collector-setup", request_body = LoginRequest,
    responses((status = 200, body = CollectorSetupResponse),
              (status = 401), (status = 403, body = CollectorSetupError), (status = 415), (status = 429)))]
pub async fn collector_setup_post(
    State(app): State<App>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, String)> {
    let caller = match crate::decided(&app, &headers).await {
        crate::Authn::Decided(caller) => caller,
        crate::Authn::LookupFailed => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal error".into()))
        }
    };
    let Some(caller) = caller else {
        app.access
            .write(collector_setup_entry(
                &headers,
                None,
                "unauthorized",
                StatusCode::UNAUTHORIZED,
            ))
            .await?;
        return Err(unauthorized());
    };
    if caller != Caller::WebSession {
        app.access
            .write(collector_setup_entry(
                &headers,
                Some(caller),
                "forbidden",
                StatusCode::FORBIDDEN,
            ))
            .await?;
        return Err((StatusCode::FORBIDDEN, "forbidden".into()));
    }
    // ログインの口と同じく JSON だけを受ける（よそのサイトの単純な text/plain の送信を受けない。review/code.md R40）
    if !is_json(&headers) {
        app.access
            .write(collector_setup_entry(
                &headers,
                Some(caller),
                "forbidden",
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ))
            .await?;
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported media type".into(),
        ));
    }
    run_to_completion(collector_setup_checked(app, headers, caller, body)).await
}

/// 照合が要る求めを、照合から行を書いて答えるまで、求めが取り消されても走り切る task で通す（review/code.md R1）。
/// 取り消された求めの照合も失敗に数えるので、その行も残す（data-sensitivity「1 件ごとに 1 行」）。
async fn run_to_completion(
    work: impl std::future::Future<Output = Result<Response, (StatusCode, String)>> + Send + 'static,
) -> Result<Response, (StatusCode, String)> {
    tokio::spawn(work)
        .await
        .unwrap_or_else(|_| Err((StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())))
}

/// 資格情報の口の、照合から後（`collector_setup_post` が `run_to_completion` で通す）。
async fn collector_setup_checked(
    app: App,
    headers: HeaderMap,
    caller: Caller,
    body: Bytes,
) -> Result<Response, (StatusCode, String)> {
    let check = check_given_password(&app, &body).await;
    if check == PasswordCheck::Throttled {
        app.access
            .write(collector_setup_entry(
                &headers,
                Some(caller),
                "login_throttled",
                StatusCode::TOO_MANY_REQUESTS,
            ))
            .await?;
        return Err((StatusCode::TOO_MANY_REQUESTS, "too many requests".into()));
    }
    if check == PasswordCheck::Mismatch {
        app.access
            .write(collector_setup_entry(
                &headers,
                Some(caller),
                "forbidden",
                StatusCode::FORBIDDEN,
            ))
            .await?;
        tokio::time::sleep(app.login.failure_delay).await;
        return Ok((
            StatusCode::FORBIDDEN,
            Json(CollectorSetupError {
                error: "wrong_password",
            }),
        )
            .into_response());
    }
    app.access
        .write(collector_setup_entry(
            &headers,
            Some(caller),
            "ok",
            StatusCode::OK,
        ))
        .await?;
    Ok(Json(CollectorSetupResponse {
        token: app.collector_token().to_owned(),
    })
    .into_response())
}

/// 合言葉の照合の結果。ログインの口と資格情報を返す口が共に使う。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PasswordCheck {
    /// 失敗が多いので照合せず断る（失敗には数えない）。
    Throttled,
    Match,
    Mismatch,
}

/// 求めの本文から入れられた合言葉を取る。無い・読めないときは空。
fn given_password(body: &[u8]) -> String {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("password")
                .and_then(|p| p.as_str().map(str::to_owned))
        })
        .unwrap_or_default()
}

/// 入れられた合言葉を、2 つの口で同じ道に通して照合する（CH15 design D1）。
/// 断る数の判定から失敗の数え上げまでを 1 つずつ通す。照合を blocking の側へ逃がしても、
/// 数え上げより先に照合が並んで上限をすり抜けないように（review R23）。
pub(crate) async fn check_given_password(app: &App, body: &[u8]) -> PasswordCheck {
    // 断られている間は、1 つずつの印を待たず・本文も読まずに断る（review/code.md R4・R5）
    if app.login.limiter.throttled() {
        return PasswordCheck::Throttled;
    }
    #[cfg(test)]
    app.login.check_waiting.notify_waiters();
    let permit = login_check_permit().await;
    // 印を待つ間に上限に届いたかもしれないので、印を持ってから数え直す
    if app.login.limiter.throttled() {
        return PasswordCheck::Throttled;
    }
    let given = given_password(body);
    // 空は照合せず失敗に数える（照らす形の `accepts` は空を自分では断らず、Argon2id を回してしまう）
    if given.is_empty() {
        app.login.limiter.record_failure();
        return PasswordCheck::Mismatch;
    }
    // Argon2id の照合は CPU を数十ミリ秒使うので、async の worker を塞がないよう blocking の側で行う（ST55 review R23）。
    // 1 つずつの印（permit）と失敗の数え上げも blocking の側へ渡し、照合が終わって数え上げるまで次の照合を始めない（review R78）
    let login = app.login.clone();
    let accepted = tokio::task::spawn_blocking(move || {
        let accepted = login.accepts(&given);
        if !accepted {
            login.limiter.record_failure();
        }
        drop(permit);
        accepted
    })
    .await
    .unwrap_or_else(|_| {
        app.login.limiter.record_failure();
        false
    });
    if accepted {
        PasswordCheck::Match
    } else {
        PasswordCheck::Mismatch
    }
}

/// ログイン。**画面の合言葉そのものを資格情報として要求する**（`authorize()` は通さない）。
#[utoipa::path(post, path = "/session", request_body = LoginRequest,
    responses((status = 204), (status = 401), (status = 415), (status = 429)))]
pub async fn session_post(
    State(app): State<App>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, String)> {
    if !is_json(&headers) {
        app.access
            .write(session_entry(
                &headers,
                None,
                "POST",
                "login_failed",
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ))
            .await?;
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported media type".into(),
        ));
    }
    run_to_completion(session_post_checked(app, headers, body)).await
}

/// ログインの口の、照合から後（`session_post` が `run_to_completion` で通す）。
async fn session_post_checked(
    app: App,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, String)> {
    let check = check_given_password(&app, &body).await;
    if check == PasswordCheck::Throttled {
        tracing::warn!(kind = "login_throttled", "ログインの失敗が多いので断る");
        app.access
            .write(session_entry(
                &headers,
                None,
                "POST",
                "login_throttled",
                StatusCode::TOO_MANY_REQUESTS,
            ))
            .await?;
        return Err((StatusCode::TOO_MANY_REQUESTS, "too many requests".into()));
    }
    if check == PasswordCheck::Mismatch {
        tracing::warn!(kind = "login_failed", "ログインの合言葉が一致しない");
        app.access
            .write(session_entry(
                &headers,
                None,
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
    let inserted = sqlx::query(
        "INSERT INTO core.web_session (token_sha256, secret_tag, issued_at) VALUES ($1, $2, $3)",
    )
    .bind(sha256(&token))
    .bind(&app.login.tag)
    .bind(app.now())
    .execute(&app.pool)
    .await;
    if let Err(e) = inserted {
        let err = internal_at("session.insert", e);
        // 印を渡せなかったログインも 1 行残す（D8。final review R8）。書けなくても 500 のまま
        app.access
            .write(session_entry(
                &headers,
                None,
                "POST",
                "login_failed",
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
            .await?;
        return Err(err);
    }
    // 書けなければ印を渡さない（渡さない印は使えない）
    app.access
        .write(session_entry(
            &headers,
            None,
            "POST",
            "login_ok",
            StatusCode::NO_CONTENT,
        ))
        .await?;
    let mut res = StatusCode::NO_CONTENT.into_response();
    res.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(&token, COOKIE_MAX_AGE_SECS, app.channel),
    );
    Ok(res)
}

/// ログアウト。印が無くても 204。行は消さず `revoked_at` を付ける。
#[utoipa::path(delete, path = "/session", responses((status = 204)))]
pub async fn session_delete(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let caller = if session_valid(&app, &headers)
        .await
        .map_err(|e| internal_at("session.lookup", e))?
    {
        Some(Caller::WebSession)
    } else {
        None
    };
    app.access
        .write(session_entry(
            &headers,
            caller,
            "DELETE",
            "logout",
            StatusCode::NO_CONTENT,
        ))
        .await?;
    if let Some(token) = cookie_token(&headers, app.channel) {
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
        .append(header::SET_COOKIE, set_cookie("", 0, app.channel));
    Ok(res)
}

/// ログインの状態。画面の印で認められれば、その種類を返す。API の合言葉と衛星の資格情報では 403。
#[utoipa::path(get, path = "/session",
    responses((status = 200, body = SessionState), (status = 401),
        (status = 403, description = "API の合言葉では通らない")))]
pub async fn session_get(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<SessionState>, (StatusCode, String)> {
    let caller = match crate::authorize(&app, &headers).await {
        Ok(c) => c,
        Err(e) => {
            // 衛星の求めは 403。middleware を通らない口なのでここで同じ行を書く（D8）
            if e.0 == StatusCode::FORBIDDEN {
                if let crate::Authn::Decided(Some(c @ Caller::Satellite { .. })) =
                    crate::decided(&app, &headers).await
                {
                    app.access
                        .write(session_entry(
                            &headers,
                            Some(c),
                            "GET",
                            "forbidden",
                            StatusCode::FORBIDDEN,
                        ))
                        .await?;
                }
            }
            // 401 も残す（画面は起動時に必ずここを引く）。判定そのものの失敗（500）は書かない
            if e.0 == StatusCode::UNAUTHORIZED {
                app.access
                    .write(session_entry(
                        &headers,
                        None,
                        "GET",
                        "unauthorized",
                        StatusCode::UNAUTHORIZED,
                    ))
                    .await?;
            }
            return Err(e);
        }
    };
    // 種類が増えたらここで止まるよう、`_` の無い match にする（design D6）
    match caller {
        // API の合言葉ではログインの状態も読めない（design D12（仮））。
        // 衛星は authorize が先に 403 で返すので届かないが、届いても同じく断る
        Caller::ApiToken | Caller::Satellite { .. } => {
            app.access
                .write(session_entry(
                    &headers,
                    Some(caller),
                    "GET",
                    "forbidden",
                    StatusCode::FORBIDDEN,
                ))
                .await?;
            return Err((StatusCode::FORBIDDEN, "forbidden".into()));
        }
        Caller::WebSession => {}
    }
    app.access
        .write(session_entry(
            &headers,
            Some(caller),
            "GET",
            "ok",
            StatusCode::OK,
        ))
        .await?;
    Ok(Json(SessionState {
        credential: caller.as_str().into(),
    }))
}

/// 仮決めの値（D17 の窓と待ち・D18 の cookie の寿命）を固定する（code-verify R15）。
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn login_limiter_window_is_60_seconds() {
        let limiter = LoginLimiter::default();
        let t0 = Instant::now();
        for _ in 0..THROTTLE_MAX_FAILURES {
            limiter.record_failure_at(t0);
        }
        assert!(limiter.throttled_at(t0 + Duration::from_secs(59)));
        assert!(!limiter.throttled_at(t0 + Duration::from_secs(60)));
    }

    #[test]
    fn login_failure_delay_default_is_1_second() {
        assert_eq!(
            WebLogin::new("session-key", "p", 0).failure_delay,
            Duration::from_secs(1)
        );
    }

    #[test]
    fn cookie_max_age_is_400_days() {
        assert_eq!(COOKIE_MAX_AGE_SECS, 34_560_000);
    }

    #[test]
    fn session_max_age_days_unset_is_never() {
        assert_eq!(session_max_age_days(None).unwrap(), 0);
        assert_eq!(session_max_age_days(Some(" 30 ")).unwrap(), 30);
        assert!(session_max_age_days(Some("30d")).is_err());
    }
}
