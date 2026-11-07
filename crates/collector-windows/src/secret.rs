// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 資格情報の利用者単位の包み。
#![cfg_attr(windows, allow(unsafe_code))]

#[cfg(windows)]
use anyhow::Context as _;

#[cfg(windows)]
pub fn protect(value: &[u8]) -> anyhow::Result<Vec<u8>> {
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(value.len()).context("資格情報が長すぎる")?,
        pbData: value.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: `input` は呼び出しの間生きている `value` を指し、DPAPI は入力を読むだけ（書かない）。
    // `output` は DPAPI が LocalAlloc した領域を受ける空の BLOB。窓を出さない（UI_FORBIDDEN。常駐から呼ぶ）
    unsafe {
        CryptProtectData(
            &raw const input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &raw mut output,
        )
    }
    .map_err(|e| anyhow::anyhow!("資格情報を包めない: {e}"))?;
    Ok(take_blob(output))
}

#[cfg(windows)]
pub fn unprotect(value: &[u8]) -> anyhow::Result<Vec<u8>> {
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(value.len()).context("包んだ資格情報が長すぎる")?,
        pbData: value.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: protect と同じ。`input` は `value` を読むだけで、`output` は DPAPI が埋める
    unsafe {
        CryptUnprotectData(
            &raw const input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &raw mut output,
        )
    }
    .map_err(|e| anyhow::anyhow!("資格情報を戻せない: {e}"))?;
    Ok(take_blob(output))
}

/// DPAPI が返した BLOB を写し取り、領域を返す。
#[cfg(windows)]
fn take_blob(output: windows::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB) -> Vec<u8> {
    if output.pbData.is_null() {
        return Vec::new();
    }
    // SAFETY: 成功した DPAPI の呼び出しは `pbData` に `cbData` バイトの LocalAlloc した領域を返す。
    // 写し取ってから 1 度だけ LocalFree し、以後は触らない
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    // SAFETY: 上の領域は DPAPI が LocalAlloc したもので、まだ返していない
    unsafe {
        windows::Win32::Foundation::LocalFree(Some(windows::Win32::Foundation::HLOCAL(
            output.pbData.cast(),
        )));
    }
    bytes
}

#[cfg(not(windows))]
pub fn protect(value: &[u8]) -> anyhow::Result<Vec<u8>> {
    // Linux の単体試験専用。実際の配布物は Windows の DPAPI 実装を使う。
    let mut out = b"ashiato-test-sealed\0".to_vec();
    out.extend(value.iter().map(|b| b ^ 0xa5));
    Ok(out)
}

#[cfg(not(windows))]
pub fn unprotect(value: &[u8]) -> anyhow::Result<Vec<u8>> {
    let payload = value
        .strip_prefix(b"ashiato-test-sealed\0")
        .ok_or_else(|| anyhow::anyhow!("資格情報を戻せない"))?;
    Ok(payload.iter().map(|b| b ^ 0xa5).collect())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn st56_setupcmd_secret_round_trips() {
        let sealed = protect(b"token-for-test-0123456789").unwrap();
        assert_ne!(sealed, b"token-for-test-0123456789");
        assert_eq!(unprotect(&sealed).unwrap(), b"token-for-test-0123456789");
    }

    #[test]
    fn st56_setupcmd_secret_rejects_invalid_blob() {
        assert!(unprotect(b"not-a-secret").is_err());
    }
}
