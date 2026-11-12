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
        Self {
            db_service: "ashiato-db",
            server_service: "ashiato-server",
            updater_service: "ashiato-updater",
            program_dir: r"C:\Program Files\ashiato",
            state_dir: r"C:\ProgramData\ashiato",
            updater_dir: r"C:\ProgramData\ashiato-updater",
            db_dir: r"C:\ProgramData\ashiato\db\17",
            db_port: 55400,
            server_port: 18700,
        }
    }

    pub const fn development() -> Self {
        Self {
            db_service: "ashiato-db-dev",
            server_service: "ashiato-server-dev",
            updater_service: "ashiato-updater-dev",
            program_dir: r"C:\Program Files\ashiato-dev",
            state_dir: r"C:\ProgramData\ashiato-dev",
            updater_dir: r"C:\ProgramData\ashiato-updater-dev",
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
