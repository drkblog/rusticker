# PowerShell script to package Windows release binaries

$ErrorActionPreference = "Stop"

# Resolve directories
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$WorkspaceRoot = Resolve-Path (Join-Path $ScriptDir "..\..")
$TargetDir = Join-Path $ScriptDir "target"
$StagingDir = Join-Path $TargetDir "staging"

Write-Host "=== Windows Packager Script ==="
Write-Host "Script directory: $ScriptDir"
Write-Host "Workspace root:   $WorkspaceRoot"

# Ensure we run from workspace root
Set-Location $WorkspaceRoot

# Parse version from main/Cargo.toml
$CargoTomlPath = Join-Path $WorkspaceRoot "main\Cargo.toml"
if (-not (Test-Path $CargoTomlPath)) {
    Write-Error "Error: main/Cargo.toml not found at $CargoTomlPath"
    exit 1
}

$CargoToml = Get-Content $CargoTomlPath -Raw
if ($CargoToml -match 'version\s*=\s*"([^"]+)"') {
    $Version = $Matches[1]
} else {
    Write-Error "Error: Could not parse version from main/Cargo.toml"
    exit 1
}
Write-Host "Parsed Version:   $Version"

# Clean and recreate target/staging folders
if (Test-Path $TargetDir) { 
    Write-Host "--> Cleaning existing target directory..."
    Remove-Item -Recurse -Force $TargetDir 
}
New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null

Write-Host "--> Building release binaries..."
cargo build --release --workspace

Write-Host "--> Copying binaries and docs to staging..."
Copy-Item "target\release\rusticker.exe" -Destination $StagingDir
Copy-Item "target\release\stickerize.exe" -Destination $StagingDir
if (Test-Path "README.md") { Copy-Item "README.md" -Destination $StagingDir }
if (Test-Path "LICENSE") { Copy-Item "LICENSE" -Destination $StagingDir }

# Create ZIP archive
$ZipName = "rusticker-v$Version-windows-x64.zip"
$ZipPath = Join-Path $TargetDir $ZipName

Write-Host "--> Compressing staging files into $ZipName..."
Compress-Archive -Path "$StagingDir\*" -DestinationPath $ZipPath

# Clean staging directory
Remove-Item -Recurse -Force $StagingDir

Write-Host "=== Package created successfully ==="
Write-Host "Archive located at: $ZipPath"
