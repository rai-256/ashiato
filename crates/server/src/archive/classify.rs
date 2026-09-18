// SPDX-License-Identifier: AGPL-3.0-only
//! Takeout の訳された名前に依存しない、中身の形による分類（ST12 / D3）。

use super::open::ArchiveFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownKind {
    YouTubeWatch,
    YouTubeSearch,
    MyActivity,
    Timeline,
    Records,
    SemanticHistory,
    ChromeHistory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownFile {
    pub index: usize,
    pub kind: KnownKind,
}

#[derive(Debug, Default)]
pub struct Classification {
    pub known: Vec<KnownFile>,
    pub skipped: usize,
    pub unreadable: usize,
}

/// 形を優先し、HTML は同じ書庫にJSONがあれば読まなかった側、無ければ読めなかった側にする。
pub fn classify_files(files: &[ArchiveFile]) -> Classification {
    let mut result = Classification::default();
    let mut html = 0;
    for (index, file) in files.iter().enumerate() {
        if file.path.to_ascii_lowercase().ends_with(".html") {
            html += 1;
            continue;
        }
        match serde_json::from_slice::<serde_json::Value>(&file.bytes)
            .ok()
            .and_then(kind_of)
        {
            Some(kind) => result.known.push(KnownFile { index, kind }),
            None => result.skipped += 1,
        }
    }
    if result.known.is_empty() {
        result.unreadable = html;
    } else {
        result.skipped += html;
    }
    result
}

fn kind_of(value: serde_json::Value) -> Option<KnownKind> {
    if let Some(object) = value.as_object() {
        if object.contains_key("semanticSegments") {
            return Some(KnownKind::Timeline);
        }
        if object
            .get("locations")
            .is_some_and(serde_json::Value::is_array)
        {
            return Some(KnownKind::Records);
        }
        if object
            .get("timelineObjects")
            .is_some_and(serde_json::Value::is_array)
        {
            return Some(KnownKind::SemanticHistory);
        }
        if object
            .get("Browser History")
            .is_some_and(serde_json::Value::is_array)
        {
            return Some(KnownKind::ChromeHistory);
        }
    }
    let item = value.as_array()?.first()?.as_object()?;
    let url = item.get("titleUrl")?.as_str()?;
    if url.contains("watch?v=") {
        Some(KnownKind::YouTubeWatch)
    } else if url.contains("results?search_query=") {
        Some(KnownKind::YouTubeSearch)
    } else if item.contains_key("products")
        && item.contains_key("header")
        && item.contains_key("time")
    {
        Some(KnownKind::MyActivity)
    } else {
        None
    }
}
