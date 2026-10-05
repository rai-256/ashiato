// SPDX-License-Identifier: AGPL-3.0-only
//! Takeout の訳された名前に依存しない、中身の形による分類（ST12 / D3）。

use super::open::ArchiveFile;

// 試験の見張りが種類を手で並べずに全部を辿れるように（code-verify 第 6 回 R97）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(strum::EnumIter))]
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
    // **先頭 1 件だけで決めない**（final review R57）。削除済みの動画は `titleUrl` を持たないので、
    // 先頭がそれだと視聴履歴がファイルごと「読まなかった」になった。URL を持つ最初の項目で決める。
    let items: Vec<&serde_json::Map<String, serde_json::Value>> = value
        .as_array()?
        .iter()
        .filter_map(serde_json::Value::as_object)
        .collect();
    if let Some(url) = items
        .iter()
        .find_map(|item| item.get("titleUrl").and_then(serde_json::Value::as_str))
    {
        if url.contains("watch?v=") {
            return Some(KnownKind::YouTubeWatch);
        }
        if url.contains("results?search_query=") {
            return Some(KnownKind::YouTubeSearch);
        }
    }
    items
        .iter()
        .find(|item| item.contains_key("titleUrl"))
        .or_else(|| items.first())
        .filter(|item| {
            item.contains_key("products")
                && item.contains_key("header")
                && item.contains_key("time")
        })
        .map(|_| KnownKind::MyActivity)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::KnownKind;

    /// 種類ごとに、読み手のすべての枝（論理ソース）を通る材料。**`match` は網羅で、`_` を置かない**
    /// （種類を足すと、ここに材料を書くまでコンパイルで落ちる）。
    fn fixture(kind: KnownKind) -> &'static str {
        match kind {
            KnownKind::YouTubeWatch => {
                r#"[{"header":"YouTube","time":"2026-01-01T00:00:00Z","products":["YouTube"],"titleUrl":"https://www.youtube.com/watch?v=x"}]"#
            }
            KnownKind::YouTubeSearch => {
                r#"[{"header":"YouTube","time":"2026-01-01T00:00:00Z","products":["YouTube"],"titleUrl":"https://www.youtube.com/results?search_query=x"}]"#
            }
            KnownKind::MyActivity => {
                r#"[{"header":"検索","time":"2026-01-01T00:00:00Z","products":["検索"],"title":"x"}]"#
            }
            KnownKind::Timeline => {
                r#"{"semanticSegments":[
                    {"startTime":"2026-01-01T00:00:00Z","endTime":"2026-01-01T01:00:00Z","visit":{}},
                    {"startTime":"2026-01-01T01:00:00Z","endTime":"2026-01-01T02:00:00Z","activity":{}},
                    {"startTime":"2026-01-01T02:00:00Z","endTime":"2026-01-01T03:00:00Z",
                     "timelinePath":[{"point":"35.0°, 139.0°","time":"2026-01-01T02:10:00Z"}]}],
                   "rawSignals":[{"position":{"LatLng":"35.0°, 139.0°","timestamp":"2026-01-01T04:00:00Z"}}]}"#
            }
            KnownKind::Records => r#"{"locations":[{"timestamp":"2026-01-01T00:00:00Z"}]}"#,
            KnownKind::SemanticHistory => {
                r#"{"timelineObjects":[
                    {"placeVisit":{"duration":{"startTimestamp":"2026-01-01T00:00:00Z","endTimestamp":"2026-01-01T01:00:00Z"}}},
                    {"activitySegment":{"duration":{"startTimestamp":"2026-01-01T01:00:00Z","endTimestamp":"2026-01-01T02:00:00Z"}}}]}"#
            }
            KnownKind::ChromeHistory => {
                r#"{"Browser History":[{"time_usec":13222310400000000,"title":"x","url":"https://example.com/"}]}"#
            }
        }
    }

    // 見張り（final review 第 4 回 R93。第 6 回 Q15 / design D22-d）: 書庫が項目を入れる論理ソースは、
    // すべて「位置」（`LOCATION_SOURCES`）か「位置の欄を見る項目」（`ITEM_SOURCES` と `c03-myactivity-*`）の
    // どちらかに分類されている。種類やソースを足して分類へ足し忘れると、消した場面の座標を原文に持ったまま
    // 生きて入る（loss: exported）ので、ここで落とす。
    //
    // 除外は無い: 読み手（`requests_for_file`）が出すのは項目の論理ソースだけで、取り込み器の生存信号
    // （`s01-archive-inbox`）や台帳は要求を作らない。
    #[test]
    fn every_archive_logical_source_is_classified_as_location_or_item() {
        // 種類は derive で全部を辿る（手で並べると、足した種類を並べ忘れても緑のまま。R97）。
        let mut seen = std::collections::BTreeSet::new();
        for kind in <KnownKind as strum::IntoEnumIterator>::iter() {
            let bytes = fixture(kind);
            let value: serde_json::Value = serde_json::from_str(bytes).unwrap();
            assert_eq!(
                super::kind_of(value),
                Some(kind),
                "材料が {kind:?} に分類されない"
            );
            let out = crate::archive::worker::requests_for_file_reporting(
                kind,
                "fixture.json",
                bytes.as_bytes(),
                uuid::Uuid::nil(),
                "x".repeat(64),
            )
            .unwrap();
            assert!(out.unreadable.is_empty(), "{kind:?}: {:?}", out.unreadable);
            assert!(!out.requests.is_empty(), "{kind:?} が要求を作らない");
            for request in out.requests {
                let source = request.logical_source;
                assert!(
                    crate::archive::LOCATION_SOURCES.contains(&source.as_str())
                        || crate::archive::ITEM_SOURCES.contains(&source.as_str())
                        || source.starts_with("c03-myactivity-"),
                    "{kind:?} の論理ソース {source} が位置にも項目にも分類されていない"
                );
                seen.insert(source);
            }
        }
        // 材料が読み手の枝をすべて通ったこと（分類の側の名前がどれも実際に出ること）。
        for source in crate::archive::LOCATION_SOURCES
            .iter()
            .chain(crate::archive::ITEM_SOURCES.iter())
        {
            assert!(seen.contains(*source), "材料が {source} を出さない");
        }
    }
}
