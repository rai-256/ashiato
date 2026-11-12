use crate::layout::Layout;
use std::path::{Path, PathBuf};

pub const RELEASE_DB_PORT: u16 = 55400;
pub const RELEASE_SERVER_PORT: u16 = 18700;

/// インストーラが起こす 1 つの命令。PowerShell を通さず、引数をそのまま渡す
/// （引用の崩れを避ける。Rust の `Command` が Windows の引数の規則で引用する）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl SetupCommand {
    pub fn new<I, A>(program: impl Into<String>, args: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: Into<String>,
    {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    /// PowerShell の式を 1 つ走らせる命令。`Get-Service` などの判定が要るものだけに使う。
    pub fn powershell(script: impl Into<String>) -> Self {
        Self::new(
            "powershell.exe",
            [
                "-NoProfile".to_owned(),
                "-NonInteractive".to_owned(),
                "-Command".to_owned(),
                script.into(),
            ],
        )
    }

    /// 命令の記録（`ASHIATO_SETUP_TEST_TRACE=1`）と試験のための 1 行。空白を含む引数は `"` でくくる。
    /// 命令には秘密を載せない（秘密は環境変数か標準入力で渡す）ので、この行にも秘密は出ない。
    pub fn line(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(|part| {
                if part.is_empty() || part.chars().any(char::is_whitespace) {
                    format!("\"{part}\"")
                } else {
                    part.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortListener {
    pub address: String,
    pub pid: u32,
}

pub fn netstat_protocols() -> [&'static str; 2] {
    ["tcp", "tcpv6"]
}

/// `netstat -ano` の IPv4/IPv6 出力から、指定した待ち受け番号を読む。
/// 状態の語（`LISTENING`）は表示の言語で変わりうるので、相手の番号が 0 の行も待ち受けとみなす。
pub fn parse_netstat_listener(output: &str, port: u16) -> Option<PortListener> {
    output.lines().find_map(|line| {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 5 || !fields[0].eq_ignore_ascii_case("tcp") {
            return None;
        }
        let local = fields[1];
        let local_port: u16 = local
            .rsplit_once(':')?
            .1
            .trim_end_matches(']')
            .parse()
            .ok()?;
        let listening = fields[3].eq_ignore_ascii_case("listening") || fields[2].ends_with(":0");
        if local_port != port || !listening {
            return None;
        }
        Some(PortListener {
            address: local.to_owned(),
            pid: fields[4].parse().ok()?,
        })
    })
}

/// `sc.exe query <名前>` の出力から状態の番号（4 = RUNNING・1 = STOPPED）を読む。
/// 表示の語ではなく番号で見る（表示の言語に依らない）。
pub fn service_state_code(output: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        let (label, rest) = line.split_once(':')?;
        if !label.trim().eq_ignore_ascii_case("state") {
            return None;
        }
        rest.split_whitespace().next()?.parse().ok()
    })
}

pub const SERVICE_RUNNING: u32 = 4;

pub fn fixed_ports_are_outside_development_ranges() -> bool {
    [55400, 18700, 55401, 18701].iter().all(|port| {
        *port != 5432
            && *port != 55432
            && *port != 55499
            && !(55500..=55599).contains(port)
            && !(55600..=55700).contains(port)
            && *port != 18787
            && !(19000..=19199).contains(port)
            && !(19200..=19399).contains(port)
    })
}

pub fn install_notice(server_port: u16, free_gb: Option<u64>) -> String {
    install_notice_with_update_check(server_port, free_gb, true)
}

pub fn install_notice_with_update_check(
    server_port: u16,
    free_gb: Option<u64>,
    update_check_enabled: bool,
) -> String {
    let mut notice = format!(
        "無人で動かすには、Tailscale の設定で tailscale serve --bg --https=443 http://127.0.0.1:{server_port} を転送先にしてください。インストーラは Tailscale の設定を変更していません。"
    );
    if let Some(gb) = free_gb.filter(|gb| *gb < 40) {
        notice.push_str(&format!(
            " 状態の置き場の空きは {gb} GB で、見積もりは 40 GB です。"
        ));
    }
    let state = if update_check_enabled {
        "有効"
    } else {
        "無効"
    };
    notice.push_str(&format!(
        " 更新の確認: {state}（切るには ashiato-setup.exe update-check off）"
    ));
    notice
}

pub fn install_completion_steps(
    setup_updater: bool,
    update_check_enabled: bool,
) -> Vec<&'static str> {
    if !setup_updater {
        return vec!["notice"];
    }
    let mut steps = vec!["notice"];
    if update_check_enabled {
        steps.push("start-updater");
    }
    steps
}

/// 取り込み待ちの場所（本人のプロファイルの直下。D9）。
pub fn inbox_dir(profile: &str) -> String {
    format!(r"{}\取り込み待ち", inbox_parent(profile))
}

pub fn inbox_parent(profile: &str) -> String {
    format!(r"{}\ashiato", profile.trim_end_matches(['/', '\\']))
}

/// `icacls` の命令。`/L` を必ず付け、リンクの行き先ではなくリンク自体を扱わせる。
fn icacls<I, A>(args: I) -> SetupCommand
where
    I: IntoIterator<Item = A>,
    A: Into<String>,
{
    let mut command = SetupCommand::new("icacls", args);
    command.args.push("/L".into());
    command
}

fn service_account(service: &str) -> String {
    format!(r"NT SERVICE\{service}")
}

/// サービスの仮想アカウントの SID（`S-1-5-80-…`）。名前の大文字を UTF-16LE にした SHA-1 の 20 バイトを、
/// 4 バイトずつ little-endian の数にして並べる（`sc.exe showsid` と同じ値）。サービスを登録する前でも
/// 決まるので、登録の前の ACL に使う（LSA は登録されていないサービスの名前を SID に引けない。review R31）。
pub fn service_sid(service: &str) -> String {
    use sha1::{Digest as _, Sha1};
    let bytes = service
        .to_uppercase()
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let digest = Sha1::digest(&bytes);
    let parts = digest
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| u32::from_le_bytes(*chunk).to_string())
        .collect::<Vec<_>>();
    format!("S-1-5-80-{}", parts.join("-"))
}

/// icacls に渡すサービスの主体（SID の形。登録の前後で同じ）。
fn service_principal(service: &str) -> String {
    format!("*{}", service_sid(service))
}

/// 消す口の写しの置き場（`C:\ProgramData\ashiato-uninstall-<印>`）。状態の置き場の親（管理者が作った
/// フォルダを、ほかの利用者が消したり名前を替えたりできない場所）に、回ごとに新しく作る（review R47）。
pub fn uninstall_copy_dir(layout: &Layout, unique: &str) -> String {
    let parent = layout
        .state_dir
        .rsplit_once('\\')
        .map_or(layout.state_dir, |(parent, _)| parent);
    format!(r"{parent}\ashiato-uninstall-{unique}")
}

/// 新しく作ったフォルダ（消す口の写しの置き場・`owner.json` の書きかけの置き場）の継承を切り、
/// 管理者（`S-1-5-32-544`）と SYSTEM（`S-1-5-18`）だけにする。中に作るファイルはこの形を受け継ぐ。
pub fn admin_only_directory_acl_command(directory: &Path) -> SetupCommand {
    icacls([
        directory.display().to_string(),
        "/inheritance:r".into(),
        "/grant:r".into(),
        "*S-1-5-32-544:(OI)(CI)F".into(),
        "*S-1-5-18:(OI)(CI)F".into(),
    ])
}

/// 管理者と SYSTEM だけに絞った置き場に、インストーラを走らせている利用者（`sid`）の読みだけを足す。
/// `initdb` は管理者として起こされると Administrators を deny-only にした制限付きの token で自分を起こし直すので、
/// 管理者の ACE では `--pwfile` を読めない。利用者の SID の読みは残る（本人とサーバのアカウントには読ませない。review R76）。
pub fn staging_reader_grant_command(directory: &Path, sid: &str) -> SetupCommand {
    icacls([
        directory.display().to_string(),
        "/grant".into(),
        format!("*{sid}:(OI)(CI)R"),
    ])
}

/// プログラムの置き場に別の組み立てのファイルがあるときの理由（review R50）。
pub fn program_version_mismatch_message(path: &str) -> String {
    format!(
        "プログラムの置き場に、この zip と違う {path} があります（別の版が入っています）。版を替える口はまだ無いので、何も替えずに止めます。入っている版の ashiato-setup.exe で走らせ直してください（docs/install.md）"
    )
}

/// 消す口の命令。サービスを止めて消し、登録を消し、プログラムの置き場を自分の写しの終了後に消す。
/// 状態の置き場と取り込み待ちの中身には触らない（Q4）。ACE を外す命令は `uninstall_acl_commands`。
/// `executable` は管理者だけが書ける置き場（`uninstall_copy_dir`）の写しで、遅れて消す台本も同じ置き場に置き、
/// 最後に置き場ごと消す。
pub fn uninstall_commands(layout: &Layout, executable: &Path) -> Vec<SetupCommand> {
    let delayed = executable.with_extension("cmd");
    vec![
        stop_service_and_wait_command(layout.server_service),
        stop_service_and_wait_command(layout.db_service),
        stop_service_and_wait_command(layout.updater_service),
        delete_service_command(layout.server_service),
        delete_service_command(layout.db_service),
        delete_service_command(layout.updater_service),
        SetupCommand::new(
            "reg.exe",
            [
                "delete",
                r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato",
                "/f",
            ],
        ),
        // 台本そのものはインストーラが `create_new` で書く（`delayed_removal_script`）。PowerShell の
        // Set-Content は先に置かれたファイルを上書きして使うので、ここでは起こすだけにする
        SetupCommand::powershell(format!(
            r#"Start-Process cmd.exe -ArgumentList '/c', '"{}"' -WindowStyle Hidden"#,
            delayed.display()
        )),
    ]
}

/// 消す口が遅れて走らせる台本（写しの置き場の `<写し>.cmd`）。3 秒待ってプログラムの置き場を消し、写しを消し、
/// 最後に写しの置き場ごと消す。
pub fn delayed_removal_script(layout: &Layout, executable: &Path) -> String {
    // 置き場の形は Windows のもの。区切りの `\` で切る（Linux の試験でも同じ文字列になる）
    let executable_text = executable.display().to_string();
    let copy_dir = executable_text
        .rsplit_once('\\')
        .map_or_else(String::new, |(parent, _)| parent.to_owned());
    [
        "@echo off".to_owned(),
        "ping 127.0.0.1 -n 3 >nul".to_owned(),
        format!(r#"rmdir /s /q "{}""#, layout.program_dir),
        format!(r#"del /f /q "{executable_text}""#),
        format!(r#"(goto) 2>nul & rmdir /s /q "{copy_dir}""#),
    ]
    .join("\r\n")
        + "\r\n"
}

pub fn delete_service_command(service: &str) -> SetupCommand {
    SetupCommand::new("sc.exe", ["delete", service])
}

/// 消す口で、ダウンロードと取り込み待ちに足したサーバのアカウントの ACE を外す。
pub fn uninstall_acl_commands(
    layout: &Layout,
    profile: &str,
    downloads: &str,
) -> Vec<SetupCommand> {
    let server = service_principal(layout.server_service);
    let mut commands = [
        downloads.to_owned(),
        inbox_dir(profile),
        inbox_parent(profile),
    ]
    .into_iter()
    .map(|path| icacls([path, "/remove:g".into(), server.clone()]))
    .collect::<Vec<_>>();
    commands.push(icacls([
        inbox_parent(profile),
        "/remove:g".into(),
        format!("*{CREATOR_OWNER}"),
    ]));
    commands
}

pub fn elevation_count(already_administrator: bool) -> u8 {
    u8::from(!already_administrator)
}

/// `whoami /user` の出力から SID を読む。
pub fn owner_sid_from_whoami(output: &str) -> Option<String> {
    output
        .split(|c: char| c.is_whitespace() || c == ',' || c == '"')
        .find(|part| part.starts_with("S-1-"))
        .map(str::to_owned)
}

/// `whoami /groups` の出力から、上がった札（高い整合性。SID `S-1-16-12288` 以上）かを読む。
/// 表示の語（High Mandatory Level）は言語で変わるので SID で見る。
pub fn is_elevated_from_whoami_groups(output: &str) -> bool {
    output.contains("S-1-16-12288") || output.contains("S-1-16-16384")
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Windows の引数の規則（CommandLineToArgvW）で 1 つの引数を `"` でくくる。
/// `Start-Process -ArgumentList` は配列を空白で繋ぐだけで引用しないので、こちらで引用しておく。
pub fn windows_arg_quote(value: &str) -> String {
    let mut quoted = String::from('"');
    let mut backslashes = 0;
    for c in value.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.push_str(&"\\".repeat(backslashes));
                quoted.push(c);
                backslashes = 0;
            }
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

/// 自分を管理者として起こし直し、終わるまで待って終了コードを返す PowerShell の式（D22）。
/// 確認を断ると `Start-Process` が失敗し、式は 0 でない終了コードで終わる。`Start-Process` の失敗は
/// 既定では文を終えるだけで残りが走り `exit $null`（= 0）になるので、失敗で式ごと止め（`Stop`）、
/// 子の結果が無ければ 1 で終わる（review R33）。
pub fn elevation_script(executable: &str, arguments: &[&str]) -> String {
    elevation_script_with_verb(executable, arguments, "-Verb RunAs ")
}

/// 昇格を断ったときと同じく `Start-Process` が失敗する式（試験の差し込み `ELEVATION=deny` だけが使う。
/// 起こすものを無い場所にし、断られたときと同じ式の残り（`Stop`・`-not $p`）を通す。review R37）。
pub fn denied_elevation_script(arguments: &[&str]) -> String {
    elevation_script_with_verb(r"C:\ashiato-elevation-denied\missing.exe", arguments, "")
}

fn elevation_script_with_verb(executable: &str, arguments: &[&str], verb: &str) -> String {
    format!(
        "$ErrorActionPreference = 'Stop'; $p = Start-Process {verb}-Wait -PassThru -FilePath {} -ArgumentList @({}); if (-not $p) {{ exit 1 }}; exit $p.ExitCode",
        powershell_quote(executable),
        arguments
            .iter()
            .map(|argument| powershell_quote(&windows_arg_quote(argument)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// 引数なしで開いたインストーラが、控えた本人を渡して自分を起こし直す式。
pub fn elevation_command(
    executable: &str,
    owner_sid: &str,
    profile: &str,
    downloads: &str,
) -> String {
    elevation_script(
        executable,
        &[
            "install",
            "--owner-sid",
            owner_sid,
            "--profile",
            profile,
            "--downloads",
            downloads,
            "--elevated",
        ],
    )
}

pub fn password_elevation_command(executable: &str) -> String {
    elevation_script(executable, &["password", "--elevated"])
}

pub fn repair_elevation_command(executable: &str, profile: &str, downloads: &str) -> String {
    elevation_script(
        executable,
        &[
            "repair",
            "--profile",
            profile,
            "--downloads",
            downloads,
            "--elevated",
        ],
    )
}

pub fn uninstall_elevation_command(executable: &str) -> String {
    elevation_script(executable, &["uninstall", "--elevated"])
}

pub fn update_check_elevation_command(executable: &str, enabled: bool) -> String {
    elevation_script(
        executable,
        &[
            "update-check",
            if enabled { "on" } else { "off" },
            "--elevated",
        ],
    )
}

/// 継承を切って ACL を付ける置き場と、そこに書くサービス（D3・D17）。
/// 根は `ashiato-server`、`db` と `logs\db` は `ashiato-db`（サーバは書けない）。
pub fn acl_targets(layout: &Layout) -> [(String, &'static str); 3] {
    [
        (layout.state_dir.to_owned(), layout.server_service),
        (format!(r"{}\db", layout.state_dir), layout.db_service),
        (format!(r"{}\logs\db", layout.state_dir), layout.db_service),
    ]
}

/// 置き場の ACL を、サービスのアカウントの書きと本人の読みだけにする（初めて作った置き場にだけ使う。
/// サービスが書いたファイルができた後は変えない）。先に `/reset` で明示の ACE を消し、
/// 次に継承を切って 2 つだけを付ける。
pub fn acl_commands(layout: &Layout, owner_sid: &str) -> Vec<SetupCommand> {
    acl_targets(layout)
        .into_iter()
        .flat_map(|(path, service)| {
            [
                icacls([path.clone(), "/reset".into()]),
                icacls([
                    path,
                    "/inheritance:r".into(),
                    "/grant:r".into(),
                    format!("{}:(OI)(CI)M", service_principal(service)),
                    format!("*{owner_sid}:(OI)(CI)RX"),
                ]),
            ]
        })
        .collect()
}

/// 初回の置き場では、サービスを先に登録して SID を Windows に引かせてから ACL を絞る。
/// 未登録のサービス SID は `icacls` が名前解決できず、ACE を付けられない。
pub fn fresh_state_setup_commands(layout: &Layout, owner_sid: &str) -> Vec<SetupCommand> {
    let mut commands = vec![
        register_database_command(layout),
        create_server_command(layout),
    ];
    commands.extend(configure_service_commands(layout));
    commands.extend(acl_commands(layout, owner_sid));
    commands
}

pub fn updater_executable(layout: &Layout) -> PathBuf {
    PathBuf::from(format!(r"{}\ashiato-updater.exe", layout.program_dir))
}

pub fn updater_settings_path(layout: &Layout) -> PathBuf {
    PathBuf::from(format!(r"{}\settings\check.json", layout.updater_dir))
}

pub fn updater_test_hooks_path(layout: &Layout) -> PathBuf {
    PathBuf::from(format!(r"{}\settings\test-hooks.json", layout.updater_dir))
}

pub fn register_updater_command(layout: &Layout) -> SetupCommand {
    SetupCommand::new(
        "sc.exe",
        [
            "create",
            layout.updater_service,
            "binPath=",
            &format!(r#""{}" service"#, updater_executable(layout).display()),
            "obj=",
            &format!(r"NT SERVICE\{}", layout.updater_service),
            "start=",
            "auto",
        ],
    )
}

pub fn updater_acl_commands(layout: &Layout, owner_sid: &str) -> Vec<SetupCommand> {
    let service = service_principal(layout.updater_service);
    let server = service_principal(layout.server_service);
    let owner = format!("*{owner_sid}");
    let root = layout.updater_dir.to_owned();
    [
        root.clone(),
        format!(r"{}\settings", root),
        format!(r"{}\logs", root),
    ]
    .into_iter()
    .map(|path| {
        let is_settings = path.ends_with("\\settings");
        icacls([
            path,
            "/inheritance:r".into(),
            "/grant:r".into(),
            format!("{service}:(OI)(CI){}", if is_settings { "R" } else { "M" }),
            format!("{server}:(OI)(CI)RX"),
            format!("{owner}:(OI)(CI)RX"),
            "*S-1-5-32-544:(OI)(CI)F".into(),
            "*S-1-5-18:(OI)(CI)F".into(),
        ])
    })
    .collect()
}

pub fn updater_settings_acl_command(layout: &Layout, owner_sid: &str) -> SetupCommand {
    let path = updater_settings_path(layout).display().to_string();
    icacls([
        path,
        "/inheritance:r".into(),
        "/grant:r".into(),
        format!("{}:R", service_principal(layout.updater_service)),
        format!("{}:R", service_principal(layout.server_service)),
        format!("*{owner_sid}:R"),
        "*S-1-5-32-544:F".into(),
        "*S-1-5-18:F".into(),
    ])
}

pub fn configure_updater_commands(layout: &Layout, enabled: bool) -> Vec<SetupCommand> {
    let start = if enabled { "auto" } else { "disabled" };
    vec![
        SetupCommand::new(
            "sc.exe",
            [
                "config",
                layout.updater_service,
                "obj=",
                &format!(r"NT SERVICE\{}", layout.updater_service),
                "start=",
                start,
            ],
        ),
        SetupCommand::new(
            "sc.exe",
            [
                "failure",
                layout.updater_service,
                "reset=",
                "86400",
                "actions=",
                "restart/10000/restart/10000/restart/10000",
            ],
        ),
        SetupCommand::new("sc.exe", ["failureflag", layout.updater_service, "1"]),
    ]
}

pub fn update_check_service_commands(layout: &Layout, enabled: bool) -> Vec<SetupCommand> {
    let start = if enabled { "auto" } else { "disabled" };
    let mut result = vec![SetupCommand::new(
        "sc.exe",
        ["config", layout.updater_service, "start=", start],
    )];
    if enabled {
        result.push(start_service_command(layout.updater_service));
    } else {
        result.insert(0, stop_service_and_wait_command(layout.updater_service));
    }
    result
}

/// 初めて作る状態の置き場の持ち主を `Administrators` にする（先に別の利用者が作っていても持ち主にさせない）。
/// `/L` でリンクの行き先ではなくリンク自体を扱う。
pub fn take_ownership_command(layout: &Layout) -> SetupCommand {
    icacls([layout.state_dir, "/setowner", "*S-1-5-32-544", "/T", "/C"])
}

/// インストーラ（上がった利用者）が書く段の間だけ書きを足す置き場（D21）。どちらもインストーラだけが
/// 書く置き場で、中のファイルの持ち主もインストーラなので、ACL の変更が中へ届く。
/// サービスが書く置き場（根の `logs\server`・`db\17` など）へは一時の書きを足さない
/// （持ち主がサービスのアカウントのファイルへは ACL の変更が届かない）。
pub fn temporary_write_paths(layout: &Layout) -> [String; 2] {
    [
        format!(r"{}\settings", layout.state_dir),
        format!(r"{}\backups\pre-migrate", layout.state_dir),
    ]
}

/// 管理者が持つ backup/restore 特権を有効にして、既存 DACL に一時の ACE を加える。
/// 閉じた既存の状態の置き場では、`takeown` の後でも所有者の暗黙の `WRITE_DAC` が使えないことが
/// ある。`icacls` は backup semantics で対象を開かないため、同じプロセスが `CreateFile` で
/// `READ_CONTROL | WRITE_DAC` のハンドルを開き、DACL を読んでから更新する。
fn privileged_ace_command(
    path: String,
    sid: &str,
    mask: u32,
    mode: u32,
    inherit: u32,
) -> SetupCommand {
    let path_for_ps = path.replace('\'', "''");
    SetupCommand::powershell(format!(
        r#"$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
public static class AshiatoRestorePrivilege {{
  private const uint READ_CONTROL = 0x00020000, WRITE_DAC = 0x00040000;
  private const uint FILE_SHARE_READ = 1, FILE_SHARE_WRITE = 2, FILE_SHARE_DELETE = 4;
  private const uint OPEN_EXISTING = 3, FILE_FLAG_BACKUP_SEMANTICS = 0x02000000;
  private const uint DACL_SECURITY_INFORMATION = 4, SE_FILE_OBJECT = 1;
  private const uint GRANT_ACCESS = 1, OBJECT_INHERIT_ACE = 1, CONTAINER_INHERIT_ACE = 2;
  private const uint MODIFY = 0x001301BF;
  [StructLayout(LayoutKind.Sequential)] private struct Luid {{ public uint LowPart; public int HighPart; }}
  [StructLayout(LayoutKind.Sequential)] private struct TokenPrivileges {{ public uint Count; public Luid Luid; public uint Attributes; }}
  [StructLayout(LayoutKind.Sequential)] private struct Trustee {{ public IntPtr MultipleTrustee; public int MultipleTrusteeOperation; public int TrusteeForm; public int TrusteeType; public IntPtr Name; }}
  [StructLayout(LayoutKind.Sequential)] private struct ExplicitAccess {{ public uint AccessPermissions; public uint AccessMode; public uint Inheritance; public Trustee Trustee; }}
  [DllImport("advapi32.dll", SetLastError=true)] private static extern bool OpenProcessToken(IntPtr process, uint access, out IntPtr token);
  [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)] private static extern bool LookupPrivilegeValue(string system, string name, out Luid luid);
  [DllImport("advapi32.dll", SetLastError=true)] private static extern bool AdjustTokenPrivileges(IntPtr token, bool disableAll, ref TokenPrivileges state, uint length, IntPtr previous, IntPtr returnedLength);
  [DllImport("advapi32.dll", SetLastError=true)] private static extern uint GetSecurityInfo(IntPtr handle, uint objectType, uint securityInformation, IntPtr owner, IntPtr group, out IntPtr dacl, IntPtr sacl, out IntPtr securityDescriptor);
  [DllImport("advapi32.dll", SetLastError=true)] private static extern uint SetEntriesInAcl(uint count, IntPtr entries, IntPtr oldAcl, out IntPtr newAcl);
  [DllImport("advapi32.dll", SetLastError=true)] private static extern uint SetSecurityInfo(IntPtr handle, uint objectType, uint securityInformation, IntPtr owner, IntPtr group, IntPtr dacl, IntPtr sacl);
  [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)] private static extern bool ConvertStringSidToSid(string value, out IntPtr sid);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] private static extern IntPtr CreateFile(string path, uint access, uint share, IntPtr security, uint creation, uint flags, IntPtr template);
  [DllImport("kernel32.dll", SetLastError=true)] private static extern bool CloseHandle(IntPtr handle);
  [DllImport("kernel32.dll")] private static extern IntPtr LocalFree(IntPtr memory);
  private static void Check(uint error) {{ if (error != 0) throw new Win32Exception((int)error); }}
  private static void Enable(string name) {{
    IntPtr token;
    if (!OpenProcessToken(Process.GetCurrentProcess().Handle, 0x28, out token)) throw new Win32Exception(Marshal.GetLastWin32Error());
    try {{
      Luid luid;
      if (!LookupPrivilegeValue(null, name, out luid)) throw new Win32Exception(Marshal.GetLastWin32Error());
      var state = new TokenPrivileges {{ Count = 1, Luid = luid, Attributes = 2 }};
      if (!AdjustTokenPrivileges(token, false, ref state, 0, IntPtr.Zero, IntPtr.Zero)) throw new Win32Exception(Marshal.GetLastWin32Error());
      int error = Marshal.GetLastWin32Error();
      if (error != 0) throw new Win32Exception(error);
    }} finally {{ CloseHandle(token); }}
  }}
  public static void Apply(string path, string sidText, uint mask, uint mode, uint inherit) {{
    Enable("SeBackupPrivilege"); Enable("SeRestorePrivilege");
    IntPtr handle = CreateFile(path, READ_CONTROL | WRITE_DAC, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, IntPtr.Zero);
    if (handle == new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());
    IntPtr descriptor = IntPtr.Zero, oldAcl = IntPtr.Zero, sid = IntPtr.Zero, entries = IntPtr.Zero, newAcl = IntPtr.Zero;
    try {{
      Check(GetSecurityInfo(handle, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION, IntPtr.Zero, IntPtr.Zero, out oldAcl, IntPtr.Zero, out descriptor));
      if (!ConvertStringSidToSid(sidText, out sid)) throw new Win32Exception(Marshal.GetLastWin32Error());
      var access = new ExplicitAccess {{ AccessPermissions = mask, AccessMode = mode, Inheritance = inherit, Trustee = new Trustee {{ TrusteeForm = 0, TrusteeType = 0, Name = sid }} }};
      entries = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(ExplicitAccess)));
      Marshal.StructureToPtr(access, entries, false);
      Check(SetEntriesInAcl(1, entries, oldAcl, out newAcl));
      Check(SetSecurityInfo(handle, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION, IntPtr.Zero, IntPtr.Zero, newAcl, IntPtr.Zero));
    }} finally {{
      if (newAcl != IntPtr.Zero) LocalFree(newAcl); if (entries != IntPtr.Zero) Marshal.FreeHGlobal(entries);
      if (sid != IntPtr.Zero) LocalFree(sid); if (descriptor != IntPtr.Zero) LocalFree(descriptor); CloseHandle(handle);
    }}
  }}
}}
'@
[AshiatoRestorePrivilege]::Apply('{path_for_ps}', '{sid}', {mask}, {mode}, {inherit})"#
    ))
}

/// 一時の書き（M・子へ継ぐ）を特権で足す。
fn restore_privileged_grant_command(path: String, sid: &str) -> SetupCommand {
    // MODIFY・GRANT_ACCESS・OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE
    privileged_ace_command(path, sid, 0x0013_01BF, 1, 1 | 2)
}

/// 状態の置き場の根だけを見る権限（RX・子へ継がない）を特権で足す。別の利用者を本人として渡した
/// インストールでは、上がった管理者は根の ACL に載らず、initdb が根を見られずに「File exists」で落ちた
/// （run 42209316618）。
fn root_traverse_grant_command(layout: &Layout, sid: &str) -> SetupCommand {
    // READ_EXECUTE（FILE_GENERIC_READ | FILE_GENERIC_EXECUTE）・GRANT_ACCESS・継がない
    privileged_ace_command(layout.state_dir.to_owned(), sid, 0x0012_00A9, 1, 0)
}

/// 上の根の権限を外す（REVOKE_ACCESS は、その主体の明示の ACE を全部外す）。
fn root_traverse_revoke_command(layout: &Layout, sid: &str) -> SetupCommand {
    privileged_ace_command(layout.state_dir.to_owned(), sid, 0, 4, 0)
}

/// 一時の書きを足す。走らせ直した管理者には、まず書込みを足す。先に `/reset` すると、本人以外の
/// 管理者は権限が無い状態で ACL の変更を求められて止まる。前の回の明示の ACE は終了時の `/reset` で消す。
pub fn temporary_write_acl_commands(layout: &Layout, sid: &str) -> Vec<SetupCommand> {
    temporary_write_paths(layout)
        .into_iter()
        .flat_map(|path| {
            [
                // 既存の状態では本人だけが ACL に載る。管理者 B が所有者になっても、暗黙の
                // WRITE_DAC に頼る `icacls /grant` は終了コード 5 になった（run 42153449016）。
                // 最初の一時 ACL は管理者の SeRestorePrivilege で付ける。これで親を読めるので、
                // 子も含めた所有者の回復と一時 ACL の付与ができる。
                restore_privileged_grant_command(path.clone(), sid),
                SetupCommand::new(
                    "takeown",
                    [
                        "/F".to_owned(),
                        path.clone(),
                        "/A".to_owned(),
                        "/R".to_owned(),
                        "/D".to_owned(),
                        "Y".to_owned(),
                    ],
                ),
                // 上の一時の書込みで子を列挙できるので、Administrators を持ち主に戻す。前の回の
                // 管理者が持ち主だった `server.json` などにも今回の書込みを届かせる。
                icacls([path.clone(), "/grant".into(), format!("*{sid}:(OI)(CI)M")]),
                // 一時の書込みを足せた後は、子を含む持ち主を Administrators へ戻す。`/reset` が
                // ACE を外しても今回の管理者が持ち主のまま残り、ACL を戻せる状態にならないようにする。
                // `icacls /setowner` は SeTakeOwnershipPrivilege を有効にせず、WRITE_OWNER の無い
                // 初回のインストールでも拒まれた（run 42051778199）。特権を自分で有効にする `takeown /A` で戻す。
                SetupCommand::new(
                    "takeown",
                    [
                        "/F".to_owned(),
                        path,
                        "/A".to_owned(),
                        "/R".to_owned(),
                        "/D".to_owned(),
                        "Y".to_owned(),
                    ],
                ),
            ]
        })
        .collect()
}

/// 一時の書きを外す。根から受け継ぐ形だけに戻す。
pub fn remove_temporary_write_acl_commands(layout: &Layout) -> Vec<SetupCommand> {
    temporary_write_paths(layout)
        .into_iter()
        .map(|path| icacls([path, "/reset".into()]))
        .collect()
}

/// `initdb` と DB の設定を書く間だけ、DB の置き場に上がった利用者の書きを足す。
/// DB のサービスを起こす前（中のファイルの持ち主がまだインストーラ）にだけ使う。
pub fn database_write_acl_commands(
    layout: &Layout,
    sid: &str,
    owner_sid: &str,
) -> Vec<SetupCommand> {
    let mut commands = Vec::new();
    // 本人と上がった管理者が違うと、管理者は根の ACL に載らない。initdb が根をたどれるよう、根だけを見る権限を
    // 一時に足す（本人と同じなら本人の RX があり、外すときに本人の ACE まで外さないよう足さない）
    if sid != owner_sid {
        commands.push(root_traverse_grant_command(layout, sid));
    }
    commands.push(restore_privileged_grant_command(
        format!(r"{}\db", layout.state_dir),
        sid,
    ));
    commands
}

/// DB の置き場の一時の書きを外し、DB のサービスの書きと本人の読みだけに戻す。
pub fn remove_database_write_acl_commands(
    layout: &Layout,
    invoking_sid: &str,
    owner_sid: &str,
) -> Vec<SetupCommand> {
    let db = format!(r"{}\db", layout.state_dir);
    let mut commands = vec![icacls([
        db.clone(),
        "/inheritance:r".into(),
        "/grant:r".into(),
        format!("{}:(OI)(CI)M", service_principal(layout.db_service)),
        format!("*{owner_sid}:(OI)(CI)RX"),
    ])];
    if invoking_sid != owner_sid {
        commands.push(icacls([db, "/remove:g".into(), format!("*{invoking_sid}")]));
        commands.push(root_traverse_revoke_command(layout, invoking_sid));
    }
    commands
}

/// 本人のダウンロードにサーバの読み、取り込み待ちに読み書き、親にそのフォルダだけの作り直しと読み（D9）。
/// 取り込み待ちのフォルダ自体はインストーラが先に作る。
pub fn inbox_acl_commands(layout: &Layout, profile: &str, downloads: &str) -> Vec<SetupCommand> {
    let server = service_principal(layout.server_service);
    vec![
        icacls([
            downloads.to_owned(),
            "/grant:r".into(),
            format!("{server}:(OI)(CI)RX"),
        ]),
        icacls([
            inbox_dir(profile),
            "/grant:r".into(),
            format!("{server}:(OI)(CI)M"),
        ]),
        icacls([
            inbox_parent(profile),
            "/grant:r".into(),
            format!("{server}:(AD,RX)"),
        ]),
        // 消された取り込み待ちをサーバが作り直したとき、作った者（サーバ）が中を読み書きできるように、
        // 親には作った者への受け継ぐだけの書きを置く（親そのものには効かない。CREATOR OWNER）
        icacls([
            inbox_parent(profile),
            "/grant".into(),
            format!("*{CREATOR_OWNER}:(OI)(CI)(IO)M"),
        ]),
    ]
}

const CREATOR_OWNER: &str = "S-1-3-0";

/// 同梱の PostgreSQL の道具（プログラムの置き場の絶対の場所。D29（仮））。
/// 置き場の形は Windows のものなので、区切りも `\` で組む（Linux の試験でも同じ文字列になる）。
pub fn bundled_tool(layout: &Layout, name: &str) -> PathBuf {
    PathBuf::from(format!(r"{}\pgsql\bin\{name}", layout.program_dir))
}

pub fn server_executable(layout: &Layout) -> PathBuf {
    PathBuf::from(format!(r"{}\ashiato-server.exe", layout.program_dir))
}

pub fn settings_path(layout: &Layout) -> PathBuf {
    PathBuf::from(format!(r"{}\settings\server.json", layout.state_dir))
}

/// 表の所有者の合言葉の置き場（`settings\owner.json`）。サーバのサービスが読む `server.json` と分ける
/// （design D4。deep.md Q11・review R48）。移行の段・`repair`・版を替える口（ST57・ST58）だけが読む。
pub fn owner_settings_path(layout: &Layout) -> PathBuf {
    PathBuf::from(format!(r"{}\settings\owner.json", layout.state_dir))
}

/// 置いた `owner.json` の継承を切り、管理者（`S-1-5-32-544`）と SYSTEM（`S-1-5-18`）だけを明示の項目にする。
/// 書きかけの置き場から受け継いだ項目のまま置くと、設定の置き場の ACL を変えたとき（一時の書きの `/reset`）に
/// 設定の置き場の形（サーバのアカウントの書きと本人の読み）を受け継ぎ直す。
pub fn owner_settings_acl_command(path: &Path) -> SetupCommand {
    icacls([
        path.display().to_string(),
        "/inheritance:r".into(),
        "/grant:r".into(),
        "*S-1-5-32-544:F".into(),
        "*S-1-5-18:F".into(),
    ])
}

/// `owner.json` の持ち主を管理者にする（持ち主は ACL を書き換えられるので、ほかの主体を持ち主にしない）。
pub fn owner_settings_owner_command(path: &Path) -> SetupCommand {
    icacls([
        path.display().to_string(),
        "/setowner".into(),
        "*S-1-5-32-544".into(),
    ])
}

/// プログラムの置き場へ写すもの（zip を展開した置き場からの相対。D29（仮））。
pub fn program_files() -> [&'static str; 4] {
    [
        "ashiato-server.exe",
        "ashiato-setup.exe",
        "db-roles.sql",
        "pgsql",
    ]
}

/// `initdb`（同梱の絶対の場所。D1）。合言葉は `--pwfile` のファイルで渡す。
pub fn initdb_command(layout: &Layout, pwfile: &Path) -> SetupCommand {
    SetupCommand::new(
        bundled_tool(layout, "initdb.exe").display().to_string(),
        crate::db::initdb_args(pwfile, Path::new(layout.db_dir)),
    )
}

/// DB のサービスの登録（D2）。仮想アカウントは合言葉を持たないので `-P` を渡さない。
pub fn register_database_command(layout: &Layout) -> SetupCommand {
    SetupCommand::new(
        bundled_tool(layout, "pg_ctl.exe").display().to_string(),
        [
            "register".to_owned(),
            "-N".into(),
            layout.db_service.into(),
            "-U".into(),
            service_account(layout.db_service),
            "-D".into(),
            layout.db_dir.into(),
            "-S".into(),
            "auto".into(),
            "-w".into(),
        ],
    )
}

/// サーバのサービスの作成（D29（仮））。プログラムの置き場の `ashiato-server.exe service --settings <設定>` を
/// サーバの仮想アカウントで、自動・DB に依って起こす。
pub fn create_server_command(layout: &Layout) -> SetupCommand {
    let bin_path = format!(
        "{} service --settings {}",
        windows_arg_quote(&server_executable(layout).display().to_string()),
        windows_arg_quote(&settings_path(layout).display().to_string())
    );
    SetupCommand::new(
        "sc.exe",
        [
            "create".to_owned(),
            layout.server_service.into(),
            "binPath=".into(),
            bin_path,
            "obj=".into(),
            service_account(layout.server_service),
            "start=".into(),
            "auto".into(),
            "depend=".into(),
            layout.db_service.into(),
        ],
    )
}

/// 登録の後の確かめ直しと回復の設定（D2・D11）。どちらのサービスも仮想アカウントだけを名指しする。
pub fn configure_service_commands(layout: &Layout) -> Vec<SetupCommand> {
    let failure = |service: &str| {
        [
            SetupCommand::new(
                "sc.exe",
                [
                    "failure",
                    service,
                    "reset=",
                    "86400",
                    "actions=",
                    // 強制終了の直後は PostgreSQL の子プロセスや待受の解放と競合しうる。3 回目だけ
                    // 60 秒待つと、回復自体は設定されていても実地検査の 90 秒内に再試行できない。
                    // 失敗した回復は同じ短い間隔で続け、サービスを止めたままにしない。
                    "restart/10000/restart/10000/restart/10000",
                ],
            ),
            // サービス自身が停止を返した場合も回復を動かす（failure だけでは異常終了だけ）。
            SetupCommand::new("sc.exe", ["failureflag", service, "1"]),
        ]
    };
    let mut commands = vec![
        SetupCommand::new(
            "sc.exe",
            [
                "config".to_owned(),
                layout.db_service.into(),
                "obj=".into(),
                service_account(layout.db_service),
                "start=".into(),
                "auto".into(),
            ],
        ),
        SetupCommand::new(
            "sc.exe",
            [
                "config".to_owned(),
                layout.server_service.into(),
                "obj=".into(),
                service_account(layout.server_service),
                "start=".into(),
                "auto".into(),
                "depend=".into(),
                layout.db_service.into(),
            ],
        ),
    ];
    commands.extend(failure(layout.db_service));
    commands.extend(failure(layout.server_service));
    commands
}

/// DB のサービスの今の SDDL を読む命令（D35 の見張りの ACE を足す前に読む）。
pub fn db_service_sdshow_command(layout: &Layout) -> SetupCommand {
    SetupCommand::new("sc.exe", ["sdshow", layout.db_service])
}

/// `sc.exe sdshow` の出力から SDDL（最後の空でない行）を取り出す。
pub fn sddl_from_sdshow(output: &str) -> Option<&str> {
    output.lines().map(str::trim).rfind(|line| !line.is_empty())
}

/// サーバの仮想アカウントに、DB のサービスの状態を読むことと開始することだけを許す ACE（D35）。
/// 停止・設定変更の権限は与えない。
pub fn db_watch_ace(layout: &Layout) -> String {
    format!("(A;;LCRP;;;{})", service_sid(layout.server_service))
}

/// 今の SDDL に見張りの ACE を足して書く命令。既に同じ ACE があれば `None`（何も書かない）。
/// インストーラは `sdshow` の結果をこれに通して `sdset` を起こす（試験が照らす関数が本番の経路。review R71・R83）。
pub fn db_watch_sdset_command(layout: &Layout, current_sddl: &str) -> Option<SetupCommand> {
    let updated = add_db_watch_ace(current_sddl, &db_watch_ace(layout));
    (updated != current_sddl)
        .then(|| SetupCommand::new("sc.exe", ["sdset", layout.db_service, updated.as_str()]))
}

/// SDDL の DACL の末尾（`S:` の前、無ければ末尾）に ACE を足す。同じ ACE があればそのまま返す。
pub fn add_db_watch_ace(sddl: &str, ace: &str) -> String {
    if sddl.contains(ace) {
        return sddl.to_owned();
    }
    sddl.find("S:").map_or_else(
        || format!("{sddl}{ace}"),
        |sacl_start| {
            let mut result = sddl.to_owned();
            result.insert_str(sacl_start, ace);
            result
        },
    )
}

/// 「アプリと機能」の登録（段 (12)。D25）。`UninstallString` は引数として直に渡す（PowerShell を通すと
/// 5.1 で中の `"` が落ちる。review R18）。版番号はインストーラに組み込んだ版（`ASHIATO_VERSION`）。
pub fn uninstall_registration_commands(layout: &Layout, version: &str) -> Vec<SetupCommand> {
    let key = r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato";
    let setup = PathBuf::from(format!(r"{}\ashiato-setup.exe", layout.program_dir));
    [
        ("DisplayName", "ashiato".to_owned()),
        ("DisplayVersion", version.to_owned()),
        (
            "UninstallString",
            format!(
                "{} uninstall",
                windows_arg_quote(&setup.display().to_string())
            ),
        ),
    ]
    .into_iter()
    .map(|(name, value)| {
        SetupCommand::new(
            "reg.exe",
            [
                "add".to_owned(),
                key.into(),
                "/v".into(),
                name.into(),
                "/t".into(),
                "REG_SZ".into(),
                "/d".into(),
                value,
                "/f".into(),
            ],
        )
    })
    .collect()
}

pub fn parse_args(args: &[&str]) -> Result<String, String> {
    let command = args.first().copied().unwrap_or("install");
    if !matches!(
        command,
        "install" | "password" | "repair" | "uninstall" | "update-check"
    ) {
        return Err(format!("不明な引数: {command}"));
    }
    let mut index = usize::from(!args.is_empty());
    if command == "update-check" {
        match args.get(index).copied() {
            Some("off" | "on") => index += 1,
            _ => return Err("update-check は off または on を指定してください".into()),
        }
    }
    while index < args.len() {
        match args[index] {
            "--owner-sid" | "--profile" | "--downloads" => {
                if args.get(index + 1).is_none() || args[index + 1].starts_with('-') {
                    return Err(format!("引数の値が無い: {}", args[index]));
                }
                index += 2;
            }
            "--elevated" => index += 1,
            _ => return Err(format!("不明な引数: {}", args[index])),
        }
    }
    Ok(command.to_owned())
}

/// 引数の値を読む（`parse_args` で形を確かめた後に使う）。
pub fn arg_value<'a>(args: &[&'a str], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|argument| *argument == name)
        .and_then(|index| args.get(index + 1))
        .copied()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub fn migrate_command(
    server: &Path,
    owner_url: &str,
    backup_dir: &Path,
    pg_dump: &Path,
) -> MigrateCommand {
    MigrateCommand {
        program: server.display().to_string(),
        args: vec!["migrate".into()],
        env: vec![
            ("DATABASE_OWNER_URL".into(), owner_url.into()),
            (
                "MIGRATE_BACKUP_DIR".into(),
                backup_dir.display().to_string(),
            ),
            ("PG_DUMP".into(), pg_dump.display().to_string()),
        ],
    }
}

/// 同梱の `psql` で DB の管理者の役割 `ashiato`（D1）として入る命令。合言葉は環境変数
/// （`PGPASSWORD`・`ADMIN_PW` など）で渡し、命令の引数には載せない。
pub fn psql_command(layout: &Layout, host: &str, port: u16, database: &str) -> SetupCommand {
    SetupCommand::new(
        bundled_tool(layout, "psql.exe").display().to_string(),
        [
            "-X".to_owned(),
            "-q".into(),
            "-v".into(),
            "ON_ERROR_STOP=1".into(),
            "-v".into(),
            format!("admin={}", crate::db::ADMIN_ROLE),
            "-v".into(),
            "dbname=ashiato".into(),
            "-U".into(),
            crate::db::ADMIN_ROLE.into(),
            "-h".into(),
            host.into(),
            "-p".into(),
            port.to_string(),
            "-d".into(),
            database.into(),
        ],
    )
}

/// 配る DB の `ashiato` を、無ければ作る SQL（initdb は `postgres` だけを作る）。
pub fn create_database_sql() -> &'static str {
    "SELECT 'CREATE DATABASE ashiato' WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'ashiato') \\gexec\n"
}

pub fn start_service_command(service: &str) -> SetupCommand {
    SetupCommand::new("sc.exe", ["start", service])
}

pub fn query_service_command(service: &str) -> SetupCommand {
    SetupCommand::new("sc.exe", ["query", service])
}

pub fn start_database_command(layout: &Layout) -> SetupCommand {
    start_service_command(layout.db_service)
}

pub fn start_server_command(layout: &Layout) -> SetupCommand {
    start_service_command(layout.server_service)
}

/// 前回の repair の途中で残った一時の DB を止める（D27 の再試行）。
pub fn repair_cleanup_commands(layout: &Layout) -> Vec<SetupCommand> {
    vec![SetupCommand::powershell(format!(
        "if ((Test-Path -LiteralPath '{}\\postmaster.pid') -and ((Get-Service -Name '{}' -ErrorAction SilentlyContinue).Status -ne 'Running')) {{ & '{}' stop -D '{}' -m fast -w }}",
        layout.db_dir,
        layout.db_service,
        bundled_tool(layout, "pg_ctl.exe").display(),
        layout.db_dir,
    ))]
}

/// サービスが止まるまで待つ上限の秒（サーバは DB を最大 120 秒待つので、その後の終わりまでを含める）。
pub const SERVICE_STOP_TIMEOUT_SECS: u32 = 180;

/// サービスを止めて、止まるまで待つ。起きる途中（`StartPending`）なら動き出すのを待ってから止める
/// （起きる途中で止めずに進むと、前の設定を読んだサーバが動き続ける）。上限を過ぎれば失敗で終わる（review R51）。
pub fn stop_service_and_wait_command(service: &str) -> SetupCommand {
    SetupCommand::powershell(format!(
        "$s = Get-Service -Name '{service}' -ErrorAction SilentlyContinue; if ($s) {{ $deadline = (Get-Date).AddSeconds({SERVICE_STOP_TIMEOUT_SECS}); while ($s.Status -ne 'Stopped') {{ if ($s.Status -eq 'Running' -or $s.Status -eq 'Paused') {{ sc.exe stop '{service}' | Out-Null }}; if ((Get-Date) -gt $deadline) {{ [Console]::Error.WriteLine('{service} が {SERVICE_STOP_TIMEOUT_SECS} 秒で止まらない'); exit 1 }}; Start-Sleep -Milliseconds 500; $s.Refresh() }} }}"
    ))
}

/// repair の一時の形（D28）。手元の一時の番号で、一時の `hba_file` と `ident_file` を渡して起こす。
/// ログは `logs\db`（DB のサービスだけが書ける）でなく、上がった利用者が一時に書ける `log`（設定の置き場）へ
/// `pg_ctl -l` で書き、ログを集める子（`logging_collector`）は起こさない（review R32）。
pub fn repair_start_command(
    layout: &Layout,
    hba: &Path,
    ident: &Path,
    log: &Path,
    port: u16,
) -> SetupCommand {
    let forward = |path: &Path| path.display().to_string().replace('\\', "/");
    SetupCommand::new(
        bundled_tool(layout, "pg_ctl.exe").display().to_string(),
        [
            "start".to_owned(),
            "-D".into(),
            layout.db_dir.into(),
            "-w".into(),
            "-l".into(),
            log.display().to_string(),
            "-o".into(),
            format!(
                "-h 127.0.0.1 -p {port} -c logging_collector=off -c \"hba_file={}\" -c \"ident_file={}\"",
                forward(hba),
                forward(ident)
            ),
        ],
    )
}

pub fn repair_stop_command(layout: &Layout) -> SetupCommand {
    SetupCommand::new(
        bundled_tool(layout, "pg_ctl.exe").display().to_string(),
        ["stop", "-D", layout.db_dir, "-m", "fast", "-w"],
    )
}

pub fn repair_hba_contents() -> &'static str {
    "host all ashiato 127.0.0.1/32 sspi map=repair\n"
}

/// `whoami` の `DOMAIN\user` を、SSPI が渡す名前（`include_realm=1` の `user@DOMAIN`）に合わせた
/// pg_ident の行にする。英字だけの名前は、大文字と小文字を Windows の名前と同じく区別しない正規表現にし、
/// 英数字以外の文字は ARE の符号の書き方（`\uXXXX`・`\UXXXXXXXX`）にして、正規表現の記号としても
/// 行の区切り（空白）としても読ませない。括弧でくくる形は `^` で `[^]`（壊れた式）になる（review R58）。
/// 英字でない名前は正規表現にせず、引用符でくくった文字どおりの行（`repair "<user>@<domain>" ashiato`）にする。
/// ANSI のコード頁で符号化する前提で、正規表現にすると cp932 の 2 バイト目の `0x5C`（`\`）が式を壊す（CH14 D9）。
///
/// crate の外（main.rs）には見せない。対応表のバイト列は `console::repair_ident_bytes`（ANSI のコード頁で
/// 符号化し、表せなければ止まる。CH14 D9）だけが組む。UTF-8 の文字列のまま書く形に戻すと組めない（review/code.md R15）
pub(crate) fn repair_ident_contents(account: &str) -> String {
    let (domain, user) = account.split_once('\\').unwrap_or(("", account));
    if !account.is_ascii() {
        // 英字でない名前は ANSI のコード頁のバイト列で来る。正規表現に入れると cp932 の 2 バイト目
        // （`\` など）で行が壊れるので、引用符でくくった文字どおりの行にする（D9）
        let name = if domain.is_empty() {
            user.to_owned()
        } else {
            format!("{user}@{domain}")
        };
        return format!("repair \"{name}\" ashiato\n");
    }
    let literal = |text: &str| {
        text.chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c.to_string()
                } else if u32::from(c) <= 0xFFFF {
                    format!("\\u{:04X}", u32::from(c))
                } else {
                    format!("\\U{:08X}", u32::from(c))
                }
            })
            .collect::<String>()
    };
    let pattern = if domain.is_empty() {
        format!("/(?i)^{}(@.*)?$", literal(user))
    } else {
        format!("/(?i)^{}@{}$", literal(user), literal(domain))
    };
    format!("repair {pattern} ashiato\n")
}
