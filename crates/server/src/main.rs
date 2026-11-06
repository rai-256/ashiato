// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! S-01 の起動口。中身は lib.rs にある（`bin/openapi.rs` から同じ型を読むため）。

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next();
    if command.as_deref() == Some("service") {
        #[cfg(not(windows))]
        {
            return ashiato_server::service::windows_only();
        }
        #[cfg(windows)]
        {
            let flag = args.next();
            let path = args.next();
            if flag.as_deref() != Some("--settings") || path.is_none() || args.next().is_some() {
                anyhow::bail!("service は --settings <場所> が必要です");
            }
            let settings_path = std::path::Path::new(path.as_deref().unwrap_or_default());
            if ashiato_server::service::prepare_service(settings_path).is_err() {
                // SCM に繋いでから「止まった」を返す。繋ぐ前に終わると異常終了とみなされ、
                // 回復の設定で起こし直しが続く（review R21）
                SETTINGS_REFUSED.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            return ashiato_winsvc::dispatch(run_windows_service, request_service_stop)
                .map_err(|error| anyhow::anyhow!(error.to_string()));
        }
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        match command.as_deref() {
            None => ashiato_server::run().await,
            Some("migrate") => ashiato_server::run_migrate().await,
            Some(other) => anyhow::bail!("知らない引数: {other}（使えるのは migrate だけ）"),
        }
    })
}

#[cfg(windows)]
static SETTINGS_REFUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
fn run_windows_service() -> u32 {
    if SETTINGS_REFUSED.load(std::sync::atomic::Ordering::SeqCst) {
        return 1;
    }
    let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    else {
        ashiato_server::service::log_startup_refusal("service_runtime");
        return 1;
    };
    if let Err(_error) = runtime.block_on(ashiato_server::service::run_service()) {
        ashiato_server::service::log_startup_refusal("service_startup");
        return 1;
    }
    0
}

#[cfg(windows)]
fn request_service_stop() {
    ashiato_server::service::request_stop();
}
