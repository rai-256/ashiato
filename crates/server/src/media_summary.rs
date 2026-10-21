// SPDX-License-Identifier: AGPL-3.0-only
//! 写真と画像の量と原本の進みの読み出し（ST09 / design D12。`GET /media/summary`）。
//!
//! 記録と 2 つの表（`core.media_original` / `core.media_missing`）から数え直す。
//! **記録の値（位置・ファイル名・原文・ハッシュ）を載せない。**
use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::media::{IMAGE_SOURCES, VIDEO_SOURCE};
use crate::{authorize, internal_at, App};

/// `?user_id=<uuid>`。
#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    user_id: Option<uuid::Uuid>,
}

/// 件数とバイト。
#[derive(Debug, Clone, Copy, Default, Serialize, utoipa::ToSchema)]
pub struct CountBytes {
    pub count: i64,
    pub bytes: i64,
}

/// 原本の欠けの、理由ごとの件数。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct MissingByReason {
    pub reason: String,
    pub count: i64,
}

/// ソース 1 つの量。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct MediaSourceSummary {
    pub logical_source: String,
    /// 生きている `kind = media` の記録の件数
    pub records: i64,
    /// その `size_bytes` の合計
    pub record_bytes: i64,
    /// D-02 に置いた原本（画像のソースだけ）
    pub originals_stored: Option<CountBytes>,
    /// 記録はあるが、受け取りの行も欠けも無い件数（画像のソースだけ）
    pub originals_waiting: Option<i64>,
    /// 原本の欠けの理由ごとの件数（画像のソースだけ。件数のある理由だけ）
    pub missing: Vec<MissingByReason>,
}

/// D-02 に書けなかった直近の失敗。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StorageErrorView {
    pub kind: String,
    pub at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MediaSummary {
    pub sources: Vec<MediaSourceSummary>,
    /// S-01 の実行中だけ持つ。起動し直すと消える（失敗そのものはログに残る）
    pub last_storage_error: Option<StorageErrorView>,
}

/// 写真と画像の量と原本の進みを返す。
#[utoipa::path(get, path = "/media/summary",
    params(("user_id" = uuid::Uuid, Query, description = "利用者")),
    responses((status = 200, body = MediaSummary), (status = 400), (status = 401)))]
pub async fn summary_get(
    State(app): State<App>,
    Query(q): Query<SummaryQuery>,
    headers: HeaderMap,
) -> Result<Json<MediaSummary>, (StatusCode, String)> {
    authorize(&app, &headers).await?;
    let user = q
        .user_id
        .ok_or((StatusCode::BAD_REQUEST, "user_id is required".to_string()))?;

    let records: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT logical_source, count(*),
                coalesce(sum(CASE WHEN payload->>'size_bytes' ~ '^[0-9]{1,15}$'
                                  THEN (payload->>'size_bytes')::bigint END), 0)::bigint
           FROM core.event
          WHERE user_id = $1 AND deleted_at IS NULL AND payload->>'kind' = 'media'
            AND logical_source = ANY($2)
          GROUP BY logical_source",
    )
    .bind(user)
    .bind(all_sources())
    .fetch_all(&app.pool)
    .await
    .map_err(|e| internal_at("media_summary.records", e))?;

    let stored: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT logical_source, count(*), coalesce(sum(size_bytes), 0)::bigint
           FROM core.media_original WHERE user_id = $1 GROUP BY logical_source",
    )
    .bind(user)
    .fetch_all(&app.pool)
    .await
    .map_err(|e| internal_at("media_summary.stored", e))?;

    let waiting: Vec<(String, i64)> = sqlx::query_as(
        "SELECT e.logical_source, count(*)
           FROM core.event e
          WHERE e.user_id = $1 AND e.deleted_at IS NULL AND e.payload->>'kind' = 'media'
            AND e.logical_source = ANY($2)
            AND NOT EXISTS (SELECT 1 FROM core.media_original o
                             WHERE o.user_id = e.user_id AND o.sha256 = e.payload->>'sha256')
            AND NOT EXISTS (SELECT 1 FROM core.media_missing m
                             WHERE m.user_id = e.user_id AND m.sha256 = e.payload->>'sha256')
          GROUP BY e.logical_source",
    )
    .bind(user)
    .bind(IMAGE_SOURCES.to_vec())
    .fetch_all(&app.pool)
    .await
    .map_err(|e| internal_at("media_summary.waiting", e))?;

    let missing: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT logical_source, reason, count(*)
           FROM core.media_missing WHERE user_id = $1
          GROUP BY logical_source, reason ORDER BY reason",
    )
    .bind(user)
    .fetch_all(&app.pool)
    .await
    .map_err(|e| internal_at("media_summary.missing", e))?;

    let records: BTreeMap<_, _> = records.into_iter().map(|(s, n, b)| (s, (n, b))).collect();
    let stored: BTreeMap<_, _> = stored.into_iter().map(|(s, n, b)| (s, (n, b))).collect();
    let waiting: BTreeMap<_, _> = waiting.into_iter().collect();

    let sources = all_sources()
        .into_iter()
        .map(|s| {
            let (n, b) = records.get(s).copied().unwrap_or((0, 0));
            let image = IMAGE_SOURCES.contains(&s);
            MediaSourceSummary {
                logical_source: s.to_string(),
                records: n,
                record_bytes: b,
                originals_stored: image.then(|| {
                    let (count, bytes) = stored.get(s).copied().unwrap_or((0, 0));
                    CountBytes { count, bytes }
                }),
                originals_waiting: image.then(|| waiting.get(s).copied().unwrap_or(0)),
                missing: missing
                    .iter()
                    .filter(|(src, _, _)| src == s)
                    .map(|(_, reason, count)| MissingByReason {
                        reason: reason.clone(),
                        count: *count,
                    })
                    .collect(),
            }
        })
        .collect();

    Ok(Json(MediaSummary {
        sources,
        last_storage_error: app.media.last_storage_error().map(|e| StorageErrorView {
            kind: e.kind.to_string(),
            at: e.at,
        }),
    }))
}

fn all_sources() -> Vec<&'static str> {
    IMAGE_SOURCES
        .iter()
        .copied()
        .chain(std::iter::once(VIDEO_SOURCE))
        .collect()
}
