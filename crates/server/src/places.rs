// SPDX-License-Identifier: AGPL-3.0-only
//! 場所の器（ST21 / design D1）。器は記録ではなく、記録を束ねる識別子。
//!
//! 識別子は画面が乱数で決めて渡す。サーバは名前・座標から計算しない（C1）。
use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone as _, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use unicode_normalization::UnicodeNormalization as _;
use uuid::Uuid;

use crate::attributes::ValidFrom;

/// 広さの記録が無い場所の広さ（m。FR-48 ★「既定 100 m」。design D6（仮））。
///
/// **`stay::Criteria::default_values().radius_m` とは別の定数** —— 滞在の判定の半径を変えても
/// 場所の既定は変わらない。どちらも 100 をテストで名指しで固定する。
pub const PLACE_DEFAULT_RADIUS_M: i64 = 100;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
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

/// 座標の記録に印す `external_ref`（どの場所の座標か）。本文を消去しても残り、錠が変化を拒む。
pub fn coord_marker(place: Uuid) -> String {
    format!("place-coord:{place}")
}

/// その場所が座標の記録を 1 件でも持つか。**削除の印の付いたものも、本文を消去したものも数える**（spec）。
///
/// 本文を消去した記録は `payload = '{}'` になるので、本文ではなく、取り込みが印した
/// `external_ref`（`coord_marker`）で数える。
pub async fn has_coord_record(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    place: Uuid,
) -> sqlx::Result<bool> {
    let (found,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM core.event
          WHERE user_id = $1 AND logical_source = $2 AND external_ref = $3)",
    )
    .bind(user_id)
    .bind(SOURCE)
    .bind(coord_marker(place))
    .fetch_one(&mut **tx)
    .await?;
    Ok(found)
}

/// 直す先として使える座標の記録か（spec「形の合わない場所の記録は受け付けない」）。
///
/// 同じ利用者・同じ場所の座標の記録でなければならない。**削除の印は見ない**（消した記録を指せる）。
/// **本文を消去した記録（`raw = ''`）も、消去を理由には断らない**（spec）—— 本文は読めないが、
/// 取り込みが印した `external_ref = coord_marker(place)` は残る（錠が凍結する）ので、それで
/// 「その場所の座標の記録か」を確かめる（D4（仮））。**行錠を取らずに読む**（ST19 D5 と同じ）。
pub async fn coord_supersedes_is_valid(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    place: Uuid,
    target: Uuid,
) -> sqlx::Result<bool> {
    let row: Option<(String, serde_json::Value, Option<String>)> = sqlx::query_as(
        "SELECT raw, payload, external_ref FROM core.event
          WHERE id = $1 AND user_id = $2 AND logical_source = $3",
    )
    .bind(target)
    .bind(user_id)
    .bind(SOURCE)
    .fetch_optional(&mut **tx)
    .await?;
    let marker = coord_marker(place);
    Ok(row.is_some_and(|(raw, payload, external_ref)| {
        if raw.is_empty() {
            return external_ref.as_deref() == Some(marker.as_str());
        }
        payload.get("field").and_then(|v| v.as_str()) == Some("coord")
            && payload.get("place").and_then(|v| v.as_str()) == Some(place.to_string().as_str())
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

/// 器を断った理由（400 だけ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlaceError {
    /// その識別子はほかの利用者の器が持っている
    PlaceIdTaken,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PlaceErrorBody {
    pub error: PlaceError,
}

/// 器を作れなかった理由のうち、本文の中身に依らないもの（401 資格情報 / 500 DB の失敗）。
/// 本文の形を 400 と揃える（`{"error": ...}`）が、400 の enum には混ぜない（review/code.md R12）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlaceUnavailable {
    Unavailable,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PlaceUnavailableBody {
    pub error: PlaceUnavailable,
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

// ------------------------------------------------------------------ いまの値と前の値（D6 / D7 / D15）

/// 格納されている場所の記録 1 件（読めたもの。削除の印の付いた行と、本文を消去した行は呼び出し側が除く）。
#[derive(Debug, Clone, PartialEq)]
pub struct StoredPlaceRecord {
    pub record: PlaceRecord,
    /// 書いた日時（出来事の時刻と、そのときの地域）
    pub written_at: DateTime<FixedOffset>,
    /// D-01 に入った時刻（FR-19）
    pub ingested_at: DateTime<Utc>,
}

impl StoredPlaceRecord {
    /// 「書いた順」の鍵。書いた日時 → D-01 に入った時刻 → 識別子（行の順に依らず決める）。
    fn order_key(&self) -> (DateTime<Utc>, DateTime<Utc>, Uuid) {
        (
            self.written_at.with_timezone(&Utc),
            self.ingested_at,
            self.record.id,
        )
    }
}

/// 座標の版 1 つと、それを当てる期間（D7）。`start` が `None` なら最も古い時刻から、
/// `end` が `None` なら終わり無し。期間は `[start, end)`。
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    /// 版の座標の記録の識別子
    pub id: Uuid,
    pub lat: f64,
    pub lon: f64,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// `Asia/Tokyo` の日の 0 時。JST は固定の +09:00（夏時間なし）なので失敗しない。
fn jst_midnight(date: NaiveDate) -> DateTime<Utc> {
    let jst = FixedOffset::east_opt(9 * 3600).unwrap_or_else(|| unreachable!("+09:00 は範囲内"));
    jst.from_local_datetime(&date.and_time(chrono::NaiveTime::MIN))
        .single()
        .unwrap_or_else(|| unreachable!("固定オフセットの時刻は一意"))
        .with_timezone(&Utc)
}

/// 座標の版（D7）。`windows` の元で、`view` が前の座標の状態を決めるのにも使う。
struct Version<'a> {
    /// 版の座標の記録
    record: &'a StoredPlaceRecord,
    /// 並び（直した記録の書いた順。直す先が使えなければ自分の書いた順）
    anchor: &'a StoredPlaceRecord,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    /// この版に負けて版にならなかった兄弟の fix（同じ記録を辿る、先に書いた fix）
    siblings: Vec<Uuid>,
}

/// 座標の記録の欄（緯度・経度・変え方・いつから・直す先）。
type CoordParts<'a> = (f64, f64, CoordChange, Option<&'a ValidFrom>, Option<Uuid>);

fn coord_of(r: &StoredPlaceRecord) -> Option<CoordParts<'_>> {
    match &r.record.field {
        PlaceField::Coord {
            lat,
            lon,
            change,
            valid_from,
            supersedes,
        } => Some((*lat, *lon, *change, valid_from.as_ref(), *supersedes)),
        _ => None,
    }
}

/// 座標の版ごとに当てる期間を決める（D7）。`records` は 1 つの場所の使える記録
/// （座標でないものは読み飛ばす）。入力の順に依らず、版を**書いた順**に返す。
fn versions<'a>(records: &'a [StoredPlaceRecord]) -> Vec<Version<'a>> {
    let mut coords: Vec<&StoredPlaceRecord> =
        records.iter().filter(|r| coord_of(r).is_some()).collect();
    coords.sort_by_key(|r| r.order_key());
    let by_id: std::collections::HashMap<Uuid, &StoredPlaceRecord> =
        coords.iter().map(|r| (r.record.id, *r)).collect();

    // 直す先を使える記録まで辿る（`Fix` の連鎖。輪になっていても止まる）。
    // 返すのは 位置と当て始めを継ぐ元の記録。
    let root = |r: &'a StoredPlaceRecord| -> &'a StoredPlaceRecord {
        let mut cur = r;
        for _ in 0..=coords.len() {
            let Some((_, _, CoordChange::Fix, _, Some(target))) = coord_of(cur) else {
                break;
            };
            match by_id.get(&target) {
                Some(next) if next.record.id != cur.record.id => cur = next,
                _ => break,
            }
        }
        cur
    };

    // 使える fix に直された記録は版ではない
    let fixed: std::collections::HashSet<Uuid> = coords
        .iter()
        .filter_map(|r| match coord_of(r) {
            Some((_, _, CoordChange::Fix, _, Some(t))) if by_id.contains_key(&t) => Some(t),
            _ => None,
        })
        .collect();

    // 当て始め。`None` は最も古い時刻
    let start_of = |r: &StoredPlaceRecord| -> Option<DateTime<Utc>> {
        match coord_of(r) {
            Some((_, _, CoordChange::Move, Some(vf), _)) => vf.key().map(jst_midnight),
            _ => None,
        }
    };

    let mut vs: Vec<(Version<'_>, DateTime<Utc>)> = coords
        .iter()
        .filter(|r| !fixed.contains(&r.record.id))
        .map(|r| {
            let anchor = root(r);
            // fix は直した記録の当て始めを継ぐ。直す先が使えなければ最も古い時刻
            let start = start_of(anchor);
            // 区切り: 当て始めがあればそれ。最も古い時刻なら、版（fix なら直した記録）を書いた日時
            let boundary = start.unwrap_or_else(|| anchor.written_at.with_timezone(&Utc));
            (
                Version {
                    record: r,
                    anchor,
                    start,
                    end: None,
                    siblings: Vec::new(),
                },
                boundary,
            )
        })
        .collect();
    // 並び: 直した記録の書いた順、同じなら自分の書いた順
    vs.sort_by_key(|(v, _)| (v.anchor.order_key(), v.record.order_key()));

    // 兄弟の fix（同じ記録を辿る fix が 2 件以上。2 つのタブ・2 つの端末から）は、最後に書いた 1 件だけを版にする
    // （D7（仮））。取り込みでは断らない —— 断ると本人の操作を捨てるので、計算し直せる読み出しの側で決める
    let mut kept: Vec<(Version<'_>, DateTime<Utc>)> = Vec::with_capacity(vs.len());
    for (v, boundary) in vs {
        match kept.last_mut() {
            Some((prev, prev_boundary)) if prev.anchor.record.id == v.anchor.record.id => {
                let mut siblings = std::mem::take(&mut prev.siblings);
                siblings.push(prev.record.record.id);
                *prev = Version { siblings, ..v };
                *prev_boundary = boundary;
            }
            _ => kept.push((v, boundary)),
        }
    }
    let mut vs = kept;

    // 当て終わり: 後に書いたどの版の区切りよりも前まで
    let mut earliest_later: Option<DateTime<Utc>> = None;
    for (v, boundary) in vs.iter_mut().rev() {
        v.end = earliest_later;
        earliest_later = Some(earliest_later.map_or(*boundary, |e| e.min(*boundary)));
    }
    vs.into_iter().map(|(v, _)| v).collect()
}

/// 座標の版ごとに当てる期間（D7）。単体テストで表を固定する。
pub fn coord_windows(records: &[StoredPlaceRecord]) -> Vec<Window> {
    versions(records)
        .into_iter()
        .filter_map(|v| {
            let (lat, lon, ..) = coord_of(v.record)?;
            Some(Window {
                id: v.record.record.id,
                lat,
                lon,
                start: v.start,
                end: v.end,
            })
        })
        .collect()
}

/// 名前の記録の読み出し（2 つの時刻つき）。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct NameRecordOut {
    pub record_id: Uuid,
    /// RFC 3339（地域のずれつき）
    pub written_at: String,
    /// RFC 3339
    pub ingested_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct PreviousNameOut {
    pub record_id: Uuid,
    pub name: String,
    pub written_at: String,
    pub ingested_at: String,
}

/// いまの座標の記録の読み出し。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct CoordOut {
    pub record_id: Uuid,
    pub lat: f64,
    pub lon: f64,
    pub change: CoordChange,
    pub valid_from: Option<ValidFrom>,
    pub supersedes: Option<Uuid>,
    pub written_at: String,
    pub ingested_at: String,
}

/// 前の座標の状態（D15）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreviousCoordState {
    /// 「間違いを直す」で直された座標
    Fixed,
    /// 移る前の座標
    BeforeMove,
    /// 今日より後の「いつから」の移転（予定）
    Upcoming,
}

impl PreviousCoordState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::BeforeMove => "before_move",
            Self::Upcoming => "upcoming",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct PreviousCoordOut {
    pub record_id: Uuid,
    pub lat: f64,
    pub lon: f64,
    pub change: CoordChange,
    pub state: PreviousCoordState,
    pub valid_from: Option<ValidFrom>,
    pub supersedes: Option<Uuid>,
    pub written_at: String,
    pub ingested_at: String,
    /// 直された座標を直した記録（`state = fixed` のときだけ）
    pub fixed_by: Option<Uuid>,
}

/// 場所の滞在の項（D9）。当たった滞在が無ければ 0 と `null`。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct PlaceStays {
    pub count: i64,
    pub minutes: i64,
    /// `YYYY-MM-DD`（Asia/Tokyo の日）。居たことが無ければ `null`
    pub last_day: Option<String>,
    /// 時間帯（0〜23 時）ごとの分
    pub hours: Vec<i64>,
}

impl PlaceStays {
    pub fn empty() -> Self {
        Self {
            count: 0,
            minutes: 0,
            last_day: None,
            hours: vec![0; 24],
        }
    }
}

/// 場所 1 つぶんの読み出し（D15）。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct PlaceOut {
    pub id: Uuid,
    pub name: String,
    pub note: Option<String>,
    pub radius_m: i64,
    pub name_record: NameRecordOut,
    pub coord: CoordOut,
    pub stays: PlaceStays,
    /// いまの名前以外の名前の記録。書いた日時の新しい順
    pub previous_names: Vec<PreviousNameOut>,
    /// いまの座標の版以外の座標の記録。書いた順
    pub previous_coords: Vec<PreviousCoordOut>,
}

/// `GET /places` の中身（D15）。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct PlacesView {
    /// `Asia/Tokyo` の今日
    pub today: String,
    pub places: Vec<PlaceOut>,
}

fn times(r: &StoredPlaceRecord) -> (String, String) {
    (r.written_at.to_rfc3339(), r.ingested_at.to_rfc3339())
}

/// 器ごとのいまの値と前の値を組む（D6（仮））。滞在は与えない（滞在の項は 0）。
pub fn view(places: &[Uuid], records: &[StoredPlaceRecord], today: NaiveDate) -> PlacesView {
    view_with_stays(places, records, &[], today)
}

/// 器ごとのいまの値と前の値に、滞在の照合と合計を足して組む（D6 / D8 / D9）。**純粋な関数**（DB を持たない）。
///
/// `places` は器の識別子を**作った順**（`seq`）で渡す。`records` は使える記録（削除の印の付いたものと
/// 本文を消去したものを除いたもの）の全部、`stays` は使える滞在の全部。
/// 返す並びは最近居た順（当たった滞在の無い場所は、その後に作った順）。
pub fn view_with_stays(
    places: &[Uuid],
    records: &[StoredPlaceRecord],
    stays: &[StayPoint],
    today: NaiveDate,
) -> PlacesView {
    let today_start = jst_midnight(today);
    let (mut out, targets) = places_and_targets(places, records, today_start);

    let assigned = assign(stays, &targets);
    let mut last_end: Vec<Option<DateTime<Utc>>> = Vec::with_capacity(out.len());
    for (i, p) in out.iter_mut().enumerate() {
        let hit: Vec<&StayPoint> = stays
            .iter()
            .zip(&assigned)
            .filter(|(_, a)| **a == Some(targets[i].id))
            .map(|(s, _)| s)
            .collect();
        last_end.push(hit.iter().map(|s| s.end).max());
        p.stays = summarize(&hit);
    }
    // 最近居た順。当たった滞在の無い場所はその後（安定な並べ替えなので、同じなら作った順のまま）
    let mut order: Vec<usize> = (0..out.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(last_end[i]));
    let mut slots: Vec<Option<PlaceOut>> = out.into_iter().map(Some).collect();
    PlacesView {
        today: today.to_string(),
        places: order.into_iter().filter_map(|i| slots[i].take()).collect(),
    }
}

/// 名前と座標を持つ場所の読み出しと、その照合の的（作った順）。
fn places_and_targets(
    places: &[Uuid],
    records: &[StoredPlaceRecord],
    today_start: DateTime<Utc>,
) -> (Vec<PlaceOut>, Vec<MatchPlace>) {
    let mut by_place: std::collections::HashMap<Uuid, Vec<StoredPlaceRecord>> =
        std::collections::HashMap::new();
    for r in records {
        by_place.entry(r.record.place).or_default().push(r.clone());
    }
    let mut out: Vec<PlaceOut> = Vec::new();
    let mut targets: Vec<MatchPlace> = Vec::new();
    for id in places {
        let Some(recs) = by_place.get(id) else {
            continue;
        };
        let Some(p) = place_out(*id, recs, today_start) else {
            continue;
        };
        targets.push(MatchPlace {
            id: *id,
            radius_m: p.radius_m,
            windows: coord_windows(recs),
        });
        out.push(p);
    }
    (out, targets)
}

fn place_out(
    id: Uuid,
    records: &[StoredPlaceRecord],
    today_start: DateTime<Utc>,
) -> Option<PlaceOut> {
    // 名前・広さ・補足は、書いた日時 → D-01 に入った時刻の順で最後の記録がいまの値
    let mut ordered: Vec<&StoredPlaceRecord> = records.iter().collect();
    ordered.sort_by_key(|r| r.order_key());
    let last = |pick: fn(&PlaceField) -> bool| ordered.iter().rev().find(|r| pick(&r.record.field));

    let current_name = last(|f| matches!(f, PlaceField::Name(_)))?;
    let PlaceField::Name(name) = &current_name.record.field else {
        return None;
    };
    let radius_m = match last(|f| matches!(f, PlaceField::Radius(_))).map(|r| &r.record.field) {
        Some(PlaceField::Radius(m)) => *m,
        _ => PLACE_DEFAULT_RADIUS_M,
    };
    let note = match last(|f| matches!(f, PlaceField::Note(_))).map(|r| &r.record.field) {
        Some(PlaceField::Note(n)) => n.clone(),
        _ => None,
    };

    // 座標。今日（の始まり）を含む版のうち並びの後のもの。無ければ書いた順の最後の版
    let vs = versions(records);
    let current = vs
        .iter()
        .rev()
        .find(|v| v.start.is_none_or(|s| s <= today_start) && v.end.is_none_or(|e| today_start < e))
        .or(vs.last())?;
    let (lat, lon, change, valid_from, supersedes) = coord_of(current.record)?;
    let (written_at, ingested_at) = times(current.record);
    let coord = CoordOut {
        record_id: current.record.record.id,
        lat,
        lon,
        change,
        valid_from: valid_from.cloned(),
        supersedes,
        written_at,
        ingested_at,
    };

    // 前の名前: いまの名前の記録以外。書いた日時の新しい順
    let previous_names = ordered
        .iter()
        .rev()
        .filter(|r| r.record.id != current_name.record.id)
        .filter_map(|r| {
            let PlaceField::Name(n) = &r.record.field else {
                return None;
            };
            let (written_at, ingested_at) = times(r);
            Some(PreviousNameOut {
                record_id: r.record.id,
                name: n.clone(),
                written_at,
                ingested_at,
            })
        })
        .collect();

    // 前の座標: いまの座標の版以外の座標の記録。書いた順
    let version_ids: std::collections::HashSet<Uuid> =
        vs.iter().map(|v| v.record.record.id).collect();
    // 版ごとに、直す先を辿った記録の集まり（版そのものを含む）。連鎖の兄弟で、直接直した fix が
    // 負けて版にならなかったときに、勝った版を指すのに使う（review/code.md R16）
    let chains: Vec<(Uuid, std::collections::HashSet<Uuid>)> = vs
        .iter()
        .map(|v| {
            let mut ids = std::collections::HashSet::new();
            let mut cur = v.record;
            while ids.insert(cur.record.id) {
                let Some((_, _, CoordChange::Fix, _, Some(target))) = coord_of(cur) else {
                    break;
                };
                let Some(next) = ordered.iter().find(|r| r.record.id == target) else {
                    break;
                };
                cur = next;
            }
            (v.record.record.id, ids)
        })
        .collect();
    let winning_version_through = |id: Uuid| {
        chains
            .iter()
            .find(|(_, ids)| ids.contains(&id))
            .map(|(v, _)| *v)
    };
    let previous_coords = ordered
        .iter()
        .filter(|r| r.record.id != current.record.record.id)
        .filter_map(|r| {
            let (lat, lon, change, valid_from, supersedes) = coord_of(r)?;
            let (state, fixed_by) = if version_ids.contains(&r.record.id) {
                let upcoming = vs
                    .iter()
                    .find(|v| v.record.record.id == r.record.id)
                    .is_some_and(|v| v.start.is_some_and(|s| s > today_start));
                if upcoming {
                    (PreviousCoordState::Upcoming, None)
                } else {
                    (PreviousCoordState::BeforeMove, None)
                }
            } else {
                // 使える fix に直された記録。最後に書いた fix を直した記録とする（兄弟の fix は D7（仮））。
                // ただしその fix が版の連鎖に無い（兄弟に負けた）なら、この記録を辿る版を直した記録とする（R16）。
                // 直す fix が無いのは兄弟に負けた fix —— 勝った版を直した記録とする
                let direct = ordered
                    .iter()
                    .rev()
                    .find(|f| {
                        matches!(coord_of(f), Some((_, _, CoordChange::Fix, _, Some(t))) if t == r.record.id)
                    })
                    .map(|f| f.record.id);
                let fixer = match direct {
                    Some(d) if winning_version_through(d).is_some() => Some(d),
                    _ => winning_version_through(r.record.id).or(direct),
                }
                .or_else(|| {
                    vs.iter()
                        .find(|v| v.siblings.contains(&r.record.id))
                        .map(|v| v.record.record.id)
                });
                (PreviousCoordState::Fixed, fixer)
            };
            let (written_at, ingested_at) = times(r);
            Some(PreviousCoordOut {
                record_id: r.record.id,
                lat,
                lon,
                change,
                state,
                valid_from: valid_from.cloned(),
                supersedes,
                written_at,
                ingested_at,
                fixed_by,
            })
        })
        .collect();

    let (name_written_at, name_ingested_at) = times(current_name);
    Some(PlaceOut {
        id,
        name: name.clone(),
        note,
        radius_m,
        name_record: NameRecordOut {
            record_id: current_name.record.id,
            written_at: name_written_at,
            ingested_at: name_ingested_at,
        },
        coord,
        stays: PlaceStays::empty(),
        previous_names,
        previous_coords,
    })
}

// ------------------------------------------------------------------ 照合と合計（D8 / D9）

/// 照合に使う滞在 1 件（`core.event_live` の `s01-stay`・`origin='derived'`）。
#[derive(Debug, Clone, PartialEq)]
pub struct StayPoint {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// 代表点。**無ければ当てない**
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

/// 照合される場所 1 つ。`windows` は `coord_windows` の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct MatchPlace {
    pub id: Uuid,
    pub radius_m: i64,
    pub windows: Vec<Window>,
}

/// 滞在がどの場所に当たるか（D8）。`places` は作った順で渡す。**純粋な関数**で、何も書かない。
///
/// 各場所について、滞在の始まりを期間に含む版の座標との距離の最小が広さ以下なら候補。
/// 候補のうち距離が最小のもの、同じなら先に作った（先に渡された）場所。
pub fn assign(stays: &[StayPoint], places: &[MatchPlace]) -> Vec<Option<Uuid>> {
    stays
        .iter()
        .map(|stay| {
            let (lat, lon) = (stay.lat?, stay.lon?);
            let mut best: Option<(f64, Uuid)> = None;
            for place in places {
                let nearest = place
                    .windows
                    .iter()
                    .filter(|w| {
                        w.start.is_none_or(|s| s <= stay.start)
                            && w.end.is_none_or(|e| stay.start < e)
                    })
                    .map(|w| crate::stay::distance_m(lat, lon, w.lat, w.lon))
                    .min_by(f64::total_cmp);
                let Some(d) = nearest.filter(|d| *d <= place.radius_m as f64) else {
                    continue;
                };
                if best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, place.id));
                }
            }
            best.map(|(_, id)| id)
        })
        .collect()
}

/// 秒を分へ丸める（四捨五入）。
fn round_minutes(secs: i64) -> i64 {
    (secs + 30).div_euclid(60)
}

/// 当たった滞在の件数・合計・最後に居た日・24 時間の帯（D9）。
fn summarize(hit: &[&StayPoint]) -> PlaceStays {
    const JST_SECS: i64 = 9 * 3600;
    let mut total = 0_i64;
    let mut hour_secs = [0_i64; 24];
    for s in hit {
        let (start, end) = (s.start.timestamp(), s.end.timestamp());
        total += end - start;
        // `Asia/Tokyo` の時刻で 1 時間ごとに切る
        let mut cur = start;
        while cur < end {
            let hour = (cur + JST_SECS).div_euclid(3600);
            let next = ((hour + 1) * 3600 - JST_SECS).min(end);
            hour_secs[hour.rem_euclid(24) as usize] += next - cur;
            cur = next;
        }
    }
    let last_day = hit
        .iter()
        .map(|s| s.end)
        .max()
        .map(|e| (e + chrono::Duration::hours(9)).date_naive().to_string());
    PlaceStays {
        count: hit.len() as i64,
        minutes: round_minutes(total),
        last_day,
        hours: hour_secs.iter().map(|s| round_minutes(*s)).collect(),
    }
}

// ------------------------------------------------------------------ 名前の無い、よく居た所（D10）

/// 名前の無い所へまとめる半径（m）。場所の広さとも滞在の判定の半径とも別の定数（D10 / C9）。
pub const CANDIDATE_RADIUS_M: f64 = 100.0;

/// 名前の無い所の滞在の項。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct CandidateStays {
    pub count: i64,
    pub minutes: i64,
    /// `YYYY-MM-DD`（Asia/Tokyo の日）。最初に居た日
    pub first_day: String,
    /// `YYYY-MM-DD`（Asia/Tokyo の日）。最後に居た日
    pub last_day: String,
    /// 時間帯（0〜23 時）ごとの分
    pub hours: Vec<i64>,
}

/// 名前の無い、よく居た所 1 つ（D10。保存しない）。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct Candidate {
    /// 属する滞在の代表点の単純平均
    pub lat: f64,
    pub lon: f64,
    pub stays: CandidateStays,
}

/// `GET /places/candidates` の中身（D15）。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct CandidatesView {
    pub candidates: Vec<Candidate>,
}

struct Cluster<'a> {
    lat: f64,
    lon: f64,
    /// 属する代表点の緯度・経度の合計（足した順）。中心を取り直すたびに全点をなめないため（R15）
    sum_lat: f64,
    sum_lon: f64,
    stays: Vec<&'a StayPoint>,
}

fn jst_day(t: DateTime<Utc>) -> String {
    (t + chrono::Duration::hours(9)).date_naive().to_string()
}

/// どの場所にも当たらない滞在から、名前の無い所を作る（D10）。**純粋な関数**で、何も書かない。
///
/// `stays` は始まり → 識別子の順。代表点の無い滞在は数えない。
/// 1 件ずつ、中心が `CANDIDATE_RADIUS_M` 以内の最初のまとまりに足し（中心は属する代表点の単純平均）、無ければ新しいまとまり。
/// 並びは属する滞在の `end` の最大の新しい順、同じなら合計の大きい順、同じなら中心の緯度・経度の順。
pub fn candidates(stays: &[StayPoint]) -> Vec<Candidate> {
    let mut clusters: Vec<Cluster> = Vec::new();
    for s in stays {
        let (Some(lat), Some(lon)) = (s.lat, s.lon) else {
            continue;
        };
        let near = clusters
            .iter_mut()
            .find(|c| crate::stay::distance_m(lat, lon, c.lat, c.lon) <= CANDIDATE_RADIUS_M);
        match near {
            Some(c) => {
                c.stays.push(s);
                c.sum_lat += lat;
                c.sum_lon += lon;
                let n = c.stays.len() as f64;
                c.lat = c.sum_lat / n;
                c.lon = c.sum_lon / n;
            }
            None => clusters.push(Cluster {
                lat,
                lon,
                // 0.0 から足した値（全点を畳んだ前の実装と同じ順の足し算で、同じ浮動小数になる）
                sum_lat: 0.0 + lat,
                sum_lon: 0.0 + lon,
                stays: vec![s],
            }),
        }
    }
    let mut out: Vec<(DateTime<Utc>, Candidate)> = clusters
        .into_iter()
        .map(|c| {
            let summary = summarize(&c.stays);
            let last = c.stays.iter().map(|s| s.end).max();
            let first = c.stays.iter().map(|s| s.start).min();
            (
                last.unwrap_or_default(),
                Candidate {
                    lat: c.lat,
                    lon: c.lon,
                    stays: CandidateStays {
                        count: summary.count,
                        minutes: summary.minutes,
                        first_day: first.map(jst_day).unwrap_or_default(),
                        last_day: summary.last_day.unwrap_or_default(),
                        hours: summary.hours,
                    },
                },
            )
        })
        .collect();
    out.sort_by(|(ea, a), (eb, b)| {
        eb.cmp(ea)
            .then(b.stays.minutes.cmp(&a.stays.minutes))
            .then(a.lat.total_cmp(&b.lat))
            .then(a.lon.total_cmp(&b.lon))
    });
    out.into_iter().map(|(_, c)| c).collect()
}

/// どの場所にも当たらない滞在から名前の無い所を作る（純粋。DB を持たない）。
pub fn candidates_view(
    places: &[Uuid],
    records: &[StoredPlaceRecord],
    stays: &[StayPoint],
    today: NaiveDate,
) -> CandidatesView {
    let (_, targets) = places_and_targets(places, records, jst_midnight(today));
    let assigned = assign(stays, &targets);
    let unassigned: Vec<StayPoint> = stays
        .iter()
        .zip(&assigned)
        .filter(|(_, a)| a.is_none())
        .map(|(s, _)| s.clone())
        .collect();
    CandidatesView {
        candidates: candidates(&unassigned),
    }
}

// ------------------------------------------------------------------ 読み出し（DB）

#[derive(sqlx::FromRow)]
struct RecordRow {
    id: Uuid,
    event_time: DateTime<Utc>,
    tz_offset_min: i32,
    ingest_time: DateTime<Utc>,
    raw: String,
}

/// その利用者の器（作った順）と、場所の記録の全部を読んで `view` に渡す。
///
/// **`core.event_live` から引く**（削除の印を効かせる。製造準備 A-3）。**感度で絞らない**
/// （感度は外へ出す口の条件で、本人の画面の条件ではない）。本文を消去した行（`raw = ''`）は
/// 読めないので落とす。読めない行は数えて叫ぶ（**値は載せない**。製造準備 A-2）。
pub async fn places_view(
    pool: &PgPool,
    user_id: Uuid,
    today: NaiveDate,
) -> sqlx::Result<PlacesView> {
    let (ids, records) = place_records(pool, user_id).await?;
    let stays = stays_of(pool, user_id).await?;
    Ok(view_with_stays(&ids, &records, &stays, today))
}

/// 名前の無い、よく居た所（D10 / D15）。**読むだけ**で、保存しない。
pub async fn candidates_of(
    pool: &PgPool,
    user_id: Uuid,
    today: NaiveDate,
) -> sqlx::Result<CandidatesView> {
    let (ids, records) = place_records(pool, user_id).await?;
    let stays = stays_of(pool, user_id).await?;
    Ok(candidates_view(&ids, &records, &stays, today))
}

async fn place_records(
    pool: &PgPool,
    user_id: Uuid,
) -> sqlx::Result<(Vec<Uuid>, Vec<StoredPlaceRecord>)> {
    let ids: Vec<(Uuid,)> =
        sqlx::query_as("SELECT id FROM core.place WHERE user_id = $1 ORDER BY seq")
            .bind(user_id)
            .fetch_all(pool)
            .await?;
    let rows: Vec<RecordRow> = sqlx::query_as(
        "SELECT id, event_time, tz_offset_min, ingest_time, raw
           FROM core.event_live
          WHERE user_id = $1 AND logical_source = $2 AND raw <> ''
          ORDER BY ingest_time, id",
    )
    .bind(user_id)
    .bind(SOURCE)
    .fetch_all(pool)
    .await?;
    let read = rows.len();
    let records: Vec<StoredPlaceRecord> = rows.into_iter().filter_map(stored_record_of).collect();
    if records.len() != read {
        tracing::error!(
            kind = "place_records_dropped",
            read = read,
            dropped = read - records.len(),
            "読めない場所の記録を読み出しから落とした（行は DB に残っている）"
        );
    }
    Ok((ids.into_iter().map(|(id,)| id).collect(), records))
}

/// 利用者の滞在の全部（`day_view` と同じ絞り方。削除の印の付いたものは `event_live` が外す）。
/// **読むだけ**（FR-48 ★: 滞在にも場所の記録にも書き込まない）。
async fn stays_of(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<StayPoint>> {
    let rows: Vec<(DateTime<Utc>, serde_json::Value)> = sqlx::query_as(
        "SELECT event_time, payload FROM core.event_live
          WHERE logical_source = $1 AND origin = 'derived' AND user_id = $2
          ORDER BY event_time, id",
    )
    .bind(crate::stay::SOURCE)
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(event_time, payload)| {
            let (start, end) = crate::stay_store::span_of(event_time, &payload);
            StayPoint {
                start,
                end,
                lat: payload.get("lat").and_then(serde_json::Value::as_f64),
                lon: payload.get("lon").and_then(serde_json::Value::as_f64),
            }
        })
        .collect())
}

fn stored_record_of(row: RecordRow) -> Option<StoredPlaceRecord> {
    let offset = row
        .tz_offset_min
        .checked_mul(60)
        .and_then(FixedOffset::east_opt)?;
    let record = parse_place_record(&row.raw, row.id).ok()?;
    Some(StoredPlaceRecord {
        record,
        written_at: row.event_time.with_timezone(&offset),
        ingested_at: row.ingest_time,
    })
}
