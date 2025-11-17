#!/bin/bash
# Build script for Linux

set -e

APP_NAME="now-playing"
VERSION="1.0.0"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Build release binary
echo "Building release binary for Linux..."
cargo build --release

# Create distribution directory
DIST_DIR="dist/linux"
echo "Creating distribution directory..."
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Copy binary
echo "Copying binary..."
cp "target/release/$APP_NAME" "$DIST_DIR/$APP_NAME"

# Create .desktop file for Linux desktop integration
echo "Creating desktop entry..."
cat > "$DIST_DIR/now-playing.desktop" << EOF
[Desktop Entry]
Version=$VERSION
Type=Application
Name=Now Playing
Comment=Music monitoring and display application
Exec=$APP_NAME
Icon=now-playing
Terminal=false
Categories=AudioVideo;Audio;Music;
StartupWMClass=now-playing
EOF

# Copy resources
cp assets/default_template.html "$DIST_DIR/default_template.html"
cp assets/template_wrapper.html "$DIST_DIR/template_wrapper.html"

# Create installation script
cat > "$DIST_DIR/install.sh" << 'EOF'
#!/bin/bash
# Installation script for Now Playing on Linux

set -e

APP_NAME="now-playing"
INSTALL_DIR="$HOME/.local/bin"
DESKTOP_DIR="$HOME/.local/share/applications"
CONFIG_DIR="$HOME/.config/now-playing"

echo "Installing Now Playing..."

# Create directories if they don't exist
mkdir -p "$INSTALL_DIR"
mkdir -p "$DESKTOP_DIR"
mkdir -p "$CONFIG_DIR"

# Copy binary
echo "Installing binary to $INSTALL_DIR..."
cp "$APP_NAME" "$INSTALL_DIR/$APP_NAME"
chmod +x "$INSTALL_DIR/$APP_NAME"

# Copy desktop file
echo "Installing desktop entry..."
cp now-playing.desktop "$DESKTOP_DIR/now-playing.desktop"

# Update desktop database
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$DESKTOP_DIR"
fi

echo ""
echo "Installation complete"
echo ""
echo "Binary installed to: $INSTALL_DIR/$APP_NAME"
echo "Desktop entry: $DESKTOP_DIR/now-playing.desktop"
echo "Config directory: $CONFIG_DIR"
echo ""
echo "Make sure $INSTALL_DIR is in your PATH"
echo "You can add it by running:"
echo "  echo 'export PATH=\"\$HOME/.local/bin:\\$PATH\"' >> ~/.bashrc"
echo ""
echo "To run: $APP_NAME"
echo "Or launch from your application menu"
EOF

chmod +x "$DIST_DIR/install.sh"

# Create README
echo "Creating README..."
cat > "$DIST_DIR/README.md" << EOF
# Now Playing v$VERSION

A cross-platform music monitoring and display application for Linux.

## Supported Music Players

- **Cider** - Cross-platform Apple Music client

Note: Apple Music is only available on macOS.

## Installation

### Quick Install (Recommended)

\`\`\`bash
./install.sh
\`\`\`

This will install the application to \`~/.local/bin\` and create a desktop entry.

### Manual Installation

1. Copy the binary to a directory in your PATH:
   \`\`\`bash
   cp now-playing ~/.local/bin/
   chmod +x ~/.local/bin/now-playing
   \`\`\`

2. (Optional) Copy the desktop file:
   \`\`\`bash
   cp now-playing.desktop ~/.local/share/applications/
   \`\`\`

## Usage

1. Run the application:
   \`\`\`bash
   now-playing
   \`\`\`
   Or launch from your application menu.

2. Configure your music service (Cider) in the application.

3. Click "Start" to begin monitoring.

4. The application creates a web server that provides:
   - A browser-based widget for OBS/streaming
   - Real-time track information
   - Progress bar and album artwork

## Configuration

Configuration file location: \`~/.config/now-playing/config.toml\`

## Dependencies

- Cider music player (for music monitoring)
- A modern Linux distribution with glibc 2.31+ or musl

## Troubleshooting

### Application won't start
- Make sure you have the required dependencies installed
- Check that you have a compatible glibc version: \`ldd --version\`

### Can't connect to Cider
- Ensure Cider is running and the WebSocket API is enabled
- Check that Cider is listening on localhost:10767
- Verify firewall settings aren't blocking local connections

## License

Copyright (c) 2025. All rights reserved.
EOF

echo ""
echo "Linux build created: $DIST_DIR"
echo ""
echo "Distribution contents:"
ls -lh "$DIST_DIR"
echo ""
echo "To create a tarball:"
echo "   cd dist && tar -czf now-playing-linux-v$VERSION.tar.gz linux/"
echo ""
echo "To test the installation:"
echo "   cd $DIST_DIR && ./install.sh"
