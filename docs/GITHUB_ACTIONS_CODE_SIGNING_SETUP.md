# GitHub Actions Code Signing Setup

Quick reference guide for setting up Windows code signing in GitHub Actions.

## Prerequisites

1. A Windows code signing certificate (`.pfx` file) or certificate thumbprint
2. Repository admin access to add secrets
3. Certificate password (if using `.pfx` file)

## Setup Steps

### Option 1: Using Base64-Encoded Certificate (Recommended for CI/CD)

This method stores the certificate directly in GitHub Secrets.

#### 1. Convert Certificate to Base64

On Windows:
```powershell
# Convert .pfx to base64
$bytes = [System.IO.File]::ReadAllBytes("path\to\your\certificate.pfx")
$base64 = [Convert]::ToBase64String($bytes)
$base64 | Set-Clipboard
# Base64 string is now in clipboard
```

On Linux/macOS:
```bash
# Convert .pfx to base64
base64 -w 0 path/to/your/certificate.pfx | pbcopy  # macOS
base64 -w 0 path/to/your/certificate.pfx | xclip   # Linux
```

#### 2. Add Secrets to GitHub

1. Go to your repository on GitHub
2. Navigate to **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Add these secrets:

| Secret Name | Value | Description |
|------------|-------|-------------|
| `WINDOWS_CERT_BASE64` | Base64 string from step 1 | Your certificate in base64 format |
| `WINDOWS_CERT_PASSWORD` | Your certificate password | Password to unlock the certificate |

#### 3. Verify Setup

The next time you:
- Push a tag (e.g., `git tag v1.0.0 && git push origin v1.0.0`)
- Manually trigger the release workflow

The Windows binaries will be automatically signed.

### Option 2: Using Certificate from Store (For Self-Hosted Runners)

If you're using a self-hosted Windows runner with a certificate already installed:

#### 1. Find Certificate Thumbprint

```powershell
# List all code signing certificates
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Format-List Subject, Thumbprint

# Example output:
# Subject    : CN=Your Name
# Thumbprint : 1234567890ABCDEF1234567890ABCDEF12345678
```

#### 2. Add Secret to GitHub

Add this secret:

| Secret Name | Value | Description |
|------------|-------|-------------|
| `WINDOWS_CERT_THUMBPRINT` | Certificate thumbprint (SHA1 hash) | From step 1 |

## Testing

### Test Locally

Before pushing to GitHub, test signing locally:

```powershell
# Build and sign
.\scripts\build-windows.ps1

# Verify the signature
signtool verify /pa "releases\v*\*\podcast-tui.exe"

# Check signature details
Get-AuthenticodeSignature "releases\v*\*\podcast-tui.exe" | Format-List
```

### Test in GitHub Actions

1. Push a test tag:
   ```bash
   git tag v0.0.0-test-signing
   git push origin v0.0.0-test-signing
   ```

2. Check the workflow run:
   - Go to **Actions** tab
   - Click on the latest workflow run
   - Expand the "Sign Windows binaries" step
   - Verify signing was successful

3. Download the artifact and verify:
   ```powershell
   # Extract the artifact
   Expand-Archive podcast-tui-windows-releases.zip
   
   # Verify signature
   signtool verify /pa "podcast-tui-windows-releases\v*\*\podcast-tui.exe"
   ```

## Verification

After setting up, verify that:

1. ✅ Secrets are added correctly (check Settings → Secrets)
2. ✅ Workflow includes code signing step (check `.github/workflows/release.yml`)
3. ✅ Test signing works locally (if possible)
4. ✅ Binaries are signed in GitHub Actions (check workflow logs)
5. ✅ Downloaded binaries have valid signatures

### Check Signature Details

```powershell
# On Windows
Get-AuthenticodeSignature "podcast-tui.exe" | Format-List

# Expected output:
# SignerCertificate : [Subject]
#   CN=Your Name or Company
# Status            : Valid
# StatusMessage     : Signature verified
```

### Check in Windows Explorer

1. Right-click `podcast-tui.exe`
2. Select **Properties**
3. Go to **Digital Signatures** tab
4. Should show your signature with "This digital signature is OK"

## Troubleshooting

### Signing Step is Skipped

**Problem:** The "Sign Windows binaries" step shows as skipped.

**Solution:** 
- Verify secrets are added: Settings → Secrets → Actions
- Check secret names match exactly: `WINDOWS_CERT_BASE64` or `WINDOWS_CERT_THUMBPRINT`
- Ensure you have admin access to the repository

### Certificate Not Found Error

**Problem:** Error "No code signing certificate found"

**Solutions:**
- For base64 method: Verify `WINDOWS_CERT_BASE64` and `WINDOWS_CERT_PASSWORD` are set
- For thumbprint method: Verify certificate is installed on the runner
- Check certificate has code signing capability

### Timestamp Server Failures

**Problem:** Errors about timestamp servers being unreachable

**Solution:**
- The script automatically retries with multiple timestamp servers
- This is usually transient - retry the workflow
- Signing may succeed without timestamp (warning will be shown)

### Invalid Signature

**Problem:** Signature verification fails

**Solutions:**
- Ensure certificate is valid and not expired
- Check certificate has code signing usage
- Verify password is correct (for .pfx method)
- Ensure certificate chain is complete

## Security Best Practices

### Certificate Protection

1. **Never commit certificates to Git**
   ```bash
   # Add to .gitignore
   echo "*.pfx" >> .gitignore
   echo "*.p12" >> .gitignore
   ```

2. **Use environment-specific secrets**
   - Use different certificates for development and production
   - Rotate certificates before expiration

3. **Limit secret access**
   - Only give admin access to trusted maintainers
   - Use GitHub's secret scanning
   - Audit secret access regularly

4. **Monitor certificate usage**
   - Set up expiration alerts
   - Review signing activity
   - Revoke if compromised

## Cost Considerations

### Certificate Types

| Type | Cost/Year | GitHub Actions | SmartScreen |
|------|-----------|----------------|-------------|
| EV Certificate | $300-600 | Limited (USB token) | Immediate |
| OV Certificate | $100-300 | ✅ Fully supported | 2-6 months |
| Standard Certificate | $50-150 | ✅ Fully supported | 3-12 months |

### Recommended Setup

For open-source projects using GitHub Actions:
- **Best:** OV Certificate ($100-300/year)
- **Budget:** Standard Certificate ($50-150/year)

EV certificates use hardware tokens (USB) and are difficult to use in CI/CD.

## Additional Resources

- [Full Code Signing Guide](CODE_SIGNING.md)
- [Microsoft SignTool Documentation](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool)
- [GitHub Actions Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)

## Support

For issues:
1. Check [CODE_SIGNING.md](CODE_SIGNING.md) troubleshooting section
2. Review GitHub Actions workflow logs
3. Open a GitHub issue with tag `build` or `ci/cd`

---

**Last Updated:** 2025-10-14
