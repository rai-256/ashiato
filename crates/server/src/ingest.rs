//! 取り込みの契約。**Kotlin(C-01) と Rust(C-02) の 2 実装が同じ形を送る**ので、
//! 形と冪等キーの作り方はここが単一の情報源になる（製造準備 A-1）。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub raw: serde_json::Value,
    pub payload: serde_json::Value,
}

/// 冪等キー。**原文と出来事の時刻とソースだけから作る** ——
/// 収集側が採番した id を混ぜると、再送のたびに別物になって重複が入る。
pub fn content_hash(req: &IngestRequest) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    req.logical_source.hash(&mut h);
    req.event_time.timestamp_micros().hash(&mut h);
    req.raw.to_string().hash(&mut h);
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
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
            raw: serde_json::json!({ "v": raw }),
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
}
