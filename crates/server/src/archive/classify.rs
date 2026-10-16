// SPDX-License-Identifier: AGPL-3.0-only
//! Takeout の訳された名前に依存しない、中身の形による分類（ST12 / D3）。

use super::open::ArchiveFile;

// 試験の見張りが種類を手で並べずに全部を辿れるように（code-verify 第 6 回 R97）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(strum::EnumIter))]
pub enum KnownKind {
    YouTube,
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
    // 配列のファイルは製品の名前で見分ける（ST47 / D1。ファイル名・フォルダ名は訳されるので見ない）。
    let items: Vec<&serde_json::Map<String, serde_json::Value>> = value
        .as_array()?
        .iter()
        .filter_map(serde_json::Value::as_object)
        .collect();
    let mut has_products = false;
    let mut first_products = std::collections::BTreeSet::new();
    for item in &items {
        if let Some(products) = item.get("products") {
            has_products = true;
            if let Some(name) = products
                .as_array()
                .and_then(|products| products.first())
                .and_then(serde_json::Value::as_str)
            {
                first_products.insert(name);
            }
        }
    }
    if has_products {
        // 最初の製品の名前の集合がちょうど「YouTube」。混ざったファイルは `MyActivity`（D2）。
        if first_products.len() == 1 && first_products.contains("YouTube") {
            return Some(KnownKind::YouTube);
        }
        return items
            .iter()
            .any(|item| {
                item.contains_key("products")
                    && item.contains_key("header")
                    && item.contains_key("time")
            })
            .then_some(KnownKind::MyActivity);
    }
    // 製品の名前を持つ項目が無い製品側の履歴の古い形（D2）。
    items
        .iter()
        .filter_map(|item| item.get("titleUrl").and_then(serde_json::Value::as_str))
        .any(|url| url.contains("watch?v=") || url.contains("results?search_query="))
        .then_some(KnownKind::YouTube)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::KnownKind;

    /// 種類ごとに、読み手のすべての枝（論理ソース）を通る材料。**`match` は網羅で、`_` を置かない**
    /// （種類を足すと、ここに材料を書くまでコンパイルで落ちる）。
    fn fixture(kind: KnownKind) -> &'static str {
        match kind {
            // 3 つの行き先（視聴・検索・その他）をすべて通す（ST47 / D11）。
            KnownKind::YouTube => {
                r#"[{"header":"YouTube","title":"x を視聴しました","time":"2026-01-01T00:00:00Z","products":["YouTube"],"titleUrl":"https://www.youtube.com/watch?v=x"},
                    {"header":"YouTube","title":"x を検索しました","time":"2026-01-01T00:00:00Z","products":["YouTube"],"titleUrl":"https://www.youtube.com/results?search_query=x"},
                    {"header":"YouTube","title":"x","time":"2026-01-01T00:00:00Z","products":["YouTube"]}]"#
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
                r#"{"Browser History":[{"time_usec":1577836800000000,"title":"x","url":"https://example.com/"}]}"#
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
