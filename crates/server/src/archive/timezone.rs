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
    Ok(SourceTimezone {
        offset_min: offset,
        id: if offset == 0 {
            "UTC".into()
        } else {
            format!(
                "Etc/GMT{}{}",
                if offset > 0 { "-" } else { "+" },
                offset.unsigned_abs() / 60
            )
        },
        from_source,
    })
}
