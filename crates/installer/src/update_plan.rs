use crate::commands::SetupCommand;
use crate::layout::{for_version, Layout};
use serde_json::json;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionAction {
    Rerun,
    Update,
    LeaveAlone,
}

pub fn version_action(installed: &str, candidate: &str) -> VersionAction {
    if ashiato_build_stamp::channel(installed) != ashiato_build_stamp::channel(candidate) {
        VersionAction::LeaveAlone
    } else if installed == candidate {
        VersionAction::Rerun
    } else {
        VersionAction::Update
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateStage {
    Started,
    Stopped,
    Moved,
    Replaced,
    Migrating,
    Starting,
    Listening,
    Restored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStep {
    RestoreDatabase,
    RestoreProgram,
    StartPrevious,
    Finish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupState {
    Completed,
    PartialOnly,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupEntryKind {
    CompletedDump,
    PartialDump,
    FailedDatabase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryPlan {
    restore_db: bool,
    steps: Vec<RecoveryStep>,
}

impl RecoveryPlan {
    pub const fn restore_database(&self) -> bool {
        self.restore_db
    }

    pub fn steps(&self) -> &[RecoveryStep] {
        &self.steps
    }
}

pub fn recovery_plan(stage: UpdateStage, exit_code: i32, backup: BackupState) -> RecoveryPlan {
    let restore_db = match stage {
        UpdateStage::Starting => backup == BackupState::Completed && !matches!(exit_code, 2..=4),
        UpdateStage::Migrating => {
            backup == BackupState::Completed && !matches!(exit_code, 0 | 2..=4)
        }
        _ => false,
    };
    let steps = if stage == UpdateStage::Listening {
        vec![RecoveryStep::Finish]
    } else if stage == UpdateStage::Restored {
        vec![RecoveryStep::StartPrevious]
    } else {
        let mut steps = Vec::new();
        if restore_db {
            steps.push(RecoveryStep::RestoreDatabase);
        }
        if matches!(
            stage,
            UpdateStage::Moved
                | UpdateStage::Replaced
                | UpdateStage::Migrating
                | UpdateStage::Starting
        ) {
            steps.extend([RecoveryStep::RestoreProgram, RecoveryStep::StartPrevious]);
        } else if matches!(stage, UpdateStage::Started | UpdateStage::Stopped) {
            steps.push(RecoveryStep::StartPrevious);
        }
        steps
    };
    RecoveryPlan { restore_db, steps }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupEntry {
    pub run_id: Option<String>,
    pub name: String,
    pub kind: BackupEntryKind,
}

impl BackupEntry {
    pub fn completed(run_id: &str, name: &str) -> Self {
        Self {
            run_id: Some(run_id.into()),
            name: name.into(),
            kind: BackupEntryKind::CompletedDump,
        }
    }
    pub fn partial(run_id: &str, name: &str) -> Self {
        Self {
            run_id: Some(run_id.into()),
            name: name.into(),
            kind: BackupEntryKind::PartialDump,
        }
    }
    pub fn direct(name: &str) -> Self {
        Self {
            run_id: None,
            name: name.into(),
            kind: if name.starts_with("ashiato_failed_") {
                BackupEntryKind::FailedDatabase
            } else {
                BackupEntryKind::CompletedDump
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRetention {
    pub keep: Vec<String>,
    pub remove: Vec<String>,
    pub drop_databases: Vec<String>,
}

pub fn classify_backup_state(current_run: &str, entries: &[BackupEntry]) -> BackupState {
    if entries.iter().any(|entry| {
        entry.run_id.as_deref() == Some(current_run) && entry.kind == BackupEntryKind::CompletedDump
    }) {
        BackupState::Completed
    } else if entries.iter().any(|entry| {
        entry.run_id.as_deref() == Some(current_run) && entry.kind == BackupEntryKind::PartialDump
    }) {
        BackupState::PartialOnly
    } else {
        BackupState::None
    }
}

pub fn retain_backups(current_run: &str, entries: &[BackupEntry]) -> BackupRetention {
    let mut keep = Vec::new();
    let mut remove = Vec::new();
    let mut drop_databases = Vec::new();
    for entry in entries {
        if entry.kind == BackupEntryKind::FailedDatabase {
            drop_databases.push(entry.name.clone());
            continue;
        }
        let is_current_complete = entry.run_id.as_deref() == Some(current_run)
            && entry.kind == BackupEntryKind::CompletedDump;
        if is_current_complete {
            keep.push(format!("{current_run}/{}", entry.name));
        } else {
            remove.push(entry.run_id.as_ref().map_or_else(
                || entry.name.clone(),
                |run_id| format!("{run_id}/{}", entry.name),
            ));
        }
    }
    BackupRetention {
        keep,
        remove,
        drop_databases,
    }
}

pub fn pg_major_matches(pg_version: &str, expected: u32) -> bool {
    pg_version
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
        == Some(expected)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasonKind {
    DbAhead,
    LedgerUnreadable,
    ChannelMismatch,
    DbNotLoopback,
    BackupFailed,
    ConnectFailed,
    ServerNotListening,
    Aborted,
    PgMajorMismatch,
}

impl ReasonKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DbAhead => "db_ahead",
            Self::LedgerUnreadable => "ledger_unreadable",
            Self::ChannelMismatch => "channel_mismatch",
            Self::DbNotLoopback => "db_not_loopback",
            Self::BackupFailed => "backup_failed",
            Self::ConnectFailed => "connect_failed",
            Self::ServerNotListening => "server_not_listening",
            Self::Aborted => "aborted",
            Self::PgMajorMismatch => "pg_major_mismatch",
        }
    }
    pub const fn description(self) -> &'static str {
        match self {
            Self::DbAhead => "DB に新しい版の知らない版がある",
            Self::LedgerUnreadable => "版の記録を読めない",
            Self::ChannelMismatch => "DB の系統が違う",
            Self::DbNotLoopback => "DB の接続先が手元でない",
            Self::BackupFailed => "移行の前の退避に失敗した",
            Self::ConnectFailed => "DB に繋げない",
            Self::ServerNotListening => "新しい版のサーバが起きない",
            Self::Aborted => "移行が途中で終わった",
            Self::PgMajorMismatch => "同梱の PostgreSQL の大きい版が違う",
        }
    }
}

pub fn reason_for_kind(kind: ReasonKind) -> Reason {
    Reason::kind(kind, "", None)
}

pub fn reason_for_sqlstate(sqlstate: &str) -> (&str, &'static str) {
    let class = sqlstate.get(..2).unwrap_or("");
    (
        class,
        match class {
            "23" => "制約に反した",
            "42" => "定義が当たらない",
            "53" => "資源が足りない",
            "57" => "止められた",
            "08" => "繋がらない",
            _ => "DB の誤り",
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reason {
    pub kind: String,
    pub failed_version: String,
    pub sqlstate: Option<String>,
}

impl Reason {
    pub fn kind(kind: ReasonKind, failed_version: &str, sqlstate: Option<&str>) -> Self {
        Self {
            kind: kind.as_str().into(),
            failed_version: failed_version.into(),
            sqlstate: sqlstate.map(str::to_owned),
        }
    }
    pub fn description(&self) -> &'static str {
        [
            ReasonKind::DbAhead,
            ReasonKind::LedgerUnreadable,
            ReasonKind::ChannelMismatch,
            ReasonKind::DbNotLoopback,
            ReasonKind::BackupFailed,
            ReasonKind::ConnectFailed,
            ReasonKind::ServerNotListening,
            ReasonKind::Aborted,
            ReasonKind::PgMajorMismatch,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == self.kind)
        .map_or("DB の誤り", ReasonKind::description)
    }
}

pub fn history_line(
    at: &str,
    result: &str,
    to_version: &str,
    from_version: &str,
    reason: Option<Reason>,
) -> String {
    let reason = reason.map(|reason| json!({"kind": reason.kind, "failed_version": reason.failed_version, "sqlstate": reason.sqlstate}));
    json!({"at": at, "result": result, "to_version": to_version, "from_version": from_version, "reason": reason}).to_string() + "\n"
}

/// 更新履歴を書けなかったときに残す控えの 1 行を作る。標準出力へ JSON を流さず、
/// CLI のログと混ざらない形でファイルへ渡すための共通入口である（D18・R35）。
pub fn pending_history_line(
    result: &str,
    to_version: &str,
    from_version: &str,
    reason: Option<Reason>,
) -> String {
    history_line("", result, to_version, from_version, reason)
}

/// `record-rollback` の終了コード 5 は、履歴を控えへ移しただけで更新処理を続けられる。
pub const HISTORY_PENDING_WRITTEN: i32 = 5;

pub fn update_run_id(now: SystemTime, pid: u32) -> String {
    let nanos = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("{nanos}-{pid}")
}

pub fn pg_restore_command(program: &Path, dump: &Path, port: u16) -> SetupCommand {
    SetupCommand::new(
        program.display().to_string(),
        [
            "--exit-on-error",
            "--no-password",
            "--dbname=ashiato_restore",
            "-U",
            "ashiato_owner",
            "-h",
            "127.0.0.1",
            "-p",
            &port.to_string(),
            &dump.display().to_string(),
        ],
    )
}

pub fn rollback_psql_command(program: &Path, host: &str, port: u16) -> SetupCommand {
    SetupCommand::new(
        program.display().to_string(),
        [
            "-X",
            "-v",
            "ON_ERROR_STOP=1",
            "-U",
            "ashiato_owner",
            "-h",
            host,
            "-p",
            &port.to_string(),
            "-d",
            "postgres",
        ],
    )
}

pub fn service_update_command(service: &str, start_type: &str) -> SetupCommand {
    SetupCommand::new("sc.exe", ["config", service, "start=", start_type])
}

pub fn service_stop_command(service: &str) -> SetupCommand {
    SetupCommand::new("sc.exe", ["stop", service])
}

pub fn service_start_command(service: &str) -> SetupCommand {
    SetupCommand::new("sc.exe", ["start", service])
}

pub fn update_layout(development: bool) -> Layout {
    for_version(if development {
        "0.1.0-dev.20261116"
    } else {
        "0.1.0"
    })
}
