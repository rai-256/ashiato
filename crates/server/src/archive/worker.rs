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
        rejected: i32,
        max_event_at: Option<chrono::DateTime<chrono::Utc>>,
    }
    let mut per_source: std::collections::BTreeMap<String, Counts> =
        std::collections::BTreeMap::new();
    for (request, outcome) in requests.iter().zip(outcomes) {
        let counts = per_source
            .entry(request.logical_source.clone())
            .or_default();
        match outcome {
            crate::StoreOutcome::Inserted(_) => counts.inserted += 1,
            crate::StoreOutcome::Duplicate(_) => counts.duplicate += 1,
            crate::StoreOutcome::DuplicateOfDeleted(_) => counts.deleted += 1,
            // **弾かれた記録は最終日を進めない**（review I4）。spec は最終日を
            // 「**入った記録のうち**いちばん新しい出来事の日」と定めている。
            // 入らなかった件数は台帳の読めなかった件数として数える。
            crate::StoreOutcome::Rejected(_) => {
                counts.rejected += 1;
                continue;
            }
        }
        counts.max_event_at = Some(
            counts
                .max_event_at
                .map_or(request.event_time, |old| old.max(request.event_time)),
        );
    }
    for (logical_source, counts) in per_source {
        // **同じ読みの行に同じ論理ソースが既にあれば足さない**（鍵 `(ledger_id, logical_source)`）。
        // 印を置いた後の読み直しは 1 回目の読みの `read` の行へ足す（D18）ので、1 回目に
        // 印のあるファイルが同じ論理ソースを書いていると鍵に当たる。当たって関数ごと抜けると、
        // 毎走査同じ行から始まって**読み直しが永久に止まった**（final review R50）。
        // 記録そのものは格納済み。台帳の件数が 1 回目のぶんだけになることはログに種別で残す。
        let inserted = sqlx::query(
            "INSERT INTO core.archive_ledger_source
               (ledger_id, logical_source, inserted_count, duplicate_count, deleted_count, unreadable_count, max_event_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (ledger_id, logical_source) DO NOTHING",
        )
        .bind(ledger_id)
        .bind(logical_source)
        .bind(counts.inserted)
        .bind(counts.duplicate)
        .bind(counts.deleted)
        .bind(counts.rejected)
        .bind(counts.max_event_at)
        .execute(pool)
        .await?;
        if inserted.rows_affected() == 0 {
            tracing::warn!(
                kind = "archive_ledger_source_overlap",
                "同じ読みの台帳に同じ論理ソースが既にある（件数は 1 回目のぶん）"
            );
        }
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

type When = anyhow::Result<(
    chrono::DateTime<chrono::Utc>,
    super::timezone::SourceTimezone,
)>;

/// 記録 1 件の材料。時刻・地域・原文の範囲・design D6 の欄を、項目の形ごとに集めたもの。
struct Item<'a> {
    source: String,
    when: When,
    /// 原文を切り出せないときに書き戻す値。
    value: &'a serde_json::Value,
    /// **書庫のバイト列の切り出し**（spec「原文は書庫のバイト列の一部と一致する」）。
    raw: Option<&'a [u8]>,
    fields: serde_json::Map<String, serde_json::Value>,
}

/// 時刻の表記から UTC の時刻と取得元が示した地域を取る。
///
/// `offset` は Timeline の `startTimeTimezoneUtcOffsetMinutes`（あれば表記より優先。D5）。
/// **示していなければ UTC のまま**（位置から推定しない。本人の決定 C2）。
fn when_text(text: Option<&str>, offset: Option<i32>) -> When {
    let text = text.ok_or_else(|| anyhow::anyhow!("書庫項目の時刻が無い"))?;
    let time = chrono::DateTime::parse_from_rfc3339(text)?.to_utc();
    Ok((time, super::timezone::from_timestamp(text, offset)?))
}

/// 時差を持たない時刻（`time_usec` / `timestampMs`）。取得元は地域を示していない。
fn when_utc(time: chrono::DateTime<chrono::Utc>) -> When {
    Ok((
        time,
        super::timezone::SourceTimezone {
            offset_min: 0,
            id: "UTC".into(),
            from_source: false,
        },
    ))
}

fn text<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a str> {
    at(value, path).and_then(serde_json::Value::as_str)
}

fn at<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    path.iter().try_fold(value, |value, key| value.get(*key))
}

/// `"35.0116°, 135.7681°"`（`geo:` が前に付く書き出しもある）を度の組にする。
/// **原文は文字列のまま**（D6）。ここは解析済みの内容に足すだけ。
fn lat_lng(text: Option<&str>) -> Option<(f64, f64)> {
    let text = text?.trim();
    let text = text.strip_prefix("geo:").unwrap_or(text);
    let (lat, lng) = text.split_once(',')?;
    let number = |part: &str| part.trim().trim_end_matches('°').trim().parse::<f64>().ok();
    Some((number(lat)?, number(lng)?))
}

/// E7（度の 1,000 万倍の整数）を度にする。
fn e7(value: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let degrees = value?.as_i64()? as f64 / 10_000_000.0;
    serde_json::Number::from_f64(degrees).map(serde_json::Value::Number)
}

/// 欄を足す。値が無い欄は足さない（`null` を入れて「欄があった」ように見せない）。
fn put(
    fields: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
    value: Option<serde_json::Value>,
) {
    if let Some(value) = value {
        fields.insert(name.to_owned(), value);
    }
}

fn put_lat_lng(
    fields: &mut serde_json::Map<String, serde_json::Value>,
    prefix: &str,
    text: Option<&str>,
) {
    if let Some((lat, lng)) = lat_lng(text) {
        fields.insert(format!("{prefix}lat"), serde_json::json!(lat));
        fields.insert(format!("{prefix}lng"), serde_json::json!(lng));
    }
}

fn cloned(value: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    value.cloned()
}

/// 解釈した配列と**添字が揃うときだけ**切り出しを使う。揃わなければ書き戻しへ落とす
/// （原文の厳密さは失うが、別の記録の原文を付ける取り違えはしない。review I3）。
fn aligned(spans: Option<Vec<&[u8]>>, expected: usize) -> Vec<Option<&[u8]>> {
    match spans {
        Some(spans) if spans.len() == expected => spans.into_iter().map(Some).collect(),
        Some(_) => {
            tracing::warn!(
                kind = "archive_raw_slice",
                "原文の切り出しと項目の数が合わない"
            );
            vec![None; expected]
        }
        None => vec![None; expected],
    }
}

/// オブジェクト `input` の欄 `key` が配列なら、その各要素の切り出し。
fn member_elements<'a>(input: Option<&'a [u8]>, key: &str) -> Option<Vec<&'a [u8]>> {
    let member = super::slice::object_member(input?, key).ok()??;
    super::slice::array_elements(member).ok()
}

fn as_array(value: Option<&serde_json::Value>) -> &[serde_json::Value] {
    value
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// 端末が書き出した `Timeline.json`（D6）。
///
/// **時刻と時差はセグメントの側にある**（`startTime` / `startTimeTimezoneUtcOffsetMinutes`）。
/// `visit` / `activity` の中を見ていたときは、本物の書き出しから 1 件も入らなかった（final review R48）。
/// 訪問・移動の原文はセグメント 1 つ、経路の点と生の信号はその項目 1 つ。
fn timeline_items<'a>(root: &'a serde_json::Value, bytes: &'a [u8]) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    let segments = as_array(root.get("semanticSegments"));
    let segment_raws = aligned(
        member_elements(Some(bytes), "semanticSegments"),
        segments.len(),
    );
    for (segment, segment_raw) in segments.iter().zip(segment_raws) {
        let offset = segment
            .get("startTimeTimezoneUtcOffsetMinutes")
            .and_then(serde_json::Value::as_i64)
            .and_then(|minutes| i32::try_from(minutes).ok());
        let start = text(segment, &["startTime"]);
        if let Some(visit) = segment.get("visit") {
            let mut fields = serde_json::Map::new();
            put(&mut fields, "end_time", cloned(segment.get("endTime")));
            put(
                &mut fields,
                "place_id",
                cloned(at(visit, &["topCandidate", "placeId"])),
            );
            put(
                &mut fields,
                "semantic_type",
                cloned(at(visit, &["topCandidate", "semanticType"])),
            );
            put_lat_lng(
                &mut fields,
                "",
                text(visit, &["topCandidate", "placeLocation", "latLng"]),
            );
            put(&mut fields, "probability", cloned(visit.get("probability")));
            items.push(Item {
                source: "c03-timeline-visit".into(),
                when: when_text(start, offset),
                value: segment,
                raw: segment_raw,
                fields,
            });
        }
        if let Some(activity) = segment.get("activity") {
            let mut fields = serde_json::Map::new();
            put(&mut fields, "end_time", cloned(segment.get("endTime")));
            put(
                &mut fields,
                "activity_type",
                cloned(at(activity, &["topCandidate", "type"])),
            );
            put(
                &mut fields,
                "distance_m",
                cloned(activity.get("distanceMeters")),
            );
            put_lat_lng(&mut fields, "start_", text(activity, &["start", "latLng"]));
            put_lat_lng(&mut fields, "end_", text(activity, &["end", "latLng"]));
            items.push(Item {
                source: "c03-timeline-move".into(),
                when: when_text(start, offset),
                value: segment,
                raw: segment_raw,
                fields,
            });
        }
        let points = as_array(segment.get("timelinePath"));
        let point_raws = aligned(member_elements(segment_raw, "timelinePath"), points.len());
        for (point, point_raw) in points.iter().zip(point_raws) {
            let mut fields = serde_json::Map::new();
            put_lat_lng(&mut fields, "", text(point, &["point"]));
            items.push(Item {
                source: "c03-timeline-route".into(),
                when: when_text(text(point, &["time"]), offset),
                value: point,
                raw: point_raw,
                fields,
            });
        }
    }
    let signals = as_array(root.get("rawSignals"));
    let signal_raws = aligned(member_elements(Some(bytes), "rawSignals"), signals.len());
    for (signal, signal_raw) in signals.iter().zip(signal_raws) {
        // **信号は 1 段入れ子**（`{"position":{…}}` / `{"wifiScan":{…}}` / `{"activityRecord":{…}}`）。
        let inner = signal
            .as_object()
            .and_then(|object| object.iter().find(|(_, value)| value.is_object()));
        let mut fields = serde_json::Map::new();
        let when = match inner {
            Some((kind, inner)) => {
                fields.insert("signal_kind".into(), serde_json::json!(kind));
                put_lat_lng(
                    &mut fields,
                    "",
                    text(inner, &["LatLng"]).or_else(|| text(inner, &["latLng"])),
                );
                put(
                    &mut fields,
                    "accuracy_m",
                    cloned(inner.get("accuracyMeters")),
                );
                when_text(
                    text(inner, &["timestamp"]).or_else(|| text(inner, &["deliveryTime"])),
                    None,
                )
            }
            None => Err(anyhow::anyhow!("生の信号の形が分からない")),
        };
        items.push(Item {
            source: "c03-timeline-signal".into(),
            when,
            value: signal,
            raw: signal_raw,
            fields,
        });
    }
    items
}

/// 移行前の時刻（`timestamp` / `startTimestamp` の表記か、`timestampMs` のミリ秒）。
fn legacy_when(value: &serde_json::Value) -> When {
    let value = value.get("duration").unwrap_or(value);
    if let Some(text) = text(value, &["timestamp"]).or_else(|| text(value, &["startTimestamp"])) {
        return when_text(Some(text), None);
    }
    let millis = text(value, &["timestampMs"])
        .or_else(|| text(value, &["startTimestampMs"]))
        .ok_or_else(|| anyhow::anyhow!("移行前の項目の時刻が無い"))?
        .parse::<i64>()?;
    when_utc(
        chrono::DateTime::from_timestamp_millis(millis)
            .ok_or_else(|| anyhow::anyhow!("移行前の時刻が範囲外"))?,
    )
}

fn legacy_end(value: &serde_json::Value) -> Option<serde_json::Value> {
    cloned(
        at(value, &["duration", "endTimestamp"])
            .or_else(|| at(value, &["duration", "endTimestampMs"])),
    )
}

/// `Records.json` と `Semantic Location History`（D6）。
fn legacy_items<'a>(
    kind: super::classify::KnownKind,
    root: &'a serde_json::Value,
    bytes: &'a [u8],
) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    if kind == super::classify::KnownKind::Records {
        let rows = as_array(root.get("locations"));
        let raws = aligned(member_elements(Some(bytes), "locations"), rows.len());
        for (row, raw) in rows.iter().zip(raws) {
            let mut fields = serde_json::Map::new();
            put(&mut fields, "lat", e7(row.get("latitudeE7")));
            put(&mut fields, "lng", e7(row.get("longitudeE7")));
            put(&mut fields, "accuracy_m", cloned(row.get("accuracy")));
            put(&mut fields, "source", cloned(row.get("source")));
            put(&mut fields, "device_tag", cloned(row.get("deviceTag")));
            items.push(Item {
                source: "c03-legacy-location".into(),
                when: legacy_when(row),
                value: row,
                raw,
                fields,
            });
        }
        return items;
    }
    let rows = as_array(root.get("timelineObjects"));
    let raws = aligned(member_elements(Some(bytes), "timelineObjects"), rows.len());
    for (row, row_raw) in rows.iter().zip(raws) {
        for (field, source) in [
            ("placeVisit", "c03-legacy-visit"),
            ("activitySegment", "c03-legacy-activity"),
        ] {
            let Some(item) = row.get(field) else {
                continue;
            };
            let raw =
                row_raw.and_then(|raw| super::slice::object_member(raw, field).ok().flatten());
            let mut fields = serde_json::Map::new();
            put(&mut fields, "end_time", legacy_end(item));
            if field == "placeVisit" {
                put(
                    &mut fields,
                    "place_id",
                    cloned(at(item, &["location", "placeId"])),
                );
                put(&mut fields, "name", cloned(at(item, &["location", "name"])));
                put(
                    &mut fields,
                    "address",
                    cloned(at(item, &["location", "address"])),
                );
                put(
                    &mut fields,
                    "lat",
                    e7(at(item, &["location", "latitudeE7"])),
                );
                put(
                    &mut fields,
                    "lng",
                    e7(at(item, &["location", "longitudeE7"])),
                );
            } else {
                put(
                    &mut fields,
                    "activity_type",
                    cloned(item.get("activityType")),
                );
                put(&mut fields, "distance_m", cloned(item.get("distance")));
            }
            items.push(Item {
                source: source.into(),
                when: legacy_when(item),
                value: item,
                raw,
                fields,
            });
        }
    }
    items
}

/// YouTube の視聴・検索、マイアクティビティ、Chrome の履歴（D6）。
fn takeout_items<'a>(
    kind: super::classify::KnownKind,
    root: &'a serde_json::Value,
    bytes: &'a [u8],
) -> Vec<Item<'a>> {
    use super::classify::KnownKind;
    let (rows, raws) = if kind == KnownKind::ChromeHistory {
        let rows = as_array(root.get("Browser History"));
        let raws = aligned(member_elements(Some(bytes), "Browser History"), rows.len());
        (rows, raws)
    } else {
        let rows = as_array(Some(root));
        let raws = aligned(super::slice::array_elements(bytes).ok(), rows.len());
        (rows, raws)
    };
    let mut items = Vec::new();
    for (row, raw) in rows.iter().zip(raws) {
        let mut fields = serde_json::Map::new();
        let url = text(row, &["titleUrl"]);
        let (source, when) = match kind {
            KnownKind::YouTubeWatch | KnownKind::YouTubeSearch => {
                // **見分けた種類を既定にする。** 削除済みの動画は `titleUrl` を持たない。
                // URL が無ければ検索へ回していたときは、視聴が検索のソースに入り、
                // 検索履歴が先に書いた台帳の行と衝突して読み直しが止まった（final review R50）。
                let search = match url {
                    Some(url) if url.contains("watch?v=") => false,
                    Some(url) if url.contains("search_query=") => true,
                    _ => kind == KnownKind::YouTubeSearch,
                };
                put(&mut fields, "title", cloned(row.get("title")));
                put(&mut fields, "url", url.map(|url| serde_json::json!(url)));
                if search {
                    let query = url
                        .and_then(|url| url.split("search_query=").nth(1))
                        .map(|encoded| encoded.split('&').next().unwrap_or(encoded))
                        .map(super::youtube::percent_decode);
                    put(&mut fields, "query", query.map(serde_json::Value::String));
                } else {
                    put(
                        &mut fields,
                        "channel_name",
                        cloned(first_subtitle(row, "name")),
                    );
                    put(
                        &mut fields,
                        "channel_url",
                        cloned(first_subtitle(row, "url")),
                    );
                }
                (
                    if search {
                        "c03-youtube-search".to_owned()
                    } else {
                        "c03-youtube-watch".to_owned()
                    },
                    when_text(text(row, &["time"]), None),
                )
            }
            KnownKind::MyActivity => {
                // 製品の名前は**画面の見出し**になる（`マイアクティビティ: <製品>`）。
                let product = row
                    .get("products")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|products| products.first())
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                fields.insert("product".into(), serde_json::json!(product));
                put(&mut fields, "title", cloned(row.get("title")));
                put(&mut fields, "url", url.map(|url| serde_json::json!(url)));
                put(&mut fields, "details", cloned(row.get("details")));
                (
                    super::myactivity::source_name(product),
                    when_text(text(row, &["time"]), None),
                )
            }
            KnownKind::ChromeHistory => {
                for name in ["title", "url", "page_transition", "client_id"] {
                    put(&mut fields, name, cloned(row.get(name)));
                }
                let when = row
                    .get("time_usec")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| anyhow::anyhow!("Chrome時刻が無い"))
                    .and_then(super::chrome::time_usec_to_utc)
                    .and_then(when_utc);
                ("c03-chrome-history".to_owned(), when)
            }
            _ => continue,
        };
        items.push(Item {
            source,
            when,
            value: row,
            raw,
            fields,
        });
    }
    items
}

fn first_subtitle<'a>(row: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    row.get("subtitles")
        .and_then(serde_json::Value::as_array)
        .and_then(|subtitles| subtitles.first())
        .and_then(|subtitle| subtitle.get(name))
}

/// `requests_for_file` の、読めなかった項目の場所も返す版。読み手はこちらを使う。
pub fn requests_for_file_reporting(
    kind: super::classify::KnownKind,
    inner_path: &str,
    bytes: &[u8],
    user_id: uuid::Uuid,
    archive_sha256: String,
) -> anyhow::Result<FileRequests> {
    use super::classify::KnownKind;
    let root: serde_json::Value = serde_json::from_slice(bytes)?;
    let items = match kind {
        KnownKind::Timeline => timeline_items(&root, bytes),
        KnownKind::Records | KnownKind::SemanticHistory => legacy_items(kind, &root, bytes),
        _ => takeout_items(kind, &root, bytes),
    };
    let mut out = FileRequests::default();
    for (position, item) in items.into_iter().enumerate() {
        match build_request(item, inner_path, user_id, &archive_sha256) {
            Ok(request) => out.requests.push(request),
            // **この項目だけ飛ばす。** 場所を残すので、入らなかったことは台帳に出る。
            Err(_) => out.unreadable.push(format!("{inner_path}#{position}")),
        }
    }
    Ok(out)
}

/// 材料から要求を作る。payload は design D6 の全記録の欄 + 論理ソースごとの欄。
fn build_request(
    item: Item<'_>,
    inner_path: &str,
    user_id: uuid::Uuid,
    archive_sha256: &str,
) -> anyhow::Result<crate::IngestRequest> {
    let (event_time, zone) = item.when?;
    let raw = match item.raw.map(std::str::from_utf8) {
        Some(Ok(raw)) => raw.to_owned(),
        // 切り出せない形のときだけ書き戻す（原文の厳密さは失うが、項目は落とさない）。
        _ => serde_json::to_string(item.value)?,
    };
    let mut payload = item.fields;
    payload.insert("archive_sha256".into(), serde_json::json!(archive_sha256));
    payload.insert("inner_path".into(), serde_json::json!(inner_path));
    // **取得元が地域を示したか**（spec「取得元が地域を持たなかった印」。D5）。
    payload.insert("tz_from_source".into(), serde_json::json!(zone.from_source));
    payload.insert(
        "parser_version".into(),
        serde_json::json!(super::PARSER_VERSION),
    );
    Ok(crate::IngestRequest {
        id: uuid::Uuid::new_v4(),
        user_id,
        logical_source: item.source,
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
        raw,
        payload: serde_json::Value::Object(payload),
    })
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
    Ok(copy_known_file_reporting(copy_dir, bytes)?.0)
}

/// `copy_known_file` の、**今回作ったか**（前から在ったのでなく）も返す版。
pub fn copy_known_file_reporting(
    copy_dir: &std::path::Path,
    bytes: &[u8],
) -> std::io::Result<(std::path::PathBuf, bool)> {
    use sha2::Digest as _;
    let hash = format!("{:x}", sha2::Sha256::digest(bytes));
    let target = copy_dir.join(&hash[..2]).join(&hash);
    if target.exists() {
        return Ok((target, false));
    }
    let parent = target.parent().expect("写しの親");
    std::fs::create_dir_all(parent)?;
    std::fs::write(&target, bytes)?;
    Ok((target, true))
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
    archive_sha256: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO core.archive_file
           (sha256, user_id, inner_path, stored_path, archive_sha256)
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
    )
    .bind(sha256)
    .bind(user_id)
    .bind(inner_path)
    .bind(stored_path.to_string_lossy().as_ref())
    .bind(archive_sha256)
    .execute(pool)
    .await?;
    Ok(())
}

/// 確認待ちのファイル 1 つの写し: 書庫のハッシュ・書庫の中のパス・写しの場所・
/// 確認待ちのために写しを作ったか・写しの中身のハッシュ。
pub type PendingCopy = (String, String, String, bool, String);

/// 印が置かれて読めるようになった、確認待ちのファイルの写し。
///
/// **写しから読む**（D16）—— 置き場の書庫は「取り込み済み」へ移っているか、
/// 既読として覚えられているので、置き場をもう一度見ても読み直せない。
pub async fn confirmed_pending_copies(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<Vec<PendingCopy>, sqlx::Error> {
    // **写しは中身のハッシュで引く**（final review R51）。書庫のハッシュで引いていたときは、
    // 中身が同じファイルを持つ 2 冊目の書庫の目録が鍵 `(user_id, sha256)` で落ちていて、
    // 2 冊目が永久に確認待ちに残った。中身のハッシュを持たない古い行だけ書庫で引く。
    sqlx::query_as(
        "SELECT p.sha256, p.inner_path, f.stored_path, p.made_copy, f.sha256
           FROM core.archive_pending_shape p
           JOIN core.archive_file f
             ON f.user_id = p.user_id
            AND (f.sha256 = p.file_sha256
                 OR (p.file_sha256 IS NULL
                     AND f.archive_sha256 = p.sha256
                     AND f.inner_path = p.inner_path))
          WHERE p.user_id = $1
            AND EXISTS (SELECT 1 FROM core.archive_shape_confirmation c
                         WHERE c.user_id = p.user_id AND c.shape_hash = p.shape_hash)
          ORDER BY p.sha256, p.inner_path",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
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
    inner_path: &str,
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
    // **判断の材料**（D16）: 見分けた種類・パスの型・最上位の鍵・欄の名前・件数・
    // 製品の値。**値（題名・URL・検索語・座標・時刻）は出さない。**
    // 件数とパスの型が無いと、本人は `tools/archive-shape.sh` の出力だけでは
    // 「何をどれだけ入れようとしているのか」を判断できない（review R15）。
    let items = match &value {
        serde_json::Value::Array(rows) => rows.len(),
        serde_json::Value::Object(object) => object
            .values()
            .filter_map(serde_json::Value::as_array)
            .map(Vec::len)
            .max()
            .unwrap_or(0),
        _ => 0,
    };
    Ok(serde_json::json!({
        "kind": format!("{kind:?}"),
        "products": products,
        "top_level_keys": top_level_keys,
        "field_names": field_names,
        "items": items,
        "path_shape": path_shape(inner_path),
    }))
}

/// 書庫の中のパスを**型**にする。名前そのものは訳で変わるので形に入れない
/// （D16。ここは確認の材料として出すだけで、`hash_shape` は見ない）。
fn path_shape(inner_path: &str) -> String {
    let depth = inner_path.matches('/').count();
    let extension = std::path::Path::new(inner_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("");
    format!("depth={depth};ext={extension}")
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
    record_pending_file(
        pool,
        user_id,
        archive_sha256,
        inner_path,
        shape,
        None,
        false,
    )
    .await
}

/// `record_pending_shape` の、写しの中身のハッシュと「確認待ちのために写しを作ったか」も積む版。
pub async fn record_pending_file(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    archive_sha256: &str,
    inner_path: &str,
    shape: &serde_json::Value,
    file_sha256: Option<&str>,
    made_copy: bool,
) -> Result<(), sqlx::Error> {
    let shape_hash = hash_shape(shape);
    sqlx::query(
        "INSERT INTO core.archive_pending_shape
           (user_id, sha256, inner_path, shape_hash, shape, file_sha256, made_copy)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (user_id, sha256, inner_path) DO UPDATE
           SET file_sha256 = COALESCE(EXCLUDED.file_sha256, core.archive_pending_shape.file_sha256),
               made_copy = core.archive_pending_shape.made_copy OR EXCLUDED.made_copy",
    )
    .bind(user_id)
    .bind(archive_sha256)
    .bind(inner_path)
    .bind(shape_hash)
    .bind(shape)
    .bind(file_sha256)
    .bind(made_copy)
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
        "INSERT INTO core.archive_scan_counter
           (user_id, scanned_at, attempts, successes, last_capturable, last_blockers)
         VALUES ($1, $2, 1, CASE WHEN $3 THEN 1 ELSE 0 END, $3, $4)
         ON CONFLICT (user_id) DO UPDATE SET scanned_at = EXCLUDED.scanned_at,
           attempts = core.archive_scan_counter.attempts + 1,
           successes = core.archive_scan_counter.successes + CASE WHEN $3 THEN 1 ELSE 0 END,
           last_capturable = EXCLUDED.last_capturable,
           last_blockers = EXCLUDED.last_blockers
         RETURNING attempts, successes",
    )
    .bind(user_id)
    .bind(emitted_at)
    .bind(capturable)
    .bind(&blockers)
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
    record_pending_ledger_at(pool, user_id, sha256, file_name, None, "inbox").await
}

/// `record_pending_ledger` の、書庫の作られた時刻と置き場の種類も残す版。
/// 印を置いた後の読み直しが、ここから `read` の行へ引き継ぐ（final review R58）。
pub async fn record_pending_ledger_at(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    sha256: String,
    file_name: Option<String>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    inbox_kind: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO core.archive_ledger
           (user_id, sha256, parser_version, outcome, file_name, created_at, inbox_kind)
         VALUES ($1, $2, $3, 'pending_shape', $4, $5, $6) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(sha256)
    .bind(super::PARSER_VERSION)
    .bind(file_name)
    .bind(created_at)
    .bind(inbox_kind)
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

/// 読み直しが書く `read` の行へ引き継ぐ、元の読みの台帳の値（final review R58）。
#[derive(Debug, Clone, Default)]
pub struct LedgerMeta {
    pub file_name: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub inbox_kind: String,
}

/// その書庫の、ある版・ある結果の台帳の行から名前・作られた時刻・置き場の種類を引く。
async fn ledger_meta(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    sha256: &str,
    parser_version: Option<&str>,
    outcome: &str,
) -> Result<LedgerMeta, sqlx::Error> {
    let row: Option<(
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
        String,
    )> = sqlx::query_as(
        "SELECT file_name, created_at, inbox_kind FROM core.archive_ledger
              WHERE user_id = $1 AND sha256 = $2 AND outcome = $3
                AND ($4::text IS NULL OR parser_version = $4)
              ORDER BY finished_at DESC, id DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(sha256)
    .bind(outcome)
    .bind(parser_version)
    .fetch_optional(pool)
    .await?;
    Ok(row
        .map(|(file_name, created_at, inbox_kind)| LedgerMeta {
            file_name,
            created_at,
            inbox_kind,
        })
        .unwrap_or_else(|| LedgerMeta {
            inbox_kind: "inbox".into(),
            ..LedgerMeta::default()
        }))
}

/// 1 冊の書庫の、写しに残したファイルを読み直して格納し、いまの版の `read` の行へ結果を残す。
///
/// 返すのは格納まで終えたファイルの書庫の中のパス。**格納の途中で落ちたら Err**
/// （台帳を書かないので、次の走査で同じファイルから読み直す。格納は内容の鍵で冪等）。
/// `require_confirmed` のときは、形の印が無いファイルを読まない（D16）。
async fn reread_archive(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    archive_sha256: &str,
    files: &[(String, String)],
    meta: &LedgerMeta,
    require_confirmed: bool,
) -> anyhow::Result<Vec<String>> {
    let sink = crate::PgSink::new(pool.clone());
    let mut stored_requests = Vec::new();
    let mut stored_outcomes = Vec::new();
    let mut unreadable = Vec::new();
    let mut done = Vec::new();
    for (inner_path, stored_path) in files {
        let Ok(bytes) = std::fs::read(stored_path) else {
            tracing::warn!(kind = "archive_reparse_copy", "書庫の写しを読めない");
            continue;
        };
        let file = super::open::ArchiveFile {
            path: inner_path.clone(),
            bytes,
        };
        let classified = super::classify::classify_files(std::slice::from_ref(&file));
        let Some(known) = classified.known.first() else {
            tracing::warn!(
                kind = "archive_reparse_classify",
                "書庫の写しを見分けられない"
            );
            continue;
        };
        if require_confirmed && requires_shape_confirmation(known.kind) {
            let shape = shape_for_file(known.kind, inner_path, &file.bytes)?;
            if !is_shape_confirmed(pool, user_id, &hash_shape(&shape)).await? {
                tracing::warn!(
                    kind = "archive_reparse_unconfirmed",
                    "形の印が無い写しは読み直さない"
                );
                continue;
            }
        }
        let read = match requests_for_file_reporting(
            known.kind,
            inner_path,
            &file.bytes,
            user_id,
            archive_sha256.to_owned(),
        ) {
            Ok(read) => read,
            // ファイルそのものが読めない。読み直しても変わらないので、読めなかったとして畳む。
            Err(_) => {
                unreadable.push(inner_path.clone());
                done.push(inner_path.clone());
                continue;
            }
        };
        for request in &read.requests {
            if request.logical_source.starts_with("c03-myactivity-") {
                ensure_myactivity_source(
                    pool,
                    &request.logical_source,
                    request.payload["product"]
                        .as_str()
                        .unwrap_or(&request.logical_source),
                )
                .await?;
            }
        }
        let outcomes = store_requests(&sink, read.requests.clone()).await?;
        unreadable.extend(read.unreadable);
        stored_requests.extend(read.requests);
        stored_outcomes.extend(outcomes);
        done.push(inner_path.clone());
    }
    if done.is_empty() {
        return Ok(done);
    }
    // **`read` の台帳の行より先に印を付ける**（final review 第 2 回 R72）。落ちたら Err で返し、いまの版の
    // `read` の行を書かないので、版の読み直しは次の周の対象に残り、確認待ちは確認待ちの行が残る。
    // どちらも次に写しから読み直して（格納は内容の鍵で増えない）印を付け直す。
    crate::stay_store::mark_archive_arrivals(pool, user_id, &stored_requests).await?;
    // **台帳の行は増やさない側に倒す**（`ON CONFLICT DO NOTHING`。design D18）。混在した書庫は
    // 1 回目の読みで既に `read` の行を持っている。名前・作られた時刻・置き場の種類は
    // 元の読みの行から引き継ぐ（NULL と既定の `inbox` で書いていた。final review R58）。
    let ledger: Option<i64> = sqlx::query_scalar(
        "INSERT INTO core.archive_ledger
           (user_id, sha256, parser_version, outcome, file_name, created_at, inbox_kind,
            unreadable_count, unreadable_at)
         VALUES ($1, $2, $3, 'read', $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING RETURNING id",
    )
    .bind(user_id)
    .bind(archive_sha256)
    .bind(super::PARSER_VERSION)
    .bind(&meta.file_name)
    .bind(meta.created_at)
    .bind(&meta.inbox_kind)
    .bind(i32::try_from(unreadable.len()).unwrap_or(i32::MAX))
    .bind(unreadable_summary(&unreadable))
    .fetch_optional(pool)
    .await?;
    let ledger_id = match ledger {
        Some(id) => id,
        None => {
            sqlx::query_scalar(
                "SELECT id FROM core.archive_ledger
                  WHERE user_id = $1 AND sha256 = $2 AND parser_version = $3
                    AND outcome = 'read'",
            )
            .bind(user_id)
            .bind(archive_sha256)
            .bind(super::PARSER_VERSION)
            .fetch_one(pool)
            .await?
        }
    };
    record_ledger_sources(pool, ledger_id, &stored_requests, &stored_outcomes).await?;
    Ok(done)
}

/// 印が置かれた後に、確認待ちだったファイルを**写しから**読み直して格納する（D16）。
///
/// 置き場の書庫はもう「取り込み済み」へ移っているか既読として覚えられているので、
/// 走査をもう一度回しても読み直せない。**ここが無いと、本人が `--confirm` を叩いても
/// 何も起きない**（実測: 混在した書庫＝本物の Takeout の形で必ず起きる）。
///
/// **書庫ごとに読み、1 冊の失敗で他の書庫を止めない**（final review R50）。`?` で関数ごと
/// 抜けていたときは、毎走査同じ順で同じ行から始まり、後ろの書庫が永久に読まれなかった。
/// 写しを残さない設定（`keep_copies = false`）のときは、確認待ちのために作った写しを
/// 読み直し終えたら消す（spec / 本人の決定 第 2 回 Q11。final review R56）。
pub async fn ingest_confirmed_pending(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    keep_copies: bool,
) -> anyhow::Result<usize> {
    let rows = confirmed_pending_copies(pool, user_id).await?;
    let mut by_archive: std::collections::BTreeMap<String, Vec<PendingCopy>> =
        std::collections::BTreeMap::new();
    for row in rows {
        by_archive.entry(row.0.clone()).or_default().push(row);
    }
    let mut ingested = 0;
    for (archive_sha256, copies) in by_archive {
        let files: Vec<(String, String)> = copies
            .iter()
            .map(|(_, inner_path, stored_path, _, _)| (inner_path.clone(), stored_path.clone()))
            .collect();
        let read = async {
            let meta = ledger_meta(
                pool,
                user_id,
                &archive_sha256,
                Some(super::PARSER_VERSION),
                "pending_shape",
            )
            .await?;
            let done = reread_archive(pool, user_id, &archive_sha256, &files, &meta, false).await?;
            for inner_path in &done {
                remove_pending_shape(pool, user_id, &archive_sha256, inner_path).await?;
            }
            anyhow::Ok(done)
        }
        .await;
        let done = match read {
            Ok(done) => done,
            Err(error) => {
                tracing::warn!(kind = "archive_reparse", error = %error, "確認待ちの書庫を読み直せない");
                continue;
            }
        };
        ingested += done.len();
        if keep_copies {
            continue;
        }
        for (_, inner_path, stored_path, made_copy, file_sha256) in &copies {
            if !made_copy || !done.contains(inner_path) {
                continue;
            }
            // 同じ中身の写しを、まだ確認待ちの別の書庫が使っていれば残す。
            let still_used: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM core.archive_pending_shape
                                WHERE user_id = $1 AND file_sha256 = $2)",
            )
            .bind(user_id)
            .bind(file_sha256)
            .fetch_one(pool)
            .await?;
            if !still_used && std::fs::remove_file(stored_path).is_err() {
                tracing::warn!(
                    kind = "archive_pending_copy_remove",
                    "確認待ちの写しを消せない"
                );
            }
        }
    }
    Ok(ingested)
}

/// 解析器の版が上がったとき、前の版で読んだ書庫を**写しから**読み直す（D8。final review R53）。
///
/// 専用のフォルダの書庫は「取り込み済み」へ移って走査されないので、写しが唯一の道。
/// ダウンロードのフォルダに残っている書庫は走査が「読む」へ回す（台帳はいまの版の行を持たない）。
/// 写しが無い書庫は読み直さず、件数だけをログに出す。
pub async fn reparse_older_versions(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> anyhow::Result<usize> {
    let archives: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT l.sha256 FROM core.archive_ledger l
          WHERE l.user_id = $1 AND l.outcome = 'read' AND l.parser_version <> $2
            AND NOT EXISTS (SELECT 1 FROM core.archive_ledger n
                             WHERE n.user_id = l.user_id AND n.sha256 = l.sha256
                               AND n.parser_version = $2
                               AND n.outcome IN ('read', 'unreadable', 'pending_shape'))
          ORDER BY l.sha256",
    )
    .bind(user_id)
    .bind(super::PARSER_VERSION)
    .fetch_all(pool)
    .await?;
    let mut reread = 0;
    let mut without_copies = 0;
    for archive_sha256 in archives {
        let files: Vec<(String, String)> = sqlx::query_as(
            "SELECT inner_path, stored_path FROM core.archive_file
              WHERE user_id = $1 AND archive_sha256 = $2 ORDER BY inner_path",
        )
        .bind(user_id)
        .bind(&archive_sha256)
        .fetch_all(pool)
        .await?;
        let files: Vec<(String, String)> = files
            .into_iter()
            .filter(|(_, stored)| std::path::Path::new(stored).exists())
            .collect();
        if files.is_empty() {
            without_copies += 1;
            continue;
        }
        let read = async {
            let meta = ledger_meta(pool, user_id, &archive_sha256, None, "read").await?;
            reread_archive(pool, user_id, &archive_sha256, &files, &meta, true).await
        }
        .await;
        match read {
            Ok(done) if !done.is_empty() => reread += 1,
            Ok(_) => without_copies += 1,
            Err(error) => {
                tracing::warn!(kind = "archive_reparse_version", error = %error, "前の版で読んだ書庫を読み直せない");
            }
        }
    }
    if without_copies > 0 {
        tracing::debug!(
            kind = "archive_reparse_no_copy",
            archives = without_copies,
            "写しが無いので前の版の書庫を読み直さない"
        );
    }
    Ok(reread)
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
            // **印が置かれていないか、走査のたびに見る**（D16）。本人が
            // `tools/archive-shape.sh --confirm` を叩いた後に効く唯一の経路。
            // **解析器の版が上がっていれば、前の版で読んだ書庫を写しから読み直す**（D8）。
            // 本番から呼ばれていなかったので、版を上げても専用のフォルダの書庫は
            // 二度と読まれなかった（final review R53）。走査より先に回すので、
            // 同じ書庫を走査が「読む」へ回しても読み手が既読として捨てる。
            if let Err(error) = reparse_older_versions(&scan_pool, user_id).await {
                tracing::warn!(kind = "archive_reparse_version", error = %error, "前の版で読んだ書庫を読み直せない");
            }
            if let Err(error) =
                ingest_confirmed_pending(&scan_pool, user_id, config.keep_copies).await
            {
                tracing::warn!(kind = "archive_reparse", error = %error, "確認待ちの書庫を読み直せない");
            }
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
            // **列に並んでいる間に読み終えた書庫は捨てる。**
            // 走査は読み手を待たないので（`scan_sec` ごとに回る）、1 冊に数分かかると
            // 同じ書庫が 2 度積まれる。2 度目はファイルが「取り込み済み」へ移った後に
            // 開かれ、**読めた書庫に `unreadable` の行が付いて画面が嘘をつく**。
            // **確認待ちの書庫も「覚えている」**（D16。final review R52）。
            match sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM core.archive_ledger
                   WHERE user_id = $1 AND sha256 = $2 AND parser_version = $3
                     AND outcome IN ('read', 'unreadable', 'pending_shape'))",
            )
            .bind(user_id)
            .bind(&sha256)
            .bind(super::PARSER_VERSION)
            .fetch_one(&pool)
            .await
            {
                Ok(true) => return,
                Ok(false) => {}
                // 台帳を引けないときは読みに進まない（次の走査でやり直す）。
                Err(_) => return,
            }
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
                            // **台帳に残せなければファイルを動かさない**（review I5）。
                            // 動かすと、置き場からも台帳からも画面からも消える。
                            if record_unreadable(
                                &pool,
                                user_id,
                                &sha256,
                                &candidate.path,
                                error.kind(),
                                candidate.from_downloads,
                            )
                            .await
                            .is_ok()
                                && !candidate.from_downloads
                            {
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
                            let shape = match shape_for_file(known.kind, &file.path, &file.bytes) {
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
                                    // **写し・目録・待ち行列のどれかを残せなければ、確認待ちの台帳を
                                    // 書かずに抜ける**（final review R51）。黙って捨てていたときは、
                                    // 写しの無い確認待ちが残り、印を置いても読み直せなかった。
                                    // 台帳を書かなければ次の走査で同じ書庫を最初から読み直す。
                                    let Ok((stored_path, created)) = copy_known_file_reporting(
                                        &read_config.copy_dir,
                                        &file.bytes,
                                    ) else {
                                        tracing::warn!(
                                            kind = "archive_copy",
                                            "確認待ちの写しを残せない"
                                        );
                                        return;
                                    };
                                    let Some(file_sha256) =
                                        stored_path.file_name().and_then(|name| name.to_str())
                                    else {
                                        return;
                                    };
                                    if record_copy(
                                        &pool,
                                        user_id,
                                        file_sha256.to_owned(),
                                        &file.path,
                                        &stored_path,
                                        &sha256,
                                    )
                                    .await
                                    .is_err()
                                        || record_pending_file(
                                            &pool,
                                            user_id,
                                            &sha256,
                                            &file.path,
                                            &shape,
                                            Some(file_sha256),
                                            // 残さない設定で、確認待ちのために作った写しだけが
                                            // 読み直しの後に消える（既に在った写しは残す）。
                                            created && !read_config.keep_copies,
                                        )
                                        .await
                                        .is_err()
                                    {
                                        tracing::warn!(
                                            kind = "archive_pending_shape",
                                            "確認待ちを残せない"
                                        );
                                        return;
                                    }
                                    if record_pending_ledger_at(
                                        &pool,
                                        user_id,
                                        sha256.clone(),
                                        file_name_of(&candidate.path),
                                        Some(archive_created_at(
                                            &candidate.path,
                                            chrono::Utc::now(),
                                        )),
                                        inbox_kind,
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
                                        &sha256,
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
                                    request.payload["product"]
                                        .as_str()
                                        .unwrap_or(&request.logical_source),
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
                        // 確認待ちだけの書庫。台帳（`pending_shape`）と写しは残したので、
                        // 専用のフォルダの書庫は「取り込み済み」へ移す（D9）。置いたままにすると
                        // 走査のたびに積まれ、書庫全体を展開し直していた（final review R52）。
                        if !candidate.from_downloads && move_to_processed(&candidate.path).is_err()
                        {
                            tracing::warn!(kind = "archive_move", "書庫を取り込み済みへ移せない");
                        }
                        return;
                    }
                    // **`read` の台帳の行より先に印を付ける**（final review 第 2 回 R72）。格納は commit 済みで
                    // 印付けは冪等。落ちたら台帳を書かずに書庫を置き場に残すので、次の走査が既読と見なさずに
                    // 読み直し（格納は内容の鍵で増えない）、印を付け直す。`read` の行を先に書いていたときは、
                    // 次の走査が既読として畳み、消した時間帯の位置が生きた記録のまま残った
                    // （取り込みは作り直しを起こさないので、他に印を付ける経路が無い）。
                    if crate::stay_store::mark_archive_arrivals(&pool, user_id, &stored_requests)
                        .await
                        .is_err()
                    {
                        tracing::warn!(
                            kind = "archive_mark_erased",
                            "消した時間帯の印を付けられない"
                        );
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
                    if record_unreadable(
                        &pool,
                        user_id,
                        &sha256,
                        &candidate.path,
                        error.kind(),
                        candidate.from_downloads,
                    )
                    .await
                    .is_ok()
                        && !candidate.from_downloads
                    {
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
