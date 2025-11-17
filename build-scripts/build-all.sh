#!/bin/bash
# Master build script - builds for all platforms

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Now Playing - Multi-Platform Build"
echo "======================================"
echo ""

# Check which platform we're on
PLATFORM=$(uname -s)

case "$PLATFORM" in
    Darwin*)
        echo "Detected: macOS"
        echo ""
        echo "Available builds:"
        echo "  1) macOS .app bundle (native)"
        echo "  2) Linux binary"
        echo "  3) Windows binary (cross-compile via MinGW)"
        echo "  4) All platforms"
        echo ""
        read -p "Select build (1-4): " choice

        case $choice in
            1)
                "$SCRIPT_DIR/build-macos.sh"
                ;;
            2)
                "$SCRIPT_DIR/build-linux.sh"
                ;;
            3)
                "$SCRIPT_DIR/build-windows.sh"
                ;;
            4)
                "$SCRIPT_DIR/build-macos.sh"
                "$SCRIPT_DIR/build-linux.sh"
                "$SCRIPT_DIR/build-windows.sh"
                ;;
            *)
                echo "Invalid choice"
                exit 1
                ;;
        esac
        ;;

    Linux*)
        echo "Detected: Linux"
        echo ""
        echo "Building Linux binary..."
        "$SCRIPT_DIR/build-linux.sh"
        ;;

    MINGW*|MSYS*|CYGWIN*)
        echo "Detected: Windows"
        echo ""
        echo "Building Windows binary..."
        # On Windows, prefer PowerShell script if available
        if command -v powershell.exe &> /dev/null; then
            powershell.exe -ExecutionPolicy Bypass -File "$SCRIPT_DIR/build-windows.ps1"
        else
            "$SCRIPT_DIR/build-windows.sh"
        fi
        ;;

    *)
        echo "Unknown platform: $PLATFORM"
        echo "Please run the platform-specific build script manually:" 
        echo "  - macOS: ./build-scripts/build-macos.sh"
        echo "  - Linux: ./build-scripts/build-linux.sh"
        echo "  - Windows: ./build-scripts/build-windows.ps1"
        exit 1
        ;;
esac

echo ""
echo "Build process complete."
