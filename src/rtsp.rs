use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::AppSrc;
use gstreamer_rtsp_server::prelude::*;
use gstreamer_rtsp_server::{RTSPMediaFactory, RTSPServer};
use log::info;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use once_cell::sync::OnceCell;

// Initialize GStreamer once
static GST_INIT: OnceCell<()> = OnceCell::new();

pub fn init() -> Result<()> {
    GST_INIT.get_or_try_init(|| {
        gst::init().context("Failed to initialize GStreamer")?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

pub struct RtspServer {
    server: RTSPServer,
    mounts: gstreamer_rtsp_server::RTSPMountPoints,
    main_loop: glib::MainLoop,
}

impl RtspServer {
    pub fn new(port: u16) -> Result<Self> {
        // Always try to initialize GStreamer
        init()?;
        
        let server = RTSPServer::new();
        server.set_service(&port.to_string());
        
        let mounts = server.mount_points().context("Failed to get mount points")?;
        let main_loop = glib::MainLoop::new(None, false);
        
        // Start the server
        let _ = server.attach(None);
        
        info!("RTSP server started on port {}", port);
        
        // Start the GLib main loop in a separate thread
        let main_loop_clone = main_loop.clone();
        std::thread::spawn(move || {
            main_loop_clone.run();
        });
        
        Ok(Self {
            server,
            mounts,
            main_loop,
        })
    }
    
    pub fn add_stream(&self, path: &str, width: u32, height: u32) -> Result<RtspMount> {
        // Create a factory for this path
        let factory = RTSPMediaFactory::new();
        factory.set_shared(true);
        
        // Pipeline name isn't used with our current implementation
        
        // Create an AppSrc-based pipeline that will receive frames from our capture thread
        let launch_str = format!(
            "( appsrc name=source is-live=true format=time ! \
             video/x-raw,format=BGR,width={},height={},framerate=30/1 ! \
             videoconvert ! video/x-raw,format=I420 ! \
             x264enc tune=zerolatency speed-preset=ultrafast key-int-max=30 ! \
             rtph264pay name=pay0 pt=96 )",
            width, height
        );
        
        factory.set_launch(&launch_str);
        
        // Add factory to mount points
        self.mounts.add_factory(path, factory);
        
        info!("Added RTSP stream at path: {}", path);
        
        // Create a pipeline for sending frames
        let pipeline_str = format!(
            "appsrc name=source is-live=true format=time ! \
             video/x-raw,format=BGR,width={},height={},framerate=30/1 ! \
             fakesink",
            width, height
        );
        
        let pipeline = gst::parse_launch(&pipeline_str)
            .context("Failed to create pipeline")?;
        
        // Convert to a bin to use by_name
        let pipeline_bin = pipeline.clone().dynamic_cast::<gst::Bin>()
            .map_err(|_| anyhow::anyhow!("Failed to cast pipeline to Bin"))?;
        
        let appsrc = pipeline_bin.by_name("source")
            .context("Failed to find appsrc element")?
            .downcast::<AppSrc>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSrc"))?;
        
        // Configure the appsrc
        appsrc.set_format(gst::Format::Time);
        appsrc.set_is_live(true);
        appsrc.set_max_bytes(0);
        
        // Create the caps for the video format
        let caps = gst::Caps::builder("video/x-raw")
            .field("format", &"BGR")
            .field("width", &(width as i32))
            .field("height", &(height as i32))
            .field("framerate", &gst::Fraction::new(30, 1))
            .build();
        
        appsrc.set_caps(Some(&caps));
        
        // Set to playing state
        pipeline.set_state(gst::State::Playing).context("Failed to set pipeline to playing state")?;
        
        Ok(RtspMount {
            appsrc: Arc::new(Mutex::new(appsrc)),
        })
    }
}

impl Drop for RtspServer {
    fn drop(&mut self) {
        self.main_loop.quit();
    }
}

#[derive(Clone)]
pub struct RtspMount {
    appsrc: Arc<Mutex<AppSrc>>,
}

impl RtspMount {
    pub fn push_frame(&self, frame_data: &[u8]) -> Result<()> {
        let appsrc = self.appsrc.lock().unwrap();
        
        // Create a buffer from the frame data
        let mut buffer = gst::Buffer::with_size(frame_data.len())
            .context("Failed to allocate buffer")?;
        
        {
            let buffer_ref = buffer.get_mut().unwrap();
            buffer_ref.copy_from_slice(0, frame_data)
                .expect("Failed to copy data into buffer");
        }
        
        // Set buffer timestamp based on current time
        let pts = gst::ClockTime::from_nseconds(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_nanos() as u64
        );
        buffer.get_mut().unwrap().set_pts(pts);
        
        // Push the buffer to the appsrc
        appsrc.push_buffer(buffer)
            .map_err(|e| anyhow::anyhow!("Failed to push buffer: {:?}", e))?;
        
        Ok(())
    }
}