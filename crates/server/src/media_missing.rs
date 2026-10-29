// SPDX-License-Identifier: AGPL-3.0-only
//! 原本の欠けの受け口（ST09 / design D11）。
//!
//! 端末が「この原本はもう送れない」と気づいたときの報告を、理由と内容ハッシュつきで `core.media_missing` に 1 行積む。
//! 受け口の形は破棄の報告（`/drops`）に揃える —— 配列でも 1 件でも受ける / 1 件ごとの結果 / 1 件も受け付けなければ 400。
//! **破棄の報告（`core.drop_report`）には入れない**: 記録は格納されていて、失ったのは原本だけだから
//! （稼働状況の「破棄された期間」と破棄の件数を動かさない）。冪等キーは端末が採番した `id`。
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::media::IMAGE_SOURCES;
use crate::{authorize, internal_at, App, Writer};

/// 原本の欠けの理由（D11 の 7 つ）。**文字列で受けてから検査する**（`drops::REASONS` と同じ向き）。
pub const REASONS: [&str; 7] = [
    "vanished",
    "changed",
    "unreadable",
    "too_large",
    "record_dropped",
    "record_rejected",
    "record_missing",
];

/// 端末が送る原本の欠けの報告 1 件。**位置もファイル名も持たない。**
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MediaMissingRequest {
    /// 端末が採番した識別子。冪等キー
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    /// 画像のソース（`c01-photo` / `c01-screenshot` / `c01-image`）。動画は原本を置かないので欠けも無い
    pub logical_source: String,
    /// どの端末が気づいたか。空なら断る
    #[serde(default)]
    pub device_id: Option<String>,
    /// 原本の内容ハッシュ（小文字 16 進 64 桁）
    pub sha256: String,
    /// `vanished` / `changed` / `unreadable` / `too_large` / `record_dropped` / `record_rejected` / `record_missing`
    #[serde(default)]
    pub reason: Option<String>,
    /// 端末が欠けに気づいた時刻
    pub noticed_at: chrono::DateTime<chrono::Utc>,
    /// 作った収集アプリの版（任意。ST49 / D8）。形は検査しない・空は不明。冪等の判定には使わない（D11）
    #[serde(default)]
    pub client_version: Option<String>,
}

/// 報告を断った理由。**受け取った値は載せない。**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MediaMissingError {
    /// 項目が形として解釈できない
    Malformed,
    /// 画像のソースではない
    UnknownSource,
    /// 内容ハッシュが小文字 16 進 64 桁ではない
    InvalidSha256,
    /// 理由が無い、または 7 つのどれでもない
    InvalidReason,
    /// 端末識別子が無い
    MissingDevice,
    /// 同じ識別子で中身の違う報告が既にある（端末が識別子を使い回した）
    IdReused,
}

/// 送った 1 件ごとの結果。欄は `DropResult` と同じ（端末は同じ型で読む）。
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MediaMissingResult {
    pub id: Option<uuid::Uuid>,
    /// 既に同じ 1 件があったか
    pub duplicate: bool,
    /// 未送信から取り除いてよいか
    pub accepted: bool,
    pub error: Option<MediaMissingError>,
}

fn rejected(id: Option<uuid::Uuid>, error: MediaMissingError) -> MediaMissingResult {
    MediaMissingResult {
        id,
        duplicate: false,
        accepted: false,
        error: Some(error),
    }
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl MediaMissingRequest {
    /// 格納の前に断るものを 1 か所で見る。
    pub fn validate(&self) -> Result<(), MediaMissingError> {
        if !IMAGE_SOURCES.contains(&self.logical_source.as_str()) {
            return Err(MediaMissingError::UnknownSource);
        }
        if !is_sha256(&self.sha256) {
            return Err(MediaMissingError::InvalidSha256);
        }
        if !self.reason.as_deref().is_some_and(|r| REASONS.contains(&r)) {
            return Err(MediaMissingError::InvalidReason);
        }
        if self
            .device_id
            .as_deref()
            .is_none_or(|d| d.trim().is_empty())
        {
            return Err(MediaMissingError::MissingDevice);
        }
        Ok(())
    }
}

async fn missing_one(
    app: &App,
    item: &serde_json::Value,
    writer: Writer,
) -> Result<MediaMissingResult, (StatusCode, String)> {
    let sent_id = item
        .get("id")
        .and_then(|v| v.as_str())
        .and_then(|v| uuid::Uuid::parse_str(v).ok());
    let Ok(req) = serde_json::from_value::<MediaMissingRequest>(item.clone()) else {
        // **元のエラーを載せない。** serde の文言は受け取った値を含むことがある
        tracing::warn!(
            kind = "media_missing_malformed",
            "解釈できない原本の欠けの報告を断った"
        );
        return Ok(rejected(sent_id, MediaMissingError::Malformed));
    };
    if let Err(error) = req.validate() {
        // log-ok: 値を持たない enum（種別の名前だけが出る）
        tracing::warn!(kind = "media_missing_invalid", error = ?error, "原本の欠けの報告を断った");
        return Ok(rejected(Some(req.id), error));
    }

    let collector_version = writer.collector_version(req.component_version());
    let inserted: Option<(uuid::Uuid,)> = sqlx::query_as(
        "INSERT INTO core.media_missing
           (id, user_id, device_id, logical_source, sha256, reason, noticed_at,
            collector_version, server_version)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
         ON CONFLICT (id) DO NOTHING
         RETURNING id",
    )
    .bind(req.id)
    .bind(req.user_id)
    .bind(&req.device_id)
    .bind(&req.logical_source)
    .bind(&req.sha256)
    .bind(&req.reason)
    .bind(req.noticed_at)
    .bind(collector_version)
    .bind(crate::BUILD_STAMP)
    .fetch_optional(&app.pool)
    .await
    .map_err(|e| internal_at("media_missing.insert", e))?;

    if inserted.is_none() {
        // 同じ識別子で同じ中身なら再送。中身が違えば端末が識別子を使い回している —— 1 件ごとに断る
        // （500 にすると端末は一括ごと一時的な失敗と読み、後ろの報告が届かなくなる）
        let same: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM core.media_missing
              WHERE id = $1 AND user_id = $2 AND logical_source = $3 AND sha256 = $4 AND reason = $5",
        )
        .bind(req.id)
        .bind(req.user_id)
        .bind(&req.logical_source)
        .bind(&req.sha256)
        .bind(&req.reason)
        .fetch_optional(&app.pool)
        .await
        .map_err(|e| internal_at("media_missing.duplicate_lookup", e))?;
        if same.is_none() {
            tracing::warn!(
                kind = "media_missing_id_reused",
                "別の報告に使われた識別子の原本の欠けの報告を断った"
            );
            return Ok(rejected(Some(req.id), MediaMissingError::IdReused));
        }
    }
    Ok(MediaMissingResult {
        id: Some(req.id),
        duplicate: inserted.is_none(),
        accepted: true,
        error: None,
    })
}

/// 原本の欠けの報告をまとめて受け取る（design D11）。400 は「1 件も受け付けなかった」ことを意味する。
#[utoipa::path(post, path = "/media/missing", request_body = Vec<MediaMissingRequest>,
    responses((status = 200, body = Vec<MediaMissingResult>), (status = 400, body = Vec<MediaMissingResult>),
              (status = 401)))]
pub async fn missing_post(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<Vec<MediaMissingResult>>), (StatusCode, String)> {
    let writer = Writer::of(authorize(&app, &headers).await?);
    let items: Vec<serde_json::Value> = match body {
        serde_json::Value::Array(a) => a,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => return Ok((StatusCode::BAD_REQUEST, Json(Vec::new()))),
    };
    if items.is_empty() {
        return Ok((StatusCode::BAD_REQUEST, Json(Vec::new())));
    }
    let mut results = Vec::with_capacity(items.len());
    for item in &items {
        results.push(missing_one(&app, item, writer).await?);
    }
    let code = if results.iter().any(|r| r.accepted) {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    // 件数だけを出す（ハッシュ・ファイル名は出さない。D21）
    tracing::info!(
        kind = "media_missing",
        sent = items.len(),
        accepted = results.iter().filter(|r| r.accepted).count(),
        "原本の欠けの報告"
    );
    Ok((code, Json(results)))
}

impl MediaMissingRequest {
    /// 格納する収集アプリの版（`IngestRequest::component_version` と同じ扱い。D8）。
    fn component_version(&self) -> Option<String> {
        crate::ingest::normalize_component_version(self.client_version.as_deref())
    }
}
