// SPDX-License-Identifier: AGPL-3.0-only
//! 生存信号の契約（FR-78 / 深掘り Q1, Q5, 第 4 回 Q13, Q14, 第 5 回 Q17）。
//!
//! **記録が 0 件の日の意味を残す唯一の材料。** FR-33 は記録を生成したときにしか
//! 稼働記録を書かないので、記録が 0 件の日は行が無いだけになり、扉 #14 が求める
//! 「動きが無かったのか / 収集が壊れていたのか」を区別できない。**区別は遡って作れない。**
use serde::{Deserialize, Serialize};

/// 収集側が送る 1 件。**`/ingest` と混ぜない**（design D9）——
/// 記録のエンベロープ（`tz_id` / `schema_version` / `crs` …）を生存信号は 1 つも持たない。
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HeartbeatRequest {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub logical_source: String,
    pub device_id: Option<String>,
    /// 収集側が信号を作った時刻。**受信時刻ではない** ——
    /// 圏外で溜めて送ると受信は数日後になる（第 6 回 Q24 と同じ理由）。
    pub emitted_at: chrono::DateTime<chrono::Utc>,
    /// そのソースを取得できる状態か（権限・センサ・接続。深掘り Q5）
    pub capturable: bool,
    /// 取得できないとき、何が満たされていないか。`capturable = false` なら空にできない
    #[serde(default)]
    pub blockers: Vec<String>,
    /// 前回の生存信号からの取得の試行回数（第 5 回 Q17）
    pub attempts: i32,
    /// そのうち成功した回数（同上）
    pub successes: i32,
    /// 収集側から受け取った原文。**素通しで残す**（0003 と同じ理由で `text`）
    pub raw: String,
}

/// 受け取り時に断る理由。**アプリ層で閉じる**（`ingest::Invalid` と同じ向き）——
/// DB の制約に任せると 500 になり、呼び出し側から「自分の要求が悪い」と分からない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    /// 原文が空、または DB に格納できないバイトを含む
    Raw,
    /// 取得できない状態を報告しながら、何が満たされていないかを持たない
    Blockerless,
    /// 取得の回数が負、または成功が試行を超える
    Counts,
}

impl HeartbeatRequest {
    /// 格納の前に断るものを 1 か所で見る。
    ///
    /// **理由の無い「取れない」を断る**のは、それが状態③の証拠にならないため ——
    /// 「取れない状態だった」とだけ残っても、権限なのかセンサなのか接続なのかが
    /// 分からなければ、扉 #14 が求めた区別に届かない（specs）。
    ///
    /// **成功が試行を超える信号を断る**のは、取得率が 1 を超えると
    /// 「眠っていた / 生きていた」の区別が数として壊れるため（2 巡目 R7）。
    /// 回数そのものを持たない信号は、この型に入る前（serde）で落ちる。
    pub fn validate(&self) -> Result<(), Invalid> {
        if self.raw.is_empty() || self.raw.contains('\0') {
            return Err(Invalid::Raw);
        }
        if !self.capturable && self.blockers.iter().all(|b| b.trim().is_empty()) {
            return Err(Invalid::Blockerless);
        }
        if self.attempts < 0 || self.successes < 0 || self.successes > self.attempts {
            return Err(Invalid::Counts);
        }
        Ok(())
    }
}

/// 冪等キー。**原文と発信の時刻とソースだけから作る**（`ingest::content_hash` と同じ作り）——
/// 収集側が採番した id を混ぜると、再送のたびに別物になって重複が入る。
///
/// ST01 の Outbox は部分失敗の後で送り直すので、**重複の到着は常態**（第 4 回 Q13）。
pub fn content_hash(req: &HeartbeatRequest) -> String {
    use sha2::{Digest as _, Sha256};
    let mut h = Sha256::new();
    let mut field = |bytes: &[u8]| {
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    };
    field(req.logical_source.as_bytes());
    field(&req.emitted_at.timestamp_micros().to_be_bytes());
    field(req.raw.as_bytes());
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn req() -> HeartbeatRequest {
        HeartbeatRequest {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::nil(),
            logical_source: "t".into(),
            device_id: Some("d".into()),
            emitted_at: chrono::DateTime::from_timestamp(1_757_000_000, 0).unwrap(),
            capturable: true,
            blockers: vec![],
            attempts: 360,
            successes: 230,
            raw: r#"{"alive":true}"#.into(),
        }
    }

    /// Scenario: 理由の無い「取れない」は受け付けられない
    #[test]
    fn blockerless_uncapturable_is_invalid() {
        let mut r = req();
        r.capturable = false;
        assert_eq!(r.validate(), Err(Invalid::Blockerless));
        // 空白だけの理由も理由ではない
        r.blockers = vec!["  ".into()];
        assert_eq!(r.validate(), Err(Invalid::Blockerless));
        r.blockers = vec!["permission".into()];
        assert_eq!(r.validate(), Ok(()));
    }

    /// Scenario: 成功が試行を超える信号は受け付けられない
    #[test]
    fn successes_over_attempts_is_invalid() {
        let mut r = req();
        r.attempts = 10;
        r.successes = 11;
        assert_eq!(r.validate(), Err(Invalid::Counts));
        r.successes = 10;
        assert_eq!(r.validate(), Ok(()));
        r.attempts = -1;
        assert_eq!(r.validate(), Err(Invalid::Counts));
    }

    /// 原文が空・U+0000 入りなら断る（`ingest` と同じ理由。DB へ届かせない）。
    #[test]
    fn empty_or_nul_raw_is_invalid() {
        let mut r = req();
        r.raw = String::new();
        assert_eq!(r.validate(), Err(Invalid::Raw));
        r.raw = "{\"a\":\"\0\"}".into();
        assert_eq!(r.validate(), Err(Invalid::Raw));
    }

    /// 冪等キーは**内容だけ**から決まる。収集側の id を変えても同じ鍵になる
    /// —— 再送が別物として入らないことの土台（第 4 回 Q13）。
    #[test]
    fn content_hash_ignores_collector_id() {
        let a = req();
        let mut b = req();
        b.id = uuid::Uuid::new_v4();
        b.device_id = Some("other".into());
        assert_eq!(content_hash(&a), content_hash(&b));
        b.raw = r#"{"alive":false}"#.into();
        assert_ne!(content_hash(&a), content_hash(&b));
    }
}
