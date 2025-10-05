# PowerShell script to build Windows release binary
# Builds for the current Windows architecture

#Requires -Version 5.1

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warning-Custom {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

function Write-Error-Custom {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

# Detect architecture
$arch = $env:PROCESSOR_ARCHITECTURE
switch ($arch) {
    "AMD64" {
        $target = "x86_64-pc-windows-msvc"
        $archName = "x86_64"
    }
    "ARM64" {
        $target = "aarch64-pc-windows-msvc"
        $archName = "aarch64"
    }
    default {
        Write-Error-Custom "Unsupported architecture: $arch"
        exit 1
    }
}

# Get version from Cargo.toml
$cargoToml = Get-Content "Cargo.toml" -Raw
if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
    $version = $matches[1]
} else {
    Write-Error-Custom "Could not extract version from Cargo.toml"
    exit 1
}

Write-Status "Building podcast-tui version: $version for Windows $archName"

# Create release directory
$releaseDir = "releases\v$version"
if (-not (Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir | Out-Null
}
Write-Status "Release directory: $releaseDir"

# Build
Write-Status "Building release binary (this may take a few minutes)..."
try {
    cargo build --release
    Write-Status "✓ Build successful!"
} catch {
    Write-Error-Custom "Build failed!"
    exit 1
}

# Package
Write-Status "Packaging binary..."
$archiveName = "podcast-tui-v$version-windows-$archName"
$archiveDir = Join-Path $releaseDir $archiveName

# Create archive directory
if (Test-Path $archiveDir) {
    Remove-Item -Recurse -Force $archiveDir
}
New-Item -ItemType Directory -Path $archiveDir | Out-Null

# Copy binary
Copy-Item "target\release\podcast-tui.exe" $archiveDir

# Copy documentation files if they exist
if (Test-Path "README.md") {
    Copy-Item "README.md" $archiveDir
}
if (Test-Path "LICENSE") {
    Copy-Item "LICENSE" $archiveDir
}
if (Test-Path "CHANGELOG.md") {
    Copy-Item "CHANGELOG.md" $archiveDir
}

# Create ZIP archive
$zipPath = Join-Path $releaseDir "$archiveName.zip"
if (Test-Path $zipPath) {
    Remove-Item $zipPath
}

Write-Status "Creating ZIP archive..."
if (Get-Command Compress-Archive -ErrorAction SilentlyContinue) {
    Compress-Archive -Path $archiveDir -DestinationPath $zipPath -CompressionLevel Optimal
    Write-Status "✓ Created $archiveName.zip"
} else {
    Write-Warning-Custom "Compress-Archive not available, archive directory created but not zipped"
}

# Generate SHA256 checksum
Write-Status "Generating SHA256 checksum..."
if (Test-Path $zipPath) {
    $hash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()
    $checksumFile = "$zipPath.sha256"
    "$hash  $archiveName.zip" | Out-File -FilePath $checksumFile -Encoding ASCII
    Write-Status "✓ Created checksum file"
}

# Display results
Write-Host ""
Write-Status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
Write-Status "Windows build complete! 🎉"
Write-Status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
Write-Status "Archive: $zipPath"

if (Test-Path $zipPath) {
    $size = (Get-Item $zipPath).Length / 1MB
    Write-Host ("Size: {0:N2} MB" -f $size) -ForegroundColor Cyan
}

Write-Host ""
Write-Warning-Custom "Note: This builds only for your current platform ($archName)"
Write-Warning-Custom "To build for multiple architectures, use:"
Write-Warning-Custom "  .\scripts\build-releases-windows.ps1"
