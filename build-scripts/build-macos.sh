#!/bin/bash
# Build script for macOS .app bundle with code signing

set -e

APP_NAME="Now Playing"
BUNDLE_NAME="Now Playing.app"
IDENTIFIER="monster.void.nowplaying"
VERSION="1.0.0"
BINARY_NAME="now-playing"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Build release binary
echo "Building release binary..."
cargo build --release

# Create bundle structure
echo "Creating app bundle structure..."
rm -rf "$BUNDLE_NAME"
mkdir -p "$BUNDLE_NAME/Contents/MacOS"
mkdir -p "$BUNDLE_NAME/Contents/Resources"

# Copy binary
echo "Copying binary..."
cp "target/release/$BINARY_NAME" "$BUNDLE_NAME/Contents/MacOS/$APP_NAME"

# Copy resources
echo "Copying resources..."
cp assets/default_template.html "$BUNDLE_NAME/Contents/Resources/default_template.html"
cp assets/template_wrapper.html "$BUNDLE_NAME/Contents/Resources/template_wrapper.html"

# Create Info.plist from template
echo "Creating Info.plist..."
sed -e "s/__APP_NAME__/$APP_NAME/g" \
    -e "s/__IDENTIFIER__/$IDENTIFIER/g" \
    -e "s/__VERSION__/$VERSION/g" \
    "$SCRIPT_DIR/Info.plist.template" > "$BUNDLE_NAME/Contents/Info.plist"

# Code signing
if [ -n "$CODESIGN_IDENTITY" ]; then
    echo "Code signing with identity: $CODESIGN_IDENTITY"

    # Sign the binary first
    codesign --force --sign "$CODESIGN_IDENTITY" \
        --options runtime \
        --timestamp \
        "$BUNDLE_NAME/Contents/MacOS/$APP_NAME"

    # Sign the bundle
    codesign --force --sign "$CODESIGN_IDENTITY" \
        --options runtime \
        --timestamp \
        --entitlements "$SCRIPT_DIR/Entitlements.plist" \
        "$BUNDLE_NAME"

    echo "Code signing complete"

    # Verify signature
    codesign --verify --deep --strict --verbose=2 "$BUNDLE_NAME"
    echo "Signature verified"
else
    echo "No CODESIGN_IDENTITY set, applying ad-hoc signature"
    codesign --force --sign - "$BUNDLE_NAME/Contents/MacOS/$APP_NAME"
    codesign --force --sign - "$BUNDLE_NAME"
    echo "Ad-hoc signature applied (for local use only)"
fi

echo ""
echo "Bundle created: $BUNDLE_NAME"
echo ""
echo "To install:"
echo "   cp -r \"$BUNDLE_NAME\" /Applications/"
echo ""
echo "To distribute with proper code signing:"
echo "   export CODESIGN_IDENTITY=\"Developer ID Application: Your Name (TEAM_ID)\""
echo "   ./build-scripts/build-macos.sh"
echo ""
echo "For notarization:"
echo "   xcrun notarytool submit \"$BUNDLE_NAME\" --keychain-profile \"AC_PASSWORD\" --wait"
echo "   xcrun stapler staple \"$BUNDLE_NAME\""
