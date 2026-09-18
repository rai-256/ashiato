// SPDX-License-Identifier: AGPL-3.0-only
//! 走査と分けた、順番を保つ単一の書庫読み手（ST12 / D1）。

use super::scan::ScanCandidate;

/// 分類済みの書庫ファイルを、既存の格納関門へ渡せる要求へ変える。
/// ここでだけ書庫の由来を payload に足し、原文は項目そのものを保つ。
pub fn requests_for_file(
    kind: super::classify::KnownKind,
    inner_path: &str,
    bytes: &[u8],
    user_id: uuid::Uuid,
    archive_sha256: String,
) -> anyhow::Result<Vec<crate::IngestRequest>> {
    if kind == super::classify::KnownKind::Timeline {
        let root: serde_json::Value = serde_json::from_slice(bytes)?;
        let mut rows: Vec<(&str, &serde_json::Value)> = Vec::new();
        for segment in root
            .get("semanticSegments")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(value) = segment.get("visit") {
                rows.push(("c03-timeline-visit", value));
            }
            if let Some(value) = segment.get("activity") {
                rows.push(("c03-timeline-move", value));
            }
            for value in segment
                .get("timelinePath")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                rows.push(("c03-timeline-route", value));
            }
        }
        for value in root
            .get("rawSignals")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            rows.push(("c03-timeline-signal", value));
        }
        return rows
            .into_iter()
            .map(|(source, value)| {
                request(
                    source.to_owned(),
                    value,
                    inner_path,
                    user_id,
                    archive_sha256.clone(),
                )
            })
            .collect();
    }
    let values: Vec<serde_json::Value> = match kind {
        super::classify::KnownKind::ChromeHistory => {
            serde_json::from_slice::<serde_json::Value>(bytes)?
                .get("Browser History")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
        }
        _ => serde_json::from_slice(bytes)?,
    };
    let mut requests = Vec::new();
    for value in values {
        let (logical_source, event_time) = match kind {
            super::classify::KnownKind::YouTubeWatch
            | super::classify::KnownKind::YouTubeSearch => {
                let url = value
                    .get("titleUrl")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let source = if url.contains("watch?v=") {
                    "c03-youtube-watch"
                } else {
                    "c03-youtube-search"
                };
                (source.to_owned(), event_time(&value)?)
            }
            super::classify::KnownKind::MyActivity => {
                let product = value
                    .get("products")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|v| v.first())
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                (super::myactivity::source_name(product), event_time(&value)?)
            }
            super::classify::KnownKind::ChromeHistory => {
                let time = value
                    .get("time_usec")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| anyhow::anyhow!("Chrome時刻が無い"))?;
                (
                    "c03-chrome-history".to_owned(),
                    super::chrome::time_usec_to_utc(time)?,
                )
            }
            _ => continue,
        };
        requests.push(request_at(
            logical_source,
            &value,
            event_time,
            inner_path,
            user_id,
            archive_sha256.clone(),
        )?);
    }
    Ok(requests)
}

fn request(
    source: String,
    value: &serde_json::Value,
    inner_path: &str,
    user_id: uuid::Uuid,
    archive_sha256: String,
) -> anyhow::Result<crate::IngestRequest> {
    request_at(
        source,
        value,
        event_time(value)?,
        inner_path,
        user_id,
        archive_sha256,
    )
}

fn request_at(
    source: String,
    value: &serde_json::Value,
    event_time: chrono::DateTime<chrono::Utc>,
    inner_path: &str,
    user_id: uuid::Uuid,
    archive_sha256: String,
) -> anyhow::Result<crate::IngestRequest> {
    Ok(crate::IngestRequest {
        id: uuid::Uuid::new_v4(),
        user_id,
        logical_source: source,
        external_id: None,
        device_id: Some("s01-c03".into()),
        origin: "collected".into(),
        event_time,
        tz_offset_min: 0,
        tz_id: "UTC".into(),
        schema_version: 1,
        unit_system: None,
        crs: None,
        source_updated_at: None,
        external_ref: None,
        raw: serde_json::to_string(value)?,
        payload: serde_json::json!({"archive_sha256": archive_sha256, "inner_path": inner_path}),
    })
}

fn event_time(value: &serde_json::Value) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    let text = value
        .get("time")
        .or_else(|| value.get("timestamp"))
        .or_else(|| value.get("startTime"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("書庫項目の時刻が無い"))?;
    Ok(chrono::DateTime::parse_from_rfc3339(text)?.to_utc())
}

/// 解析前に、書庫を開いて既知・未読・読めない中身を数える結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inspection {
    pub known: usize,
    pub skipped: usize,
    pub unreadable: usize,
}

/// 読み手が処理する候補を、値をログへ出さずに検査する。
pub fn inspect(candidate: ScanCandidate) -> Result<Inspection, super::open::OpenError> {
    let files = super::open::open_archive(&candidate.path)?;
    let classified = super::classify::classify_files(&files);
    Ok(Inspection {
        known: classified.known.len(),
        skipped: classified.skipped,
        unreadable: classified.unreadable,
    })
}

/// 送られた順に 1 冊ずつ読み終える。走査側は別taskでこの送信側を保持する。
pub async fn read_in_order<F, Fut>(
    mut receiver: tokio::sync::mpsc::Receiver<ScanCandidate>,
    mut read: F,
) where
    F: FnMut(ScanCandidate) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    while let Some(candidate) = receiver.recv().await {
        read(candidate).await;
    }
}

/// 走査と直列読み手を背景で起こす。利用者が未設定なら呼び出し側は起こさない。
pub fn spawn_inspecting(
    pool: sqlx::PgPool,
    config: super::config::ArchiveConfig,
    user_id: uuid::Uuid,
) {
    let (sender, receiver) = tokio::sync::mpsc::channel(32);
    let scan_pool = pool.clone();
    tokio::spawn(async move {
        loop {
            match super::scan::scan_once(&scan_pool, &config, user_id).await {
                Ok(candidates) => {
                    for candidate in candidates {
                        if sender.send(candidate).await.is_err() {
                            return;
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(kind = "archive_scan", error = %error, "書庫の置き場を走査できない")
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(config.scan_sec)).await;
        }
    });
    tokio::spawn(read_in_order(receiver, move |candidate| {
        let pool = pool.clone();
        async move {
            let sha256 = candidate.sha256.clone();
            let inbox_kind = if candidate.from_downloads {
                "downloads"
            } else {
                "inbox"
            };
            match inspect(candidate.clone()) {
                Ok(result) => {
                    // 台帳を書く前に、読める項目を既存の格納関門へ通す。途中で DB が落ちた
                    // 場合は台帳を成功として残さず、次の走査で読み直せるようにする。
                    let candidate_for_read = ScanCandidate {
                        path: candidate.path.clone(),
                        from_downloads: candidate.from_downloads,
                        sha256: sha256.clone(),
                        disposition: candidate.disposition,
                    };
                    let files = match super::open::open_archive(&candidate_for_read.path) {
                        Ok(files) => files,
                        Err(error) => {
                            tracing::warn!(kind = error.kind(), "書庫を開けない");
                            return;
                        }
                    };
                    let classified = super::classify::classify_files(&files);
                    for known in classified.known {
                        let file = &files[known.index];
                        let requests = match requests_for_file(
                            known.kind,
                            &file.path,
                            &file.bytes,
                            user_id,
                            sha256.clone(),
                        ) {
                            Ok(requests) => requests,
                            Err(_) => continue,
                        };
                        for request in requests {
                            if let Err(_) = crate::store_one(&pool, request).await {
                                tracing::warn!(kind = "archive_store", "書庫の格納に失敗した");
                                return;
                            }
                        }
                    }
                    if let Err(error) = sqlx::query(
                    "INSERT INTO core.archive_ledger
                       (user_id, sha256, parser_version, outcome, inbox_kind, unreadable_count, skipped_file_count)
                     VALUES ($1, $2, $3, 'read', $4, $5, $6)
                     ON CONFLICT DO NOTHING",
                )
                .bind(user_id)
                .bind(sha256)
                .bind(super::PARSER_VERSION)
                .bind(inbox_kind)
                .bind(i32::try_from(result.unreadable).unwrap_or(i32::MAX))
                .bind(i32::try_from(result.skipped).unwrap_or(i32::MAX))
                .execute(&pool)
                .await
                {
                    tracing::warn!(kind = "archive_ledger", error = %error, "書庫の台帳を残せない");
                } else {
                    tracing::info!(
                        kind = "archive_inspect",
                        known = result.known,
                        skipped = result.skipped,
                        unreadable = result.unreadable,
                        "書庫を検査した"
                    );
                }
                }
                Err(error) => tracing::warn!(kind = error.kind(), "書庫を開けない"),
            }
        }
    }));
}
