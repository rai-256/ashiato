//! 窓（コンソール）へ問いの文を直に書く口（CH14 D4）。
//!
//! 標準出力や標準エラーを向け替えても、問いは窓に出る。読みは rpassword の `read_password()` が
//! `CONIN$` から行うので、書きだけをここに持つ。`unsafe` はこのモジュールだけ。
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
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::WriteConsoleW;

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
    #[test]
    fn ch14_write_to_console_succeeds_off_windows() {
        assert!(super::write_to_console("").is_ok());
    }
}
