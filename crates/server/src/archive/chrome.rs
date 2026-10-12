// SPDX-License-Identifier: AGPL-3.0-only
//! Takeout の Chrome の履歴の `time_usec`（Unix epoch のマイクロ秒）を UTC へ変換する。
//! 端末の `History`（SQLite）の 1601 年起点とは違う（確認バッチ 20261006-0754 の本物の書庫で確かめた）。
pub fn time_usec_to_utc(value: i64) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::from_timestamp_micros(value)
        .ok_or_else(|| anyhow::anyhow!("Chrome時刻が範囲外"))
}
