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

/// 決まり文句を題名のどちら側で比べるか。
#[derive(Clone, Copy)]
enum At {
    Start,
    End,
}

/// 決まり文句の組。比べる向きは組ごとに持つ
/// （日本語は評価が**先頭**・視聴と検索が**末尾**、英語はどれも**先頭**。deep.md 第 2 回 Q9 の前提の訂正）。
/// 頭か尾に固定で付く形だけを見るのは、動画名の途中に決まり文句が含まれても当たらないように。
struct Group {
    at: At,
    phrases: &'static [&'static str],
}

impl Group {
    fn matches(&self, title: &str) -> bool {
        self.phrases.iter().any(|phrase| match self.at {
            At::Start => title.starts_with(phrase),
            At::End => title.ends_with(phrase),
        })
    }
}

/// 言語ごとの決まり文句の表（ST47 / D4（仮））。
struct Phrases {
    language: &'static str,
    rating: Group,
    /// 言語の見分けだけに使う。評価の無いファイルでも言語が分かるように。
    recognition: Group,
}

/// 上から順に当たりを見る（`languages` が題名を最初に当たった言語だけに数える。順は日本語が先）。
const PHRASES: [Phrases; 2] = [
    Phrases {
        language: "ja",
        rating: Group {
            at: At::Start,
            phrases: &["高く評価しました ", "低く評価しました "],
        },
        recognition: Group {
            at: At::End,
            phrases: &["を視聴しました", "を検索しました"],
        },
    },
    Phrases {
        language: "en",
        rating: Group {
            at: At::Start,
            phrases: &["Liked ", "Disliked "],
        },
        recognition: Group {
            at: At::Start,
            phrases: &["Watched ", "Searched for "],
        },
    },
];

/// ファイルの題名から「当たった言語」（並べて重複を除いたもの）を返す。
/// 題名は表の上から順に最初に当たった言語にだけ数える
/// （日本語の書き出しの「Liked by the Sea を視聴しました」を英語の当たりに数えないため。spec R8）。
pub fn languages<'a>(titles: impl IntoIterator<Item = &'a str>) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = Vec::new();
    for title in titles {
        if let Some(phrases) = PHRASES
            .iter()
            .find(|phrases| phrases.rating.matches(title) || phrases.recognition.matches(title))
        {
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
        .any(|phrases| phrases.rating.matches(title))
}
