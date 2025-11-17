# Build script for Windows (PowerShell)
# Run this on Windows with: .\build-windows.ps1

$ErrorActionPreference = "Stop"

$APP_NAME = "Now Playing"
$BINARY_NAME = "now-playing"
$VERSION = "1.0.0"

$SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
$PROJECT_ROOT = Split-Path -Parent $SCRIPT_DIR

Set-Location $PROJECT_ROOT

# Build release binary
Write-Host "Building release binary..." -ForegroundColor Green
cargo build --release

# Create distribution directory
$DIST_DIR = "dist\windows"
Write-Host "Creating distribution directory..." -ForegroundColor Green
if (Test-Path $DIST_DIR) {
    Remove-Item -Recurse -Force $DIST_DIR
}
New-Item -ItemType Directory -Force -Path $DIST_DIR | Out-Null

# Copy binary
Write-Host "Copying binary..." -ForegroundColor Green
Copy-Item "target\release\$BINARY_NAME.exe" "$DIST_DIR\$APP_NAME.exe"

# Create README
Write-Host "Creating README..." -ForegroundColor Green
$readme = @"
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
%APPDATA%\monster\void\NowPlaying\config.toml

Copyright (c) 2025. All rights reserved.
"@
$readme | Out-File -FilePath "$DIST_DIR\README.txt" -Encoding ASCII

# Create a batch file launcher
$batchContent = @"
@echo off
start "" "%~dp0$APP_NAME.exe"
"@
$batchContent | Out-File -FilePath "$DIST_DIR\$APP_NAME.bat" -Encoding ASCII

Write-Host ""
Write-Host "Windows build created: $DIST_DIR" -ForegroundColor Green
Write-Host ""
Write-Host "Distribution contents:" -ForegroundColor Cyan
Get-ChildItem $DIST_DIR | Format-Table Name, Length
Write-Host ""
Write-Host "To create a ZIP archive:" -ForegroundColor Yellow
Write-Host "   Compress-Archive -Path '$DIST_DIR' -DestinationPath 'dist\now-playing-windows-v$VERSION.zip'"
Write-Host ""
Write-Host "Build complete." -ForegroundColor Green
