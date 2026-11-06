#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]
//! Windows サービス入口だけを隔離する crate。サーバ本体の lint 境界を越えない。

use std::ffi::OsString;
use std::sync::{Arc, OnceLock};
use windows_service::service::{ServiceAccess, ServiceControl};
use windows_service::service::{
    ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

windows_service::define_windows_service!(ffi_service_main, service_main);
static RUN: OnceLock<fn() -> u32> = OnceLock::new();
static STOP: OnceLock<fn()> = OnceLock::new();

fn service_main(_arguments: Vec<OsString>) {
    let status_for_handler: Arc<OnceLock<service_control_handler::ServiceStatusHandle>> =
        Arc::new(OnceLock::new());
    let status_for_callback = Arc::clone(&status_for_handler);
    let Ok(status_handle) =
        service_control_handler::register("ashiato-server", move |control| match control {
            // OS の終わり・再起動（Windows Update など）でも止める処理を通す。PRESHUTDOWN を受けると、
            // SCM は OS を止める前に知らせて待つ（書き出しを止めて書きかけを片付ける時間を取る。review R53）
            ServiceControl::Stop | ServiceControl::Preshutdown | ServiceControl::Shutdown => {
                if let Some(status_handle) = status_for_callback.get() {
                    let _ = status_handle.set_service_status(ServiceStatus {
                        service_type: ServiceType::OWN_PROCESS,
                        current_state: ServiceState::StopPending,
                        controls_accepted: ServiceControlAccept::empty(),
                        exit_code: ServiceExitCode::Win32(0),
                        checkpoint: 1,
                        wait_hint: std::time::Duration::from_secs(120),
                        process_id: None,
                    });
                }
                if let Some(stop) = STOP.get() {
                    stop();
                }
                ServiceControlHandlerResult::NoError
            }
            // 問い合わせには誤りでなく「受けた」を返す（いまの状態は SCM が持っている）
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })
    else {
        return;
    };
    let _ = status_for_handler.set(status_handle);
    let _ = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 1,
        wait_hint: std::time::Duration::from_secs(120),
        process_id: None,
    });
    let _ = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: accepted_controls(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    });
    let exit_code = RUN.get().map_or(0, |run| run());
    let _ = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(exit_code),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    });
}

/// 動いている間に受ける制御。止める・OS の終わりの前・OS の終わり（review R53）。
pub fn accepted_controls() -> ServiceControlAccept {
    ServiceControlAccept::STOP | ServiceControlAccept::PRESHUTDOWN | ServiceControlAccept::SHUTDOWN
}

/// Windows のサービス制御マネージャから入口を起こす。
pub fn dispatch(run: fn() -> u32, stop: fn()) -> windows_service::Result<()> {
    let _ = RUN.set(run);
    let _ = STOP.set(stop);
    service_dispatcher::start("ashiato-server", ffi_service_main)
}

/// DB のサービスの状態を読み、停止なら開始できるハンドルを返す（D35）。
pub fn query_db_service_state() -> Result<u32, String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|error| error.to_string())?;
    let service = manager
        .open_service(
            "ashiato-db",
            ServiceAccess::QUERY_STATUS | ServiceAccess::START,
        )
        .map_err(|error| error.to_string())?;
    service
        .query_status()
        .map(|status| status.current_state as u32)
        .map_err(|error| error.to_string())
}

pub fn start_db_service() -> Result<(), String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|error| error.to_string())?;
    let service = manager
        .open_service(
            "ashiato-db",
            ServiceAccess::QUERY_STATUS | ServiceAccess::START,
        )
        .map_err(|error| error.to_string())?;
    service
        .start::<&std::ffi::OsStr>(&[])
        .map_err(|error| error.to_string())
}
