// SPDX-License-Identifier: AGPL-3.0-only
//! YouTubeの視聴と検索をURLの経路で分ける。あわせて、評価と書き出しの言語を決まり文句の表で見分ける（ST47 / D4（仮））。
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
pub fn percent_decode(input: &str) -> String {
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

/// 言語ごとの決まり文句の表（ST47 / D4（仮））。比べ方は日本語が**末尾**、英語が**先頭**
/// （動画名の途中に決まり文句が含まれても当たらないように）。
struct Phrases {
    language: &'static str,
    ends_with: bool,
    rating: &'static [&'static str],
    /// 言語の見分けだけに使う。評価の無いファイルでも言語が分かるように。
    recognition: &'static [&'static str],
}

/// 上から順に当たりを見る（`languages` が題名を最初に当たった言語だけに数える。順は日本語が先）。
const PHRASES: [Phrases; 2] = [
    Phrases {
        language: "ja",
        ends_with: true,
        rating: &["を高く評価しました", "を低く評価しました"],
        recognition: &["を視聴しました", "を検索しました"],
    },
    Phrases {
        language: "en",
        ends_with: false,
        rating: &["Liked ", "Disliked "],
        recognition: &["Watched ", "Searched for "],
    },
];

impl Phrases {
    fn matches(&self, title: &str, phrases: &[&str]) -> bool {
        phrases.iter().any(|phrase| {
            if self.ends_with {
                title.ends_with(phrase)
            } else {
                title.starts_with(phrase)
            }
        })
    }
}

/// ファイルの題名から「当たった言語」（並べて重複を除いたもの）を返す。
/// 題名は表の上から順に最初に当たった言語にだけ数える
/// （日本語の書き出しの「Liked by the Sea を視聴しました」を英語の当たりに数えないため。spec R8）。
pub fn languages<'a>(titles: impl IntoIterator<Item = &'a str>) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = Vec::new();
    for title in titles {
        if let Some(phrases) = PHRASES.iter().find(|phrases| {
            phrases.matches(title, phrases.rating) || phrases.matches(title, phrases.recognition)
        }) {
            found.push(phrases.language);
        }
    }
    found.sort_unstable();
    found.dedup();
    found
}

/// 題名が評価か。**当たった言語の表だけ**で見る（言語をまたがない）。
pub fn is_rating(title: &str, languages: &[&str]) -> bool {
    PHRASES
        .iter()
        .filter(|phrases| languages.contains(&phrases.language))
        .any(|phrases| phrases.matches(title, phrases.rating))
}
