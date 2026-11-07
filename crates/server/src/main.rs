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
                // SCM に繋いでから終了コード 0 で「止まった」を返す。繋ぐ前に終わると異常終了とみなされ、
                // 非 0 で返すと `failureflag 1` の回復が 10 秒ごとに起こし直す（設定は起こし直しても直らない。
                // 理由は prepare_service がサーバのログに書いた。review R21・R60）
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

/// SCM へ返す終了コードは `ServiceEnd` が決める。起こし直しても直らない拒否は 0、DB を待ちきれない・
/// 実行環境を作れない・それ以外の失敗は 1（`failureflag 1` の回復が起こし直す。review R60）。
#[cfg(windows)]
fn run_windows_service() -> u32 {
    use ashiato_server::service::{self, ServiceEnd};
    if SETTINGS_REFUSED.load(std::sync::atomic::Ordering::SeqCst) {
        return ServiceEnd::Refused.exit_code();
    }
    let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    else {
        service::log_startup_refusal("service_runtime");
        return ServiceEnd::RuntimeUnavailable.exit_code();
    };
    let end = ServiceEnd::classify(&runtime.block_on(service::run_service()));
    // 拒否と DB の時間切れは、返す前に理由を書いてある。2 行目を書かない（review R67）
    if end.needs_log() {
        service::log_startup_refusal("service_startup");
    }
    end.exit_code()
}

#[cfg(windows)]
fn request_service_stop() {
    ashiato_server::service::request_stop();
}
