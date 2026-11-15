//! PC 収集アプリの版別入れ替え。旧プロセスを止めてから新プロセスを確認し、失敗時は戻す。

use anyhow::{Context as _, Result};
use ashiato_build_stamp::Channel;
use ashiato_update_check::{select_asset, Component};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const HANDOFF: &str = "update-handoff.json";
const HANDOFF_ACK: &str = "update-handoff.ack.json";
const ALIVE: &str = "update-alive.json";
const OFFICIAL_EXECUTABLE_NAME: &str = "ashiato-collector-windows.exe";
const DEV_EXECUTABLE_NAME: &str = "ashiato-collector-dev.exe";
const WAIT_FOR_ALIVE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct UpdateLayout {
    pub state_dir: PathBuf,
}

impl UpdateLayout {
    pub fn new(state_dir: &Path) -> Self {
        Self {
            state_dir: state_dir.to_owned(),
        }
    }
    pub fn versions_dir(&self) -> PathBuf {
        self.state_dir.join("versions")
    }
    pub fn version_dir(&self, version: &str) -> PathBuf {
        self.versions_dir().join(version)
    }
    fn handoff_path(&self) -> PathBuf {
        self.state_dir.join(HANDOFF)
    }
    fn ack_path(&self) -> PathBuf {
        self.state_dir.join(HANDOFF_ACK)
    }
    fn alive_path(&self) -> PathBuf {
        self.state_dir.join(ALIVE)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Handoff {
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Alive {
    version: String,
    at: chrono::DateTime<Utc>,
}

pub fn is_newer_version(candidate: &str, installed: &str) -> bool {
    ashiato_update_check::update_available(installed, Some(candidate)).unwrap_or(false)
}

/// 新しいプロセスが収集の初期化を終えた後に呼び出す生存信号。
pub fn write_alive(state_dir: &Path, version: &str) -> Result<()> {
    #[cfg(ashiato_test_build)]
    if std::env::var_os("ASHIATO_COLLECTOR_TEST_UPDATE_NO_ALIVE").is_some() {
        return Ok(());
    }
    let layout = UpdateLayout::new(state_dir);
    crate::fsutil::atomic_write(
        &layout.alive_path(),
        &serde_json::to_vec(&Alive {
            version: version.to_owned(),
            at: Utc::now(),
        })?,
    )?;
    Ok(())
}

/// 動いている版が handoff を一度だけ受け取り、更新窓へ停止の応答を返す。
pub fn take_handoff(state_dir: &Path) -> Result<bool> {
    let layout = UpdateLayout::new(state_dir);
    let bytes = match std::fs::read(layout.handoff_path()) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let _: Handoff = serde_json::from_slice(&bytes).context("更新の停止印が読めない")?;
    std::fs::remove_file(layout.handoff_path()).context("更新の停止印を消せない")?;
    // ack は収集側が rt.stop()（flush・停止印の保存）を完了した後に書く。
    // ここで先に書くと、更新窓が新しい版を起こして旧版と同時に記録できる。
    Ok(true)
}

/// 収集側の停止処理が終わったことを更新窓へ知らせる。
pub fn acknowledge_handoff(state_dir: &Path) -> Result<()> {
    let layout = UpdateLayout::new(state_dir);
    crate::fsutil::atomic_write(&layout.ack_path(), br#"{}"#)?;
    Ok(())
}

pub fn run_update<R: BufRead, W: Write>(
    cfg: &crate::config::Config,
    channel: Channel,
    input: &mut R,
    output: &mut W,
) -> Result<()> {
    let result = run_update_inner(cfg, channel, input, output);
    match &result {
        Ok(()) => writeln!(output, "Enter で閉じます")?,
        Err(error) => writeln!(output, "更新に失敗しました: {error}\nEnter で閉じます")?,
    }
    output.flush()?;
    let mut ignored = String::new();
    let _ = input.read_line(&mut ignored);
    result
}

fn run_update_inner<R: BufRead, W: Write>(
    cfg: &crate::config::Config,
    channel: Channel,
    input: &mut R,
    output: &mut W,
) -> Result<()> {
    let layout = UpdateLayout::new(&cfg.state_dir);
    let installed = env!("ASHIATO_VERSION");
    let state = crate::update_check::UpdateChecker::new(&cfg.state_dir).state()?;
    let Some(latest) = state
        .latest
        .as_deref()
        .filter(|v| is_newer_version(v, installed))
    else {
        writeln!(output, "入っている版が最新です")?;
        return Ok(());
    };
    writeln!(
        output,
        "入っている版 {installed} / 新しい版 {latest} / 入れ替えますか [y/N]"
    )?;
    output.flush()?;
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    if answer.trim() != "y" {
        return Ok(());
    }

    writeln!(output, "配布物を落としています")?;
    output.flush()?;

    let releases = crate::update_check::fetch_releases(&cfg.state_dir)
        .map_err(|e| anyhow::anyhow!("版の一覧を取り直せない: {e:?}"))?;
    let release = releases
        .iter()
        .find(|r| r.assets.iter().any(|a| a.name.contains(latest)))
        .context("新しい版が一覧に無い")?;
    let asset =
        select_asset(Component::CollectorWindows, release).context("指紋のある PC 配布物が無い")?;
    let bytes = crate::update_check::download_asset(
        asset.browser_download_url.as_deref().unwrap_or_default(),
    )
    .map_err(|e| anyhow::anyhow!("配布物を落とせない: {e:?}"))?;
    verify_digest(
        &bytes,
        asset.digest.as_deref().context("配布物の指紋が無い")?,
    )?;
    let staged = layout.version_dir(latest);
    extract_zip(&bytes, &staged)?;
    let executable = update_executable(&staged, channel)?;
    let _ = std::fs::remove_file(layout.ack_path());
    let _ = std::fs::remove_file(layout.alive_path());
    crate::fsutil::atomic_write(
        &layout.handoff_path(),
        &serde_json::to_vec(&Handoff {
            version: latest.to_owned(),
        })?,
    )?;
    wait_for_ack(&layout, WAIT_FOR_ALIVE)?;
    writeln!(output, "配布物を照らしています")?;
    writeln!(output, "新しい版の起動を待っています")?;
    output.flush()?;
    let mut child = spawn_staged_collector(&staged, cfg, channel)?;
    if wait_for_alive(&layout, latest, WAIT_FOR_ALIVE)? {
        crate::autostart::rewrite_existing_for_setup(&executable, channel)?;
        remove_old_versions(&layout, latest)?;
        writeln!(output, "{latest} に入れ替えました")?;
    } else {
        let _ = child.kill();
        let _ = spawn_previous_collector(cfg)?;
        writeln!(
            output,
            "新しい版 {latest} が動き出さなかったので、{installed} に戻しました"
        )?;
        #[cfg(windows)]
        if let Ok(toaster) = crate::toast::WindowsToaster::register(channel) {
            let _ = crate::update_check::Toaster::show_update(
                &toaster,
                &crate::update_check::toast_xml(
                    "更新を戻しました",
                    &format!("新しい版 {latest} が動き出しませんでした"),
                ),
                crate::toast::UPDATE_ROLLBACK_TAG,
            );
        }
    }
    Ok(())
}

fn verify_digest(bytes: &[u8], digest: &str) -> Result<()> {
    let expected = digest
        .strip_prefix("sha256:")
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .context("配布物の指紋が不正")?;
    let actual = Sha256::digest(bytes);
    let actual_hex = format!("{actual:x}");
    anyhow::ensure!(
        actual_hex.eq_ignore_ascii_case(expected),
        "配布物の指紋が合わない"
    );
    Ok(())
}

fn extract_zip(bytes: &[u8], destination: &Path) -> Result<()> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).context("配布物を zip として開けない")?;
    std::fs::create_dir_all(destination)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(name) = entry.enclosed_name() else {
            anyhow::bail!("zip の道が不正")
        };
        let path = destination.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&path)?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::File::create(&path)?;
            std::io::copy(&mut entry, &mut file)?;
            #[cfg(unix)]
            if let Some(mode) = entry.unix_mode() {
                std::fs::set_permissions(
                    &path,
                    std::os::unix::fs::PermissionsExt::from_mode(mode),
                )?;
            }
        }
    }
    Ok(())
}

pub fn update_executable(staged: &Path, channel: Channel) -> Result<PathBuf> {
    let name = match channel {
        Channel::Official => OFFICIAL_EXECUTABLE_NAME,
        Channel::Dev => DEV_EXECUTABLE_NAME,
    };
    let path = staged.join(name);
    anyhow::ensure!(path.is_file(), "版ごとの置き場に実行ファイルが無い");
    Ok(path)
}

fn wait_for_ack(layout: &UpdateLayout, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if layout.ack_path().is_file() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = std::fs::remove_file(layout.handoff_path());
    anyhow::bail!("前の版が停止しない")
}

pub fn wait_for_alive(layout: &UpdateLayout, version: &str, timeout: Duration) -> Result<bool> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(bytes) = std::fs::read(layout.alive_path()) {
            if let Ok(alive) = serde_json::from_slice::<Alive>(&bytes) {
                if alive.version == version
                    && Utc::now()
                        .signed_duration_since(alive.at)
                        .num_seconds()
                        .abs()
                        <= 60
                {
                    return Ok(true);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(false)
}

#[allow(clippy::needless_borrow)]
fn spawn_staged_collector(
    staged: &Path,
    cfg: &crate::config::Config,
    channel: Channel,
) -> Result<Child> {
    let mut command = Command::new(update_executable(&staged, channel)?);
    command
        .env("ASHIATO_STATE_DIR", &cfg.launch_state_dir)
        .env("ASHIATO_DEVICE_ID", &cfg.launch_device_id);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    command.spawn().context("収集アプリを起こせない")
}

fn spawn_previous_collector(cfg: &crate::config::Config) -> Result<Child> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .env("ASHIATO_STATE_DIR", &cfg.launch_state_dir)
        .env("ASHIATO_DEVICE_ID", &cfg.launch_device_id);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    command.spawn().context("前の版を起こせない")
}

fn remove_old_versions(layout: &UpdateLayout, current: &str) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(layout.versions_dir()) else {
        return Ok(());
    };
    let mut previous = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name != current
            && entry.path().is_dir()
            && previous
                .as_deref()
                .is_none_or(|known| is_newer_version(&name, known))
        {
            previous = Some(name);
        }
    }
    let Ok(entries) = std::fs::read_dir(layout.versions_dir()) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name != current && previous.as_deref() != Some(name.as_str()) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn st58_update_digest_rejects_missing_or_wrong_fingerprint() {
        assert!(verify_digest(b"x", "sha256:00").is_err());
        assert!(verify_digest(
            b"x",
            "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        )
        .is_err());
    }

    #[test]
    fn st58_update_timeout_removes_stale_handoff() {
        let dir = tempfile::tempdir().unwrap();
        let layout = UpdateLayout::new(dir.path());
        std::fs::write(layout.handoff_path(), br#"{"version":"0.5.0"}"#).unwrap();
        assert!(wait_for_ack(&layout, Duration::ZERO).is_err());
        assert!(!layout.handoff_path().exists());
    }

    #[test]
    fn st58_update_uses_channel_specific_executable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(OFFICIAL_EXECUTABLE_NAME), b"official").unwrap();
        std::fs::write(dir.path().join(DEV_EXECUTABLE_NAME), b"dev").unwrap();
        assert_eq!(
            update_executable(dir.path(), Channel::Official).unwrap(),
            dir.path().join(OFFICIAL_EXECUTABLE_NAME)
        );
        assert_eq!(
            update_executable(dir.path(), Channel::Dev).unwrap(),
            dir.path().join(DEV_EXECUTABLE_NAME)
        );
    }
}
