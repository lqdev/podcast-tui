# PowerShell script to install build dependencies on Windows
# Run as Administrator if you need to install Rust

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

Write-Status "Installing build dependencies for Windows..."

# Check if Rust is installed
try {
    $rustVersion = rustc --version
    Write-Status "Rust is already installed: $rustVersion"
} catch {
    Write-Error-Custom "Rust is not installed!"
    Write-Host ""
    Write-Host "Please install Rust from: https://rustup.rs/" -ForegroundColor Cyan
    Write-Host "After installation, restart PowerShell and run this script again."
    exit 1
}

# Check if cargo is available
try {
    $cargoVersion = cargo --version
    Write-Status "Cargo is available: $cargoVersion"
} catch {
    Write-Error-Custom "Cargo is not available!"
    exit 1
}

# Add MSVC targets (usually already installed on Windows)
Write-Status "Ensuring Windows MSVC targets are installed..."
rustup target add x86_64-pc-windows-msvc | Out-Null
rustup target add aarch64-pc-windows-msvc | Out-Null
Write-Status "✓ Windows MSVC targets installed"

# Check if Visual Studio Build Tools are available
Write-Status "Checking for Visual Studio Build Tools..."
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vsWhere) {
    $vsInstallPath = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($vsInstallPath) {
        Write-Status "✓ Visual Studio Build Tools found at: $vsInstallPath"
    } else {
        Write-Warning-Custom "Visual Studio Build Tools not found"
        Write-Warning-Custom "You may need to install Visual Studio Build Tools for C++ development"
        Write-Warning-Custom "Download from: https://visualstudio.microsoft.com/downloads/"
    }
} else {
    Write-Warning-Custom "vswhere.exe not found - Visual Studio may not be installed"
    Write-Warning-Custom "For native builds, install Visual Studio Build Tools"
    Write-Warning-Custom "Download from: https://visualstudio.microsoft.com/downloads/"
}

# Verify build works
Write-Status "Verifying Rust build system..."
try {
    $testBuild = cargo build --help
    Write-Status "✓ Cargo build system is working"
} catch {
    Write-Error-Custom "Cargo build system verification failed"
    exit 1
}

Write-Host ""
Write-Status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
Write-Status "Dependencies installed successfully! 🎉"
Write-Status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
Write-Host ""
Write-Status "You can now run: .\scripts\build-windows.ps1"
Write-Host ""
