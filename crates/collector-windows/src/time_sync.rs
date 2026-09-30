// SPDX-License-Identifier: AGPL-3.0-only
//! Windows の時刻同期の状態を読む口（ST05 design D8）。
//!
//! 本番は `w32tm /query /status /verbose` を子プロセスで走らせる。**渡す引数は照会だけ**
//! （`/resync` や `/config` は渡さない。外部への通信も起きない）。出力は OS の表示言語の
//! コンソールの符号ページ（日本語は cp932）のバイト列なので、**バイト列のまま原文として持ち**、
//! 見出しの照合もバイト列で行う。
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// 子プロセスを待つ上限（design D8）。
pub const TIME_SYNC_TIMEOUT: Duration = Duration::from_secs(5);

const PROGRAM: &str = "w32tm";
const ARGS: [&str; 3] = ["/query", "/status", "/verbose"];

/// 読んだ時刻同期の状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSyncReading {
    /// OS から読んだ出力。**手を加えない**。
    pub raw: Vec<u8>,
    /// 最後に正常に同期した時刻（表示のままの文字列。同期していなければ `None`）。
    pub last_sync: Option<String>,
    /// 同期元。
    pub source: Option<String>,
    /// OS の見積もったずれ（位相のずれ。ミリ秒）。
    pub os_offset_ms: Option<i64>,
    /// 項目を 1 つも読み取れなかったとき `Some("unparsed")`。
    pub reason: Option<String>,
}

/// 読めなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeSyncError {
    SpawnFailed,
    Exit(Option<i32>),
    Timeout,
}

impl TimeSyncError {
    /// 記録に載せる理由（`spawn_failed` / `exit:<code>` / `timeout`）。
    pub fn reason(&self) -> String {
        match self {
            Self::SpawnFailed => "spawn_failed".into(),
            Self::Exit(Some(code)) => format!("exit:{code}"),
            Self::Exit(None) => "exit:none".into(),
            Self::Timeout => "timeout".into(),
        }
    }
}

/// Windows の時刻同期の状態をくれるもの。**試験では偽物に差し替える。**
pub trait TimeSyncSource: std::fmt::Debug + Send + Sync {
    fn read(&self) -> Result<TimeSyncReading, TimeSyncError>;
}

/// 本番: `w32tm` の子プロセス。
#[derive(Debug, Default)]
pub struct ProcessTimeSync;

impl ProcessTimeSync {
    pub fn new() -> Self {
        Self
    }

    /// 走らせるコマンド。**引数は照会だけに固定**（試験で固定する）。
    pub fn command(&self) -> Command {
        let mut c = Command::new(PROGRAM);
        c.args(ARGS);
        c
    }
}

impl TimeSyncSource for ProcessTimeSync {
    fn read(&self) -> Result<TimeSyncReading, TimeSyncError> {
        run_with_timeout(self.command(), TIME_SYNC_TIMEOUT).map(parse_status)
    }
}

/// 子プロセスを走らせて標準出力を返す。`timeout` で打ち切って殺す。
fn run_with_timeout(mut cmd: Command, timeout: Duration) -> Result<Vec<u8>, TimeSyncError> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| TimeSyncError::SpawnFailed)?;
    // 出力が管に詰まって子が止まらないよう、別のスレッドで読む
    let mut stdout = child.stdout.take().ok_or(TimeSyncError::SpawnFailed)?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(TimeSyncError::Timeout);
            }
            Err(_) => return Err(TimeSyncError::Exit(None)),
        }
    };
    let out = reader.join().unwrap_or_default();
    if status.success() {
        Ok(out)
    } else {
        Err(TimeSyncError::Exit(status.code()))
    }
}

// 見出し（英語 / 日本語 cp932）。日本語の出力は cp932 のバイト列のまま照合する。
const LAST_SYNC_KEYS: [&[u8]; 2] = [
    b"Last Successful Sync Time",
    b"\x8d\xc5\x8f\x49\x90\xb3\x8f\xed\x93\xaf\x8a\xfa\x8e\x9e\x8d\x8f", // 最終正常同期時刻
];
const SOURCE_KEYS: [&[u8]; 2] = [
    b"Source",
    b"\x83\x5c\x81\x5b\x83\x58", // ソース
];
const PHASE_OFFSET_KEYS: [&[u8]; 2] = [
    b"Phase Offset",
    b"\x83\x74\x83\x46\x81\x5b\x83\x59\x20\x83\x49\x83\x74\x83\x5a\x83\x62\x83\x67", // フェーズ オフセット
];
const UNSPECIFIED: [&[u8]; 2] = [
    b"unspecified",
    b"\x96\xa2\x8e\x77\x92\xe8", // 未指定
];

/// 見出しが `keys` のどれかの行の値。値が空・「未指定」なら `None`。
fn value_of(out: &[u8], keys: &[&[u8]]) -> Option<String> {
    out.split(|&b| b == b'\n').find_map(|line| {
        let colon = line.iter().position(|&b| b == b':')?;
        let (key, value) = (line[..colon].trim_ascii(), line[colon + 1..].trim_ascii());
        let unspecified =
            value.is_empty() || UNSPECIFIED.iter().any(|u| value.eq_ignore_ascii_case(u));
        (keys.contains(&key) && !unspecified).then(|| String::from_utf8_lossy(value).into_owned())
    })
}

/// `-0.0012345s` のような位相のずれをミリ秒にする。
fn parse_offset_ms(value: &str) -> Option<i64> {
    let seconds: f64 = value.trim().strip_suffix('s')?.trim().parse().ok()?;
    seconds
        .is_finite()
        .then(|| (seconds * 1000.0).round() as i64)
}

/// 出力を解析する。**原文は必ず持つ。** 1 項目も読めなければ `reason: unparsed`。
pub fn parse_status(raw: Vec<u8>) -> TimeSyncReading {
    let last_sync = value_of(&raw, &LAST_SYNC_KEYS);
    let source = value_of(&raw, &SOURCE_KEYS);
    let os_offset_ms = value_of(&raw, &PHASE_OFFSET_KEYS).and_then(|v| parse_offset_ms(&v));
    let reason = (last_sync.is_none() && source.is_none() && os_offset_ms.is_none())
        .then(|| "unparsed".to_string());
    TimeSyncReading {
        raw,
        last_sync,
        source,
        os_offset_ms,
        reason,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// Windows の日本語表示の出力（cp932 のバイト列）。同期していない機械の形（design D8 の確かめ）。
    fn japanese_output() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(
            b"\x89\x64\x82\xc9\x83\x43\x83\x93\x83\x68\x83\x4c\x83\x5e: 0(\x8e\xc0\x8d\xdb)\r\n",
        );
        out.extend_from_slice(b"\x8d\xc5\x8f\x49\x90\xb3\x8f\xed\x93\xaf\x8a\xfa\x8e\x9e\x8d\x8f: 2026/09/29 12:34:56\r\n");
        out.extend_from_slice(b"\x83\x5c\x81\x5b\x83\x58: time.windows.com,0x8\r\n");
        out.extend_from_slice(b"\x83\x74\x83\x46\x81\x5b\x83\x59 \x83\x49\x83\x74\x83\x5a\x83\x62\x83\x67: -0.0012345s\r\n");
        out
    }

    fn english_output() -> Vec<u8> {
        b"Leap Indicator: 0(no warning)\r\nStratum: 4 (secondary reference - syncd by (S)NTP)\r\nLast Successful Sync Time: 9/29/2026 12:34:56 PM\r\nSource: time.windows.com,0x8\r\nPoll Interval: 10 (1024s)\r\nPhase Offset: 0.0004680s\r\n".to_vec()
    }

    /// Scenario: Windows の時刻同期の状態が入っている
    /// Scenario: Windows の時刻同期の状態は読んだままの出力が残る
    #[test]
    fn time_sync_parses_english_output() {
        let r = parse_status(english_output());
        assert_eq!(r.last_sync.as_deref(), Some("9/29/2026 12:34:56 PM"));
        assert_eq!(r.source.as_deref(), Some("time.windows.com,0x8"));
        assert_eq!(r.os_offset_ms, Some(0));
        assert_eq!(r.reason, None);
        assert_eq!(r.raw, english_output(), "原文に手を加えない");
    }

    /// Scenario: Windows の時刻同期の状態が入っている
    /// Scenario: OS の見積もったずれが読めたときは並ぶ
    /// Scenario: Windows の時刻同期の状態は読んだままの出力が残る
    #[test]
    fn time_sync_parses_japanese_output() {
        let r = parse_status(japanese_output());
        assert_eq!(r.last_sync.as_deref(), Some("2026/09/29 12:34:56"));
        assert_eq!(r.source.as_deref(), Some("time.windows.com,0x8"));
        assert_eq!(r.os_offset_ms, Some(-1));
        assert_eq!(r.raw, japanese_output(), "原文に手を加えない");
    }

    #[test]
    fn time_sync_unsynced_machine_keeps_source_only() {
        let out =
            b"Last Successful Sync Time: unspecified\r\nSource: Local CMOS Clock\r\n".to_vec();
        let r = parse_status(out);
        assert_eq!(r.last_sync, None);
        assert_eq!(r.source.as_deref(), Some("Local CMOS Clock"));
        assert_eq!(r.os_offset_ms, None);
        assert_eq!(r.reason, None, "同期元が読めれば解析できている");
    }

    /// Scenario: 項目を読み取れなかった出力も残る
    #[test]
    fn time_sync_unparsable_output_is_kept_with_reason() {
        let out = b"totally different\r\nformat: here\r\n".to_vec();
        let r = parse_status(out.clone());
        assert_eq!(r.raw, out);
        assert_eq!(r.reason.as_deref(), Some("unparsed"));
        assert_eq!((r.last_sync, r.source, r.os_offset_ms), (None, None, None));
    }

    /// Scenario: PC は Windows に時刻を同期させない
    #[test]
    fn time_sync_arguments_are_pinned_to_the_query() {
        let cmd = ProcessTimeSync::new().command();
        assert_eq!(cmd.get_program(), "w32tm");
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, ["/query", "/status", "/verbose"]);
    }

    #[test]
    fn time_sync_spawn_failure_is_reported_by_reason() {
        let err = run_with_timeout(
            std::process::Command::new("/nonexistent/ashiato-w32tm"),
            std::time::Duration::from_secs(1),
        )
        .unwrap_err();
        assert_eq!(err.reason(), "spawn_failed");
    }

    #[cfg(unix)]
    #[test]
    fn time_sync_nonzero_exit_is_reported_by_code() {
        let mut c = std::process::Command::new("sh");
        c.args(["-c", "echo x; exit 3"]);
        let err = run_with_timeout(c, std::time::Duration::from_secs(5)).unwrap_err();
        assert_eq!(err.reason(), "exit:3");
    }

    #[cfg(unix)]
    #[test]
    fn time_sync_timeout_kills_the_child() {
        let mut c = std::process::Command::new("sleep");
        c.arg("30");
        let started = std::time::Instant::now();
        let err = run_with_timeout(c, std::time::Duration::from_millis(200)).unwrap_err();
        assert_eq!(err.reason(), "timeout");
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }

    #[cfg(unix)]
    #[test]
    fn time_sync_output_is_returned_verbatim() {
        let mut c = std::process::Command::new("printf");
        c.arg("Source: X\\n");
        let out = run_with_timeout(c, std::time::Duration::from_secs(5)).unwrap();
        assert_eq!(out, b"Source: X\n");
    }
}
