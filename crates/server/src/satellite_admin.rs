// SPDX-License-Identifier: AGPL-3.0-only
//! 衛星の登録と外す、その一覧（ST29 / design D7 / D11 / D16）。**画面のログインでだけ**通る（D8）。
//! 衛星の判定は最も外側の層（`access_log::middleware`）と `authorize` が持つので、ここに衛星の分岐は書かない。
//! 行は消さない。外すのは `removed_at` と資格情報の `revoked_at` を付けるだけ。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use base64::Engine as _;
use rand::RngCore as _;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{authorize, internal_at, satellite, web_session, web_session::Caller, App};

/// 400 / 409 の本文の種別（D11）。画面が日本語の理由に引く。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SatelliteError {
    /// 識別子の形に合わない・長すぎる。
    Malformed,
    /// 本体が使う識別子（`ashiato.` で始まる）。
    Reserved,
    /// 本体の前置きの形で始まる。
    BodyPrefix,
    /// 名前が、前後の空白を除いて空か 41 文字以上。
    NameInvalid,
    /// 外していない同じ識別子の衛星がある。
    IdentifierTaken,
    /// 外していない同じ名前の衛星がある。
    NameTaken,
    /// 最初から登録されている衛星は外せない。
    Builtin,
    /// 外した衛星。
    Removed,
}

impl From<satellite::IdentifierRefusal> for SatelliteError {
    fn from(r: satellite::IdentifierRefusal) -> Self {
        match r {
            satellite::IdentifierRefusal::Malformed => Self::Malformed,
            satellite::IdentifierRefusal::Reserved => Self::Reserved,
            satellite::IdentifierRefusal::BodyPrefix => Self::BodyPrefix,
        }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SatelliteErrorBody {
    pub error: SatelliteError,
}

/// 口の断り。認可の断り（401 / 403 / 500）は本文が文字列、種別の断り（400 / 404 / 409）は JSON。
#[derive(Debug)]
pub enum Refusal {
    Auth(StatusCode, String),
    Kind(StatusCode, SatelliteError),
    NotFound,
    Internal(StatusCode, String),
}

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        match self {
            Self::Auth(s, b) | Self::Internal(s, b) => (s, b).into_response(),
            Self::Kind(s, error) => (s, Json(SatelliteErrorBody { error })).into_response(),
            Self::NotFound => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

fn internal(op: &'static str, e: sqlx::Error) -> Refusal {
    let (s, b) = internal_at(op, e);
    Refusal::Internal(s, b)
}

/// 画面のログインだけを通す。API の合言葉は 403、無ければ 401（`authorize`）。
async fn web_session_only(app: &App, headers: &axum::http::HeaderMap) -> Result<(), Refusal> {
    match authorize(app, headers).await {
        Ok(Caller::WebSession) => Ok(()),
        Ok(_) => Err(Refusal::Auth(StatusCode::FORBIDDEN, "forbidden".into())),
        Err((s, b)) => Err(Refusal::Auth(s, b)),
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CredentialOut {
    pub id: Uuid,
    pub label: String,
    pub issued_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// 衛星 1 つ。`credentials` は取り消していないものだけ（値もハッシュも載せない。D11）。
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SatelliteOut {
    pub id: Uuid,
    pub identifier: String,
    pub name: String,
    pub builtin: bool,
    pub registered_at: DateTime<Utc>,
    pub removed_at: Option<DateTime<Utc>>,
    pub credentials: Vec<CredentialOut>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    pub name: String,
    pub identifier: String,
}

type SatelliteRow = (
    Uuid,
    String,
    String,
    bool,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);

/// 資格情報の行：（衛星の番号, 番号, 名前, 発行, 最後に使った）。
type CredentialRow = (Uuid, Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>);

fn out(row: SatelliteRow, credentials: Vec<CredentialOut>) -> SatelliteOut {
    SatelliteOut {
        id: row.0,
        identifier: row.1,
        name: row.2,
        builtin: row.3,
        registered_at: row.4,
        removed_at: row.5,
        credentials,
    }
}

/// 外していない衛星（取り消していない資格情報つき）と外した衛星の一覧。
#[utoipa::path(get, path = "/satellites",
    responses((status = 200, body = Vec<SatelliteOut>), (status = 401), (status = 403), (status = 500)))]
pub async fn satellites_get(
    State(app): State<App>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<SatelliteOut>>, Refusal> {
    web_session_only(&app, &headers).await?;
    let rows: Vec<SatelliteRow> = sqlx::query_as(
        "SELECT id, identifier, name, builtin, registered_at, removed_at
           FROM core.satellite ORDER BY removed_at IS NOT NULL, registered_at, id",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(|e| internal("satellite.list", e))?;
    let creds: Vec<CredentialRow> = sqlx::query_as(
        "SELECT satellite_id, id, label, issued_at, last_used_at
           FROM core.satellite_credential WHERE revoked_at IS NULL ORDER BY issued_at, id",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(|e| internal("satellite.list_credentials", e))?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                let own = creds
                    .iter()
                    .filter(|c| c.0 == row.0)
                    .map(|c| CredentialOut {
                        id: c.1,
                        label: c.2.clone(),
                        issued_at: c.3,
                        last_used_at: c.4,
                    })
                    .collect();
                out(row, own)
            })
            .collect(),
    ))
}

/// 一意制約の違反を、重なりの種別へ引く。
fn conflict_of(e: &sqlx::Error) -> Option<SatelliteError> {
    let db = e.as_database_error()?;
    if !db.is_unique_violation() {
        return None;
    }
    match db.constraint() {
        Some("satellite_live_name") => Some(SatelliteError::NameTaken),
        Some("satellite_identifier_key") => Some(SatelliteError::IdentifierTaken),
        _ => None,
    }
}

/// 登録する。外した衛星と同じ識別子なら同じ行を戻す（200。`registered_at` は変えない。Q2）。
#[utoipa::path(post, path = "/satellites", request_body = RegisterRequest,
    responses((status = 201, body = SatelliteOut), (status = 200, body = SatelliteOut),
              (status = 400, body = SatelliteErrorBody), (status = 409, body = SatelliteErrorBody),
              (status = 401), (status = 403), (status = 500)))]
pub async fn satellites_post(
    State(app): State<App>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<SatelliteOut>), Refusal> {
    web_session_only(&app, &headers).await?;
    satellite::check_identifier(&req.identifier)
        .map_err(|r| Refusal::Kind(StatusCode::BAD_REQUEST, r.into()))?;
    let name = satellite::check_satellite_name(&req.name).ok_or(Refusal::Kind(
        StatusCode::BAD_REQUEST,
        SatelliteError::NameInvalid,
    ))?;

    let mut tx = app
        .pool
        .begin()
        .await
        .map_err(|e| internal("satellite.register", e))?;
    let existing: Option<(Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, removed_at FROM core.satellite WHERE identifier = $1 FOR UPDATE",
    )
    .bind(&req.identifier)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| internal("satellite.register", e))?;
    let (status, row): (StatusCode, Result<SatelliteRow, sqlx::Error>) = match existing {
        Some((_, None)) => {
            return Err(Refusal::Kind(
                StatusCode::CONFLICT,
                SatelliteError::IdentifierTaken,
            ))
        }
        Some((id, Some(_))) => (
            StatusCode::OK,
            sqlx::query_as(
                "UPDATE core.satellite SET removed_at = NULL, name = $2 WHERE id = $1
                 RETURNING id, identifier, name, builtin, registered_at, removed_at",
            )
            .bind(id)
            .bind(name)
            .fetch_one(&mut *tx)
            .await,
        ),
        None => (
            StatusCode::CREATED,
            sqlx::query_as(
                "INSERT INTO core.satellite (id, identifier, name, registered_at)
                 VALUES ($1, $2, $3, $4)
                 RETURNING id, identifier, name, builtin, registered_at, removed_at",
            )
            .bind(Uuid::new_v4())
            .bind(&req.identifier)
            .bind(name)
            .bind(app.now())
            .fetch_one(&mut *tx)
            .await,
        ),
    };
    let row = match row {
        Ok(row) => row,
        Err(e) => {
            return Err(match conflict_of(&e) {
                Some(kind) => Refusal::Kind(StatusCode::CONFLICT, kind),
                None => internal("satellite.register", e),
            })
        }
    };
    tx.commit()
        .await
        .map_err(|e| internal("satellite.register", e))?;
    Ok((status, Json(out(row, Vec::new()))))
}

/// 外す。最初の 2 つと外した衛星は 409。資格情報は全部、同じトランザクションで取り消す。
#[utoipa::path(delete, path = "/satellites/{id}",
    params(("id" = Uuid, Path, description = "衛星の番号")),
    responses((status = 204), (status = 404), (status = 409, body = SatelliteErrorBody),
              (status = 401), (status = 403), (status = 500)))]
pub async fn satellites_delete(
    State(app): State<App>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, Refusal> {
    web_session_only(&app, &headers).await?;
    let mut tx = app
        .pool
        .begin()
        .await
        .map_err(|e| internal("satellite.remove", e))?;
    let row: Option<(bool, Option<DateTime<Utc>>)> =
        sqlx::query_as("SELECT builtin, removed_at FROM core.satellite WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| internal("satellite.remove", e))?;
    match row {
        None => return Err(Refusal::NotFound),
        Some((true, _)) => {
            return Err(Refusal::Kind(StatusCode::CONFLICT, SatelliteError::Builtin))
        }
        Some((false, Some(_))) => {
            return Err(Refusal::Kind(StatusCode::CONFLICT, SatelliteError::Removed))
        }
        Some((false, None)) => {}
    }
    let now = app.now();
    sqlx::query("UPDATE core.satellite SET removed_at = $2 WHERE id = $1")
        .bind(id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| internal("satellite.remove", e))?;
    sqlx::query(
        "UPDATE core.satellite_credential SET revoked_at = $2
          WHERE satellite_id = $1 AND revoked_at IS NULL",
    )
    .bind(id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| internal("satellite.remove", e))?;
    tx.commit()
        .await
        .map_err(|e| internal("satellite.remove", e))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct IssueRequest {
    #[serde(default)]
    pub label: String,
}

/// 発行の応答。`value` はこの 1 回だけ載る（D6）。
#[derive(Serialize, utoipa::ToSchema)]
pub struct IssuedCredential {
    pub id: Uuid,
    pub label: String,
    pub issued_at: DateTime<Utc>,
    pub value: String,
}

/// 値を書き出さない（ログに出さない。D6）。
impl std::fmt::Debug for IssuedCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IssuedCredential")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// 資格情報を発行する。値は `ashsat_<識別子>_<乱数 43 文字>`、保存は値全体の SHA-256 だけ（D6）。
#[utoipa::path(post, path = "/satellites/{id}/credentials", request_body = IssueRequest,
    params(("id" = Uuid, Path, description = "衛星の番号")),
    responses((status = 201, body = IssuedCredential), (status = 400, body = SatelliteErrorBody),
              (status = 404), (status = 409, body = SatelliteErrorBody),
              (status = 401), (status = 403), (status = 500)))]
pub async fn credentials_post(
    State(app): State<App>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<IssueRequest>,
) -> Result<(StatusCode, Json<IssuedCredential>), Refusal> {
    web_session_only(&app, &headers).await?;
    if !satellite::check_credential_label(&req.label) {
        return Err(Refusal::Kind(
            StatusCode::BAD_REQUEST,
            SatelliteError::NameInvalid,
        ));
    }
    let mut tx = app
        .pool
        .begin()
        .await
        .map_err(|e| internal("satellite.issue", e))?;
    let row: Option<(String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT identifier, removed_at FROM core.satellite WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| internal("satellite.issue", e))?;
    let identifier = match row {
        None => return Err(Refusal::NotFound),
        Some((_, Some(_))) => {
            return Err(Refusal::Kind(StatusCode::CONFLICT, SatelliteError::Removed))
        }
        Some((identifier, None)) => identifier,
    };
    let mut raw = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    let value = format!(
        "{}{identifier}_{}",
        satellite::CREDENTIAL_PREFIX,
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
    );
    let (credential, issued_at) = (Uuid::new_v4(), app.now());
    sqlx::query(
        "INSERT INTO core.satellite_credential (id, satellite_id, label, token_sha256, issued_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(credential)
    .bind(id)
    .bind(&req.label)
    .bind(web_session::sha256(&value))
    .bind(issued_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| internal("satellite.issue", e))?;
    tx.commit()
        .await
        .map_err(|e| internal("satellite.issue", e))?;
    Ok((
        StatusCode::CREATED,
        Json(IssuedCredential {
            id: credential,
            label: req.label,
            issued_at,
            value,
        }),
    ))
}

/// 資格情報を 1 本取り消す（行は消さず `revoked_at` を付ける。取り消し済みはそのまま 204）。
#[utoipa::path(delete, path = "/satellites/{id}/credentials/{cid}",
    params(("id" = Uuid, Path, description = "衛星の番号"),
           ("cid" = Uuid, Path, description = "資格情報の番号")),
    responses((status = 204), (status = 404), (status = 401), (status = 403), (status = 500)))]
pub async fn credentials_delete(
    State(app): State<App>,
    headers: axum::http::HeaderMap,
    Path((id, cid)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, Refusal> {
    web_session_only(&app, &headers).await?;
    let found: Option<(Uuid,)> = sqlx::query_as(
        "UPDATE core.satellite_credential SET revoked_at = COALESCE(revoked_at, $3)
          WHERE id = $2 AND satellite_id = $1 RETURNING id",
    )
    .bind(id)
    .bind(cid)
    .bind(app.now())
    .fetch_optional(&app.pool)
    .await
    .map_err(|e| internal("satellite.revoke", e))?;
    found
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or(Refusal::NotFound)
}
