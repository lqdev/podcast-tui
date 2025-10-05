# Windows Build Scripts

PowerShell scripts for building podcast-tui on Windows.

## Prerequisites

- Windows 10 or later
- PowerShell 5.1 or later
- [Rust](https://rustup.rs/) installed
- **Visual Studio Build Tools (REQUIRED)** - See [INSTALL-MSVC-TOOLS.md](INSTALL-MSVC-TOOLS.md) for detailed instructions

> ⚠️ **Important:** Windows builds REQUIRE Visual Studio Build Tools with C++ support. Without this, you'll get "linker `link.exe` not found" errors.

## Installation

### 1. Install Rust

If you haven't already, install Rust:

```powershell
# Download and run rustup-init.exe from https://rustup.rs/
# Or use winget:
winget install Rustlang.Rustup
```

After installation, **restart PowerShell** to ensure Rust is in your PATH.

### 2. Install Visual Studio Build Tools

Download and install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/):

- Select "Desktop development with C++"
- This provides the MSVC compiler needed for Windows builds

### 3. Verify Dependencies

Run the dependency installer to verify everything is set up:

```powershell
.\scripts\install-build-deps.ps1
```

## Building

### Quick Build (Current Architecture)

Build for your current Windows architecture (x64 or ARM64):

```powershell
.\scripts\build-windows.ps1
```

**Output:**
- Binary: `target\release\podcast-tui.exe`
- Package: `releases\v{VERSION}\podcast-tui-v{VERSION}-windows-{arch}.zip`
- Checksum: `releases\v{VERSION}\podcast-tui-v{VERSION}-windows-{arch}.zip.sha256`

**Build time:** ~2-5 minutes (first build takes longer)

### Multi-Architecture Build

Build for both x64 and ARM64:

```powershell
.\scripts\build-releases-windows.ps1
```

**Output:**
- `podcast-tui-v{VERSION}-windows-x86_64.zip`
- `podcast-tui-v{VERSION}-windows-aarch64.zip`
- SHA256 checksums for each

**Build time:** ~5-15 minutes (builds both architectures)

## PowerShell Execution Policy

If you get an error about script execution being disabled:

```powershell
# Check current policy
Get-ExecutionPolicy

# Set policy to allow local scripts (run as Administrator)
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# Or run script with bypass (one-time)
PowerShell -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1
```

## Troubleshooting

### Error: "rustc is not recognized"

**Solution:** Restart PowerShell after installing Rust, or add Rust to PATH manually:

```powershell
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

### Error: "link.exe not found" or MSVC errors

**Solution:** Install Visual Studio Build Tools with C++ support.

Verify installation:

```powershell
# Look for vcvarsall.bat
dir "C:\Program Files (x86)\Microsoft Visual Studio" -Recurse -Filter "vcvarsall.bat"
```

### Error: "Cannot find target aarch64-pc-windows-msvc"

**Solution:** Add the target:

```powershell
rustup target add aarch64-pc-windows-msvc
```

### Build is slow

**First build:** Compiles all dependencies (10-15 minutes)
**Subsequent builds:** Much faster with caching (2-5 minutes)

**Tips:**
- Use `cargo build` for development (faster, unoptimized)
- Use `cargo build --release` or the scripts for production builds
- Clean build cache if needed: `cargo clean`

### ARM64 build fails on x64 machine

**This is expected!** ARM64 builds are cross-compiled. The binary will be created but cannot run on x64.

**To test ARM64 binaries:**
- Use an ARM64 Windows device
- Or use Windows ARM64 virtual machines

## Development Workflow

### Quick Testing

```powershell
# Fast development build (unoptimized)
cargo build

# Run directly
cargo run

# Run tests
cargo test
```

### Release Build

```powershell
# Build optimized binary for current architecture
.\scripts\build-windows.ps1

# Test the release binary
.\target\release\podcast-tui.exe --version
```

### Creating Official Release

For official multi-platform releases, use GitHub Actions:

```powershell
# Create and push tag
git tag v1.0.0
git push origin v1.0.0

# GitHub Actions will build for Windows and Linux
```

## Script Details

### `install-build-deps.ps1`

- Checks Rust installation
- Verifies cargo is available
- Ensures MSVC targets are installed
- Checks for Visual Studio Build Tools
- Validates build system

### `build-windows.ps1`

- Detects current architecture (x64 or ARM64)
- Builds optimized release binary
- Creates ZIP archive with binary + docs
- Generates SHA256 checksum
- ~2-5 minutes build time

### `build-releases-windows.ps1`

- Builds for both x64 and ARM64
- Ensures targets are installed
- Creates separate archives for each architecture
- Generates checksums for all archives
- Comprehensive build summary
- ~5-15 minutes build time

## Binary Size

Optimized release binaries are approximately:
- **Windows x64:** ~6-7 MB
- **Windows ARM64:** ~6-7 MB

Size includes:
- LTO (Link-Time Optimization)
- Stripped debug symbols
- Static linking of Rust standard library

## Output Structure

```
releases\
└── v1.0.0-mvp\
    ├── podcast-tui-v1.0.0-mvp-windows-x86_64\
    │   ├── podcast-tui.exe
    │   ├── README.md
    │   ├── LICENSE
    │   └── CHANGELOG.md
    ├── podcast-tui-v1.0.0-mvp-windows-x86_64.zip
    ├── podcast-tui-v1.0.0-mvp-windows-x86_64.zip.sha256
    ├── podcast-tui-v1.0.0-mvp-windows-aarch64\
    │   └── ... (same structure)
    ├── podcast-tui-v1.0.0-mvp-windows-aarch64.zip
    └── podcast-tui-v1.0.0-mvp-windows-aarch64.zip.sha256
```

## Verifying Checksums

To verify download integrity:

```powershell
# Calculate checksum
$hash = (Get-FileHash -Path podcast-tui-v1.0.0-mvp-windows-x86_64.zip -Algorithm SHA256).Hash.ToLower()

# Compare with checksum file
Get-Content podcast-tui-v1.0.0-mvp-windows-x86_64.zip.sha256

# Should match!
```

## CI/CD Integration

For automated builds, see `.github\workflows\release.yml`.

The GitHub Actions workflow:
- Runs on Windows runners
- Builds both x64 and ARM64
- Creates GitHub releases
- Uploads all artifacts

## Additional Resources

- [Rust for Windows](https://www.rust-lang.org/tools/install)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
- [PowerShell Documentation](https://docs.microsoft.com/powershell/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)

## Support

For issues or questions:
- Check the [main README](../README.md)
- See [BUILD_SYSTEM_FINAL.md](../BUILD_SYSTEM_FINAL.md) for architecture details
- Open an issue on GitHub

---

**Note:** These scripts are designed for local Windows development. For official multi-platform releases, use GitHub Actions which provides both Windows and Linux build environments.
