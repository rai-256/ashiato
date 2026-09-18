// SPDX-License-Identifier: AGPL-3.0-only
//! ChromeのWindows epochマイクロ秒をUTCへ変換する。
pub fn time_usec_to_utc(value: i64) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    let unix_usec = value
        .checked_sub(11_644_473_600_000_000)
        .ok_or_else(|| anyhow::anyhow!("Chrome時刻が範囲外"))?;
    chrono::DateTime::from_timestamp_micros(unix_usec)
        .ok_or_else(|| anyhow::anyhow!("Chrome時刻が範囲外"))
}
