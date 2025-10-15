# Understanding Windows SmartScreen Warnings

When downloading and running `podcast-tui` on Windows, you may encounter a SmartScreen warning that says "Windows protected your PC" or "Unknown publisher." This document explains why this happens and how to safely proceed.

## Why This Happens

Windows Defender SmartScreen is a security feature that warns users about applications that:
1. Haven't been signed with a code signing certificate
2. Don't have an established reputation with Microsoft
3. Are from "unknown publishers"

**Important:** This warning does NOT mean the software is malicious. It simply means:
- The executable is not digitally signed, OR
- The digital signature doesn't yet have enough reputation with Microsoft's SmartScreen service

## Is podcast-tui Safe?

Yes! podcast-tui is safe to run. Here's how to verify:

### 1. Download from Official Sources Only

✅ **Safe sources:**
- GitHub Releases: https://github.com/lqdev/podcast-tui/releases
- Direct from repository maintainer

❌ **Avoid:**
- Third-party download sites
- Unofficial mirrors
- Random file sharing services

### 2. Verify the Checksum

Every release includes a SHA256 checksum file:

```powershell
# Download both the .zip and .zip.sha256 files
# Then verify:
$actualHash = (Get-FileHash -Path "podcast-tui-v1.0.0-mvp-windows-x86_64.zip" -Algorithm SHA256).Hash.ToLower()
$expectedHash = (Get-Content "podcast-tui-v1.0.0-mvp-windows-x86_64.zip.sha256").Split(' ')[0]

if ($actualHash -eq $expectedHash) {
    Write-Host "✓ Checksum verified - file is authentic!" -ForegroundColor Green
} else {
    Write-Host "✗ Checksum mismatch - DO NOT RUN!" -ForegroundColor Red
}
```

### 3. Check the Source Code

podcast-tui is open source! You can:
- Review the code on GitHub: https://github.com/lqdev/podcast-tui
- Build from source if preferred
- Report security issues if you find any

## How to Run Despite the Warning

If you've verified the checksum and downloaded from official sources, you can safely bypass the warning:

### Method 1: Via SmartScreen Dialog

1. When you see "Windows protected your PC":
2. Click **"More info"**
3. Click **"Run anyway"**

![SmartScreen warning - More info button](https://user-images.githubusercontent.com/example/smartscreen-more-info.png)

### Method 2: Via File Properties

1. Right-click the downloaded `.exe` file
2. Select **Properties**
3. Check **"Unblock"** at the bottom
4. Click **Apply** and **OK**
5. Now you can run the file normally

### Method 3: Via PowerShell

```powershell
# Unblock the downloaded file
Unblock-File -Path "podcast-tui.exe"

# Now you can run it
.\podcast-tui.exe
```

## Why Isn't podcast-tui Code Signed?

Code signing requires purchasing a certificate that costs $50-$600 per year, depending on the type:

| Certificate Type | Cost/Year | Benefit |
|-----------------|-----------|---------|
| Standard | $50-150 | Basic signing |
| OV (Organization Validation) | $100-300 | Better trust, CI/CD friendly |
| EV (Extended Validation) | $300-600 | Immediate SmartScreen trust |

### Current Status

- ✅ **Infrastructure is ready**: All scripts and workflows support code signing
- ⏳ **Certificate pending**: We're evaluating certificate providers
- 📋 **Documentation complete**: Setup guides are ready for when we acquire a certificate

### Future Plans

We plan to acquire a code signing certificate in the future. This will:
- Display a verified publisher name
- Eliminate SmartScreen warnings
- Improve user trust and experience

**Tracking Issue:** [#XX] Add Code Signing Certificate

## For Project Maintainers

If you'd like to contribute to purchasing a code signing certificate, or if you represent an organization interested in sponsoring one, please:
1. Open a discussion on GitHub
2. Contact the maintainers
3. See `docs/CODE_SIGNING.md` for technical setup details

## Alternatives to Downloading Binaries

If you're not comfortable with the SmartScreen warning, you have alternatives:

### Build from Source

```bash
# Clone the repository
git clone https://github.com/lqdev/podcast-tui.git
cd podcast-tui

# Build (requires Rust + MSVC Build Tools)
cargo build --release

# Binary will be at: target/release/podcast-tui.exe
```

See [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed build instructions.

### Use Windows Subsystem for Linux (WSL)

Run the Linux version via WSL:
```bash
# In WSL
wget https://github.com/lqdev/podcast-tui/releases/download/v1.0.0-mvp/podcast-tui-v1.0.0-mvp-linux-x86_64.tar.gz
tar -xzf podcast-tui-v1.0.0-mvp-linux-x86_64.tar.gz
cd podcast-tui-v1.0.0-mvp-linux-x86_64
./podcast-tui
```

## Reporting Security Issues

If you discover a security vulnerability:

1. **DO NOT** open a public GitHub issue
2. Email the maintainers privately: [security contact]
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We take security seriously and will respond promptly to legitimate reports.

## Common Questions

### Q: Will this change after code signing?
**A:** Yes! Once we acquire a code signing certificate, Windows will:
- Display our verified publisher name
- Show green checkmarks in security dialogs
- Not show SmartScreen warnings

### Q: Can I just disable SmartScreen?
**A:** Not recommended! SmartScreen protects you from actual malware. It's better to:
- Verify checksums for downloads
- Run software from trusted sources
- Keep SmartScreen enabled for protection

### Q: How do other open-source projects handle this?
**A:** Common approaches:
- Purchase code signing certificates (larger projects)
- Accept SmartScreen warnings (smaller projects)
- Build reputation over time with downloads
- Some users build from source

### Q: What about antivirus false positives?
**A:** Occasionally, antivirus software may flag unsigned executables. This is usually a false positive. You can:
- Verify the checksum (most important!)
- Submit to antivirus vendors as false positive
- Add an exception in your antivirus software
- Build from source

## Additional Resources

- [Microsoft: SmartScreen Overview](https://learn.microsoft.com/en-us/windows/security/threat-protection/windows-defender-smartscreen/windows-defender-smartscreen-overview)
- [Code Signing Setup Guide](CODE_SIGNING.md) (for maintainers)
- [GitHub Actions Code Signing](GITHUB_ACTIONS_CODE_SIGNING_SETUP.md) (for maintainers)

## Summary

✅ **podcast-tui is safe** - it's open source and you can verify checksums

⚠️ **SmartScreen warnings are expected** - the app isn't code signed yet

🔐 **Always verify checksums** - ensure you downloaded authentic files

🚀 **You can safely run it** - click "More info" → "Run anyway"

---

**Last Updated:** 2025-10-14  
**Applies to:** All unsigned releases (v1.0.0-mvp and later, until code signing certificate is acquired)
