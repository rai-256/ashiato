// SPDX-License-Identifier: AGPL-3.0-only
//! 衛星の識別子と名前の形（ST29 / design D2 / D5 / D15）。DB に触らない純粋関数だけを置く。
//! **形の検査はここ 1 か所**。移行の CHECK と画面の文言はここの定数に合わせ、テストで突き合わせる
//! （`satellite_form_tests`）。

/// 識別子の形（移行の CHECK と同じ正規表現。正本はこの定数）。
pub const IDENTIFIER_PATTERN: &str = r"^[a-z0-9-]+(\.[a-z0-9-]+)+$";
/// 識別子の長さの上限（D15）。
pub const IDENTIFIER_MAX_LEN: usize = 64;
/// 本体が使う識別子の前置き（D5）。
pub const BUILTIN_PREFIX: &str = "ashiato.";
/// 名前の持ち主を分ける区切り。`<識別子>:<名前の部分>`。
pub const NAME_SEPARATOR: char = ':';
/// 名前の部分の長さ（D15）。形は `^[a-z0-9-]{1,64}$`。
pub const NAME_PART_MAX_LEN: usize = 64;
/// 衛星の名前の長さ（D15。前後の空白を除いた後の文字数）。
pub const SATELLITE_NAME_MAX_LEN: usize = 40;
/// 資格情報の名前の長さの上限（D15）。
pub const CREDENTIAL_LABEL_MAX_LEN: usize = 40;

/// 識別子を断る理由の種別。応答の本文はこの固定の語だけにする（D5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierRefusal {
    /// 形に合わない、または長すぎる。
    Malformed,
    /// 本体が使う `ashiato.` で始まる。
    Reserved,
    /// 本体の前置き（`^[a-z][0-9]{2}-`）で始まる（D2）。
    BodyPrefix,
}

impl IdentifierRefusal {
    /// 応答の種別の固定の語。
    pub fn kind(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::Reserved => "reserved",
            Self::BodyPrefix => "body_prefix",
        }
    }
}

fn is_label_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
}

/// `^[a-z0-9-]+(\.[a-z0-9-]+)+$`：`.` で分けた 2 つ以上の部分が、どれも 1 文字以上の `[a-z0-9-]`。
fn matches_identifier_shape(s: &str) -> bool {
    let mut parts = 0;
    for part in s.split('.') {
        if part.is_empty() || !part.chars().all(is_label_char) {
            return false;
        }
        parts += 1;
    }
    parts >= 2
}

/// `^[a-z][0-9]{2}-` で始まるか。
fn has_body_prefix(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 4
        && b[0].is_ascii_lowercase()
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3] == b'-'
}

/// 識別子の検査。形 → 本体が使う → 本体の前置き の順に見る。正規化（大文字を小文字にするなど）はしない。
pub fn check_identifier(s: &str) -> Result<(), IdentifierRefusal> {
    if s.len() > IDENTIFIER_MAX_LEN || !matches_identifier_shape(s) {
        return Err(IdentifierRefusal::Malformed);
    }
    if s.starts_with(BUILTIN_PREFIX) {
        return Err(IdentifierRefusal::Reserved);
    }
    if has_body_prefix(s) {
        return Err(IdentifierRefusal::BodyPrefix);
    }
    Ok(())
}

/// 名前の部分の形：`^[a-z0-9-]{1,64}$`。
fn is_name_part(s: &str) -> bool {
    (1..=NAME_PART_MAX_LEN).contains(&s.len()) && s.chars().all(is_label_char)
}

/// `<識別子>:<名前の部分>` の持ち主（識別子）。両方が形に合うときだけ `Some`。
/// 識別子は `check_identifier` を通るものに限る。
pub fn name_owner(name: &str) -> Option<&str> {
    let (identifier, part) = name.split_once(NAME_SEPARATOR)?;
    (check_identifier(identifier).is_ok() && is_name_part(part)).then_some(identifier)
}

/// 衛星の名前を前後の空白を除いて検査する。通れば保存する名前（除いた後）を返す。
pub fn check_satellite_name(raw: &str) -> Option<&str> {
    let name = raw.trim();
    (1..=SATELLITE_NAME_MAX_LEN)
        .contains(&name.chars().count())
        .then_some(name)
}

/// 資格情報の名前の検査（空は可、40 文字まで）。
pub fn check_credential_label(label: &str) -> bool {
    label.chars().count() <= CREDENTIAL_LABEL_MAX_LEN
}

#[cfg(test)]
mod tests {
    use super::*;
    use IdentifierRefusal::*;

    #[test]
    // Scenario: 形に合わない識別子の衛星は登録されない
    fn malformed_identifiers_are_refused() {
        let long = format!("dev.{}", "a".repeat(61));
        assert_eq!(long.len(), 65);
        for id in [
            "booklog",
            "Dev.example.booklog",
            "dev..booklog",
            "dev.example.",
            "dev_example.booklog",
            "dev.example:booklog",
            long.as_str(),
        ] {
            assert_eq!(check_identifier(id), Err(Malformed), "{id}");
        }
    }

    #[test]
    // Scenario: 本体が使う識別子の衛星は登録されない
    fn builtin_identifiers_are_reserved() {
        for id in ["ashiato.booklog", "ashiato.ai-external"] {
            assert_eq!(check_identifier(id), Err(Reserved), "{id}");
        }
    }

    #[test]
    // Scenario: 本体の前置きの形で始まる識別子の衛星は登録されない
    fn body_prefixed_identifiers_are_refused() {
        for id in ["c03-takeout.example", "s01-x.example"] {
            assert_eq!(check_identifier(id), Err(BodyPrefix), "{id}");
        }
    }

    #[test]
    fn valid_identifiers_pass_at_the_length_edge() {
        assert_eq!(check_identifier("dev.example.booklog"), Ok(()));
        let edge = format!("dev.{}", "a".repeat(60));
        assert_eq!(edge.len(), 64);
        assert_eq!(check_identifier(&edge), Ok(()));
        // 前置きの形に近いが合わないものは通す（`^[a-z][0-9]{2}-`）
        for id in [
            "c3-x.example",
            "cc3-x.example",
            "c033-x.example",
            "dev.c03-x",
        ] {
            assert_eq!(check_identifier(id), Ok(()), "{id}");
        }
        assert_eq!(Malformed.kind(), "malformed");
        assert_eq!(Reserved.kind(), "reserved");
        assert_eq!(BodyPrefix.kind(), "body_prefix");
    }

    #[test]
    // Scenario: 名前の持ち主は区切りより前の識別子で決まる
    fn name_owner_is_the_identifier_before_the_separator() {
        let long = format!("dev.example.booklog:{}", "a".repeat(65));
        let edge = format!("dev.example.booklog:{}", "a".repeat(64));
        let cases: [(&str, Option<&str>); 8] = [
            ("dev.example.booklog:reading", Some("dev.example.booklog")),
            ("dev.example.booklog:", None),
            (long.as_str(), None),
            (edge.as_str(), Some("dev.example.booklog")),
            ("dev.example.booklog:Reading", None),
            ("c03-takeout", None),
            ("x:reading", None),
            ("ashiato.ai-local:reading", None),
        ];
        for (name, owner) in cases {
            assert_eq!(name_owner(name), owner, "{name}");
        }
    }

    #[test]
    fn name_lengths_are_checked_after_trimming() {
        assert_eq!(check_satellite_name("  読書  "), Some("読書"));
        assert_eq!(check_satellite_name("   "), None);
        assert_eq!(
            check_satellite_name(&"あ".repeat(40)),
            Some("あ".repeat(40).as_str())
        );
        assert_eq!(check_satellite_name(&"あ".repeat(41)), None);
        assert!(check_credential_label(""));
        assert!(check_credential_label(&"a".repeat(40)));
        assert!(!check_credential_label(&"a".repeat(41)));
    }
}
