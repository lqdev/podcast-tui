# Installing Visual Studio Build Tools for Windows

## Why You Need This

Building native Windows applications with Rust requires the MSVC (Microsoft Visual C++) linker (`link.exe`). This is especially true for ARM64 Windows, where GNU toolchains are not available.

## Installation Steps

### Option 1: Visual Studio Build Tools (Recommended - Smaller Download)

1. Download the **Build Tools for Visual Studio** from:
   https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022

2. Run the installer and select:
   - ✅ **Desktop development with C++**
   
3. In the installation details on the right, ensure these are checked:
   - ✅ MSVC v143 - VS 2022 C++ ARM64/ARM64EC build tools (Latest)
   - ✅ Windows SDK (latest version)
   
4. Click **Install** (this may take 15-30 minutes)

5. After installation, restart your terminal/PowerShell

### Option 2: Full Visual Studio Community Edition (Free)

1. Download **Visual Studio Community** from:
   https://visualstudio.microsoft.com/vs/community/

2. During installation, select:
   - ✅ **Desktop development with C++**

3. Click **Install**

4. After installation, restart your terminal/PowerShell

## Verify Installation

After installing and restarting your terminal, verify the linker is available:

```powershell
Get-Command link.exe
```

You should see output showing the path to `link.exe`.

## Build After Installation

Once installed, run the build script again:

```powershell
.\scripts\build-windows.ps1
```

## Troubleshooting

### "link.exe not found" after installation

1. Make sure you selected the **C++ build tools** during installation
2. Restart your terminal/PowerShell (or entire computer)
3. Check your PATH includes Visual Studio paths:
   ```powershell
   $env:PATH -split ';' | Where-Object { $_ -like '*Visual Studio*' }
   ```

### For ARM64 Specific Issues

Make sure you selected:
- **MSVC v143 - VS 2022 C++ ARM64/ARM64EC build tools**

Not just the x64 tools.

## Alternative: Cross-Compile from Another Platform

If you cannot install Visual Studio Build Tools, you can:
1. Build on a Linux machine using the build script: `./scripts/build-releases.sh`
2. Use GitHub Actions or CI/CD to build releases
3. Use WSL2 (Windows Subsystem for Linux) to cross-compile

However, for native development on Windows ARM64, MSVC tools are required.
