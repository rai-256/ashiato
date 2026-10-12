// SPDX-License-Identifier: AGPL-3.0-only
//! 取得元にある時差だけを使い、位置から地域を推定しない（ST12 / C2）。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceTimezone {
    pub offset_min: i32,
    pub id: String,
    pub from_source: bool,
}

pub fn from_rfc3339(value: &str) -> anyhow::Result<SourceTimezone> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value)?;
    let offset = parsed.offset().local_minus_utc() / 60;
    let from_source = !value.ends_with('Z');
    Ok(from_offset(offset, from_source))
}

/// Timeline の `startTimeTimezoneUtcOffsetMinutes` があれば、時刻文字列の
/// 表記より優先する。取得元が示した明示的な地域情報だからである。
pub fn from_timestamp(
    value: &str,
    start_time_timezone_utc_offset_minutes: Option<i32>,
) -> anyhow::Result<SourceTimezone> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value)?;
    let offset = start_time_timezone_utc_offset_minutes
        .unwrap_or_else(|| parsed.offset().local_minus_utc() / 60);
    Ok(from_offset(
        offset,
        start_time_timezone_utc_offset_minutes.is_some() || !value.ends_with('Z'),
    ))
}

fn from_offset(offset: i32, from_source: bool) -> SourceTimezone {
    SourceTimezone {
        offset_min: offset,
        // `Etc/GMT±h` は**正時のずれしか表せない**。+05:30（インド）や +09:30（豪州中部）を
        // 入れると分が切り捨てられ、`tz_offset_min` と `tz_id` が食い違ったまま
        // 記録に凍結される（記録は書き換えられないので後から直せない。review I7）。
        id: if offset == 0 {
            "UTC".into()
        } else if offset % 60 == 0 {
            format!(
                "Etc/GMT{}{}",
                if offset > 0 { "-" } else { "+" },
                offset.unsigned_abs() / 60
            )
        } else {
            format!(
                "UTC{}{:02}:{:02}",
                if offset > 0 { "+" } else { "-" },
                offset.unsigned_abs() / 60,
                offset.unsigned_abs() % 60
            )
        },
        from_source,
    }
}
