use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{error, info};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod capture;
mod config;
mod rtsp;
mod service;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install as a Windows service
    Install {
        /// Run the service after installation
        #[arg(long)]
        start: bool,
    },
    /// Uninstall the Windows service
    Uninstall,
    /// Start the Windows service
    Start,
    /// Stop the Windows service
    Stop,
    /// Run in foreground (not as a service)
    Run,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let config_path = cli.config.unwrap_or_else(|| {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("display_rtsp_streamer");
        path.push("config.toml");
        path
    });

    let config = config::load_config(&config_path)
        .context("Failed to load configuration")?;

    match cli.command {
        Some(Commands::Install { start: _start }) => {
            #[cfg(windows)]
            {
                service::install_service()?;
                if _start {
                    service::start_service()?;
                }
                Ok(())
            }
            #[cfg(not(windows))]
            {
                error!("Service installation is only supported on Windows");
                anyhow::bail!("Not running on Windows")
            }
        }
        Some(Commands::Uninstall) => {
            #[cfg(windows)]
            {
                service::stop_service()?;
                service::uninstall_service()?;
                Ok(())
            }
            #[cfg(not(windows))]
            {
                error!("Service uninstallation is only supported on Windows");
                anyhow::bail!("Not running on Windows")
            }
        }
        Some(Commands::Start) => {
            #[cfg(windows)]
            {
                service::start_service()?;
                Ok(())
            }
            #[cfg(not(windows))]
            {
                error!("Service control is only supported on Windows");
                anyhow::bail!("Not running on Windows")
            }
        }
        Some(Commands::Stop) => {
            #[cfg(windows)]
            {
                service::stop_service()?;
                Ok(())
            }
            #[cfg(not(windows))]
            {
                error!("Service control is only supported on Windows");
                anyhow::bail!("Not running on Windows")
            }
        }
        Some(Commands::Run) | None => {
            info!("Starting in foreground mode");
            run_app(config)
        }
    }
}

fn run_app(config: config::Config) -> Result<()> {
    info!("Starting display RTSP streamer");
    
    // Set up shutdown signal
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        info!("Received shutdown signal");
        r.store(false, Ordering::SeqCst);
    })
    .context("Error setting Ctrl-C handler")?;

    // Discover displays
    let displays = capture::get_displays()?;
    if displays.is_empty() {
        error!("No displays found");
        anyhow::bail!("No displays found");
    }

    info!("Found {} displays", displays.len());
    for (i, display) in displays.iter().enumerate() {
        info!(
            "Display {}: {}x{} at {},{}", 
            i, display.width, display.height, display.x, display.y
        );
    }

    // Initialize RTSP server
    let rtsp_server = rtsp::RtspServer::new(config.rtsp_port)?;
    
    // Start capture and streaming for each display
    let mut capture_handles = Vec::new();
    
    for (i, display) in displays.iter().enumerate() {
        let stream_path = format!("/display{}", i);
        let rtsp_mount = rtsp_server.add_stream(&stream_path, display.width, display.height)?;
        
        let running_clone = running.clone();
        let capture_handle = capture::start_capture_thread(
            i, 
            display.clone(),
            rtsp_mount,
            config.frame_rate,
            running_clone,
        )?;
        
        capture_handles.push(capture_handle);
        
        info!("Started streaming display {} at rtsp://{}:{}{}", 
              i, config.server_address, config.rtsp_port, stream_path);
    }

    // Keep running until shutdown signal
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    info!("Shutting down");
    
    // Wait for all capture threads to finish
    for handle in capture_handles {
        if let Err(e) = handle.join() {
            error!("Error joining capture thread: {:?}", e);
        }
    }

    info!("All streams stopped");
    Ok(())
}