pub mod commands;
pub mod db;
pub mod layout;
pub mod password;
pub mod secrets;
pub mod state;

#[cfg(test)]
mod tests {
    use super::commands::SetupCommand;
    use super::{commands, db, layout, password, secrets, state};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn st55_password_length_boundary_is_sixteen() {
        // Scenario: 短い・既定の雛形・一致しない合言葉は受け付けない
        // 境目で見る: 15 文字は断り、16 文字は通す（review R44）
        let fifteen = "web-password-15";
        let sixteen = "web-password-016";
        assert_eq!((fifteen.chars().count(), sixteen.chars().count()), (15, 16));
        let api = "api-secret-0123456789"; // gitleaks:allow（試験の固定値）
        let key = "session-secret-0123456789";
        assert_eq!(
            password::validate(fifteen, api, key, fifteen),
            Err(password::PasswordRefusal::TooShort)
        );
        assert_eq!(password::validate(sixteen, api, key, sixteen), Ok(()));
    }

    #[test]
    fn st55_password_rejects_all_five_invalid_reasons() {
        // Scenario: 短い・既定の雛形・一致しない合言葉は受け付けない
        assert_eq!(
            password::validate(
                "short-password",
                "api-secret-0123456789",
                "session-secret-0123456789",
                "other-password-012345"
            ),
            Err(password::PasswordRefusal::TooShort)
        );
        assert_eq!(
            password::validate(
                "change-me-secret-012345",
                "api-secret-0123456789",
                "session-secret-0123456789",
                "change-me-secret-012345"
            ),
            Err(password::PasswordRefusal::Placeholder)
        );
        assert_eq!(
            password::validate(
                "api-secret-0123456789",
                "api-secret-0123456789",
                "session-secret-0123456789",
                "api-secret-0123456789"
            ),
            Err(password::PasswordRefusal::SameAsApiToken)
        );
        assert_eq!(
            password::validate(
                "session-secret-0123456789",
                "api-secret-0123456789",
                "session-secret-0123456789",
                "session-secret-0123456789"
            ),
            Err(password::PasswordRefusal::SameAsSessionKey)
        );
        assert_eq!(
            password::validate(
                "first-password-012345",
                "api-secret-0123456789",
                "session-secret-0123456789",
                "second-password-012345"
            ),
            Err(password::PasswordRefusal::Mismatch)
        );
    }

    #[test]
    fn st55_password_verifier_does_not_contain_raw_password() {
        // Scenario: インストールした形の設定は、画面の合言葉の元の値を持たない
        let password = "install-password-012345";
        let verifier = password::verifier(password).expect("test password should hash");
        assert!(verifier.starts_with("$argon2id$") && !verifier.contains(password));
    }

    #[test]
    fn st55_password_accepts_valid_pair() {
        // Scenario: 決めた合言葉で、そのまま画面にログインできる
        assert!(password::validate(
            "install-password-012345",
            "api-secret-0123456789",
            "session-secret-0123456789",
            "install-password-012345",
        )
        .is_ok());
    }

    #[test]
    fn st55_password_settings_keep_verifier_only() {
        // Scenario: 合言葉の元の値は、どこにも残らない
        let raw = "install-password-012345";
        let verifier = password::verifier(raw).expect("test password should hash");
        let settings = secrets::new_install_secrets()
            .with_web_password_verifier(verifier)
            .settings_json(
                &layout::Layout::release(),
                "C:\\Users\\A",
                "C:\\Downloads",
                "S-1-0",
            );
        let text = settings.to_string();
        assert!(!text.contains(raw));
        assert!(text.contains("$argon2id$"));
    }

    #[test]
    fn st55_password_reset_keeps_other_secrets() {
        // Scenario: 決め直しても、ほかの秘密と記録は変わらない
        let original = secrets::new_install_secrets();
        let changed = original.clone().with_web_password_verifier(
            password::verifier("new-password-012345").expect("test password should hash"),
        );
        assert_eq!(original.api_token, changed.api_token);
        assert_eq!(original.web_session_key, changed.web_session_key);
        assert_eq!(original.app_password, changed.app_password);
        assert_eq!(original.owner_password, changed.owner_password);
    }

    #[test]
    fn st55_password_reset_replaces_only_the_verifier_in_settings() {
        let original = serde_json::json!({
            "WEB_PASSWORD_VERIFIER": "old-verifier",
            "ASHIATO_INBOX_DIR": "C:/custom/inbox",
            "ASHIATO_DOWNLOADS_DIR": "D:/custom/downloads",
            "ASHIATO_OWNER_SID": "S-1-5-21-owner",
            "API_TOKEN": "api-secret",
        });
        let changed = secrets::replace_web_password_verifier(&original, "new-verifier");
        assert_eq!(changed["WEB_PASSWORD_VERIFIER"], "new-verifier");
        assert_eq!(changed["ASHIATO_INBOX_DIR"], original["ASHIATO_INBOX_DIR"]);
        assert_eq!(
            changed["ASHIATO_DOWNLOADS_DIR"],
            original["ASHIATO_DOWNLOADS_DIR"]
        );
        assert_eq!(changed["ASHIATO_OWNER_SID"], original["ASHIATO_OWNER_SID"]);
        assert_eq!(changed["API_TOKEN"], original["API_TOKEN"]);
    }

    fn lines(commands: &[SetupCommand]) -> String {
        commands
            .iter()
            .map(SetupCommand::line)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn st55_password_restart_waits_for_service_stop() {
        let command = commands::stop_service_and_wait_command("ashiato-server").line();
        assert!(command.contains("sc.exe stop 'ashiato-server'"));
        assert!(command.contains("Status -ne 'Stopped'"));
        // 起きる途中（StartPending）でも止めずに進まない: 動き出すまで待ってから止める（review R51）
        assert!(!command.contains("if ((Get-Service -Name 'ashiato-server' -ErrorAction SilentlyContinue).Status -eq 'Running')"));
        assert!(command.contains("$s.Status -eq 'Running'") && command.contains("$s.Refresh()"));
        // 止まらなければ上限で失敗する
        assert_eq!(commands::SERVICE_STOP_TIMEOUT_SECS, 180);
        assert!(command.contains("AddSeconds(180)") && command.contains("exit 1"));
    }

    #[test]
    fn st55_password_elevation_command_waits_and_returns_exit_code() {
        // 管理者で起こし直した `password` の失敗を親へ戻す（review R19）
        let script = commands::password_elevation_command(r"C:\Users\A\setup.exe");
        assert!(script.contains("-Verb RunAs"));
        assert!(script.contains("-Wait") && script.contains("-PassThru"));
        assert!(script.ends_with("exit $p.ExitCode"));
        assert!(script.contains(r#"'"password"'"#));
        assert!(!script.contains("secret"));
        // 確認を断ると Start-Process が失敗する。既定では文が終わるだけで exit $null（0）になるので、
        // 式ごと止め、子の結果が無ければ 1 で終わる（review R33）
        assert!(script.starts_with("$ErrorActionPreference = 'Stop'; "));
        assert!(script.contains("if (-not $p) { exit 1 }; exit $p.ExitCode"));
        // 試験の差し込みの断りも同じ式の形を通す（review R37）
        let denied = commands::denied_elevation_script(&["password", "--elevated"]);
        assert!(denied.starts_with("$ErrorActionPreference = 'Stop'; "));
        assert!(denied.contains("if (-not $p) { exit 1 }"));
        assert!(!denied.contains("RunAs"));
    }

    #[test]
    fn st55_layout_service_sid_matches_windows_virtual_account_sid() {
        // `sc.exe showsid TrustedInstaller` の値（Windows の既知の SID）と同じ計算で、登録の前に決まる（review R31）
        assert_eq!(
            commands::service_sid("TrustedInstaller"),
            "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464"
        );
        // 名前の大文字と小文字に依らない
        assert_eq!(
            commands::service_sid("ashiato-server"),
            commands::service_sid("ASHIATO-SERVER")
        );
        assert_ne!(
            commands::service_sid("ashiato-server"),
            commands::service_sid("ashiato-db")
        );
    }

    #[test]
    fn st55_layout_release_uses_fixed_names() {
        assert_eq!(layout::for_version("0.4.0"), layout::Layout::release());
    }

    #[test]
    fn st55_layout_beta_uses_fixed_names() {
        assert_eq!(
            layout::for_version("0.4.0-beta.1"),
            layout::Layout::release()
        );
    }

    #[test]
    fn st55_layout_dev_uses_dev_names() {
        assert_eq!(
            layout::for_version("0.4.0-dev.20261104"),
            layout::Layout::development()
        );
    }

    #[test]
    fn st55_layout_uac_command_has_no_secret() {
        let script = commands::elevation_command(
            "C:\\Users\\A\\setup.exe",
            "S-1-5-21",
            "C:\\Users\\A",
            "C:\\Downloads",
        );
        assert!(script.contains("-Verb RunAs") && script.contains("exit $p.ExitCode"));
        assert!(!script.contains("password") && !script.contains("token"));
    }

    // Scenario: 上がっていない状態から開くと、管理者に上がる起こし直しは 1 回だけ
    #[test]
    fn st55_layout_elevation_is_one_restart() {
        assert_eq!(commands::elevation_count(false), 1);
        assert_eq!(commands::elevation_count(true), 0);
    }

    // Scenario: 上がる前の段は、開いた利用者を本人として控える
    #[test]
    fn st55_layout_owner_sid_is_parsed_from_whoami() {
        assert_eq!(
            commands::owner_sid_from_whoami(
                "USER INFORMATION\n----------------\nUser Name                  SID\nowner                     S-1-5-21-42"
            ),
            Some("S-1-5-21-42".into())
        );
        // `whoami /user /fo csv /nh` の形
        assert_eq!(
            commands::owner_sid_from_whoami("\"pc\\owner\",\"S-1-5-21-7-8-9-1001\"\r\n"),
            Some("S-1-5-21-7-8-9-1001".into())
        );
    }

    #[test]
    fn st55_layout_elevation_is_read_from_integrity_sid() {
        assert!(commands::is_elevated_from_whoami_groups(
            "Mandatory Label\\High Mandatory Level Label S-1-16-12288"
        ));
        assert!(!commands::is_elevated_from_whoami_groups(
            "Mandatory Label\\Medium Mandatory Level Label S-1-16-8192"
        ));
    }

    #[test]
    fn st55_layout_acl_has_only_service_and_owner() {
        let release = layout::Layout::release();
        let acl = commands::acl_commands(&release, "S-1-5-21-42");
        let text = lines(&acl);
        for forbidden in ["Users", "Authenticated Users", "Everyone", "Administrators"] {
            assert!(!text.contains(forbidden));
        }
        // 置き場ごとに /reset の後に継承を切って 2 つだけを付ける
        assert_eq!(acl.len(), 6);
        for pair in acl.chunks(2) {
            assert_eq!(pair[0].args[1], "/reset");
            assert_eq!(pair[1].args[0], pair[0].args[0]);
            assert_eq!(pair[1].args[1..3], ["/inheritance:r", "/grant:r"]);
            assert_eq!(pair[1].args[4], "*S-1-5-21-42:(OI)(CI)RX");
        }
        // サービスのアカウントは SID で渡す（登録の前でも引ける。review R31）
        let server = commands::service_sid("ashiato-server");
        let db = commands::service_sid("ashiato-db");
        assert_eq!(acl[1].args[0], r"C:\ProgramData\ashiato");
        assert_eq!(acl[1].args[3], format!("*{server}:(OI)(CI)M"));
        assert!(!text.contains("NT SERVICE"));
        // DB の置き場と DB のログにはサーバのアカウントの書きが無い
        assert_eq!(acl[3].args[0], r"C:\ProgramData\ashiato\db");
        assert_eq!(acl[5].args[0], r"C:\ProgramData\ashiato\logs\db");
        for line in [&acl[3], &acl[5]] {
            assert_eq!(line.args[3], format!("*{db}:(OI)(CI)M"));
            assert!(!line.line().contains(&server));
        }
    }

    // Scenario: インストールの間、ネットワークに出ない
    #[test]
    fn st55_layout_registers_services_before_granting_their_sids() {
        let release = layout::Layout::release();
        let commands = commands::fresh_state_setup_commands(&release, "S-1-5-21-42");
        let db_registration = commands
            .iter()
            .position(|command| command == &commands::register_database_command(&release))
            .expect("DB service registration");
        let server_registration = commands
            .iter()
            .position(|command| command == &commands::create_server_command(&release))
            .expect("server service registration");
        let first_acl = commands
            .iter()
            .position(|command| command.program == "icacls")
            .expect("state ACL");

        assert!(db_registration < first_acl);
        assert!(server_registration < first_acl);
    }

    #[test]
    fn st55_layout_fresh_state_is_owned_by_administrators() {
        let command = commands::take_ownership_command(&layout::Layout::release());
        assert_eq!(
            command.line(),
            r"icacls C:\ProgramData\ashiato /setowner *S-1-5-32-544 /T /C /L"
        );
    }

    #[test]
    fn st55_layout_service_registration_uses_virtual_accounts() {
        let release = layout::Layout::release();
        let register = commands::register_database_command(&release);
        assert_eq!(
            register.program,
            r"C:\Program Files\ashiato\pgsql\bin\pg_ctl.exe"
        );
        assert!(register
            .args
            .windows(2)
            .any(|pair| pair == ["-U", r"NT SERVICE\ashiato-db"]));
        assert!(!register.args.iter().any(|arg| arg == "-P"));
        let create = commands::create_server_command(&release);
        assert_eq!(create.program, "sc.exe");
        assert_eq!(create.args[..2], ["create", "ashiato-server"]);
        assert!(create
            .args
            .windows(2)
            .any(|pair| pair == ["obj=", r"NT SERVICE\ashiato-server"]));
        assert!(create
            .args
            .windows(2)
            .any(|pair| pair == ["depend=", "ashiato-db"]));
        let text = format!(
            "{}\n{}\n{}",
            register.line(),
            create.line(),
            lines(&commands::configure_service_commands(&release))
        );
        assert!(text.contains("sc.exe config ashiato-db") && text.contains("sc.exe failure"));
        // 失敗の数え直しは 1 日（R60 の「1 日 3 回まで」はこの値に乗る。review R74）
        for service in ["ashiato-db", "ashiato-server"] {
            assert!(
                text.contains(&format!("sc.exe failure {service} reset= 86400 actions=")),
                "{service} の回復の数え直しが 1 日でない"
            );
        }
        assert!(
            text.contains("restart/10000/restart/10000/restart/10000"),
            "DB の再起動が競合しても、90 秒の実地検査中に次の回復を試せる間隔にする"
        );
        for service in ["ashiato-db", "ashiato-server"] {
            assert!(
                text.contains(&format!("sc.exe failureflag {service} 1")),
                "{service} の正常終了にも回復を有効にしていない"
            );
        }
        assert!(!text.contains("LocalSystem"));
        assert!(!text.contains("NetworkService"));
        assert!(!text.contains("password="));
    }

    #[test]
    fn st55_db_watch_acl_grants_only_query_and_start_to_server_sid() {
        let release = layout::Layout::release();
        assert_eq!(
            commands::db_service_sdshow_command(&release).line(),
            "sc.exe sdshow ashiato-db"
        );
        let ace = commands::db_watch_ace(&release);
        assert_eq!(
            ace,
            format!("(A;;LCRP;;;{})", commands::service_sid("ashiato-server"))
        );
        let set =
            commands::db_watch_sdset_command(&release, "D:(A;;CC;;;S-1-1-0)S:(AU;;FA;;;S-1-1-0)")
                .expect("ACE が無ければ sdset を起こす");
        assert_eq!(set.program, "sc.exe");
        assert_eq!(
            set.args,
            [
                "sdset".to_owned(),
                "ashiato-db".to_owned(),
                format!("D:(A;;CC;;;S-1-1-0){ace}S:(AU;;FA;;;S-1-1-0)"),
            ]
        );
        // 足すのは状態を読む（LC・RP）だけ。停止（WP）・設定変更（DC）は与えない
        assert!(!ace.contains("WP") && !ace.contains("DC"));
        // 回復の設定の命令には見張りの ACE を混ぜない（sdset は sdshow の結果を見てから起こす）
        assert!(!lines(&commands::configure_service_commands(&release)).contains("sdset"));
    }

    #[test]
    fn st55_db_watch_acl_does_not_append_existing_server_ace_twice() {
        let release = layout::Layout::release();
        let ace = commands::db_watch_ace(&release);
        // sdshow は SDDL の前に空の行を出す。最後の空でない行を読む
        let output = format!("\r\nD:(A;;CC;;;S-1-1-0){ace}S:(AU;;FA;;;S-1-1-0)\r\n\r\n");
        let current = commands::sddl_from_sdshow(&output).expect("SDDL の行");
        assert_eq!(
            current,
            format!("D:(A;;CC;;;S-1-1-0){ace}S:(AU;;FA;;;S-1-1-0)")
        );
        assert!(commands::db_watch_sdset_command(&release, current).is_none());
        assert!(commands::sddl_from_sdshow("\r\n\r\n").is_none());
    }

    #[test]
    fn st55_db_watch_acl_inserts_ace_before_sacl_and_deduplicates() {
        let sid = commands::service_sid("ashiato-server");
        let ace = format!("(A;;LCRP;;;{sid})");
        assert_eq!(
            commands::add_db_watch_ace("D:(A;;CC;;;S-1-1-0)S:(AU;;FA;;;S-1-1-0)", &ace),
            format!("D:(A;;CC;;;S-1-1-0){ace}S:(AU;;FA;;;S-1-1-0)")
        );
        assert_eq!(
            commands::add_db_watch_ace("D:(A;;CC;;;S-1-1-0)", &ace),
            format!("D:(A;;CC;;;S-1-1-0){ace}")
        );
        let existing = format!("D:{ace}S:(AU;;FA;;;S-1-1-0)");
        assert_eq!(commands::add_db_watch_ace(&existing, &ace), existing);
    }

    #[test]
    fn st55_layout_server_service_runs_installed_server_with_settings() {
        // サーバのサービスはプログラムの置き場の実行ファイルを、設定のファイルを渡して起こす（D29（仮）。review R6）
        let create = commands::create_server_command(&layout::Layout::release());
        let bin_path = &create.args[create
            .args
            .iter()
            .position(|arg| arg == "binPath=")
            .expect("binPath")
            + 1];
        assert_eq!(
            bin_path,
            r#""C:\Program Files\ashiato\ashiato-server.exe" service --settings "C:\ProgramData\ashiato\settings\server.json""#
        );
        let dev = commands::create_server_command(&layout::Layout::development());
        assert!(dev.line().contains("ashiato-server-dev"));
        assert!(dev.line().contains("depend= ashiato-db-dev"));
    }

    #[test]
    fn st55_layout_database_tools_are_called_from_program_directory() {
        let release = layout::Layout::release();
        let initdb = commands::initdb_command(
            &release,
            Path::new(r"C:\ProgramData\ashiato\settings\.initdb.pwfile"),
        );
        assert_eq!(
            initdb.program,
            r"C:\Program Files\ashiato\pgsql\bin\initdb.exe"
        );
        let psql = commands::psql_command(&release, "127.0.0.1", 55400, "postgres");
        assert_eq!(psql.program, r"C:\Program Files\ashiato\pgsql\bin\psql.exe");
        assert!(commands::repair_stop_command(&release)
            .program
            .ends_with(r"pgsql\bin\pg_ctl.exe"));
        assert_eq!(
            commands::program_files(),
            [
                "ashiato-server.exe",
                "ashiato-setup.exe",
                "db-roles.sql",
                "pgsql"
            ]
        );
    }

    #[test]
    fn st55_layout_command_line_quotes_arguments_with_spaces() {
        let command = SetupCommand::new("icacls", [r"C:\Users\A B\Downloads", "/grant:r", ""]);
        assert_eq!(
            command.line(),
            r#"icacls "C:\Users\A B\Downloads" /grant:r """#
        );
        assert_eq!(commands::windows_arg_quote(r"C:\A B"), r#""C:\A B""#);
        assert_eq!(commands::windows_arg_quote(r"C:\A\"), r#""C:\A\\""#);
        assert_eq!(commands::windows_arg_quote(r#"a"b"#), r#""a\"b""#);
    }

    #[test]
    fn st55_layout_service_state_is_read_from_number() {
        let running = "SERVICE_NAME: ashiato-db\n        TYPE               : 10  WIN32_OWN_PROCESS\n        STATE              : 4  RUNNING\n";
        assert_eq!(commands::service_state_code(running), Some(4));
        let stopped = "        STATE              : 1  STOPPED\n";
        assert_eq!(commands::service_state_code(stopped), Some(1));
        assert_eq!(commands::service_state_code("FAILED 1060"), None);
    }

    // Scenario: インストーラは状態の置き場を選ぶ口を持たない
    #[test]
    fn st55_layout_args_reject_state_directory() {
        assert!(commands::parse_args(&["install", "--state-dir", "C:\\other"]).is_err());
        assert!(commands::parse_args(&["install", "--bogus"]).is_err());
        assert!(commands::parse_args(&[
            "install",
            "--owner-sid",
            "S-1-5-21-42",
            "--profile",
            "C:\\Users\\A",
            "--downloads",
            "C:\\Downloads"
        ])
        .is_ok());
        assert!(
            commands::parse_args(&["password", "--owner-sid", "S-1-5-21-42", "--elevated"]).is_ok()
        );
    }

    #[test]
    fn st55_layout_commands_quote_paths_and_argument_list() {
        let script = commands::elevation_command(
            r"C:\Program Files\ashiato setup.exe",
            "S-1-5-21-42",
            r"C:\Users\A B",
            r"C:\Downloads,old",
        );
        assert!(script.contains(r#"-FilePath 'C:\Program Files\ashiato setup.exe'"#));
        // Start-Process は配列を空白で繋ぐだけなので、空白を含む値は " でくくって渡す
        assert!(script.contains(
            r#"@('"install"','"--owner-sid"','"S-1-5-21-42"','"--profile"','"C:\Users\A B"','"--downloads"','"C:\Downloads,old"','"--elevated"')"#
        ));
    }

    #[test]
    fn st55_db_initdb_uses_builtin_locale_checksums_and_fixed_admin() {
        let args = db::initdb_args(
            std::path::Path::new(r"C:\ProgramData\ashiato\db\17\.pwfile"),
            std::path::Path::new(r"C:\ProgramData\ashiato\db\17"),
        );
        assert!(args.windows(2).any(|pair| pair == ["-U", "ashiato"]));
        for expected in [
            "--locale-provider=builtin",
            "--builtin-locale=C.UTF-8",
            "--encoding=UTF8",
            "--data-checksums",
            "-A",
            "scram-sha-256",
        ] {
            assert!(args.iter().any(|arg| arg == expected));
        }
        assert!(!args.iter().any(|arg| arg == "trust"));
    }

    #[test]
    fn st55_db_config_does_not_enable_trust() {
        let config =
            db::postgresql_conf_append(std::path::Path::new(r"C:\ProgramData\ashiato"), 55400);
        assert!(!config.contains("trust"));
    }

    #[test]
    fn st55_db_config_binds_loopback_and_release_port() {
        let config =
            db::postgresql_conf_append(std::path::Path::new(r"C:\ProgramData\ashiato"), 55400);
        assert!(config.contains("listen_addresses='127.0.0.1'"));
        assert!(config.contains("port=55400"));
    }

    #[test]
    fn st55_db_config_redacts_sql_and_error_text() {
        let config =
            db::postgresql_conf_append(std::path::Path::new(r"C:\ProgramData\ashiato"), 55400);
        for expected in [
            "log_statement='none'",
            "log_min_duration_statement=-1",
            "log_error_verbosity='terse'",
            "log_min_error_statement='panic'",
        ] {
            assert!(config.contains(expected));
        }
    }

    #[test]
    fn st55_db_config_writes_to_state_log_directory() {
        let config =
            db::postgresql_conf_append(std::path::Path::new(r"C:\ProgramData\ashiato"), 55400);
        assert!(config.contains("logging_collector=on"));
        assert!(config.contains("log_directory='C:/ProgramData/ashiato/logs/db'"));
        assert!(config.contains("log_filename='postgresql-%a.log'"));
    }

    #[test]
    fn st55_db_config_rotates_weekday_files() {
        let config =
            db::postgresql_conf_append(std::path::Path::new(r"C:\ProgramData\ashiato"), 55400);
        assert!(config.contains("log_rotation_age=1d"));
        assert!(config.contains("log_truncate_on_rotation=on"));
    }

    #[test]
    fn st55_db_config_is_appended_once_with_layout_port() {
        let state = std::path::Path::new(r"C:\ProgramData\ashiato-dev");
        let first =
            db::with_distribution_settings("# initdb\n", state, 55401).expect("first run appends");
        assert!(first.starts_with("# initdb\n"));
        assert!(first.contains("port=55401"));
        assert!(first.contains("log_directory='C:/ProgramData/ashiato-dev/logs/db'"));
        assert_eq!(db::with_distribution_settings(&first, state, 55401), None);
    }

    #[test]
    fn st55_db_role_step_creates_database_and_keeps_secrets_out_of_arguments() {
        assert!(commands::create_database_sql().contains("CREATE DATABASE ashiato"));
        let psql =
            commands::psql_command(&layout::Layout::release(), "127.0.0.1", 55400, "ashiato");
        assert!(psql.args.windows(2).any(|pair| pair == ["-U", "ashiato"]));
        assert!(psql.args.windows(2).any(|pair| pair == ["-d", "ashiato"]));
        assert!(!psql.line().contains("PASSWORD") && !psql.line().contains("_PW"));
    }

    #[test]
    fn st55_db_pwfile_cleanup_removes_temporary_file() {
        let temp = TempDir::new().expect("temporary directory");
        let pwfile = temp.path().join("pwfile");
        fs::write(&pwfile, "x").expect("pwfile");
        db::remove_pwfile(&pwfile).expect("remove pwfile");
        assert!(!pwfile.exists());
        db::remove_pwfile(&pwfile).expect("idempotent cleanup");
    }

    #[test]
    fn st55_db_config_uses_file_logging() {
        let config =
            db::postgresql_conf_append(std::path::Path::new(r"C:\ProgramData\ashiato"), 55400);
        assert!(config.contains("log_destination='stderr'"));
        assert!(!config.contains("log_statement='all'"));
    }

    #[test]
    fn st55_db_log_path_is_under_state_directory() {
        let config =
            db::postgresql_conf_append(std::path::Path::new(r"C:\ProgramData\ashiato"), 55400);
        assert!(config.contains("C:/ProgramData/ashiato/logs/db"));
    }

    // Scenario: 配る番号は開発の台本の割り当てと被らない
    #[test]
    fn st55_port_fixed_numbers_are_outside_development_ranges() {
        assert!(commands::fixed_ports_are_outside_development_ranges());
        assert_eq!(layout::Layout::release().db_port, 55400);
        assert_eq!(layout::Layout::release().server_port, 18700);
        assert_eq!(layout::Layout::development().db_port, 55401);
        assert_eq!(layout::Layout::development().server_port, 18701);
    }

    // Scenario: 番号が塞がっていると、登録せずに止まる
    #[test]
    fn st55_port_netstat_reads_ipv4_ipv6_and_ignores_other_ports() {
        let output = " TCP 127.0.0.1:55400 0.0.0.0:0 LISTENING 42\n TCP [::1]:18700 [::]:0 LISTENING 99\n TCP 127.0.0.1:5432 0.0.0.0:0 LISTENING 7";
        assert_eq!(
            commands::parse_netstat_listener(output, 55400)
                .expect("IPv4 listener should be parsed")
                .pid,
            42
        );
        assert_eq!(
            commands::parse_netstat_listener(output, 18700)
                .expect("IPv6 listener should be parsed")
                .pid,
            99
        );
        assert!(commands::parse_netstat_listener(output, 55401).is_none());
    }

    #[test]
    fn st55_port_netstat_checks_ipv4_and_ipv6_commands() {
        assert_eq!(commands::netstat_protocols(), ["tcp", "tcpv6"]);
    }

    #[test]
    fn st55_port_netstat_reads_listener_without_english_state_word() {
        // 状態の語が表示の言語で変わっても、相手の番号 0 の行を待ち受けとして読む
        let output = "  TCP    127.0.0.1:55400    0.0.0.0:0    ABHOEREN    42\n  TCP    127.0.0.1:55400    127.0.0.1:50000    HERGESTELLT    43";
        assert_eq!(
            commands::parse_netstat_listener(output, 55400).map(|listener| listener.pid),
            Some(42)
        );
    }

    // Scenario: 最後に Tailscale の転送先を示し、Tailscale の設定は変えない
    #[test]
    fn st55_notice_mentions_forwarding_without_an_invocation() {
        let notice = commands::install_notice(18700, Some(40));
        assert!(notice.contains("無人で動かす"));
        assert!(notice.contains("tailscale serve"));
        assert!(notice.contains("127.0.0.1:18700"));
        assert!(!notice.contains("tailscale.exe"));
    }

    // Scenario: 空きが見積もりより少なければ知らせ、インストールは続ける
    #[test]
    fn st55_notice_warns_only_below_forty_gb() {
        assert!(commands::install_notice(18700, Some(39)).contains("39 GB"));
        assert!(!commands::install_notice(18700, Some(40)).contains("見積もりは 40 GB"));
    }

    #[test]
    fn st55_notice_keeps_installation_advisory_only() {
        let notice = commands::install_notice(18700, Some(10));
        assert!(notice.contains("10 GB"));
        assert!(notice.contains("見積もりは 40 GB"));
    }

    // Scenario: インストールすると「アプリと機能」に載る
    #[test]
    fn st55_uninstall_registration_has_display_and_uninstall_values() {
        let commands =
            commands::uninstall_registration_commands(&layout::Layout::release(), "0.4.0-beta.1");
        let value = |name: &str| {
            commands
                .iter()
                .find(|command| command.args[3] == name)
                .map(|command| command.args[7].clone())
        };
        assert_eq!(value("DisplayName").as_deref(), Some("ashiato"));
        // 版番号は渡した版番号（ASHIATO_VERSION）（review R27）
        assert_eq!(value("DisplayVersion").as_deref(), Some("0.4.0-beta.1"));
        assert!(value("UninstallString").is_some_and(|value| value.ends_with(" uninstall")));
    }

    // Scenario: 消すとサービスとプログラムと登録が消える
    #[test]
    fn st55_uninstall_commands_remove_services_program_and_registration() {
        let text = lines(&commands::uninstall_commands(
            &layout::Layout::release(),
            Path::new(r"C:\Program Files\ashiato\ashiato-setup.exe"),
        ));
        for expected in [
            "sc.exe delete ashiato-db",
            "sc.exe delete ashiato-server",
            "reg.exe delete",
            "Program Files\\ashiato",
        ] {
            assert!(text.contains(expected), "missing {expected}");
        }
    }

    // Scenario: 消しても記録の置き場は残り、その場所を示す
    #[test]
    fn st55_uninstall_commands_preserve_state_and_inbox() {
        let release = layout::Layout::release();
        let text = lines(&commands::uninstall_commands(
            &release,
            Path::new("setup.exe"),
        ));
        assert!(!text.contains("ProgramData"));
        assert!(!text.contains("取り込み待ち"));
        assert!(text.contains("Start-Process cmd.exe") && text.contains("'/c'"));
    }

    // 消す口の写しと台本は、管理者と SYSTEM だけが書ける新しい置き場に置く（review R47）
    #[test]
    fn st55_uninstall_copy_is_placed_where_only_administrators_write() {
        let release = layout::Layout::release();
        let directory = commands::uninstall_copy_dir(&release, "42-7");
        assert_eq!(directory, r"C:\ProgramData\ashiato-uninstall-42-7");
        assert!(!directory.contains("Temp") && !directory.contains("Users"));
        let acl = commands::admin_only_directory_acl_command(Path::new(&directory)).line();
        assert_eq!(
            acl,
            r"icacls C:\ProgramData\ashiato-uninstall-42-7 /inheritance:r /grant:r *S-1-5-32-544:(OI)(CI)F *S-1-5-18:(OI)(CI)F /L"
        );
        let copy = Path::new(r"C:\ProgramData\ashiato-uninstall-42-7\ashiato-setup-uninstall.exe");
        let text = lines(&commands::uninstall_commands(&release, copy));
        // 遅れて消す台本も同じ置き場にあり、最後に置き場ごと消える
        // 台本はインストーラが create_new で書き、命令は起こすだけ（Set-Content で上書きして使わない）
        assert!(!text.contains("Set-Content"));
        assert!(text.contains(r"C:\ProgramData\ashiato-uninstall-42-7\ashiato-setup-uninstall.cmd"));
        let script = commands::delayed_removal_script(&release, copy);
        assert!(script.contains(r#"rmdir /s /q "C:\Program Files\ashiato""#));
        assert!(script
            .contains(r#"(goto) 2>nul & rmdir /s /q "C:\ProgramData\ashiato-uninstall-42-7""#));
        assert!(!text.contains("Temp"));
        let acl = lines(&commands::uninstall_acl_commands(
            &release,
            r"C:\Users\A",
            r"C:\Users\A\Downloads",
        ));
        assert!(acl.contains(r"C:\Users\A\ashiato\取り込み待ち /remove:g"));
        assert!(acl.contains(r"C:\Users\A\Downloads /remove:g"));
        assert!(!acl.contains("/reset") && !acl.contains("del ") && !acl.contains("rmdir"));
    }

    #[test]
    fn st55_uninstall_registration_quotes_only_program_path() {
        // 引数として直に渡し、PowerShell の引用を通さない（review R18）
        let commands =
            commands::uninstall_registration_commands(&layout::Layout::release(), "0.4.0");
        let uninstall = commands
            .iter()
            .find(|command| command.args[3] == "UninstallString")
            .expect("UninstallString");
        assert_eq!(uninstall.program, "reg.exe");
        assert_eq!(
            uninstall.args[7],
            r#""C:\Program Files\ashiato\ashiato-setup.exe" uninstall"#
        );
    }

    #[test]
    fn st55_db_uses_postgresql_17_layout() {
        assert!(db::initdb_args(
            std::path::Path::new("pw"),
            std::path::Path::new(r"C:\ProgramData\ashiato\db\17")
        )
        .last()
        .is_some_and(|path| path.ends_with(r"db\17")));
    }

    // Scenario: インストールごとに秘密が違う
    #[test]
    fn st55_secret_values_are_unique_base64url() {
        let a = secrets::new_install_secrets();
        let b = secrets::new_install_secrets();
        for (left, right) in a.values().into_iter().zip(b.values()) {
            assert_ne!(left, right);
        }
        assert_eq!(
            a.values()
                .into_iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            5
        );
        for value in [
            &a.admin_password,
            &a.owner_password,
            &a.app_password,
            &a.api_token,
            &a.web_session_key,
        ] {
            assert_eq!(value.len(), 43);
            assert!(value
                .bytes()
                .all(|byte| { byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' }));
        }
    }

    // Scenario: インストーラの出力とログに秘密が出ない
    #[test]
    fn st55_secret_command_text_contains_no_secret_value() {
        // インストーラが起こす命令（記録の行）の全部に、作った秘密の値が無い
        let secrets = secrets::new_install_secrets();
        let release = layout::Layout::release();
        let settings = secrets.settings_json(
            &release,
            r"C:\Users\A",
            r"C:\Users\A\Downloads",
            "S-1-5-21-42",
        );
        let owner_settings = secrets.owner_settings_json(&release);
        let migrate = commands::migrate_command(
            &commands::server_executable(&release),
            owner_settings["DATABASE_OWNER_URL"]
                .as_str()
                .expect("owner url"),
            Path::new(r"C:\ProgramData\ashiato\backups\pre-migrate"),
            &commands::bundled_tool(&release, "pg_dump.exe"),
        );
        let mut all = commands::acl_commands(&release, "S-1-5-21-42");
        all.extend(commands::temporary_write_acl_commands(
            &release,
            "S-1-5-21-9",
        ));
        all.push(commands::initdb_command(
            &release,
            Path::new(r"C:\ProgramData\ashiato\settings\.initdb.pwfile"),
        ));
        all.push(commands::register_database_command(&release));
        all.push(commands::create_server_command(&release));
        all.extend(commands::configure_service_commands(&release));
        all.push(commands::psql_command(
            &release,
            "127.0.0.1",
            55400,
            "ashiato",
        ));
        all.push(SetupCommand::new(
            migrate.program.clone(),
            migrate.args.clone(),
        ));
        all.extend(commands::inbox_acl_commands(
            &release,
            r"C:\Users\A",
            r"C:\Users\A\Downloads",
        ));
        all.extend(commands::uninstall_registration_commands(&release, "0.4.0"));
        all.push(commands::start_server_command(&release));
        let owner_path = commands::owner_settings_path(&release);
        all.push(commands::owner_settings_acl_command(&owner_path));
        all.push(commands::owner_settings_owner_command(&owner_path));
        let text = lines(&all);
        assert!(!settings.to_string().contains(&secrets.owner_password));
        for value in secrets.values() {
            assert!(!text.contains(value));
        }
        assert!(text.contains("-U ashiato") && text.contains("admin=ashiato"));
        assert!(text.contains("--pwfile="));
    }

    // Scenario: 作った秘密でサーバの起動の検査が通る
    #[test]
    fn st55_secret_settings_have_api_and_session_keys() {
        let secrets = secrets::new_install_secrets();
        let layout = layout::Layout::release();
        let settings = secrets.settings_json(
            &layout,
            r"C:\Users\A",
            r"C:\Users\A\Downloads",
            "S-1-5-21-42",
        );
        assert_eq!(settings["API_TOKEN"], secrets.api_token);
        assert_eq!(settings["WEB_SESSION_KEY"], secrets.web_session_key);
        assert_ne!(settings["API_TOKEN"], settings["WEB_SESSION_KEY"]);
        let keys: Vec<_> = settings
            .as_object()
            .map(|object| object.keys().map(String::as_str).collect())
            .unwrap_or_default();
        assert_eq!(
            keys,
            vec![
                "API_TOKEN",
                "ASHIATO_ARCHIVE_USER_ID",
                "ASHIATO_DB_SERVICE",
                "ASHIATO_DOWNLOADS_DIR",
                "ASHIATO_INBOX_DIR",
                "ASHIATO_OWNER_SID",
                "BIND",
                "DATABASE_URL",
                "MIGRATE_BACKUP_DIR",
                "PG_DUMP",
                "WEB_PASSWORD_VERIFIER",
                "WEB_SESSION_KEY",
            ]
        );
        assert_eq!(settings["BIND"], "127.0.0.1:18700");
        // 見張りが起こす DB のサービスの名前を、置き場ごとに書く（開発版は ashiato-db-dev。review R66）
        assert_eq!(settings["ASHIATO_DB_SERVICE"], "ashiato-db");
        let dev = secrets.settings_json(
            &layout::Layout::development(),
            r"C:\Users\A",
            r"C:\Users\A\Downloads",
            "S-1-5-21-42",
        );
        assert_eq!(dev["ASHIATO_DB_SERVICE"], "ashiato-db-dev");
        assert_eq!(
            settings["ASHIATO_INBOX_DIR"],
            "C:/Users/A/ashiato/取り込み待ち"
        );
        assert_eq!(settings["ASHIATO_DOWNLOADS_DIR"], "C:/Users/A/Downloads");
        assert_eq!(
            settings["MIGRATE_BACKUP_DIR"],
            "C:/ProgramData/ashiato/backups/pre-migrate"
        );
        assert_eq!(
            settings["PG_DUMP"],
            "C:/Program Files/ashiato/pgsql/bin/pg_dump.exe"
        );
        assert_eq!(
            settings["DATABASE_URL"],
            secrets::database_url("ashiato_app", &secrets.app_password, 55400)
        );
        // 表の所有者の合言葉は server.json に無く、owner.json にだけある（D4。deep.md Q11・review R48）
        assert!(settings.get("DATABASE_OWNER_URL").is_none());
        let owner = secrets.owner_settings_json(&layout);
        assert_eq!(
            owner,
            serde_json::json!({
                "DATABASE_OWNER_URL": secrets::database_url(
                    "ashiato_owner", &secrets.owner_password, 55400
                )
            })
        );
    }

    // Scenario: 表の所有者の合言葉は、サーバのサービスが読めないファイルにだけある
    #[test]
    fn st55_owner_settings_are_only_for_administrators_and_system() {
        let release = layout::Layout::release();
        let path = commands::owner_settings_path(&release);
        assert_eq!(
            path.display().to_string(),
            r"C:\ProgramData\ashiato\settings\owner.json"
        );
        let acl = commands::owner_settings_acl_command(&path).line();
        assert_eq!(
            acl,
            r"icacls C:\ProgramData\ashiato\settings\owner.json /inheritance:r /grant:r *S-1-5-32-544:F *S-1-5-18:F /L"
        );
        // サーバ・DB のサービスのアカウントと本人に何も与えない
        for service in [release.server_service, release.db_service] {
            assert!(!acl.contains(&commands::service_sid(service)));
        }
        assert!(!acl.contains("S-1-5-21-"));
        assert_eq!(
            commands::owner_settings_owner_command(&path).line(),
            r"icacls C:\ProgramData\ashiato\settings\owner.json /setowner *S-1-5-32-544 /L"
        );
    }

    #[test]
    fn st55_inbox_acl_commands_use_downloads_and_profile_paths() {
        // Scenario: ダウンロードにはサーバのアカウントの読みだけが足される
        // Scenario: 取り込み待ちは本人のプロファイルの直下にでき、サーバが読み書きできる
        // Scenario: 取り込み待ちを消しても、サーバが作り直す
        let commands = commands::inbox_acl_commands(
            &layout::Layout::release(),
            r"C:\Users\A",
            r"C:\Users\A\OneDrive\Downloads",
        );
        let text = lines(&commands);
        assert_eq!(
            commands::inbox_dir(r"C:\Users\A\"),
            r"C:\Users\A\ashiato\取り込み待ち"
        );
        let server = commands::service_sid("ashiato-server");
        assert!(text.contains(&format!(
            r"icacls C:\Users\A\OneDrive\Downloads /grant:r *{server}:(OI)(CI)RX"
        )));
        assert!(text.contains(&format!(
            r"icacls C:\Users\A\ashiato\取り込み待ち /grant:r *{server}:(OI)(CI)M"
        )));
        assert!(text.contains("(AD,RX)"));
        assert!(!text.contains("Documents") && !text.contains("Desktop"));
    }

    #[test]
    fn st55_inbox_expands_one_drive_downloads_without_syncing_inbox() {
        // Scenario: ダウンロードに置いた Takeout が、インストールした形で取り込まれる
        let commands = commands::inbox_acl_commands(
            &layout::Layout::release(),
            r"C:\Users\A",
            r"C:\Users\A\OneDrive\Downloads",
        );
        let text = lines(&commands);
        assert!(text.contains(r"C:\Users\A\OneDrive\Downloads"));
        assert!(!text.contains(r"C:\Users\A\OneDrive\ashiato"));
    }

    #[test]
    fn st55_inbox_parent_acl_allows_only_recreation_and_reading() {
        // Scenario: 取り込み待ちを消しても、サーバが作り直す
        let commands = commands::inbox_acl_commands(
            &layout::Layout::release(),
            r"C:\Users\A",
            r"C:\Users\A\Downloads",
        );
        let parent = commands[2].line();
        let recreate = commands[3].line();
        assert!(recreate.contains(r"C:\Users\A\ashiato /grant *S-1-3-0:(OI)(CI)(IO)M"));
        assert!(parent.contains(r"C:\Users\A\ashiato /grant:r"));
        assert!(parent.contains("(AD,RX)"));
        assert!(!parent.contains("(OI)(CI)M"));
    }

    #[test]
    fn st55_rerun_reads_existing_database_secrets_from_settings() {
        let original = secrets::new_install_secrets();
        let settings = original.settings_json(
            &layout::Layout::release(),
            r"C:\Users\A",
            r"C:\Users\A\Downloads",
            "S-1-5-21-42",
        );
        let owner = original.owner_settings_json(&layout::Layout::release());
        let rerun = secrets::InstallSecrets::from_settings(&settings, &owner, "new-admin".into())
            .expect("existing settings are reusable");
        // 所有者の合言葉は owner.json からだけ読む（server.json に無い）
        assert!(secrets::InstallSecrets::from_settings(
            &settings,
            &serde_json::json!({}),
            "new-admin".into()
        )
        .is_err());
        assert_eq!(rerun.owner_password, original.owner_password);
        assert_eq!(rerun.app_password, original.app_password);
        assert_eq!(rerun.api_token, original.api_token);
        assert_eq!(rerun.web_session_key, original.web_session_key);
        assert_eq!(rerun.admin_password, "new-admin");
    }

    #[test]
    fn st55_secret_migrate_command_uses_absolute_backup_paths() {
        let command = commands::migrate_command(
            std::path::Path::new(r"C:\Program Files\ashiato\ashiato-server.exe"),
            &secrets::database_url("ashiato_owner", "owner-secret", 55400),
            std::path::Path::new(r"C:\ProgramData\ashiato\backups\pre-migrate"),
            std::path::Path::new(r"C:\Program Files\ashiato\pgsql\bin\pg_dump.exe"),
        );
        assert_eq!(
            command.program,
            r"C:\Program Files\ashiato\ashiato-server.exe"
        );
        assert_eq!(command.args, ["migrate"]);
        assert_eq!(
            command.env[0],
            (
                "DATABASE_OWNER_URL".into(),
                secrets::database_url("ashiato_owner", "owner-secret", 55400)
            )
        );
        assert_eq!(
            command.env[1],
            (
                "MIGRATE_BACKUP_DIR".into(),
                "C:\\ProgramData\\ashiato\\backups\\pre-migrate".into()
            )
        );
        assert_eq!(
            command.env[2],
            (
                "PG_DUMP".into(),
                "C:\\Program Files\\ashiato\\pgsql\\bin\\pg_dump.exe".into()
            )
        );
        assert!(command.args.iter().all(|arg| !arg.contains("owner-secret")));
    }

    // Scenario: 実行時の役割は管理者でも表の所有者でもない
    #[test]
    fn st55_secret_runtime_database_role_is_app_role() {
        assert_eq!(db::runtime_role(), "ashiato_app");
        assert_ne!(db::runtime_role(), "ashiato");
        assert_ne!(db::runtime_role(), "ashiato_owner");
    }

    // Scenario: 移行の前の退避は状態の置き場に、同梱の pg_dump で取られる
    #[test]
    fn st55_secret_backup_command_points_at_bundled_dump() {
        let command = commands::migrate_command(
            std::path::Path::new(r"C:\server.exe"),
            &secrets::database_url("owner", "secret", 5432),
            std::path::Path::new(r"C:\ProgramData\ashiato\backups\pre-migrate"),
            std::path::Path::new(r"C:\pgsql\bin\pg_dump.exe"),
        );
        assert_eq!(command.args, ["migrate"]);
        assert_eq!(command.env[1].0, "MIGRATE_BACKUP_DIR");
        assert!(command.env[1].1.starts_with(r"C:\ProgramData\"));
        assert_eq!(command.env[2].0, "PG_DUMP");
        assert!(command.env[2].1.starts_with(r"C:\"));
        assert!(command
            .env
            .iter()
            .all(|(_, value)| !value.contains("%TEMP%")));
    }

    #[test]
    fn st55_secret_service_command_does_not_migrate() {
        let release = layout::Layout::release();
        assert_eq!(
            commands::start_server_command(&release).line(),
            "sc.exe start ashiato-server"
        );
        let create = commands::create_server_command(&release).line();
        assert!(create.contains("service --settings") && !create.contains("migrate"));
    }

    // Scenario: 走らせ直しても秘密と DB は変わらない
    #[test]
    fn st55_rerun_all_state_combinations_choose_a_safe_phase() {
        for settings in [false, true] {
            for database in [false, true] {
                for verifier in [false, true] {
                    for services in [false, true] {
                        let decision = state::decide_install(state::InstallState {
                            settings,
                            database,
                            verifier,
                            services,
                        });
                        let expected = if database && !settings {
                            state::InstallDecision::RepairRequired
                        } else if !settings {
                            state::InstallDecision::Continue(state::InstallPhase::WriteSecrets)
                        } else if !database {
                            state::InstallDecision::Continue(state::InstallPhase::InitDatabase)
                        } else if !services {
                            state::InstallDecision::Continue(state::InstallPhase::RegisterServices)
                        } else if !verifier {
                            state::InstallDecision::Continue(state::InstallPhase::Password)
                        } else {
                            state::InstallDecision::Continue(state::InstallPhase::Complete)
                        };
                        assert_eq!(decision, expected);
                    }
                }
            }
        }
    }

    // Scenario: DB を作る途中で落ちても、走らせ直しで DB ができる
    #[test]
    fn st55_rerun_missing_database_resumes_at_initdb() {
        let decision = state::decide_install(state::InstallState {
            settings: true,
            database: false,
            verifier: false,
            services: false,
        });
        assert_eq!(
            decision,
            state::InstallDecision::Continue(state::InstallPhase::InitDatabase)
        );
    }

    // Scenario: DB はあるのに秘密が無いと、何も書き換えずに止まる
    #[test]
    fn st55_rerun_database_without_settings_is_read_only() {
        let decision = state::decide_install(state::InstallState {
            settings: false,
            database: true,
            verifier: false,
            services: true,
        });
        assert_eq!(decision, state::InstallDecision::RepairRequired);
        assert!(
            state::repair_message(std::path::Path::new(r"C:\ProgramData\ashiato"))
                .contains("ashiato-setup.exe repair")
        );
    }

    // Scenario: 合言葉を決める前に閉じたら、走らせ直しで合言葉の段からやり直す
    #[test]
    fn st55_rerun_missing_verifier_resumes_at_password() {
        let decision = state::decide_install(state::InstallState {
            settings: true,
            database: true,
            verifier: false,
            services: true,
        });
        assert_eq!(
            decision,
            state::InstallDecision::Continue(state::InstallPhase::Password)
        );
    }

    // Scenario: 別の利用者が走らせ直しても、本人は変わらない
    #[test]
    fn st55_rerun_preserves_owner_sid() {
        assert_eq!(
            state::owner_sid_for_rerun(Some("S-1-5-21-owner"), "S-1-5-21-other"),
            "S-1-5-21-owner"
        );
        assert_eq!(
            state::owner_sid_for_rerun(Some(""), "S-1-5-21-first"),
            "S-1-5-21-first"
        );
        assert_eq!(
            state::owner_sid_for_rerun(None, "S-1-5-21-first"),
            "S-1-5-21-first"
        );
    }

    // Scenario: 設定を退けて止まった後に repair すると、記録を残したままログインできる
    #[test]
    fn st55_repair_only_accepts_stopped_state() {
        assert!(state::repair_allowed(true, false, true));
        assert!(!state::repair_allowed(true, true, true));
        assert!(!state::repair_allowed(false, false, true));
        // 役割の完了記録が無い DB は、設定があっても repair で役割を作り直せる（消させない。review R40）
        assert!(state::repair_allowed(true, true, false));
        let message = state::roles_missing_message(Path::new(r"C:\ProgramData\ashiato\db\17"));
        assert!(message.contains("消しません") && message.contains("repair"));
        assert!(!message.contains("記録はまだありません"));
    }

    // Scenario: repair の後は前の API の合言葉では送れず、繋ぎ直しの案内が出る
    #[test]
    fn st55_repair_output_is_secret_free_and_mentions_reconnect_docs() {
        let output = state::repair_completion_message();
        assert!(output.contains("収集アプリ"));
        assert!(output.contains("docs/install.md"));
        assert!(!output.contains("token") && !output.contains("password"));
    }

    // Scenario: 止まった状態でなければ repair は何も変えずに終わる
    #[test]
    fn st55_repair_rejects_non_stopped_state_without_writing() {
        assert!(!state::repair_allowed(false, true, true));
        assert!(!state::repair_allowed(false, false, false));
        assert!(!state::repair_allowed(true, true, true));
    }

    // Scenario: repair の後に合言葉なしの入口と一時の番号が残らない
    #[test]
    fn st55_repair_commands_close_temporary_entry() {
        let start = commands::repair_start_command(
            &layout::Layout::release(),
            std::path::Path::new(r"C:\ProgramData\ashiato\settings\repair.pg_hba.conf"),
            std::path::Path::new(r"C:\ProgramData\ashiato\settings\repair.pg_ident.conf"),
            std::path::Path::new(r"C:\ProgramData\ashiato\settings\repair-db.log"),
            49152,
        )
        .line();
        // 一時の起動はログを集める子を起こさず、上がった利用者が書ける置き場へ書く（logs\db は書けない。review R32）
        assert!(start.contains(r"-l C:\ProgramData\ashiato\settings\repair-db.log"));
        assert!(start.contains("-c logging_collector=off"));
        assert!(start.contains(r#"pg_ctl.exe" start"#) && start.contains("hba_file="));
        assert!(start.contains("-p 49152"));
        assert!(start.contains("C:/ProgramData/ashiato/settings/repair.pg_hba.conf"));
        assert!(!start.contains("password") && !start.contains("token"));
        let hba = commands::repair_hba_contents();
        assert_eq!(hba.matches("sspi").count(), 1);
        assert!(!hba.contains("trust"));
        // SSPI が渡す名前は user@DOMAIN（include_realm=1）。大文字と小文字は区別しない
        assert_eq!(
            commands::repair_ident_contents("CONTOSO\\alice.b"),
            "repair /(?i)^alice\\u002Eb@CONTOSO$ ashiato\n"
        );
        // `^` を括弧でくくると `[^]`（ARE で壊れた式）になる。空白も行の区切りにしない（review R58）
        let caret = commands::repair_ident_contents("PC\\a^b c");
        assert_eq!(caret, "repair /(?i)^a\\u005Eb\\u0020c@PC$ ashiato\n");
        assert!(!caret.contains("[^]") && caret.trim_end().split(' ').count() == 3);
        let stop = commands::stop_service_and_wait_command("ashiato-db").line();
        assert!(stop.contains("Get-Service") && stop.contains("sc.exe stop 'ashiato-db'"));
    }

    // Scenario: 別の管理者が repair しても、本人は変わらない
    #[test]
    fn st55_repair_keeps_owner_from_acl_not_invoking_user() {
        // Get-Acl を SID に直した形（サービスの SID は本人の候補にしない）
        assert_eq!(
            state::owner_sid_from_acl("S-1-5-80-1-2-3-4-5\nS-1-5-21-owner\n"),
            Ok("S-1-5-21-owner".to_owned())
        );
        assert_eq!(
            state::owner_sid_from_acl("NT SERVICE\\ashiato-db:(M)\nS-1-5-21-owner:(RX)"),
            Ok("S-1-5-21-owner".to_owned())
        );
        assert_eq!(
            state::owner_sid_from_acl("S-1-5-21-owner:(RX)\nS-1-5-21-other:(RX)"),
            Err(state::RepairRefusal::OwnerAmbiguous)
        );
    }

    // Scenario: repair の途中で落ちても、もう一度 repair できる
    #[test]
    fn st55_repair_cleans_leftover_temporary_server_first() {
        let text = lines(&commands::repair_cleanup_commands(
            &layout::Layout::release(),
        ));
        assert!(text.contains("postmaster.pid"));
        assert!(text.contains(r"& 'C:\Program Files\ashiato\pgsql\bin\pg_ctl.exe' stop"));
        assert!(text.contains("Get-Service"));
    }

    #[test]
    fn st55_rerun_temporary_write_targets_installer_owned_places_only() {
        // 一時の書きは、インストーラだけが書く置き場（設定・移行の前の退避）に足し、/reset で外す（D21）
        let release = layout::Layout::release();
        assert_eq!(
            commands::temporary_write_paths(&release),
            [
                r"C:\ProgramData\ashiato\settings".to_owned(),
                r"C:\ProgramData\ashiato\backups\pre-migrate".to_owned()
            ]
        );
        let grant = commands::temporary_write_acl_commands(&release, "S-1-5-21-admin");
        assert_eq!(grant.len(), 8);
        for (commands, path) in grant
            .chunks(4)
            .zip(commands::temporary_write_paths(&release))
        {
            // 別管理者 B は所有者になっても `icacls /grant` を実行できなかった。最初の ACL 更新は
            // 同じプロセスが backup/restore 用のハンドルで既存 DACL に ACE を加える。この形なら
            // 特権を有効にした親から、通常のハンドルで開く子プロセスへ失敗を持ち込まない。
            assert_eq!(commands[0].program, "powershell.exe");
            assert!(commands[0]
                .args
                .last()
                .is_some_and(|script| script.contains("SeRestorePrivilege")));
            assert!(commands[0]
                .args
                .last()
                .is_some_and(|script| script.contains("SeBackupPrivilege")));
            for required in [
                "FILE_FLAG_BACKUP_SEMANTICS",
                "GetSecurityInfo",
                "SetEntriesInAcl",
                "SetSecurityInfo",
            ] {
                assert!(commands[0]
                    .args
                    .last()
                    .is_some_and(|script| script.contains(required)));
            }
            assert!(commands[0]
                .args
                .last()
                .is_some_and(|script| !script.contains("icacls.exe")));
            assert_eq!(commands[1].program, "takeown");
            assert_eq!(commands[1].args, ["/F", &path, "/A", "/R", "/D", "Y"]);
            assert_eq!(
                commands[2].args[1..],
                ["/grant", "*S-1-5-21-admin:(OI)(CI)M", "/L"]
            );
            // 一時の書込みを渡した後だけ Administrators を持ち主へ戻す。
            assert_eq!(commands[3].program, "takeown");
            assert_eq!(commands[3].args, ["/F", &path, "/A", "/R", "/D", "Y"]);
        }
        let remove = lines(&commands::remove_temporary_write_acl_commands(&release));
        assert!(remove.contains("settings /reset"));
        assert!(!remove.contains("S-1-5-21-admin"));
        assert!(!remove.contains("S-1-5-32-544"));
    }

    #[test]
    fn st55_repair_acl_cleanup_preserves_owner_read() {
        let release = layout::Layout::release();
        let other = lines(&commands::remove_database_write_acl_commands(
            &release,
            "S-1-5-21-admin",
            "S-1-5-21-owner",
        ));
        assert!(other.contains(&format!(
            "*{}:(OI)(CI)M",
            commands::service_sid("ashiato-db")
        )));
        assert!(other.contains("*S-1-5-21-owner:(OI)(CI)RX"));
        assert!(other.contains("/remove:g *S-1-5-21-admin"));
    }

    #[test]
    fn st55_repair_acl_cleanup_keeps_owner_rx_when_owner_repairs() {
        let text = lines(&commands::remove_database_write_acl_commands(
            &layout::Layout::release(),
            "S-1-5-21-owner",
            "S-1-5-21-owner",
        ));
        assert!(text.contains("/grant:r") && text.contains("*S-1-5-21-owner:(OI)(CI)RX"));
        assert!(!text.contains("/remove:g"));
    }

    #[test]
    fn st55_other_owner_install_lets_the_admin_traverse_the_state_root_only_while_writing() {
        // Scenario: 上がった方が別の管理者でも、本人は上がる前に控えた利用者になる
        // 本人と違う管理者は根の ACL に載らず、initdb が根を見られずに落ちた（run 42209316618）。
        // DB を書く間だけ根の RX（継がない）を足し、書き終えたら外す
        let release = layout::Layout::release();
        let grant =
            commands::database_write_acl_commands(&release, "S-1-5-21-admin", "S-1-5-21-owner");
        assert_eq!(grant.len(), 2);
        let root = grant[0].args.last().expect("PowerShell script");
        assert!(root.contains(r"'C:\ProgramData\ashiato'"));
        assert!(root.contains("'S-1-5-21-admin', 1179817, 1, 0)"));
        let remove = commands::remove_database_write_acl_commands(
            &release,
            "S-1-5-21-admin",
            "S-1-5-21-owner",
        );
        let revoke = remove
            .last()
            .expect("revoke")
            .args
            .last()
            .expect("PowerShell script");
        assert!(revoke.contains("'S-1-5-21-admin', 0, 4, 0)"));
        // 本人と同じなら根には触れない（外すときに本人の ACE まで外さない）
        let same = commands::remove_database_write_acl_commands(
            &release,
            "S-1-5-21-owner",
            "S-1-5-21-owner",
        );
        assert!(same.iter().all(|c| c.program != "powershell.exe"));
        assert_eq!(
            commands::database_write_acl_commands(&release, "S-1-5-21-owner", "S-1-5-21-owner")
                .len(),
            1
        );
    }

    #[test]
    fn st55_repair_database_write_grant_uses_restore_privilege() {
        // 別管理者は DB の DACL を通常の icacls では開けない。repair が一時の M を
        // 足す最初の更新は、既存の設定・退避場所と同じ特権付きハンドルで行う。
        let commands = commands::database_write_acl_commands(
            &layout::Layout::release(),
            "S-1-5-21-admin",
            "S-1-5-21-admin",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].program, "powershell.exe");
        let script = commands[0].args.last().expect("PowerShell script");
        assert!(script.contains("SeRestorePrivilege"));
        assert!(script.contains("SeBackupPrivilege"));
        assert!(script.contains(r"C:\ProgramData\ashiato\db"));
        assert!(script.contains("S-1-5-21-admin"));
        assert!(!script.contains("icacls.exe"));
    }
}
