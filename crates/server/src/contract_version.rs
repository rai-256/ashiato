// SPDX-License-Identifier: AGPL-3.0-only
//! 収集アプリとの契約の版と、それを確かめる口 `GET /contract`（ST50 / design D4 / D15）。
//!
//! 版は「その版以下を全部受け付ける」の意味。**最初の配った印までは上げない**（D8）。
//! 版を設定や環境変数で変える口は作らない —— `App` の持ち物は試験だけが差し替える。

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{authorize, web_session::Caller, App};

/// S-01 が受け付ける一番新しい契約の版。
pub const CONTRACT_VERSION: u32 = 1;

/// 要求が契約の版を載せる見出し。
pub const CONTRACT_HEADER: &str = "Ashiato-Contract-Version";

/// 見出しが正の整数として読めない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unreadable;

/// 要求の見出しから契約の版を読む。無ければ 1（見出しを載せる前の収集アプリ）、
/// 正の整数でなければ「読めない」。
pub fn requested_version(headers: &HeaderMap) -> Result<u32, Unreadable> {
    let Some(value) = headers.get(CONTRACT_HEADER) else {
        return Ok(1);
    };
    let text = value.to_str().map_err(|_| Unreadable)?;
    // 符号・空白・先頭の `+` を認めない（`u32::from_str` は `+3` を通す）
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Unreadable);
    }
    match text.parse::<u32>() {
        Ok(v) if v >= 1 => Ok(v),
        _ => Err(Unreadable),
    }
}

/// `GET /contract` の答え。
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ContractAnswer {
    /// S-01 が受け付ける一番新しい契約の版（その版以下を全部受け付ける）
    pub contract_version: u32,
}

/// 契約の版を答える。API の合言葉だけが通る（画面のログイン・衛星は 403、無ければ 401）。
#[utoipa::path(get, path = "/contract",
    responses((status = 200, body = ContractAnswer), (status = 401), (status = 403)))]
pub async fn contract_get(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<ContractAnswer>, (StatusCode, String)> {
    match authorize(&app, &headers).await? {
        Caller::ApiToken => Ok(Json(ContractAnswer {
            contract_version: app.contract_version,
        })),
        _ => Err((StatusCode::FORBIDDEN, "forbidden".into())),
    }
}

/// 要求の版が S-01 の版を超える（または読めない）ときの断り。何も格納せず 426 を返す（D4）。
#[derive(Debug, Clone, Copy)]
pub struct Unsupported {
    /// S-01 が受け付ける版（本文に載せる）
    pub supported: u32,
    /// `HEAD` は本文を持てない
    pub bodyless: bool,
}

impl IntoResponse for Unsupported {
    fn into_response(self) -> Response {
        if self.bodyless {
            return StatusCode::UPGRADE_REQUIRED.into_response();
        }
        (
            StatusCode::UPGRADE_REQUIRED,
            Json(serde_json::json!({
                "error": "contract_unsupported",
                "contract_version": self.supported,
            })),
        )
            .into_response()
    }
}

/// 収集アプリの要求（`Caller::ApiToken`）の見出しを検査する。画面のログイン・衛星には掛けない（D16）。
/// 版の値と断りの種別だけをログに出す（記録の値は出さない）。
pub fn check_header(
    app: &App,
    caller: &Caller,
    headers: &HeaderMap,
    bodyless: bool,
) -> Result<(), Unsupported> {
    if !matches!(caller, Caller::ApiToken) {
        return Ok(());
    }
    let refuse = |requested: Option<u32>| {
        // log-ok: 契約の版は整数の定数で、記録の値ではない（ST50 の規則）
        tracing::warn!(
            kind = "contract_unsupported",
            requested = requested.map(i64::from).unwrap_or(-1),
            supported = app.contract_version,
            "新しすぎる契約の版の要求を断った"
        );
        Err(Unsupported {
            supported: app.contract_version,
            bodyless,
        })
    };
    match requested_version(headers) {
        Ok(v) if v <= app.contract_version => Ok(()),
        Ok(v) => refuse(Some(v)),
        Err(Unreadable) => refuse(None),
    }
}

/// 収集アプリが送る口の断り。従来の `(状態符号, 文字)` の断りと 426 を同じ `?` で返せる。
#[derive(Debug)]
pub struct Refusal(Response);

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        self.0
    }
}

impl From<(StatusCode, String)> for Refusal {
    fn from(e: (StatusCode, String)) -> Self {
        Self(e.into_response())
    }
}

impl From<Unsupported> for Refusal {
    fn from(u: Unsupported) -> Self {
        Self(u.into_response())
    }
}

/// `authorize` の直後に見出しを検査する。収集アプリが送る口（取り込み・生存信号・破棄・原本の欠け）が使う。
pub async fn authorize_versioned(
    app: &App,
    headers: &HeaderMap,
) -> Result<crate::web_session::Caller, Refusal> {
    let caller = authorize(app, headers).await?;
    check_header(app, &caller, headers, false)?;
    Ok(caller)
}
