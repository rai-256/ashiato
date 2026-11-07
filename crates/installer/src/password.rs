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
    pub fn reason(self) -> &'static str {
        match self {
            Self::TooShort => "短い",
            Self::Placeholder => "雛形",
            Self::SameAsApiToken => "API の合言葉と同じ",
            Self::SameAsSessionKey => "ログインの印の鍵と同じ",
            Self::Mismatch => "一致しない",
        }
    }
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
