# Getting Started with Podcast TUI

## Quick Start Guide

This guide will help you get Podcast TUI running on your system, regardless of platform.

---

## TL;DR - 5 Minute Quick Start

### What Works Right Now (v1.6.0)
✅ Subscribe to RSS podcast feeds  
✅ Browse episodes with full metadata  
✅ Download episodes (configurable concurrent)  
✅ Create/manage playlists and auto-generated `Today` playlist  
✅ Sync to MP3 players/USB devices  
✅ Search & filter episodes (text, status, date range)  
✅ OPML import/export  
✅ Audio playback (rodio backend + external player fallback)  
✅ Intuitive keyboard shortcuts  
✅ 4 color themes  

### Not Yet Working
❌ Episode notes  
❌ Statistics  

### Speed Run Installation

**Windows x64:**
```powershell
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools
git clone https://github.com/lqdev/podcast-tui.git
cd podcast-tui
cargo build --release
.\target\release\podcast-tui.exe
```

**Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install build-essential pkg-config libssl-dev libasound2-dev  # Ubuntu/Debian
git clone https://github.com/lqdev/podcast-tui.git
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
- `Shift+Enter` → Play downloaded episode
- `Shift+P` → Toggle play/pause
- `p` → Add selected episode to playlist
- `q` or `F10` → Quit

---

## Current Development Status (February 2026)

**✅ What Works (v1.6.0):**
- Subscribe to podcast RSS feeds
- Browse episode lists with metadata
- Download episodes (configurable concurrent downloads)
- Device sync to MP3 players/USB drives
- Search & filter episodes (text, status, date range)
- OPML import/export
- Playlist management (user + auto-generated Today playlist)
- Audio playback (rodio backend, external player fallback)
- Intuitive keyboard navigation
- Multiple color themes
- Cross-platform builds (Windows/Linux)

**🚧 What's Coming:**
- Episode notes
- Statistics tracking

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
git clone https://github.com/lqdev/podcast-tui.git
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
git clone https://github.com/lqdev/podcast-tui.git
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
git clone https://github.com/lqdev/podcast-tui.git
cd podcast-tui

# Build the project
cargo build --release

# Run the application
./target/release/podcast-tui
```

#### Using Pre-built Binaries
```bash
# Download the latest release
wget https://github.com/lqdev/podcast-tui/releases/download/vX.X.X/podcast-tui-vX.X.X-linux-x86_64.tar.gz

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

### 4. Playing Episodes

1. Navigate to a downloaded episode in the episode list
2. Press `Shift+Enter` to play the episode
3. Press `F9` to open the now playing buffer (shows progress, volume, and playback state)

**Playback Controls (work from any buffer):**
- `Shift+P` - Toggle play/pause (also ⏯ media key)
- `Ctrl+Left` - Seek backward 10s
- `Ctrl+Right` - Seek forward 10s
- `+` / `=` - Volume up
- `-` - Volume down

**External Player:**
If the built-in rodio backend is unavailable, Podcast TUI falls back to an external player. Configure a preferred player in `config.json`:

```json
{
  "audio": {
    "external_player": "mpv"
  }
}
```

### 5. Essential Keybindings

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
- `Shift+X` or `X` - Delete downloaded file
- `p` - Add selected episode to playlist
- `Ctrl+x` - Delete ALL downloads
- `:clean-older-than <dur>` - Delete downloads older than duration (e.g., `7d`, `2w`)
- `:cleanup <dur>` - Alias for clean-older-than

**Search & Filter:**
- `/` - Open search (filter episodes by text)
- `:filter-status <status>` - Filter by status (`new`, `downloaded`, `played`, `downloading`, `failed`)
- `:filter-date <range>` - Filter by date (`today`, `7d`, `2w`, `1m`)
- `:clear-filters` - Clear all active filters

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
    "sync_include_playlists": true,
    "sync_preview_before_sync": false,
    "sync_filter_removable_only": false
  },
  "playlist": {
    "today_refresh_policy": "daily",
    "auto_download_on_add": true,
    "download_retries": 3
  },
  "ui": {
    "theme": "dark"
  },
  "audio": {
    "volume": 0.8,
    "seek_seconds": 10,
    "external_player": null,
    "auto_play_next": false,
    "remember_position": true
  },
  "storage": {
    "data_directory": null
  }
}
```

## Device Sync

Sync your downloaded episodes and playlists to an external device (USB drive, MP3 player, etc.).

### Basic Workflow

1. Press `F8` to open the Sync buffer.
2. Press `s` to sync immediately (or be prompted for a path if no target is saved).
3. Press `d` to perform a **dry-run preview** first — see exactly what would be copied,
   deleted, and skipped before committing.

### Dry-Run Preview

Pressing `d` runs a dry-run and opens a tabbed preview showing:

| Tab | Contents |
|-----|----------|
| **To Copy** | Files that will be copied to the device (with sizes) |
| **To Delete** | Orphan files that will be removed from the device |
| **Skipped** | Files already identical on the device |
| **Errors** | Any problems encountered |

Use `[` and `]` to cycle between tabs. Press `Enter` or `s` to confirm and start the real sync,
or `Esc` to cancel.

### Progress View

When a real sync is running, the buffer switches to a live progress view showing:
- Byte-based progress bar (`bytes copied / total bytes`)
- Currently-copying filename
- Running counters: ✅ Copied / 🗑️ Deleted / ⏭ Skipped / ❌ Errors
- Elapsed time

The view automatically transitions back to Overview when the sync completes.

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `sync_preview_before_sync` | `false` | If `true`, pressing `s` always shows a dry-run preview first |
| `sync_filter_removable_only` | `false` | If `true`, directory picker only shows removable/external drives |
| `sync_delete_orphans` | `true` | Delete device files that no longer exist on the PC |
| `sync_include_playlists` | `true` | Include playlist files in the sync |
| `sync_device_path` | `null` | Default device path used when no saved target exists |

### Commands

```
:sync <path>             Sync to device at <path>
:sync --hard <path>      Wipe managed directories first, then fresh copy
:sync-dry-run <path>     Preview sync without applying changes
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
3. **Issues**: Report bugs at https://github.com/lqdev/podcast-tui/issues
4. **Keybindings Reference**: [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)

## Next Steps

1. **Import existing subscriptions**: Use OPML import (press `:` then type `import-opml`)
2. **Customize configuration**: Edit `config.json` to your preferences
3. **Explore themes**: Try different color themes (press `:` then type `theme <name>`)
4. **Check for updates**: The project is in active development

## Development Status

**Completed:**
- ✅ Foundation (Storage, Models, Config)
- ✅ Core UI (Emacs-style TUI, Buffers, Themes)
- ✅ RSS & Podcasts (Feed parsing, Subscriptions, OPML)
- ✅ Downloads (Concurrent downloads, File management, Cleanup)
- ✅ Device Sync (MP3 player sync, metadata-based comparison)
- ✅ Search & Filter (Text, status, date range)
- ✅ Playlists (User playlists + Today auto-playlist)
- ✅ Audio Playback (rodio backend, external player fallback, now playing buffer)

**In Progress / Planned:**
- 🚧 Episode Notes
- 🚧 Statistics & Reporting

---

**Last Updated**: February 2026  
**Version**: 1.6.0  
**Status**: Active Development
