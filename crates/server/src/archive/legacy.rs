// SPDX-License-Identifier: AGPL-3.0-only
//! 移行前のロケーション履歴を、書庫の形ごとに 3 本の論理ソースへ分ける。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub logical_source: &'static str,
    pub event_time: chrono::DateTime<chrono::Utc>,
}

pub fn parse_records(bytes: &[u8]) -> anyhow::Result<Vec<Record>> {
    let root: serde_json::Value = serde_json::from_slice(bytes)?;
    Ok(root
        .get("locations")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            timestamp(row).map(|event_time| Record {
                logical_source: "c03-legacy-location",
                event_time,
            })
        })
        .collect::<Vec<_>>())
}

pub fn parse_semantic(bytes: &[u8]) -> anyhow::Result<Vec<Record>> {
    let root: serde_json::Value = serde_json::from_slice(bytes)?;
    let mut out = Vec::new();
    for row in root
        .get("timelineObjects")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        for (field, source) in [
            ("placeVisit", "c03-legacy-visit"),
            ("activitySegment", "c03-legacy-activity"),
        ] {
            if let Some(value) = row.get(field).and_then(timestamp) {
                out.push(Record {
                    logical_source: source,
                    event_time: value,
                });
            }
        }
    }
    Ok(out)
}

fn timestamp(value: &serde_json::Value) -> Option<chrono::DateTime<chrono::Utc>> {
    let value = value.get("duration").unwrap_or(value);
    value
        .get("timestamp")
        .or_else(|| value.get("startTimestamp"))
        .and_then(serde_json::Value::as_str)
        .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
        .map(|time| time.to_utc())
        .or_else(|| {
            value
                .get("timestampMs")
                .or_else(|| value.get("startTimestampMs"))
                .and_then(serde_json::Value::as_str)
                .and_then(|text| text.parse::<i64>().ok())
                .and_then(chrono::DateTime::from_timestamp_millis)
        })
}
