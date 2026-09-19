// SPDX-License-Identifier: AGPL-3.0-only
//! 走査と分けた、順番を保つ単一の書庫読み手（ST12 / D1）。

use super::scan::ScanCandidate;

/// Takeout の中身は本人が形を確認するまで格納しない。端末 Timeline と移行前位置は待たない。
pub fn requires_shape_confirmation(kind: super::classify::KnownKind) -> bool {
    matches!(
        kind,
        super::classify::KnownKind::YouTubeWatch
            | super::classify::KnownKind::YouTubeSearch
            | super::classify::KnownKind::MyActivity
            | super::classify::KnownKind::ChromeHistory
    )
}

/// 読み手が「いま読んでいる書庫」を置く場所（D12）。
///
/// **台帳は読み終えてから 1 回で書く**（D7）ので、読んでいる途中の状態は台帳から出ない。
/// 百万件級の書庫は読み終わるまで数分かかり、その間に画面を開いた本人には
/// 「置いたのに何も起きていない」ようにしか見えない。
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Reading {
    pub file_name: String,
    pub inner_path: String,
    pub items_read: i64,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// 読み手と `/archives/status` が共有する、読んでいる途中の状態。
///
/// **プロセスの大域に置かない** —— 大域にすると、並んで走る試験どうしが
/// 同じ状態を上書きし合う。起こす側が 1 つ作って読み手と App の両方へ渡す。
pub type ReadingState = std::sync::Arc<std::sync::RwLock<Option<Reading>>>;

/// 読んでいる途中の状態を更新する間隔（D12 の「1,000 件ごとに更新する」）。
pub const READING_STEP: usize = 1_000;

/// 読み手がその書庫から抜けたら、**どの経路でも**読んでいる途中の状態を畳む。
///
/// 読み手は失敗のたびに `return` するので、畳むのを手で書くと必ずどれか 1 本を落とす
/// —— 落ちた経路では、終わった書庫を画面が永久に「読んでいます」と出し続ける。
struct ReadingGuard(ReadingState);

impl Drop for ReadingGuard {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.0.write() {
            *slot = None;
        }
    }
}

/// 1 冊のファイルから得た要求を、順番を変えずに既存の格納関門へ渡す。
///
/// 途中の失敗は成功として畳まない。呼び出し側が台帳を追記しないことで、次の
/// 走査で同じ書庫を最初から読み直せる。
pub async fn store_requests(
    sink: &dyn crate::RecordSink,
    requests: Vec<crate::IngestRequest>,
) -> anyhow::Result<Vec<crate::StoreOutcome>> {
    store_requests_with_progress(sink, requests, &mut |_| {}).await
}

/// 格納の進みを呼び出し側へ知らせながら渡す。`progress` は **`READING_STEP` 件ごと**と
/// 最後に 1 回呼ばれる（毎件呼ぶと、読んでいる途中の状態を書く鍵の取り合いで遅くなる）。
pub async fn store_requests_with_progress(
    sink: &dyn crate::RecordSink,
    requests: Vec<crate::IngestRequest>,
    progress: &mut (dyn FnMut(usize) + Send),
) -> anyhow::Result<Vec<crate::StoreOutcome>> {
    let mut outcomes = Vec::with_capacity(requests.len());
    for request in requests {
        outcomes.push(sink.store(request).await?);
        if outcomes.len() % READING_STEP == 0 {
            progress(outcomes.len());
        }
    }
    progress(outcomes.len());
    Ok(outcomes)
}

/// 置き場の中の名前だけを取る。**フォルダのパスは台帳へ持ち込まない**（D7）。
pub fn file_name_of(path: &std::path::Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

/// 既に読んだ書庫を置き直されたとき、台帳へ 1 行だけ足す。
///
/// **走査のたびには足さない** —— 一意索引 `(user_id, sha256, parser_version, outcome)` と
/// `ON CONFLICT DO NOTHING` の組で、同じ書庫の「既に読んだ」は生涯 1 行に固定される
/// （spec「ダウンロードのフォルダに残り続ける書庫は台帳を増やさない」）。
pub async fn record_already_read(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    sha256: &str,
    file_name: Option<String>,
) -> Result<(), sqlx::Error> {
    let previous: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM core.archive_ledger
          WHERE user_id = $1 AND sha256 = $2 AND parser_version = $3
            AND outcome IN ('read', 'unreadable')
          ORDER BY finished_at DESC, id DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(sha256)
    .bind(super::PARSER_VERSION)
    .fetch_optional(pool)
    .await?;
    sqlx::query(
        "INSERT INTO core.archive_ledger
           (user_id, sha256, parser_version, outcome, file_name, already_read_ledger_id)
         VALUES ($1, $2, $3, 'already_read', $4, $5) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(sha256)
    .bind(super::PARSER_VERSION)
    .bind(file_name)
    .bind(previous)
    .execute(pool)
    .await?;
    Ok(())
}

/// 格納失敗を走査の可変な観測値へ記録する。3 回目でのみ追記台帳へ失敗を残し、
/// 以後 1 時間は同じファイルを再投入しないよう `retry_after` を置く。
pub async fn record_store_failure(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    path: &std::path::Path,
    sha256: String,
) -> Result<bool, sqlx::Error> {
    let file_name = file_name_of(path);
    let path = path.to_string_lossy();
    let failures: i32 = sqlx::query_scalar(
        "UPDATE core.archive_sighting
            SET consecutive_failures = consecutive_failures + 1,
                retry_after = CASE WHEN consecutive_failures + 1 >= 3
                                   THEN now() + interval '1 hour' ELSE NULL END
          WHERE user_id = $1 AND path = $2
          RETURNING consecutive_failures",
    )
    .bind(user_id)
    .bind(path.as_ref())
    .fetch_one(pool)
    .await?;
    if failures < 3 {
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO core.archive_ledger
           (user_id, sha256, parser_version, outcome, file_name)
         VALUES ($1, $2, $3, 'store_failed', $4) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(sha256)
    .bind(super::PARSER_VERSION)
    .bind(file_name)
    .execute(pool)
    .await?;
    Ok(true)
}

/// 読めなかった書庫を台帳へ 1 行残す（spec「読めない形の書庫は台帳に残す」）。
///
/// **黙って `return` しない。** Takeout の書き出しは約 7 日で失効するので、
/// 置いたのに何も起きないまま気づけないと**取り直せない**（R4 / R6）。
pub async fn record_unreadable(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    sha256: &str,
    path: &std::path::Path,
    unreadable_kind: &str,
    from_downloads: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO core.archive_ledger
           (user_id, sha256, parser_version, outcome, file_name, unreadable_kind, inbox_kind, created_at)
         VALUES ($1, $2, $3, 'unreadable', $4, $5, $6, $7) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(sha256)
    .bind(super::PARSER_VERSION)
    .bind(file_name_of(path))
    .bind(unreadable_kind)
    .bind(if from_downloads { "downloads" } else { "inbox" })
    .bind(archive_created_at(path, chrono::Utc::now()))
    .execute(pool)
    .await?;
    Ok(())
}

/// 読み終えた台帳行へ、論理ソースごとの格納結果を追記する。
/// 台帳は更新できないため、格納の全結果を先に畳んでから 1 行ずつ INSERT する。
pub async fn record_ledger_sources(
    pool: &sqlx::PgPool,
    ledger_id: i64,
    requests: &[crate::IngestRequest],
    outcomes: &[crate::StoreOutcome],
) -> Result<(), sqlx::Error> {
    #[derive(Default)]
    struct Counts {
        inserted: i32,
        duplicate: i32,
        deleted: i32,
        max_event_at: Option<chrono::DateTime<chrono::Utc>>,
    }
    let mut per_source: std::collections::BTreeMap<String, Counts> =
        std::collections::BTreeMap::new();
    for (request, outcome) in requests.iter().zip(outcomes) {
        let counts = per_source
            .entry(request.logical_source.clone())
            .or_default();
        counts.max_event_at = Some(
            counts
                .max_event_at
                .map_or(request.event_time, |old| old.max(request.event_time)),
        );
        match outcome {
            crate::StoreOutcome::Inserted(_) => counts.inserted += 1,
            crate::StoreOutcome::Duplicate(_) => counts.duplicate += 1,
            crate::StoreOutcome::DuplicateOfDeleted(_) => counts.deleted += 1,
            crate::StoreOutcome::Rejected(_) => {}
        }
    }
    for (logical_source, counts) in per_source {
        sqlx::query(
            "INSERT INTO core.archive_ledger_source
               (ledger_id, logical_source, inserted_count, duplicate_count, deleted_count, max_event_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(ledger_id)
        .bind(logical_source)
        .bind(counts.inserted)
        .bind(counts.duplicate)
        .bind(counts.deleted)
        .bind(counts.max_event_at)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// 読めなかった項目の場所を、台帳へ安全に残すための要約。
/// 本文・題名・URLは含めず、障害調査に必要なパスと項目位置だけを先頭100件に限る。
pub fn unreadable_summary(locations: &[String]) -> Option<String> {
    (!locations.is_empty()).then(|| {
        locations
            .iter()
            .take(100)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// 書庫名に `takeout-YYYYMMDD-HHMMSS` があればその UTC 時刻を台帳へ残す。
/// 書き出し時刻を持たない端末ファイルなどは、走査で見つけた時刻を使う。
pub fn archive_created_at(
    path: &std::path::Path,
    discovered_at: chrono::DateTime<chrono::Utc>,
) -> chrono::DateTime<chrono::Utc> {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return discovered_at;
    };
    // **本物の名前は `takeout-YYYYMMDDTHHMMSSZ-NNN`**（区切りは `T`、末尾に `Z`）。
    // `-` 区切りだけを読んでいたときは、実物がどれも見つけた時刻へ落ちていた（R8）。
    let stamp = name
        .strip_prefix("takeout-")
        .and_then(|rest| rest.get(..15));
    stamp
        .and_then(|stamp| {
            chrono::NaiveDateTime::parse_from_str(stamp, "%Y%m%dT%H%M%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(stamp, "%Y%m%d-%H%M%S"))
                .ok()
        })
        .map(|time| time.and_utc())
        .unwrap_or(discovered_at)
}

/// 1 冊のファイルを読んだ結果。**読めなかった項目は落とさず数える**。
#[derive(Debug, Default)]
pub struct FileRequests {
    pub requests: Vec<crate::IngestRequest>,
    /// 読めなかった項目の場所（`<書庫の中のパス>#<項目の位置>`）。台帳に残す。
    pub unreadable: Vec<String>,
}

/// 分類済みの書庫ファイルを、既存の格納関門へ渡せる要求へ変える。
///
/// **読めない項目 1 件でファイル全体を落とさない**（spec「1 件が読めなくても書庫の残りを読む」）。
/// 落としていたときは、Google が 1 件だけ壊れた時刻を書き出すと、その製品の
/// 全期間ぶんが黙って入らなかった。
pub fn requests_for_file(
    kind: super::classify::KnownKind,
    inner_path: &str,
    bytes: &[u8],
    user_id: uuid::Uuid,
    archive_sha256: String,
) -> anyhow::Result<Vec<crate::IngestRequest>> {
    Ok(requests_for_file_reporting(kind, inner_path, bytes, user_id, archive_sha256)?.requests)
}

/// `requests_for_file` の、読めなかった項目の場所も返す版。読み手はこちらを使う。
pub fn requests_for_file_reporting(
    kind: super::classify::KnownKind,
    inner_path: &str,
    bytes: &[u8],
    user_id: uuid::Uuid,
    archive_sha256: String,
) -> anyhow::Result<FileRequests> {
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
        let mut out = FileRequests::default();
        for (position, (source, value)) in rows.into_iter().enumerate() {
            match request(
                source.to_owned(),
                value,
                inner_path,
                user_id,
                archive_sha256.clone(),
            ) {
                Ok(request) => out.requests.push(request),
                Err(_) => out.unreadable.push(format!("{inner_path}#{position}")),
            }
        }
        return Ok(out);
    }
    if kind == super::classify::KnownKind::Records
        || kind == super::classify::KnownKind::SemanticHistory
    {
        let records = if kind == super::classify::KnownKind::Records {
            super::legacy::parse_records(bytes)?
        } else {
            super::legacy::parse_semantic(bytes)?
        };
        let mut out = FileRequests::default();
        for (position, record) in records.into_iter().enumerate() {
            match request_at(
                record.logical_source.to_owned(),
                &record.item,
                record.event_time,
                inner_path,
                user_id,
                archive_sha256.clone(),
            ) {
                Ok(request) => out.requests.push(request),
                Err(_) => out.unreadable.push(format!("{inner_path}#{position}")),
            }
        }
        return Ok(out);
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
    // **原文は書庫のバイト列の切り出し**（spec「原文は書庫のバイト列の一部と一致する」）。
    // 解釈して書き戻すと、欄の並び・空白・数値の表記が変わり、**書庫を消した後に
    // 元の 1 件を復元できない**（review R2）。切り出せない形のときだけ書き戻す。
    let sliced = match kind {
        super::classify::KnownKind::ChromeHistory => Vec::new(),
        _ => super::slice::array_items(bytes).unwrap_or_default(),
    };
    let mut out = FileRequests::default();
    for (position, value) in values.into_iter().enumerate() {
        let parsed = (|| -> anyhow::Result<Option<(String, chrono::DateTime<chrono::Utc>)>> {
            Ok(Some(match kind {
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
                _ => return Ok(None),
            }))
        })();
        let Some((logical_source, event_time)) = (match parsed {
            Ok(parsed) => parsed,
            // **この項目だけ飛ばす。** 場所を残すので、入らなかったことは台帳に出る。
            Err(_) => {
                out.unreadable.push(format!("{inner_path}#{position}"));
                continue;
            }
        }) else {
            continue;
        };
        let raw = sliced
            .get(position)
            .and_then(|item| std::str::from_utf8(item).ok())
            .map(str::to_owned);
        match request_at_raw(
            logical_source,
            &value,
            raw,
            event_time,
            inner_path,
            user_id,
            archive_sha256.clone(),
        ) {
            Ok(request) => out.requests.push(request),
            Err(_) => out.unreadable.push(format!("{inner_path}#{position}")),
        }
    }
    Ok(out)
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

/// 項目が持つ時刻の表記から、取得元が示した時差を引く（`archive::timezone`）。
///
/// Timeline は `startTimeTimezoneUtcOffsetMinutes` を明示するので、あればそれを優先する。
/// **示していなければ UTC のまま**（位置から推定しない。本人の決定 C2）。
fn source_timezone(value: &serde_json::Value) -> super::timezone::SourceTimezone {
    let offset = value
        .get("startTimeTimezoneUtcOffsetMinutes")
        .and_then(serde_json::Value::as_i64)
        .and_then(|minutes| i32::try_from(minutes).ok());
    let text = ["time", "startTime", "timestamp", "endTime"]
        .iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_str))
        .or_else(|| {
            value
                .get("duration")
                .and_then(|duration| duration.get("startTimestamp"))
                .and_then(serde_json::Value::as_str)
        });
    text.and_then(|text| super::timezone::from_timestamp(text, offset).ok())
        .unwrap_or(super::timezone::SourceTimezone {
            offset_min: 0,
            id: "UTC".into(),
            from_source: false,
        })
}

fn request_at(
    source: String,
    value: &serde_json::Value,
    event_time: chrono::DateTime<chrono::Utc>,
    inner_path: &str,
    user_id: uuid::Uuid,
    archive_sha256: String,
) -> anyhow::Result<crate::IngestRequest> {
    request_at_raw(
        source,
        value,
        None,
        event_time,
        inner_path,
        user_id,
        archive_sha256,
    )
}

/// `raw` に**書庫のバイト列の切り出し**を渡せる版。`None` なら書き戻す。
fn request_at_raw(
    source: String,
    value: &serde_json::Value,
    raw: Option<String>,
    event_time: chrono::DateTime<chrono::Utc>,
    inner_path: &str,
    user_id: uuid::Uuid,
    archive_sha256: String,
) -> anyhow::Result<crate::IngestRequest> {
    let zone = source_timezone(value);
    Ok(crate::IngestRequest {
        id: uuid::Uuid::new_v4(),
        user_id,
        logical_source: source,
        external_id: None,
        device_id: Some("s01-c03".into()),
        origin: "collected".into(),
        event_time,
        // **取得元が示した時差をそのまま残す**（C2）。0 / UTC に畳んでいたときは、
        // `+09:00` で書き出された記録がどれも「地域を持たなかった」ことになり、
        // 後から取り直せなかった（review R3）。位置からの推定はしない。
        tz_offset_min: zone.offset_min,
        tz_id: zone.id,
        schema_version: 1,
        unit_system: None,
        crs: None,
        source_updated_at: None,
        external_ref: None,
        raw: match raw {
            Some(raw) => raw,
            None => serde_json::to_string(value)?,
        },
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

/// 移行前の書き出しが運んだ最終日の翌日に、3 本の旧ソースを退役させる。
/// より古い書庫を後から読んでも退役日は戻さない。
pub async fn retire_legacy_sources(
    pool: &sqlx::PgPool,
    last_event_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    let day = (last_event_at + chrono::Duration::hours(9)).date_naive() + chrono::Duration::days(1);
    sqlx::query(
        "UPDATE core.source SET retired_on = GREATEST(COALESCE(retired_on, $1), $1)
          WHERE logical_source IN ('c03-legacy-location', 'c03-legacy-visit', 'c03-legacy-activity')",
    )
    .bind(day)
    .execute(pool)
    .await?;
    Ok(())
}

/// マイアクティビティの製品名は書庫ごとに増えるため、印を通った製品だけ登録簿へ足す。
pub async fn ensure_myactivity_source(
    pool: &sqlx::PgPool,
    logical_source: &str,
    product: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ($1, $2, 5184000, 'none') ON CONFLICT (logical_source) DO NOTHING",
    )
    .bind(logical_source)
    .bind(format!("マイアクティビティ: {product}"))
    .execute(pool)
    .await?;
    Ok(())
}

/// 専用置き場の書庫だけを、台帳の追記後に本人の「取り込み済み」へ移す。
pub fn move_to_processed(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("書庫の親が無い"))?;
    let processed = parent.join("取り込み済み");
    std::fs::create_dir_all(&processed)?;
    let original_name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("書庫名が無い"))?
        .to_owned();
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| std::io::Error::other("書庫名が読めない"))?;
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    let mut index = 1;
    loop {
        let name = if index == 1 {
            original_name.clone()
        } else if extension.is_empty() {
            format!("{stem} ({index})").into()
        } else {
            format!("{stem} ({index}).{extension}").into()
        };
        let target = processed.join(name);
        if !target.exists() {
            std::fs::rename(path, &target)?;
            return Ok(target);
        }
        index += 1;
    }
}

/// 読んだ中身を内容ハッシュ名で 1 度だけ写す。書庫そのものはここへ渡さない。
pub fn copy_known_file(
    copy_dir: &std::path::Path,
    bytes: &[u8],
) -> std::io::Result<std::path::PathBuf> {
    use sha2::Digest as _;
    let hash = format!("{:x}", sha2::Sha256::digest(bytes));
    let target = copy_dir.join(&hash[..2]).join(&hash);
    if !target.exists() {
        let parent = target.parent().expect("写しの親");
        std::fs::create_dir_all(parent)?;
        std::fs::write(&target, bytes)?;
    }
    Ok(target)
}

/// 写しを残す設定のときだけ内容ハッシュの写しを作る。
pub fn copy_if_enabled(
    keep_copies: bool,
    copy_dir: &std::path::Path,
    bytes: &[u8],
) -> std::io::Result<Option<std::path::PathBuf>> {
    keep_copies
        .then(|| copy_known_file(copy_dir, bytes))
        .transpose()
}

/// 写しの実体と台帳を同じ内容ハッシュで結ぶ。`archive_file` は追記のみなので、
/// 同じファイルを読み直しても目録を増やさない。
pub async fn record_copy(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    sha256: String,
    inner_path: &str,
    stored_path: &std::path::Path,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO core.archive_file (sha256, user_id, inner_path, stored_path)
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(sha256)
    .bind(user_id)
    .bind(inner_path)
    .bind(stored_path.to_string_lossy().as_ref())
    .execute(pool)
    .await?;
    Ok(())
}

/// 解析器版の更新時は、残っている写しを本人の置き場より優先して読み直す。
pub async fn reparse_path(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    sha256: String,
    inbox_path: &std::path::Path,
) -> Result<std::path::PathBuf, sqlx::Error> {
    let copied: Option<String> = sqlx::query_scalar(
        "SELECT stored_path FROM core.archive_file WHERE user_id = $1 AND sha256 = $2",
    )
    .bind(user_id)
    .bind(sha256)
    .fetch_optional(pool)
    .await?;
    Ok(copied
        .map(std::path::PathBuf::from)
        .filter(|path| path.exists())
        .unwrap_or_else(|| inbox_path.to_owned()))
}

/// 保存した内部ファイルをそのまま解析器へ戻すため、目録から書庫内パスとバイト列を復元する。
pub async fn copied_files_for_reparse(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<Vec<super::open::ArchiveFile>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT inner_path, stored_path FROM core.archive_file WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(path, stored)| {
            std::fs::read(stored)
                .ok()
                .map(|bytes| super::open::ArchiveFile { path, bytes })
        })
        .collect())
}

/// 確認に必要な構造だけを取り出す。記録値・題名・検索語は形に含めない。
pub fn shape_for_file(
    kind: super::classify::KnownKind,
    bytes: &[u8],
) -> anyhow::Result<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    // **製品の名前は集合**（design D16）。項目ごとに積むと、同じ製品でも件数が
    // 変われば形が変わり、**2 か月ごとの書き出しのたびに確認待ちになる**
    // （本人が第 3 回 Q12 で「ならない」と決めた型。R7）。並びにも依存させない。
    let products: Vec<String> = if kind == super::classify::KnownKind::MyActivity {
        let mut names: Vec<String> = value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row.get("products"))
            .filter_map(serde_json::Value::as_array)
            .filter_map(|products| products.first())
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect();
        names.sort();
        names.dedup();
        names
    } else {
        Vec::new()
    };
    let (top_level_keys, field_names) = match &value {
        serde_json::Value::Object(object) => {
            (object.keys().cloned().collect::<Vec<_>>(), Vec::new())
        }
        serde_json::Value::Array(rows) => {
            let mut fields = rows
                .iter()
                .filter_map(serde_json::Value::as_object)
                .flat_map(|object| object.keys().cloned())
                .collect::<Vec<_>>();
            fields.sort();
            fields.dedup();
            (Vec::new(), fields)
        }
        _ => (Vec::new(), Vec::new()),
    };
    Ok(serde_json::json!({
        "kind": format!("{kind:?}"),
        "products": products,
        "top_level_keys": top_level_keys,
        "field_names": field_names,
    }))
}

pub async fn is_shape_confirmed(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    shape_hash: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM core.archive_shape_confirmation WHERE user_id = $1 AND shape_hash = $2)",
    )
    .bind(user_id)
    .bind(shape_hash)
    .fetch_one(pool)
    .await
}

/// 未確認の形を1回だけ待ち行列へ積む。台帳の追記は読み手側がまとめて行う。
pub async fn record_pending_shape(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    archive_sha256: &str,
    inner_path: &str,
    shape: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let shape_hash = hash_shape(shape);
    sqlx::query(
        "INSERT INTO core.archive_pending_shape (user_id, sha256, inner_path, shape_hash, shape)
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(archive_sha256)
    .bind(inner_path)
    .bind(shape_hash)
    .bind(shape)
    .execute(pool)
    .await?;
    Ok(())
}

/// 印を通って格納まで終えた内部ファイルは待ち行列から外す。待ち行列は
/// 書き換え可能な観測値なので、追記台帳とは分けて消せる。
pub async fn remove_pending_shape(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    archive_sha256: &str,
    inner_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM core.archive_pending_shape
          WHERE user_id = $1 AND sha256 = $2 AND inner_path = $3",
    )
    .bind(user_id)
    .bind(archive_sha256)
    .bind(inner_path)
    .execute(pool)
    .await?;
    Ok(())
}

/// 走査のたびに回数を永続化し、その日の最初だけ取り込み器自身へ生存信号を残す。
/// 書庫の各ソースには信号を送らず、途絶は書庫記録だけから導かせる。
pub async fn record_archive_heartbeat(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    emitted_at: chrono::DateTime<chrono::Utc>,
    capturable: bool,
    blockers: Vec<String>,
) -> anyhow::Result<()> {
    let (attempts, successes): (i32, i32) = sqlx::query_as(
        "INSERT INTO core.archive_scan_counter (user_id, scanned_at, attempts, successes)
         VALUES ($1, $2, 1, CASE WHEN $3 THEN 1 ELSE 0 END)
         ON CONFLICT (user_id) DO UPDATE SET scanned_at = EXCLUDED.scanned_at,
           attempts = core.archive_scan_counter.attempts + 1,
           successes = core.archive_scan_counter.successes + CASE WHEN $3 THEN 1 ELSE 0 END
         RETURNING attempts, successes",
    )
    .bind(user_id)
    .bind(emitted_at)
    .bind(capturable)
    .fetch_one(pool)
    .await?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM core.heartbeat
          WHERE user_id = $1 AND logical_source = 's01-archive-inbox'
            AND (emitted_at AT TIME ZONE 'Asia/Tokyo')::date = ($2 AT TIME ZONE 'Asia/Tokyo')::date)",
    )
    .bind(user_id)
    .bind(emitted_at)
    .fetch_one(pool)
    .await?;
    if exists {
        return Ok(());
    }
    let raw = serde_json::json!({
        "archive_inbox": true,
        "attempts": attempts,
        "successes": successes,
        "capturable": capturable,
        "blockers": blockers,
    })
    .to_string();
    crate::store_heartbeat(
        pool,
        crate::heartbeat::HeartbeatRequest {
            id: uuid::Uuid::new_v4(),
            user_id,
            logical_source: "s01-archive-inbox".into(),
            device_id: Some("s01-c03".into()),
            emitted_at,
            capturable,
            blockers,
            attempts,
            successes,
            raw,
        },
    )
    .await?;
    sqlx::query(
        "UPDATE core.archive_scan_counter SET attempts = 0, successes = 0 WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 同じ未確認書庫は、走査回数に関わらず確認待ち台帳を 1 行だけ残す。
pub async fn record_pending_ledger(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    sha256: String,
    file_name: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome, file_name)
         VALUES ($1, $2, $3, 'pending_shape', $4) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(sha256)
    .bind(super::PARSER_VERSION)
    .bind(file_name)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn hash_shape(shape: &serde_json::Value) -> String {
    use sha2::Digest as _;
    // 確認を要するのは論理ソース名を決める種類と製品名だけ。欄の追加や
    // パスの翻訳で、既に確認した書庫まで止めない。
    let identity = serde_json::json!({
        "kind": shape.get("kind"),
        "products": shape.get("products"),
    });
    let encoded = serde_json::to_vec(&identity).expect("形はJSON");
    format!("{:x}", sha2::Sha256::digest(encoded))
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

/// 起こした取り込み器を止める手。**落とすと走査も読み手も止まる。**
///
/// 止める手が無かったときは、試験が終わっても走査が 1 秒ごとに DB を叩き続け、
/// 試験が増えるほど接続を食い合って全体が止まった（review の追試）。
/// 本番は起動から終了まで動かすので、握ったまま持つ。
#[derive(Debug)]
pub struct WorkerHandle {
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// 走査と直列読み手を背景で起こす。利用者が未設定なら呼び出し側は起こさない。
#[must_use = "落とすと取り込み器が止まる。本番は握ったまま持つ"]
pub fn spawn_inspecting(
    pool: sqlx::PgPool,
    config: super::config::ArchiveConfig,
    user_id: uuid::Uuid,
    reading: ReadingState,
) -> WorkerHandle {
    let (sender, receiver) = tokio::sync::mpsc::channel(32);
    let scan_pool = pool.clone();
    let read_config = config.clone();
    let scanning = tokio::spawn(async move {
        loop {
            match super::scan::scan_once(&scan_pool, &config, user_id).await {
                Ok(candidates) => {
                    if let Err(error) = record_archive_heartbeat(
                        &scan_pool,
                        user_id,
                        chrono::Utc::now(),
                        true,
                        Vec::new(),
                    )
                    .await
                    {
                        tracing::warn!(kind = "archive_heartbeat", error = %error, "取り込み器の生存信号を残せない");
                    }
                    for candidate in candidates {
                        if sender.send(candidate).await.is_err() {
                            return;
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(kind = "archive_scan", error = %error, "書庫の置き場を走査できない");
                    // **読めない置き場だけを挙げる**（spec「読めない置き場の種類を
                    // 満たされていないものとして残す」）。両方を決め打ちで並べていたときは、
                    // 専用のフォルダだけが読めない日にも画面が「ダウンロードのフォルダも
                    // 読めません」と出し、本人が直す場所を誤る（review R11）。
                    let mut blockers = Vec::new();
                    if std::fs::read_dir(&config.inbox_dir).is_err() {
                        blockers.push("dedicated_inbox_unreadable".to_owned());
                    }
                    if std::fs::read_dir(&config.downloads_dir).is_err() {
                        blockers.push("downloads_unreadable".to_owned());
                    }
                    if blockers.is_empty() {
                        blockers.push("inbox_scan_failed".to_owned());
                    }
                    if let Err(heartbeat_error) = record_archive_heartbeat(
                        &scan_pool,
                        user_id,
                        chrono::Utc::now(),
                        false,
                        blockers,
                    )
                    .await
                    {
                        tracing::warn!(kind = "archive_heartbeat", error = %heartbeat_error, "取り込み器の生存信号を残せない");
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(config.scan_sec)).await;
        }
    });
    let reading_task = tokio::spawn(read_in_order(receiver, move |candidate| {
        let pool = pool.clone();
        let read_config = read_config.clone();
        let reading = reading.clone();
        async move {
            let _reading_guard = ReadingGuard(reading.clone());
            // 同じ内容を別名で置き直した候補は、走査側で既読と判定済み。
            // 再び格納・台帳追記へ進むと一意制約に当たり、専用置き場にも残り続ける。
            if candidate.disposition == super::scan::ScanDisposition::AlreadyRead {
                // **ダウンロードのフォルダのファイルは動かさない**ので、走査のたびに
                // ここへ来る。台帳へ足すのは本人が置き直した専用のフォルダの側だけ
                // （spec「ダウンロードのフォルダに残り続ける書庫は台帳を増やさない」）。
                if !candidate.from_downloads {
                    let _ = record_already_read(
                        &pool,
                        user_id,
                        &candidate.sha256,
                        file_name_of(&candidate.path),
                    )
                    .await;
                    let _ = move_to_processed(&candidate.path);
                }
                return;
            }
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
                            let _ = record_unreadable(
                                &pool,
                                user_id,
                                &sha256,
                                &candidate.path,
                                error.kind(),
                                candidate.from_downloads,
                            )
                            .await;
                            if !candidate.from_downloads {
                                let _ = move_to_processed(&candidate.path);
                            }
                            return;
                        }
                    };
                    let classified = super::classify::classify_files(&files);
                    let mut stored_requests = Vec::new();
                    let mut stored_outcomes = Vec::new();
                    let mut unreadable_locations = Vec::new();
                    let mut has_pending_shape = false;
                    for known in classified.known {
                        let file = &files[known.index];
                        if requires_shape_confirmation(known.kind) {
                            let shape = match shape_for_file(known.kind, &file.bytes) {
                                Ok(shape) => shape,
                                Err(_) => {
                                    unreadable_locations.push(file.path.clone());
                                    continue;
                                }
                            };
                            let shape_hash = hash_shape(&shape);
                            match is_shape_confirmed(&pool, user_id, &shape_hash).await {
                                Ok(true) => {
                                    if remove_pending_shape(&pool, user_id, &sha256, &file.path)
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                Ok(false) => {
                                    if let Ok(stored_path) =
                                        copy_known_file(&read_config.copy_dir, &file.bytes)
                                    {
                                        if let Some(file_sha256) =
                                            stored_path.file_name().and_then(|name| name.to_str())
                                        {
                                            let _ = record_copy(
                                                &pool,
                                                user_id,
                                                file_sha256.to_owned(),
                                                &file.path,
                                                &stored_path,
                                            )
                                            .await;
                                        }
                                    }
                                    let _ = record_pending_shape(
                                        &pool, user_id, &sha256, &file.path, &shape,
                                    )
                                    .await;
                                    if record_pending_ledger(
                                        &pool,
                                        user_id,
                                        sha256.clone(),
                                        file_name_of(&candidate.path),
                                    )
                                    .await
                                    .is_err()
                                    {
                                        return;
                                    }
                                    has_pending_shape = true;
                                    continue;
                                }
                                Err(_) => return,
                            }
                        }
                        let legacy = matches!(
                            known.kind,
                            super::classify::KnownKind::Records
                                | super::classify::KnownKind::SemanticHistory
                        );
                        if read_config.keep_copies {
                            if let Ok(Some(stored_path)) =
                                copy_if_enabled(true, &read_config.copy_dir, &file.bytes)
                            {
                                if let Some(file_sha256) =
                                    stored_path.file_name().and_then(|name| name.to_str())
                                {
                                    if record_copy(
                                        &pool,
                                        user_id,
                                        file_sha256.to_owned(),
                                        &file.path,
                                        &stored_path,
                                    )
                                    .await
                                    .is_err()
                                    {
                                        tracing::warn!(
                                            kind = "archive_copy_catalog",
                                            "書庫写しの目録を残せない"
                                        );
                                        return;
                                    }
                                }
                            } else {
                                tracing::warn!(kind = "archive_copy", "書庫の写しを残せない");
                                return;
                            }
                        }
                        let requests = match requests_for_file_reporting(
                            known.kind,
                            &file.path,
                            &file.bytes,
                            user_id,
                            sha256.clone(),
                        ) {
                            Ok(read) => {
                                // 項目ごとに読めなかった場所も台帳へ運ぶ（spec R4）。
                                unreadable_locations.extend(read.unreadable);
                                read.requests
                            }
                            // ここまで来るのはファイルそのものが読めないとき。
                            Err(_) => {
                                unreadable_locations.push(file.path.clone());
                                continue;
                            }
                        };
                        for request in &requests {
                            if request.logical_source.starts_with("c03-myactivity-")
                                && ensure_myactivity_source(
                                    &pool,
                                    &request.logical_source,
                                    &request.logical_source,
                                )
                                .await
                                .is_err()
                            {
                                tracing::warn!(
                                    kind = "archive_register_source",
                                    "製品ソースを登録できない"
                                );
                                return;
                            }
                        }
                        let sink = crate::PgSink::new(pool.clone());
                        let started_at = chrono::Utc::now();
                        let reading_file = file_name_of(&candidate.path).unwrap_or_default();
                        let inner_path = file.path.clone();
                        let mut note = |items_read: usize| {
                            if let Ok(mut slot) = reading.write() {
                                *slot = Some(Reading {
                                    file_name: reading_file.clone(),
                                    inner_path: inner_path.clone(),
                                    items_read: i64::try_from(items_read).unwrap_or(i64::MAX),
                                    started_at,
                                });
                            }
                        };
                        note(0);
                        let outcomes =
                            match store_requests_with_progress(&sink, requests.clone(), &mut note)
                                .await
                            {
                                Ok(outcomes) => outcomes,
                                Err(_) => {
                                    let _ = record_store_failure(
                                        &pool,
                                        user_id,
                                        &candidate.path,
                                        sha256.clone(),
                                    )
                                    .await;
                                    tracing::warn!(kind = "archive_store", "書庫の格納に失敗した");
                                    return;
                                }
                            };
                        stored_requests.extend(requests);
                        stored_outcomes.extend(outcomes);
                        if legacy {
                            // このファイルが実際に格納できた後だけ、旧経路を退役させる。
                            // `requests` は消費済みなので、元ファイルの解析結果から最終日を導く。
                            let last = if known.kind == super::classify::KnownKind::Records {
                                super::legacy::parse_records(&file.bytes)
                            } else {
                                super::legacy::parse_semantic(&file.bytes)
                            }
                            .ok()
                            .and_then(|records| {
                                records.into_iter().map(|record| record.event_time).max()
                            });
                            if let Some(last) = last {
                                if retire_legacy_sources(&pool, last).await.is_err() {
                                    tracing::warn!(
                                        kind = "archive_retire_legacy",
                                        "移行前ソースを退役できない"
                                    );
                                }
                            }
                        }
                    }
                    if has_pending_shape && stored_requests.is_empty() {
                        return;
                    }
                    let ledger = sqlx::query_scalar(
                    "INSERT INTO core.archive_ledger
                       (user_id, sha256, parser_version, outcome, created_at, inbox_kind, unreadable_count, unreadable_at, skipped_file_count, file_name)
                     VALUES ($1, $2, $3, 'read', $4, $5, $6, $7, $8, $9)
                     RETURNING id",
                )
                .bind(user_id)
                .bind(sha256)
                .bind(super::PARSER_VERSION)
                .bind(archive_created_at(&candidate.path, chrono::Utc::now()))
                .bind(inbox_kind)
                .bind(i32::try_from(result.unreadable + unreadable_locations.len()).unwrap_or(i32::MAX))
                .bind(unreadable_summary(&unreadable_locations))
                .bind(i32::try_from(result.skipped).unwrap_or(i32::MAX))
                .bind(file_name_of(&candidate.path))
                .fetch_one(&pool)
                .await
                ;
                    match ledger {
                        Ok(ledger_id) => {
                            if record_ledger_sources(
                                &pool,
                                ledger_id,
                                &stored_requests,
                                &stored_outcomes,
                            )
                            .await
                            .is_err()
                            {
                                tracing::warn!(
                                    kind = "archive_ledger_source",
                                    "書庫のソース別台帳を残せない"
                                );
                                return;
                            }
                            if !candidate.from_downloads
                                && move_to_processed(&candidate.path).is_err()
                            {
                                tracing::warn!(
                                    kind = "archive_move",
                                    "書庫を取り込み済みへ移せない"
                                );
                            }
                            tracing::info!(
                                kind = "archive_inspect",
                                known = result.known,
                                skipped = result.skipped,
                                unreadable = result.unreadable,
                                "書庫を検査した"
                            );
                        }
                        Err(error) => {
                            tracing::warn!(kind = "archive_ledger", error = %error, "書庫の台帳を残せない")
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(kind = error.kind(), "書庫を開けない");
                    let _ = record_unreadable(
                        &pool,
                        user_id,
                        &sha256,
                        &candidate.path,
                        error.kind(),
                        candidate.from_downloads,
                    )
                    .await;
                    if !candidate.from_downloads {
                        let _ = move_to_processed(&candidate.path);
                    }
                }
            }
        }
    }));

    WorkerHandle {
        tasks: vec![scanning, reading_task],
    }
}
