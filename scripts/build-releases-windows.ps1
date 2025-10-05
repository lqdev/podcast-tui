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

# Get version from Cargo.toml
$cargoToml = Get-Content "Cargo.toml" -Raw
if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
    $version = $matches[1]
} else {
    Write-Error-Custom "Could not extract version from Cargo.toml"
    exit 1
}

Write-Status "Building podcast-tui version: $version for all Windows platforms"

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
