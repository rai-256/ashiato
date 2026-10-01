// SPDX-License-Identifier: AGPL-3.0-only
//! 場所の器（ST21 / design D1）。器は記録ではなく、記録を束ねる識別子。
//!
//! 識別子は画面が乱数で決めて渡す。サーバは名前・座標から計算しない（C1）。
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// `POST /places` の本文。
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct PlaceCreateRequest {
    pub id: Uuid,
    /// 省けば nil UUID（ほかの口と同じ。ST29 まで利用者は名乗り）
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// 器を作った結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct PlaceCreated {
    pub id: Uuid,
}

/// 器を断った理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlaceError {
    /// その識別子はほかの利用者の器が持っている
    PlaceIdTaken,
    /// 資格情報・DB の失敗（本文の形を 400 と揃えるための値。状態符号は 401 / 500）
    Unavailable,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PlaceErrorBody {
    pub error: PlaceError,
}

/// 器を作る。同じ利用者の同じ識別子は何度でも `Ok`（押し直しで 2 つにしない）。
/// 別の利用者の行がある識別子は `Err(PlaceIdTaken)`。
pub async fn create_place(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
) -> sqlx::Result<Result<PlaceCreated, PlaceError>> {
    sqlx::query("INSERT INTO core.place (id, user_id) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    let (owner,): (Uuid,) = sqlx::query_as("SELECT user_id FROM core.place WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(if owner == user_id {
        Ok(PlaceCreated { id })
    } else {
        Err(PlaceError::PlaceIdTaken)
    })
}
