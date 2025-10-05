# Getting Started with Podcast TUI

## Quick Start Guide

This guide will help you get Podcast TUI running on your system, regardless of platform.

## Current Development Status (October 2025)

**✅ What Works:**
- Subscribe to podcast RSS feeds
- Browse episode lists with metadata
- Download episodes (2-3 concurrent downloads)
- OPML import/export
- Emacs-style keyboard navigation
- Multiple color themes
- Cross-platform builds (Windows/Linux)

**🚧 What's Coming:**
- Audio playback (Sprint 4 - next up)
- Playlists (Sprint 5)
- Episode notes (Sprint 5)
- Statistics tracking (Sprint 6)
- Search & filtering enhancements (Sprint 5)

## Platform-Specific Setup

### Windows (x64)

#### Prerequisites
1. **Rust** (1.75+): Install from https://rustup.rs/
2. **MSVC Build Tools**: Required for the `ring` cryptography dependency
   - See detailed instructions: [scripts/INSTALL-MSVC-TOOLS.md](scripts/INSTALL-MSVC-TOOLS.md)
   - Quick install: Run `.\scripts\install-build-deps.ps1` in PowerShell

#### Building from Source
```powershell
# Clone the repository
git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui

# Verify build dependencies
.\scripts\install-build-deps.ps1

# Build the project
cargo build --release

# Run the application
.\target\release\podcast-tui.exe
```

#### Using Pre-built Binaries
```powershell
# Download the latest release
# Extract podcast-tui-vX.X.X-windows-x86_64.zip
# Run podcast-tui.exe
```

### Windows (ARM64)

#### Prerequisites
1. **Rust** (1.75+): Install ARM64 version from https://rustup.rs/
2. **LLVM/Clang**: Required for the `ring` dependency on ARM64
   - See detailed instructions: [scripts/INSTALL-LLVM.md](scripts/INSTALL-LLVM.md)
   - Download from: https://github.com/llvm/llvm-project/releases

#### Building from Source
```powershell
# Clone the repository
git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui

# Install LLVM (see scripts/INSTALL-LLVM.md for details)
# Set environment variables (in PowerShell)
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
$env:PATH += ";C:\Program Files\LLVM\bin"

# Build the project
cargo build --release

# Run the application
.\target\release\podcast-tui.exe
```

### Linux (Ubuntu/Debian)

#### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev
```

#### Building from Source
```bash
# Clone the repository
git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui

# Build the project
cargo build --release

# Run the application
./target/release/podcast-tui
```

#### Using Pre-built Binaries
```bash
# Download the latest release
wget https://github.com/yourusername/podcast-tui/releases/download/vX.X.X/podcast-tui-vX.X.X-linux-x86_64.tar.gz

# Extract and run
tar -xzf podcast-tui-vX.X.X-linux-x86_64.tar.gz
cd podcast-tui-vX.X.X-linux-x86_64
./podcast-tui
```

### Linux (Other Distributions)

#### Fedora/RHEL/CentOS
```bash
sudo dnf install gcc gcc-c++ openssl-devel
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Arch Linux
```bash
sudo pacman -S base-devel rust openssl
```

## First Time Usage

### 1. Start the Application
```bash
# Linux/Mac
./target/release/podcast-tui

# Windows
.\target\release\podcast-tui.exe
```

### 2. Subscribe to Your First Podcast

**Method 1: Using M-x Command**
1. Press `M-x` (Alt+x or Esc then x)
2. Type `subscribe` and press Enter
3. Enter the RSS feed URL
4. Wait for the feed to parse and episodes to load

**Method 2: Using Keybinding**
1. Press `a` from the podcast list
2. Enter the RSS feed URL
3. Wait for the feed to parse

**Popular Test Feeds:**
- The Changelog: `https://changelog.com/podcast/feed`
- Syntax: `https://feed.syntax.fm/rss`
- Reply All: `https://feeds.megaphone.fm/replyall`

### 3. Browse and Download Episodes

1. Use `C-n` / `C-p` to navigate episodes (or Up/Down arrows)
2. Press `Enter` on an episode to view details
3. Press `D` to download the selected episode
4. Press `C-x b` to switch between buffers (podcast list, episode list)

### 4. Essential Keybindings

**Navigation:**
- `C-n` / `C-p` - Next/Previous item (or Down/Up arrows)
- `C-f` / `C-b` - Forward/Backward character (or Right/Left arrows)
- `C-a` / `C-e` - Beginning/End of line
- `Enter` - Select/Activate item

**Buffer Management:**
- `C-x b` - Switch buffer
- `C-x 1` - Focus current buffer
- `C-x 2` - Split horizontally
- `C-x 3` - Split vertically

**Podcast Management:**
- `a` - Add podcast subscription
- `d` - Delete podcast
- `r` - Refresh podcast feed
- `R` - Refresh all feeds

**Episode Actions:**
- `D` - Download episode
- `X` - Delete downloaded file
- `C-x` - Delete ALL downloads for podcast

**Help:**
- `C-h ?` - Show help
- `C-h k` - Describe key
- `C-h b` - Show all bindings
- `M-x help` - M-x command for help

**Application:**
- `C-g` - Cancel current operation
- `C-x C-c` - Quit application
- `M-x quit` - Alternative quit command

## Configuration

The application will create configuration files on first run:

**Linux:**
```
~/.config/podcast-tui/config.json
~/.local/share/podcast-tui/
```

**Windows:**
```
%APPDATA%\podcast-tui\config.json
%LOCALAPPDATA%\podcast-tui\
```

### Basic Configuration Example

```json
{
  "downloads": {
    "directory": "~/Downloads/Podcasts",
    "concurrent": 3,
    "cleanup_after_days": 30
  },
  "ui": {
    "theme": "dark"
  },
  "storage": {
    "data_directory": null
  }
}
```

## Troubleshooting

### Build Issues

#### "failed to find tool 'clang'" (Windows ARM64)
- Install LLVM/Clang: See [scripts/INSTALL-LLVM.md](scripts/INSTALL-LLVM.md)
- Set `LIBCLANG_PATH` environment variable

#### "link.exe not found" (Windows x64)
- Install MSVC Build Tools: See [scripts/INSTALL-MSVC-TOOLS.md](scripts/INSTALL-MSVC-TOOLS.md)
- Or run: `.\scripts\install-build-deps.ps1`

#### "openssl not found" (Linux)
```bash
# Ubuntu/Debian
sudo apt-get install pkg-config libssl-dev

# Fedora/RHEL
sudo dnf install openssl-devel

# Arch
sudo pacman -S openssl
```

### Runtime Issues

#### Terminal doesn't display colors correctly
- Try changing theme: Press `M-x`, type `theme`, select different theme
- Ensure your terminal supports 256 colors
- Try a different terminal emulator (Windows Terminal recommended on Windows)

#### Keybindings don't work
- Some terminal emulators intercept certain key combinations
- Try alternative bindings (arrow keys instead of C-n/C-p)
- Check terminal emulator settings for key mapping conflicts

#### Download fails
- Check internet connection
- Verify RSS feed URL is correct
- Some feeds may have non-standard audio URL formats
- Check downloads directory permissions

#### Feed parsing fails
- Verify the URL is a valid RSS/Atom feed
- Some feeds have authentication requirements
- Try the feed URL in a web browser to verify it's accessible

## Getting Help

1. **Built-in Help**: Press `C-h ?` for the help system
2. **Documentation**: See [docs/](docs/) directory for detailed documentation
3. **Issues**: Report bugs at https://github.com/yourusername/podcast-tui/issues
4. **Keybindings Reference**: [docs/EMACS_KEYBINDINGS.md](docs/EMACS_KEYBINDINGS.md)

## Next Steps

1. **Import existing subscriptions**: Use OPML import (`M-x import-opml`)
2. **Customize configuration**: Edit `config.json` to your preferences
3. **Explore themes**: Try different color themes (`M-x theme`)
4. **Check for updates**: The project is in active development

## Development Status

**Completed (75% of core MVP):**
- ✅ Sprint 0: Foundation (Storage, Models, Config)
- ✅ Sprint 1: Core UI (Emacs-style TUI, Buffers, Themes)
- ✅ Sprint 2: RSS & Podcasts (Feed parsing, Subscriptions, OPML)
- ✅ Sprint 3: Downloads (Concurrent downloads, File management)

**In Progress (Next Sprints):**
- 🚧 Sprint 4: Audio Playback (rodio integration)
- 🚧 Sprint 5: Enhanced Features (Playlists, Notes, Search)
- 🚧 Sprint 6: Statistics & Polish
- 🚧 Sprint 7: Final testing & Documentation

---

**Last Updated**: October 2025  
**Version**: 1.0.0-mvp (in development)  
**Status**: Sprint 3 Complete
