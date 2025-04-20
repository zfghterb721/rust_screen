use anyhow::{Context, Result};
use display_info::DisplayInfo;
use log::{error, info};
use scrap::{Capturer, Display};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::rtsp::RtspMount;

#[derive(Debug, Clone)]
pub struct DisplayMetadata {
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
    pub name: String,
}

pub fn get_displays() -> Result<Vec<DisplayMetadata>> {
    let displays = Display::all().context("Failed to enumerate displays")?;
    let display_infos = DisplayInfo::all().context("Failed to get display info")?;
    
    let mut result = Vec::new();
    
    for (i, display) in displays.iter().enumerate() {
        // Get width and height from scrap::Display
        let width = display.width() as u32;
        let height = display.height() as u32;
        
        // Try to match with display info to get more metadata
        let display_info = display_infos.iter().find(|info| {
            info.width == width && info.height == height
        });
        
        let (x, y, is_primary, name) = if let Some(info) = display_info {
            (
                info.x, 
                info.y, 
                info.is_primary,
                format!("Display {}", i),
            )
        } else {
            // Fallback if we can't match
            (0, 0, i == 0, format!("Display {}", i))
        };
        
        result.push(DisplayMetadata {
            index: i,
            x,
            y,
            width,
            height,
            is_primary,
            name,
        });
    }
    
    // Sort by display position (if available)
    result.sort_by_key(|d| (d.x, d.y));
    
    Ok(result)
}


fn capture_display_thread(
    index: usize,
    rtsp_mount: RtspMount,
    frame_rate: u32,
    running: Arc<AtomicBool>
) -> Result<thread::JoinHandle<()>> {
    // Create a separate thread to own the Display and Capturer
    let handle = thread::spawn(move || {
        // Perform display capture within the thread
        match capture_frames(index, rtsp_mount, frame_rate, running) {
            Ok(_) => info!("Capture thread for display {} completed", index),
            Err(e) => error!("Capture thread for display {} failed: {}", index, e),
        }
    });
    
    Ok(handle)
}

// This function is called within the thread and handles the actual frame capture
fn capture_frames(
    display_index: usize,
    rtsp_mount: RtspMount,
    frame_rate: u32,
    running: Arc<AtomicBool>
) -> Result<()> {
    // Get displays now, within the thread
    let displays = Display::all().context("Failed to enumerate displays")?;
    
    if display_index >= displays.len() {
        anyhow::bail!("Display index {} out of bounds (only {} displays found)", 
                      display_index, displays.len());
    }
    
    // Get a reference to the display
    // Get dimensions for BGR conversion
    let width = displays[display_index].width() as u32;
    let height = displays[display_index].height() as u32;
    
    // Create a capturer
    // This takes ownership of the display
    let mut capturer = Capturer::new(displays.into_iter().nth(display_index).unwrap())
        .context("Failed to create screen capturer")?;
    
    info!("Started capture thread for display {}", display_index);
    
    let frame_delay = Duration::from_micros((1_000_000.0 / frame_rate as f64) as u64);
    
    // Main capture loop
    while running.load(Ordering::SeqCst) {
        let start_time = Instant::now();
        
        // Capture frame
        match capturer.frame() {
            Ok(frame) => {
                if frame.is_empty() {
                    // Occasionally, we might get an empty frame, just wait a bit and try again
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                
                // Convert frame to BGR format expected by GStreamer video sink
                // scrap gives us BGRA, we need to strip the alpha channel
                let bgr_frame = convert_bgra_to_bgr(&frame, width, height);
                
                // Push the frame to the RTSP stream
                if let Err(e) = rtsp_mount.push_frame(&bgr_frame) {
                    error!("Failed to push frame to RTSP stream: {}", e);
                    // Don't break immediately, try again
                }
            }
            Err(error) => {
                // Some capture errors can be transient
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                error!("Error capturing frame from display {}: {}", display_index, error);
                break;
            }
        };
        
        // Sleep to maintain desired frame rate
        let elapsed = start_time.elapsed();
        if elapsed < frame_delay {
            thread::sleep(frame_delay - elapsed);
        }
    }
    
    info!("Capture thread for display {} exiting", display_index);
    Ok(())
}

// Entry point for starting a capture thread
pub fn start_capture_thread(
    display_index: usize,
    _display_meta: DisplayMetadata,  // Metadata kept for possible future use
    rtsp_mount: RtspMount,
    frame_rate: u32,
    running: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>> {
    capture_display_thread(display_index, rtsp_mount, frame_rate, running)
}

// Convert BGRA format from scrap to BGR format needed by GStreamer
fn convert_bgra_to_bgr(bgra: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut bgr = Vec::with_capacity(pixel_count * 3);
    
    for i in 0..pixel_count {
        let bgra_offset = i * 4;
        if bgra_offset + 2 < bgra.len() {
            bgr.push(bgra[bgra_offset]);     // B
            bgr.push(bgra[bgra_offset + 1]); // G
            bgr.push(bgra[bgra_offset + 2]); // R
            // Skip alpha channel
        }
    }
    
    bgr
}