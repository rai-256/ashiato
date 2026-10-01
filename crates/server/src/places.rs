// SPDX-License-Identifier: AGPL-3.0-only
//! 場所の器（ST21 / design D1）。器は記録ではなく、記録を束ねる識別子。
//!
//! 識別子は画面が乱数で決めて渡す。サーバは名前・座標から計算しない（C1）。
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use unicode_normalization::UnicodeNormalization as _;
use uuid::Uuid;

use crate::attributes::ValidFrom;

/// 場所の記録を置く論理ソース（design D2）。
pub const SOURCE: &str = "s01-place";

/// 既定の感度（外部 AI に出してよい。深掘り Q3 / design D11（仮））。
///
/// **DB の既定と同じ値でも、`default_sensitivity` の分岐として明示する**（黙って既定に乗らない）。
/// **反転条件**: ST24 が登録簿に「ソースごとの既定の感度」を持たせたとき、値は 1 のまま分岐を移す。
pub const DEFAULT_SENSITIVITY: i32 = 1;

/// 広さの範囲（m。design D4（仮））。画面の選択肢（50 / 100 / 200 / 300）より広く取り、
/// API から書く経路を狭めない。**反転条件**: 5 km より広い場所を持ちたいとき（範囲を広げるだけ）。
pub const PLACE_RADIUS_MIN_M: i64 = 10;
pub const PLACE_RADIUS_MAX_M: i64 = 5000;

/// 場所ごとの錠の名前空間（`pg_advisory_xact_lock(key, hashtext(place))` の 1 つ目）。
/// 種類の錠（4_819_019）・滞在の作り直し（4_816_016）・移行（4_820_251）とは別の空間。
pub(crate) const LOCK_KEY: i32 = 4_821_021;

/// 座標の変え方（FR-49 ★ / 深掘り Q2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordChange {
    /// 登録のときの 1 件だけ
    First,
    /// 間違いを直す（直す先を持ち、「いつから」を持たない）
    Fix,
    /// 移った（「いつから」を持ち、直す先を持たない）
    Move,
}

impl CoordChange {
    fn as_str(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Fix => "fix",
            Self::Move => "move",
        }
    }
}

/// 場所の記録が書く 1 項目（design D2。1 記録 1 項目）。
#[derive(Debug, Clone, PartialEq)]
pub enum PlaceField {
    Name(String),
    Coord {
        lat: f64,
        lon: f64,
        change: CoordChange,
        valid_from: Option<ValidFrom>,
        supersedes: Option<Uuid>,
    },
    Radius(i64),
    Note(Option<String>),
}

/// 原文から読んだ 1 件の場所の記録。
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceRecord {
    /// 記録の識別子（**記録の `id` と同じでなければならない**）
    pub id: Uuid,
    /// 指す器
    pub place: Uuid,
    pub field: PlaceField,
    /// **原文から `nonce` を除いて組み直した解析済み**（文字列は NFC。design D3）。
    /// 送り主の `payload` は使わない。**乱数をここへ写さない** —— 写すと消去でしか消えず、
    /// 列に残った乱数から鍵を作り直せてしまう。
    pub payload: serde_json::Value,
}

/// 場所の記録の形が合わない理由（design D4）。`IngestError` へ写す。
/// 由来・外部識別子・DB を見る検査は `ingest_one` が持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceRecordInvalid {
    Malformed,
    Name,
    Coordinate,
    CoordChange,
    ValidFrom,
    Radius,
}

/// 文字列に、`jsonb` に入らない・画面に出せない制御文字があるか（改行とタブは通す）。
/// 原文は JSON の**テキスト**なので `\u0000` は NUL バイトにならず、`validate` の NUL 検査をすり抜ける。
fn has_control(s: &str) -> bool {
    s.chars()
        .any(|c| c == '\0' || (c.is_control() && c != '\n' && c != '\t'))
}

fn uuid_field(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<Uuid> {
    obj.get(key)
        .and_then(|x| x.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// 原文を読み、形を確かめて場所の記録にする（design D4 / tasks 3.1）。
///
/// `id` は**記録の**識別子。原文の `record` がこれと違えば断る。
/// 「いつから」の形と暦は ST19 の `attributes` の解釈を呼ぶ（同じ規則を 2 か所に書かない）。
/// 座標系（`crs`）は原文の外（エンベロープ）にあるので、呼び出し側が見る。
pub fn parse_place_record(raw: &str, id: Uuid) -> Result<PlaceRecord, PlaceRecordInvalid> {
    use PlaceRecordInvalid as Bad;
    let v: serde_json::Value = serde_json::from_str(raw).map_err(|_| Bad::Malformed)?;
    let obj = v.as_object().ok_or(Bad::Malformed)?;

    // --- 必須の欄（欠ければ `malformed_place_record`）
    if uuid_field(obj, "record") != Some(id) {
        return Err(Bad::Malformed);
    }
    let place = uuid_field(obj, "place").ok_or(Bad::Malformed)?;
    // **乱数は長さと文字種だけを見る**（中身は画面が作る。サーバは作らない。design D3）
    let nonce = obj
        .get("nonce")
        .and_then(|x| x.as_str())
        .ok_or(Bad::Malformed)?;
    if nonce.chars().count() < crate::attributes::NONCE_MIN_CHARS
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(Bad::Malformed);
    }
    let kind = obj
        .get("field")
        .and_then(|x| x.as_str())
        .ok_or(Bad::Malformed)?;

    let (field, body) = match kind {
        "name" => {
            let name: String = obj
                .get("name")
                .and_then(|x| x.as_str())
                .ok_or(Bad::Malformed)?
                .nfc()
                .collect();
            if name.trim().is_empty() || has_control(&name) {
                return Err(Bad::Name);
            }
            let body = serde_json::json!({ "name": name });
            (PlaceField::Name(name), body)
        }
        "coord" => parse_coord(obj)?,
        "radius" => {
            let radius = obj.get("radius_m").ok_or(Bad::Malformed)?;
            let m = radius.as_i64().ok_or(Bad::Radius)?;
            if !(PLACE_RADIUS_MIN_M..=PLACE_RADIUS_MAX_M).contains(&m) {
                return Err(Bad::Radius);
            }
            (PlaceField::Radius(m), serde_json::json!({ "radius_m": m }))
        }
        "note" => {
            // 欄そのものが要る（`null` が「補足なし」なので、欠落と区別する）
            let note = match obj.get("note").ok_or(Bad::Malformed)? {
                serde_json::Value::Null => None,
                serde_json::Value::String(s) => Some(s.nfc().collect::<String>()),
                _ => return Err(Bad::Malformed),
            };
            if note.as_deref().is_some_and(has_control) {
                return Err(Bad::Malformed);
            }
            let body = serde_json::json!({ "note": note });
            (PlaceField::Note(note), body)
        }
        // 名前・座標・広さ・補足のどれでもない
        _ => return Err(Bad::Malformed),
    };

    let mut payload = serde_json::json!({ "record": id, "place": place, "field": kind });
    for (k, val) in body.as_object().into_iter().flatten() {
        payload[k] = val.clone();
    }
    Ok(PlaceRecord {
        id,
        place,
        field,
        payload,
    })
}

fn parse_coord(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<(PlaceField, serde_json::Value), PlaceRecordInvalid> {
    use PlaceRecordInvalid as Bad;
    // 欄が無ければ形の誤り、あっても数でない・範囲の外なら座標の誤り
    let number = |key: &str, limit: f64| -> Result<f64, Bad> {
        let x = obj.get(key).ok_or(Bad::Malformed)?;
        x.as_f64()
            .filter(|n| n.abs() <= limit)
            .ok_or(Bad::Coordinate)
    };
    let lat = number("lat", 90.0)?;
    let lon = number("lon", 180.0)?;

    let change_text = obj
        .get("change")
        .and_then(|x| x.as_str())
        .ok_or(Bad::Malformed)?;
    let valid_from = match obj.get("valid_from") {
        None | Some(serde_json::Value::Null) => None,
        Some(vf) => Some(ValidFrom::from_json(vf).map_err(|_| Bad::Malformed)?),
    };
    let supersedes = match obj.get("supersedes") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(Uuid::parse_str(s).map_err(|_| Bad::Malformed)?),
        Some(_) => return Err(Bad::Malformed),
    };

    // 変え方と欄の組（spec の表の `invalid_coord_change`）
    let change = match (change_text, &valid_from, &supersedes) {
        ("first", None, None) => CoordChange::First,
        ("fix", None, Some(_)) => CoordChange::Fix,
        ("move", Some(_), None) => CoordChange::Move,
        _ => return Err(Bad::CoordChange),
    };
    if valid_from.as_ref().is_some_and(|vf| !vf.is_well_formed()) {
        return Err(Bad::ValidFrom);
    }

    let body = serde_json::json!({
        "lat": lat,
        "lon": lon,
        "change": change.as_str(),
        "valid_from": valid_from.as_ref().map(|vf| serde_json::json!({
            "precision": vf.precision,
            "date": vf.date,
        })),
        "supersedes": supersedes,
    });
    Ok((
        PlaceField::Coord {
            lat,
            lon,
            change,
            valid_from,
            supersedes,
        },
        body,
    ))
}

// ------------------------------------------------------------------ 取り込み口の DB を見る検査（D4）

/// 場所ごとの錠を取る。**トランザクションの終わりまで**（`_xact_`）。
/// 座標の記録の先頭で呼ぶ —— `first` を同時に 2 本送ったとき、両方が「まだ座標が無い」を見て
/// 2 本入るのを止める。
pub async fn lock_place(tx: &mut Transaction<'_, Postgres>, place: Uuid) -> sqlx::Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text))")
        .bind(LOCK_KEY)
        .bind(place)
        .execute(&mut **tx)
        .await
        .map(|_| ())
}

/// その器がその利用者のものか。
pub async fn place_exists(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    place: Uuid,
) -> sqlx::Result<bool> {
    let (found,): (bool,) =
        sqlx::query_as("SELECT EXISTS (SELECT 1 FROM core.place WHERE id = $1 AND user_id = $2)")
            .bind(place)
            .bind(user_id)
            .fetch_one(&mut **tx)
            .await?;
    Ok(found)
}

/// その内容の鍵の場所の記録が既にあるか（**再送**）。削除の印の付いた行も数える
/// （`event_dedup_hash` が畳むのと同じ範囲）。
///
/// 再送は、いまの座標の有無で断らない —— 「初めての座標」を送ったあと応答を取り落とした端末の
/// 押し直しが `invalid_coord_change` で恒久的に拒まれてしまう。
pub async fn is_resend(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    content_hash: &str,
) -> sqlx::Result<bool> {
    let (found,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM core.event
          WHERE user_id = $1 AND logical_source = $2 AND content_hash = $3)",
    )
    .bind(user_id)
    .bind(SOURCE)
    .bind(content_hash)
    .fetch_one(&mut **tx)
    .await?;
    Ok(found)
}

/// その場所が座標の記録を 1 件でも持つか。**削除の印の付いたものも数える**（spec）。
///
/// 本文を消去した記録は `payload = '{}'` で、どの場所のどの項目だったかが残らない
/// （FR-51）ので、ここでは数えられない。
pub async fn has_coord_record(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    place: Uuid,
) -> sqlx::Result<bool> {
    let (found,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM core.event
          WHERE user_id = $1 AND logical_source = $2
            AND payload->>'field' = 'coord' AND payload->>'place' = $3)",
    )
    .bind(user_id)
    .bind(SOURCE)
    .bind(place.to_string())
    .fetch_one(&mut **tx)
    .await?;
    Ok(found)
}

/// 直す先として使える座標の記録か（spec「形の合わない場所の記録は受け付けない」）。
///
/// 同じ利用者・同じ場所の座標の記録でなければならない。**削除の印は見ない**（消した記録を指せる）。
/// **本文を消去した記録（`raw = ''`）も断らない** —— どの場所のものかは残らないので確かめられない
/// （spec が理由では断らないと定めている）。**行錠を取らずに読む**（ST19 D5 と同じ）。
pub async fn coord_supersedes_is_valid(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    place: Uuid,
    target: Uuid,
) -> sqlx::Result<bool> {
    let row: Option<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT raw, payload FROM core.event
          WHERE id = $1 AND user_id = $2 AND logical_source = $3",
    )
    .bind(target)
    .bind(user_id)
    .bind(SOURCE)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.is_some_and(|(raw, payload)| {
        raw.is_empty()
            || (payload.get("field").and_then(|v| v.as_str()) == Some("coord")
                && payload.get("place").and_then(|v| v.as_str())
                    == Some(place.to_string().as_str()))
    }))
}

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
