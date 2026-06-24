#!/usr/bin/env zsh

# Exit immediately if a command exits with a non-zero status
set -e

# Resolve the absolute path of the directory where this script resides
SCRIPT_DIR="${0:A:h}"
WORKSPACE_ROOT="${SCRIPT_DIR:h:h}"
TARGET_DIR="$SCRIPT_DIR/target"
STAGING_DIR="$TARGET_DIR/staging"

echo "=== Windows Cross-Packager Script (Zsh) ==="
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

# Clean and recreate target/staging folders
rm -rf "$TARGET_DIR"
mkdir -p "$STAGING_DIR"

echo "--> Building release binaries for Windows (x86_64-pc-windows-msvc) using cargo-xwin..."
cargo xwin build --release --target x86_64-pc-windows-msvc

echo "--> Copying binaries and docs to staging..."
cp "target/x86_64-pc-windows-msvc/release/rusticker.exe" "$STAGING_DIR/"
cp "target/x86_64-pc-windows-msvc/release/stickerize.exe" "$STAGING_DIR/"
if [[ -f "README.md" ]]; then
    cp "README.md" "$STAGING_DIR/"
fi
if [[ -f "LICENSE" ]]; then
    cp "LICENSE" "$STAGING_DIR/"
fi

# Create ZIP archive
ZIP_NAME="rusticker-v$VERSION-windows-x64.zip"
ZIP_PATH="$TARGET_DIR/$ZIP_NAME"

echo "--> Compressing staging files into $ZIP_NAME..."
# Zip files from the staging folder so they reside at the root of the archive
(cd "$STAGING_DIR" && zip -r -q "$ZIP_PATH" .)

# Clean staging directory
rm -rf "$STAGING_DIR"

echo "=== Package created successfully ==="
echo "Archive located at: $ZIP_PATH"
ls -lh "$ZIP_PATH"
