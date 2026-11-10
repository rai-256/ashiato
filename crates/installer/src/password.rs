use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};

const MIN_LEN: usize = 16;
const PLACEHOLDER: &str = "change-me";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordRefusal {
    TooShort,
    Placeholder,
    SameAsApiToken,
    SameAsSessionKey,
    Mismatch,
}

impl PasswordRefusal {
    pub fn reason(self) -> String {
        match self {
            Self::TooShort => format!("短い（{MIN_LEN} 文字以上）"),
            Self::Placeholder => format!("雛形（{PLACEHOLDER} で始めない）"),
            Self::SameAsApiToken => "API の合言葉と同じ".to_string(),
            Self::SameAsSessionKey => "ログインの印の鍵と同じ".to_string(),
            Self::Mismatch => "一致しない".to_string(),
        }
    }
}

/// 最初の問いの前に 1 回出す、合言葉の条件の一覧（1 行）。
/// 端末でないとき（管）は表示の問題が無いので「入れた文字は表示されません」を付けない。
pub fn requirements_line(terminal: bool) -> String {
    let mut line = format!(
        "画面の合言葉を決めます。{MIN_LEN} 文字以上・{PLACEHOLDER} で始めない・\
         自動で作った API の合言葉やログインの印の鍵と同じにしない（ふつうは当たりません）"
    );
    if terminal {
        line.push_str("。入れた文字は表示されません");
    }
    line
}

/// 画面の合言葉を設定する前に、仕様で決めた五つの理由を検査する。
pub fn validate(
    password: &str,
    api_token: &str,
    session_key: &str,
    confirmation: &str,
) -> Result<(), PasswordRefusal> {
    if password.chars().count() < MIN_LEN {
        return Err(PasswordRefusal::TooShort);
    }
    if password.starts_with(PLACEHOLDER) {
        return Err(PasswordRefusal::Placeholder);
    }
    if password == api_token {
        return Err(PasswordRefusal::SameAsApiToken);
    }
    if password == session_key {
        return Err(PasswordRefusal::SameAsSessionKey);
    }
    if password != confirmation {
        return Err(PasswordRefusal::Mismatch);
    }
    Ok(())
}

/// 合言葉を設定に残さない Argon2id PHC 文字列へ変換する。
pub fn verifier(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ch14_requirements_line_names_every_condition() {
        let line = requirements_line(true);
        assert!(line.contains(&MIN_LEN.to_string()));
        assert!(line.contains(PLACEHOLDER));
        assert!(line.contains("API の合言葉"));
        assert!(line.contains("ログインの印の鍵"));
    }

    #[test]
    fn ch14_requirements_line_mentions_hidden_only_on_terminal() {
        assert!(requirements_line(true).contains("表示されません"));
        assert!(!requirements_line(false).contains("表示されません"));
    }

    #[test]
    fn ch14_refusal_reason_keeps_word_and_adds_condition() {
        let short = PasswordRefusal::TooShort.reason();
        assert!(short.starts_with("短い") && short.contains(&MIN_LEN.to_string()));
        let placeholder = PasswordRefusal::Placeholder.reason();
        assert!(placeholder.starts_with("雛形") && placeholder.contains(PLACEHOLDER));
        assert_eq!(
            PasswordRefusal::SameAsApiToken.reason(),
            "API の合言葉と同じ"
        );
        assert_eq!(
            PasswordRefusal::SameAsSessionKey.reason(),
            "ログインの印の鍵と同じ"
        );
        assert_eq!(PasswordRefusal::Mismatch.reason(), "一致しない");
    }
}
