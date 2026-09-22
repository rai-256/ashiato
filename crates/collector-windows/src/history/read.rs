// SPDX-License-Identifier: AGPL-3.0-only
//! SQLite 履歴 DB の写しを読み取る。

/// Chromium の Windows epoch（1601-01-01）からのマイクロ秒を UTC へ換算する。
pub fn chromium_micros(value: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    if value < 0 { return None; }
    let epoch = chrono::DateTime::parse_from_rfc3339("1601-01-01T00:00:00Z").ok()?.with_timezone(&chrono::Utc);
    epoch.checked_add_signed(chrono::Duration::microseconds(value))
}

/// Firefox の Unix epoch からのマイクロ秒を UTC へ換算する。
pub fn firefox_micros(value: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    if value < 0 { return None; }
    chrono::DateTime::<chrono::Utc>::UNIX_EPOCH.checked_add_signed(chrono::Duration::microseconds(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_epoch_conversion() {
        assert_eq!(chromium_micros(0), Some(chrono::DateTime::parse_from_rfc3339("1601-01-01T00:00:00Z").expect("epoch").with_timezone(&chrono::Utc)));
        assert_eq!(firefox_micros(0), Some(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH));
        assert_eq!(chromium_micros(-1), None);
        assert_eq!(firefox_micros(-1), None);
    }
}
