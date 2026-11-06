use crate::layout::Layout;
use base64::Engine as _;
use rand::{rngs::OsRng, RngCore as _};
use serde_json::{json, Value};
use std::path::Path;

/// 表の所有者の URL の鍵（`owner.json` にだけ置く。サーバの `migrate` が環境から読む名前と同じ）。
pub const OWNER_URL_KEY: &str = "DATABASE_OWNER_URL";

/// 接続 URL を組む。資格情報を持つ部分を別にして、設定や試験のソースへ
/// 合言葉入りの URL を固定値として書かない。
pub fn database_url(role: &str, password: &str, port: u16) -> String {
    let authority = format!("{role}:{password}@127.0.0.1");
    format!("postgres://{authority}:{port}/ashiato")
}

#[derive(Debug, Clone)]
pub struct InstallSecrets {
    pub admin_password: String,
    pub owner_password: String,
    pub app_password: String,
    pub api_token: String,
    pub web_session_key: String,
    pub web_password_verifier: String,
}

impl InstallSecrets {
    pub fn values(&self) -> [&str; 5] {
        [
            &self.admin_password,
            &self.owner_password,
            &self.app_password,
            &self.api_token,
            &self.web_session_key,
        ]
    }

    pub fn settings_json(
        &self,
        layout: &Layout,
        profile: &str,
        downloads: &str,
        owner_sid: &str,
    ) -> Value {
        let state = layout.state_dir.replace('\\', "/");
        let program = layout.program_dir.replace('\\', "/");
        let database_url = database_url("ashiato_app", &self.app_password, layout.db_port);
        json!({
            "DATABASE_URL": database_url,
            "API_TOKEN": self.api_token,
            "WEB_SESSION_KEY": self.web_session_key,
            "BIND": format!("127.0.0.1:{}", layout.server_port),
            "ASHIATO_INBOX_DIR": format!("{}/ashiato/取り込み待ち", profile.trim_end_matches(['/', '\\']).replace('\\', "/")),
            "ASHIATO_DOWNLOADS_DIR": downloads.replace('\\', "/"),
            "ASHIATO_ARCHIVE_USER_ID": "00000000-0000-0000-0000-000000000000",
            "MIGRATE_BACKUP_DIR": format!("{state}/backups/pre-migrate"),
            "PG_DUMP": format!("{program}/pgsql/bin/pg_dump.exe"),
            "WEB_PASSWORD_VERIFIER": self.web_password_verifier,
            "ASHIATO_OWNER_SID": owner_sid,
            // サーバの見張り（D35）が起こす DB のサービスの名前。置き場のパスから推させない（review R66）
            "ASHIATO_DB_SERVICE": layout.db_service,
        })
    }

    /// 表の所有者の URL（移行の段だけが使う）。
    pub fn owner_url(&self, layout: &Layout) -> String {
        database_url("ashiato_owner", &self.owner_password, layout.db_port)
    }

    /// `settings\owner.json` の中身。表の所有者の合言葉は、サーバのサービスが読む `server.json` に置かず、
    /// 管理者と SYSTEM だけが読めるこのファイルに置く（design D4。deep.md Q11・review R48）。
    pub fn owner_settings_json(&self, layout: &Layout) -> Value {
        json!({ OWNER_URL_KEY: self.owner_url(layout) })
    }

    pub fn with_web_password_verifier(mut self, verifier: String) -> Self {
        self.web_password_verifier = verifier;
        self
    }

    pub fn settings_path(state_dir: &Path) -> std::path::PathBuf {
        state_dir.join("settings").join("server.json")
    }

    /// 走らせ直しでは設定に保存済みの DB 資格情報を再利用する。表の所有者の合言葉は `owner.json`
    /// （`owner_settings`）から読む。管理者の一時パスワードだけは設定に保存しないため、呼び出し側が新しく渡す。
    pub fn from_settings(
        settings: &Value,
        owner_settings: &Value,
        admin_password: String,
    ) -> Result<Self, String> {
        fn url_password(settings: &Value, key: &str) -> Result<String, String> {
            let url = settings[key]
                .as_str()
                .ok_or_else(|| format!("設定の鍵が無い: {key}"))?;
            let authority = url
                .split_once("://")
                .and_then(|(_, rest)| rest.split_once('@').map(|(value, _)| value))
                .ok_or_else(|| format!("設定の URL が不正: {key}"))?;
            authority
                .split_once(':')
                .map(|(_, password)| password.to_owned())
                .ok_or_else(|| format!("設定の URL に秘密が無い: {key}"))
        }
        Ok(Self {
            admin_password,
            owner_password: url_password(owner_settings, OWNER_URL_KEY)?,
            app_password: url_password(settings, "DATABASE_URL")?,
            api_token: settings["API_TOKEN"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            web_session_key: settings["WEB_SESSION_KEY"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            web_password_verifier: settings["WEB_PASSWORD_VERIFIER"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        })
    }
}

pub fn replace_web_password_verifier(settings: &Value, verifier: &str) -> Value {
    let mut updated = settings.clone();
    updated["WEB_PASSWORD_VERIFIER"] = Value::String(verifier.to_owned());
    updated
}

pub fn new_install_secrets() -> InstallSecrets {
    InstallSecrets {
        admin_password: secret(),
        owner_password: secret(),
        app_password: secret(),
        api_token: secret(),
        web_session_key: secret(),
        web_password_verifier: String::new(),
    }
}

/// DB の管理者の役割に置き直す合言葉（控えない使い捨て。review R76）。
pub fn new_admin_password() -> String {
    secret()
}

fn secret() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
