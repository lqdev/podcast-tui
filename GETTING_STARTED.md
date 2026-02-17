# Getting Started with Podcast TUI

## Quick Start Guide

This guide will help you get Podcast TUI running on your system, regardless of platform.

---

## TL;DR - 5 Minute Quick Start

### What Works Right Now (Sprint 3 Complete)
✅ Subscribe to RSS podcast feeds  
✅ Browse episodes with full metadata  
✅ Download episodes (2-3 at a time)  
✅ Create/manage playlists and auto-generated `Today` playlist  
✅ OPML import/export  
✅ Intuitive keyboard shortcuts  
✅ 4 color themes  

### Not Yet Working
❌ Audio playback (coming in Sprint 4)  
❌ Episode notes (Sprint 5)  
❌ Statistics (Sprint 6)  

### Speed Run Installation

**Windows x64:**
```powershell
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools
git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui
cargo build --release
.\target\release\podcast-tui.exe
```

**Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install build-essential pkg-config libssl-dev  # Ubuntu/Debian
git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui
cargo build --release
./target/release/podcast-tui
```

### Essential Keys to Know
- `F1` or `?` → Help
- `:` → Command prompt
- `a` → Add podcast
- `F7` → Open playlists buffer
- `↓`/`↑` → Navigate
- `Enter` → Select
- `Shift+D` → Download episode
- `p` → Add selected episode to playlist
- `q` or `F10` → Quit

---

## Current Development Status (October 2025)

**✅ What Works:**
- Subscribe to podcast RSS feeds
- Browse episode lists with metadat- **Keybindings Reference**: [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)
- Download episodes (2-3 concurrent downloads)
- OPML import/export
- Intuitive keyboard navigation
- Multiple color themes
- Cross-platform builds (Windows/Linux)

**🚧 What's Coming:**
- Audio playback (Sprint 4 - next up)
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

**Method 1: Using Quick Key**
1. Press `a` from the podcast list
2. Enter the RSS feed URL
3. Press Enter
4. Wait for the feed to parse and episodes to load

**Method 2: Using Command Prompt**
1. Press `:` to open command prompt
2. Type `add-podcast <URL>` and press Enter
3. Wait for the feed to parse

**Popular Test Feeds:**
- The Changelog: `https://changelog.com/podcast/feed`
- Syntax: `https://feed.syntax.fm/rss`
- Reply All: `https://feeds.megaphone.fm/replyall`

### 3. Browse and Download Episodes

1. Use Up/Down arrow keys to navigate episodes
2. Press `Enter` on an episode to view details
3. Press `Shift+D` to download the episode (works in both episode list and episode detail views)
4. Press `Tab` or `Ctrl+b` to switch between buffers (podcast list, episode list, downloads)

### 4. Essential Keybindings

**Navigation:**
- `↑` / `↓` - Previous/Next item
- `←` / `→` - Left/Right
- `Home` / `End` - First/Last item
- `Page Up` / `Page Down` - Scroll by page
- `Enter` - Select/Activate item

**Buffer Management:**
- `Tab` / `Shift+Tab` - Next/Previous buffer
- `Ctrl+b` - Switch buffer (with completion)
- `Ctrl+k` - Close current buffer
- `Ctrl+l` - List all buffers
- `F2` - Podcast list
- `F3` - Help
- `F4` - Downloads
- `F7` - Playlists

**Podcast Management:**
- `a` - Add podcast subscription
- `d` - Delete podcast
- `r` - Refresh podcast feed
- `Shift+R` - Refresh all feeds
- `Ctrl+r` - Hard refresh (re-parse all episodes)

**Episode Actions:**
- `Shift+D` - Download episode
- `Shift+X` - Delete downloaded file
- `p` - Add selected episode to playlist
- `Ctrl+x` - Delete ALL downloads
- `:clean-older-than <dur>` - Delete downloads older than duration (e.g., `7d`, `2w`)
- `:cleanup <dur>` - Alias for clean-older-than

**Help:**
- `F1` or `?` or `h` - Show help
- `:` - Open command prompt

**Application:**
- `Esc` - Cancel current operation
- `q` or `F10` - Quit application
- `:quit` - Alternative quit command

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
    "concurrent_downloads": 3,
    "cleanup_after_days": 30,
    "sync_include_playlists": true
  },
  "playlist": {
    "today_refresh_policy": "daily",
    "auto_download_on_add": true,
    "download_retries": 3
  },
  "ui": {
    "theme": "dark"
  },
  "storage": {
    "data_directory": null
  }
}
```

## Good Test Feeds

Try these popular podcasts to get started:

- **The Changelog**: `https://changelog.com/podcast/feed`
- **Syntax**: `https://feed.syntax.fm/rss`
- **Reply All**: `https://feeds.megaphone.fm/replyall`

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
- Most navigation works with arrow keys, Page Up/Down, Home/End
- Function keys (F1-F10) work in most terminals
- Check terminal emulator settings for key mapping conflicts
- Try Windows Terminal on Windows for best compatibility

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

1. **Built-in Help**: Press `F1` or `?` for the help system
2. **Documentation**: See [docs/](docs/) directory for detailed documentation
3. **Issues**: Report bugs at https://github.com/yourusername/podcast-tui/issues
4. **Keybindings Reference**: [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)

## Next Steps

1. **Import existing subscriptions**: Use OPML import (press `:` then type `import-opml`)
2. **Customize configuration**: Edit `config.json` to your preferences
3. **Explore themes**: Try different color themes (press `:` then type `theme <name>`)
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
