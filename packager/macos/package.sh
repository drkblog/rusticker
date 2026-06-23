#!/usr/bin/env zsh

# Exit immediately if a command exits with a non-zero status
set -e

# Resolve the absolute path of the directory where this script resides
SCRIPT_DIR="${0:A:h}"
WORKSPACE_ROOT="${SCRIPT_DIR:h:h}"
TARGET_DIR="$SCRIPT_DIR/target"
STAGING_DIR="$TARGET_DIR/staging"

echo "=== macOS Packager Script ==="
echo "Script directory: $SCRIPT_DIR"
echo "Workspace root:   $WORKSPACE_ROOT"

# Ensure we run from the workspace root
cd "$WORKSPACE_ROOT"

# Parse version from main/Cargo.toml
VERSION=$(grep -m 1 '^version = ' main/Cargo.toml | cut -d '"' -f 2)
if [[ -z "$VERSION" ]]; then
    echo "Error: Could not parse version from main/Cargo.toml"
    exit 1
fi
echo "Parsed Version:   $VERSION"

# Parse CLI options
BUILD_MODE="universal"
for arg in "$@"; do
    case $arg in
        --native)
            BUILD_MODE="native"
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --native   Build only for the host's native architecture (faster)"
            echo "  --help, -h Show this help message"
            exit 0
            ;;
    esac
done

echo "Build Mode:       $BUILD_MODE"

# Create/clean clean target directories
rm -rf "$TARGET_DIR"
mkdir -p "$STAGING_DIR/usr/local/bin"

if [[ "$BUILD_MODE" == "native" ]]; then
    echo "--> Building release binaries for host native architecture..."
    cargo build --release --workspace
    
    echo "--> Copying native binaries to staging..."
    cp "target/release/rusticker" "$STAGING_DIR/usr/local/bin/"
    cp "target/release/stickerize" "$STAGING_DIR/usr/local/bin/"
else
    echo "--> Ensuring toolchains are installed for universal build..."
    rustup target add x86_64-apple-darwin aarch64-apple-darwin
    
    echo "--> Building release binaries for x86_64-apple-darwin..."
    cargo build --release --target x86_64-apple-darwin --workspace
    
    echo "--> Building release binaries for aarch64-apple-darwin..."
    cargo build --release --target aarch64-apple-darwin --workspace
    
    echo "--> Creating universal binaries with lipo..."
    lipo -create -output "$STAGING_DIR/usr/local/bin/rusticker" \
        "target/x86_64-apple-darwin/release/rusticker" \
        "target/aarch64-apple-darwin/release/rusticker"
        
    lipo -create -output "$STAGING_DIR/usr/local/bin/stickerize" \
        "target/x86_64-apple-darwin/release/stickerize" \
        "target/aarch64-apple-darwin/release/stickerize"
fi

# Optional Code Signing of binaries
if [[ -n "$DEV_APP_CERT" ]]; then
    echo "--> Code signing binaries with Developer ID Application certificate..."
    codesign --force --options runtime --sign "$DEV_APP_CERT" "$STAGING_DIR/usr/local/bin/rusticker"
    codesign --force --options runtime --sign "$DEV_APP_CERT" "$STAGING_DIR/usr/local/bin/stickerize"
else
    echo "--> [Skipped] Code signing binaries (DEV_APP_CERT environment variable is not set)"
fi

# Build package
PKG_NAME="rusticker-$VERSION.pkg"
PKG_PATH="$TARGET_DIR/$PKG_NAME"

if [[ -n "$DEV_INSTALLER_CERT" ]]; then
    PKG_PATH_UNSIGNED="$TARGET_DIR/unsigned-$PKG_NAME"
    echo "--> Building unsigned PKG installer..."
    pkgbuild --root "$STAGING_DIR" \
             --identifier com.drkbugs.rusticker \
             --version "$VERSION" \
             --install-location / \
             "$PKG_PATH_UNSIGNED"
             
    echo "--> Signing PKG installer with Developer ID Installer certificate..."
    productsign --sign "$DEV_INSTALLER_CERT" "$PKG_PATH_UNSIGNED" "$PKG_PATH"
    rm "$PKG_PATH_UNSIGNED"
else
    echo "--> Building PKG installer (unsigned)..."
    pkgbuild --root "$STAGING_DIR" \
             --identifier com.drkbugs.rusticker \
             --version "$VERSION" \
             --install-location / \
             "$PKG_PATH"
fi

# Clean up staging directory
rm -rf "$STAGING_DIR"

echo "=== Package created successfully ==="
echo "Installer located at: $PKG_PATH"
ls -lh "$PKG_PATH"
