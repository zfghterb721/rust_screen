# Display RTSP Streamer

A lightweight Windows application that captures your desktop screens and makes them available as RTSP streams for integration with security camera systems like BlueIris.

**NOTE**: The final build requires the xcb-shm and xcb-randr libraries on Linux for development. On Windows, you'll need the GStreamer runtime installed.

## Features

- Automatically detects and streams all connected displays
- Runs silently as a Windows background service
- Configurable stream quality and frame rate
- Easy installation and management
- Low CPU and memory footprint
- Works with all RTSP-compatible systems (BlueIris, etc.)

## Requirements

- Windows 10/11
- [GStreamer 1.20+ Runtime](https://gstreamer.freedesktop.org/download/) (MinGW 64-bit)

## Installation

1. Download the latest release from the [Releases](https://github.com/yourusername/display-rtsp-streamer/releases) page
2. Install GStreamer Runtime if you haven't already (select "Complete" installation)
3. Run the installer or extract the zip to a location of your choice
4. Open Command Prompt as Administrator and run:
   ```
   cd C:\path\to\extracted\folder
   display_rtsp_streamer.exe install --start
   ```

## Usage

After installation, the service will automatically start and begin streaming all your displays.

By default, streams are available at:
- `rtsp://YOUR_PC_IP:8554/display0` (first display)
- `rtsp://YOUR_PC_IP:8554/display1` (second display)
- etc.

### Command Line Options

```
USAGE:
    display_rtsp_streamer.exe [OPTIONS] [COMMAND]

OPTIONS:
    -c, --config <FILE>    Path to configuration file

COMMANDS:
    install     Install as a Windows service
    uninstall   Uninstall the Windows service
    start       Start the Windows service
    stop        Stop the Windows service
    run         Run in foreground (not as a service)
    help        Print this help information
```

### Adding to BlueIris

1. Open BlueIris
2. Add a new camera
3. Set the camera type to "RTSP/HTTP"
4. In the URL field, enter: `rtsp://YOUR_PC_IP:8554/display0`
5. Configure other settings as desired
6. Click "OK" to save

## Configuration

The default configuration file is located at:
`C:\Users\<username>\AppData\Roaming\display_rtsp_streamer\config.toml`

You can modify the following settings:

```toml
# IP address to advertise in RTSP URLs (defaults to local IP)
server_address = "192.168.1.100"

# Port for the RTSP server
rtsp_port = 8554

# Frames per second to capture and stream
frame_rate = 15

# Video quality (0-10, higher is better quality but more bandwidth)
quality = 7

# Whether to capture cursor in the screen capture
capture_cursor = true

# List of display indices to capture (empty = all displays)
displays = []
```

After modifying the configuration, restart the service:

```
display_rtsp_streamer.exe stop
display_rtsp_streamer.exe start
```

## Building from Source

### Prerequisites

- Rust toolchain (rustup)
- GStreamer development libraries
- Windows 10 SDK

### Build Steps

```
git clone https://github.com/yourusername/display-rtsp-streamer.git
cd display-rtsp-streamer
cargo build --release
```

## License

MIT License