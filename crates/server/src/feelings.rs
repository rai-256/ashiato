// SPDX-License-Identifier: AGPL-3.0-only
//! 主観の記録の原文の解釈（ST17 / FR-36 / FR-37。design D1 / D4 / D5）。**DB に触らない。**
//!
//! 取り込み口（`ingest_request`）は、`logical_source = 's01-feeling'` のときだけここを通す
//! （ST19 の `attributes::parse_claim` と同じ差し込み方）。
//!
//! 解析済み（`payload`）は**送り主の値を使わず、原文から組み直す**。組み直した値に `nonce` は
//! 入れず、前/後を表す欄も持たない（前後は時刻から読み出しが決める。FR-37）。
use chrono::{DateTime, FixedOffset, NaiveDate};
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
