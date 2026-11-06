#![allow(clippy::print_stdout)] // インストーラは端末に案内と問いを表示する唯一の crate。

use ashiato_setup::commands::{self, SetupCommand};
use ashiato_setup::{db, layout, secrets, state};
use std::fs;
use std::io::{BufRead as _, IsTerminal as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// 配る zip を作る前に、サーバと同じ組み立ての印を成果物から照合する。
// `main` の `black_box` で参照し、Windows/MSVC のリンク最適化後にも exe に残す。
const BUILD_STAMP: &str = env!("ASHIATO_BUILD_STAMP");

/// 試験の組み立てだけが読む差し込み（`ASHIATO_SETUP_TEST_<名前>`。D24）。配る組み立てでは
/// 名前の文字列ごと組み込まれない（`check-artifacts.sh` が配る実行ファイルの文字列を見る）。
macro_rules! test_hook {
    ($name:literal) => {{
        #[cfg(ashiato_test_build)]
        {
            std::env::var(concat!("ASHIATO_SETUP_TEST_", $name)).ok()
        }
        #[cfg(not(ashiato_test_build))]
        {
            None::<String>
        }
    }};
}

fn terminal_line(line: &str, error: bool) {
    let mut output: Box<dyn std::io::Write> = if error {
        Box::new(std::io::stderr())
    } else {
        Box::new(std::io::stdout())
    };
    let _ = writeln!(output, "{line}");
}

/// User Shell Folders の REG_EXPAND_SZ を、昇格前に得た本人のプロファイルへ展開する。
#[cfg_attr(not(windows), allow(dead_code))] // Windows の user_downloads からだけ呼ぶ
fn expand_downloads_shell_folder(value: &str, user_profile: &str) -> String {
    value.replace("%USERPROFILE%", user_profile)
}

/// `reg.exe query … /v <名前>` の出力から値を読む。値は型の語（`REG_EXPAND_SZ` など）の後ろの全部
/// （空白を含むパスを途中で切らない。review R24）。
#[cfg_attr(not(windows), allow(dead_code))] // Windows の user_downloads からだけ呼ぶ
fn reg_query_value(output: &str, name: &str) -> Option<String> {
    let line = output.lines().find(|line| line.contains(name))?;
    let after_name = &line[line.find(name)? + name.len()..];
    let after_type = after_name.trim_start();
    let (kind, rest) = after_type.split_once(char::is_whitespace)?;
    kind.starts_with("REG_")
        .then(|| rest.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(windows)]
fn user_downloads(profile: &str) -> String {
    const DOWNLOADS: &str = "{374DE290-123F-4565-9164-39C4925E467B}";
    Command::new("reg.exe")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders",
            "/v",
            DOWNLOADS,
        ])
        .output()
        .ok()
        .filter(|result| result.status.success())
        .and_then(|result| reg_query_value(&String::from_utf8_lossy(&result.stdout), DOWNLOADS))
        .map(|value| expand_downloads_shell_folder(&value, profile))
        .unwrap_or_else(|| format!(r#"{}\Downloads"#, profile.trim_end_matches(['/', '\\'])))
}

#[cfg(not(windows))]
fn user_downloads(profile: &str) -> String {
    format!(r#"{}\Downloads"#, profile.trim_end_matches(['/', '\\']))
}

fn output_tail(output: &std::process::Output) -> String {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    lines[lines.len().saturating_sub(5)..].join(" / ")
}

/// 命令を起こし、失敗なら理由を返す。命令には秘密を載せないので、記録の行にも秘密は出ない。
fn run(command: &SetupCommand, trace: bool) -> Result<(), String> {
    let output = Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("{} を起こせない: {error}", command.program))?;
    if !output.status.success() {
        return Err(format!(
            "{} が失敗した (終了コード {:?}): {}",
            command.line(),
            output.status.code(),
            output_tail(&output)
        ));
    }
    if trace {
        terminal_line(&format!("result=ok {}", command.line()), false);
    }
    Ok(())
}

/// 常駐する子を起こす命令（repair の `pg_ctl start`）を起こし、**親の終了だけ**を待つ。
/// `output()` は子が継承した pipe を閉じるまで待つので、PostgreSQL が動く間ずっと戻らない
/// （run 42163474147 の repair が 1 時間戻らなかった）。標準ハンドルを null にして `status()` で待つ。
/// ほかの命令は `run` のまま（出力の末尾を理由に載せる。全部をこちらにすると、DB のサービスの
/// 回復の Scenario が落ちるようになった。run 42175126266・42179701302）
fn run_detached(command: &SetupCommand, trace: bool) -> Result<(), String> {
    let status = Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("{} を起こせない: {error}", command.program))?;
    if !status.success() {
        return Err(format!(
            "{} が失敗した (終了コード {:?})",
            command.line(),
            status.code(),
        ));
    }
    if trace {
        terminal_line(&format!("result=ok {}", command.line()), false);
    }
    Ok(())
}

fn run_all(commands: &[SetupCommand], trace: bool) -> Result<(), String> {
    commands.iter().try_for_each(|command| run(command, trace))
}

/// サーバの仮想アカウントに DB のサービスの見張り（状態を読む・開始する）を許す（D35）。
/// `sc.exe sdshow` の結果を `commands::db_watch_sdset_command` に通し、足すものがあるときだけ `sdset` を起こす。
fn grant_db_watch(layout: &layout::Layout, trace: bool) -> Result<(), String> {
    let show = commands::db_service_sdshow_command(layout);
    let output = Command::new(&show.program)
        .args(&show.args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("{} を起こせない: {error}", show.program))?;
    if !output.status.success() {
        return Err(format!(
            "{} が失敗した (終了コード {:?}): {}",
            show.line(),
            output.status.code(),
            output_tail(&output)
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let current = commands::sddl_from_sdshow(&text)
        .ok_or_else(|| format!("{} の出力に SDDL が無い", show.line()))?;
    match commands::db_watch_sdset_command(layout, current) {
        Some(set) => run(&set, trace),
        None => {
            if trace {
                terminal_line("result=ok db watch ace already present", false);
            }
            Ok(())
        }
    }
}

/// サービスの確かめ直し・回復の設定と、見張りの ACE。
fn configure_services(layout: &layout::Layout, trace: bool) -> Result<(), String> {
    run_all(&commands::configure_service_commands(layout), trace)?;
    grant_db_watch(layout, trace)
}

/// サービスの状態の番号。サービスが無ければ `None`。
fn service_state(service: &str) -> Option<u32> {
    let command = commands::query_service_command(service);
    let output = Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    commands::service_state_code(&String::from_utf8_lossy(&output.stdout))
}

fn service_exists(service: &str) -> bool {
    service_state(service).is_some()
}

fn service_running(service: &str) -> bool {
    service_state(service) == Some(commands::SERVICE_RUNNING)
}

fn ensure_running(service: &str, trace: bool) -> Result<(), String> {
    if service_running(service) {
        return Ok(());
    }
    run(&commands::start_service_command(service), trace)
}

fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    while Instant::now() < deadline {
        if std::net::TcpStream::connect_timeout(&address, Duration::from_secs(1)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    false
}

/// 同梱の `pg_isready` が「接続を受け付ける」（終了コード 0）を返すまで待つ。
fn wait_for_database(layout: &layout::Layout, port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let tool = commands::bundled_tool(layout, "pg_isready.exe");
    while Instant::now() < deadline {
        let ready = Command::new(&tool)
            .args(["-q", "-h", "127.0.0.1", "-p", &port.to_string()])
            .stdin(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if ready {
            return true;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    false
}

fn whoami(args: &[&str]) -> Result<String, String> {
    let output = Command::new("whoami.exe")
        .args(args)
        .output()
        .or_else(|_| Command::new("whoami").args(args).output())
        .map_err(|error| format!("実行した利用者を読めない: {error}"))?;
    if !output.status.success() {
        return Err("実行した利用者を読めない".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// いま動いている利用者の SID（上がる前なら本人、上がった後なら上がった利用者）。
fn current_user_sid() -> Result<String, String> {
    commands::owner_sid_from_whoami(&whoami(&["/user", "/fo", "csv", "/nh"])?)
        .ok_or_else(|| "実行した利用者の SID を読めない".to_owned())
}

#[cfg(windows)]
fn is_elevated() -> bool {
    whoami(&["/groups"]).is_ok_and(|groups| commands::is_elevated_from_whoami_groups(&groups))
}

#[cfg(not(windows))]
fn is_elevated() -> bool {
    true
}

/// 自分を管理者として起こし直し、その終了コードで終わる（D22。確認は 1 回）。
fn relaunch_elevated(script: &str) -> Result<(), String> {
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .status()
        .map_err(|error| format!("管理者として起こし直せない: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "管理者として動いた方が失敗したか、確認を断った (終了コード {:?})",
            status.code()
        ))
    }
}

fn read_web_passwords(api_token: &str, session_key: &str) -> Result<String, String> {
    let terminal = std::io::stdin().is_terminal();
    let mut lines = std::io::stdin().lock().lines();
    loop {
        let pair = if terminal {
            (
                rpassword::prompt_password("画面の合言葉: ")
                    .map_err(|error| format!("合言葉を読めない: {error}"))?,
                rpassword::prompt_password("画面の合言葉（もう一度）: ")
                    .map_err(|error| format!("合言葉を読めない: {error}"))?,
            )
        } else {
            (
                lines
                    .next()
                    .ok_or("合言葉の入力が無い")?
                    .map_err(|error| format!("合言葉を読めない: {error}"))?,
                lines
                    .next()
                    .ok_or("合言葉の確認入力が無い")?
                    .map_err(|error| format!("合言葉を読めない: {error}"))?,
            )
        };
        match ashiato_setup::password::validate(&pair.0, api_token, session_key, &pair.1) {
            Ok(()) => {
                return ashiato_setup::password::verifier(&pair.0)
                    .map_err(|error| format!("合言葉の照らすための値を作れない: {error}"))
            }
            Err(reason) => terminal_line(
                &format!(
                    "合言葉を受け付けない: {}。入れ直してください",
                    reason.reason()
                ),
                true,
            ),
        }
    }
}

/// 既定のブラウザで画面を開く。開けなくてもインストールは終わっているので止めない。
fn open_browser(layout: &layout::Layout, trace: bool) {
    let url = format!("http://127.0.0.1:{}/", layout.server_port);
    let command = SetupCommand::powershell(format!("Start-Process '{url}'"));
    match Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => {
            if trace {
                terminal_line("result=ok Start-Process <web-url>", false);
            }
        }
        _ => terminal_line(&format!("ブラウザで {url} を開いてください"), false),
    }
}

/// 登録の前に番号を見る（D5）。自分のサービスが動いて持っている番号は塞がっているとみなさない。
fn ensure_ports_available(layout: &layout::Layout) -> Result<(), String> {
    let mut text = String::new();
    for protocol in commands::netstat_protocols() {
        let args = ["-ano", "-p", protocol];
        let output = Command::new("netstat.exe")
            .args(args)
            .output()
            .or_else(|_| Command::new("netstat").args(args).output())
            .map_err(|error| format!("待ち受け番号 ({protocol}) を調べられない: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "待ち受け番号 ({protocol}) を調べられない (終了コード {:?})",
                output.status.code()
            ));
        }
        text.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    for (port, service) in [
        (layout.db_port, layout.db_service),
        (layout.server_port, layout.server_service),
    ] {
        let Some(listener) = commands::parse_netstat_listener(&text, port) else {
            continue;
        };
        if service_running(service) {
            continue;
        }
        let owner = Command::new("tasklist.exe")
            .args([
                "/FI",
                &format!("PID eq {}", listener.pid),
                "/FO",
                "CSV",
                "/NH",
            ])
            .output()
            .ok()
            .map(|result| String::from_utf8_lossy(&result.stdout).trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("PID {}", listener.pid));
        return Err(format!(
            "番号 {port} は {} ({owner}) が使用中です",
            listener.address
        ));
    }
    Ok(())
}

fn free_space_gb() -> Option<u64> {
    if let Some(value) = test_hook!("FREE_GB") {
        return value.parse().ok();
    }
    #[cfg(windows)]
    {
        let bytes = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-PSDrive -Name C).Free",
            ])
            .output()
            .ok()
            .filter(|result| result.status.success())
            .and_then(|result| {
                String::from_utf8_lossy(&result.stdout)
                    .trim()
                    .parse::<u64>()
                    .ok()
            });
        bytes.map(|value| value / 1024 / 1024 / 1024)
    }
    #[cfg(not(windows))]
    None
}

/// 上がった利用者と本人。本人は ACL の読みを持ち、上がった利用者は書く段の間だけ書きを持つ（D21・D22）。
struct Identity {
    owner: String,
    invoking: String,
}

/// 書く段の間だけ足した一時の書きを、途中で落ちても外す（D21）。
struct TemporaryWrite<'a> {
    layout: &'a layout::Layout,
    active: bool,
}

impl<'a> TemporaryWrite<'a> {
    fn grant(layout: &'a layout::Layout, invoking: &str, trace: bool) -> Result<Self, String> {
        let guard = Self {
            layout,
            active: true,
        };
        run_all(
            &commands::temporary_write_acl_commands(layout, invoking),
            trace,
        )?;
        Ok(guard)
    }

    fn finish(mut self, trace: bool) -> Result<(), String> {
        self.active = false;
        run_all(
            &commands::remove_temporary_write_acl_commands(self.layout),
            trace,
        )
    }
}

impl Drop for TemporaryWrite<'_> {
    fn drop(&mut self) {
        if self.active {
            for command in commands::remove_temporary_write_acl_commands(self.layout) {
                let _ = run(&command, false);
            }
        }
    }
}

/// DB の置き場に足した一時の書きを、途中で落ちても外す（DB のサービスを起こす前に外す）。
struct DatabaseWrite<'a> {
    layout: &'a layout::Layout,
    invoking: String,
    owner: String,
    active: bool,
}

impl<'a> DatabaseWrite<'a> {
    fn grant(
        layout: &'a layout::Layout,
        invoking: &str,
        owner: &str,
        trace: bool,
    ) -> Result<Self, String> {
        let guard = Self {
            layout,
            invoking: invoking.to_owned(),
            owner: owner.to_owned(),
            active: true,
        };
        run_all(
            &commands::database_write_acl_commands(layout, invoking, owner),
            trace,
        )?;
        Ok(guard)
    }

    fn finish(mut self, trace: bool) -> Result<(), String> {
        self.active = false;
        run_all(
            &commands::remove_database_write_acl_commands(self.layout, &self.invoking, &self.owner),
            trace,
        )
    }
}

impl Drop for DatabaseWrite<'_> {
    fn drop(&mut self) {
        if self.active {
            for command in commands::remove_database_write_acl_commands(
                self.layout,
                &self.invoking,
                &self.owner,
            ) {
                let _ = run(&command, false);
            }
        }
    }
}

/// repair の一時の DB。途中で落ちても止める（Drop）。止めずに残すと、上がった利用者を DB の管理者として
/// 信じる一時の接続規則のまま動き続け、`postmaster.pid` で DB のサービスも起きない（review R52）。
struct TemporaryPostmaster<'a> {
    layout: &'a layout::Layout,
    active: bool,
}

impl TemporaryPostmaster<'_> {
    fn finish(mut self, trace: bool) -> Result<(), String> {
        self.active = false;
        run(&commands::repair_stop_command(self.layout), trace)
    }
}

impl Drop for TemporaryPostmaster<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = run(&commands::repair_stop_command(self.layout), false);
        }
    }
}

/// `initdb` の一時の合言葉のファイル。管理者と SYSTEM だけの置き場（`admin_only_staging_dir`）に作り、
/// 成否に依らず置き場ごと消す（本人とサーバのアカウントに読ませない。review R76）。
struct PwFile {
    directory: PathBuf,
    file: PathBuf,
}

impl PwFile {
    fn write(settings_dir: &Path, password: &str, trace: bool) -> Result<Self, String> {
        let directory = admin_only_staging_dir(
            settings_dir,
            INITDB_STAGING_PREFIX,
            "DB の一時の合言葉",
            trace,
        )?;
        let pwfile = Self {
            file: directory.join("initdb.pwfile"),
            directory,
        };
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pwfile.file)
            .and_then(|mut file| file.write_all(password.as_bytes()))
            .map_err(|error| format!("DB の一時の合言葉を書けない: {error}"))?;
        Ok(pwfile)
    }
}

impl Drop for PwFile {
    fn drop(&mut self) {
        let _ = db::remove_pwfile(&self.file);
        discard_directory(&self.directory);
    }
}

/// あるかどうか。読めない（別の管理者で、ACE を持たない）ものは「ある」と読む —— 見えないことを
/// 「無い」と読むと、走らせ直しを初めてのインストールとして扱ってしまう（review R35）。
fn present(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(_) => true,
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

/// ACL を付ける前に、その場所が本物のフォルダ（ジャンクションやリンクでない）であることを見る。
/// リンクだと、行き先の別の置き場に権限を付けてしまう。
fn ensure_real_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("{} を読めない: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "{} がフォルダでないか、別の場所へのリンクなので止めます",
            path.display()
        ));
    }
    Ok(())
}

/// 一時の書きを足す前に設定を読めるか試す（本人なら読める。別の管理者なら読めないので None）。
/// 段の判定の手がかりだけに使い、読めなくても止めない。
fn read_settings_unprivileged(path: &Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn read_settings(path: &Path) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| format!("設定を読めない: {error}"))?)
        .map_err(|error| format!("設定を読めない: {error}"))
}

fn write_settings(path: &Path, settings: &serde_json::Value) -> Result<(), String> {
    let temporary = path.with_extension("json.writing");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("設定を書けない: {error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("設定を置けない: {error}"))
}

/// `owner.json` の書きかけの置き場の名前の頭（`settings\.owner-staging-<印>`）。
const OWNER_STAGING_PREFIX: &str = ".owner-staging-";

/// `initdb` の一時の合言葉の置き場の名前の頭（`settings\\.initdb-staging-<印>`）。
const INITDB_STAGING_PREFIX: &str = ".initdb-staging-";

/// 設定の置き場の下に、管理者と SYSTEM だけが開ける新しいフォルダを作って返す。前の回の同じ頭の置き場は先に消す。
/// 設定の置き場はサーバのアカウントと本人が受け継ぐので、秘密をそこに直に作ると読まれる。新しいフォルダを
/// 作って継承を切り、空であることを確かめてから使う（中のファイルは作られたときから絞った形を受け継ぐ。
/// design D4。deep.md Q11・review R48・R76）。
fn admin_only_staging_dir(
    settings_dir: &Path,
    prefix: &str,
    label: &str,
    trace: bool,
) -> Result<PathBuf, String> {
    if let Ok(entries) = fs::read_dir(settings_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(prefix) {
                discard_directory(&entry.path());
            }
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let directory = settings_dir.join(format!("{prefix}{}-{stamp}", std::process::id()));
    fs::create_dir(&directory).map_err(|error| format!("{label}の置き場を作れない: {error}"))?;
    let prepared = (|| {
        run(
            &commands::admin_only_directory_acl_command(&directory),
            trace,
        )?;
        ensure_real_directory(&directory)?;
        let empty = fs::read_dir(&directory)
            .map_err(|error| format!("{label}の置き場を読めない: {error}"))?
            .next()
            .is_none();
        if !empty {
            return Err(format!(
                "{label}の置き場に知らないファイルがあるので止めます"
            ));
        }
        Ok(())
    })();
    if let Err(error) = prepared {
        discard_directory(&directory);
        return Err(error);
    }
    Ok(directory)
}

/// 表の所有者の合言葉（`owner.json`）の書きかけを作り、その場所を返す（置くのは `place_owner_settings`）。
/// 管理者と SYSTEM だけの書きかけの置き場（`admin_only_staging_dir`）の中に作る。
fn stage_owner_settings(
    settings_dir: &Path,
    settings: &serde_json::Value,
    trace: bool,
) -> Result<PathBuf, String> {
    let directory = admin_only_staging_dir(
        settings_dir,
        OWNER_STAGING_PREFIX,
        "所有者の設定の書きかけ",
        trace,
    )?;
    let staged = directory.join("owner.json");
    let written = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
            .map_err(|error| format!("所有者の設定を作れない: {error}"))?;
        file.write_all(&serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("所有者の設定を書けない: {error}"))
    })();
    if let Err(error) = written {
        discard_directory(&directory);
        return Err(error);
    }
    Ok(staged)
}

/// 書きかけの `owner.json` を本来の場所へ名前を替えて置き、明示の ACL と持ち主を付け、書きかけの置き場を消す。
fn place_owner_settings(staged: &Path, target: &Path, trace: bool) -> Result<(), String> {
    fs::rename(staged, target).map_err(|error| format!("所有者の設定を置けない: {error}"))?;
    run(&commands::owner_settings_acl_command(target), trace)?;
    run(&commands::owner_settings_owner_command(target), trace)?;
    if let Some(directory) = staged.parent() {
        fs::remove_dir(directory)
            .map_err(|error| format!("所有者の設定の書きかけの置き場を消せない: {error}"))?;
    }
    Ok(())
}

/// 作ったフォルダを消す。リンクに替わっていれば、行き先でなくリンクだけを消す（失敗しても止めない）。
fn discard_directory(directory: &Path) {
    if fs::symlink_metadata(directory).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        let _ = fs::remove_dir(directory).or_else(|_| fs::remove_file(directory));
    } else {
        let _ = fs::remove_dir_all(directory);
    }
}

/// 状態の置き場の ACL から本人の SID を読む（サービスのアカウントを除いた読みの主体が 1 つ）。
fn owner_from_state_acl(layout: &layout::Layout) -> Result<String, String> {
    // icacls は名前で表示するので、Get-Acl で主体を SID に直して読む
    let acl = SetupCommand::powershell(format!(
        "(Get-Acl -LiteralPath '{}').Access | ForEach-Object {{ $id = $_.IdentityReference; try {{ $id.Translate([System.Security.Principal.SecurityIdentifier]).Value }} catch {{ $id.Value }} }}",
        layout.state_dir
    ));
    let acl = Command::new(&acl.program)
        .args(&acl.args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("状態の置き場の ACL を読めない: {error}"))?;
    if !acl.status.success() {
        return Err("状態の置き場の ACL を読めない".into());
    }
    state::owner_sid_from_acl(&String::from_utf8_lossy(&acl.stdout))
        .map_err(|_| "本人の SID を ACL から 1 つに決められない".to_owned())
}

/// プログラムの置き場へ、zip を展開した置き場から写す（D29（仮））。無いファイルだけを、一時の名前に
/// 写してから名前を替えて置く（途中で切れても半端なファイルを本来の名前で残さない。review R38）。
/// 既にあるファイルが zip のものと違えば、別の版が入っているので何も替えずに止まる（版を替える口は
/// まだ無い。古い実行ファイルのまま新しい版の記録を書かない。D34（仮）・review R50）。
fn copy_program_files(layout: &layout::Layout, trace: bool) -> Result<(), String> {
    let source = std::env::current_exe()
        .map_err(|error| format!("インストーラの場所を読めない: {error}"))?
        .parent()
        .map(Path::to_path_buf)
        .ok_or("インストーラの置き場を読めない")?;
    let target = PathBuf::from(layout.program_dir);
    if source.canonicalize().ok() == target.canonicalize().ok() {
        return Ok(());
    }
    let mut missing = Vec::new();
    for name in commands::program_files() {
        program_file_plan(&source.join(name), &target.join(name), &mut missing).map_err(
            |error| match error {
                CopyPlanError::Io(error) => {
                    format!("プログラムの置き場の {name} を読めない: {error}")
                }
                CopyPlanError::Differs(path) => {
                    commands::program_version_mismatch_message(&path.display().to_string())
                }
            },
        )?;
    }
    for (from, to) in &missing {
        copy_atomically(from, to).map_err(|error| {
            format!("プログラムの置き場へ {} を写せない: {error}", to.display())
        })?;
    }
    if trace {
        terminal_line(
            &format!("result=ok copy-program-files {}", target.display()),
            false,
        );
    }
    Ok(())
}

enum CopyPlanError {
    Io(std::io::Error),
    Differs(PathBuf),
}

/// 写すものを決める。無いファイルは `missing` に足し、あるファイルは中身が同じことを確かめる。
fn program_file_plan(
    from: &Path,
    to: &Path,
    missing: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), CopyPlanError> {
    if from.is_dir() {
        for entry in fs::read_dir(from).map_err(CopyPlanError::Io)? {
            let entry = entry.map_err(CopyPlanError::Io)?;
            program_file_plan(&entry.path(), &to.join(entry.file_name()), missing)?;
        }
        return Ok(());
    }
    if !to.exists() {
        missing.push((from.to_path_buf(), to.to_path_buf()));
        return Ok(());
    }
    if same_contents(from, to).map_err(CopyPlanError::Io)? {
        Ok(())
    } else {
        Err(CopyPlanError::Differs(to.to_path_buf()))
    }
}

fn same_contents(left: &Path, right: &Path) -> std::io::Result<bool> {
    use std::io::Read as _;
    if fs::metadata(left)?.len() != fs::metadata(right)?.len() {
        return Ok(false);
    }
    let (mut left, mut right) = (fs::File::open(left)?, fs::File::open(right)?);
    let (mut a, mut b) = (vec![0_u8; 1 << 16], vec![0_u8; 1 << 16]);
    loop {
        let read = left.read(&mut a)?;
        if read == 0 {
            return Ok(true);
        }
        right.read_exact(&mut b[..read])?;
        if a[..read] != b[..read] {
            return Ok(false);
        }
    }
}

/// `<名前>.copying` に写してから本来の名前へ替える。前の回の `.copying` は上書きする。
fn copy_atomically(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut partial = to.as_os_str().to_owned();
    partial.push(".copying");
    let partial = PathBuf::from(partial);
    fs::copy(from, &partial)?;
    fs::rename(&partial, to)
}

/// DB の役割を作る（D7）。DB の管理者の役割 `ashiato`（D1）で入り、合言葉は環境変数で渡す（review R8）。
fn run_psql(
    layout: &layout::Layout,
    generated: &secrets::InstallSecrets,
    port: u16,
    trace: bool,
) -> Result<(), String> {
    // DB の管理者の合言葉は、入るときの値（initdb の値）と別の新しい乱数に置き直し、どこにも控えない。
    // initdb の値が漏れても、役割を作った後は使えない（以後の管理は repair の一時の sspi で行う。review R76）
    let admin_after = secrets::new_admin_password();
    for (database, script) in [
        ("postgres", commands::create_database_sql().as_bytes()),
        (
            "ashiato",
            include_bytes!("../../../tools/db-roles.sql").as_slice(),
        ),
    ] {
        let command = commands::psql_command(layout, "127.0.0.1", port, database);
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .env("PGPASSWORD", &generated.admin_password)
            .env("ADMIN_PW", &admin_after)
            .env("OWNER_PW", &generated.owner_password)
            .env("APP_PW", &generated.app_password)
            .env_remove("PGPASSFILE")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("同梱 psql を起こせない: {error}"))?;
        child
            .stdin
            .take()
            .ok_or_else(|| "psql の入力を開けない".to_owned())?
            .write_all(script)
            .map_err(|error| format!("DB 役割 SQL を渡せない: {error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("psql が終わらない: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "DB の役割を作れない ({database}): {}",
                output_tail(&output)
            ));
        }
        if trace {
            terminal_line(&format!("result=ok {} <stdin>", command.line()), false);
        }
    }
    Ok(())
}

fn run_migrate(
    layout: &layout::Layout,
    generated: &secrets::InstallSecrets,
    trace: bool,
) -> Result<(), String> {
    let settings = generated.settings_json(layout, "", "", "");
    let command = commands::migrate_command(
        &commands::server_executable(layout),
        &generated.owner_url(layout),
        Path::new(settings["MIGRATE_BACKUP_DIR"].as_str().unwrap_or_default()),
        Path::new(settings["PG_DUMP"].as_str().unwrap_or_default()),
    );
    let output = Command::new(&command.program)
        .args(&command.args)
        .envs(command.env.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("ashiato-server migrate を起こせない: {error}"))?;
    if !output.status.success() {
        // migrate が端末に出すのは種別・版の名前・SQLSTATE の 1 行だけ（秘密を持たない）
        return Err(format!("移行に失敗した: {}", output_tail(&output)));
    }
    if trace {
        terminal_line(
            &format!(
                "result=ok {} migrate (env {})",
                command.program,
                command
                    .env
                    .iter()
                    .map(|(key, _)| key.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            false,
        );
    }
    Ok(())
}

/// インストールの段（D10・D29（仮））。各段は「既にあるか」を見て、あれば触らない。
fn install(
    layout: &layout::Layout,
    identity: &Identity,
    profile: &str,
    downloads: &str,
    trace: bool,
) -> Result<(), String> {
    ensure_ports_available(layout)?;
    let state_root = PathBuf::from(layout.state_dir);
    let settings_dir = state_root.join("settings");
    let settings_path = commands::settings_path(layout);
    let owner_settings_path = commands::owner_settings_path(layout);
    let db_dir = PathBuf::from(layout.db_dir);
    let database_exists = present(&db_dir.join("PG_VERSION"));
    let db_service_exists = service_exists(layout.db_service);
    let server_service_exists = service_exists(layout.server_service);
    // 設定は `server.json` と表の所有者の `owner.json` の 2 つが揃って「ある」（片方だけなら秘密が欠けている。D4）
    let settings_exists = present(&settings_path) && present(&owner_settings_path);
    // DB があるのに設定が無ければ、何も書かずに止まる（(2) より前に判定する。D10）
    let verifier_present = settings_exists
        && read_settings_unprivileged(&settings_path).is_some_and(|settings| {
            !settings["WEB_PASSWORD_VERIFIER"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
        });
    let decision = state::decide_install(state::InstallState {
        settings: settings_exists,
        database: database_exists,
        verifier: verifier_present,
        services: db_service_exists && server_service_exists,
    });
    if decision == state::InstallDecision::RepairRequired {
        return Err(state::repair_message(&state_root));
    }
    if trace {
        terminal_line(&format!("phase={decision:?}"), false);
    }

    // (1) 置き場と ACL。初めて作るときだけ、中の置き場を作ってから継承を切り、2 つの主体だけにする
    // （存在しない置き場に icacls をかけない・サービスが書いたファイルができた後は ACL を変えない。review R4）
    let fresh = !settings_exists;
    if fresh {
        for directory in [
            settings_dir.clone(),
            state_root.join("db"),
            state_root.join("backups").join("pre-migrate"),
            state_root.join("logs").join("db"),
            state_root.join("logs").join("server"),
        ] {
            fs::create_dir_all(&directory)
                .map_err(|error| format!("状態の置き場を作れない: {error}"))?;
        }
        for directory in [
            state_root.clone(),
            settings_dir.clone(),
            state_root.join("db"),
            state_root.join("backups"),
            state_root.join("backups").join("pre-migrate"),
            state_root.join("logs"),
            state_root.join("logs").join("db"),
            state_root.join("logs").join("server"),
        ] {
            ensure_real_directory(&directory)?;
        }
        // 先に誰かが作っていた置き場でも、持ち主を Administrators にしてから ACL を付ける
        // （持ち主は ACL を書き換えられるので、ほかの利用者を持ち主のままにしない）
        run(&commands::take_ownership_command(layout), trace)?;
        // 未登録のサービス SID は `icacls` が名前解決できない。初回は先にサービスを登録してから
        // SID の ACE を付ける。片方だけ残った異常な状態では、登録済みのものを作り直さない。
        if test_hook!("REGISTER_FAIL").as_deref() == Some("1") {
            return Err("サービス専用アカウントの登録に失敗したため中止".into());
        }
        copy_program_files(layout, trace)?;
        if !db_service_exists && !server_service_exists {
            run_all(
                &commands::fresh_state_setup_commands(layout, &identity.owner),
                trace,
            )?;
            grant_db_watch(layout, trace)?;
        } else {
            if !db_service_exists {
                run(&commands::register_database_command(layout), trace)?;
            }
            if !server_service_exists {
                run(&commands::create_server_command(layout), trace)?;
            }
            configure_services(layout, trace)?;
            run_all(&commands::acl_commands(layout, &identity.owner), trace)?;
        }
    }
    // その後で、書く段の間の一時の書きを足す（ACL を絞った後に足すので消えない。D21）
    let temporary = TemporaryWrite::grant(layout, &identity.invoking, trace)?;
    let existing = if settings_exists {
        Some((
            read_settings(&settings_path)?,
            read_settings(&owner_settings_path)?,
        ))
    } else {
        None
    };
    // 走らせ直しでは、設定に控えた本人を使う（別の利用者が走らせ直しても本人は変わらない。review R16）
    let owner = state::owner_sid_for_rerun(
        existing
            .as_ref()
            .and_then(|(settings, _)| settings["ASHIATO_OWNER_SID"].as_str()),
        &identity.owner,
    )
    .to_owned();

    // 前の版が設定の置き場に直に残した一時の合言葉のファイルを消す（中身は使わない）。
    // 今の版の置き場（`.initdb-staging-*`）は `PwFile::write` が作る前に消す
    db::remove_pwfile(&settings_dir.join(".initdb.pwfile"))
        .map_err(|error| format!("DB の一時の合言葉を消せない: {error}"))?;

    // (2) 秘密（ACL を絞った後に書く。review R13）
    let generated = match &existing {
        Some((settings, owner_settings)) => secrets::InstallSecrets::from_settings(
            settings,
            owner_settings,
            secrets::new_install_secrets().admin_password,
        )?,
        None => secrets::new_install_secrets(),
    };
    if existing.is_none() {
        // 所有者の合言葉を先に置く（`server.json` が「設定がある」の印なので、後に置く）。書く間はサーバの
        // サービスを動かさない（片方だけの設定で起きていたサーバが、書いている途中のファイルに触れないように）
        run(
            &commands::stop_service_and_wait_command(layout.server_service),
            trace,
        )?;
        let staged =
            stage_owner_settings(&settings_dir, &generated.owner_settings_json(layout), trace)?;
        place_owner_settings(&staged, &owner_settings_path, trace)?;
        write_settings(
            &settings_path,
            &generated.settings_json(layout, profile, downloads, &owner),
        )?;
    }

    // (3)(4) DB と DB の設定。役割を作る前に落ちた DB は、管理者の合言葉がもう無い（一時のファイルは
    // 成否に依らず消す）。作り直すのは、このインストーラが initdb を始めた印（`.installer-initdb-started`）が
    // あり、役割の完了記録が無い DB だけ（役割が無ければサーバは繋げず、記録は入らない）。印の無い DB は
    // 記録が入っているかもしれないので消さずに止まる（D32（仮）。review R40）
    let roles_marker = settings_dir.join(".installer-roles-complete");
    let migration_marker = settings_dir.join(".installer-migration-complete");
    let initdb_marker = settings_dir.join(".installer-initdb-started");
    let roles_done = roles_marker.exists();
    // 印は設定の置き場（サーバのアカウントも書ける）にあるので、印だけでは消さない。DB のサービスが無く、
    // 移行の完了記録も無いとき（このインストーラが initdb から役割の前までの間に落ちたとき）だけ作り直す
    let recreatable = initdb_marker.exists() && !db_service_exists && !migration_marker.exists();
    if database_exists && !roles_done && !recreatable {
        return Err(state::roles_missing_message(&db_dir));
    }
    let init_database = !database_exists || !roles_done;
    if init_database {
        let database_write = DatabaseWrite::grant(layout, &identity.invoking, &owner, trace)?;
        fs::write(&initdb_marker, b"started")
            .map_err(|error| format!("initdb を始めた印を書けない: {error}"))?;
        if db_dir.exists() {
            fs::remove_dir_all(&db_dir)
                .map_err(|error| format!("作りかけの DB の置き場を消せない: {error}"))?;
        }
        db::remove_pwfile(&migration_marker)
            .map_err(|error| format!("移行の完了記録を消せない: {error}"))?;
        if test_hook!("STOP_AT").as_deref() == Some("initdb") {
            fs::create_dir_all(&db_dir).map_err(|error| error.to_string())?;
            fs::write(db_dir.join("initdb-interrupted"), b"").map_err(|error| error.to_string())?;
            return Err("initdb の段で停止しました".into());
        }
        let pwfile = PwFile::write(&settings_dir, &generated.admin_password, trace)?;
        run(&commands::initdb_command(layout, &pwfile.file), trace)?;
        drop(pwfile);
        // (4) initdb の後・DB の起動の前に postgresql.conf の末尾へ足す（D6。review R7）
        let conf = db_dir.join("postgresql.conf");
        let current =
            fs::read_to_string(&conf).map_err(|error| format!("DB の設定を読めない: {error}"))?;
        if let Some(updated) = db::with_distribution_settings(&current, &state_root, layout.db_port)
        {
            fs::write(&conf, updated).map_err(|error| format!("DB の設定を書けない: {error}"))?;
            if trace {
                terminal_line("result=ok append postgresql.conf", false);
            }
        }
        database_write.finish(trace)?;
    }

    // (5) 既存の状態では、プログラムの置き場への写しとサービスの登録をここで行う。
    // 初回は SID の ACL より前に済ませている。
    if !fresh {
        copy_program_files(layout, trace)?;
        if !db_service_exists {
            run(&commands::register_database_command(layout), trace)?;
        }
        if !server_service_exists {
            run(&commands::create_server_command(layout), trace)?;
        }
        configure_services(layout, trace)?;
    }

    // (6) DB の開始。待ち受けが開いても回復の途中は接続を断るので、pg_isready が受け付けるまで待つ
    ensure_running(layout.db_service, trace)?;
    if !wait_for_database(layout, layout.db_port, Duration::from_secs(120)) {
        return Err(format!(
            "DB が起きない。ログ: {}",
            state_root.join("logs").join("db").display()
        ));
    }

    // (7) 役割
    if init_database {
        run_psql(layout, &generated, layout.db_port, trace)?;
        fs::write(&roles_marker, b"ok")
            .map_err(|error| format!("役割の完了記録を書けない: {error}"))?;
        db::remove_pwfile(&initdb_marker)
            .map_err(|error| format!("initdb を始めた印を消せない: {error}"))?;
    }

    // (8) 移行（失敗したらサーバを開始しない）
    if test_hook!("MIGRATE_FAIL").as_deref() == Some("1") {
        return Err("移行に失敗した: 試験の差し込み".into());
    }
    if !migration_marker.exists() {
        run_migrate(layout, &generated, trace)?;
        fs::write(&migration_marker, b"ok")
            .map_err(|error| format!("移行の完了記録を書けない: {error}"))?;
    }

    // (9) 取り込み待ちとダウンロードの権限
    fs::create_dir_all(commands::inbox_dir(profile))
        .map_err(|error| format!("取り込み待ちを作れない: {error}"))?;
    for directory in [
        commands::inbox_dir(profile),
        commands::inbox_parent(profile),
        downloads.to_owned(),
    ] {
        ensure_real_directory(Path::new(&directory))?;
    }
    run_all(
        &commands::inbox_acl_commands(layout, profile, downloads),
        trace,
    )?;

    // (11) 画面の合言葉（照らすための値が無ければ）。サーバは照らすための値が無いと起動しないので、
    // (10) より前に決める
    let settings = read_settings(&settings_path)?;
    if settings["WEB_PASSWORD_VERIFIER"]
        .as_str()
        .unwrap_or_default()
        .is_empty()
    {
        let verifier = read_web_passwords(&generated.api_token, &generated.web_session_key)?;
        write_settings(
            &settings_path,
            &secrets::replace_web_password_verifier(&settings, &verifier),
        )?;
    }
    temporary.finish(trace)?;

    // (10) サーバの開始（起こし直し）
    run(
        &commands::stop_service_and_wait_command(layout.server_service),
        trace,
    )?;
    ensure_running(layout.server_service, trace)?;
    if !wait_for_port(layout.server_port, Duration::from_secs(180)) {
        return Err(format!(
            "サーバが起きない。ログ: {}",
            state_root.join("logs").join("server").display()
        ));
    }

    // (12) 「アプリと機能」
    run_all(
        &commands::uninstall_registration_commands(layout, env!("ASHIATO_VERSION")),
        trace,
    )?;

    // (13) 案内
    open_browser(layout, trace);
    terminal_line(
        &commands::install_notice(layout.server_port, free_space_gb()),
        false,
    );
    Ok(())
}

fn uninstall(layout: &layout::Layout, trace: bool) -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("インストーラの場所を読めない: {error}"))?;
    if std::env::var_os("ASHIATO_UNINSTALL_COPY").is_none() {
        // 写しと遅れて消す台本は、管理者と SYSTEM だけが書ける置き場に置く。本人の一時フォルダに置くと、
        // 本人の権限で動くプログラムが写しか台本を差し替え、管理者の権限で動かせる（review R47）
        let copy = uninstall_copy_path(layout, trace)?;
        // 写しを作れない・起こせないときは、作った置き場を残さない（review N3）。起こせた後は、写しの
        // 遅れて消す台本が置き場ごと消す
        let discard = |error: String| {
            if let Some(directory) = copy.parent() {
                discard_directory(directory);
            }
            error
        };
        copy_new_file(&executable, &copy)
            .map_err(|error| discard(format!("アンインストール用の写しを作れない: {error}")))?;
        // 写しの終わりを待ち、その結果を返す（review R20）
        let status = Command::new(&copy)
            .arg("uninstall")
            .env("ASHIATO_UNINSTALL_COPY", "1")
            .status()
            .map_err(|error| discard(format!("アンインストール用の写しを起こせない: {error}")))?;
        return if status.success() {
            Ok(())
        } else {
            Err(format!(
                "アンインストールに失敗した (終了コード {:?})",
                status.code()
            ))
        };
    }
    let settings = if present(&commands::settings_path(layout)) {
        let temporary = TemporaryWrite::grant(layout, &current_user_sid()?, trace)?;
        let settings = read_settings(&commands::settings_path(layout)).ok();
        temporary.finish(trace)?;
        settings
    } else {
        None
    };
    let profile = settings
        .as_ref()
        .and_then(|value| value["ASHIATO_INBOX_DIR"].as_str())
        .map(|value| value.replace('/', "\\"))
        .and_then(|value| {
            value
                .strip_suffix(r"\ashiato\取り込み待ち")
                .map(ToOwned::to_owned)
        });
    let downloads = settings
        .as_ref()
        .and_then(|value| value["ASHIATO_DOWNLOADS_DIR"].as_str())
        .map(|value| value.replace('/', "\\"));
    // 遅れて消す台本は、管理者だけが書ける写しの置き場に新しいファイルとして書く（先に置かれたものを使わない）
    {
        use std::io::Write as _;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(executable.with_extension("cmd"))
            .and_then(|mut file| {
                file.write_all(commands::delayed_removal_script(layout, &executable).as_bytes())
            })
            .map_err(|error| format!("遅れて消す台本を書けない: {error}"))?;
    }
    for command in commands::uninstall_commands(layout, &executable) {
        // 無いサービスは消さない（走らせ直しの消す口が止まらないように）
        let missing_service = command.program == "sc.exe"
            && command.args.first().is_some_and(|arg| arg == "delete")
            && command
                .args
                .get(1)
                .is_some_and(|name| !service_exists(name));
        if missing_service {
            continue;
        }
        if command.program == "reg.exe" && !uninstall_key_exists() {
            continue;
        }
        run(&command, trace)?;
    }
    if let (Some(profile), Some(downloads)) = (profile, downloads) {
        for command in commands::uninstall_acl_commands(layout, &profile, &downloads) {
            if command
                .args
                .first()
                .is_some_and(|path| Path::new(path).exists())
            {
                run(&command, trace)?;
            }
        }
    }
    terminal_line(
        &format!(
            "状態の置き場は {} に残しました。記録を手で消す手順: docs/install.md",
            layout.state_dir
        ),
        false,
    );
    Ok(())
}

/// 消す口の写しの置き場。新しいフォルダを作り（既にあれば作らない）、継承を切って管理者と SYSTEM だけにし、
/// 空であることを確かめてから写しの場所を返す（作ってから ACL を絞るまでの間に置かれたものを使わない）。
fn uninstall_copy_path(layout: &layout::Layout, trace: bool) -> Result<PathBuf, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let directory = PathBuf::from(commands::uninstall_copy_dir(
        layout,
        &format!("{}-{stamp}", std::process::id()),
    ));
    fs::create_dir(&directory)
        .map_err(|error| format!("アンインストール用の置き場を作れない: {error}"))?;
    let prepared = (|| {
        run(
            &commands::admin_only_directory_acl_command(&directory),
            trace,
        )?;
        ensure_real_directory(&directory)?;
        let empty = fs::read_dir(&directory)
            .map_err(|error| format!("アンインストール用の置き場を読めない: {error}"))?
            .next()
            .is_none();
        if !empty {
            return Err(
                "アンインストール用の置き場に知らないファイルがあるので止めます".to_owned(),
            );
        }
        Ok(())
    })();
    if let Err(error) = prepared {
        // 作った置き場を残さない（review N3）
        discard_directory(&directory);
        return Err(error);
    }
    Ok(directory.join("ashiato-setup-uninstall.exe"))
}

/// 新しいファイルとして写す（既にあれば失敗する。ほかの者が先に置いたファイルを上書きして使わない）。
fn copy_new_file(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut source = fs::File::open(from)?;
    let mut target = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)?;
    std::io::copy(&mut source, &mut target).map(|_| ())
}

fn uninstall_key_exists() -> bool {
    Command::new("reg.exe")
        .args([
            "query",
            r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn password(layout: &layout::Layout, trace: bool) -> Result<(), String> {
    let settings_path = commands::settings_path(layout);
    let temporary = TemporaryWrite::grant(layout, &current_user_sid()?, trace)?;
    let settings = read_settings(&settings_path)?;
    let verifier = read_web_passwords(
        settings["API_TOKEN"].as_str().unwrap_or_default(),
        settings["WEB_SESSION_KEY"].as_str().unwrap_or_default(),
    )?;
    run(
        &commands::stop_service_and_wait_command(layout.server_service),
        trace,
    )?;
    write_settings(
        &settings_path,
        &secrets::replace_web_password_verifier(&settings, &verifier),
    )?;
    temporary.finish(trace)?;
    ensure_running(layout.server_service, trace)?;
    if !wait_for_port(layout.server_port, Duration::from_secs(180)) {
        return Err("決め直した後にサーバが起きない".into());
    }
    if trace {
        terminal_line("result=ok restart ashiato-server", false);
    }
    Ok(())
}

fn repair(
    layout: &layout::Layout,
    profile: &str,
    downloads: &str,
    trace: bool,
) -> Result<(), String> {
    let state_root = PathBuf::from(layout.state_dir);
    let settings = commands::settings_path(layout);
    let owner_settings = commands::owner_settings_path(layout);
    if !state::repair_allowed(
        present(&Path::new(layout.db_dir).join("PG_VERSION")),
        present(&settings) && present(&owner_settings),
        present(
            &state_root
                .join("settings")
                .join(".installer-roles-complete"),
        ),
    ) {
        return Err(
            "repair は要りません。インストールを走らせ直すか password を使ってください。".into(),
        );
    }
    run(
        &commands::stop_service_and_wait_command(layout.server_service),
        trace,
    )?;
    run(
        &commands::stop_service_and_wait_command(layout.db_service),
        trace,
    )?;
    run_all(&commands::repair_cleanup_commands(layout), trace)?;
    let owner = owner_from_state_acl(layout)?;
    let invoking = current_user_sid()?;
    let account = whoami(&[])?.trim().to_owned();
    if account.is_empty() {
        return Err("実行した利用者を読めない".into());
    }
    // 今回の管理者に、設定と DB の置き場の一時の書きを足す（前の回の残りは `/reset` で消える。D21・D28）
    let temporary = TemporaryWrite::grant(layout, &invoking, trace)?;
    let database_write = DatabaseWrite::grant(layout, &invoking, &owner, trace)?;
    let hba = state_root.join("settings").join("repair.pg_hba.conf");
    let ident = state_root.join("settings").join("repair.pg_ident.conf");
    let _ = fs::remove_file(&hba);
    let _ = fs::remove_file(&ident);
    let repair_log = state_root.join("settings").join("repair-db.log");
    // 前の回の残り（リンクを含む）を消してから pg_ctl に作らせる。消せなければ止まる
    match fs::remove_file(&repair_log) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("repair の一時のログを消せない: {error}"));
        }
        _ => {}
    }
    let generated = secrets::new_install_secrets();
    let repair_settings = state_root.join("settings").join("server.json.repair");
    fs::write(
        &repair_settings,
        serde_json::to_vec_pretty(&generated.settings_json(layout, profile, downloads, &owner))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("repair の設定を書けない: {error}"))?;
    let repair_owner_settings = stage_owner_settings(
        &state_root.join("settings"),
        &generated.owner_settings_json(layout),
        trace,
    )?;
    fs::write(&hba, commands::repair_hba_contents())
        .map_err(|error| format!("一時の接続規則を書けない: {error}"))?;
    fs::write(&ident, commands::repair_ident_contents(&account))
        .map_err(|error| format!("一時の利用者規則を書けない: {error}"))?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("一時の番号を確保できない: {error}"))?;
    let temporary_port = listener
        .local_addr()
        .map_err(|error| format!("一時の番号を読めない: {error}"))?
        .port();
    drop(listener);
    run_detached(
        &commands::repair_start_command(layout, &hba, &ident, &repair_log, temporary_port),
        trace,
    )?;
    // ここから先で落ちても、一時の DB（上がった利用者を信じる接続規則で動く）を残さない（review R52）
    let temporary_postmaster = TemporaryPostmaster {
        layout,
        active: true,
    };
    run_psql(layout, &generated, temporary_port, trace)?;
    if test_hook!("STOP_AT").as_deref() == Some("repair-roles") {
        return Err("repair の役割の段で停止しました".into());
    }
    temporary_postmaster.finish(trace)?;
    database_write.finish(trace)?;
    fs::remove_file(&hba).map_err(|error| format!("一時の接続規則を消せない: {error}"))?;
    fs::remove_file(&ident).map_err(|error| format!("一時の利用者規則を消せない: {error}"))?;
    let _ = fs::remove_file(&repair_log);
    // 役割を作り直したので、走らせ直しでは役割を再実行せず、通常の (5)〜(13) を通す（D27）。完了記録を先に書き、
    // 設定を最後に置く —— 設定を置いた後・記録の前に落ちると、走らせ直しが記録の入った DB を「役割の前の DB」と
    // 読む（review R40）。移行の完了記録は書かない（あればそのまま、無ければ走らせ直しが移行を当てる）。
    // 役割の前に落ちた DB（R40 の出口）は移行していないので、書くと表の無い DB でサーバを起こす（review N2）
    fs::write(
        state_root
            .join("settings")
            .join(".installer-roles-complete"),
        b"ok",
    )
    .map_err(|error| format!(".installer-roles-complete を書けない: {error}"))?;
    db::remove_pwfile(
        &state_root
            .join("settings")
            .join(".installer-initdb-started"),
    )
    .map_err(|error| format!("initdb を始めた印を消せない: {error}"))?;
    // 所有者の設定を先に置き、`server.json`（「設定がある」の印）を最後に置く
    place_owner_settings(&repair_owner_settings, &owner_settings, trace)?;
    fs::rename(&repair_settings, &settings)
        .map_err(|error| format!("repair の設定を置けない: {error}"))?;
    temporary.finish(trace)?;
    install(
        layout,
        &Identity {
            owner: owner.clone(),
            invoking,
        },
        profile,
        downloads,
        trace,
    )?;
    terminal_line(state::repair_completion_message(), false);
    Ok(())
}

fn main() {
    std::hint::black_box(BUILD_STAMP);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let command = match commands::parse_args(&refs) {
        Ok(command) => command,
        Err(error) => {
            terminal_line(&error, true);
            std::process::exit(2);
        }
    };
    let elevated_child = refs.contains(&"--elevated");
    let result = run_command(&command, &refs);
    if let Err(error) = &result {
        terminal_line(error, true);
    }
    // 起こし直した窓は終わると閉じるので、結果を読めるように Enter を待つ（D22）
    if elevated_child && std::io::stdin().is_terminal() {
        terminal_line("Enter で閉じる", false);
        let _ = std::io::stdin().lock().lines().next();
    }
    if result.is_err() {
        std::process::exit(1);
    }
}

fn run_command(command: &str, refs: &[&str]) -> Result<(), String> {
    let version = env!("ASHIATO_VERSION");
    let selected = layout::for_version(version);
    let profile = commands::arg_value(refs, "--profile")
        .map(ToOwned::to_owned)
        .or_else(|| std::env::var("USERPROFILE").ok())
        .ok_or("本人のプロファイルの場所を読めない")?;
    let downloads = commands::arg_value(refs, "--downloads")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| user_downloads(&profile));
    let downloads = test_hook!("DOWNLOADS_DIR").unwrap_or(downloads);
    let trace = test_hook!("TRACE").as_deref() == Some("1");
    if trace {
        let trace_command = if command == "password" {
            "credential-update"
        } else {
            command
        };
        terminal_line(&format!("command={trace_command}"), false);
    }
    let executable = std::env::current_exe()
        .map_err(|error| format!("インストーラの場所を読めない: {error}"))?
        .display()
        .to_string();
    // 上がる前に本人を控え、自分を管理者として 1 回だけ起こし直す（D22。review R3）
    let invoking = current_user_sid()?;
    // 試験の差し込み `ELEVATION=deny` は、断られたときと同じく `Start-Process` が失敗する式で、
    // 本物の起こし直しの道（relaunch_elevated と式の失敗の扱い）を通す（review R37）
    if test_hook!("ELEVATION").as_deref() == Some("deny") {
        return relaunch_elevated(&commands::denied_elevation_script(&[command, "--elevated"]));
    }
    if commands::elevation_count(is_elevated()) == 1 {
        let script = match command {
            "install" => commands::elevation_command(&executable, &invoking, &profile, &downloads),
            "password" => commands::password_elevation_command(&executable),
            "repair" => commands::repair_elevation_command(&executable, &profile, &downloads),
            _ => commands::uninstall_elevation_command(&executable),
        };
        return relaunch_elevated(&script);
    }
    match command {
        "repair" => repair(&selected, &profile, &downloads, trace),
        "install" => install(
            &selected,
            &Identity {
                owner: commands::arg_value(refs, "--owner-sid")
                    .map_or_else(|| invoking.clone(), ToOwned::to_owned),
                invoking,
            },
            &profile,
            &downloads,
            trace,
        ),
        "password" => password(&selected, trace),
        "uninstall" => uninstall(&selected, trace),
        _ => Err(format!("{command} の口はありません")),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        expand_downloads_shell_folder, reg_query_value, run_detached, Duration, Instant,
        SetupCommand,
    };

    #[test]
    fn st55_run_does_not_wait_for_a_descendant_holding_its_output_handles() {
        // Scenario: 設定を退けて止まった後に repair すると、記録を残したままログインできる
        // pg_ctl は PostgreSQL を子として起こす。子が標準出力・標準エラーを継承しても、親の終了だけを待つ。
        let command = if cfg!(windows) {
            SetupCommand::new(
                "cmd.exe",
                [
                    "/C",
                    "start \"\" /B powershell.exe -NoProfile -NonInteractive -Command \"Start-Sleep -Seconds 2\"",
                ],
            )
        } else {
            SetupCommand::new("sh", ["-c", "sleep 2 &"])
        };
        let started = Instant::now();
        run_detached(&command, false).expect("親の command は子を待たずに終わる");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "子孫が標準ハンドルを持つため親の終了を待った"
        );
    }

    #[test]
    fn st55_expands_userprofile_downloads_value() {
        assert_eq!(
            expand_downloads_shell_folder(r"%USERPROFILE%\Downloads", r"C:\Users\A"),
            r"C:\Users\A\Downloads"
        );
    }

    #[test]
    fn st55_expands_onedrive_downloads_without_changing_inbox() {
        assert_eq!(
            expand_downloads_shell_folder(r"%USERPROFILE%\OneDrive\Downloads", r"C:\Users\A"),
            r"C:\Users\A\OneDrive\Downloads"
        );
    }

    #[test]
    fn st55_inbox_reads_downloads_value_with_spaces() {
        // 空白を含むダウンロードの場所を途中で切らない（review R24）
        let output = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\User Shell Folders\r\n    {374DE290-123F-4565-9164-39C4925E467B}    REG_EXPAND_SZ    D:\\My Files\\Downloads\r\n";
        assert_eq!(
            reg_query_value(output, "{374DE290-123F-4565-9164-39C4925E467B}").as_deref(),
            Some(r"D:\My Files\Downloads")
        );
        let plain =
            "    {374DE290-123F-4565-9164-39C4925E467B}    REG_SZ    %USERPROFILE%\\Downloads";
        assert_eq!(
            reg_query_value(plain, "{374DE290-123F-4565-9164-39C4925E467B}").as_deref(),
            Some(r"%USERPROFILE%\Downloads")
        );
        assert_eq!(reg_query_value("nothing", "{374DE290}"), None);
    }
}
