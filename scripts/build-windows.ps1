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

# Detect architecture and check for available toolchain
$arch = $env:PROCESSOR_ARCHITECTURE
$linkExeAvailable = Get-Command link.exe -ErrorAction SilentlyContinue

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

# Check if MSVC linker is available, if not try to initialize VS environment
if (-not $linkExeAvailable) {
    Write-Status "MSVC linker not in PATH, attempting to initialize Visual Studio environment..."
    
    # Common Visual Studio installation paths
    $vsPaths = @(
        "C:\Program Files\Microsoft Visual Studio\2022\Community",
        "C:\Program Files\Microsoft Visual Studio\2022\Professional",
        "C:\Program Files\Microsoft Visual Studio\2022\Enterprise",
        "C:\Program Files\Microsoft Visual Studio\2022\BuildTools",
        "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community",
        "C:\Program Files (x86)\Microsoft Visual Studio\2022\Professional",
        "C:\Program Files (x86)\Microsoft Visual Studio\2022\Enterprise",
        "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
    )
    
    $vcvarsall = $null
    foreach ($vsPath in $vsPaths) {
        $testPath = Join-Path $vsPath "VC\Auxiliary\Build\vcvarsall.bat"
        if (Test-Path $testPath) {
            $vcvarsall = $testPath
            break
        }
    }
    
    if ($vcvarsall) {
        Write-Status "Found Visual Studio at: $vcvarsall"
        
        # Determine the correct architecture argument for vcvarsall
        $vcArch = if ($arch -eq "ARM64") { "arm64" } else { "x64" }
        
        Write-Status "Initializing MSVC environment for $vcArch..."
        
        # Run vcvarsall and capture environment variables
        $tempFile = [System.IO.Path]::GetTempFileName()
        cmd /c "`"$vcvarsall`" $vcArch && set" | Out-File -FilePath $tempFile -Encoding ASCII
        
        # Parse and set environment variables
        Get-Content $tempFile | ForEach-Object {
            if ($_ -match '^([^=]+)=(.*)$') {
                $varName = $matches[1]
                $varValue = $matches[2]
                # Update important environment variables
                if ($varName -eq "PATH" -or $varName -eq "LIB" -or $varName -eq "INCLUDE" -or $varName -eq "LIBPATH") {
                    Set-Item -Path "env:$varName" -Value $varValue
                }
            }
        }
        Remove-Item $tempFile
        
        # Verify link.exe is now available
        $linkExeAvailable = Get-Command link.exe -ErrorAction SilentlyContinue
        if ($linkExeAvailable) {
            Write-Status "✓ MSVC environment initialized successfully!"
        } else {
            Write-Error-Custom "Failed to initialize MSVC environment"
            exit 1
        }
    } else {
        Write-Error-Custom "MSVC linker (link.exe) not found and Visual Studio installation not detected!"
        Write-Host ""
        Write-Host "Building native Windows applications requires Visual Studio Build Tools." -ForegroundColor Yellow
        Write-Host ""
        Write-Host "Please install:" -ForegroundColor Cyan
        Write-Host "  1. Download from: https://visualstudio.microsoft.com/downloads/" -ForegroundColor Cyan
        Write-Host "  2. Select 'Build Tools for Visual Studio 2022'" -ForegroundColor Cyan
        Write-Host "  3. Choose 'Desktop development with C++'" -ForegroundColor Cyan
        if ($arch -eq "ARM64") {
            Write-Host "  4. Ensure ARM64 build tools are selected" -ForegroundColor Cyan
        }
        Write-Host "  4. Restart your terminal after installation" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "For detailed instructions, see: .\scripts\INSTALL-MSVC-TOOLS.md" -ForegroundColor Green
        Write-Host ""
        exit 1
    }
}

# Check for clang/LLVM (required for some dependencies on ARM64)
if ($arch -eq "ARM64") {
    $clangAvailable = Get-Command clang -ErrorAction SilentlyContinue
    
    # If not in PATH, check common installation locations
    if (-not $clangAvailable) {
        $llvmPaths = @(
            "C:\Program Files\LLVM\bin",
            "C:\Program Files (x86)\LLVM\bin",
            "$env:ProgramFiles\LLVM\bin",
            "${env:ProgramFiles(x86)}\LLVM\bin"
        )
        
        foreach ($llvmPath in $llvmPaths) {
            if (Test-Path "$llvmPath\clang.exe") {
                Write-Status "Found LLVM at: $llvmPath"
                Write-Status "Adding LLVM to PATH for this session..."
                $env:PATH += ";$llvmPath"
                $clangAvailable = $true
                break
            }
        }
    }
    
    if (-not $clangAvailable) {
        Write-Warning-Custom "Clang/LLVM not found - required for ARM64 builds"
        Write-Host ""
        Write-Host "Some Rust dependencies require LLVM/Clang for ARM64 Windows." -ForegroundColor Yellow
        Write-Host ""
        Write-Host "Quick install:" -ForegroundColor Cyan
        Write-Host "  winget install LLVM.LLVM" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "Then restart your terminal and run this script again." -ForegroundColor Cyan
        Write-Host ""
        Write-Host "For detailed instructions, see: .\scripts\INSTALL-LLVM.md" -ForegroundColor Green
        Write-Host ""
        exit 1
    } else {
        $clangVersion = clang --version 2>&1 | Select-Object -First 1
        Write-Status "✓ Clang/LLVM found: $clangVersion"
    }
}

# Ensure target is installed
Write-Status "Using MSVC toolchain: $target"
$installedTargets = rustup target list --installed
if (-not ($installedTargets -match $target)) {
    Write-Status "Installing target: $target"
    rustup target add $target
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Failed to install target $target"
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
Write-Status "Target: $target"
cargo build --release --target $target
if ($LASTEXITCODE -ne 0) {
    Write-Error-Custom "Build failed with exit code $LASTEXITCODE"
    Write-Host ""
    Write-Host "This usually means dependencies failed to compile." -ForegroundColor Yellow
    Write-Host "Check the error messages above for details." -ForegroundColor Yellow
    exit 1
}
Write-Status "✓ Build successful!"

# Package
Write-Status "Packaging binary..."
$archiveName = "podcast-tui-v$version-windows-$archName"
$archiveDir = Join-Path $releaseDir $archiveName

# Create archive directory
if (Test-Path $archiveDir) {
    Remove-Item -Recurse -Force $archiveDir
}
New-Item -ItemType Directory -Path $archiveDir | Out-Null

# Copy binary (check for both .exe and no extension in target-specific directory)
$targetBinaryPath = "target\$target\release\podcast-tui.exe"
$targetBinaryPathNoExt = "target\$target\release\podcast-tui"
$binaryPath = "target\release\podcast-tui.exe"
$binaryPathNoExt = "target\release\podcast-tui"

$sourceBinary = $null
if (Test-Path $targetBinaryPath) {
    $sourceBinary = $targetBinaryPath
} elseif (Test-Path $targetBinaryPathNoExt) {
    $sourceBinary = $targetBinaryPathNoExt
} elseif (Test-Path $binaryPath) {
    $sourceBinary = $binaryPath
} elseif (Test-Path $binaryPathNoExt) {
    $sourceBinary = $binaryPathNoExt
} else {
    Write-Error-Custom "Could not find podcast-tui binary in target directories"
    Write-Error-Custom "Checked: $targetBinaryPath, $targetBinaryPathNoExt, $binaryPath, $binaryPathNoExt"
    exit 1
}

# Sign binary if certificate is available (optional, won't fail if no cert)
$signScript = "scripts\sign-windows-binary.ps1"
if (Test-Path $signScript) {
    Write-Status "Checking for code signing certificate..."
    try {
        & $signScript -BinaryPath $sourceBinary -SkipIfNoCert
        if ($LASTEXITCODE -eq 0) {
            Write-Status "✓ Binary signing completed"
        }
    } catch {
        Write-Warning-Custom "Code signing skipped: $_"
    }
} else {
    Write-Warning-Custom "Code signing script not found, skipping signing"
}

# Copy signed binary to archive directory
Copy-Item $sourceBinary (Join-Path $archiveDir "podcast-tui.exe")

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
    
    # Clean up archive directory to avoid conflicts when extracting
    Remove-Item -Recurse -Force $archiveDir
    Write-Status "✓ Cleaned up temporary directory"
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
