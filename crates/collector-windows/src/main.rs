// SPDX-License-Identifier: AGPL-3.0-only
//! C-02 Windows 収集の入口。**規則は lib 側**（`ashiato_collector_windows`）にある。
//!
//! ```text
//! ASHIATO_BASE_URL=http://127.0.0.1:8787 ASHIATO_API_TOKEN=… \
//! ASHIATO_USER_ID=… ASHIATO_DEVICE_ID=pc-01 ASHIATO_STATE_DIR=C:\Users\me\AppData\Roaming\ashiato \
//!   ashiato-collector-windows
//! ```
//!
//! `--install-autostart` でログオン時の自動起動を仕込む（design D7）。
use ashiato_collector_windows as c02;

fn main() -> anyhow::Result<()> {
    let cfg = c02::config::Config::from_env()?;
    init_logging(&cfg);
    if std::env::args().any(|a| a == "--install-autostart") {
        return install_autostart(&cfg);
    }
    run(cfg)
}

/// ログは tracing に一本化する（製造準備 A-2）。**置き場のファイルにも書く** ——
/// 自動起動は最小化した窓で動くので、窓だけに出すと落ちた理由がどこにも残らない（R16）。
fn init_logging(cfg: &c02::config::Config) {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let _ = std::fs::create_dir_all(&cfg.state_dir);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(cfg.state_dir.join("collector.log"));
    match file {
        Ok(f) => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(f))
            .init(),
        Err(_) => tracing_subscriber::fmt().with_env_filter(filter).init(),
    }
}

#[cfg(windows)]
fn install_autostart(cfg: &c02::config::Config) -> anyhow::Result<()> {
    let path = c02::autostart::install(cfg)?;
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

/// 収集を回す。**止まるのは終了の合図を受けたときだけ**（design D7 / D22）。
#[cfg(windows)]
fn run(cfg: c02::config::Config) -> anyhow::Result<()> {
    use chrono::Utc;
    use std::sync::atomic::{AtomicBool, Ordering};

    // **二重に起動しない**（R32）。動いている収集は 1 分ごとに印を書くので、
    // 印が新しければもう 1 つが動いている —— 2 つが同じ未送信を書き直し合うと記録が消える
    let marker = c02::marker::Marker::new(&cfg.state_dir);
    if let Ok(Some(last)) = marker.read() {
        let age = Utc::now() - last;
        if age >= chrono::Duration::zero()
            && age < chrono::Duration::seconds(c02::marker::TOUCH_INTERVAL_SEC * 2 + 30)
        {
            tracing::info!(
                "{}",
                c02::telemetry::line("already_running", Some(1), None, None)
            );
            return Ok(());
        }
    }

    let zone = c02::config::Zone::current()?;
    // 除外の登録が壊れていたら**止まる**（空に倒すと除外が黙って外れる）。理由はログに残る
    let exclusions = c02::exclusion::Exclusions::load(&cfg.state_dir.join("exclusions.json"))
        .inspect_err(|_| {
            tracing::error!(
                "{}",
                c02::telemetry::line("exclusions_invalid", None, None, None)
            )
        })?;
    tracing::info!(
        "{}",
        c02::telemetry::line(
            "exclusions_loaded",
            Some(exclusions.rules.len()),
            None,
            None
        )
    );
    let engine = c02::engine::Engine::new(exclusions);
    let transport = c02::sender::HttpTransport::new(&cfg.base_url, &cfg.api_token);
    let reference = c02::clock::HttpDateClock::new(&cfg.base_url);
    let mut rt =
        c02::runtime::Runtime::new(&cfg, zone, engine, &transport, &reference, Utc::now())?;
    let mut source = c02::platform::WindowsSource::open();
    if !source.focus_events() {
        tracing::info!(
            "{}",
            c02::telemetry::line("focus_events_unavailable", None, None, None)
        );
    }

    let stop = std::sync::Arc::new(AtomicBool::new(false));
    {
        let stop = std::sync::Arc::clone(&stop);
        let _ = ctrlc::set_handler(move || stop.store(true, Ordering::Relaxed));
    }

    rt.start(&source);
    tracing::info!("{}", c02::telemetry::line("started", None, None, None));
    while !stop.load(Ordering::Relaxed) {
        rt.tick(&mut source);
        wait_for_change(&source, &stop);
    }
    // **手元の数えを吐き出し、自分で止まった印を残してから終わる**（R13 / D23）
    rt.stop();
    tracing::info!("{}", c02::telemetry::line("stopped", None, None, None));
    Ok(())
}

/// 次の見回りまで待つ。**OS からの通知・終了の合図が来たら待たない**（design D5）。
#[cfg(windows)]
fn wait_for_change(source: &c02::platform::WindowsSource, stop: &std::sync::atomic::AtomicBool) {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let slice = Duration::from_millis(50);
    let slices = (c02::runtime::POLL_INTERVAL_SEC * 1000 / 50).max(1);
    for _ in 0..slices {
        if source.take_changed() || stop.load(Ordering::Relaxed) {
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
