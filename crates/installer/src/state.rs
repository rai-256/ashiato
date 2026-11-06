use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct InstallState {
    pub settings: bool,
    pub database: bool,
    pub verifier: bool,
    pub services: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallPhase {
    WriteSecrets,
    InitDatabase,
    RegisterServices,
    Password,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallDecision {
    Continue(InstallPhase),
    RepairRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairRefusal {
    OwnerAmbiguous,
}

/// D10 の順序を、状態の見え方だけから決める判定である。
pub fn decide_install(state: InstallState) -> InstallDecision {
    if state.database && !state.settings {
        return InstallDecision::RepairRequired;
    }
    if !state.settings {
        return InstallDecision::Continue(InstallPhase::WriteSecrets);
    }
    if !state.database {
        return InstallDecision::Continue(InstallPhase::InitDatabase);
    }
    if !state.services {
        return InstallDecision::Continue(InstallPhase::RegisterServices);
    }
    if !state.verifier {
        return InstallDecision::Continue(InstallPhase::Password);
    }
    InstallDecision::Continue(InstallPhase::Complete)
}

/// repair を受け付けるか。DB があって、設定が無いか、役割の完了記録が無いとき（役割の前に止まった DB を
/// 消さずに役割を作り直す出口。D32（仮）・review R40）。
pub fn repair_allowed(database: bool, settings: bool, roles_complete: bool) -> bool {
    database && (!settings || !roles_complete)
}

/// 役割の完了記録も initdb を始めた印も無い DB で止まるときの理由。記録が入っているかもしれないので消させない。
pub fn roles_missing_message(db_dir: &Path) -> String {
    format!(
        "DB の役割の完了記録がありません。記録が入っているかもしれないので、{} は消しません。ashiato-setup.exe repair で DB の役割を作り直してください（docs/install.md）",
        db_dir.display()
    )
}

pub fn repair_message(state_dir: &Path) -> String {
    format!(
        "DB と秘密が食い違っています。状態の置き場: {}。\nashiato-setup.exe repair を実行してください。",
        state_dir.display()
    )
}

pub fn repair_completion_message() -> &'static str {
    "repair が完了しました。収集アプリは繋ぎ直しが必要です。手順: docs/install.md"
}

/// 走らせ直しでは、設定に控えた本人（`ASHIATO_OWNER_SID`）を使う。無い（初回）ときだけ今回の本人。
pub fn owner_sid_for_rerun<'a>(stored: Option<&'a str>, invoking: &'a str) -> &'a str {
    stored.filter(|sid| !sid.is_empty()).unwrap_or(invoking)
}

/// 状態の置き場の ACL の主体（`Get-Acl` を SID に直した 1 行 1 つ、または `icacls` の行）から、
/// サービスのアカウント（`NT SERVICE\…`・サービスの SID `S-1-5-80-…`）ではない読み取り主体を選ぶ。
/// repair は本人を新しく実行した管理者へ変更しないため、候補が一つでなければ止める。
pub fn owner_sid_from_acl(acl: &str) -> Result<String, RepairRefusal> {
    let mut owners = acl
        .lines()
        .filter(|line| !line.contains("NT SERVICE\\"))
        .filter_map(|line| {
            line.split_whitespace()
                .find(|part| part.starts_with("S-1-"))
                .map(|part| part.split(':').next().unwrap_or(part).to_owned())
        })
        .filter(|sid| !sid.starts_with("S-1-5-80-"))
        .collect::<Vec<_>>();
    owners.sort();
    owners.dedup();
    match owners.as_slice() {
        [owner] => Ok(owner.clone()),
        _ => Err(RepairRefusal::OwnerAmbiguous),
    }
}
