# Windows Code Signing Guide

This document provides comprehensive instructions for code signing Windows binaries to eliminate SmartScreen warnings and "Unknown Publisher" issues.

## Overview

Code signing is the process of digitally signing executable files (.exe) to verify the publisher's identity and ensure the software hasn't been tampered with. This eliminates Windows SmartScreen warnings and builds user trust.

### Benefits

- ✅ Eliminates "Unknown Publisher" warnings
- ✅ Prevents Windows Defender SmartScreen warnings
- ✅ Reduces false positive malware detections
- ✅ Displays verified publisher name in Windows properties
- ✅ Improves user confidence during installation

## Certificate Types

### Extended Validation (EV) Certificates

**Recommended for open-source projects**

- **Pros:**
  - Immediate SmartScreen reputation (no waiting period)
  - Highest level of trust
  - No need to build reputation over time
  - Hardware-based security (USB token)
  
- **Cons:**
  - More expensive ($300-$600/year)
  - Requires business verification
  - Physical USB token required for signing
  - More complex setup

- **Providers:**
  - DigiCert
  - Sectigo
  - GlobalSign

### Organization Validation (OV) Certificates

- **Pros:**
  - Less expensive ($100-$300/year)
  - Can be used from CI/CD pipeline
  - Easier to automate
  
- **Cons:**
  - Requires building SmartScreen reputation (time + downloads)
  - May show warnings initially until reputation is established
  
- **Providers:**
  - SSL.com
  - Sectigo
  - Comodo

### Standard (Code Signing) Certificates

- **Pros:**
  - Cheapest option ($50-$150/year)
  - Easy to obtain
  
- **Cons:**
  - Still requires building reputation
  - May not eliminate all warnings initially
  
- **Providers:**
  - Sectigo
  - Comodo
  - Certum

## Certificate Acquisition Process

### 1. Choose a Provider

For this project, we recommend:
- **Best Option:** DigiCert EV Code Signing Certificate
- **Budget Option:** SSL.com OV Code Signing Certificate

### 2. Business Identity Verification

You'll need to provide:
- Business registration documents (or personal ID for individual developers)
- D-U-N-S number (for businesses)
- Phone number verification
- Physical address verification

**For Open Source Projects:**
- Consider registering as an individual developer
- Or create a simple LLC/business entity
- Some providers offer discounts for open-source projects

### 3. Certificate Purchase Timeline

- **Application:** 1-2 hours
- **Verification:** 1-7 business days
- **Certificate Issuance:** Within 24 hours after verification
- **Total Time:** 2-10 business days

### 4. Certificate Formats

You'll receive:
- **EV Certificate:** USB hardware token (SafeNet or similar)
- **OV Certificate:** `.pfx` or `.p12` file + password

## Setting Up Code Signing

### Prerequisites

1. **Windows 10/11** with Windows SDK installed
2. **SignTool.exe** (included with Windows SDK or Visual Studio)
3. **Code signing certificate** (EV USB token or OV .pfx file)

### Installing Windows SDK (if needed)

```powershell
# Check if signtool is available
Get-Command signtool.exe -ErrorAction SilentlyContinue

# If not found, install Windows SDK
winget install Microsoft.WindowsSDK

# Or download from:
# https://developer.microsoft.com/en-us/windows/downloads/windows-sdk/
```

### Local Development Setup

#### For EV Certificates (USB Token)

1. Insert USB token into your computer
2. Install the certificate provider's software (e.g., SafeNet Authentication Client)
3. Verify the certificate is accessible:

```powershell
# List available certificates
certutil -store My

# Or use PowerShell
Get-ChildItem Cert:\CurrentUser\My
```

#### For OV Certificates (.pfx file)

1. **Secure Storage:** Store your `.pfx` file in a secure location
2. **Never commit to Git:** Add to `.gitignore`
3. **Use strong password:** Store password in a password manager

**Import certificate to Windows certificate store:**

```powershell
# Import .pfx to user certificate store
$pfxPath = "C:\path\to\your\certificate.pfx"
$password = Read-Host -AsSecureString -Prompt "Enter PFX password"
Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation Cert:\CurrentUser\My -Password $password
```

### Signing Your Binary

#### Manual Signing

```powershell
# Sign a single binary
signtool sign /a /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 "podcast-tui.exe"

# With specific certificate (by thumbprint)
signtool sign /sha1 <CERT_THUMBPRINT> /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 "podcast-tui.exe"

# With .pfx file directly
signtool sign /f "certificate.pfx" /p "PASSWORD" /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 "podcast-tui.exe"
```

#### Using the Project's Signing Script

We provide a PowerShell script for automated signing:

```powershell
# Sign after building
./scripts/sign-windows-binary.ps1 -BinaryPath "target\release\podcast-tui.exe"

# With specific certificate thumbprint
./scripts/sign-windows-binary.ps1 -BinaryPath "target\release\podcast-tui.exe" -CertThumbprint "YOUR_CERT_THUMBPRINT"

# With .pfx file
./scripts/sign-windows-binary.ps1 -BinaryPath "target\release\podcast-tui.exe" -PfxPath "certificate.pfx" -PfxPassword "PASSWORD"
```

### Timestamp Servers

**Always use timestamping!** This ensures your signature remains valid even after your certificate expires.

Recommended timestamp servers:
- DigiCert: `http://timestamp.digicert.com`
- Sectigo: `http://timestamp.sectigo.com`
- GlobalSign: `http://timestamp.globalsign.com`

If primary fails, the script will try alternates.

## GitHub Actions Integration

### Setting Up Secrets

Add these secrets to your GitHub repository (Settings → Secrets and variables → Actions):

#### Option 1: Using Certificate from Windows Certificate Store

1. **`WINDOWS_CERT_THUMBPRINT`**: Certificate thumbprint (SHA1 hash)
   - Find via: `Get-ChildItem Cert:\CurrentUser\My | Format-List`

#### Option 2: Using .pfx File

1. **`WINDOWS_CERT_BASE64`**: Base64-encoded .pfx file
   ```powershell
   $bytes = [System.IO.File]::ReadAllBytes("certificate.pfx")
   [Convert]::ToBase64String($bytes) | Set-Clipboard
   ```

2. **`WINDOWS_CERT_PASSWORD`**: Password for .pfx file

### Workflow Configuration

The release workflow is already configured to sign Windows binaries if a certificate is available. The signing is conditional and won't fail if no certificate is present.

To enable signing:
1. Add the required secrets to your repository
2. The workflow will automatically sign binaries during the build process

## Verification

### Verify Signature on Signed Binary

```powershell
# Check signature
signtool verify /pa "podcast-tui.exe"

# Detailed verification
signtool verify /v /pa "podcast-tui.exe"

# View signature details
Get-AuthenticodeSignature "podcast-tui.exe" | Format-List
```

### Expected Output

A properly signed binary should show:
```
SignerCertificate : [Subject]
  CN=Your Name or Company
  ...

Status            : Valid
StatusMessage     : Signature verified
```

### Check in Windows Explorer

Right-click the `.exe` file → Properties → Digital Signatures tab

You should see:
- Your name/company as the signer
- Timestamp showing when it was signed
- "This digital signature is OK"

## Troubleshooting

### Certificate Not Found

```powershell
# List all certificates in personal store
Get-ChildItem Cert:\CurrentUser\My | Format-List Subject, Thumbprint

# Check if certificate has code signing capability
$cert = Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Thumbprint -eq "YOUR_THUMBPRINT" }
$cert.EnhancedKeyUsageList
# Should include "Code Signing" (1.3.6.1.5.5.7.3.3)
```

### Timestamp Server Failures

If timestamp servers are down:
- The script automatically tries multiple servers
- Signing will still succeed but without a timestamp
- **Not recommended:** Signatures without timestamps expire with the certificate

### SmartScreen Still Shows Warnings

Even with valid code signing:
- **New certificates** need to build reputation (can take weeks/months)
- **EV certificates** have immediate reputation
- **OV/Standard certificates** require:
  - Time (several weeks to months)
  - Download volume (thousands of downloads)
  - No malware reports

### "The specified timestamp server either could not be reached or returned an invalid response"

This is common and usually transient:
- Our script retries with multiple timestamp servers
- If all fail, consider signing without timestamp temporarily
- Try again in a few minutes

## Cost Analysis

### Annual Costs

| Certificate Type | Cost/Year | Setup Time | SmartScreen | Best For |
|-----------------|-----------|------------|-------------|----------|
| EV Code Signing | $300-600 | 3-10 days | Immediate | Production releases |
| OV Code Signing | $100-300 | 2-7 days | 2-6 months | Growing projects |
| Standard Signing | $50-150 | 1-3 days | 3-12 months | Small projects |

### Recommendations by Project Stage

- **MVP/Early Development:** Wait, focus on features
- **Public Beta:** Consider OV certificate
- **Production Release:** EV certificate recommended
- **Enterprise Users:** EV certificate required

## Free Alternatives (Not Recommended)

### Self-Signed Certificates

❌ **Do not use for distribution**
- Creates MORE warnings than unsigned
- Only useful for internal testing
- Windows treats as untrusted

### Azure Code Signing

- In preview as of 2024
- Requires Azure subscription
- Cloud-based HSM solution
- May be cost-effective for CI/CD
- Still requires certificate purchase

## For This Project

### Current Status

⚠️ **Certificate not yet acquired**

The infrastructure is in place for code signing, but we need to:
1. Decide on certificate provider (recommendation: DigiCert EV or SSL.com OV)
2. Complete business verification process
3. Purchase certificate
4. Add certificate secrets to GitHub Actions
5. Enable signing in releases

### Estimated Timeline

1. **Decision & Purchase:** 1 day
2. **Verification:** 2-7 business days
3. **Certificate Issuance:** 1 day
4. **Setup & Testing:** 1-2 days
5. **First Signed Release:** ~2 weeks total

### Budget Required

- **Recommended (EV):** $300-600/year
- **Budget Option (OV):** $100-300/year

## Security Best Practices

### Certificate Protection

1. **Never commit certificates to Git**
   - Add `*.pfx`, `*.p12`, `*.key` to `.gitignore`
   - Use GitHub Secrets for CI/CD

2. **Use strong passwords**
   - Minimum 20 characters
   - Use password manager
   - Rotate annually

3. **Limit access**
   - Only authorized personnel
   - Use hardware tokens when possible
   - Audit signing activity

4. **Backup your certificate**
   - Store encrypted backups
   - Document recovery process
   - Test recovery procedure

### Revocation

If your certificate is compromised:
1. Contact certificate provider immediately
2. Request revocation
3. Remove from all systems
4. Purchase new certificate
5. Re-sign all distributed binaries

## Resources

### Official Documentation

- [Microsoft: Sign Tool](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool)
- [Microsoft: Introduction to Code Signing](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/introduction-to-code-signing)
- [SmartScreen Reputation System](https://learn.microsoft.com/en-us/windows/security/threat-protection/windows-defender-smartscreen/windows-defender-smartscreen-overview)

### Certificate Providers

- [DigiCert Code Signing](https://www.digicert.com/signing/code-signing-certificates)
- [SSL.com Code Signing](https://www.ssl.com/code-signing/)
- [Sectigo Code Signing](https://sectigo.com/ssl-certificates-tls/code-signing)

### Tools

- [Windows SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-sdk/)
- [SignTool Documentation](https://learn.microsoft.com/en-us/dotnet/framework/tools/signtool-exe)

## Support

For questions about code signing setup:
1. Check this documentation
2. Review troubleshooting section
3. Open a GitHub issue with tag `build`
4. Contact certificate provider support

---

**Last Updated:** 2025-10-14
**Status:** Infrastructure ready, awaiting certificate acquisition
