use std::fs;
use std::io;
use std::path::Path;

pub const ADMIN_ROLE: &str = "ashiato";
pub const RUNTIME_ROLE: &str = "ashiato_app";

pub fn runtime_role() -> &'static str {
    RUNTIME_ROLE
}

pub fn initdb_args(pwfile: &Path, data_dir: &Path) -> Vec<String> {
    vec![
        "-U".into(),
        ADMIN_ROLE.into(),
        "--locale-provider=builtin".into(),
        "--builtin-locale=C.UTF-8".into(),
        "--encoding=UTF8".into(),
        "--data-checksums".into(),
        "-A".into(),
        "scram-sha-256".into(),
        format!("--pwfile={}", pwfile.display()),
        "-D".into(),
        data_dir.display().to_string(),
    ]
}

/// `initdb` の後・DB の起動の前に `postgresql.conf` の末尾へ足す行（D6・D17）。
pub fn postgresql_conf_append(state_dir: &Path, port: u16) -> String {
    let log_dir = state_dir.join("logs").join("db");
    let log_dir = log_dir.to_string_lossy().replace('\\', "/");
    format!(
        "\n# ashiato distribution database\nlisten_addresses='127.0.0.1'\nport={port}\nlog_statement='none'\nlog_min_duration_statement=-1\nlog_error_verbosity='terse'\nlog_min_error_statement='panic'\nlogging_collector=on\nlog_destination='stderr'\nlog_directory='{log_dir}'\nlog_filename='postgresql-%a.log'\nlog_rotation_age=1d\nlog_truncate_on_rotation=on\n"
    )
}

/// 足す行の見出し。走らせ直しで 2 回足さないために見る。
pub const CONF_MARKER: &str = "# ashiato distribution database";

/// `postgresql.conf` の中身に、まだ足していなければ足した後の中身を返す。
pub fn with_distribution_settings(current: &str, state_dir: &Path, port: u16) -> Option<String> {
    if current.contains(CONF_MARKER) {
        return None;
    }
    Some(format!(
        "{current}{}",
        postgresql_conf_append(state_dir, port)
    ))
}

pub fn remove_pwfile(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
