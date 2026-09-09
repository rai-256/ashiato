// SPDX-License-Identifier: AGPL-3.0-only
//! 取り込みの契約。**Kotlin(C-01) と Rust(C-02) の 2 実装が同じ形を送る**ので、
//! 形と冪等キーの作り方はここが単一の情報源になる（製造準備 A-1）。
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization as _;

/// 単位系の既定。収集側が省略したときにサーバが入れる（design D4）。
pub const DEFAULT_UNIT_SYSTEM: &str = "si";
/// 座標系の既定。同上。
pub const DEFAULT_CRS: &str = "EPSG:4326";

/// 由来の分類。**この 3 つ以外は受け取り時に 400 で落とす**（design D5）。
pub const ORIGINS: [&str; 3] = ["collected", "authored", "derived"];

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestRequest {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub logical_source: String,
    pub external_id: Option<String>,
    pub device_id: Option<String>,
    pub origin: String,
    pub event_time: chrono::DateTime<chrono::Utc>,
    pub tz_offset_min: i32,
    pub tz_id: String,
    pub schema_version: i32,
    /// 単位系。省略時は `si`（FR-28 / design D4）
    #[serde(default)]
    pub unit_system: Option<String>,
    /// 座標系。省略時は `EPSG:4326`（FR-28 / design D4）
    #[serde(default)]
    pub crs: Option<String>,
    /// 取得元から受け取った原文。**文字列で持つ**（深掘り 第 2 回 / design D16）——
    /// JSON 型に入れると DB がキー順・重複キー・数値表記を正規化し、
    /// 「バイト単位で一致する」が成り立たなくなる。
    pub raw: String,
    pub payload: serde_json::Value,
}

impl IngestRequest {
    /// 収集側が省略した単位系を既定で埋めた値を返す。
    pub fn unit_system_or_default(&self) -> &str {
        self.unit_system.as_deref().unwrap_or(DEFAULT_UNIT_SYSTEM)
    }

    /// 収集側が省略した座標系を既定で埋めた値を返す。
    pub fn crs_or_default(&self) -> &str {
        self.crs.as_deref().unwrap_or(DEFAULT_CRS)
    }

    /// 由来の分類が列挙のどれかであることを保証する。
    pub fn origin_is_known(&self) -> bool {
        ORIGINS.contains(&self.origin.as_str())
    }
}

/// 冪等キー。**原文と出来事の時刻とソースだけから作る** ——
/// 収集側が採番した id を混ぜると、再送のたびに別物になって重複が入る。
///
/// SHA-256 を使う（design D1）。`DefaultHasher` は std が版をまたぐ安定性を
/// 保証しておらず、**コンパイラを上げた日に全件が重複として二重に入る**。
///
/// 各項目は長さを前置してから混ぜる。前置しないと
/// `("ab", "c")` と `("a", "bc")` が同じ鍵になる。
///
/// **原文は受け取った文字列そのものを混ぜる**（design D16）—— 構造として解釈し直すと、
/// 保存する値（文字列）と鍵の入力（正規化された構造）がずれる。
/// 同じ内容でも表記が違えば別の鍵になるが、収集側の直列化は決まった形なので実害は無い。
pub fn content_hash(req: &IngestRequest) -> String {
    use sha2::{Digest as _, Sha256};
    let mut h = Sha256::new();
    let mut field = |bytes: &[u8]| {
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    };
    field(req.logical_source.as_bytes());
    field(&req.event_time.timestamp_micros().to_be_bytes());
    field(req.raw.as_bytes());
    format!("{:x}", h.finalize())
}

/// JSON に含まれる文字列を再帰的に Unicode NFC へ揃えた値を返す。
///
/// **`payload` にだけ使う。`raw` には決して使わない**（design D2 / FR-18）——
/// 原文のバイト列は一度変換すると二度と戻らない。
/// オブジェクトのキーも対象にする。検索・照合はキーにも当たるため。
pub fn to_nfc(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => serde_json::Value::String(s.nfc().collect()),
        serde_json::Value::Array(a) => serde_json::Value::Array(a.iter().map(to_nfc).collect()),
        serde_json::Value::Object(o) => serde_json::Value::Object(
            o.iter()
                .map(|(k, v)| (k.nfc().collect::<String>(), to_nfc(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn req(id: uuid::Uuid, raw: &str) -> IngestRequest {
        IngestRequest {
            id,
            user_id: uuid::Uuid::nil(),
            logical_source: "test".into(),
            external_id: None,
            device_id: None,
            origin: "collected".into(),
            event_time: chrono::DateTime::from_timestamp(1_757_000_000, 0).unwrap(),
            tz_offset_min: 540,
            tz_id: "Asia/Tokyo".into(),
            schema_version: 1,
            unit_system: None,
            crs: None,
            raw: format!(r#"{{"v":"{raw}"}}"#),
            payload: serde_json::json!({}),
        }
    }

    #[test]
    /// 同じ原文なら、収集側が採番した id が違っても同じ鍵になる
    fn same_raw_same_hash() {
        let a = content_hash(&req(uuid::Uuid::new_v4(), "x"));
        let b = content_hash(&req(uuid::Uuid::new_v4(), "x"));
        assert_eq!(a, b, "再送で重複が入る");
    }

    #[test]
    /// 原文が違えば別の鍵になる
    fn different_raw_different_hash() {
        let a = content_hash(&req(uuid::Uuid::nil(), "x"));
        let b = content_hash(&req(uuid::Uuid::nil(), "y"));
        assert_ne!(a, b);
    }

    #[test]
    /// **鍵が固定値である。** 既知の入力に対する期待値を直書きする ——
    /// これが無いと、鍵の作り方が変わって保存済みが全部ずれても誰も気付かない（tasks 1.3）。
    ///
    /// 期待値はこの実装の出力を写したものではなく、**別実装で独立に算出した**:
    /// ```text
    /// python3 -c 'import hashlib,struct
    /// h=hashlib.sha256()
    /// f=lambda b:(h.update(struct.pack(">Q",len(b))),h.update(b))
    /// f(b"test"); f(struct.pack(">q",1757000000*1000000)); f(b"{\"v\":\"x\"}")
    /// print(h.hexdigest())'
    /// ```
    /// 他言語から同じ鍵を再現できることの確認も兼ねる（design D1 のリスク欄）。
    fn hash_is_pinned() {
        assert_eq!(
            content_hash(&req(uuid::Uuid::nil(), "x")),
            "39d0ebc5c3c1d17a5deec93e03e60c3be02e5ddd741a2d5575b789c16ee15df1",
            "冪等キーの作り方が変わっている。保存済みの記録が全部ずれる"
        );
    }

    #[test]
    /// 項目の境目が曖昧でない —— ソース名と原文の切れ目がずれても別の鍵になる
    fn field_boundaries_are_unambiguous() {
        let mut a = req(uuid::Uuid::nil(), "x");
        a.logical_source = "ab".into();
        let mut b = req(uuid::Uuid::nil(), "x");
        b.logical_source = "a".into();
        assert_ne!(content_hash(&a), content_hash(&b));
    }

    #[test]
    /// NFD の濁点が NFC になる（tasks 2.1）
    fn nfc_composes_dakuten() {
        // "が" を NFD（か + 濁点）で書いたもの
        let nfd = "\u{304B}\u{3099}";
        let out = to_nfc(&serde_json::json!({ "k": nfd }));
        assert_eq!(out["k"], serde_json::json!("\u{304C}"));
    }

    #[test]
    /// 入れ子の配列・オブジェクトとキーまで届く
    fn nfc_reaches_nested_and_keys() {
        let nfd = "\u{304B}\u{3099}";
        let out = to_nfc(&serde_json::json!({ nfd: [{ "inner": nfd }] }));
        assert_eq!(out["\u{304C}"][0]["inner"], serde_json::json!("\u{304C}"));
    }

    #[test]
    /// 文字列以外は素通しする
    fn nfc_leaves_non_strings() {
        let v = serde_json::json!({ "n": 1.5, "b": true, "z": null });
        assert_eq!(to_nfc(&v), v);
    }

    #[test]
    /// 単位系と座標系は省略できて、既定が入る（tasks 2.3）
    fn units_default_when_absent() {
        let r = req(uuid::Uuid::nil(), "x");
        assert_eq!(r.unit_system_or_default(), "si");
        assert_eq!(r.crs_or_default(), "EPSG:4326");
    }

    #[test]
    /// 指定した座標系はそのまま使われる（tasks 2.4）
    fn crs_is_taken_as_given() {
        let mut r = req(uuid::Uuid::nil(), "x");
        r.crs = Some("EPSG:6668".into());
        assert_eq!(r.crs_or_default(), "EPSG:6668");
    }

    #[test]
    /// 単位系と座標系を省いた JSON がそのまま解釈できる（既存の収集側を壊さない）
    fn units_are_optional_in_json() {
        let json = serde_json::json!({
            "id": uuid::Uuid::nil(), "user_id": uuid::Uuid::nil(),
            "logical_source": "test", "external_id": null, "device_id": null,
            "origin": "collected", "event_time": "2026-09-08T02:00:00Z",
            "tz_offset_min": 540, "tz_id": "Asia/Tokyo", "schema_version": 1,
            "raw": "{}", "payload": {}
        });
        let r: IngestRequest = serde_json::from_value(json).unwrap();
        assert_eq!(r.unit_system_or_default(), "si");
    }

    #[test]
    /// **原文は文字列でなければ受け取らない**（design D16 / tasks 9.2）——
    /// JSON の値として受けると、直列化のたびに並びと表記が正規化されて
    /// 「受け取ったまま」が成り立たなくなる。
    fn raw_must_be_a_string() {
        let mut json = serde_json::json!({
            "id": uuid::Uuid::nil(), "user_id": uuid::Uuid::nil(),
            "logical_source": "test", "external_id": null, "device_id": null,
            "origin": "collected", "event_time": "2026-09-08T02:00:00Z",
            "tz_offset_min": 540, "tz_id": "Asia/Tokyo", "schema_version": 1,
            "raw": "{}", "payload": {}
        });
        assert!(serde_json::from_value::<IngestRequest>(json.clone()).is_ok());
        json["raw"] = serde_json::json!({ "lat": 1 });
        assert!(
            serde_json::from_value::<IngestRequest>(json).is_err(),
            "オブジェクトの原文が通ってしまう（受け取った表記が失われる）"
        );
    }

    #[test]
    /// **鍵は原文の「文字列」に従い、構造には従わない**（design D16 / tasks 9.2）。
    ///
    /// これが逆向き（構造に従う）だと、保存する値（受け取った文字列）と
    /// 鍵の入力（正規化された構造）がずれ、**同じ行に別の鍵が立ちうる**。
    ///
    /// 期待値は実装の出力を写したものではなく、別実装で独立に算出した:
    /// ```text
    /// python3 -c 'import hashlib,struct
    /// h=hashlib.sha256()
    /// f=lambda b:(h.update(struct.pack(">Q",len(b))),h.update(b))
    /// f(b"test"); f(struct.pack(">q",1757000000*1000000)); f(b"{ \"v\" : \"x\" }")
    /// print(h.hexdigest())'
    /// ```
    fn hash_follows_text_not_structure() {
        let mut spaced = req(uuid::Uuid::nil(), "x");
        spaced.raw = r#"{ "v" : "x" }"#.into();
        assert_eq!(
            content_hash(&spaced),
            "738cb0caac2d95b8463218c2b3f40f3a857d308a73b03c2ef7421a141315accf",
            "原文を構造として解釈し直している（保存する値と鍵の入力がずれる）"
        );
        assert_ne!(
            content_hash(&spaced),
            content_hash(&req(uuid::Uuid::nil(), "x")),
            "表記の違う原文が同じ鍵になっている"
        );
    }

    #[test]
    /// 列挙にない由来は弾ける（tasks 2.5）
    fn origin_enum_is_closed() {
        let mut r = req(uuid::Uuid::nil(), "x");
        assert!(r.origin_is_known());
        r.origin = "guessed".into();
        assert!(!r.origin_is_known());
    }
}
