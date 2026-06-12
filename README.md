# Now Playing

A cross-platform desktop application for monitoring music playback with OBS WebSocket integration and a customizable web widget interface.

## Features

- Multi-platform music monitoring:
  - Apple Music (macOS only, via AppleScript)
  - Cider (cross-platform, via WebSocket)
- Web server with customizable HTML templates
- Live template rendering with placeholders
- OBS WebSocket integration for automatic scene/source management
- GUI configuration interface
- Server-Sent Events (SSE) for live updates
- JSON API for current track information
- Album artwork support

## Quick Start

### Build and Run

```sh
cargo build --release
cargo run --release
```

The application will launch a GUI where you can:
1. Select your music service (Apple Music or Cider)
2. Configure server settings
3. Set up OBS WebSocket connection (optional)
4. Customize the HTML widget template
5. Start the monitoring and web server

### Creating Distribution Packages

#### All Platforms (Interactive)
```sh
./build-scripts/build-all.sh
```

#### Platform-Specific

**macOS:**
```sh
./build-scripts/build-macos.sh
```
Creates `Now Playing.app` bundle with code signing support.

**Linux:**
```sh
./build-scripts/build-linux.sh
```
Creates distribution package in `dist/linux/` with installer.

**Windows:**
```powershell
# On Windows
.\build-scripts\build-windows.ps1

# Or cross-compile from macOS/Linux
./build-scripts/build-windows.sh
```
Creates distribution package in `dist/windows/`.

See [build-scripts/README.md](build-scripts/README.md) for detailed build documentation.

## Music Service Configuration

### Apple Music (macOS only)
- Requires macOS with Apple Music/iTunes installed
- Uses AppleScript for track monitoring
- No additional setup required

### Cider (All platforms)
- Install [Cider](https://cider.sh/)
- Ensure WebSocket API is enabled (default: localhost:10767)
- Configure host and port in the application

## Configuration

Configuration is stored in:
- macOS: `~/Library/Application Support/monster/void/NowPlaying/config.toml`
- Linux: `~/.config/now-playing/config.toml`
- Windows: `%APPDATA%\monster\void\NowPlaying\config.toml`

### Configuration Options

```toml
[service]
service_type = "Cider"  # or "AppleMusic" (macOS only)

[service.cider]
host = "localhost"
port = 10767

[server]
host = "127.0.0.1"  # Use "0.0.0.0" for all IPv4, "::" for all IPv6
port = 8765

[obs]
enabled = false
host = "localhost"
port = 4455
password = ""
scene_name = "Now Playing"
source_name = "Now Playing"

[widget]
width = 360
height = 160
template = "..." # HTML template with placeholders
```

### Server Host Configuration

The `server.host` setting controls which network interfaces the web server listens on:

- **`127.0.0.1`** (default) - Only accepts connections from localhost (most secure)
- **`0.0.0.0`** - Listens on all IPv4 interfaces (allows remote connections)
- **`::`** - Listens on all IPv6 interfaces (allows remote connections)
- **Specific IP** - Only listens on a specific network interface

⚠️ **Security Note**: When using `0.0.0.0` or `::`, the web server will be accessible from other devices on your network. Use firewall rules if needed.

## HTML Template System

The widget uses a simple template system with placeholders:

```html
<div>
    <img src="{artwork_base64}" alt="Album Art" />
    <h1>{title}</h1>
    <p>{artist} - {album}</p>
    <p>State: {state}</p>
    <div class="progress" style="width: {progress}%"></div>
    <span>{current_time} / {duration_time}</span>
</div>
```

### Available Placeholders

- `{title}` - Track title
- `{artist}` - Artist name
- `{album}` - Album name
- `{state}` - Playback state (playing, paused)
- `{progress}` - Progress percentage (0-100)
- `{current_time}` - Current position (formatted as M:SS)
- `{duration_time}` - Total duration (formatted as M:SS)
- `{position}` - Raw position in seconds
- `{duration}` - Raw duration in seconds
- `{artwork_base64}` - Base64-encoded album artwork (or URL for Cider)

## OBS Integration

The application can connect to OBS via WebSocket and:

1. **Auto-update existing sources**: If a scene and browser source exist with the configured names, the app will update the URL automatically
2. **Create scene and source**: Click the button in the GUI to create a new scene with a browser source pointing to the widget

### OBS WebSocket Setup

1. In OBS, go to Tools -> WebSocket Server Settings
2. Enable WebSocket server
3. Note the port (default: 4455) and set a password
4. In Now Playing app, enter these credentials
5. Click "Connect to OBS"
6. Click "Create Scene & Source" to set up the widget in OBS

## API Endpoints

Once the server is running:

### GET /
Returns the rendered HTML widget with current track data

### GET /widget.html
Same as `/`

### GET /now-playing
Returns current track information as JSON:
```json
{
  "title": "Song Title",
  "artist": "Artist Name",
  "album": "Album Name",
  "state": "playing",
  "position": 45.2,
  "duration": 180.5
}
```

### GET /events
Server-Sent Events stream that pushes updates whenever track changes

### GET /health
Health check endpoint, returns "ok"

## Requirements

- **macOS** (for Apple Music AppleScript integration)
- Rust 1.70 or later
- Apple Music application
- OBS Studio with WebSocket plugin (optional, for OBS integration)

## Development

Build in development mode:
```sh
cargo build
```

Run with logging:
```sh
RUST_LOG=info cargo run
```

## Security / Permissions

- **AppleScript**: First run may prompt for permission to control Apple Music
- **Network**: App needs permission to create local web server
- **OBS WebSocket**: Requires WebSocket server enabled in OBS

## Project Structure

- `src/main.rs` - GUI application entry point
- `src/lib.rs` - Library exports
- `src/apple_music.rs` - Apple Music monitoring via AppleScript
- `src/webserver.rs` - HTTP server with template rendering
- `src/template.rs` - HTML template engine
- `src/config.rs` - Configuration management
- `src/obs_client.rs` - OBS WebSocket client
- `src/rt.rs` - Global Tokio runtime

## License

Licensed under the [GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0-only).
