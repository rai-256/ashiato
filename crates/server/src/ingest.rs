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

/// 受け取り時に断る理由。**アプリ層で閉じる** —— DB の制約に任せると 500 になり、
/// 呼び出し側から「自分の要求が悪い」と分からない（design D5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    /// 由来の分類が列挙のどれでもない
    Origin,
    /// 原文が空、または DB に格納できないバイトを含む
    Raw,
    /// 「収集した」記録なのに端末識別子が無い
    DeviceId,
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

    /// 格納の前に断るものを 1 か所で見る。
    ///
    /// **DB へ届かせてはいけない値がある。** PostgreSQL の `text` は U+0000 を格納できず
    /// （SQLSTATE 22021）、届くと `ingest_one` が `Err` を返してまとめ送り全体が 500 になる。
    /// 収集側は本文を読めず 1 件も取り除けないので、**その 1 件が後続を永久に止める**。
    ///
    /// 空の原文も断る。冪等キーは `logical_source` + `event_time` + `raw` だけから作るので、
    /// 原文が空だと**同じ時刻の別々の記録が 1 行に畳まれ**、収集側には
    /// `duplicate: true`（＝受理）として返る —— 記録が正常応答の顔をして消える。
    pub fn validate(&self) -> Result<(), Invalid> {
        if !self.origin_is_known() {
            return Err(Invalid::Origin);
        }
        if self.raw.is_empty() || self.raw.contains('\0') {
            return Err(Invalid::Raw);
        }
        // 「収集した」記録は spec が端末識別子を要求する（「どの端末が生成したか」）。
        // 本人が書いた記録・派生させた記録に端末は無いので、そこは求めない。
        if self.origin == "collected" && self.device_id.as_deref().unwrap_or_default().is_empty() {
            return Err(Invalid::DeviceId);
        }
        Ok(())
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
            device_id: Some("device-1".into()),
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
    /// 列挙にない由来は弾ける（tasks 2.5）。**3 分類すべてを通す** ——
    /// `collected` だけ見ていると `ORIGINS` を縮めても気付かない（review R19）
    fn origin_enum_is_closed() {
        let mut r = req(uuid::Uuid::nil(), "x");
        for known in ["collected", "authored", "derived"] {
            r.origin = known.into();
            assert!(r.origin_is_known(), "{known} が弾かれている");
        }
        for unknown in ["guessed", "Collected", "", "collected "] {
            r.origin = unknown.into();
            assert!(!r.origin_is_known(), "{unknown} が通っている");
        }
    }

    #[test]
    /// **DB へ届かせてはいけない原文を、格納の前に断る**（review R11 / R18）。
    ///
    /// PostgreSQL の `text` は U+0000 を格納できない。届くとまとめ送り全体が 500 になり、
    /// 収集側は本文を読めず 1 件も取り除けない —— **その 1 件が後続を永久に止める**。
    fn raw_that_cannot_be_stored_is_rejected() {
        let mut r = req(uuid::Uuid::nil(), "x");
        assert_eq!(r.validate(), Ok(()));

        r.raw = "{\"v\":\"\0\"}".into();
        assert_eq!(
            r.validate(),
            Err(Invalid::Raw),
            "NUL を含む原文が通っている"
        );

        r.raw = String::new();
        assert_eq!(r.validate(), Err(Invalid::Raw), "空の原文が通っている");
    }

    #[test]
    /// **空の原文は冪等キーを潰す。** 断らないと、同じ時刻の別々の記録が 1 行に畳まれ、
    /// 収集側には `duplicate: true`（＝受理）として返る —— 記録が正常応答の顔をして消える
    fn empty_raw_would_collapse_distinct_records() {
        let mut a = req(uuid::Uuid::nil(), "x");
        let mut b = req(uuid::Uuid::nil(), "y");
        a.raw = String::new();
        b.raw = String::new();
        assert_eq!(content_hash(&a), content_hash(&b), "前提が変わっている");
        assert_eq!(a.validate(), Err(Invalid::Raw), "だから断らねばならない");
    }

    #[test]
    /// 「収集した」記録には端末識別子が要る（spec「どの端末が生成したか」/ review R3）。
    /// **本人が書いた記録・派生させた記録には求めない** —— そこに端末は無い
    fn collected_records_require_a_device() {
        let mut r = req(uuid::Uuid::nil(), "x");
        r.device_id = None;
        assert_eq!(r.validate(), Err(Invalid::DeviceId));
        r.device_id = Some(String::new());
        assert_eq!(r.validate(), Err(Invalid::DeviceId), "空文字が通っている");

        for other in ["authored", "derived"] {
            r.origin = other.into();
            assert_eq!(r.validate(), Ok(()), "{other} に端末を求めている");
        }
    }
}
