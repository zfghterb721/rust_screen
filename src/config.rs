use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Address to advertise in RTSP URLs (defaults to local IP)
    pub server_address: String,
    
    /// Port for the RTSP server
    pub rtsp_port: u16,
    
    /// Frames per second to capture and stream
    pub frame_rate: u32,
    
    /// Video quality (0-10, higher is better quality but more bandwidth)
    pub quality: u32,
    
    /// Whether to capture cursor in the screen capture
    pub capture_cursor: bool,
    
    /// List of display indices to capture (empty = all displays)
    pub displays: Vec<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_address: local_ip_address(),
            rtsp_port: 8554,
            frame_rate: 15,
            quality: 7,
            capture_cursor: true,
            displays: Vec::new(),
        }
    }
}

fn local_ip_address() -> String {
    match local_ip_address::local_ip() {
        Ok(ip) => ip.to_string(),
        Err(_) => "127.0.0.1".to_string(),
    }
}

pub fn load_config(path: &Path) -> Result<Config> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let config: Config = toml::from_str(&contents)
                .context("Failed to parse config file")?;
            Ok(config)
        }
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            // Config file doesn't exist, create default config
            let config = Config::default();
            save_config(path, &config)?;
            Ok(config)
        }
        Err(e) => Err(e).context("Failed to read config file"),
    }
}

pub fn save_config(path: &Path, config: &Config) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create config directory")?;
    }
    
    let contents = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;
    
    fs::write(path, contents).context("Failed to write config file")?;
    
    Ok(())
}