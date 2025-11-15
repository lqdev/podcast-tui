# PowerShell script to sign Windows binaries with code signing certificate
# Supports both certificate store and .pfx file methods
# Includes automatic retry logic for timestamp servers

#Requires -Version 5.1

param(
    [Parameter(Mandatory=$true)]
    [string]$BinaryPath,
    
    [Parameter(Mandatory=$false)]
    [string]$CertThumbprint,
    
    [Parameter(Mandatory=$false)]
    [string]$PfxPath,
    
    [Parameter(Mandatory=$false)]
    [string]$PfxPassword,
    
    [Parameter(Mandatory=$false)]
    [string]$PfxBase64,
    
    [Parameter(Mandatory=$false)]
    [switch]$SkipIfNoCert = $false,
    
    [Parameter(Mandatory=$false)]
    [int]$MaxRetries = 3
)

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

# Check if signtool is available
function Test-SignTool {
    $signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if (-not $signtool) {
        Write-Warning-Custom "signtool.exe not found in PATH"
        
        # Try to find it in common Windows SDK locations
        $sdkPaths = @(
            "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe",
            "${env:ProgramFiles}\Windows Kits\10\bin\*\x64\signtool.exe",
            "${env:ProgramFiles(x86)}\Windows Kits\10\App Certification Kit\signtool.exe",
            "${env:ProgramFiles}\Windows Kits\10\App Certification Kit\signtool.exe"
        )
        
        foreach ($pattern in $sdkPaths) {
            $found = Get-ChildItem $pattern -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($found) {
                Write-Status "Found signtool at: $($found.FullName)"
                return $found.FullName
            }
        }
        
        Write-Error-Custom "signtool.exe not found. Please install Windows SDK."
        Write-Host "Install via: winget install Microsoft.WindowsSDK" -ForegroundColor Yellow
        return $null
    }
    
    return $signtool.Source
}

# List of timestamp servers to try (in order of preference)
$timestampServers = @(
    "http://timestamp.digicert.com",
    "http://timestamp.sectigo.com",
    "http://timestamp.globalsign.com/tsa/r6advanced1",
    "http://timestamp.comodoca.com",
    "http://ts.ssl.com"
)

# Sign binary with retry logic for timestamp servers
function Sign-Binary {
    param(
        [string]$SignToolPath,
        [string]$Binary,
        [string]$CertThumbprint,
        [string]$PfxFilePath,
        [string]$PfxPass
    )
    
    Write-Status "Signing binary: $Binary"
    
    # Try each timestamp server
    foreach ($tsServer in $timestampServers) {
        for ($attempt = 1; $attempt -le $MaxRetries; $attempt++) {
            try {
                Write-Status "Attempting to sign with timestamp server: $tsServer (attempt $attempt/$MaxRetries)"
                
                # Build signtool command based on certificate method
                $signArgs = @()
                
                if ($CertThumbprint) {
                    # Use certificate from certificate store
                    $signArgs = @(
                        "sign",
                        "/sha1", $CertThumbprint,
                        "/fd", "SHA256",
                        "/tr", $tsServer,
                        "/td", "SHA256",
                        "/v"
                    )
                } elseif ($PfxFilePath -and $PfxPass) {
                    # Use .pfx file
                    $signArgs = @(
                        "sign",
                        "/f", $PfxFilePath,
                        "/p", $PfxPass,
                        "/fd", "SHA256",
                        "/tr", $tsServer,
                        "/td", "SHA256",
                        "/v"
                    )
                } else {
                    # Auto-select certificate (requires only one valid cert in store)
                    $signArgs = @(
                        "sign",
                        "/a",
                        "/fd", "SHA256",
                        "/tr", $tsServer,
                        "/td", "SHA256",
                        "/v"
                    )
                }
                
                $signArgs += $Binary
                
                # Execute signtool
                & $SignToolPath $signArgs
                
                if ($LASTEXITCODE -eq 0) {
                    Write-Status "✓ Binary signed successfully with timestamp from $tsServer"
                    return $true
                } else {
                    Write-Warning-Custom "Signing failed with exit code $LASTEXITCODE"
                }
            } catch {
                Write-Warning-Custom "Signing attempt failed: $_"
            }
            
            if ($attempt -lt $MaxRetries) {
                Write-Status "Waiting 2 seconds before retry..."
                Start-Sleep -Seconds 2
            }
        }
        
        Write-Warning-Custom "All retries exhausted for timestamp server: $tsServer"
    }
    
    # If all timestamp servers failed, try signing without timestamp (not recommended)
    Write-Warning-Custom "All timestamp servers failed. Attempting to sign without timestamp..."
    Write-Warning-Custom "NOTE: Signatures without timestamps will expire when the certificate expires!"
    
    try {
        $signArgs = @()
        
        if ($CertThumbprint) {
            $signArgs = @("sign", "/sha1", $CertThumbprint, "/fd", "SHA256", "/v", $Binary)
        } elseif ($PfxFilePath -and $PfxPass) {
            $signArgs = @("sign", "/f", $PfxFilePath, "/p", $PfxPass, "/fd", "SHA256", "/v", $Binary)
        } else {
            $signArgs = @("sign", "/a", "/fd", "SHA256", "/v", $Binary)
        }
        
        & $SignToolPath $signArgs
        
        if ($LASTEXITCODE -eq 0) {
            Write-Warning-Custom "✓ Binary signed (without timestamp)"
            return $true
        }
    } catch {
        Write-Error-Custom "Failed to sign binary: $_"
    }
    
    return $false
}

# Verify the signature
function Verify-Signature {
    param(
        [string]$SignToolPath,
        [string]$Binary
    )
    
    Write-Status "Verifying signature..."
    
    & $SignToolPath verify /pa /v $Binary
    
    if ($LASTEXITCODE -eq 0) {
        Write-Status "✓ Signature verification successful"
        return $true
    } else {
        Write-Error-Custom "Signature verification failed"
        return $false
    }
}

# Main execution
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Status "Windows Code Signing Script"
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# Validate binary exists
if (-not (Test-Path $BinaryPath)) {
    Write-Error-Custom "Binary not found: $BinaryPath"
    exit 1
}

Write-Status "Binary: $BinaryPath"

# Check if signtool is available
$signtoolPath = Test-SignTool
if (-not $signtoolPath) {
    if ($SkipIfNoCert) {
        Write-Warning-Custom "SignTool not available, skipping signing"
        exit 0
    }
    exit 1
}

# Handle base64-encoded certificate (for CI/CD)
$tempPfxPath = $null
if ($PfxBase64) {
    Write-Status "Decoding base64 certificate..."
    $tempPfxPath = Join-Path $env:TEMP "temp-cert-$(New-Guid).pfx"
    try {
        $bytes = [Convert]::FromBase64String($PfxBase64)
        [System.IO.File]::WriteAllBytes($tempPfxPath, $bytes)
        $PfxPath = $tempPfxPath
        Write-Status "✓ Certificate decoded to temporary file"
    } catch {
        Write-Error-Custom "Failed to decode base64 certificate: $_"
        exit 1
    }
}

# Determine signing method
$hasSigningMethod = $false

if ($CertThumbprint) {
    Write-Status "Signing method: Certificate Store (thumbprint: $CertThumbprint)"
    $hasSigningMethod = $true
} elseif ($PfxPath) {
    Write-Status "Signing method: PFX file ($PfxPath)"
    
    if (-not (Test-Path $PfxPath)) {
        Write-Error-Custom "PFX file not found: $PfxPath"
        if ($tempPfxPath -and (Test-Path $tempPfxPath)) {
            Remove-Item $tempPfxPath -Force
        }
        exit 1
    }
    
    if (-not $PfxPassword) {
        Write-Error-Custom "PFX password is required when using PFX file"
        if ($tempPfxPath -and (Test-Path $tempPfxPath)) {
            Remove-Item $tempPfxPath -Force
        }
        exit 1
    }
    
    $hasSigningMethod = $true
} else {
    Write-Status "Signing method: Auto-select (first valid certificate in store)"
    
    # Check if there's at least one code signing certificate available
    $certs = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert -ErrorAction SilentlyContinue
    if (-not $certs) {
        if ($SkipIfNoCert) {
            Write-Warning-Custom "No code signing certificate found, skipping signing"
            exit 0
        }
        Write-Error-Custom "No code signing certificate found in certificate store"
        Write-Host ""
        Write-Host "To sign binaries, you need either:" -ForegroundColor Yellow
        Write-Host "  1. A certificate in your Windows certificate store" -ForegroundColor Cyan
        Write-Host "  2. A .pfx file with the -PfxPath and -PfxPassword parameters" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "See docs/CODE_SIGNING.md for detailed instructions" -ForegroundColor Yellow
        exit 1
    }
    
    Write-Status "Found $($certs.Count) code signing certificate(s) in store"
    $hasSigningMethod = $true
}

if (-not $hasSigningMethod) {
    if ($SkipIfNoCert) {
        Write-Warning-Custom "No signing method configured, skipping signing"
        exit 0
    }
    Write-Error-Custom "No signing method configured"
    exit 1
}

# Perform signing
$signSuccess = Sign-Binary -SignToolPath $signtoolPath -Binary $BinaryPath `
    -CertThumbprint $CertThumbprint -PfxFilePath $PfxPath -PfxPass $PfxPassword

# Clean up temporary certificate file
if ($tempPfxPath -and (Test-Path $tempPfxPath)) {
    Remove-Item $tempPfxPath -Force
    Write-Status "✓ Cleaned up temporary certificate file"
}

if (-not $signSuccess) {
    Write-Error-Custom "Failed to sign binary"
    exit 1
}

# Verify signature
$verifySuccess = Verify-Signature -SignToolPath $signtoolPath -Binary $BinaryPath

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
if ($verifySuccess) {
    Write-Status "Code signing completed successfully! ✓"
} else {
    Write-Warning-Custom "Binary was signed but verification failed"
}
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

exit 0
