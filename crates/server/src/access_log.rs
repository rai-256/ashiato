// SPDX-License-Identifier: AGPL-3.0-only
//! 読み出しの記録（`core.access_log`。ST28 / design D8 / C5）。
//! 残すのは 経路・資格情報の種類・route の型・method・結果・status だけ。
//! **クエリ文字列・path の値・合言葉・印・本文は入れない**（spec「中身も合言葉も残らない」）。

use std::sync::Arc;

use axum::{
    extract::{MatchedPath, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse as _, Response},
};

use crate::{decided, internal_at, App, Authn};

/// 記録の経路（D8）。`x-forwarded-for` があれば網越し（`tailscale serve` が付ける）。
/// **同じ PC のプロセスは偽れる**（spec の注記）。
pub fn via_of(headers: &HeaderMap) -> &'static str {
    if headers.contains_key("x-forwarded-for") {
        "forwarded"
    } else {
        "direct"
    }
}

/// 読み出しの記録の 1 行。値はどれも固定の語彙か route の型で、利用者のデータを含まない。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessEntry {
    /// `direct` | `forwarded`
    pub via: &'static str,
    /// `web_session` | `api_token` | `none`
    pub credential: &'static str,
    /// axum の `MatchedPath`（`/stays`）。値の入った path は入れない。
    pub route: String,
    pub method: String,
    /// `ok` | `unauthorized` | `login_ok` | `login_failed` | `login_throttled` | `logout`
    pub outcome: &'static str,
    pub status: u16,
}

/// 書く口が返す future。
pub type AccessFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), sqlx::Error>> + Send>>;

/// 読み出しの記録を書く口（design D8）。
///
/// **差し替えられる形にしてある** —— 書けないときに記録を返さないことを、表の権限を剥がさずに
/// 確かめるため（剥がすと並走する試験を壊す）。
pub trait AccessSink: Send + Sync {
    fn record(&self, entry: AccessEntry) -> AccessFuture;
}

/// 本物。`core.access_log` へ 1 行足す。
#[derive(Debug)]
pub struct PgAccessSink(pub sqlx::PgPool);

impl AccessSink for PgAccessSink {
    fn record(&self, e: AccessEntry) -> AccessFuture {
        let pool = self.0.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO core.access_log (via, credential, route, method, outcome, status)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(e.via)
            .bind(e.credential)
            .bind(e.route)
            .bind(e.method)
            .bind(e.outcome)
            .bind(i16::try_from(e.status).unwrap_or(0))
            .execute(&pool)
            .await
            .map(|_| ())
        })
    }
}

/// `App` が持つ書く口。
#[derive(Clone)]
pub struct AccessLog(Arc<dyn AccessSink>);

impl AccessLog {
    pub fn new(sink: impl AccessSink + 'static) -> Self {
        Self(Arc::new(sink))
    }

    /// 1 行書く。失敗は SQLSTATE 等の操作名だけをログに出し、呼び出し側が 500 にする。
    pub(crate) async fn write(&self, entry: AccessEntry) -> Result<(), (StatusCode, String)> {
        self.0
            .record(entry)
            .await
            .map_err(|e| internal_at("access_log.insert", e))
    }
}

impl std::fmt::Debug for AccessLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccessLog")
    }
}

/// 取り込みの口。**資格情報が認められたときは書かない**（spec）。断られたときは書く。
fn is_ingest(route: &str) -> bool {
    matches!(route, "/ingest" | "/heartbeat" | "/drops")
}

/// 1 求め 1 行を**ハンドラの前**に書く。書けなければハンドラを呼ばずに 500（D8）。
/// `/healthz` と `/session`（ハンドラが判定の場で自分で書く）はここを通らない。
pub async fn middleware(State(app): State<App>, req: Request, next: Next) -> Response {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_owned())
        .unwrap_or_default();
    if route == "/healthz" || route == "/session" {
        return next.run(req).await;
    }
    // 判定は外側の層が 1 回だけ下したもの（final review R3）。ハンドラも同じ判定を読む
    let caller = match decided(&app, req.headers()).await {
        Authn::Decided(c) => c,
        Authn::LookupFailed => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
        }
    };
    if caller.is_some() && is_ingest(&route) {
        return next.run(req).await;
    }
    let (credential, outcome, status) = match caller {
        Some(c) => (c.as_str(), "ok", 200),
        None => ("none", "unauthorized", 401),
    };
    let entry = AccessEntry {
        via: via_of(req.headers()),
        credential,
        route,
        method: req.method().as_str().to_owned(),
        outcome,
        status,
    };
    if let Err(e) = app.access.write(entry).await {
        return e.into_response();
    }
    next.run(req).await
}
