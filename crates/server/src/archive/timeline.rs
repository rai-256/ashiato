// SPDX-License-Identifier: AGPL-3.0-only
//! 端末が書き出した Timeline.json を記録種別ごとに分ける。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRecord {
    pub logical_source: &'static str,
}

pub fn parse(bytes: &[u8]) -> anyhow::Result<Vec<TimelineRecord>> {
    let root: serde_json::Value = serde_json::from_slice(bytes)?;
    let mut records = Vec::new();
    for segment in root
        .get("semanticSegments")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        if segment.get("visit").is_some() {
            records.push(TimelineRecord {
                logical_source: "c03-timeline-visit",
            });
        }
        if segment.get("activity").is_some() {
            records.push(TimelineRecord {
                logical_source: "c03-timeline-move",
            });
        }
        for _ in segment
            .get("timelinePath")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            records.push(TimelineRecord {
                logical_source: "c03-timeline-route",
            });
        }
    }
    for _ in root
        .get("rawSignals")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        records.push(TimelineRecord {
            logical_source: "c03-timeline-signal",
        });
    }
    Ok(records)
}
