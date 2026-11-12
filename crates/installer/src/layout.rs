#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub db_service: &'static str,
    pub server_service: &'static str,
    pub updater_service: &'static str,
    pub program_dir: &'static str,
    pub state_dir: &'static str,
    pub updater_dir: &'static str,
    pub db_dir: &'static str,
    pub db_port: u16,
    pub server_port: u16,
}

impl Layout {
    pub const fn release() -> Self {
        let updater = ashiato_update_check::updater_layout(false);
        Self {
            db_service: "ashiato-db",
            server_service: "ashiato-server",
            updater_service: updater.service,
            program_dir: r"C:\Program Files\ashiato",
            state_dir: r"C:\ProgramData\ashiato",
            updater_dir: updater.directory,
            db_dir: r"C:\ProgramData\ashiato\db\17",
            db_port: 55400,
            server_port: 18700,
        }
    }

    pub const fn development() -> Self {
        let updater = ashiato_update_check::updater_layout(true);
        Self {
            db_service: "ashiato-db-dev",
            server_service: "ashiato-server-dev",
            updater_service: updater.service,
            program_dir: r"C:\Program Files\ashiato-dev",
            state_dir: r"C:\ProgramData\ashiato-dev",
            updater_dir: updater.directory,
            db_dir: r"C:\ProgramData\ashiato-dev\db\17",
            db_port: 55401,
            server_port: 18701,
        }
    }
}

pub fn for_version(version: &str) -> Layout {
    if version.contains("-dev.") {
        Layout::development()
    } else {
        Layout::release()
    }
}
