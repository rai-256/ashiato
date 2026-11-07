// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! C-02 の設定文字列。共有契約の文字列を読む。

use serde::Deserialize;

const PREFIX: &str = "ashiato-collector:1:";

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SetupString {
    pub url: String,
    pub token: String,
}

/// **資格情報を出さない**（Global Constraints。review/code.md R12）。`{:?}` 1 つでログに落ちる。
impl std::fmt::Debug for SetupString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetupString")
            .field("url", &self.url)
            .field("token", &"***")
            .finish()
    }
}

pub fn parse_setup_string(input: &str) -> Option<SetupString> {
    let encoded = input.trim().strip_prefix(PREFIX)?;
    if encoded.is_empty()
        || !encoded
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    let mut bytes = Vec::with_capacity(encoded.len() * 3 / 4 + 3);
    let mut value = 0u32;
    let mut bits = 0u8;
    for byte in encoded.bytes() {
        value = (value << 6)
            | u32::from(match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'-' => 62,
                b'_' => 63,
                _ => return None,
            });
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((value >> bits) as u8);
        }
    }
    if bits >= 6 || (bits > 0 && value & ((1 << bits) - 1) != 0) {
        return None;
    }
    let value: SetupString = serde_json::from_slice(&bytes).ok()?;
    if value.url.is_empty() || value.token.is_empty() {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use serde_json::Value;

    #[test]
    fn st56_setup_string_debug_hides_the_token() {
        let parsed = SetupString {
            url: "https://pc.example.ts.net/api".into(),
            token: "token-for-test-0123456789".into(),
        };
        let shown = format!("{parsed:?}");
        assert!(shown.contains("https://pc.example.ts.net/api"));
        assert!(!shown.contains("token-for-test-0123456789"));
    }

    fn vectors() -> Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/contracts/collector-setup-vectors.json"
        );
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn st56_setup_string_valid_vectors() {
        for vector in vectors()["valid"].as_array().unwrap() {
            let expected: SetupString = serde_json::from_value(vector["output"].clone()).unwrap();
            assert_eq!(
                parse_setup_string(vector["input"].as_str().unwrap()),
                Some(expected)
            );
        }
    }

    #[test]
    fn st56_setup_string_invalid_vectors() {
        for vector in vectors()["invalid"].as_array().unwrap() {
            assert_eq!(parse_setup_string(vector["input"].as_str().unwrap()), None);
        }
    }
}
