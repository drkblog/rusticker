#!/usr/bin/env zsh

# Resolve absolute path of workspace root
SCRIPT_DIR="${0:A:h}"
WORKSPACE_ROOT="${SCRIPT_DIR:h:h}"

cd "$WORKSPACE_ROOT"

# Function to report error to stderr and exit
err() {
    echo "ERROR: $1" >&2
    exit 1
}

# 1. Get source of truth version from main/Cargo.toml
if [[ ! -f "main/Cargo.toml" ]]; then
    err "main/Cargo.toml not found"
fi
VERSION=$(grep -m 1 '^version = ' main/Cargo.toml | cut -d '"' -f 2)
if [[ -z "$VERSION" ]]; then
    err "Could not parse version from main/Cargo.toml"
fi

# 2. Check other Cargo.toml files in the workspace
CRATES=(background_remover mask_generator pdf_generator stickerize)
for crate in $CRATES; do
    cargo_file="$crate/Cargo.toml"
    if [[ ! -f "$cargo_file" ]]; then
        err "$cargo_file not found"
    fi
    crate_version=$(grep -m 1 '^version = ' "$cargo_file" | cut -d '"' -f 2)
    if [[ "$crate_version" != "$VERSION" ]]; then
        err "Crate '$crate' has version '$crate_version', but main has '$VERSION'"
    fi
done

# 3. Check README.md consistency
if [[ ! -f "README.md" ]]; then
    err "README.md not found"
fi

if ! grep -q "rusticker-${VERSION}.pkg" README.md; then
    err "README.md does not reference the correct macOS package: rusticker-${VERSION}.pkg"
fi

if ! grep -q "rusticker-v${VERSION}-windows-x64.zip" README.md; then
    err "README.md does not reference the correct Windows ZIP: rusticker-v${VERSION}-windows-x64.zip"
fi

if ! grep -q "packager/windows/winget/${VERSION}" README.md; then
    err "README.md does not reference the correct WinGet manifest directory: packager/windows/winget/${VERSION}"
fi

# 4. Check WinGet Manifests
WINGET_DIR="packager/windows/winget/${VERSION}"
if [[ ! -d "$WINGET_DIR" ]]; then
    err "WinGet manifest directory '$WINGET_DIR' does not exist"
fi

MANIFESTS=(
    "drkbugs.rusticker.yaml"
    "drkbugs.rusticker.locale.en-US.yaml"
    "drkbugs.rusticker.installer.yaml"
)

for manifest in $MANIFESTS; do
    file_path="$WINGET_DIR/$manifest"
    if [[ ! -f "$file_path" ]]; then
        err "WinGet manifest file '$file_path' does not exist"
    fi
    
    if ! grep -q "PackageVersion: ${VERSION}" "$file_path"; then
        err "WinGet manifest file '$file_path' does not contain 'PackageVersion: ${VERSION}'"
    fi
done

# Check installer URL and ZIP filename in installer.yaml
INSTALLER_YAML="$WINGET_DIR/drkbugs.rusticker.installer.yaml"
EXPECTED_URL="https://github.com/drkblog/rusticker/releases/download/v${VERSION}/rusticker-v${VERSION}-windows-x64.zip"
if ! grep -q "$EXPECTED_URL" "$INSTALLER_YAML"; then
    err "Installer manifest '$INSTALLER_YAML' does not contain the expected download URL: $EXPECTED_URL"
fi

# All checks passed, output version to stdout
echo "$VERSION"
