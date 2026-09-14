// SPDX-License-Identifier: AGPL-3.0-only
//! Windows の上でだけ走る**実行時テスト**（ST07 / 2026-09-14 の決定）。
//!
//! `platform.rs`（前景・入力・アドレスバーを OS から読む側）は `cargo test` が ubuntu で走る限り
//! 1 行もコンパイルされず、確かめる手段が「人間が実機で見る」しか無かった。
//! ここでは**テストが自分で窓を作り**、本物の `WindowsSource` に読ませ、`Engine` に通して
//! 記録を数える。相手役の窓は `tests/support/helper_window.ps1`（WinForms）と Edge。
//! （mshta.exe は `about:` で即座に終了し、cmd.exe の窓は Windows Terminal が持つので題名で探せなかった。実測 2026-09-14）
//!
//! 走らせ方: Windows の上で `cargo test -p ashiato-collector-windows --test runtime_windows`。
//! CI は `windows-latest` の job が同じものを走らせる（`.github/workflows/ci.yml`）。
//! **手元で走らせる間はマウスとキーボードに触らない** —— 前景と最後の入力を本物から読むので、
//! 触ると観測が変わる。
//!
//! 閾値は短くする（最小の滞留 1 秒・離席 2 秒）。**本人が決めた 5 秒・5 分は `engine.rs` の
//! 単体テストが固定している**。ここで見るのは「OS から読んだものが Engine の規則に載るか」。
//!
//! 人間に残るもの: 画面ロックとスリープ（runner ではロックを解除できない）、本物の再起動。
#![cfg(windows)]
#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use ashiato_collector_windows::config::Config;
use ashiato_collector_windows::contract::{RecordKind, Transition, WindowPayload};
use ashiato_collector_windows::engine::{Engine, Foreground, Observation, UrlRead};
use ashiato_collector_windows::exclusion::{Exclusions, Rule};
use ashiato_collector_windows::marker::Marker;
use ashiato_collector_windows::platform::WindowsSource;
use ashiato_collector_windows::runtime::{Runtime, Source};
use ashiato_collector_windows::sender::{Reply, Transport};
use chrono::{DateTime, Utc};
use uiautomation::controls::ControlType;
use uiautomation::types::Handle;
use uiautomation::UIAutomation;

/// 前景は 1 つしか無い。**テストどうしを直列にする**（cargo test は既定で並列に走る）。
static DESKTOP: Mutex<()> = Mutex::new(());

fn desktop() -> std::sync::MutexGuard<'static, ()> {
    DESKTOP.lock().unwrap_or_else(|e| e.into_inner())
}

const MIN_DWELL: chrono::Duration = chrono::Duration::seconds(1);
const IDLE_THRESHOLD: chrono::Duration = chrono::Duration::seconds(2);

fn engine(rules: Vec<Rule>) -> Engine {
    Engine::with_thresholds(Exclusions { rules }, MIN_DWELL, IDLE_THRESHOLD)
}

// ---------------------------------------------------------------------------
// 相手役

/// WinForms の窓 1 つ（`helper_window.ps1`。プロセスは powershell.exe）。落とすと閉じる。
struct HelperWindow {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    /// 窓の HWND（相手役が `ready <HWND>` で返す）。UI Automation で前景にするときの鍵
    hwnd: isize,
}

impl HelperWindow {
    fn open(title: &str) -> Self {
        let script = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/helper_window.ps1"
        );
        let mut child = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NoLogo",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                script,
                "-Title",
                title,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("powershell.exe が起動しない");
        let stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        assert!(
            line.starts_with("ready "),
            "相手役の窓が ready を返さない: {line:?}"
        );
        let hwnd: isize = line.trim()["ready ".len()..]
            .parse()
            .expect("HWND が整数でない");
        Self {
            child,
            stdin,
            stdout,
            hwnd,
        }
    }

    fn command(&mut self, cmd: &str) {
        writeln!(self.stdin, "{cmd}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert_eq!(
            line.trim(),
            "ok",
            "相手役の窓が ok を返さない（{cmd}）: {line:?}"
        );
    }

    fn set_title(&mut self, title: &str) {
        self.command(&format!("title {title}"));
    }

    /// 自分の窓へ無害なキー（F16）を 1 つ送る。**最後の入力の時刻がいまになる**（FR-81 の材料）。
    fn input(&mut self) {
        self.command("input");
    }

    fn focus(&self) {
        focus_hwnd(self.hwnd);
    }
}

impl Drop for HelperWindow {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.stdin.flush();
        std::thread::sleep(Duration::from_millis(200));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// UI Automation でその窓を前景にする。**テスト側から呼ぶ**（`SetForegroundWindow` の
/// 制約は UI Automation の `SetFocus` には掛からない）。
fn focus_hwnd(hwnd: isize) {
    let a = UIAutomation::new().expect("UI Automation が開けない");
    let w = a
        .element_from_handle(Handle::from(hwnd))
        .unwrap_or_else(|e| panic!("HWND {hwnd} の要素が取れない: {e}"));
    w.set_focus().expect("set_focus が失敗");
}

/// プロセス識別子から最上位の窓を探して前景にする（HWND を知らない相手役 = Edge 用）。
fn focus_window_of(pid: u32) {
    let a = UIAutomation::new().expect("UI Automation が開けない");
    let start = Instant::now();
    loop {
        let walker = a.get_control_view_walker().unwrap();
        let root = a.get_root_element().unwrap();
        let mut cur = walker.get_first_child(&root).ok();
        while let Some(w) = cur {
            if w.get_process_id().ok() == Some(pid)
                && w.get_control_type().ok() == Some(ControlType::Window)
            {
                w.set_focus().expect("set_focus が失敗");
                return;
            }
            cur = walker.get_next_sibling(&w).ok();
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "pid {pid} の窓が見つからない"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

// ---------------------------------------------------------------------------
// 観測と記録

/// 前景が `want` を満たすまで、最長 `timeout` の間くり返し読む。
fn observe_until(
    source: &mut WindowsSource,
    timeout: Duration,
    want: impl Fn(&Foreground) -> bool,
) -> Observation {
    let start = Instant::now();
    loop {
        let obs = source.observe(Utc::now());
        if obs.foreground.as_ref().is_some_and(&want) || start.elapsed() > timeout {
            return obs;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// `secs` 秒のあいだ 250 ms ごとに読んで Engine に通し、出た記録を全部返す（見回りの模擬）。
fn run_for(source: &mut WindowsSource, engine: &mut Engine, secs: f64) -> Vec<WindowPayload> {
    let mut out = Vec::new();
    let start = Instant::now();
    while start.elapsed().as_secs_f64() < secs {
        out.extend(engine.observe(source.observe(Utc::now())));
        std::thread::sleep(Duration::from_millis(250));
    }
    out
}

/// 前景が `want` になるまで待ち、その間の記録も Engine に通して返す。
fn run_until(
    source: &mut WindowsSource,
    engine: &mut Engine,
    timeout: Duration,
    want: impl Fn(&Foreground) -> bool,
) -> Vec<WindowPayload> {
    let mut out = Vec::new();
    let start = Instant::now();
    loop {
        let obs = source.observe(Utc::now());
        let done = obs.foreground.as_ref().is_some_and(&want);
        out.extend(engine.observe(obs));
        if done {
            return out;
        }
        assert!(
            start.elapsed() < timeout,
            "前景が期待した状態にならない（{:?} 待った）",
            timeout
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn foregrounds(records: &[WindowPayload]) -> Vec<&WindowPayload> {
    records
        .iter()
        .filter(|r| r.kind == RecordKind::Foreground)
        .collect()
}

fn describe(obs: &Observation) -> String {
    match &obs.foreground {
        Some(f) => format!(
            "process={} title={:?} url={:?}",
            f.process_name, f.title, f.url
        ),
        None => "foreground=None".into(),
    }
}

fn summarize(records: &[WindowPayload]) -> String {
    records
        .iter()
        .map(|r| {
            format!(
                "{:?} {} {:?} {:?}",
                r.kind,
                r.process_name.as_deref().unwrap_or("-"),
                r.title,
                r.url
            )
        })
        .collect::<Vec<_>>()
        .join("\n  ")
}

// ---------------------------------------------------------------------------

/// 足場そのもの: 自分で出した窓が、本物の `WindowsSource` から題名つきで読める。
#[test]
fn helper_window_is_observed_with_its_title() {
    let _g = desktop();
    let mut w = HelperWindow::open("ashiato-rt 足場");
    w.focus();
    let mut src = WindowsSource::open();
    let obs = observe_until(&mut src, Duration::from_secs(5), |f| {
        f.process_name.eq_ignore_ascii_case("powershell.exe") && f.title == "ashiato-rt 足場"
    });
    let fg = obs
        .foreground
        .as_ref()
        .unwrap_or_else(|| panic!("前景が読めない: {}", describe(&obs)));
    assert!(
        fg.process_name.eq_ignore_ascii_case("powershell.exe"),
        "{}",
        describe(&obs)
    );
    assert_eq!(fg.title, "ashiato-rt 足場", "{}", describe(&obs));
    assert_eq!(fg.url, UrlRead::NotBrowser, "{}", describe(&obs));

    w.set_title("ashiato-rt 足場 2");
    let obs = observe_until(&mut src, Duration::from_secs(5), |f| {
        f.title == "ashiato-rt 足場 2"
    });
    assert_eq!(
        obs.foreground.as_ref().map(|f| f.title.as_str()),
        Some("ashiato-rt 足場 2"),
        "{}",
        describe(&obs)
    );
}

// Scenario: アプリを切り替えると 1 件増える
// Scenario: アプリの切り替えは滞留時間に関わらず記録される
#[test]
fn switching_app_adds_one_record_with_the_new_app() {
    let _g = desktop();
    let a = HelperWindow::open("ashiato-rt A");
    a.focus();
    let mut src = WindowsSource::open();
    let mut eng = engine(Vec::new());
    let before = run_until(&mut src, &mut eng, Duration::from_secs(5), |f| {
        f.title == "ashiato-rt A"
    });
    assert_eq!(
        foregrounds(&before).len(),
        1,
        "最初の前景が 1 件:\n  {}",
        summarize(&before)
    );

    // 別のアプリ = Edge（窓を持つプロセスが起動したプロセスと一致するので、前景にできる）
    let pages = Pages::serve();
    let edge = Edge::open(&format!("http://127.0.0.1:{}/b", pages.port));
    edge.focus();
    // 滞留を待たない —— 前景になった観測で即座に読む
    let after = run_until(&mut src, &mut eng, Duration::from_secs(20), |f| {
        f.process_name.eq_ignore_ascii_case("msedge.exe")
    });
    let fg = foregrounds(&after);
    assert_eq!(
        fg.len(),
        1,
        "切り替えで増えるのは 1 件:\n  {}",
        summarize(&after)
    );
    assert!(
        fg[0]
            .process_name
            .as_deref()
            .is_some_and(|p| p.eq_ignore_ascii_case("msedge.exe")),
        "切り替えた後のアプリを持つ:\n  {}",
        summarize(&after)
    );
    assert!(
        fg[0]
            .title
            .as_deref()
            .is_some_and(|t| t.contains("ashiato-rt page")),
        "切り替えた後の題名を持つ:\n  {}",
        summarize(&after)
    );
}

// Scenario: 題名が最小滞留より短く変わり続けても記録は増えない
// Scenario: 同じアプリの中で題名が変われば 1 件増える
#[test]
fn rapid_title_changes_add_only_the_last_title() {
    let _g = desktop();
    let mut a = HelperWindow::open("ashiato-rt T0");
    a.focus();
    let mut src = WindowsSource::open();
    let mut eng = engine(Vec::new());
    let before = run_until(&mut src, &mut eng, Duration::from_secs(5), |f| {
        f.title == "ashiato-rt T0"
    });
    assert_eq!(foregrounds(&before).len(), 1, "{}", summarize(&before));

    // 最小の滞留（1 秒）より短い間隔で 10 回変える。**間の観測も Engine に通す**
    let mut during = Vec::new();
    for i in 1..=10 {
        a.set_title(&format!("ashiato-rt T{i}"));
        std::thread::sleep(Duration::from_millis(120));
        during.extend(eng.observe(src.observe(Utc::now())));
    }
    // 最後の題名が滞留するまで待つ
    let settled = run_for(&mut src, &mut eng, 2.0);
    let all: Vec<_> = during.into_iter().chain(settled).collect();
    let fg = foregrounds(&all);
    assert_eq!(
        fg.len(),
        1,
        "増える記録は 1 件だけ（10 回の変化のうち滞留したのは最後だけ）:\n  {}",
        summarize(&all)
    );
    assert_eq!(
        fg[0].title.as_deref(),
        Some("ashiato-rt T10"),
        "最後の題名を持つ"
    );
}

// Scenario: 除外に登録した対象の本文は残らない
// Scenario: 除外した本文は取り込み口へ送られない
#[test]
fn excluded_app_leaves_no_text_in_any_record() {
    let _g = desktop();
    // 除外する側 = Edge。題名（頁の `<title>`）にも URL にも秘密の文字列を持たせる
    let pages = Pages::serve();
    let secret_url = format!("http://127.0.0.1:{}/himitsu-XK7?q=secret-XK7", pages.port);
    let mut src = WindowsSource::open();
    let a = HelperWindow::open("ashiato-rt A");
    a.focus();
    let mut eng = engine(vec![Rule::ProcessName {
        value: "msedge.exe".into(),
    }]);
    let mut all = run_until(&mut src, &mut eng, Duration::from_secs(5), |f| {
        f.title == "ashiato-rt A"
    });

    let edge = Edge::open(&secret_url);
    edge.focus();
    all.extend(run_until(
        &mut src,
        &mut eng,
        Duration::from_secs(20),
        |f| {
            f.process_name.eq_ignore_ascii_case("msedge.exe")
                && matches!(&f.url, UrlRead::Read(u) if u.contains("XK7"))
        },
    ));
    all.extend(run_for(&mut src, &mut eng, 1.5));
    // 戻す —— 除外の区間が閉じて件数の記録が出る
    a.focus();
    all.extend(run_until(&mut src, &mut eng, Duration::from_secs(8), |f| {
        f.title == "ashiato-rt A"
    }));
    all.extend(eng.flush(Utc::now()));

    let text = serde_json::to_string(&all).unwrap();
    assert!(
        !text.contains("XK7")
            && !text.contains("ashiato-rt page")
            && !text.to_lowercase().contains("msedge")
            && !text.contains("Edge"),
        "除外した対象の題名・URL・アプリ名がどの記録にも無い:\n  {}",
        summarize(&all)
    );
    assert!(
        all.iter()
            .any(|r| r.kind == RecordKind::Excluded && r.excluded_count.is_some_and(|n| n >= 1)),
        "除外が起きたことと件数だけは残る:\n  {}",
        summarize(&all)
    );
}

// Scenario: 離席の始まりと終わりが残る
// Scenario: 離席の記録は前景の記録と区別できる
#[test]
fn idle_enter_and_leave_are_both_recorded() {
    let _g = desktop();
    let mut a = HelperWindow::open("ashiato-rt idle");
    a.focus();
    let mut src = WindowsSource::open();
    let mut eng = engine(Vec::new());
    a.input();
    let mut all = run_until(&mut src, &mut eng, Duration::from_secs(5), |f| {
        f.title == "ashiato-rt idle"
    });
    // 入力を止めて閾値（2 秒）を超える
    all.extend(run_for(&mut src, &mut eng, 3.5));
    let entered = all
        .iter()
        .filter(|r| r.kind == RecordKind::Idle && r.transition == Some(Transition::Enter))
        .count();
    assert_eq!(entered, 1, "入った側が 1 件:\n  {}", summarize(&all));

    // 入力を再開する
    a.input();
    all.extend(run_for(&mut src, &mut eng, 1.5));
    let left: Vec<_> = all
        .iter()
        .filter(|r| r.kind == RecordKind::Idle && r.transition == Some(Transition::Leave))
        .collect();
    assert_eq!(left.len(), 1, "出た側が 1 件:\n  {}", summarize(&all));
    assert!(left[0].range_end.is_some(), "出た側は範囲の終わりを持つ");
    assert!(
        left[0].title.is_none() && left[0].app_name.is_none(),
        "離席の記録は前景の本文を持たない（前景の記録と区別できる）"
    );
}

// Scenario: 起動時に止まっていた期間が 1 件残る
#[test]
fn powered_off_span_is_recorded_on_start_with_real_boot_time() {
    let _g = desktop();
    #[derive(Debug, Default)]
    struct Capture(std::cell::RefCell<Vec<(String, String)>>);
    impl Transport for Capture {
        fn post(&self, path: &str, body: &str) -> anyhow::Result<Reply> {
            self.0.borrow_mut().push((path.into(), body.into()));
            let n = serde_json::from_str::<Vec<serde_json::Value>>(body)
                .map(|v| v.len())
                .unwrap_or(0);
            let items: Vec<serde_json::Value> = (0..n)
                .map(|_| serde_json::json!({"accepted": true, "duplicate": false}))
                .collect();
            Ok(Reply {
                status: 200,
                body: serde_json::to_string(&items).unwrap(),
            })
        }
    }
    #[derive(Debug)]
    struct NoReference;
    impl ashiato_collector_windows::clock::ReferenceClock for NoReference {
        fn now(&self) -> anyhow::Result<DateTime<Utc>> {
            anyhow::bail!("基準時刻は取らない")
        }
        fn source(&self) -> String {
            "none".into()
        }
    }

    let dir = std::env::temp_dir().join(format!("ashiato-rt-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let last_seen = Utc::now() - chrono::Duration::hours(1);
    Marker::new(&dir).touch(last_seen).unwrap();

    let cfg = Config {
        base_url: "http://127.0.0.1:1".into(),
        api_token: "t".into(),
        user_id: uuid::Uuid::nil(),
        device_id: "rt".into(),
        state_dir: dir.clone(),
    };
    let transport = Capture::default();
    let reference = NoReference;
    let zone = ashiato_collector_windows::config::Zone::current().unwrap();
    let mut rt = Runtime::new(
        &cfg,
        zone,
        engine(Vec::new()),
        &transport,
        &reference,
        Utc::now(),
    )
    .unwrap();
    let mut source = WindowsSource::open();
    rt.start(&source);
    rt.tick(&mut source);
    rt.stop();

    let sent: Vec<serde_json::Value> = transport
        .0
        .borrow()
        .iter()
        .filter(|(p, _)| p == "/ingest")
        .flat_map(|(_, b)| serde_json::from_str::<Vec<serde_json::Value>>(b).unwrap())
        .collect();
    let off: Vec<_> = sent
        .iter()
        .filter(|r| r["payload"]["kind"] == "powered-off")
        .collect();
    assert_eq!(off.len(), 1, "止まっていた期間が 1 件: {sent:?}");
    let p = &off[0]["payload"];
    assert_eq!(
        p["at"].as_str().map(|s| &s[..16]),
        Some(&last_seen.to_rfc3339()[..16]),
        "範囲の始まりは前回の印: {p}"
    );
    assert!(p["range_end"].is_string(), "範囲の終わりを持つ: {p}");
    assert!(
        p["boot_at"].is_string(),
        "本物の OS の起動時刻が載る（design D23）: {p}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// ブラウザ（Edge）。アドレスバーは UI Automation でしか読めないので、ここでしか確かめられない。

/// どのパスにも同じ `<title>` の頁を返す小さな HTTP サーバ（**題名を変えずに URL だけ変える**ため）。
///
/// 頁は `/next` を 200 ms ごとに読み、テストが `go(url)` で次の URL を置いたら**自分でそこへ移る**
/// （同じタブの中で URL だけが変わる。Edge をもう 1 度起動すると新しいタブが開き、
/// 窓の題名に「および他 1 ページ」が付いて「題名を変えずに」が崩れる。実測 2026-09-14）。
struct Pages {
    port: u16,
    next: std::sync::Arc<Mutex<Option<String>>>,
}

impl Pages {
    fn serve() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let next = std::sync::Arc::new(Mutex::new(None::<String>));
        let shared = std::sync::Arc::clone(&next);
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]);
                let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (ctype, body) = if path.starts_with("/next") {
                    (
                        "text/plain",
                        shared.lock().unwrap().take().unwrap_or_default(),
                    )
                } else {
                    (
                        "text/html; charset=utf-8",
                        "<html><head><title>ashiato-rt page</title></head><body>rt<script>\
                         setInterval(function(){fetch('/next').then(function(r){return r.text()})\
                         .then(function(u){if(u){location.href=u;}});},200);\
                         </script></body></html>"
                            .to_string(),
                    )
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
                    ctype,
                    body.len(),
                    body
                );
            }
        });
        Self { port, next }
    }

    /// 次にどの頁も `url` へ移る。
    fn go(&self, url: &str) {
        *self.next.lock().unwrap() = Some(url.to_string());
    }
}

struct Edge {
    child: Child,
    profile: std::path::PathBuf,
}

impl Edge {
    fn exe() -> Option<&'static str> {
        [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ]
        .into_iter()
        .find(|p| std::path::Path::new(p).is_file())
    }

    /// 使い捨てのプロファイルで開く（初回体験や同期の画面を出さない）。
    fn open(url: &str) -> Self {
        let profile =
            std::env::temp_dir().join(format!("ashiato-rt-edge-{}", uuid::Uuid::new_v4()));
        let child = Command::new(Self::exe().expect("Edge が無い"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .args([
                "--no-first-run",
                "--disable-sync",
                "--disable-features=msEdgeFirstRunExperience,msImplicitSignin",
                "--new-window",
                "--window-size=900,600",
                url,
            ])
            .spawn()
            .expect("Edge が起動しない");
        Self { child, profile }
    }

    fn focus(&self) {
        focus_window_of(self.child.id());
    }
}

impl Drop for Edge {
    fn drop(&mut self) {
        let _ = Command::new("taskkill")
            .args(["/PID", &self.child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = self.child.wait();
        std::thread::sleep(Duration::from_millis(500));
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

// Scenario: クエリとフラグメントが残る
// Scenario: 表示されている文字列を補正しない
// Scenario: URL だけが変われば 1 件増える
// Scenario: URL の変化は滞留時間に関わらず記録される
#[test]
fn browser_url_is_recorded_as_displayed_and_a_url_change_adds_one_record() {
    let _g = desktop();
    let pages = Pages::serve();
    let port = pages.port;
    let first = format!("http://127.0.0.1:{port}/one?x=1&y=%E3%81%82#frag-1");
    let second = format!("http://127.0.0.1:{port}/two?x=2#frag-2");
    let edge = Edge::open(&first);
    let mut src = WindowsSource::open();
    let mut eng = engine(Vec::new());
    edge.focus();
    let all = run_until(&mut src, &mut eng, Duration::from_secs(20), |f| {
        f.process_name.eq_ignore_ascii_case("msedge.exe")
            && matches!(&f.url, UrlRead::Read(u) if u.contains("/one"))
    });
    let fg = foregrounds(&all);
    let url = fg
        .last()
        .and_then(|r| r.url.clone())
        .unwrap_or_else(|| panic!("URL が読めていない:\n  {}", summarize(&all)));
    assert!(
        url.contains("?x=1&y=") && url.ends_with("#frag-1"),
        "クエリとフラグメントが残る: {url}"
    );
    // Edge は `http://` を隠して表示する。**補正しない**ので、記録も隠したまま
    assert!(
        !url.starts_with("http://") && url.starts_with("127.0.0.1:"),
        "表示されている文字列（`http://` 無し）のまま: {url}"
    );

    // 題名を変えずに URL だけ変える（両方の頁の `<title>` は同じ）
    let n_before = fg.len();
    pages.go(&second);
    let more = run_until(
        &mut src,
        &mut eng,
        Duration::from_secs(20),
        |f| matches!(&f.url, UrlRead::Read(u) if u.contains("/two")),
    );
    let all: Vec<_> = all.into_iter().chain(more).collect();
    let fg = foregrounds(&all);
    assert_eq!(
        fg.len(),
        n_before + 1,
        "URL だけが変われば 1 件増える:\n  {}",
        summarize(&all)
    );
    let last = fg.last().unwrap();
    assert!(
        last.url
            .as_deref()
            .is_some_and(|u| u.contains("/two?x=2#frag-2")),
        "新しい URL を持つ:\n  {}",
        summarize(&all)
    );
    assert_eq!(
        last.title,
        fg[n_before - 1].title,
        "題名は変わっていない:\n  {}",
        summarize(&all)
    );
}
