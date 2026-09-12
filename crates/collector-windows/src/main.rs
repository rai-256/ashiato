// SPDX-License-Identifier: AGPL-3.0-only
//! C-02 Windows 収集の入口。**規則は lib 側**（`ashiato_collector_windows`）にある。
//!
//! ```text
//! ASHIATO_BASE_URL=http://127.0.0.1:8787 ASHIATO_API_TOKEN=… \
//! ASHIATO_USER_ID=… ASHIATO_DEVICE_ID=pc-01 ASHIATO_STATE_DIR=%APPDATA%\ashiato \
//!   ashiato-collector-windows
//! ```
//!
//! `--install-autostart` でログオン時の自動起動を仕込む（design D7）。
use ashiato_collector_windows as c02;

fn main() -> anyhow::Result<()> {
    // **ログは tracing に一本化する**（製造準備 A-2。出さないものを決めてある）
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = c02::config::Config::from_env()?;
    if std::env::args().any(|a| a == "--install-autostart") {
        return install_autostart(&cfg);
    }
    run(cfg)
}

#[cfg(windows)]
fn install_autostart(cfg: &c02::config::Config) -> anyhow::Result<()> {
    let path = c02::autostart::install(&cfg.state_dir)?;
    tracing::info!(
        "{}",
        c02::telemetry::line("autostart_installed", None, None, None)
    );
    tracing::info!("置いた場所: {}", path.display());
    Ok(())
}

#[cfg(not(windows))]
fn install_autostart(_cfg: &c02::config::Config) -> anyhow::Result<()> {
    anyhow::bail!("自動起動を仕込めるのは Windows だけ")
}

/// 収集を回す。**止まる契機は持たない**（常駐する。design D7）。
#[cfg(windows)]
fn run(cfg: c02::config::Config) -> anyhow::Result<()> {
    use chrono::Utc;

    let zone = c02::config::Zone::current()?;
    let exclusions = c02::exclusion::Exclusions::load(&cfg.state_dir.join("exclusions.json"))?;
    let engine = c02::engine::Engine::new(exclusions);
    let transport = c02::sender::HttpTransport::new(&cfg.base_url, &cfg.api_token);
    let reference = c02::clock::HttpDateClock::new(&cfg.base_url);
    let mut rt =
        c02::runtime::Runtime::new(&cfg, zone, engine, &transport, &reference, Utc::now())?;
    let mut source = c02::platform::WindowsSource::open();
    rt.start(Utc::now())?;
    tracing::info!("{}", c02::telemetry::line("started", None, None, None));
    loop {
        rt.tick(&mut source, Utc::now())?;
        wait_for_change(&source);
    }
}

/// 次の見回りまで待つ。**OS からの通知が来たら待たない**（design D5 の併用）。
#[cfg(windows)]
fn wait_for_change(source: &c02::platform::WindowsSource) {
    use std::time::Duration;

    let slice = Duration::from_millis(50);
    let slices = (c02::runtime::POLL_INTERVAL_SEC * 1000 / 50).max(1);
    for _ in 0..slices {
        if source.take_changed() {
            return;
        }
        std::thread::sleep(slice);
    }
}

/// Windows 以外では動かない。**黙って何もしないのではなく落とす** ——
/// 動いているつもりで 1 件も入らない状態が、いちばん見つかりにくい。
#[cfg(not(windows))]
fn run(_cfg: c02::config::Config) -> anyhow::Result<()> {
    anyhow::bail!("C-02 は Windows の上でだけ動く（前景・入力・UI Automation を読む）")
}
