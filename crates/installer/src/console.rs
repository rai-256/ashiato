//! 窓（コンソール）へ問いの文を直に書く口（CH14 D4）。
//!
//! 標準出力や標準エラーを向け替えても、問いは窓に出る。読みは rpassword の `read_password()` が
//! `CONIN$` から行うので、書きだけをここに持つ。
//! あわせて、子の命令の出力を窓のコード頁で読む口（D8）と、Windows の文字コードへ符号化する口（D9）を持つ。
//! `unsafe` はこのモジュールだけ。
#![cfg_attr(windows, allow(unsafe_code))]

/// `text` を窓へ書く。Windows では `CONOUT$` へ `WriteConsoleW`（UTF-16）。
/// 窓を開けない・書けないときは標準エラーへ書いて続ける（問いが出ないまま止まるよりよい）。
///
/// # Errors
/// 標準エラーへの書き込みに失敗したとき。
pub fn write_to_console(text: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        if windows_console::write(text) {
            return Ok(());
        }
    }
    write_to_stderr(text)
}

/// Windows の命令（`whoami`・`reg.exe`・`sc.exe` など）の出力を値として読む（D8）。Windows では
/// 窓のコード頁（窓が無ければ OEM）が 65001 でなければ先にそのコード頁で読み、読めないときだけ UTF-8 を見る
/// （cp932 の並びは UTF-8 としても正しいことがある。`pc\凜子` → `pc\꣎q`。review/code.md R1・deep.md Q6）。
/// 読めない並びは U+FFFD にして止まらない。
#[must_use]
#[cfg_attr(not(windows), allow(clippy::disallowed_methods))] // Windows 以外の子は UTF-8 で書く
pub fn decode_child_output(bytes: &[u8]) -> String {
    #[cfg(windows)]
    {
        windows_console::decode_child_output(bytes)
    }
    #[cfg(not(windows))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// 失敗の理由に添える子の出力を読む。UTF-8 として正しければそのまま、そうでなければ窓のコード頁で読む
/// （Rust の子 `ashiato-server migrate` が窓のコード頁に依らず UTF-8 で書くので、UTF-8 を先に見るのはここだけ。D8）。
#[cfg_attr(not(windows), allow(clippy::disallowed_methods))] // Windows 以外の子は UTF-8 で書く
fn decode_tail_output(bytes: &[u8]) -> String {
    #[cfg(windows)]
    {
        windows_console::decode_tail_output(bytes)
    }
    #[cfg(not(windows))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// 子の命令の出力の末尾（空行を除いた末尾 5 行を ` / ` で繋ぐ）。読みは UTF-8 を先に見る（`decode_tail_output`）。
#[must_use]
pub fn output_tail_text(stdout: &[u8], stderr: &[u8]) -> String {
    let text = format!(
        "{}{}",
        decode_tail_output(stdout),
        decode_tail_output(stderr)
    );
    let lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    lines[lines.len().saturating_sub(5)..].join(" / ")
}

/// コード頁 `code_page` で読む。UTF-8 として正しければそのまま（失敗の理由の末尾の読み）。
#[cfg(windows)]
#[must_use]
pub fn decode_with_code_page(bytes: &[u8], code_page: u32) -> String {
    windows_console::decode_with_code_page(bytes, code_page)
}

/// 値の口の読み（`decode_child_output`）の中身。`code_page` が 65001 でなければ先にそのコード頁で読み、
/// 読めないときだけ UTF-8 を見る。
#[cfg(windows)]
#[must_use]
pub fn decode_value_with_code_page(bytes: &[u8], code_page: u32) -> String {
    windows_console::decode_value_with_code_page(bytes, code_page)
}

/// `text` を `code_page` のバイト列にする。表せない文字があれば理由を `Err` で返す。
///
/// # Errors
/// `code_page` で表せない文字を含むとき。
#[cfg(windows)]
pub fn encode_with_code_page(text: &str, code_page: u32) -> Result<Vec<u8>, String> {
    windows_console::encode_with_code_page(text, code_page)
}

/// `text` を Windows の ANSI のコード頁（`GetACP()`）のバイト列にする。Windows 以外は UTF-8。
///
/// # Errors
/// ANSI のコード頁で表せない文字を含むとき（`Windows の文字コード <番号> で表せない`）。
pub fn encode_ansi(text: &str) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    {
        windows_console::encode_with_code_page(text, windows_console::ansi_code_page())
    }
    #[cfg(not(windows))]
    {
        Ok(text.as_bytes().to_vec())
    }
}

/// `repair` の DB の認証の対応表（`repair.pg_ident.conf`）の中身を、Windows の ANSI のコード頁（`GetACP()`）の
/// バイト列で組む（D9）。Windows 以外は UTF-8。
///
/// # Errors
/// 利用者名を ANSI のコード頁で表せないとき（`利用者名 <名前> を Windows の文字コード <番号> で表せない`）。
/// 合わない行は書かずに止まる。
pub fn repair_ident_bytes(account: &str) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    {
        repair_ident_bytes_with_code_page(account, windows_console::ansi_code_page())
    }
    #[cfg(not(windows))]
    {
        encode_ansi(&crate::commands::repair_ident_contents(account))
            .map_err(|reason| format!("利用者名 {account} を {reason}"))
    }
}

/// `repair_ident_bytes` の中身。コード頁を引数で受ける（試験が 1252 などを指す）。
///
/// # Errors
/// 利用者名を `code_page` で表せないとき。
#[cfg(windows)]
pub fn repair_ident_bytes_with_code_page(account: &str, code_page: u32) -> Result<Vec<u8>, String> {
    encode_with_code_page(&crate::commands::repair_ident_contents(account), code_page)
        .map_err(|reason| format!("利用者名 {account} を {reason}"))
}

fn write_to_stderr(text: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut stderr = std::io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.flush()
}

#[cfg(windows)]
mod windows_console {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Globalization::{
        GetACP, GetOEMCP, IsDBCSLeadByteEx, MultiByteToWideChar, WideCharToMultiByte,
        MB_ERR_INVALID_CHARS, WC_NO_BEST_FIT_CHARS,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{GetConsoleOutputCP, WriteConsoleW};

    const CP_UTF8: u32 = 65001;

    pub(super) fn ansi_code_page() -> u32 {
        // SAFETY: 引数なしで、プロセスの ANSI のコード頁の番号を返すだけ。
        unsafe { GetACP() }
    }

    /// 窓の出力のコード頁。窓が無ければ OEM のコード頁。
    fn console_code_page() -> u32 {
        // SAFETY: 引数なしで、窓の出力のコード頁（窓が無ければ 0）を返すだけ。
        let console = unsafe { GetConsoleOutputCP() };
        if console == 0 {
            // SAFETY: 引数なしで、OEM のコード頁の番号を返すだけ。
            unsafe { GetOEMCP() }
        } else {
            console
        }
    }

    pub(super) fn decode_child_output(bytes: &[u8]) -> String {
        decode_value_with_code_page(bytes, console_code_page())
    }

    pub(super) fn decode_tail_output(bytes: &[u8]) -> String {
        decode_with_code_page(bytes, console_code_page())
    }

    pub(super) fn decode_value_with_code_page(bytes: &[u8], code_page: u32) -> String {
        if code_page == CP_UTF8 {
            return decode_with_code_page(bytes, code_page);
        }
        if let Some(wide) = strict_decode(bytes, code_page) {
            return String::from_utf16_lossy(&wide);
        }
        if let Ok(text) = std::str::from_utf8(bytes) {
            return text.to_owned();
        }
        lossy_decode(bytes, code_page)
    }

    /// 読めない並びがあれば `None`（旗 `MB_ERR_INVALID_CHARS`）。
    fn strict_decode(bytes: &[u8], code_page: u32) -> Option<Vec<u16>> {
        let len = i32::try_from(bytes.len()).ok()?;
        if len == 0 {
            return Some(Vec::new());
        }
        // SAFETY: `bytes` は `len` バイトの有効な領域。出力先 null・0 は必要な長さを聞く呼び方。
        let size = unsafe {
            MultiByteToWideChar(
                code_page,
                MB_ERR_INVALID_CHARS,
                bytes.as_ptr(),
                len,
                std::ptr::null_mut(),
                0,
            )
        };
        if size <= 0 {
            return None;
        }
        let mut wide = vec![0u16; usize::try_from(size).ok()?];
        // SAFETY: `wide` は `size` 個の u16 を書ける領域で、`bytes` は上と同じ。
        let written = unsafe {
            MultiByteToWideChar(
                code_page,
                MB_ERR_INVALID_CHARS,
                bytes.as_ptr(),
                len,
                wide.as_mut_ptr(),
                size,
            )
        };
        (written == size).then_some(wide)
    }

    pub(super) fn decode_with_code_page(bytes: &[u8], code_page: u32) -> String {
        if let Ok(text) = std::str::from_utf8(bytes) {
            return text.to_owned();
        }
        if let Some(wide) = strict_decode(bytes, code_page) {
            return String::from_utf16_lossy(&wide);
        }
        lossy_decode(bytes, code_page)
    }

    /// 1 文字ずつ読み、読めない文字だけを U+FFFD にする。
    fn lossy_decode(bytes: &[u8], code_page: u32) -> String {
        // 1 文字ずつ。先頭バイトが 2 バイト文字の先頭なら 2 バイト、そうでなければ 1 バイト
        let mut out = String::new();
        let mut index = 0;
        while index < bytes.len() {
            let first = bytes[index];
            // SAFETY: 値を渡すだけで、ポインタは使わない。
            let lead = unsafe { IsDBCSLeadByteEx(code_page, first) } != 0;
            let width = if lead { 2 } else { 1 };
            let end = (index + width).min(bytes.len());
            let chunk = &bytes[index..end];
            match (chunk.len() == width)
                .then(|| strict_decode(chunk, code_page))
                .flatten()
            {
                Some(wide) => out.push_str(&String::from_utf16_lossy(&wide)),
                None => out.push('\u{FFFD}'),
            }
            index = end;
        }
        out
    }

    pub(super) fn encode_with_code_page(text: &str, code_page: u32) -> Result<Vec<u8>, String> {
        let refused = || format!("Windows の文字コード {code_page} で表せない");
        let wide: Vec<u16> = text.encode_utf16().collect();
        let len = i32::try_from(wide.len()).map_err(|_| refused())?;
        if len == 0 {
            return Ok(Vec::new());
        }
        // UTF-8 などでは `lpUsedDefaultChar` を null にする決まり
        let mut used_default = 0i32;
        let used: *mut i32 = if code_page == CP_UTF8 {
            std::ptr::null_mut()
        } else {
            &raw mut used_default
        };
        let flags = if code_page == CP_UTF8 {
            0
        } else {
            WC_NO_BEST_FIT_CHARS
        };
        // SAFETY: `wide` は `len` 個の有効な u16。出力先 null・0 は必要な長さを聞く呼び方で、
        // `used` は null か書ける i32 を指す。
        let size = unsafe {
            WideCharToMultiByte(
                code_page,
                flags,
                wide.as_ptr(),
                len,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                used,
            )
        };
        if size <= 0 || used_default != 0 {
            return Err(refused());
        }
        let mut out = vec![0u8; usize::try_from(size).map_err(|_| refused())?];
        // SAFETY: `out` は `size` バイトを書ける領域で、ほかは上と同じ。
        let written = unsafe {
            WideCharToMultiByte(
                code_page,
                flags,
                wide.as_ptr(),
                len,
                out.as_mut_ptr(),
                size,
                std::ptr::null(),
                used,
            )
        };
        if written != size || used_default != 0 {
            return Err(refused());
        }
        Ok(out)
    }

    /// 窓へ全部書けたら true。
    pub(super) fn write(text: &str) -> bool {
        let name: Vec<u16> = "CONOUT$".encode_utf16().chain(Some(0)).collect();
        // SAFETY: `name` は NUL で終わる UTF-16 で、呼びの間生きている。ほかの引数は
        // 「既存の窓を開く」ための定数と null（既定の保安・鋳型なし）。
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return false;
        }
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut done = 0usize;
        let mut ok = true;
        while done < wide.len() {
            let rest = &wide[done..];
            let count = u32::try_from(rest.len()).unwrap_or(u32::MAX);
            let mut written = 0u32;
            // SAFETY: `handle` は上で開いた有効な窓の handle。`rest` は `count` 個以上の
            // 有効な u16 を指し、`written` は書き込める u32 を指す。
            let result = unsafe {
                WriteConsoleW(
                    handle,
                    rest.as_ptr().cast(),
                    count,
                    &raw mut written,
                    std::ptr::null(),
                )
            };
            if result == 0 || written == 0 {
                ok = false;
                break;
            }
            done += written as usize;
        }
        // SAFETY: `handle` は上で開いた有効な handle で、ここで一度だけ閉じる。
        unsafe { CloseHandle(handle) };
        ok
    }
}

#[cfg(test)]
mod tests {
    // Windows では窓へ書く口を通るので名前と合わない。Windows の CI の下限（11 本）にも数えない（review/code.md R4・R14）
    #[cfg(not(windows))]
    #[test]
    fn ch14_write_to_console_succeeds_off_windows() {
        assert!(super::write_to_console("").is_ok());
    }

    #[cfg(not(windows))]
    #[test]
    fn ch14_output_tail_text_keeps_last_five_nonblank_lines() {
        let text = super::output_tail_text(b"a\n\nb\nc\n", b"d\ne\nf\n");
        assert_eq!(text, "b / c / d / e / f");
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::{
        decode_value_with_code_page, decode_with_code_page, encode_with_code_page, output_tail_text,
    };
    use crate::commands;
    use windows_sys::Win32::System::Console::{
        AllocConsole, FreeConsole, GetConsoleOutputCP, SetConsoleOutputCP,
    };

    // 期待するバイト列は `python3 -c "print('<文>'.encode('cp932').hex(' '))"` で作った。
    // 指定された
    const CP932_SHITEI: [u8; 10] = [0x8e, 0x77, 0x92, 0xe8, 0x82, 0xb3, 0x82, 0xea, 0x82, 0xbd];

    fn hex(text: &str) -> Vec<u8> {
        text.split(' ')
            .map(|b| u8::from_str_radix(b, 16).expect("16 進"))
            .collect()
    }

    // Scenario: 窓のコード頁で書いた子の命令の失敗の理由が、化けずに読める
    #[test]
    fn ch14_output_tail_reads_console_code_page() {
        // 指定されたサービスはインストールされたサービスとして存在しません。
        let bytes = hex("8e 77 92 e8 82 b3 82 ea 82 bd 83 54 81 5b 83 72 83 58 82 cd 83 43 83 93 83 58 83 67 81 5b 83 8b 82 b3 82 ea 82 bd 83 54 81 5b 83 72 83 58 82 c6 82 b5 82 c4 91 b6 8d dd 82 b5 82 dc 82 b9 82 f1 81 42");
        // 窓の出力のコード頁はプロセスに 1 つ。ほかに窓のコード頁を読む試験は無い（ch14_ の残りはコード頁を指して読む）。
        // SAFETY: 窓が無ければ作り、窓の出力のコード頁を読み書きするだけ（引数は値）。
        let (allocated, before) = unsafe {
            let allocated = AllocConsole() != 0;
            let before = GetConsoleOutputCP();
            SetConsoleOutputCP(932);
            (allocated, before)
        };
        let tail = output_tail_text(&bytes, b"");
        // SAFETY: 元のコード頁へ戻し、この試験が作った窓なら閉じるだけ（review/code.md R11）。
        unsafe {
            if allocated {
                FreeConsole();
            } else if before != 0 {
                SetConsoleOutputCP(before);
            }
        }
        assert!(
            tail.contains("指定されたサービスはインストールされたサービスとして存在しません。"),
            "{tail}"
        );
        assert!(!tail.contains('\u{FFFD}'), "{tail}");
    }

    // Scenario: どのコード頁でも読めない並びは、置換の文字にして止まらない
    #[test]
    fn ch14_unreadable_bytes_become_replacement() {
        let mut bytes = CP932_SHITEI.to_vec();
        bytes.push(0x82);
        let text = decode_with_code_page(&bytes, 932);
        assert!(text.starts_with("指定された"), "{text}");
        assert!(text.ends_with('\u{FFFD}'), "{text}");
    }

    // Scenario: UTF-8 で書く子の出力は、窓のコード頁が 932 でもそのまま読める
    // UTF-8 で書く子は Rust の子（`ashiato-server migrate`）で、その出力は失敗の理由の末尾（`output_tail_text`）として読む
    #[test]
    fn ch14_utf8_child_output_is_kept_on_932() {
        let line = "サービス ashiato-db は実行中です";
        assert_eq!(decode_with_code_page(line.as_bytes(), 932), line);
    }

    // Scenario: 英字だけの子の出力は、いまと同じ値として読む
    #[test]
    #[allow(clippy::disallowed_methods)] // いまの読み（UTF-8）と比べる
    fn ch14_ascii_child_output_is_unchanged() {
        let samples = [
            "pc\\owner\r\n",
            "\r\nHKEY_CURRENT_USER\\Software\\X\r\n    {374DE290}    REG_EXPAND_SZ    %USERPROFILE%\\Downloads\r\n",
            "SERVICE_NAME: ashiato-db\r\n        STATE              : 4  RUNNING\r\n",
        ];
        for sample in samples {
            for code_page in [932, 437, 65001] {
                assert_eq!(
                    decode_with_code_page(sample.as_bytes(), code_page),
                    String::from_utf8_lossy(sample.as_bytes())
                );
                assert_eq!(
                    decode_value_with_code_page(sample.as_bytes(), code_page),
                    String::from_utf8_lossy(sample.as_bytes())
                );
            }
        }
    }

    // Scenario: コード頁 932 で書いた日本語の利用者名を、同じ文字として読む
    #[test]
    fn ch14_account_read_in_code_page_932() {
        // pc\足跡太郎
        let bytes = hex("70 63 5c 91 ab 90 d5 91 be 98 59");
        let account = decode_value_with_code_page(&bytes, 932);
        assert_eq!(account, "pc\\足跡太郎");
        assert!(commands::repair_ident_contents(&account).contains("足跡太郎"));
        // pc\凜子: cp932 の並びが UTF-8 としても正しい（UTF-8 で読むと pc\꣎q。review/code.md R1・deep.md Q6）
        let bytes = hex("70 63 5c ea a3 8e 71");
        assert!(std::str::from_utf8(&bytes).is_ok());
        let account = decode_value_with_code_page(&bytes, 932);
        assert_eq!(account, "pc\\凜子");
        let line = commands::repair_ident_contents(&account);
        assert!(line.contains("凜子"), "{line}");
        assert!(encode_with_code_page(&line, 932).is_ok());
        // 窓が 65001 なら Windows の命令は UTF-8 で書く
        assert_eq!(
            decode_value_with_code_page("pc\\凜子".as_bytes(), 65001),
            "pc\\凜子"
        );
    }

    // Scenario: 英字でない利用者名の対応表の行は、Windows の文字コードのバイト列で、文字どおりに書く
    #[test]
    fn ch14_ident_line_is_literal_in_ansi_bytes() {
        // 'pc\足跡表ソ' の利用者名: 'ashiato' ではなく 足跡表ソ@pc
        let line = commands::repair_ident_contents("pc\\足跡表ソ");
        let bytes = encode_with_code_page(&line, 932).expect("符号化できる");
        // 足跡表ソ@pc
        let mut expected = b"repair \"".to_vec();
        expected.extend(hex("91 ab 90 d5 95 5c 83 5c"));
        expected.extend(b"@pc\" ashiato\n");
        assert_eq!(bytes, expected);
        assert!(bytes.contains(&0x5c));
        assert!(!line.contains("/(?i)"));
        let ascii = commands::repair_ident_contents("pc\\owner");
        assert_eq!(
            encode_with_code_page(&ascii, 932).expect("符号化できる"),
            b"repair /(?i)^owner@pc$ ashiato\n"
        );
    }

    // Scenario: Windows の文字コードで表せない利用者名なら、repair は理由を示して止まる
    #[test]
    fn ch14_unrepresentable_name_is_refused() {
        let error = encode_with_code_page("足跡", 1252).expect_err("表せない");
        assert!(error.contains("1252"), "{error}");
        // repair が対応表を組む口（main.rs はこれを GetACP() で呼び、Err ならサービスを止める前に止まる）
        let refused =
            super::repair_ident_bytes_with_code_page("pc\\足跡", 1252).expect_err("行は組まれない");
        assert_eq!(
            refused,
            "利用者名 pc\\足跡 を Windows の文字コード 1252 で表せない"
        );
    }
}
