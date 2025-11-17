# Build Scripts

This directory contains build scripts for creating distributable packages of Now Playing for different platforms.

## Quick Start

### Build for Your Platform

```bash
./build-scripts/build-all.sh
```

This will detect your platform and build accordingly. On macOS, it will offer options to build for multiple platforms.

### Platform-Specific Builds

#### macOS
```bash
./build-scripts/build-macos.sh
```

Creates a `.app` bundle in the project root. Supports code signing if `CODESIGN_IDENTITY` is set.

**Output:** `Now Playing.app`

**Installation:**
```bash
cp -r "Now Playing.app" /Applications/
```

#### Linux
```bash
./build-scripts/build-linux.sh
```

Creates a distribution package with binary, desktop file, and installation script.

**Output:** `dist/linux/`

**Installation:**
```bash
cd dist/linux && ./install.sh
```

#### Windows

**On Windows (PowerShell):**
```powershell
.\build-scripts\build-windows.ps1
```

**On macOS/Linux (cross-compile):**
```bash
./build-scripts/build-windows.sh
```

Creates a distribution package with executable, launcher, and README.

**Output:** `dist/windows/`

## File Structure

build-scripts/
- build-all.sh              # Master build script (auto-detects platform)
- build-macos.sh            # macOS app bundle builder
- build-linux.sh            # Linux distribution builder
- build-windows.sh          # Windows builder (bash/cross-compile)
- build-windows.ps1         # Windows builder (PowerShell/native)
- Info.plist.template       # macOS app bundle metadata
- entitlements.plist        # macOS code signing entitlements
- README.md                 # This file

## Code Signing (macOS)

For distributable macOS builds, set your code signing identity:

```bash
export CODESIGN_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
./build-scripts/build-macos.sh
```

For local testing, the script will apply ad-hoc signatures automatically.

### Notarization (macOS)

For distribution outside the App Store:

```bash
# Build with proper signing
export CODESIGN_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
./build-scripts/build-macos.sh

# Submit for notarization
xcrun notarytool submit "Now Playing.app" \
    --keychain-profile "AC_PASSWORD" \
    --wait

# Staple the notarization ticket
xcrun stapler staple "Now Playing.app"
```

## Creating Distribution Archives

### macOS
```bash
# Create a DMG (requires create-dmg tool)
create-dmg \
    --volname "Now Playing" \
    --window-pos 200 120 \
    --window-size 600 400 \
    --icon-size 100 \
    --app-drop-link 425 120 \
    "now-playing-macos-v1.0.0.dmg" \
    "Now Playing.app"

# Or create a simple ZIP
zip -r now-playing-macos-v1.0.0.zip "Now Playing.app"
```

### Linux
```bash
cd dist
tar -czf now-playing-linux-v1.0.0.tar.gz linux/
```

### Windows
```bash
cd dist
zip -r now-playing-windows-v1.0.0.zip windows/
```

Or on Windows PowerShell:
```powershell
Compress-Archive -Path dist\windows -DestinationPath dist\now-playing-windows-v1.0.0.zip
```

## Cross-Compilation

### Windows from macOS/Linux

Install MinGW:

**macOS:**
```bash
brew install mingw-w64
rustup target add x86_64-pc-windows-gnu
```

**Linux:**
```bash
sudo apt-get install mingw-w64
rustup target add x86_64-pc-windows-gnu
```

Then run:
```bash
./build-scripts/build-windows.sh
```

## Platform Support Matrix

| Platform | Native Build | Cross-Compile From | Notes |
|----------|-------------|-------------------|-------|
| macOS | native macOS build supported | not supported | Requires macOS for code signing |
| Linux | native Linux build supported | can be cross-compiled from macOS/Windows | Binary compatible with most distros |
| Windows | native Windows build supported | can be cross-compiled from macOS/Linux | Cross-compile via MinGW |

## Troubleshooting

### macOS: "App is damaged and can't be opened"
- The app needs to be code-signed for distribution
- For local use, you may need to run: `xattr -cr "Now Playing.app"`

### Linux: "cannot execute binary file"
- Check architecture: `file dist/linux/now-playing`
- Ensure you built for the correct target architecture

### Windows: Missing DLL errors
- When cross-compiling, ensure all dependencies are statically linked
- For native builds on Windows, use the PowerShell script

## Version Management

Version numbers are defined at the top of each build script:

```bash
VERSION="1.0.0"
```

Update this in all scripts before creating release builds.

## License

Copyright (c) 2025. All rights reserved.
