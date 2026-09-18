// SPDX-License-Identifier: AGPL-3.0-only
//! YouTubeの視聴と検索をURLの経路で分ける。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub logical_source: &'static str,
    pub search_query: Option<String>,
}
pub fn parse(bytes: &[u8]) -> anyhow::Result<Vec<Row>> {
    let values: Vec<serde_json::Value> = serde_json::from_slice(bytes)?;
    Ok(values
        .into_iter()
        .filter_map(|value| {
            let url = value.get("titleUrl")?.as_str()?;
            if url.contains("watch?v=") {
                Some(Row {
                    logical_source: "c03-youtube-watch",
                    search_query: None,
                })
            } else {
                url.split("search_query=").nth(1).map(|encoded| Row {
                    logical_source: "c03-youtube-search",
                    search_query: Some(percent_decode(encoded)),
                })
            }
        })
        .collect())
}
fn percent_decode(input: &str) -> String {
    let mut bytes = Vec::new();
    let raw = input.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%' && i + 2 < raw.len() {
            if let Ok(value) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                bytes.push(value);
                i += 3;
                continue;
            }
        }
        bytes.push(if raw[i] == b'+' { b' ' } else { raw[i] });
        i += 1;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
