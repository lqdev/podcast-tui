# Installing LLVM for ARM64 Windows Builds

## Why LLVM is Needed

Some Rust dependencies (like `ring`, a cryptography library) require LLVM/Clang to build on ARM64 Windows. This is because they contain assembly code or use build tools that depend on Clang.

## Installation Options

### Option 1: Install LLVM via winget (Recommended - Easiest)

```powershell
# Install LLVM using Windows Package Manager
winget install LLVM.LLVM
```

After installation:
1. **Restart your terminal**
2. Verify installation:
   ```powershell
   clang --version
   ```

### Option 2: Install LLVM Manually

1. Download LLVM from: https://github.com/llvm/llvm-project/releases

2. Get the latest **ARM64 Windows** installer (e.g., `LLVM-<version>-win-arm64.exe`)

3. Run the installer

4. **Important**: During installation, check **"Add LLVM to the system PATH"**

5. Restart your terminal

6. Verify:
   ```powershell
   clang --version
   ```

### Option 3: Install via Chocolatey

```powershell
# Install using Chocolatey (if you have it installed)
choco install llvm
```

## Verify Installation

After installing LLVM, verify that `clang` is available:

```powershell
Get-Command clang
clang --version
```

You should see output like:
```
clang version 18.1.8
Target: aarch64-pc-windows-msvc
...
```

## Build After Installation

Once LLVM is installed and `clang` is in your PATH:

```powershell
# Restart PowerShell first!
.\scripts\build-windows.ps1
```

## Troubleshooting

### "clang not found" after installation

1. **Restart your terminal completely** (close and reopen PowerShell)
2. Check if LLVM is in PATH:
   ```powershell
   $env:PATH -split ';' | Where-Object { $_ -like '*LLVM*' }
   ```
3. Manually add to PATH if needed:
   ```powershell
   # Typical installation path
   $env:PATH += ";C:\Program Files\LLVM\bin"
   ```

### Wrong architecture installed

Make sure you installed the **ARM64** version of LLVM, not x64. Check:
```powershell
clang --version
# Should show: Target: aarch64-pc-windows-msvc
```

## Alternative: Use Different Crypto Backend

If you don't want to install LLVM, you can modify the project to use `aws-lc-rs` instead of `ring`:

1. In `Cargo.toml`, find dependencies that use `rustls` (like `reqwest`)
2. Add feature flags to use `aws-lc-rs`:
   ```toml
   [dependencies]
   reqwest = { version = "...", features = ["rustls-tls-aws-lc-rs"], default-features = false }
   ```

This is more involved and requires code changes, so installing LLVM is simpler.

## Summary

For ARM64 Windows development with Rust:
- **Visual Studio Build Tools** (MSVC) → Required for linking
- **LLVM/Clang** → Required for some dependencies with assembly/native code

Both are free and standard tools for native development on Windows.
