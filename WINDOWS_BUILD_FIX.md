# Windows Build Fix Summary

## Problem
The `build-windows.ps1` script was failing with the error:
```
error: linker `link.exe` not found
Cannot find path 'C:\Dev\podcast-tui\target\release\podcast-tui.exe' because it does not exist.
```

## Root Cause
Building native Windows applications with Rust requires the **MSVC linker** (`link.exe`), which is part of the Visual Studio Build Tools. This is especially critical for ARM64 Windows, where GNU toolchains are not available.

## Solution

### Immediate Action Required
**You need to install Visual Studio Build Tools:**

1. **Download** Build Tools for Visual Studio 2022:
   https://visualstudio.microsoft.com/downloads/

2. **Run the installer** and select:
   - ✅ **Desktop development with C++**
   
3. **For ARM64 builds**, ensure these are checked in installation details:
   - ✅ MSVC v143 - VS 2022 C++ ARM64/ARM64EC build tools (Latest)
   - ✅ Windows SDK (latest version)

4. **Install** (takes 15-30 minutes)

5. **Restart your terminal/PowerShell**

6. **Verify** installation:
   ```powershell
   Get-Command link.exe
   ```

7. **Build again**:
   ```powershell
   .\scripts\build-windows.ps1
   ```

### Detailed Instructions
See [scripts/INSTALL-MSVC-TOOLS.md](scripts/INSTALL-MSVC-TOOLS.md) for comprehensive installation guide.

## Script Improvements Made

The build script has been improved with:

1. **Early validation**: Checks for `link.exe` before attempting build
2. **Clear error messages**: Explains what's needed and provides links
3. **Proper target handling**: Uses correct target for architecture
4. **Better binary detection**: Handles various binary locations
5. **Improved error handling**: Catches build failures properly

## Files Changed

- `scripts/build-windows.ps1` - Enhanced with MSVC validation and better error handling
- `scripts/INSTALL-MSVC-TOOLS.md` - New comprehensive installation guide
- `scripts/README-WINDOWS.md` - Updated with clearer prerequisites

## Why Visual Studio Build Tools?

- **Required for Windows**: The Rust toolchain on Windows uses MSVC by default
- **No GNU alternative on ARM64**: Unlike x86_64, ARM64 Windows has no GNU toolchain
- **Industry standard**: MSVC provides best compatibility with Windows APIs
- **Free**: Build Tools are free, don't require full Visual Studio IDE

## After Installation

Once Visual Studio Build Tools are installed:

```powershell
# Build for current architecture
.\scripts\build-windows.ps1

# Or build for all Windows architectures
.\scripts\build-releases-windows.ps1
```

The build should complete successfully and create:
- Binary: `target\<target>\release\podcast-tui.exe`
- Archive: `releases\v{VERSION}\podcast-tui-v{VERSION}-windows-{arch}.zip`
- Checksum: `releases\v{VERSION}\podcast-tui-v{VERSION}-windows-{arch}.zip.sha256`

## Troubleshooting

If you still get errors after installing:

1. **Restart terminal completely** (close and reopen)
2. **Verify PATH includes Visual Studio**:
   ```powershell
   $env:PATH -split ';' | Where-Object { $_ -like '*Visual Studio*' }
   ```
3. **Check for ARM64 tools** (if on ARM64):
   ```powershell
   dir "C:\Program Files (x86)\Microsoft Visual Studio" -Recurse -Filter "*arm64*"
   ```

## Alternative: Use Linux/WSL

If you cannot install Visual Studio Build Tools, you can:
- Build on Linux using `./scripts/build-releases.sh`
- Use WSL2 to build Linux versions
- Use GitHub Actions for automated builds

---

**Status**: Script improved, installation guide created, ready to build after MSVC tools are installed.
