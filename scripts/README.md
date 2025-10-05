# Build Scripts

This directory contains scripts for building cross-platform release binaries of podcast-tui.

## Prerequisites

Before running the build scripts, you need to install the required dependencies.

## Installation

Run the installation script to set up all necessary tools:

```bash
./scripts/install-build-deps.sh
```

This script will install:
- `cargo-zigbuild` - A Cargo wrapper for using Zig as the linker
- `zig` - The Zig compiler/linker (used for cross-compilation)
- Additional system dependencies (on Linux)

## Building Releases

Once dependencies are installed, build release binaries for all supported platforms:

```bash
./scripts/build-releases.sh
```

### Supported Platforms

The build script creates binaries for:

**Windows:**
- x86_64 (64-bit Intel/AMD) - MSVC
- aarch64 (ARM64) - MSVC

**Linux:**
- x86_64 (64-bit Intel/AMD) - GNU
- aarch64 (ARM64) - GNU

### Output

Release artifacts are created in `releases/v{VERSION}/`:
- Windows builds: `.zip` archives
- Linux builds: `.tar.gz` archives
- SHA256 checksums for all archives

Each archive contains:
- The compiled binary
- README.md
- LICENSE
- CHANGELOG.md

## Manual Build

To build for a specific target manually:

```bash
# Add the target (first time only)
rustup target add <target-triple>

# Build with cargo-zigbuild
cargo zigbuild --release --target <target-triple>
```

Example:
```bash
cargo zigbuild --release --target x86_64-pc-windows-gnu
```

## Troubleshooting

### cargo-zigbuild not found
Ensure `~/.cargo/bin` is in your PATH:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Zig not found
If Zig installation fails, install it manually:
- **Linux/macOS:** `pip3 install ziglang`
- **Homebrew (macOS):** `brew install zig`
- **Manual:** Download from https://ziglang.org/download/

### Build failures
- Ensure you have enough disk space (at least 5GB free)
- Try building targets individually to isolate issues
- Check that all dependencies are installed
- On Linux, ensure `libasound2-dev` is installed for audio support

### Permission denied
Make scripts executable:
```bash
chmod +x scripts/*.sh
```

## CI/CD Integration

These scripts are designed to work in CI/CD environments. Example GitHub Actions workflow:

```yaml
name: Release Builds

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Install dependencies
        run: ./scripts/install-build-deps.sh
      - name: Build releases
        run: ./scripts/build-releases.sh
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: releases
          path: releases/
```

## Performance Notes

- Full cross-compilation build takes approximately 10-30 minutes depending on hardware
- Parallel builds use multiple CPU cores
- Release binaries are optimized for size and performance
- LTO (Link Time Optimization) is enabled for smaller binaries

## License

Same as the parent project (MIT).
