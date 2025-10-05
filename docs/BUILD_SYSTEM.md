# Cross-Platform Release Build System

This document provides a quick reference for building cross-platform releases of podcast-tui.

## Quick Start

```bash
# 1. Install dependencies (one-time setup)
./scripts/install-build-deps.sh

# 2. Build all release binaries
./scripts/build-releases.sh
```

## Architecture Overview

The build system uses:
- **cargo-zigbuild**: A wrapper around Cargo that uses Zig as a cross-compilation toolchain
- **Zig**: Provides a universal C/C++ compiler that works across platforms
- **Custom scripts**: Automate the build and packaging process

### Why Zig?

Zig provides several advantages for cross-compilation:
- No need for platform-specific toolchains
- Works on any host to build for any target
- Simpler than traditional cross-compilation setups
- Handles linking and system libraries automatically

## Supported Targets

| Platform | Architecture | Target Triple | Binary Name |
|----------|-------------|---------------|-------------|
| Windows | x86_64 | `x86_64-pc-windows-msvc` | `podcast-tui.exe` |
| Windows | ARM64 | `aarch64-pc-windows-msvc` | `podcast-tui.exe` |
| Linux | x86_64 | `x86_64-unknown-linux-gnu` | `podcast-tui` |
| Linux | ARM64 | `aarch64-unknown-linux-gnu` | `podcast-tui` |

## Build Process

### Installation Script (`install-build-deps.sh`)

Installs required tools:
1. `cargo-zigbuild` via `cargo install`
2. `zig` compiler (via pip, apt, or manual download)
3. System dependencies (Linux: libasound2-dev, zip, etc.)

### Build Script (`build-releases.sh`)

For each target:
1. Runs `cargo zigbuild --release --target <triple>`
2. Creates a directory with the binary and documentation
3. Archives as `.zip` (Windows) or `.tar.gz` (Linux)
4. Generates SHA256 checksums

### Output Structure

```
releases/
└── v1.0.0-mvp/
    ├── podcast-tui-v1.0.0-mvp-windows-x86_64.zip
    ├── podcast-tui-v1.0.0-mvp-windows-x86_64.zip.sha256
    ├── podcast-tui-v1.0.0-mvp-linux-x86_64.tar.gz
    ├── podcast-tui-v1.0.0-mvp-linux-x86_64.tar.gz.sha256
    └── ... (other platforms)
```

## Configuration Files

### `.cargo/config.toml`

Configures:
- Linker settings for each target platform
- Build parallelism
- Release profile optimizations

Key settings:
```toml
[target.x86_64-pc-windows-gnu]
linker = "cargo-zigbuild"
rustflags = ["-C", "target-feature=+crt-static"]

[profile.release]
opt-level = 3        # Maximum optimization
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization, slower compile
strip = true        # Remove debug symbols
```

## Manual Building

### Single Target Build

```bash
# Build for a specific target
cargo zigbuild --release --target x86_64-pc-windows-gnu

# Binary will be at:
# target/x86_64-pc-windows-gnu/release/podcast-tui.exe
```

### Testing a Build

```bash
# Linux (on Linux host)
./target/x86_64-unknown-linux-gnu/release/podcast-tui --help

# Windows (using Wine on Linux)
wine ./target/x86_64-pc-windows-gnu/release/podcast-tui.exe --help
```

## CI/CD Integration

### GitHub Actions

The `.github/workflows/release.yml` workflow:
- Triggers on version tags (`v*`)
- Builds all platform binaries
- Creates GitHub release with artifacts
- Can be manually triggered via workflow_dispatch

### Usage

```bash
# Create and push a tag
git tag v1.0.0
git push origin v1.0.0

# GitHub Actions will automatically:
# 1. Build all releases
# 2. Create a GitHub release
# 3. Upload all artifacts
```

## Troubleshooting

### Build Errors

**Problem**: `error: failed to run custom build command for alsa-sys`
**Solution**: Install ALSA development libraries:
```bash
sudo apt-get install libasound2-dev
```

**Problem**: `cargo-zigbuild: command not found`
**Solution**: Add cargo bin to PATH:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
# Add to ~/.bashrc or ~/.zshrc for persistence
```

**Problem**: `zig: command not found`
**Solution**: Reinstall Zig:
```bash
pip3 install ziglang
# or
./scripts/install-build-deps.sh
```

### Build Performance

- **Slow builds**: First build compiles all dependencies (10-30 min)
- **Subsequent builds**: Much faster with cargo caching
- **Parallel builds**: Adjust `jobs` in `.cargo/config.toml`
- **Disk space**: Ensure 5GB+ free (target dir can get large)

### Platform-Specific Issues

**Windows ARM64**:
- Less tested platform
- May have runtime issues with some dependencies
- Test thoroughly before release

**32-bit targets**:
- Increasingly rare
- Consider dropping if maintenance burden is high
- Keep if targeting older/embedded systems

## Optimization Tips

### Binary Size

Current settings produce optimized binaries (~5-15 MB per platform).

For even smaller binaries:
```bash
# Use the release-small profile
cargo zigbuild --release --target <triple> --profile release-small
```

This uses `opt-level = "z"` for minimum size.

### Build Speed

Speed up local development builds:
```bash
# Skip building all targets, just build one
cargo zigbuild --release --target x86_64-unknown-linux-gnu

# Or use dev profile for faster iteration
cargo build --target x86_64-unknown-linux-gnu
```

## Distribution

### Package Managers

Consider publishing to:
- **crates.io**: `cargo install podcast-tui`
- **Homebrew** (macOS/Linux): Custom tap
- **Chocolatey** (Windows): Submit package
- **Snap** (Linux): Create snapcraft.yaml
- **Flatpak** (Linux): Create flatpak manifest

### Direct Downloads

Host binaries on:
- GitHub Releases (automated via Actions)
- Project website
- Package hosting services

Include:
- Installation instructions
- Platform-specific notes
- Verification instructions (SHA256 checksums)

## Security

### Checksums

Always verify checksums before release:
```bash
cd releases/v1.0.0/
sha256sum -c *.sha256
```

### Signing

Consider signing releases:
```bash
# GPG sign archives
gpg --armor --detach-sign podcast-tui-v1.0.0-linux-x86_64.tar.gz

# Users verify with:
gpg --verify podcast-tui-v1.0.0-linux-x86_64.tar.gz.asc
```

## Future Enhancements

Potential improvements:
- [ ] Add macOS targets (x86_64, aarch64)
- [ ] Add FreeBSD support
- [ ] Implement code signing for Windows/macOS
- [ ] Create installers (MSI, DMG, DEB, RPM)
- [ ] Add automated benchmarking
- [ ] Set up reproducible builds
- [ ] Add SBOM (Software Bill of Materials) generation

## References

- [cargo-zigbuild documentation](https://github.com/rust-cross/cargo-zigbuild)
- [Zig download page](https://ziglang.org/download/)
- [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [Cross-compilation in Rust](https://rust-lang.github.io/rustup/cross-compilation.html)
