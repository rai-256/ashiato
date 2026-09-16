// SPDX-License-Identifier: AGPL-3.0-only
//! 個人属性の主張の解釈と、「いまの値」の導き方（ST19 / FR-44 / FR-45。design D5 / D6）。
//! **DB に触らない。**
//!
//! 取り込み口（`ingest_one`）も読み出し（`GET /attributes`）も判定はここだけを通す ——
//! 規則が 2 か所に割れると、片方だけ直したときに格納と読み出しが食い違う。
//!
//! **主張は書き換えられない**（深掘り Q1 / C1。DB の錠は移行の側）。ここが持つのは
//! 「原文をどう読むか」と「積んだ主張からどう『いまの値』を導くか」だけで、
//! **導き方は後から計算し直せる**（D6（仮）の反転条件）。
use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use serde::Serialize;
use unicode_normalization::UnicodeNormalization as _;

/// 主張を置く論理ソース（design D1）。
pub const SOURCE: &str = "s01-attribute";

/// 既定の感度（ローカル AI まで。PERM-4 / 深掘り Q3。design D3（仮））。
///
/// **反転条件**: ST24 が登録簿に「ソースごとの既定の感度」を持たせたとき、値は 2 のまま
/// この分岐を登録簿の値へ移す。
pub const DEFAULT_SENSITIVITY: i32 = 2;

/// 原文の乱数の最小の長さ（128 bit を base64url で書いた 22 文字。design D4 / 深掘り C12）。
///
/// **短い乱数は総当たりの範囲に入る** —— 消去の後に残る `id` / `event_time` / `content_hash` と
/// 値の候補から原文を組み直して鍵と照らせば、消したはずの値が確かめられる。
pub const NONCE_MIN_CHARS: usize = 22;

/// 「いつから」の精度（深掘り C3）。**丸めない** ——
/// 2019 年とだけ分かっている値を 2019-01-01 にすると、丸めたことが後から分からない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Precision {
    Year,
    Month,
    Day,
    /// 「分からない」も値として受け付ける（深掘り C3）
    Unknown,
}

impl Precision {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "year" => Some(Self::Year),
            "month" => Some(Self::Month),
            "day" => Some(Self::Day),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// 「いつから」。精度と、その精度の日付の組（design D1 の原文の形）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ValidFrom {
    pub precision: Precision,
    /// `2019` / `2019-10` / `2019-10-01`。精度が「分からない」なら `null`
    pub date: Option<String>,
}

impl ValidFrom {
    /// 並べ替えと「今日以前か」の比較に使う鍵（design D6 の手順 3）。
    ///
    /// **精度「分からない」は `None`** = 最も古い側。年はその年の 1 月 1 日、
    /// 年月はその月の 1 日から有効とみなす（**表示は丸めない。比較だけ**）。
    pub fn key(&self) -> Option<NaiveDate> {
        let date = self.date.as_deref()?;
        match self.precision {
            Precision::Unknown => None,
            Precision::Year => date
                .parse::<i32>()
                .ok()
                .and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1)),
            Precision::Month => {
                let (y, m) = date.split_once('-')?;
                NaiveDate::from_ymd_opt(y.parse().ok()?, m.parse().ok()?, 1)
            }
            Precision::Day => NaiveDate::parse_from_str(date, "%Y-%m-%d").ok(),
        }
    }

    /// 精度と日付の組が合っているか、暦にある日付か（spec「形の合わない主張は受け付けない」）。
    fn is_well_formed(&self) -> bool {
        let Some(date) = self.date.as_deref() else {
            // 日付を持たないのは「分からない」のときだけ
            return self.precision == Precision::Unknown;
        };
        match self.precision {
            // **「分からない」に日付を添えさせない** —— 添うと、読む側が精度と日付のどちらを
            // 信じるかで割れる（比較の鍵は「分からない」を最も古い側に置くので日付は死ぬ）
            Precision::Unknown => false,
            Precision::Year => {
                date.len() == 4 && date.bytes().all(|b| b.is_ascii_digit()) && self.key().is_some()
            }
            Precision::Month => {
                date.len() == 7
                    && date.as_bytes()[4] == b'-'
                    && self.key().is_some_and(|_| {
                        date[5..]
                            .parse::<u32>()
                            .is_ok_and(|m| (1..=12).contains(&m))
                    })
            }
            // `parse_from_str` は暦に無い日付（2019-02-30）を読めないので、ここで落ちる
            Precision::Day => date.len() == 10 && self.key().is_some(),
        }
    }
}

/// 原文から読んだ 1 件の主張（design D1）。
#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    /// 主張の識別子。**記録の `id` と同じでなければならない**
    pub id: uuid::Uuid,
    pub kind: uuid::Uuid,
    /// 値。`None` が「なし」（その属性が終わった。深掘り C10）
    pub value: Option<String>,
    pub valid_from: ValidFrom,
    /// 取り消す主張（深掘り C5）
    pub supersedes: Option<uuid::Uuid>,
    pub note: Option<String>,
    /// **原文から `nonce` を除いて組み直した解析済み**（NFC。design D4）。
    ///
    /// 送り主の `payload` は使わない —— 原文と解析済みがずれる経路を作らない。
    /// **乱数をここへ写さない** —— 写すと消去（`raw=''` / `payload='{}'`）でしか消えず、
    /// 列に残った乱数から鍵を作り直せてしまう。
    pub payload: serde_json::Value,
}

/// 主張の形が合わない理由（design D5）。`IngestError` へ写す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimInvalid {
    /// 原文が JSON でない / 必須の欄が欠ける / 識別子が記録と違う / 乱数が短い
    Malformed,
    /// 値が、前後の空白を除いて空で、「なし」でもない
    Value,
    /// 精度と日付の形が合わない / 暦に無い日付
    ValidFrom,
}

/// 原文を読み、形を確かめて主張にする（design D5 / tasks 2.1）。
///
/// `id` は**記録の**識別子。原文の `claim` がこれと違えば断る ——
/// 食い違うと、消去の後に残る `id` の列とは別の識別子が原文の中にいることになり、
/// どちらがその主張かが決められない。
pub fn parse_claim(raw: &str, id: uuid::Uuid) -> Result<Claim, ClaimInvalid> {
    let v: serde_json::Value = serde_json::from_str(raw).map_err(|_| ClaimInvalid::Malformed)?;
    let obj = v.as_object().ok_or(ClaimInvalid::Malformed)?;

    // --- 必須の欄（欠ければ `malformed_claim`）
    let claim_id = obj
        .get("claim")
        .and_then(|x| x.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(ClaimInvalid::Malformed)?;
    if claim_id != id {
        return Err(ClaimInvalid::Malformed);
    }
    // **乱数は長さだけを見る**（中身は画面が `crypto.getRandomValues` で作る。design D4）。
    // サーバは乱数を作らない —— 足すと原文が変わり「受け取ったまま」が成り立たない。
    let nonce = obj
        .get("nonce")
        .and_then(|x| x.as_str())
        .ok_or(ClaimInvalid::Malformed)?;
    if nonce.chars().count() < NONCE_MIN_CHARS
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(ClaimInvalid::Malformed);
    }
    let kind = obj
        .get("kind")
        .and_then(|x| x.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(ClaimInvalid::Malformed)?;

    // `value` は欄そのものが要る（`null` が「なし」なので、欠落と区別する）
    let value_field = obj.get("value").ok_or(ClaimInvalid::Malformed)?;
    let value = match value_field {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s.nfc().collect::<String>()),
        _ => return Err(ClaimInvalid::Malformed),
    };

    let vf = obj.get("valid_from").ok_or(ClaimInvalid::Malformed)?;
    let precision = vf
        .get("precision")
        .and_then(|x| x.as_str())
        .and_then(Precision::from_str)
        .ok_or(ClaimInvalid::Malformed)?;
    let date = match vf.get("date") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(_) => return Err(ClaimInvalid::Malformed),
    };
    let valid_from = ValidFrom { precision, date };

    let supersedes = match obj.get("supersedes") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => {
            Some(uuid::Uuid::parse_str(s).map_err(|_| ClaimInvalid::Malformed)?)
        }
        Some(_) => return Err(ClaimInvalid::Malformed),
    };
    let note = match obj.get("note") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(s.nfc().collect::<String>()),
        Some(_) => return Err(ClaimInvalid::Malformed),
    };

    // --- 値と「いつから」（形は合っているが中身が通らないもの。種別を分けて返す）
    //
    // **「なし」は空ではない**（深掘り C10）—— 置き換える値が無い終わり方を表す。
    if value.as_deref().is_some_and(|s| s.trim().is_empty()) {
        return Err(ClaimInvalid::Value);
    }
    if !valid_from.is_well_formed() {
        return Err(ClaimInvalid::ValidFrom);
    }

    Ok(Claim {
        payload: payload_of(
            &valid_from,
            claim_id,
            kind,
            value.as_deref(),
            supersedes,
            note.as_deref(),
        ),
        id: claim_id,
        kind,
        value,
        valid_from,
        supersedes,
        note,
    })
}

/// 解析済みを原文から組み直す。**`nonce` を入れない**（design D4）。
fn payload_of(
    valid_from: &ValidFrom,
    id: uuid::Uuid,
    kind: uuid::Uuid,
    value: Option<&str>,
    supersedes: Option<uuid::Uuid>,
    note: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "claim": id,
        "kind": kind,
        "value": value,
        "valid_from": {
            "precision": valid_from.precision,
            "date": valid_from.date,
        },
        "supersedes": supersedes,
        "note": note,
    })
}

// ------------------------------------------------------------------ いまの値の導き方（D6（仮））

/// 属性の種類 1 つ（design D7）。**いまの名前**は呼び出し側が台帳から引いて渡す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kind {
    pub id: uuid::Uuid,
    pub name: String,
}

/// 格納されている主張 1 件。`view` の入力。
///
/// **削除の印の付いた行は呼び出し側が除いて渡す**（`core.event_live`）。
/// 本文を消去した行（`raw = ''`）は `erased` を立てて渡してよい —— `view` が落とす。
#[derive(Debug, Clone, PartialEq)]
pub struct StoredClaim {
    pub claim: Claim,
    /// 主張した日時（出来事の時刻と、そのときの地域。深掘り C4）
    pub asserted_at: DateTime<FixedOffset>,
    /// D-01 に入った時刻（FR-19）
    pub ingested_at: DateTime<Utc>,
    /// 本文を消去された（`raw = ''`）。**種類も値も読めないので並べる置き場を持たない**
    pub erased: bool,
}

/// 読み出しが返す 1 件（design D8）。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct ClaimOut {
    pub id: uuid::Uuid,
    /// `null` が「なし」
    pub value: Option<String>,
    pub valid_from: ValidFrom,
    /// RFC 3339（地域のずれつき）
    pub asserted_at: String,
    /// RFC 3339
    pub ingested_at: String,
    pub supersedes: Option<uuid::Uuid>,
    /// どの主張に取り消されたか（取り消された主張にだけ入る）
    pub superseded_by: Option<uuid::Uuid>,
    pub note: Option<String>,
}

/// 種類 1 つぶんの読み出し（design D8）。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct KindView {
    pub id: uuid::Uuid,
    pub name: String,
    /// 「いつから」が今日以前で最も新しい主張。無ければ `null`
    pub current: Option<ClaimOut>,
    /// 「いつから」が今日より後（予定）。古い順
    pub upcoming: Vec<ClaimOut>,
    /// 取り消されていない主張の全部。「いつから」の新しい順
    pub claims: Vec<ClaimOut>,
    /// 訂正で取り消された主張
    pub superseded: Vec<ClaimOut>,
}

/// 個人属性の読み出し全体（design D8）。
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct AttributesView {
    /// `Asia/Tokyo` の今日
    pub today: String,
    pub kinds: Vec<KindView>,
}

/// 並べ替えの鍵（design D6 の手順 3）。`None` の「いつから」は最も古い側。
type SortKey = (Option<NaiveDate>, DateTime<Utc>, DateTime<Utc>);

fn sort_key(s: &StoredClaim) -> SortKey {
    (
        s.claim.valid_from.key(),
        s.asserted_at.with_timezone(&Utc),
        s.ingested_at,
    )
}

fn out_of(s: &StoredClaim, superseded_by: Option<uuid::Uuid>) -> ClaimOut {
    ClaimOut {
        id: s.claim.id,
        value: s.claim.value.clone(),
        valid_from: s.claim.valid_from.clone(),
        asserted_at: s.asserted_at.to_rfc3339(),
        ingested_at: s.ingested_at.to_rfc3339(),
        supersedes: s.claim.supersedes,
        superseded_by,
        note: s.claim.note.clone(),
    }
}

/// 種類ごとの「いまの値」・予定・積んだ主張・取り消された主張を組む（design D6（仮））。
///
/// **純粋な関数**（DB を持たない）—— 導き方は主張を書き換えずに計算し直せるので、
/// 単体テストで固定しておけば反転条件が満たされたときに安全に変えられる。
///
/// 手順:
/// 1. 本文を消去した主張を落とす（削除の印の付いた行は呼び出し側が除く）
/// 2. 残った主張が指す取り消し先を「取り消された」側へ移す。
///    **取り消された主張がした取り消しも効かせる**ので、指し先は残り全部から集める
/// 3. 比較の鍵 →主張した日時 → D-01 に入った時刻の順で並べる
/// 4. 鍵が今日以前のうち最後が「いまの値」、今日より後は「予定」
/// 5. 積んだ主張は鍵の降順（精度「分からない」は最後）
pub fn view(kinds: &[Kind], claims: &[StoredClaim], today: NaiveDate) -> AttributesView {
    // 1. **消去した主張はどこにも出さない**（spec-review R18）。種類も値も読めない
    let live: Vec<&StoredClaim> = claims.iter().filter(|c| !c.erased).collect();

    // 2. 取り消しの関係。**消えた主張がした取り消しは効かない**（live からだけ集める）
    let superseded_by: std::collections::HashMap<uuid::Uuid, uuid::Uuid> = live
        .iter()
        .filter_map(|c| c.claim.supersedes.map(|target| (target, c.claim.id)))
        .collect();

    let kind_views = kinds
        .iter()
        .map(|k| {
            let mut mine: Vec<&StoredClaim> = live
                .iter()
                .copied()
                .filter(|c| c.claim.kind == k.id)
                .collect();
            // 3. 鍵の昇順。**同じ鍵なら主張した日時が後、それも同じなら D-01 に入った時刻が後**
            mine.sort_by_key(|c| sort_key(c));

            let (active, gone): (Vec<&StoredClaim>, Vec<&StoredClaim>) = mine
                .iter()
                .partition(|c| !superseded_by.contains_key(&c.claim.id));

            // 4. 今日以前の最後が「いまの値」。今日より後は予定（古い順）
            //    **鍵が `None`（分からない）は最も古い側**なので、常に今日以前に入る
            let is_future = |c: &StoredClaim| c.claim.valid_from.key().is_some_and(|d| d > today);
            let current = active
                .iter()
                .rev()
                .find(|c| !is_future(c))
                .map(|c| out_of(c, None));
            let upcoming: Vec<ClaimOut> = active
                .iter()
                .filter(|c| is_future(c))
                .map(|c| out_of(c, None))
                .collect();

            // 5. 積んだ主張は鍵の降順（`None` は最後に落ちる）
            let mut stacked = active.clone();
            stacked.reverse();

            KindView {
                id: k.id,
                name: k.name.clone(),
                current,
                upcoming,
                claims: stacked.iter().map(|c| out_of(c, None)).collect(),
                superseded: gone
                    .iter()
                    .rev()
                    .map(|c| out_of(c, superseded_by.get(&c.claim.id).copied()))
                    .collect(),
            }
        })
        .collect();

    AttributesView {
        today: today.to_string(),
        kinds: kind_views,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    const NONCE: &str = "Zm9vYmFyYmF6cXV4MTIzNDU2";

    fn raw_of(id: uuid::Uuid, kind: uuid::Uuid, body: &str) -> String {
        format!(r#"{{"claim":"{id}","nonce":"{NONCE}","kind":"{kind}",{body}}}"#)
    }

    /// 通る形の主張の原文（値と「いつから」だけを差し替える）。
    fn ok_raw(id: uuid::Uuid, kind: uuid::Uuid) -> String {
        raw_of(
            id,
            kind,
            r#""value":"東京都 目黒区","valid_from":{"precision":"month","date":"2019-10"},"supersedes":null,"note":null"#,
        )
    }

    #[test]
    /// 通る形は通る（以下の否定の検査が「何でも落ちる」で緑にならないように）
    fn well_formed_claim_parses() {
        let id = uuid::Uuid::new_v4();
        let kind = uuid::Uuid::new_v4();
        let c = parse_claim(&ok_raw(id, kind), id).unwrap();
        assert_eq!(c.id, id);
        assert_eq!(c.kind, kind);
        assert_eq!(c.value.as_deref(), Some("東京都 目黒区"));
        assert_eq!(c.valid_from.precision, Precision::Month);
        assert_eq!(c.valid_from.date.as_deref(), Some("2019-10"));
    }

    #[test]
    // Scenario: 値が空の主張は受け付けない
    /// **「なし」（`null`）は空ではない**（深掘り C10）。空白だけの文字列とは分ける
    fn parse_rejects_blank_value() {
        let id = uuid::Uuid::new_v4();
        let kind = uuid::Uuid::new_v4();
        for blank in [r#""""#, r#""   ""#, "\"\\u3000\""] {
            let raw = raw_of(
                id,
                kind,
                &format!(
                    r#""value":{blank},"valid_from":{{"precision":"month","date":"2019-10"}},"supersedes":null,"note":null"#
                ),
            );
            assert_eq!(
                parse_claim(&raw, id),
                Err(ClaimInvalid::Value),
                "空の値 {blank} が通っている"
            );
        }
        // 「なし」は通る
        let none = raw_of(
            id,
            kind,
            r#""value":null,"valid_from":{"precision":"month","date":"2019-10"},"supersedes":null,"note":null"#,
        );
        assert_eq!(parse_claim(&none, id).unwrap().value, None);
    }

    #[test]
    // Scenario: 精度と日付が合わないいつからは受け付けない
    /// **精度を丸めない**（深掘り C3）ので、精度と日付の組が合わないものは断る
    fn parse_rejects_precision_date_mismatch() {
        let id = uuid::Uuid::new_v4();
        let kind = uuid::Uuid::new_v4();
        for (precision, date) in [
            ("year", "\"2019-10-01\""),
            ("year", "\"2019-10\""),
            ("month", "\"2019\""),
            ("month", "\"2019-10-01\""),
            ("day", "\"2019-10\""),
            ("day", "\"2019\""),
            // 「分からない」に日付を添えさせない（読む側が精度と日付で割れる）
            ("unknown", "\"2019-10\""),
            // 日付を欠くのは「分からない」のときだけ
            ("year", "null"),
            ("month", "null"),
            ("day", "null"),
        ] {
            let raw = raw_of(
                id,
                kind,
                &format!(
                    r#""value":"x","valid_from":{{"precision":"{precision}","date":{date}}},"supersedes":null,"note":null"#
                ),
            );
            assert_eq!(
                parse_claim(&raw, id),
                Err(ClaimInvalid::ValidFrom),
                "{precision} / {date} が通っている"
            );
        }
        // 合う組は通る
        for (precision, date) in [
            ("year", "2019"),
            ("month", "2019-10"),
            ("day", "2019-10-01"),
        ] {
            let raw = raw_of(
                id,
                kind,
                &format!(
                    r#""value":"x","valid_from":{{"precision":"{precision}","date":"{date}"}},"supersedes":null,"note":null"#
                ),
            );
            assert!(
                parse_claim(&raw, id).is_ok(),
                "{precision} / {date} が落ちている"
            );
        }
        // 「分からない」は日付なしで通る
        let unknown = raw_of(
            id,
            kind,
            r#""value":"x","valid_from":{"precision":"unknown","date":null},"supersedes":null,"note":null"#,
        );
        assert_eq!(
            parse_claim(&unknown, id).unwrap().valid_from.precision,
            Precision::Unknown
        );
    }

    #[test]
    // Scenario: 暦に無いいつからは受け付けない
    fn parse_rejects_impossible_dates() {
        let id = uuid::Uuid::new_v4();
        let kind = uuid::Uuid::new_v4();
        for (precision, date) in [
            ("day", "2019-02-30"),
            ("day", "2019-13-01"),
            ("day", "2019-00-10"),
            ("day", "2019-04-31"),
            ("month", "2019-13"),
            ("month", "2019-00"),
        ] {
            let raw = raw_of(
                id,
                kind,
                &format!(
                    r#""value":"x","valid_from":{{"precision":"{precision}","date":"{date}"}},"supersedes":null,"note":null"#
                ),
            );
            assert_eq!(
                parse_claim(&raw, id),
                Err(ClaimInvalid::ValidFrom),
                "{date} が通っている"
            );
        }
        // うるう年の 2 月 29 日は暦にある
        let leap = raw_of(
            id,
            kind,
            r#""value":"x","valid_from":{"precision":"day","date":"2020-02-29"},"supersedes":null,"note":null"#,
        );
        assert!(parse_claim(&leap, id).is_ok(), "2020-02-29 が落ちている");
    }

    #[test]
    // Scenario: 乱数が短い主張は受け付けない
    /// **128 bit に満たない乱数は総当たりの範囲に入る**（design D4 / 深掘り C12）——
    /// 消去の後に残る列と値の候補から原文を組み直して鍵と照らせる
    fn parse_rejects_short_nonce() {
        let id = uuid::Uuid::new_v4();
        let kind = uuid::Uuid::new_v4();
        // 64 bit を base64url で書くと 11 文字
        // 21 文字（= 126 bit 未満）までは断る。**境目は 22 文字**
        for short in ["", "abc", "MTIzNDU2Nzg", "Zm9vYmFyYmF6cXV4MTIzN"] {
            let raw = format!(
                r#"{{"claim":"{id}","nonce":"{short}","kind":"{kind}","value":"x","valid_from":{{"precision":"year","date":"2019"}},"supersedes":null,"note":null}}"#
            );
            assert_eq!(
                parse_claim(&raw, id),
                Err(ClaimInvalid::Malformed),
                "{} 文字の乱数が通っている",
                short.chars().count()
            );
        }
        // **22 文字ちょうどは通る**（境目を両側から押さえる）
        for enough in ["Zm9vYmFyYmF6cXV4MTIzND", NONCE] {
            assert!(enough.chars().count() >= NONCE_MIN_CHARS);
            let ok = format!(
                r#"{{"claim":"{id}","nonce":"{enough}","kind":"{kind}","value":"x","valid_from":{{"precision":"year","date":"2019"}},"supersedes":null,"note":null}}"#
            );
            assert!(parse_claim(&ok, id).is_ok(), "{enough} が落ちている");
        }
        // base64url の外の文字は断る（乱数の強さを文字数で測っているので、詰め物を混ぜさせない）
        let odd = format!(
            r#"{{"claim":"{id}","nonce":"++++++++++++++++++++++","kind":"{kind}","value":"x","valid_from":{{"precision":"year","date":"2019"}},"supersedes":null,"note":null}}"#
        );
        assert_eq!(parse_claim(&odd, id), Err(ClaimInvalid::Malformed));
    }

    #[test]
    /// 原文が JSON でない・必須の欄が欠ける・識別子が記録と違う（表の `malformed_claim`）
    fn parse_rejects_malformed() {
        let id = uuid::Uuid::new_v4();
        let kind = uuid::Uuid::new_v4();
        assert_eq!(parse_claim("not json", id), Err(ClaimInvalid::Malformed));
        assert_eq!(parse_claim("[]", id), Err(ClaimInvalid::Malformed));
        // **原文の識別子が記録の識別子と違う**（消去の後にどちらがその主張かを決められない）
        assert_eq!(
            parse_claim(&ok_raw(uuid::Uuid::new_v4(), kind), id),
            Err(ClaimInvalid::Malformed),
            "原文と記録で識別子が食い違う主張が通っている"
        );
        // 必須の欄を 1 つずつ落とす
        for missing in ["claim", "nonce", "kind", "value", "valid_from"] {
            let mut v: serde_json::Value = serde_json::from_str(&ok_raw(id, kind)).unwrap();
            v.as_object_mut().unwrap().remove(missing);
            assert_eq!(
                parse_claim(&v.to_string(), id),
                Err(ClaimInvalid::Malformed),
                "{missing} が欠けても通っている"
            );
        }
    }

    #[test]
    /// **解析済みに乱数を写さない**（design D4 / 深掘り C12）。文字列は NFC にする
    fn parse_builds_payload_without_nonce() {
        let id = uuid::Uuid::new_v4();
        let kind = uuid::Uuid::new_v4();
        // "が" を NFD（か + 濁点）で書いたもの
        let raw = raw_of(
            id,
            kind,
            r#""value":"\u304B\u3099","valid_from":{"precision":"year","date":"2019"},"supersedes":null,"note":"\u304B\u3099""#,
        );
        let c = parse_claim(&raw, id).unwrap();
        assert_eq!(
            c.value.as_deref(),
            Some("\u{304C}"),
            "値が NFC になっていない"
        );
        assert_eq!(
            c.note.as_deref(),
            Some("\u{304C}"),
            "補足が NFC になっていない"
        );
        let text = c.payload.to_string();
        assert!(!text.contains(NONCE), "解析済みに乱数が写っている: {text}");
        assert!(!text.contains("nonce"), "解析済みに乱数の欄がある: {text}");
        assert_eq!(c.payload["value"], serde_json::json!("\u{304C}"));
        assert_eq!(
            c.payload["valid_from"]["precision"],
            serde_json::json!("year")
        );
    }

    // ------------------------------------------------------------------ いまの値の導き方

    fn at(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    /// 主張 1 件を組む。`vf` は `("month", Some("2019-10"))` のような組。
    fn stored(
        kind: uuid::Uuid,
        value: Option<&str>,
        precision: Precision,
        date: Option<&str>,
        asserted: &str,
    ) -> StoredClaim {
        StoredClaim {
            claim: Claim {
                id: uuid::Uuid::new_v4(),
                kind,
                value: value.map(str::to_string),
                valid_from: ValidFrom {
                    precision,
                    date: date.map(str::to_string),
                },
                supersedes: None,
                note: None,
                payload: serde_json::json!({}),
            },
            asserted_at: at(asserted),
            ingested_at: at(asserted).with_timezone(&Utc),
            erased: false,
        }
    }

    fn one_kind(id: uuid::Uuid) -> Vec<Kind> {
        vec![Kind {
            id,
            name: "住所".into(),
        }]
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()
    }

    #[test]
    // Scenario: いつからが最も新しい主張がいまの値になる
    fn view_current_is_newest_valid_from() {
        let k = uuid::Uuid::new_v4();
        let old = stored(
            k,
            Some("A"),
            Precision::Month,
            Some("2017-04"),
            "2026-09-01T10:00:00+09:00",
        );
        let new = stored(
            k,
            Some("B"),
            Precision::Day,
            Some("2023-03-18"),
            "2026-09-02T10:00:00+09:00",
        );
        // **並びに依らない**（入力の順を変えても同じ結論になる）
        for input in [
            vec![old.clone(), new.clone()],
            vec![new.clone(), old.clone()],
        ] {
            let v = view(&one_kind(k), &input, today());
            assert_eq!(
                v.kinds[0].current.as_ref().unwrap().value.as_deref(),
                Some("B")
            );
        }
    }

    #[test]
    // Scenario: 同じいつからなら主張した日時が後の主張がいまの値になる
    fn view_ties_break_on_asserted_at() {
        let k = uuid::Uuid::new_v4();
        let first = stored(
            k,
            Some("A"),
            Precision::Month,
            Some("2019-10"),
            "2026-09-01T10:00:00+09:00",
        );
        let later = stored(
            k,
            Some("B"),
            Precision::Month,
            Some("2019-10"),
            "2026-09-05T10:00:00+09:00",
        );
        let v = view(&one_kind(k), &[first, later], today());
        assert_eq!(
            v.kinds[0].current.as_ref().unwrap().value.as_deref(),
            Some("B")
        );
    }

    #[test]
    // Scenario: 未来のいつからは予定に出ていまの値にならない
    fn view_future_goes_to_upcoming() {
        let k = uuid::Uuid::new_v4();
        let now = stored(
            k,
            Some("A"),
            Precision::Month,
            Some("2019-10"),
            "2026-09-01T10:00:00+09:00",
        );
        let soon = stored(
            k,
            Some("B"),
            Precision::Day,
            Some("2026-10-01"),
            "2026-09-02T10:00:00+09:00",
        );
        let v = view(&one_kind(k), &[now, soon], today());
        assert_eq!(
            v.kinds[0].current.as_ref().unwrap().value.as_deref(),
            Some("A")
        );
        assert_eq!(v.kinds[0].upcoming.len(), 1);
        assert_eq!(v.kinds[0].upcoming[0].value.as_deref(), Some("B"));
    }

    #[test]
    // Scenario: 積んだ主張はいまの値と予定を含む
    fn view_stacked_includes_current_and_upcoming() {
        let k = uuid::Uuid::new_v4();
        let old = stored(
            k,
            Some("A"),
            Precision::Month,
            Some("2013-04"),
            "2026-09-01T10:00:00+09:00",
        );
        let now = stored(
            k,
            Some("B"),
            Precision::Month,
            Some("2019-10"),
            "2026-09-02T10:00:00+09:00",
        );
        let soon = stored(
            k,
            Some("C"),
            Precision::Day,
            Some("2026-10-01"),
            "2026-09-03T10:00:00+09:00",
        );
        let v = view(&one_kind(k), &[old, now, soon], today());
        let got: Vec<_> = v.kinds[0]
            .claims
            .iter()
            .map(|c| c.value.clone().unwrap())
            .collect();
        assert_eq!(got, ["C", "B", "A"], "「いつから」の新しい順になっていない");
    }

    #[test]
    // Scenario: いつからを直す訂正で古い開始が残らない
    fn view_correcting_valid_from_drops_the_old_start() {
        let k = uuid::Uuid::new_v4();
        let wrong = stored(
            k,
            Some("A"),
            Precision::Month,
            Some("2019-04"),
            "2026-09-01T10:00:00+09:00",
        );
        let mut right = stored(
            k,
            Some("A"),
            Precision::Month,
            Some("2019-10"),
            "2026-09-02T10:00:00+09:00",
        );
        right.claim.supersedes = Some(wrong.claim.id);
        let v = view(&one_kind(k), &[wrong.clone(), right.clone()], today());
        assert!(
            v.kinds[0]
                .claims
                .iter()
                .all(|c| c.valid_from.date.as_deref() != Some("2019-04")),
            "取り消した 2019 年 4 月の開始が積んだ主張に残っている"
        );
        assert_eq!(v.kinds[0].superseded.len(), 1);
        assert_eq!(v.kinds[0].superseded[0].id, wrong.claim.id);
        assert_eq!(
            v.kinds[0].superseded[0].superseded_by,
            Some(right.claim.id),
            "どの主張に取り消されたかが添えられていない"
        );
    }

    #[test]
    // Scenario: 年だけの主張はその年の初めから有効とみなす
    fn view_year_precision_starts_in_january() {
        let k = uuid::Uuid::new_v4();
        let day = stored(
            k,
            Some("A"),
            Precision::Day,
            Some("2023-03-18"),
            "2026-09-01T10:00:00+09:00",
        );
        let year = stored(
            k,
            Some("B"),
            Precision::Year,
            Some("2026"),
            "2026-09-02T10:00:00+09:00",
        );
        let v = view(&one_kind(k), &[day, year], today());
        assert_eq!(
            v.kinds[0].current.as_ref().unwrap().value.as_deref(),
            Some("B")
        );
        // **比較で丸めても、返す「いつから」は精度のまま**（深掘り C3）
        assert_eq!(
            v.kinds[0].current.as_ref().unwrap().valid_from.precision,
            Precision::Year
        );
        assert_eq!(
            v.kinds[0]
                .current
                .as_ref()
                .unwrap()
                .valid_from
                .date
                .as_deref(),
            Some("2026")
        );
    }

    #[test]
    // Scenario: いつからが分からない主張は最も古い側に置く
    fn view_unknown_valid_from_sorts_oldest() {
        let k = uuid::Uuid::new_v4();
        let unknown = stored(
            k,
            Some("A"),
            Precision::Unknown,
            None,
            "2026-09-01T10:00:00+09:00",
        );
        let known = stored(
            k,
            Some("B"),
            Precision::Month,
            Some("2013-04"),
            "2026-09-02T10:00:00+09:00",
        );
        let v = view(&one_kind(k), &[unknown, known], today());
        assert_eq!(
            v.kinds[0].current.as_ref().unwrap().value.as_deref(),
            Some("B")
        );
        let last = v.kinds[0].claims.last().unwrap();
        assert_eq!(
            last.valid_from.precision,
            Precision::Unknown,
            "並びの最後が「分からない」でない"
        );
    }

    #[test]
    // Scenario: なしの主張がいまの値になる
    /// **「なし」は値が無いことではない**（深掘り C10）—— 置き換える値が無い終わり方
    fn view_none_claim_can_be_current() {
        let k = uuid::Uuid::new_v4();
        let had = stored(
            k,
            Some("A"),
            Precision::Month,
            Some("2019-10"),
            "2026-09-01T10:00:00+09:00",
        );
        let ended = stored(
            k,
            None,
            Precision::Month,
            Some("2024-03"),
            "2026-09-02T10:00:00+09:00",
        );
        let v = view(&one_kind(k), &[had, ended.clone()], today());
        let current = v.kinds[0].current.as_ref().unwrap();
        assert_eq!(
            current.id, ended.claim.id,
            "「なし」がいまの値になっていない"
        );
        assert_eq!(current.value, None);
    }

    #[test]
    // Scenario: 取り消された主張がした取り消しも効く
    /// C が B を取り消しても、**B が A にした取り消しは残る** ——
    /// 効かなくすると、訂正を訂正した瞬間に古い値が黙って復活する
    fn view_supersession_by_a_superseded_claim_still_counts() {
        let k = uuid::Uuid::new_v4();
        let a = stored(
            k,
            Some("A"),
            Precision::Month,
            Some("2013-04"),
            "2026-09-01T10:00:00+09:00",
        );
        let mut b = stored(
            k,
            Some("B"),
            Precision::Month,
            Some("2019-10"),
            "2026-09-02T10:00:00+09:00",
        );
        b.claim.supersedes = Some(a.claim.id);
        let mut c = stored(
            k,
            Some("C"),
            Precision::Month,
            Some("2023-03"),
            "2026-09-03T10:00:00+09:00",
        );
        c.claim.supersedes = Some(b.claim.id);
        let v = view(&one_kind(k), &[a.clone(), b.clone(), c.clone()], today());
        let gone: Vec<_> = v.kinds[0].superseded.iter().map(|x| x.id).collect();
        assert!(gone.contains(&a.claim.id), "A が取り消された主張に無い");
        assert!(gone.contains(&b.claim.id), "B が取り消された主張に無い");
        assert_eq!(v.kinds[0].claims.len(), 1);
        assert_eq!(v.kinds[0].claims[0].id, c.claim.id);
    }

    #[test]
    /// **消去した主張はどこにも出ない**（spec-review R18）。その取り消しも効かない ——
    /// 種類も値も読めない行を並べる置き場は無い
    fn view_drops_erased_claims_and_their_supersession() {
        let k = uuid::Uuid::new_v4();
        let a = stored(
            k,
            Some("A"),
            Precision::Month,
            Some("2013-04"),
            "2026-09-01T10:00:00+09:00",
        );
        let mut b = stored(
            k,
            Some("B"),
            Precision::Month,
            Some("2019-10"),
            "2026-09-02T10:00:00+09:00",
        );
        b.claim.supersedes = Some(a.claim.id);
        b.erased = true;
        let v = view(&one_kind(k), &[a.clone(), b.clone()], today());
        let seen: Vec<_> = v.kinds[0]
            .claims
            .iter()
            .chain(&v.kinds[0].superseded)
            .map(|x| x.id)
            .collect();
        assert!(!seen.contains(&b.claim.id), "消去した主張が出ている");
        assert_eq!(seen, vec![a.claim.id], "消去した主張の取り消しが効いたまま");
        assert_eq!(v.kinds[0].current.as_ref().unwrap().id, a.claim.id);
    }

    #[test]
    /// 種類は渡された順に返り、**他の種類の主張が混ざらない**（Q4: 1 種類 1 値）
    fn view_keeps_kinds_separate_and_in_order() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let kinds = vec![
            Kind {
                id: a,
                name: "住所".into(),
            },
            Kind {
                id: b,
                name: "職業".into(),
            },
        ];
        let ca = stored(
            a,
            Some("東京"),
            Precision::Year,
            Some("2019"),
            "2026-09-01T10:00:00+09:00",
        );
        let cb = stored(
            b,
            Some("会社員"),
            Precision::Year,
            Some("2020"),
            "2026-09-02T10:00:00+09:00",
        );
        let v = view(&kinds, &[cb, ca], today());
        assert_eq!(
            v.kinds.iter().map(|k| k.name.as_str()).collect::<Vec<_>>(),
            ["住所", "職業"]
        );
        assert_eq!(v.kinds[0].claims.len(), 1);
        assert_eq!(
            v.kinds[0].current.as_ref().unwrap().value.as_deref(),
            Some("東京")
        );
        assert_eq!(
            v.kinds[1].current.as_ref().unwrap().value.as_deref(),
            Some("会社員")
        );
    }

    #[test]
    /// 主張を 1 件も持たない種類は「いまの値」が無い（画面は「まだ書いていない」と出す）
    fn view_kind_without_claims_has_no_current() {
        let k = uuid::Uuid::new_v4();
        let v = view(&one_kind(k), &[], today());
        assert!(v.kinds[0].current.is_none());
        assert!(v.kinds[0].claims.is_empty());
        assert_eq!(v.today, "2026-09-15");
    }
}
