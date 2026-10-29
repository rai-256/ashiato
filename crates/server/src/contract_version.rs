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
/// 呼び出し元を返す（1 件ごとの検査 [`ItemGate`] が使う）。
pub async fn authorize_versioned(app: &App, headers: &HeaderMap) -> Result<Caller, Refusal> {
    let caller = authorize(app, headers).await?;
    check_header(app, &caller, headers, false)?;
    Ok(caller)
}

/// 型が知る最上位の欄の名前。`utoipa` の schema から作る（手で並べない。D2）。
pub fn known_fields<T: utoipa::PartialSchema>() -> Vec<String> {
    use utoipa::openapi::{RefOr, Schema};
    match T::schema() {
        RefOr::T(Schema::Object(o)) => o.properties.keys().cloned().collect(),
        // 型が object の schema でないなら、知る欄を数えられない。空にすると全部を断ってしまうので落とす
        _ => unreachable!("1 件の型は object の schema を持つ"),
    }
}

/// 1 件の最上位の鍵に、`known` に無いものがあるか（`payload` などの中は見ない）。
pub fn has_unknown_field(item: &serde_json::Value, known: &[String]) -> bool {
    item.as_object()
        .is_some_and(|o| o.keys().any(|k| !known.iter().any(|f| f == k)))
}

/// 1 件を格納する前の断り。収集アプリが捨てない種別（D2・D4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemRefusal {
    UnknownField,
    ContractUnsupported,
}

/// 1 件ごとの検査。`Caller::ApiToken` の要求にだけ掛ける（D16）。
#[derive(Debug)]
pub struct ItemGate {
    strict: bool,
    known: Vec<String>,
    supported: u32,
}

impl ItemGate {
    pub fn new<T: utoipa::PartialSchema>(app: &App, caller: &Caller) -> Self {
        let strict = matches!(caller, Caller::ApiToken);
        Self {
            strict,
            known: if strict {
                known_fields::<T>()
            } else {
                Vec::new()
            },
            supported: app.contract_version,
        }
    }

    /// 型の解釈（`serde_json::from_value`）の前に見る。知らない欄 → 1 件の版の順。
    pub fn check(&self, item: &serde_json::Value) -> Option<ItemRefusal> {
        if !self.strict {
            return None;
        }
        let refusal = if has_unknown_field(item, &self.known) {
            ItemRefusal::UnknownField
        } else if !self.item_version_ok(item) {
            ItemRefusal::ContractUnsupported
        } else {
            return None;
        };
        // log-ok: 断りの種別と版の定数だけ。記録の値は出さない
        tracing::warn!(
            kind = match refusal {
                ItemRefusal::UnknownField => "unknown_field",
                ItemRefusal::ContractUnsupported => "contract_unsupported",
            },
            supported = self.supported,
            "契約に合わない 1 件を断った"
        );
        Some(refusal)
    }

    /// 欄が無い・null は版 1。あれば正の整数で S-01 の版以下。
    fn item_version_ok(&self, item: &serde_json::Value) -> bool {
        match item.get("contract_version") {
            None | Some(serde_json::Value::Null) => true,
            Some(v) => v
                .as_i64()
                .is_some_and(|n| n >= 1 && n <= i64::from(self.supported)),
        }
    }
}

/// 形が壊れていても拾えるなら、送った識別子を結果に載せる（対応づけの助け）。
pub fn sent_id(item: &serde_json::Value) -> Option<uuid::Uuid> {
    item.get("id")
        .and_then(|v| v.as_str())
        .and_then(|v| uuid::Uuid::parse_str(v).ok())
}
