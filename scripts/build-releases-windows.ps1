# PowerShell script to build Windows release binaries for all architectures
# Builds both x86_64 and ARM64 versions

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

# Function to ensure LLVM is available for ARM64 builds
function Ensure-LLVM {
    Write-Status "Checking for LLVM/Clang (required for ARM64 builds)..."
    
    # Check if clang is already in PATH
    $clangCmd = Get-Command clang -ErrorAction SilentlyContinue
    if ($clangCmd) {
        Write-Status "✓ Clang found in PATH: $($clangCmd.Source)"
        return $true
    }
    
    # Check common installation locations
    $llvmPaths = @(
        "C:\Program Files\LLVM\bin",
        "C:\Program Files (x86)\LLVM\bin",
        "$env:ProgramFiles\LLVM\bin",
        "${env:ProgramFiles(x86)}\LLVM\bin"
    )
    
    foreach ($path in $llvmPaths) {
        if (Test-Path "$path\clang.exe") {
            Write-Status "✓ Found LLVM at: $path"
            Write-Status "Adding LLVM to PATH for this session..."
            $env:PATH += ";$path"
            
            # Verify it works
            $clangCmd = Get-Command clang -ErrorAction SilentlyContinue
            if ($clangCmd) {
                Write-Status "✓ Clang is now accessible"
                return $true
            }
        }
    }
    
    # LLVM not found, try to install it
    Write-Warning-Custom "LLVM/Clang not found. This is required for ARM64 Windows builds."
    Write-Host ""
    Write-Host "Attempting to install LLVM via winget..." -ForegroundColor Cyan
    
    try {
        $wingetCmd = Get-Command winget -ErrorAction SilentlyContinue
        if ($wingetCmd) {
            Write-Status "Installing LLVM.LLVM..."
            winget install LLVM.LLVM --accept-source-agreements --accept-package-agreements
            
            if ($LASTEXITCODE -eq 0) {
                Write-Status "✓ LLVM installed successfully"
                Write-Status "Adding LLVM to PATH..."
                
                # Try to find it again after installation
                foreach ($path in $llvmPaths) {
                    if (Test-Path "$path\clang.exe") {
                        $env:PATH += ";$path"
                        Write-Status "✓ LLVM added to PATH"
                        return $true
                    }
                }
                
                Write-Warning-Custom "LLVM was installed but couldn't be found in PATH."
                Write-Warning-Custom "You may need to restart your terminal and run this script again."
                return $false
            }
        } else {
            Write-Warning-Custom "winget not found. Cannot auto-install LLVM."
        }
    } catch {
        Write-Warning-Custom "Failed to install LLVM: $_"
    }
    
    # Installation failed or not possible
    Write-Host ""
    Write-Error-Custom "LLVM/Clang is required for ARM64 Windows builds but is not available."
    Write-Host ""
    Write-Host "Please install LLVM manually:" -ForegroundColor Yellow
    Write-Host "  Option 1: winget install LLVM.LLVM" -ForegroundColor Cyan
    Write-Host "  Option 2: Download from https://github.com/llvm/llvm-project/releases" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "See scripts/INSTALL-LLVM.md for detailed instructions." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "After installation, restart your terminal and run this script again." -ForegroundColor Yellow
    Write-Host ""
    
    return $false
}

# Get version from Cargo.toml
$cargoToml = Get-Content "Cargo.toml" -Raw
if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
    $version = $matches[1]
} else {
    Write-Error-Custom "Could not extract version from Cargo.toml"
    exit 1
}

Write-Status "Building podcast-tui version: $version for all Windows platforms"

# Ensure LLVM is available (required for ARM64 builds)
$llvmAvailable = Ensure-LLVM
if (-not $llvmAvailable) {
    Write-Warning-Custom "Continuing without LLVM - ARM64 builds will likely fail"
    Write-Host "Press Ctrl+C to cancel, or any key to continue with x64-only build..." -ForegroundColor Yellow
    $null = $host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    Write-Host ""
}

# Create release directory
$releaseDir = "releases\v$version"
if (-not (Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir | Out-Null
}
Write-Status "Release directory: $releaseDir"

# Define targets
$targets = @(
    @{
        Triple = "x86_64-pc-windows-msvc"
        Name = "x86_64"
        Description = "Windows x64"
    },
    @{
        Triple = "aarch64-pc-windows-msvc"
        Name = "aarch64"
        Description = "Windows ARM64"
    }
)

$successful = @()
$failed = @()

Write-Status "Starting builds for all Windows targets..."
Write-Host ""

foreach ($target in $targets) {
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    Write-Status "Building for: $($target.Description) ($($target.Triple))"
    Write-Host ""
    
    # Ensure target is installed
    Write-Status "Ensuring target is installed..."
    rustup target add $target.Triple | Out-Null
    
    # Build
    try {
        Write-Status "Building (this may take several minutes)..."
        cargo build --release --target $target.Triple
        
        if ($LASTEXITCODE -eq 0) {
            Write-Status "✓ Build successful for $($target.Description)"
            
            # Package
            $archiveName = "podcast-tui-v$version-windows-$($target.Name)"
            $archiveDir = Join-Path $releaseDir $archiveName
            
            # Create archive directory
            if (Test-Path $archiveDir) {
                Remove-Item -Recurse -Force $archiveDir
            }
            New-Item -ItemType Directory -Path $archiveDir | Out-Null
            
            # Copy binary
            $binaryPath = "target\$($target.Triple)\release\podcast-tui.exe"
            if (Test-Path $binaryPath) {
                # Sign binary if certificate is available (optional, won't fail if no cert)
                $signScript = "scripts\sign-windows-binary.ps1"
                if (Test-Path $signScript) {
                    Write-Status "Checking for code signing certificate..."
                    try {
                        & $signScript -BinaryPath $binaryPath -SkipIfNoCert
                        if ($LASTEXITCODE -eq 0) {
                            Write-Status "✓ Binary signing completed"
                        }
                    } catch {
                        Write-Warning-Custom "Code signing skipped: $_"
                    }
                } else {
                    Write-Warning-Custom "Code signing script not found, skipping signing"
                }
                
                Copy-Item $binaryPath $archiveDir
                Write-Status "✓ Binary copied"
                
                # Copy documentation
                if (Test-Path "README.md") { Copy-Item "README.md" $archiveDir }
                if (Test-Path "LICENSE") { Copy-Item "LICENSE" $archiveDir }
                if (Test-Path "CHANGELOG.md") { Copy-Item "CHANGELOG.md" $archiveDir }
                
                # Create ZIP
                $zipPath = Join-Path $releaseDir "$archiveName.zip"
                if (Test-Path $zipPath) {
                    Remove-Item $zipPath
                }
                
                Compress-Archive -Path $archiveDir -DestinationPath $zipPath -CompressionLevel Optimal
                Write-Status "✓ Created $archiveName.zip"
                
                # Generate checksum
                $hash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()
                $checksumFile = "$zipPath.sha256"
                "$hash  $archiveName.zip" | Out-File -FilePath $checksumFile -Encoding ASCII
                Write-Status "✓ Created checksum"
                
                # Clean up archive directory to avoid conflicts when extracting
                Remove-Item -Recurse -Force $archiveDir
                Write-Status "✓ Cleaned up temporary directory"
                
                $successful += $target.Description
            } else {
                Write-Error-Custom "Binary not found at $binaryPath"
                $failed += $target.Description
            }
        } else {
            throw "Build failed with exit code $LASTEXITCODE"
        }
    } catch {
        Write-Error-Custom "Build failed for $($target.Description): $_"
        $failed += $target.Description
    }
    
    Write-Host ""
}

# Summary
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Status "Build Summary"
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

if ($successful.Count -gt 0) {
    Write-Host "Successful builds ($($successful.Count)):" -ForegroundColor Green
    foreach ($target in $successful) {
        Write-Host "  ✓ $target" -ForegroundColor Green
    }
}

if ($failed.Count -gt 0) {
    Write-Host ""
    Write-Host "Failed builds ($($failed.Count)):" -ForegroundColor Red
    foreach ($target in $failed) {
        Write-Host "  ✗ $target" -ForegroundColor Red
    }
}

Write-Host ""
Write-Status "Release artifacts available in: $releaseDir"

# List created archives
$archives = Get-ChildItem "$releaseDir\*.zip" -ErrorAction SilentlyContinue
if ($archives) {
    Write-Host ""
    Write-Status "Created archives:"
    foreach ($archive in $archives) {
        $size = $archive.Length / 1MB
        Write-Host ("  {0} ({1:N2} MB)" -f $archive.Name, $size) -ForegroundColor Cyan
    }
}

# Exit with error if any builds failed
if ($failed.Count -gt 0) {
    Write-Host ""
    Write-Error-Custom "Some builds failed!"
    exit 1
}

Write-Host ""
Write-Status "All builds completed successfully! 🎉"
