# Build Commands Quick Reference

## Linux/macOS

### Dependencies
```bash
./scripts/install-build-deps.sh
```

### Local Build (Fast)
```bash
./scripts/build-linux.sh
```
- Builds for current architecture only
- ~2 minutes
- Output: `releases/v{VERSION}/podcast-tui-v{VERSION}-linux-{arch}.tar.gz`

### Development Build
```bash
cargo build                 # Debug build (fast, unoptimized)
cargo build --release       # Release build (optimized)
cargo run                   # Build and run
```

### Create Release
```bash
git tag v1.0.0
git push origin v1.0.0      # Triggers GitHub Actions for all platforms
```

---

## Windows

### Dependencies
```powershell
.\scripts\install-build-deps.ps1
```

### Local Build (Fast)
```powershell
.\scripts\build-windows.ps1
```
- Builds for current architecture only
- ~2-5 minutes
- Output: `releases\v{VERSION}\podcast-tui-v{VERSION}-windows-{arch}.zip`

### Multi-Architecture Build
```powershell
.\scripts\build-releases-windows.ps1
```
- Builds for both x64 and ARM64
- ~5-15 minutes
- Output: ZIP files for each architecture

### Development Build
```powershell
cargo build                 # Debug build (fast, unoptimized)
cargo build --release       # Release build (optimized)
cargo run                   # Build and run
```

### Create Release
```powershell
git tag v1.0.0
git push origin v1.0.0      # Triggers GitHub Actions for all platforms
```

---

## Quick Comparison

| Task | Linux/macOS | Windows |
|------|-------------|---------|
| **Install deps** | `./scripts/install-build-deps.sh` | `.\scripts\install-build-deps.ps1` |
| **Quick build** | `./scripts/build-linux.sh` | `.\scripts\build-windows.ps1` |
| **Multi-platform** | GitHub Actions | `.\scripts\build-releases-windows.ps1` |
| **Dev build** | `cargo build` | `cargo build` |
| **Run** | `cargo run` | `cargo run` |
| **Clean** | `cargo clean` | `cargo clean` |
| **Test** | `cargo test` | `cargo test` |

---

## Platform Support

| Platform | Local Build | CI/CD Build |
|----------|-------------|-------------|
| Windows x64 | ✅ Native | ✅ GitHub Actions |
| Windows ARM64 | ✅ Native or Cross | ✅ GitHub Actions |
| Linux x64 | ⚠️ CI Only* | ✅ GitHub Actions |
| Linux ARM64 | ✅ Native | ✅ GitHub Actions |

\* Linux x64 cross-compile from ARM64 has issues, use CI or native x64 machine

---

## Output Locations

### Linux
```
releases/v{VERSION}/
├── podcast-tui-v{VERSION}-linux-x86_64.tar.gz
├── podcast-tui-v{VERSION}-linux-x86_64.tar.gz.sha256
├── podcast-tui-v{VERSION}-linux-aarch64.tar.gz
└── podcast-tui-v{VERSION}-linux-aarch64.tar.gz.sha256
```

### Windows
```
releases\v{VERSION}\
├── podcast-tui-v{VERSION}-windows-x86_64.zip
├── podcast-tui-v{VERSION}-windows-x86_64.zip.sha256
├── podcast-tui-v{VERSION}-windows-aarch64.zip
└── podcast-tui-v{VERSION}-windows-aarch64.zip.sha256
```

---

## Troubleshooting

### Linux: "cargo-zigbuild: command not found"
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Windows: "Script execution is disabled"
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Windows: "link.exe not found"
Install Visual Studio Build Tools with C++ support

### Build is slow
First build compiles all dependencies (~10-15 min), subsequent builds are faster (~2-5 min)

### Clean build cache
```bash
cargo clean
```

---

## CI/CD (GitHub Actions)

Automatically builds all platforms when you push a tag:

```bash
git tag v1.0.0
git push origin v1.0.0
```

Workflow: `.github/workflows/release.yml`

**Builds:**
- Windows x64 & ARM64
- Linux x64 & ARM64

**Creates:**
- GitHub Release
- All binary archives
- SHA256 checksums

---

## Binary Sizes

Optimized release binaries:
- **Linux:** ~6-7 MB
- **Windows:** ~6-7 MB

Includes:
- LTO optimization
- Stripped debug symbols
- Static linking (Windows)

---

## Recommended Workflow

### Daily Development
```bash
cargo run          # Fast iteration
```

### Testing Release Build
```bash
# Linux
./scripts/build-linux.sh

# Windows
.\scripts\build-windows.ps1
```

### Official Release
```bash
git tag v1.0.0
git push origin v1.0.0
# Wait for GitHub Actions to complete
# Download artifacts from GitHub Releases page
```

---

**For detailed instructions, see:**
- Linux: `scripts/README.md`
- Windows: `scripts/README-WINDOWS.md`
- Architecture: `BUILD_SYSTEM_FINAL.md`
