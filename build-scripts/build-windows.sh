#!/bin/bash
# Build script for Windows executable

set -e

APP_NAME="Now Playing"
BINARY_NAME="now-playing"
VERSION="1.0.0"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Build release binary for Windows
echo "Building release binary for Windows..."
cargo build --release --target x86_64-pc-windows-gnu

# Create distribution directory
DIST_DIR="dist/windows"
echo "Creating distribution directory..."
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Copy binary
echo "Copying binary..."
cp "target/x86_64-pc-windows-gnu/release/$BINARY_NAME.exe" "$DIST_DIR/$APP_NAME.exe"

# Create README
echo "Creating README..."
cat > "$DIST_DIR/README.txt" << EOF
Now Playing v$VERSION
====================

A cross-platform music monitoring and display application.

Supports:
- Apple Music (macOS only)
- Cider (all platforms)

To run:
1. Double-click "$APP_NAME.exe"
2. Configure your music service in the application
3. Click "Start" to begin monitoring

The application will create a web server that provides:
- A browser-based widget for OBS/streaming
- Real-time track information
- Progress bar and album artwork

For more information, visit:
https://github.com/yourusername/now-playing

Configuration file location:
%APPDATA%\\monster\\void\\NowPlaying\\config.toml

Copyright (c) 2025. All rights reserved.
EOF

# Copy resources
cp assets/default_template.html "$DIST_DIR/default_template.html"
cp assets/template_wrapper.html "$DIST_DIR/template_wrapper.html"

# Create a basic batch file launcher
cat > "$DIST_DIR/Now Playing.bat" << 'EOF'
@echo off
start "" "%~dp0Now Playing.exe"
EOF

echo ""
echo "Windows build created: $DIST_DIR"
echo ""
echo "Distribution contents:"
ls -lh "$DIST_DIR"
echo ""
echo "To create a ZIP archive:"
echo "   cd dist && zip -r \"now-playing-windows-v$VERSION.zip\" windows/"
echo ""
echo "Note: This uses MinGW cross-compilation. For native Windows builds, run this on Windows with MSVC:"
echo "   cargo build --release"
