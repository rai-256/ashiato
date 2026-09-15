// SPDX-License-Identifier: AGPL-3.0-only
//! 端末からの破棄の報告（ST04 / FR-9 / FR-33。design D4 / D7）。
//!
//! **「バッファから破棄されたのか」の唯一の証拠**（扉 #14）。端末は捨てた時点を過ぎると
//! その記録について何も持たないので、どの端末が・なぜ・どの時間に何件捨てたかは、
//! 受け手が行として持つしかない（deep.md C8 / C12）。
//!
//! 受け口の形は生存信号（`/heartbeat`）に揃える —— 配列でも 1 件でも受ける / 1 件ごとの結果 /
//! 1 件も受け付けなければ 400 / 応答に受け取った値を含めない。
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{authorize, internal_at, App};

/// 破棄の理由（spec の 4 つ）。**文字列で受けてから検査する** ——
/// 型で受けると知らない理由が「形として読めない」に畳まれ、端末のログから区別できない。
pub const REASONS: [&str; 4] = ["age", "bytes", "write_failed", "unreadable"];

/// 範囲と時間ごとの件数を**持たなくてよい**理由（spec）。
/// 出来事の時刻が分からない破棄 —— 読めなかった行と、数えきれなかった書けなかった記録。
const RANGELESS_REASONS: [&str; 2] = ["write_failed", "unreadable"];

/// 出来事の時刻の 1 時間（UTC）ごとの件数（C12）。
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DropHour {
    /// 時間の始まり（UTC の正時）
    pub hour: chrono::DateTime<chrono::Utc>,
    pub count: i64,
}

/// 端末が送る破棄の報告 1 件（design D4）。**位置の値も原文の中身も持たない。**
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DropReportRequest {
    /// 端末が振った識別子。**冪等キーには混ぜない**
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub logical_source: String,
    /// どの端末が捨てたか。空なら断る
    #[serde(default)]
    pub device_id: Option<String>,
    /// `age` / `bytes` / `write_failed` / `unreadable`
    #[serde(default)]
    pub reason: Option<String>,
    /// 端末が報告を作った時刻
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最初に捨てた記録の出来事の時刻（含む）
    #[serde(default)]
    pub range_start: Option<chrono::DateTime<chrono::Utc>>,
    /// 範囲の終わり（含まない）
    #[serde(default)]
    pub range_end: Option<chrono::DateTime<chrono::Utc>>,
    pub count: i64,
    #[serde(default)]
    pub hourly: Vec<DropHour>,
    /// 端末が組んだ原文。**素通しで残す**（`text`。0003 と同じ理由）
    pub raw: String,
}

/// 受け取り時に断る理由。**アプリ層で閉じる**（`heartbeat::Invalid` と同じ向き）——
/// DB の制約に任せると 500 になり、端末は断られた報告を永久に送り直す（C2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    /// 原文が空、または DB に格納できないバイトを含む
    Raw,
    /// 理由が無い、または 4 つのどれでもない
    Reason,
    /// 端末識別子が無い
    Device,
    /// 件数が 0 以下、または格納できない大きさ
    Count,
    /// 範囲が空・逆順・片側だけ、またはこの理由で範囲を欠く
    Range,
    /// 時間ごとの件数が合わない・範囲の外・正時でない・重複・0 以下
    Hourly,
}

impl DropReportRequest {
    /// 格納の前に断るものを 1 か所で見る（spec の拒否の条件）。
    ///
    /// **`raw` と欄の食い違いは見ない**（`heartbeat.rs` と同じ。欄を集計に使い、`raw` は証拠）。
    pub fn validate(&self) -> Result<(), Invalid> {
        if self.raw.is_empty() || self.raw.contains('\0') {
            return Err(Invalid::Raw);
        }
        let reason = match self.reason.as_deref() {
            Some(r) if REASONS.contains(&r) => r,
            _ => return Err(Invalid::Reason),
        };
        if self
            .device_id
            .as_deref()
            .is_none_or(|d| d.trim().is_empty())
        {
            return Err(Invalid::Device);
        }
        if self.count <= 0 || self.count > i64::from(i32::MAX) {
            return Err(Invalid::Count);
        }
        match (self.range_start, self.range_end) {
            (None, None) => {
                // 範囲を持たない形は、出来事の時刻が分からない理由だけに許す。
                // 時間ごとの件数だけを持つ形も許さない（両方持つか、両方持たない）
                if !RANGELESS_REASONS.contains(&reason) {
                    return Err(Invalid::Range);
                }
                if !self.hourly.is_empty() {
                    return Err(Invalid::Hourly);
                }
                Ok(())
            }
            (Some(start), Some(end)) => {
                if start >= end {
                    return Err(Invalid::Range);
                }
                self.validate_hourly(start, end)
            }
            _ => Err(Invalid::Range),
        }
    }

    fn validate_hourly(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), Invalid> {
        if self.hourly.is_empty() {
            return Err(Invalid::Hourly);
        }
        let mut seen = std::collections::HashSet::new();
        let mut sum: i64 = 0;
        for h in &self.hourly {
            // 正時でない時間は、日への割り当て（時間の始まりの日）が端末と受け手でずれうる
            if h.hour.timestamp().rem_euclid(3600) != 0 || h.hour.timestamp_subsec_nanos() != 0 {
                return Err(Invalid::Hourly);
            }
            if h.count <= 0 || !seen.insert(h.hour) {
                return Err(Invalid::Hourly);
            }
            // 時間 [hour, hour + 1h) が範囲 [start, end) と重ならなければ範囲の外
            let hour_end = h.hour + chrono::Duration::hours(1);
            if !(h.hour < end && hour_end > start) {
                return Err(Invalid::Hourly);
            }
            sum = sum.saturating_add(h.count);
        }
        if sum != self.count {
            return Err(Invalid::Hourly);
        }
        Ok(())
    }
}

/// 冪等キー。**ソースと原文だけから作る**（design D7）。
///
/// 端末は凍結した報告を同じ原文で送り直すので、原文が同じなら同じ鍵になる。
/// 端末が振った `id` を混ぜると、再送のたびに別物になって件数が二重に数えられる（R2）。
pub fn content_hash(req: &DropReportRequest) -> String {
    use sha2::{Digest as _, Sha256};
    let mut h = Sha256::new();
    let mut field = |bytes: &[u8]| {
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    };
    field(req.logical_source.as_bytes());
    field(req.raw.as_bytes());
    format!("{:x}", h.finalize())
}

// ------------------------------------------------------------------ 受け口

/// 破棄の報告を断った理由。**受け取った値は載せない**（`HeartbeatError` と同じ向き）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DropError {
    /// 項目が形として解釈できない
    Malformed,
    /// 登録簿に無い論理ソース
    UnknownSource,
    /// 原文が空、または DB に格納できないバイトを含む
    InvalidRaw,
    /// 理由が無い、または 4 つのどれでもない
    InvalidReason,
    /// 端末識別子が無い
    MissingDevice,
    /// 件数が 0 以下
    InvalidCount,
    /// 範囲が空・逆順・片側だけ、またはこの理由で範囲を欠く
    InvalidRange,
    /// 時間ごとの件数が合わない・範囲の外にある
    InvalidHourly,
}

impl From<Invalid> for DropError {
    fn from(v: Invalid) -> Self {
        match v {
            Invalid::Raw => Self::InvalidRaw,
            Invalid::Reason => Self::InvalidReason,
            Invalid::Device => Self::MissingDevice,
            Invalid::Count => Self::InvalidCount,
            Invalid::Range => Self::InvalidRange,
            Invalid::Hourly => Self::InvalidHourly,
        }
    }
}

/// 送った 1 件ごとの結果。**送った順に並ぶ。** 欄は `IngestResult` / `HeartbeatResult` と同じ ——
/// 端末は 3 つの応答を同じ型で読む（`tools/check-openapi.sh` が形の一致を見る）。
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DropResult {
    pub id: Option<uuid::Uuid>,
    /// 既に同じ 1 件があったか
    pub duplicate: bool,
    /// 未送信から取り除いてよいか
    pub accepted: bool,
    pub error: Option<DropError>,
}

fn rejected(id: Option<uuid::Uuid>, error: DropError) -> DropResult {
    DropResult {
        id,
        duplicate: false,
        accepted: false,
        error: Some(error),
    }
}

async fn drop_one(app: &App, item: &serde_json::Value) -> Result<DropResult, (StatusCode, String)> {
    let sent_id = item
        .get("id")
        .and_then(|v| v.as_str())
        .and_then(|v| uuid::Uuid::parse_str(v).ok());
    let req: DropReportRequest = match serde_json::from_value(item.clone()) {
        Ok(r) => r,
        Err(_) => {
            // **元のエラーを載せない。** serde の文言は受け取った値を含むことがある
            tracing::warn!(kind = "drop_malformed", "解釈できない破棄の報告を断った");
            return Ok(rejected(sent_id, DropError::Malformed));
        }
    };
    if let Err(invalid) = req.validate() {
        tracing::warn!(kind = "drop_invalid", error = ?invalid, "破棄の報告を断った");
        return Ok(rejected(Some(req.id), invalid.into()));
    }
    let known: Option<(String,)> =
        sqlx::query_as("SELECT logical_source FROM core.source WHERE logical_source = $1")
            .bind(&req.logical_source)
            .fetch_optional(&app.pool)
            .await
            .map_err(|e| internal_at("drops.source_lookup", e))?;
    if known.is_none() {
        return Ok(rejected(Some(req.id), DropError::UnknownSource));
    }

    let hash = content_hash(&req);
    // 報告と時間ごとの件数は**1 トランザクション**。片方だけ入ると、日の件数が 0 のまま
    // 「破棄された期間」だけが立つ（あるいはその逆）。
    let mut tx = app
        .pool
        .begin()
        .await
        .map_err(|e| internal_at("drops.begin", e))?;
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "INSERT INTO core.drop_report
           (id, user_id, logical_source, device_id, reason, range_start, range_end,
            count, created_at, content_hash, raw)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
         ON CONFLICT (user_id, logical_source, content_hash) DO NOTHING
         RETURNING id",
    )
    .bind(req.id)
    .bind(req.user_id)
    .bind(&req.logical_source)
    .bind(&req.device_id)
    .bind(&req.reason)
    .bind(req.range_start)
    .bind(req.range_end)
    .bind(req.count as i32)
    .bind(req.created_at)
    .bind(&hash)
    .bind(&req.raw)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| internal_at("drops.insert", e))?;

    if let Some((id,)) = row {
        for h in &req.hourly {
            sqlx::query(
                "INSERT INTO core.drop_report_hour (report_id, hour, count) VALUES ($1,$2,$3)",
            )
            .bind(id)
            .bind(h.hour)
            .bind(h.count as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| internal_at("drops.insert_hour", e))?;
        }
    }
    tx.commit()
        .await
        .map_err(|e| internal_at("drops.commit", e))?;

    Ok(DropResult {
        id: Some(row.map_or(req.id, |(id,)| id)),
        duplicate: row.is_none(),
        accepted: true,
        error: None,
    })
}

/// 破棄の報告をまとめて受け取る（FR-9 / design D7）。
///
/// 400 は「1 件も受け付けなかった」ことを意味する（`/ingest` / `/heartbeat` と同じ約束）。
/// **端末は断られた報告も捨てずに送り直す**（C2）ので、断るのは形の不正と登録簿に無いソースに限る。
#[utoipa::path(post, path = "/drops", request_body = Vec<DropReportRequest>,
    responses((status = 200, body = Vec<DropResult>), (status = 400, body = Vec<DropResult>),
              (status = 401)))]
pub async fn drops_post(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<Vec<DropResult>>), (StatusCode, String)> {
    authorize(&app, &headers)?;
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
        results.push(drop_one(&app, item).await?);
    }
    let code = if results.iter().any(|r| r.accepted) {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    // **件数だけを出す**（製造準備 A-2）。範囲の時刻は位置の手がかりになりうるので出さない
    tracing::info!(
        kind = "drops",
        sent = items.len(),
        accepted = results.iter().filter(|r| r.accepted).count(),
        "破棄の報告"
    );
    Ok((code, Json(results)))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn t(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s).unwrap().into()
    }

    /// 10:00〜13:00（UTC）に 60 件ずつ、計 180 件の 90 日の破棄。
    fn req() -> DropReportRequest {
        DropReportRequest {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::nil(),
            logical_source: "t".into(),
            device_id: Some("d".into()),
            reason: Some("age".into()),
            created_at: t("2026-09-14T00:00:00Z"),
            range_start: Some(t("2026-09-01T10:00:00Z")),
            range_end: Some(t("2026-09-01T13:00:00Z")),
            count: 180,
            hourly: ["10", "11", "12"]
                .iter()
                .map(|h| DropHour {
                    hour: t(&format!("2026-09-01T{h}:00:00Z")),
                    count: 60,
                })
                .collect(),
            raw: r#"{"reason":"age"}"#.into(),
        }
    }

    #[test]
    fn drop_report_validate_accepts_a_well_formed_report() {
        assert_eq!(req().validate(), Ok(()));
    }

    /// Scenario: 時間ごとの件数が合わない報告は受け付けられない
    #[test]
    fn drop_report_validate_rejects_hourly_sum_mismatch() {
        let mut r = req();
        r.hourly[2].count = 50; // 合計 170
        assert_eq!(r.validate(), Err(Invalid::Hourly));
    }

    /// Scenario: 範囲が空の報告は受け付けられない
    #[test]
    fn drop_report_validate_rejects_empty_range() {
        let mut r = req();
        r.range_end = r.range_start;
        assert_eq!(r.validate(), Err(Invalid::Range));
        // 逆順も空と同じく断る
        r.range_end = Some(t("2026-09-01T09:00:00Z"));
        assert_eq!(r.validate(), Err(Invalid::Range));
    }

    /// Scenario: 読めなかった行の報告は範囲なしで受け付けられる
    #[test]
    fn drop_report_validate_accepts_rangeless_unreadable() {
        let mut r = req();
        r.reason = Some("unreadable".into());
        r.range_start = None;
        r.range_end = None;
        r.hourly.clear();
        r.count = 2;
        assert_eq!(r.validate(), Ok(()));
        // 書けなかった記録のうち数えきれなかった分も同じ形
        r.reason = Some("write_failed".into());
        assert_eq!(r.validate(), Ok(()));
        // 範囲を持たずに時間ごとの件数だけを持つ形は断る（両方持つか、両方持たない）
        r.hourly = vec![DropHour {
            hour: t("2026-09-01T10:00:00Z"),
            count: 2,
        }];
        assert_eq!(r.validate(), Err(Invalid::Hourly));
    }

    /// Scenario: 端末識別子の無い報告は受け付けられない
    #[test]
    fn drop_report_validate_rejects_missing_device() {
        let mut r = req();
        r.device_id = None;
        assert_eq!(r.validate(), Err(Invalid::Device));
        r.device_id = Some("  ".into());
        assert_eq!(r.validate(), Err(Invalid::Device));
    }

    /// Scenario: 知らない理由の報告は受け付けられない
    #[test]
    fn drop_report_validate_rejects_unknown_reason() {
        let mut r = req();
        r.reason = Some("full".into());
        assert_eq!(r.validate(), Err(Invalid::Reason));
        r.reason = None;
        assert_eq!(r.validate(), Err(Invalid::Reason));
        for ok in REASONS {
            r.reason = Some(ok.into());
            assert_eq!(r.validate(), Ok(()), "{ok}");
        }
    }

    /// Scenario: 件数が 0 の報告は受け付けられない
    #[test]
    fn drop_report_validate_rejects_zero_count() {
        let mut r = req();
        r.count = 0;
        assert_eq!(r.validate(), Err(Invalid::Count));
        r.count = -1;
        assert_eq!(r.validate(), Err(Invalid::Count));
    }

    /// Scenario: 範囲を欠く 90 日の報告は受け付けられない
    #[test]
    fn drop_report_validate_rejects_rangeless_age() {
        let mut r = req();
        r.range_start = None;
        r.range_end = None;
        r.hourly.clear();
        assert_eq!(r.validate(), Err(Invalid::Range));
        r.reason = Some("bytes".into());
        assert_eq!(r.validate(), Err(Invalid::Range));
        // 片側だけの範囲も断る
        let mut r = req();
        r.range_end = None;
        assert_eq!(r.validate(), Err(Invalid::Range));
        // 範囲を持つのに時間ごとの件数が無い
        let mut r = req();
        r.hourly.clear();
        assert_eq!(r.validate(), Err(Invalid::Hourly));
    }

    /// Scenario: 範囲の外の時間に件数を置いた報告は受け付けられない
    #[test]
    fn drop_report_validate_rejects_hour_outside_range() {
        let mut r = req();
        r.hourly[2].hour = t("2026-09-01T15:00:00Z");
        assert_eq!(r.validate(), Err(Invalid::Hourly));
        // 終わりちょうどの時間（13 時台）は範囲 [10:00, 13:00) と重ならない
        r.hourly[2].hour = t("2026-09-01T13:00:00Z");
        assert_eq!(r.validate(), Err(Invalid::Hourly));
        // 範囲の始まりを含む時間は、始まりが正時でなくても重なる
        let mut r = req();
        r.range_start = Some(t("2026-09-01T10:30:00Z"));
        assert_eq!(r.validate(), Ok(()));
    }

    /// 正時でない時間・重複した時間は断る（格納で主キーに当たって 500 になり、永久に再送される）。
    #[test]
    fn drop_report_validate_rejects_misaligned_or_duplicate_hours() {
        let mut r = req();
        r.hourly[0].hour = t("2026-09-01T10:15:00Z");
        assert_eq!(r.validate(), Err(Invalid::Hourly));
        let mut r = req();
        r.hourly[1].hour = r.hourly[0].hour;
        assert_eq!(r.validate(), Err(Invalid::Hourly));
    }

    /// 原文が空・U+0000 入りなら断る（`heartbeat` と同じ理由）。
    #[test]
    fn drop_report_validate_rejects_empty_raw() {
        let mut r = req();
        r.raw = String::new();
        assert_eq!(r.validate(), Err(Invalid::Raw));
        r.raw = "{\"a\":\"\0\"}".into();
        assert_eq!(r.validate(), Err(Invalid::Raw));
    }

    /// 冪等キーは端末が振った id を混ぜない。原文が変われば別の鍵。
    #[test]
    fn drop_report_content_hash_ignores_collector_id() {
        let a = req();
        let mut b = req();
        b.id = uuid::Uuid::new_v4();
        assert_eq!(content_hash(&a), content_hash(&b));
        b.raw = r#"{"reason":"bytes"}"#.into();
        assert_ne!(content_hash(&a), content_hash(&b));
    }
}
