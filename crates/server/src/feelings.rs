// SPDX-License-Identifier: AGPL-3.0-only
//! 主観の記録の原文の解釈（ST17 / FR-36 / FR-37。design D1 / D4 / D5）。
//! `parse_feeling` は**DB に触らない**。DB を見る紐づけ先の検査は末尾の `target_state_error`。
//!
//! 取り込み口（`ingest_request`）は、`logical_source = 's01-feeling'` のときだけここを通す
//! （ST19 の `attributes::parse_claim` と同じ差し込み方）。
//!
//! 解析済み（`payload`）は**送り主の値を使わず、原文から組み直す**。組み直した値に `nonce` は
//! 入れず、前/後を表す欄も持たない（前後は時刻から読み出しが決める。FR-37）。
use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use unicode_normalization::UnicodeNormalization as _;

/// 主観の記録を置く論理ソース（design D1）。
pub const SOURCE: &str = "s01-feeling";

/// 既定の感度（ローカル AI まで。design D3（仮））。
///
/// **反転条件**: ST24 が登録簿に「ソースごとの既定の感度」を持たせたとき、値は 2 のまま
/// `default_sensitivity` の分岐を登録簿の値へ移す。
pub const DEFAULT_SENSITIVITY: i32 = 2;

/// 原文の乱数の最小の長さ（128 bit を base64url で書いた 22 文字。design D4）。
pub const NONCE_MIN_CHARS: usize = 22;

/// ひとことの上限。NFC にした Unicode のコードポイントの数（design D5 / 深掘り C10）。
pub const NOTE_MAX_CHARS: usize = 2000;

/// 対象の地域。出来事の時刻の地域もこれ（design D1）。
pub const TARGET_TZ: &str = "Asia/Tokyo";
/// `TARGET_TZ` の UTC からのずれ（分）。
pub const TARGET_TZ_OFFSET_MIN: i32 = 540;

/// 尺度の表（識別子, 版）。**知らない組は断る**（器は並びなので、足すときは 1 行足す。design D5）。
const SCALES: [(&str, i64); 2] = [("valence", 1), ("energy", 1)];
const SCALE_MIN: i64 = -2;
const SCALE_MAX: i64 = 2;

/// 解釈できなかった理由。取り込み口で理由の種別に写す（spec「形の合わない主観の記録は受け付けない」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeelingInvalid {
    /// 原文が JSON でない / 必須の欄が欠ける / 識別子が違う / 乱数が短い / 記入の日時が読めない
    Malformed,
    /// 快–不快が無い / 値が範囲の外 / 知らない尺度 / 同じ尺度が 2 つ
    Scale,
    /// ひとことが長すぎる
    Note,
    /// 紐づけ先の形が合わない（暦に無い日付・始まりが終わりより後・コードが検査を持たない種類）
    Target,
}

/// 紐づけ先。DB を見る検査（登録簿・滞在）はここに含まない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Day {
        date: NaiveDate,
    },
    Stay {
        stay: uuid::Uuid,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
    },
}

/// 原文を読んで形を確かめた主観の記録。
#[derive(Debug, Clone)]
pub struct Feeling {
    pub id: uuid::Uuid,
    pub target: Target,
    /// 解析済み。**`nonce` を含まない**
    pub payload: serde_json::Value,
}

fn nfc(s: &str) -> String {
    s.nfc().collect()
}

/// 原文を読み、形を確かめて主観の記録にする（design D5）。
///
/// `id` は**記録の**識別子。原文の `feeling` がこれと違えば断る。
/// 順は 形 → 尺度 → ひとこと → 紐づけ先（spec の表の上から）。
pub fn parse_feeling(raw: &str, id: uuid::Uuid) -> Result<Feeling, FeelingInvalid> {
    use FeelingInvalid::{Malformed, Note, Scale, Target as BadTarget};

    let v: serde_json::Value = serde_json::from_str(raw).map_err(|_| Malformed)?;
    let obj = v.as_object().ok_or(Malformed)?;

    // --- 形（必須の欄・識別子・乱数・記入の日時）
    let feeling_id = obj
        .get("feeling")
        .and_then(|x| x.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(Malformed)?;
    if feeling_id != id {
        return Err(Malformed);
    }
    // **乱数は長さと文字種だけを見る**（中身は画面が `crypto.getRandomValues` で作る。サーバは作らない）
    let nonce = obj.get("nonce").and_then(|x| x.as_str()).ok_or(Malformed)?;
    if nonce.chars().count() < NONCE_MIN_CHARS
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(Malformed);
    }
    let written_at = obj
        .get("written_at")
        .and_then(|x| x.as_str())
        .ok_or(Malformed)?;
    DateTime::parse_from_rfc3339(written_at).map_err(|_| Malformed)?;
    let written_tz = obj
        .get("written_tz")
        .and_then(|x| x.as_str())
        .ok_or(Malformed)?;
    // 解析済み（`jsonb`）に入る欄なので、`\u0000` などの制御文字は INSERT を落とす前に断る（ひとこと・:150 と同じ穴）
    if written_tz.chars().any(char::is_control) {
        return Err(Malformed);
    }
    let scales_raw = obj
        .get("scales")
        .and_then(|x| x.as_array())
        .ok_or(Malformed)?;
    let target_raw = obj
        .get("target")
        .and_then(|x| x.as_object())
        .ok_or(Malformed)?;
    let note_raw = match obj.get("note") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(nfc(s)),
        Some(_) => return Err(Malformed),
    };

    // --- 尺度（快–不快は必須。同じ識別子は 2 つ持てない）
    let mut scales = Vec::with_capacity(scales_raw.len());
    let mut seen: Vec<&str> = Vec::new();
    for s in scales_raw {
        let o = s.as_object().ok_or(Scale)?;
        let sid = o.get("id").and_then(|x| x.as_str()).ok_or(Scale)?;
        let version = o.get("version").and_then(|x| x.as_i64()).ok_or(Scale)?;
        let value = o.get("value").and_then(|x| x.as_i64()).ok_or(Scale)?;
        if !SCALES.contains(&(sid, version)) || !(SCALE_MIN..=SCALE_MAX).contains(&value) {
            return Err(Scale);
        }
        if seen.contains(&sid) {
            return Err(Scale);
        }
        seen.push(sid);
        scales.push(serde_json::json!({ "id": sid, "version": version, "value": value }));
    }
    if !seen.contains(&"valence") {
        return Err(Scale);
    }

    // --- ひとこと（前後の空白を除いて空なら「無し」。切り詰めない）
    let note = note_raw.filter(|s| !s.trim().is_empty());
    if let Some(n) = &note {
        // **制御文字を断る** —— 原文は JSON のテキストなので `\u0000` は NUL バイトに見えず、
        // 解析済み（`jsonb`）の INSERT が落ちてまとめ送り全体が 500 になる（ST19 の R14 と同じ）
        if n.chars()
            .any(|c| c == '\0' || (c.is_control() && c != '\n' && c != '\t'))
            || n.chars().count() > NOTE_MAX_CHARS
        {
            return Err(Note);
        }
    }

    // --- 紐づけ先（形だけ。登録簿と滞在は DB を見る側）
    let kind = target_raw
        .get("kind")
        .and_then(|x| x.as_str())
        .ok_or(BadTarget)?;
    let (target, target_json) = match kind {
        "day" => {
            let date = target_raw
                .get("date")
                .and_then(|x| x.as_str())
                .ok_or(BadTarget)?;
            let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| BadTarget)?;
            // `2026-1-7` のような崩れた書き方を通さない（日の鍵は 1 つの書き方に揃える）
            if parsed.format("%Y-%m-%d").to_string() != date {
                return Err(BadTarget);
            }
            (
                Target::Day { date: parsed },
                serde_json::json!({ "kind": "day", "date": date }),
            )
        }
        "stay" => {
            let stay = target_raw
                .get("stay")
                .and_then(|x| x.as_str())
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                .ok_or(BadTarget)?;
            let time = |key: &str| {
                target_raw
                    .get(key)
                    .and_then(|x| x.as_str())
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|t| (s, t)))
                    .ok_or(BadTarget)
            };
            let (start_s, start) = time("start")?;
            let (end_s, end) = time("end")?;
            if start > end {
                return Err(BadTarget);
            }
            (
                Target::Stay { stay, start, end },
                serde_json::json!({ "kind": "stay", "stay": stay, "start": start_s, "end": end_s }),
            )
        }
        _ => return Err(BadTarget),
    };

    Ok(Feeling {
        id: feeling_id,
        target,
        payload: serde_json::json!({
            "feeling": feeling_id,
            "target": target_json,
            "written_at": written_at,
            "written_tz": nfc(written_tz),
            "scales": scales,
            "note": note,
        }),
    })
}

/// 同じ記録（内容の一致）が既に入っているか。再送は紐づけ先の検査をやり直さない
/// （後から滞在が消えても、受理した記録を「断られた」に変えない）。
pub async fn is_resend(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
    content_hash: &str,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM core.event
          WHERE user_id = $1 AND logical_source = $2 AND content_hash = $3)",
    )
    .bind(user_id)
    .bind(SOURCE)
    .bind(content_hash)
    .fetch_one(&mut **tx)
    .await
}

/// 紐づけ先の DB を見る検査（design D5。順は 登録簿 → 滞在 → 区間の重なり → 出来事の時刻）。
/// 当たれば `Some(FeelingInvalid::Target)`。**削除の印は見ない**（削除の印の付いた滞在・吸収された滞在は通す）。
/// 本文に終わりを持たない（読めない）滞在は、読み方（ST16 の `span_of`）と同じく始まりの一点とみなす。
/// 記録の区間が始まりを含めば通し、始まりより後から始まる記録は断る（ch01-st17-review-fixes D9。
/// 製品に滞在の本文を消す経路はまだ無い。消した滞在への付け方は ST23 が決める —— `docs/handoff/ST23.md`）。
/// 暦と形は `parse_feeling` が済ませてある。
pub async fn target_state_error(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
    target: &Target,
    event_time: DateTime<chrono::Utc>,
) -> sqlx::Result<Option<FeelingInvalid>> {
    let bad = Some(FeelingInvalid::Target);
    let kind = match target {
        Target::Day { .. } => "day",
        Target::Stay { .. } => "stay",
    };
    // 登録簿に行があり、かつコードがその種類の検査を持つ（上の 2 種類）ときだけ受ける
    let registered: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM core.feeling_target_kind WHERE kind = $1)",
    )
    .bind(kind)
    .fetch_one(&mut **tx)
    .await?;
    if !registered {
        return Ok(bad);
    }
    match target {
        Target::Day { date } => {
            // 日の鍵は `Asia/Tokyo` のその日の 0 時
            let tz = FixedOffset::east_opt(TARGET_TZ_OFFSET_MIN * 60).expect("+09:00");
            let midnight = date
                .and_hms_opt(0, 0, 0)
                .and_then(|t| t.and_local_timezone(tz).single());
            if midnight.map(|m| m.with_timezone(&chrono::Utc)) != Some(event_time) {
                return Ok(bad);
            }
        }
        Target::Stay { stay, start, end } => {
            let row: Option<(DateTime<chrono::Utc>, serde_json::Value)> = sqlx::query_as(
                "SELECT event_time, payload FROM core.event
                  WHERE id = $1 AND user_id = $2 AND logical_source = 's01-stay'",
            )
            .bind(stay)
            .bind(user_id)
            .fetch_optional(&mut **tx)
            .await?;
            let Some((stay_start, payload)) = row else {
                return Ok(bad);
            };
            let stay_end = payload
                .get("end")
                .and_then(serde_json::Value::as_str)
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map_or(stay_start, |t| t.with_timezone(&chrono::Utc));
            // 区間が重なる（端が触れるのも重なり）。一致までは求めない
            if start.with_timezone(&chrono::Utc) > stay_end
                || stay_start > end.with_timezone(&chrono::Utc)
            {
                return Ok(bad);
            }
            if start.with_timezone(&chrono::Utc) != event_time {
                return Ok(bad);
            }
        }
    }
    Ok(None)
}

// ================================================================ 読み出し（design D6 / D7 / D8）

/// 1 回の読み出しの範囲の上限（日数。両端を含む）。S-3 の暦（30 日）と S-2 の 1 日を 1 回で読める大きさに余裕を持たせた値（design D8）。
pub const RANGE_MAX_DAYS: i64 = 62;

/// 吸収の連鎖をたどる回数の上限。着かなければ表示しない側に回す（design D6）。
pub const ABSORB_HOPS_MAX: usize = 16;

/// 前に書いたか後に書いたか。**保存せず、読むたびに計算する**（FR-37 / design D7）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Before,
    After,
}

/// 日の状態（design D7 / D8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DayStatus {
    /// 日にも滞在にも、生きた記録が 1 件以上ある（表示しない滞在の記録も数える）
    Recorded,
    /// 使い始めた後の今日以前で、記録が無い
    Missing,
    /// 最初の記録の日より前
    BeforeStart,
    /// 今日より後（「これから」。未記入に数えない）
    Future,
}

/// 記録の紐づけ先。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    Day,
    /// `original` は書いたときの滞在。`shown` は読み出しに出す滞在（吸収をたどった先）、
    /// 表示しない側（本人が消した・たどり着かない）は `None`
    Stay {
        original: uuid::Uuid,
        shown: Option<uuid::Uuid>,
        /// 原文の `target.end`（前/後の境）
        end: DateTime<Utc>,
    },
}

/// 読み出す主観の記録 1 件（生きていて、本文が消されていないもの）。
#[derive(Debug, Clone)]
pub struct Record {
    pub id: uuid::Uuid,
    pub placement: Placement,
    /// 出来事の時刻（日は `Asia/Tokyo` の 0 時、滞在は始まり）。記録が紐づく日はこの `Asia/Tokyo` の日付
    pub event_time: DateTime<Utc>,
    /// 記入の日時（原文のまま。地域のずれつき）
    pub written_at: String,
    pub ingested_at: DateTime<Utc>,
    pub scales: Vec<ScaleOut>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ScaleOut {
    pub id: String,
    pub version: i64,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct FeelingOut {
    pub id: uuid::Uuid,
    pub scales: Vec<ScaleOut>,
    pub note: Option<String>,
    /// 記入の日時（RFC 3339。地域のずれつき）
    pub written_at: String,
    /// D-01 に入った時刻
    pub ingested_at: DateTime<Utc>,
    pub phase: Phase,
    /// 吸収された滞在に書いた記録のとき、元の滞在
    pub target_stay: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Default, Serialize, utoipa::ToSchema)]
pub struct Latest {
    pub before: Option<FeelingOut>,
    pub after: Option<FeelingOut>,
}

#[derive(Debug, Clone, Default, Serialize, utoipa::ToSchema)]
pub struct Bucket {
    pub latest: Latest,
    /// 記入の日時の新しい順
    pub records: Vec<FeelingOut>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StayBucket {
    pub stay: uuid::Uuid,
    pub latest: Latest,
    pub records: Vec<FeelingOut>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DayFeelings {
    pub date: NaiveDate,
    pub status: DayStatus,
    pub day: Bucket,
    /// 記録のある滞在だけ（表示しない側は含まない）
    pub stays: Vec<StayBucket>,
    /// 記録のある滞在の数（書き直しは 1。表示しない滞在も数える）
    pub stays_with_records: u32,
    /// 表示しない滞在の記録の件数
    pub hidden_stay_records: u32,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct FeelingsResponse {
    pub today: NaiveDate,
    pub first_day: Option<NaiveDate>,
    pub days: Vec<DayFeelings>,
}

/// `Asia/Tokyo` の日付。
pub fn jst_date(t: DateTime<Utc>) -> NaiveDate {
    crate::stay_store::jst_date(t)
}

/// 前/後。対象の終わりより前に書いていれば前（design D7）。日の対象の終わりは翌日 0 時 `Asia/Tokyo`。
pub fn phase_of(written_at: DateTime<Utc>, target_end: DateTime<Utc>) -> Phase {
    if written_at < target_end {
        Phase::Before
    } else {
        Phase::After
    }
}

/// 吸収をたどって、表示する（生きている）滞在に着いたらその識別子を返す。
/// `marks` に無い・本人が消した・範囲を消して隠れた・たどり着かない（`ABSORB_HOPS_MAX` 回まで）は `None`（design D6）。
pub(crate) fn resolve_stay(
    start: uuid::Uuid,
    marks: &HashMap<uuid::Uuid, crate::stay_store::Mark>,
    into: &HashMap<uuid::Uuid, Option<uuid::Uuid>>,
) -> Option<uuid::Uuid> {
    use crate::stay_store::Mark;
    let mut cur = start;
    for _ in 0..=ABSORB_HOPS_MAX {
        match marks.get(&cur)? {
            Mark::Live => return Some(cur),
            Mark::ErasedRange | Mark::UserDeleted => return None,
            Mark::Absorbed => cur = (*into.get(&cur)?)?,
        }
    }
    None
}

fn newest_first(
    a: &(DateTime<FixedOffset>, &Record),
    b: &(DateTime<FixedOffset>, &Record),
) -> std::cmp::Ordering {
    (b.0, b.1.ingested_at, b.1.id).cmp(&(a.0, a.1.ingested_at, a.1.id))
}

/// 記録を（紐づけ先・前/後）ごとの最新と履歴にする。`items` は (前/後, 記録, 元の滞在)。
fn bucket(items: Vec<StayItem<'_>>) -> (Latest, Vec<FeelingOut>) {
    let mut sorted: Vec<_> = items
        .into_iter()
        .map(|(p, r, orig)| {
            let w = DateTime::parse_from_rfc3339(&r.written_at)
                .unwrap_or_else(|_| r.event_time.fixed_offset());
            ((w, r), p, orig)
        })
        .collect();
    sorted.sort_by(|a, b| newest_first(&a.0, &b.0));
    let out: Vec<(Phase, FeelingOut)> = sorted
        .into_iter()
        .map(|((_, r), phase, orig)| {
            (
                phase,
                FeelingOut {
                    id: r.id,
                    scales: r.scales.clone(),
                    note: r.note.clone(),
                    written_at: r.written_at.clone(),
                    ingested_at: r.ingested_at,
                    phase,
                    target_stay: orig,
                },
            )
        })
        .collect();
    let first = |p: Phase| out.iter().find(|(q, _)| *q == p).map(|(_, f)| f.clone());
    let latest = Latest {
        before: first(Phase::Before),
        after: first(Phase::After),
    };
    (latest, out.into_iter().map(|(_, f)| f).collect())
}

/// (前/後, 記録, 吸収された元の滞在)
type StayItem<'a> = (Phase, &'a Record, Option<uuid::Uuid>);

/// 範囲 `from..=to` の日ごとの主観を組む。**DB に触らない**（design D6 / D7）。
///
/// - `records` は範囲の日に紐づく、生きた記録（本文が消えたものを含まない）
/// - `first_day` は利用者の生きた主観で最も早い記録の日（範囲の外も含む）。無ければ `None`
/// - `today` は `Asia/Tokyo` の今日
pub fn day_feelings(
    records: &[Record],
    first_day: Option<NaiveDate>,
    today: NaiveDate,
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<DayFeelings> {
    let mut by_day: BTreeMap<NaiveDate, Vec<&Record>> = BTreeMap::new();
    for r in records {
        by_day.entry(jst_date(r.event_time)).or_default().push(r);
    }
    let mut days = Vec::new();
    let mut date = from;
    while date <= to {
        let recs = by_day.remove(&date).unwrap_or_default();
        let status = if date > today {
            DayStatus::Future
        } else if first_day.is_none_or(|f| date < f) {
            DayStatus::BeforeStart
        } else if recs.is_empty() {
            DayStatus::Missing
        } else {
            DayStatus::Recorded
        };

        let day_end = crate::stay_store::day_bounds(date).1;
        let mut day_items = Vec::new();
        let mut by_stay: BTreeMap<uuid::Uuid, Vec<StayItem<'_>>> = BTreeMap::new();
        let mut hidden_originals = std::collections::BTreeSet::new();
        let mut hidden_stay_records = 0u32;
        for r in recs {
            let written = DateTime::parse_from_rfc3339(&r.written_at)
                .map_or(r.event_time, |w| w.with_timezone(&Utc));
            match &r.placement {
                Placement::Day => day_items.push((phase_of(written, day_end), r, None)),
                Placement::Stay {
                    original,
                    shown,
                    end,
                } => {
                    let phase = phase_of(written, *end);
                    match shown {
                        Some(s) => by_stay.entry(*s).or_default().push((
                            phase,
                            r,
                            (s != original).then_some(*original),
                        )),
                        None => {
                            hidden_originals.insert(*original);
                            hidden_stay_records += 1;
                        }
                    }
                }
            }
        }
        let (latest, records) = bucket(day_items);
        // 滞在は最も早い記録の出来事の時刻の順
        let mut grouped: Vec<_> = by_stay.into_iter().collect();
        grouped
            .sort_by_key(|(stay, items)| (items.iter().map(|(_, r, _)| r.event_time).min(), *stay));
        let stays_with_records = (grouped.len() + hidden_originals.len()) as u32;
        let stays = grouped
            .into_iter()
            .map(|(stay, items)| {
                let (latest, records) = bucket(items);
                StayBucket {
                    stay,
                    latest,
                    records,
                }
            })
            .collect();
        days.push(DayFeelings {
            date,
            status,
            day: Bucket { latest, records },
            stays,
            stays_with_records,
            hidden_stay_records,
        });
        date += chrono::Duration::days(1);
    }
    days
}

/// 範囲の日に紐づく主観を DB から読み、滞在の記録は吸収をたどって `Record` にする（design D6）。
/// 読むのは生きた行（`event_live`）で本文が消えていないものだけ。**感度では絞らない。**
pub async fn load_response(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    from: NaiveDate,
    to: NaiveDate,
    now: DateTime<Utc>,
) -> sqlx::Result<FeelingsResponse> {
    use crate::stay_store::Mark;
    let (lo, _) = crate::stay_store::day_bounds(from);
    let (_, hi) = crate::stay_store::day_bounds(to);
    let rows: Vec<(uuid::Uuid, DateTime<Utc>, DateTime<Utc>, serde_json::Value)> = sqlx::query_as(
        "SELECT id, event_time, ingest_time, payload FROM core.event_live
          WHERE user_id = $1 AND logical_source = $2 AND raw <> ''
            AND event_time >= $3 AND event_time < $4",
    )
    .bind(user_id)
    .bind(SOURCE)
    .bind(lo)
    .bind(hi)
    .fetch_all(pool)
    .await?;
    // 始まりの日。既存の索引 `event_by_source_time (logical_source, event_time)`（移行 202609112113）に乗り、
    // 利用者では絞らないが先頭の 1 行を読むだけで済む。合成データ（位置 100 万行・主観 500 行）で 4 ページ・0.079 ms
    // （premises.md 前提 9）。利用者が複数になり他人の主観が古い場合に読みが増えると実測で分かったら、
    // 利用者の列を先頭に持つ索引を足す（ch01-st17-review-fixes D10）。
    let first: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT min(event_time) FROM core.event_live
          WHERE user_id = $1 AND logical_source = $2 AND raw <> ''",
    )
    .bind(user_id)
    .bind(SOURCE)
    .fetch_one(pool)
    .await?;

    // 滞在の記録の紐づけ先をたどる。1 往復ごとに 1 ホップ進める
    let originals: Vec<uuid::Uuid> = rows
        .iter()
        .filter_map(|(_, _, _, p)| stay_of(p).map(|(s, _)| s))
        .collect();
    let mut marks: HashMap<uuid::Uuid, Mark> = HashMap::new();
    let mut into: HashMap<uuid::Uuid, Option<uuid::Uuid>> = HashMap::new();
    let mut frontier = originals;
    for _ in 0..=ABSORB_HOPS_MAX {
        frontier.retain(|id| !marks.contains_key(id));
        frontier.sort();
        frontier.dedup();
        if frontier.is_empty() {
            break;
        }
        let found: Vec<(uuid::Uuid, Option<DateTime<Utc>>, Option<String>)> = sqlx::query_as(
            "SELECT id, deleted_at, deleted_by FROM core.event
              WHERE user_id = $1 AND logical_source = 's01-stay' AND id = ANY($2)",
        )
        .bind(user_id)
        .bind(&frontier)
        .fetch_all(pool)
        .await?;
        let mut absorbed = Vec::new();
        for (id, at, by) in found {
            let m = Mark::of(at, by.as_deref());
            if m == Mark::Absorbed {
                absorbed.push(id);
            }
            marks.insert(id, m);
        }
        // 見つからなかった行は印を持たない（`resolve_stay` が表示しない側に回す）
        let next: Vec<(uuid::Uuid, Option<uuid::Uuid>)> = sqlx::query_as(
            "SELECT DISTINCT ON (event_id) event_id, into_event_id FROM core.stay_absorbed
              WHERE user_id = $1 AND event_id = ANY($2) ORDER BY event_id, id DESC",
        )
        .bind(user_id)
        .bind(&absorbed)
        .fetch_all(pool)
        .await?;
        frontier = Vec::new();
        for (e, i) in next {
            into.insert(e, i);
            frontier.extend(i);
        }
    }

    // 解析済みの形が読めない行（取り込み口の検査を通ったものなので、本来は無い）は読み飛ばす。
    // 黙って捨てると画面の「未記入」や件数が静かにずれるので、捨てた件数だけを記録に残す（値・本文は出さない）
    let total = rows.len();
    let records: Vec<Record> = rows
        .into_iter()
        .filter_map(|(id, event_time, ingested_at, p)| {
            let written_at = p.get("written_at")?.as_str()?.to_string();
            let placement = match stay_of(&p) {
                None => Placement::Day,
                Some((original, end)) => Placement::Stay {
                    original,
                    shown: resolve_stay(original, &marks, &into),
                    end,
                },
            };
            let scales = p
                .get("scales")?
                .as_array()?
                .iter()
                .filter_map(|s| {
                    Some(ScaleOut {
                        id: s.get("id")?.as_str()?.to_string(),
                        version: s.get("version")?.as_i64()?,
                        value: s.get("value")?.as_i64()?,
                    })
                })
                .collect();
            let note = p.get("note").and_then(|n| n.as_str()).map(str::to_string);
            Some(Record {
                id,
                placement,
                event_time,
                written_at,
                ingested_at,
                scales,
                note,
            })
        })
        .collect();
    let dropped = total - records.len();
    if dropped > 0 {
        tracing::warn!(
            kind = "feeling_unreadable",
            dropped,
            "解析済みの形が読めない主観の行を読み出しから外した"
        );
    }

    let today = jst_date(now);
    let first_day = first.map(jst_date);
    Ok(FeelingsResponse {
        today,
        first_day,
        days: day_feelings(&records, first_day, today, from, to),
    })
}

/// 解析済みの滞在の紐づけ先 `(滞在, 原文の target.end)`。日なら `None`。
fn stay_of(payload: &serde_json::Value) -> Option<(uuid::Uuid, DateTime<Utc>)> {
    let t = payload.get("target")?;
    if t.get("kind")?.as_str()? != "stay" {
        return None;
    }
    let stay = uuid::Uuid::parse_str(t.get("stay")?.as_str()?).ok()?;
    let end = DateTime::parse_from_rfc3339(t.get("end")?.as_str()?).ok()?;
    Some((stay, end.with_timezone(&Utc)))
}

// ================================================================ 通知の時刻と通知の判断（design D9）

/// 通知の時刻の既定（`HH:MM`。Q7）。台帳に行が無い利用者はこれ。
pub const DEFAULT_REMINDER_AT: &str = "21:00";

/// `"HH:MM"` を時刻にする。秒は持たない（design D9）。`24:10`・`9:05`・`21:00:00` は読めない。
pub fn parse_reminder_at(s: &str) -> Option<chrono::NaiveTime> {
    let b = s.as_bytes();
    let digits = |i: usize| b[i].is_ascii_digit();
    if b.len() != 5 || b[2] != b':' || !(digits(0) && digits(1) && digits(3) && digits(4)) {
        return None;
    }
    chrono::NaiveTime::parse_from_str(s, "%H:%M").ok()
}

/// 通知の判断に要るもの。**この 2 欄だけ**（主観の中身を返さない。design D9）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ReminderDecision {
    /// 通知の時刻（`HH:MM`）
    pub at: String,
    /// その日に紐づく記録が 1 件以上あるか（削除の印・消去を除く）
    pub recorded: bool,
}

/// 台帳に 1 行足した結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ReminderSet {
    pub at: String,
}

/// 通知の時刻を台帳に 1 行足す（追記のみ）。
pub async fn set_reminder(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    at: chrono::NaiveTime,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO core.feeling_reminder (user_id, at, server_version) VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(at)
    .bind(crate::BUILD_STAMP)
    .execute(pool)
    .await
    .map(|_| ())
}

/// いまの通知の時刻（利用者ごとに `id` が最大の行。無ければ既定）と、その日の記録の有無。
pub async fn reminder_decision(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    date: NaiveDate,
) -> sqlx::Result<ReminderDecision> {
    let at: Option<chrono::NaiveTime> = sqlx::query_scalar(
        "SELECT at FROM core.feeling_reminder WHERE user_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let (lo, hi) = crate::stay_store::day_bounds(date);
    let recorded: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM core.event_live
          WHERE user_id = $1 AND logical_source = $2 AND raw <> ''
            AND event_time >= $3 AND event_time < $4)",
    )
    .bind(user_id)
    .bind(SOURCE)
    .bind(lo)
    .bind(hi)
    .fetch_one(pool)
    .await?;
    Ok(ReminderDecision {
        at: at.map_or_else(
            || DEFAULT_REMINDER_AT.to_string(),
            |t| t.format("%H:%M").to_string(),
        ),
        recorded,
    })
}
