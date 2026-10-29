// SPDX-License-Identifier: AGPL-3.0-only
//! 収集アプリとの契約の版と、それを確かめる口 `GET /contract`（ST50 / design D4 / D15）。
//!
//! 版は「その版以下を全部受け付ける」の意味。**最初の配った印までは上げない**（D8）。
//! 版を設定や環境変数で変える口は作らない —— `App` の持ち物は試験だけが差し替える。

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
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
