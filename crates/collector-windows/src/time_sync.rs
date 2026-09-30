// SPDX-License-Identifier: AGPL-3.0-only
//! Windows の時刻同期の状態を読む口（ST05 design D8）。
//!
//! 本番は `w32tm /query /status /verbose` を子プロセスで走らせる。**渡す引数は照会だけ**
//! （`/resync` や `/config` は渡さない。外部への通信も起きない）。出力は OS の表示言語の
//! コンソールの符号ページ（日本語は cp932）のバイト列なので、**バイト列のまま原文として持ち**、
//! 見出しの照合もバイト列で行う。
//!
//! **W32Time が止まっているとき**（`w32tm` が `0x80070426` で終わる）は、System のイベントログの
//! `Microsoft-Windows-Time-Service` の同期の記録（Event 35 / 37）の最新 1 件を `wevtutil qe` で読み、
//! その時刻と同期元を並べる（deep Q4。2026-09-30 本人の答え）。**サービスは起動しない・起動の種類も変えない。**
//! 記録が無い・読めないときは、止まっていたこと（`service_stopped`）だけを残す。
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// 子プロセスを待つ上限（design D8）。
pub const TIME_SYNC_TIMEOUT: Duration = Duration::from_secs(5);

const ARGS: [&str; 3] = ["/query", "/status", "/verbose"];

/// イベントログの照会（`qe` = 読むだけ）。Time-Service の同期の記録（35 = 同期元を選んで同期している /
/// 37 = 同期元から正しい時刻を受けている）の**新しいほうから 1 件**を XML で。
const EVENT_LOG_ARGS: [&str; 6] = [
    "qe",
    "System",
    "/q:*[System[Provider[@Name='Microsoft-Windows-Time-Service'] and (EventID=35 or EventID=37)]]",
    "/c:1",
    "/rd:true",
    "/f:xml",
];

/// 時刻同期の状態をどこから読んだか（記録の `sync_via`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSyncVia {
    /// `w32tm /query /status /verbose`
    W32tm,
    /// W32Time が止まっていたので、イベントログの同期の記録から（`last_sync` は UTC の RFC 3339）
    EventLog,
}

impl TimeSyncVia {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::W32tm => "w32tm",
            Self::EventLog => "eventlog",
        }
    }
}

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
    /// どこから読んだか。
    pub via: TimeSyncVia,
}

/// 読めなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeSyncError {
    SpawnFailed,
    Exit(Option<i32>),
    Timeout,
}

/// `w32tm` の終了コードが「Windows Time サービスが開始されていない」（HRESULT `0x80070426`）。
/// W32Time は既定で手動（トリガー）起動なので、止まっているのは異常ではない（2026-09-30 実測: `DEMAND_START` で停止中）。
pub const SERVICE_NOT_STARTED: i32 = 0x8007_0426_u32 as i32;

impl TimeSyncError {
    /// 記録に載せる理由（`spawn_failed` / `service_stopped` / `exit:<code>` / `timeout`）。
    pub fn reason(&self) -> String {
        match self {
            Self::SpawnFailed => "spawn_failed".into(),
            Self::Exit(Some(SERVICE_NOT_STARTED)) => "service_stopped".into(),
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
        let mut c = Command::new(program(std::env::var_os("SystemRoot"), "w32tm.exe"));
        c.args(ARGS);
        c
    }

    /// W32Time が止まっているときに走らせるコマンド。**引数はイベントログの照会だけに固定**（試験で固定する）。
    pub fn event_log_command(&self) -> Command {
        let mut c = Command::new(program(std::env::var_os("SystemRoot"), "wevtutil.exe"));
        c.args(EVENT_LOG_ARGS);
        c
    }

    /// イベントログから最後の同期を読む。記録が無ければ `Ok(None)`。
    pub fn read_event_log(&self) -> Result<Option<TimeSyncReading>, TimeSyncError> {
        run_with_timeout(self.event_log_command(), TIME_SYNC_TIMEOUT).map(parse_event_log)
    }
}

/// `%SystemRoot%\System32\<exe>`。**PATH や exe の置き場所からは探さない** ——
/// 探すと、収集の exe と同じ場所や PATH の先に置かれた別の `w32tm` を掴む（review R13）。
/// `SystemRoot` が無いときは Windows の既定の置き場所。
fn program(system_root: Option<std::ffi::OsString>, exe: &str) -> std::ffi::OsString {
    let mut p = system_root.unwrap_or_else(|| r"C:\Windows".into());
    p.push(r"\System32\");
    p.push(exe);
    p
}

impl TimeSyncSource for ProcessTimeSync {
    fn read(&self) -> Result<TimeSyncReading, TimeSyncError> {
        with_event_log_fallback(
            run_with_timeout(self.command(), TIME_SYNC_TIMEOUT).map(parse_status),
            || self.read_event_log(),
        )
    }
}

/// `w32tm` が「サービスが開始されていない」で終わったときだけ、イベントログを読む。
/// イベントログに記録が無い・読めない（権限・打ち切り）ときは `service_stopped` のまま返す
/// （止まっていたことだけを残す。deep Q4 の推奨の条件）。
pub fn with_event_log_fallback(
    w32tm: Result<TimeSyncReading, TimeSyncError>,
    event_log: impl FnOnce() -> Result<Option<TimeSyncReading>, TimeSyncError>,
) -> Result<TimeSyncReading, TimeSyncError> {
    match w32tm {
        Err(stopped @ TimeSyncError::Exit(Some(SERVICE_NOT_STARTED))) => {
            event_log().ok().flatten().ok_or(stopped)
        }
        other => other,
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

/// 出力を記録に載せる文字列にする。**JSON の文字列は cp932 のバイト列を持てない**ので、
/// ASCII はそのまま、ASCII 以外のバイトと `\` は `\xNN` にして、元のバイト列へ戻せる形にする（D8（仮））。
pub fn raw_text(raw: &[u8]) -> String {
    let mut s = String::with_capacity(raw.len());
    for &b in raw {
        if b.is_ascii() && b != b'\\' {
            s.push(char::from(b));
        } else {
            s.push_str(&format!("\\x{b:02x}"));
        }
    }
    s
}

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
        via: TimeSyncVia::W32tm,
    }
}

/// `wevtutil qe … /f:xml` の出力（最新 1 件）を解析する。**記録が 1 件も無ければ `None`**。
/// 時刻は `TimeCreated` の `SystemTime`（UTC）、同期元は `TimeSource` の値。原文は必ず持つ。
pub fn parse_event_log(raw: Vec<u8>) -> Option<TimeSyncReading> {
    let text = String::from_utf8_lossy(&raw).into_owned();
    if !text.contains("<Event") {
        return None;
    }
    let last_sync = attr_value(&text, "SystemTime=");
    let source = between(&text, "Name='TimeSource'>", "</Data>")
        .or_else(|| between(&text, "Name=\"TimeSource\">", "</Data>"))
        .map(unescape_xml)
        .filter(|s| !s.trim().is_empty());
    let reason = (last_sync.is_none() && source.is_none()).then(|| "unparsed".to_string());
    Some(TimeSyncReading {
        raw,
        last_sync,
        source,
        os_offset_ms: None,
        reason,
        via: TimeSyncVia::EventLog,
    })
}

/// `name='値'` / `name="値"` の値（最初の 1 つ）。
fn attr_value(text: &str, name: &str) -> Option<String> {
    let rest = &text[text.find(name)? + name.len()..];
    let quote = rest.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    let rest = &rest[1..];
    Some(rest[..rest.find(quote)?].to_string()).filter(|v| !v.is_empty())
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let rest = &text[text.find(start)? + start.len()..];
    Some(&rest[..rest.find(end)?])
}

fn unescape_xml(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
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
        let program = cmd.get_program().to_string_lossy();
        assert!(program.ends_with(r"\System32\w32tm.exe"), "{program}");
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, ["/query", "/status", "/verbose"]);
    }

    /// W32Time が止まっているときもサービスを起動しない。渡すのはイベントログの照会（`qe`）だけ（deep Q4）。
    ///
    /// Scenario: PC は Windows に時刻を同期させない
    #[test]
    fn time_sync_event_log_arguments_are_pinned_to_the_query() {
        let cmd = ProcessTimeSync::new().event_log_command();
        let program = cmd.get_program().to_string_lossy();
        assert!(program.ends_with(r"\System32\wevtutil.exe"), "{program}");
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(
            args,
            [
                "qe",
                "System",
                "/q:*[System[Provider[@Name='Microsoft-Windows-Time-Service'] and (EventID=35 or EventID=37)]]",
                "/c:1",
                "/rd:true",
                "/f:xml",
            ]
        );
    }

    /// 手元の Windows で読めた Event 37 の形（2026-09-30。`wevtutil qe System … /f:xml`）。
    fn event_log_output() -> Vec<u8> {
        br#"<Event xmlns='http://schemas.microsoft.com/win/2004/08/events/event'><System><Provider Name='Microsoft-Windows-Time-Service' Guid='{06edcfeb-0fd0-4e53-acca-a6f8bbf81bcb}'/><EventID>37</EventID><Version>0</Version><Level>4</Level><Task>0</Task><Opcode>0</Opcode><Keywords>0x8000000000000000</Keywords><TimeCreated SystemTime='2026-09-30T11:47:51.9244260Z'/><EventRecordID>1463</EventRecordID><Correlation/><Execution ProcessID='9856' ThreadID='4112'/><Channel>System</Channel><Computer>pc</Computer><Security UserID='S-1-5-19'/></System><EventData Name='TMP_EVENT_TIME_SOURCE_REACHABLE'><Data Name='TimeSource'>time.windows.com,0x9 (ntp.m|0x9|0.0.0.0:123-&gt;20.43.94.199:123)</Data></EventData></Event>"#.to_vec()
    }

    /// Scenario: Windows の時刻同期のサービスが止まっていても最後の同期が並ぶ
    /// Scenario: Windows の時刻同期の状態は読んだままの出力が残る
    #[test]
    fn time_sync_event_log_gives_last_sync_and_source() {
        let r = parse_event_log(event_log_output()).unwrap();
        assert_eq!(r.last_sync.as_deref(), Some("2026-09-30T11:47:51.9244260Z"));
        assert_eq!(
            r.source.as_deref(),
            Some("time.windows.com,0x9 (ntp.m|0x9|0.0.0.0:123->20.43.94.199:123)")
        );
        assert_eq!(r.os_offset_ms, None);
        assert_eq!(r.reason, None);
        assert_eq!(r.via, TimeSyncVia::EventLog);
        assert_eq!(r.raw, event_log_output(), "原文に手を加えない");
    }

    #[test]
    fn time_sync_event_log_without_records_is_none() {
        assert_eq!(parse_event_log(Vec::new()), None);
        assert_eq!(parse_event_log(b"\r\n".to_vec()), None);
    }

    /// Scenario: 項目を読み取れなかった出力も残る
    #[test]
    fn time_sync_event_log_unparsable_record_is_kept_with_reason() {
        let out = b"<Event><System/></Event>".to_vec();
        let r = parse_event_log(out.clone()).unwrap();
        assert_eq!(r.raw, out);
        assert_eq!(r.reason.as_deref(), Some("unparsed"));
    }

    /// 止まっているときだけイベントログを読み、読めればその値、無い・読めなければ `service_stopped`。
    ///
    /// Scenario: Windows の時刻同期のサービスが止まっていても最後の同期が並ぶ
    /// Scenario: Windows の時刻同期のサービスが止まっていて同期の記録も無ければ止まっていたことが残る
    #[test]
    fn time_sync_stopped_service_falls_back_to_the_event_log() {
        let stopped = || Err(TimeSyncError::Exit(Some(SERVICE_NOT_STARTED)));
        let from_log = || Ok(parse_event_log(event_log_output()));
        let r = with_event_log_fallback(stopped(), from_log).unwrap();
        assert_eq!(r.via, TimeSyncVia::EventLog);
        assert_eq!(r.last_sync.as_deref(), Some("2026-09-30T11:47:51.9244260Z"));

        let err = with_event_log_fallback(stopped(), || Ok(None)).unwrap_err();
        assert_eq!(err.reason(), "service_stopped", "記録が無い");
        let err =
            with_event_log_fallback(stopped(), || Err(TimeSyncError::Exit(Some(5)))).unwrap_err();
        assert_eq!(err.reason(), "service_stopped", "読めない（権限など）");

        // 止まっている以外の失敗・取れたときはイベントログを読まない
        let never = || -> Result<Option<TimeSyncReading>, TimeSyncError> {
            panic!("イベントログを読んだ")
        };
        let err = with_event_log_fallback(Err(TimeSyncError::Timeout), never).unwrap_err();
        assert_eq!(err.reason(), "timeout");
        let r = with_event_log_fallback(Ok(parse_status(english_output())), never).unwrap();
        assert_eq!(r.via, TimeSyncVia::W32tm);
    }

    /// 走らせるのは `%SystemRoot%` の下の `w32tm.exe` だけ（PATH から探さない。review R13）。
    #[test]
    fn time_sync_program_is_resolved_from_the_system_root() {
        assert_eq!(
            program(Some(r"D:\WinNT".into()), "w32tm.exe"),
            r"D:\WinNT\System32\w32tm.exe"
        );
        assert_eq!(
            program(None, "wevtutil.exe"),
            r"C:\Windows\System32\wevtutil.exe"
        );
    }

    /// 記録に載る原文（`raw_text`）は元のバイト列へ戻せる。`\` と `\x8d` という 4 文字の並びと、
    /// cp932 の 1 バイトを区別できる（Scenario の「読んだままの出力」を記録の段で見る。review R23）。
    ///
    /// Scenario: Windows の時刻同期の状態は読んだままの出力が残る
    #[test]
    fn time_sync_raw_text_round_trips_to_the_bytes() {
        fn back(s: &str) -> Vec<u8> {
            let b = s.as_bytes();
            let mut out = Vec::new();
            let mut i = 0;
            while i < b.len() {
                if b[i] == b'\\' {
                    assert_eq!(b.get(i + 1), Some(&b'x'), "`\\` の後ろが `x` でない: {s}");
                    let hex = std::str::from_utf8(&b[i + 2..i + 4]).unwrap();
                    out.push(u8::from_str_radix(hex, 16).unwrap());
                    i += 4;
                } else {
                    out.push(b[i]);
                    i += 1;
                }
            }
            out
        }
        let mut raw = japanese_output();
        raw.extend_from_slice(br"C:\x8d\path\ Source: \x");
        raw.extend_from_slice(&[0x8d, 0x5c, b'\n', 0x00, 0xff]);
        let text = raw_text(&raw);
        assert!(text.is_ascii());
        assert_eq!(back(&text), raw);
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

    #[test]
    fn time_sync_stopped_service_is_reported_as_such() {
        // 実 OS の状態に依らない: 停止中の w32tm が返す終了コードを固定で与える
        assert_eq!(
            TimeSyncError::Exit(Some(-2_147_023_834)).reason(),
            "service_stopped"
        );
        assert_eq!(
            TimeSyncError::Exit(Some(SERVICE_NOT_STARTED)).reason(),
            "service_stopped"
        );
        assert_eq!(TimeSyncError::Exit(Some(1)).reason(), "exit:1");
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
