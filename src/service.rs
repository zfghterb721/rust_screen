#![cfg(windows)]

use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;
use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, service_manager::{ServiceManager, ServiceManagerAccess},
};

use crate::config;

const SERVICE_NAME: &str = "DisplayRtspStreamer";
const SERVICE_DISPLAY_NAME: &str = "Display RTSP Streamer";
const SERVICE_DESCRIPTION: &str = "Streams desktop displays over RTSP for use with security camera systems";

define_windows_service!(ffi_service_main, service_main);

fn service_main(arguments: Vec<OsString>) {
    // Register service control handler
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                info!("Service stop requested");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match service_control_handler::register(SERVICE_NAME, event_handler) {
        Ok(handle) => handle,
        Err(e) => {
            error!("Failed to register service control handler: {}", e);
            return;
        }
    };

    // Tell the system that the service is running
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    if let Err(e) = status_handle.set_service_status(next_status) {
        error!("Failed to set service status: {}", e);
        return;
    }

    // Get configuration and run the application
    let config_path = {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("display_rtsp_streamer");
        path.push("config.toml");
        path
    };

    match config::load_config(&config_path) {
        Ok(config) => {
            if let Err(e) = crate::run_app(config) {
                error!("Service application error: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to load service configuration: {}", e);
        }
    }

    // Tell the system that service is stopped
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    if let Err(e) = status_handle.set_service_status(next_status) {
        error!("Failed to set service status: {}", e);
    }
}

pub fn run_service() -> Result<()> {
    // Start the service dispatcher
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .context("Failed to start service dispatcher")
}

pub fn install_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
        .context("Failed to connect to service manager")?;

    // Get the current executable path
    let current_exe = std::env::current_exe()
        .context("Failed to get current executable path")?;

    // Create the service
    let service = manager.create_service(
        &OsString::from(SERVICE_NAME),
        &OsString::from(SERVICE_DISPLAY_NAME),
        ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::START,
        ServiceType::OWN_PROCESS,
        ServiceStartType::AutoStart,
        ServiceErrorControl::Normal,
        Some(&OsString::from(format!("\"{}\"", current_exe.display()))),
        None::<&str>,  // No load ordering group
        None,          // No tag identifier
        &[],           // No dependencies
        None::<&str>,  // Use Local System account
        None::<&str>,  // No password
    )
    .context("Failed to create service")?;

    // Set the service description
    service.set_description(SERVICE_DESCRIPTION)
        .context("Failed to set service description")?;

    info!("Service installed successfully");
    Ok(())
}

pub fn uninstall_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .context("Failed to connect to service manager")?;

    let service = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
    )
    .context("Failed to open service")?;

    // Delete the service
    service.delete().context("Failed to delete service")?;

    info!("Service uninstalled successfully");
    Ok(())
}

pub fn start_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .context("Failed to connect to service manager")?;

    let service = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::START,
    )
    .context("Failed to open service")?;

    service.start(&[]).context("Failed to start service")?;

    info!("Service started successfully");
    Ok(())
}

pub fn stop_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .context("Failed to connect to service manager")?;

    let service = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::STOP,
    )
    .context("Failed to open service")?;

    service.stop().context("Failed to stop service")?;

    info!("Service stopped successfully");
    Ok(())
}