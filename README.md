# Podcast TUI

A cross-platform terminal user interface for podcast management built with Rust.

![Build Status](https://github.com/lqdev/podcast-tui/workflows/CI/badge.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.75+-red.svg)
![Version](https://img.shields.io/badge/version-1.6.0-green)
![Development Status](https://img.shields.io/badge/status-Active%20Development-blue)

> 📚 **Documentation:** For comprehensive architecture and design patterns, see [**ARCHITECTURE.md**](docs/ARCHITECTURE.md)

## 📊 Current Status (February 2026)

**v1.6.0** — The application has a fully working feature set for podcast subscription management, downloading, device sync, playlist management, and audio playback.

✅ **Working Features:**
- RSS feed subscription management with OPML import/export
- Episode browsing with comprehensive metadata
- Parallel episode downloads (configurable concurrent, default 3)
- MP3 metadata embedding (ID3 tags, artwork, track numbers, readable filenames)
- Device sync to MP3 players/USB drives with metadata-based comparison
- Playlist management (user playlists + auto-generated "Today" playlist)
- Search & filter by text, status, date range
- Download cleanup (auto on startup + manual `:clean-older-than`)
- Audio playback with rodio backend and external player fallback
- Intuitive keyboard navigation and buffer management
- Multi-theme support (dark, light, high-contrast, solarized)
- Cross-platform builds (Windows x64/ARM64, Linux x64/ARM64)

⏳ **Not Yet Implemented:**
- Episode notes
- Statistics tracking

**⚠️ Episode notes and statistics tracking are not yet implemented.** The current release is suitable for managing subscriptions, downloading episodes, playing audio, syncing to devices, and managing playlists.

## 🎧 Features

### MVP Release (v1.6.0)

**✅ Completed Features:**
- ✅ **RSS Subscription Management** - Subscribe to podcasts via RSS feeds
- ✅ **OPML Import/Export** - Non-destructive import and export of subscriptions  
- ✅ **Episode Management** - Browse and manage episodes
- ✅ **Download System** - Parallel episode downloads with progress tracking and bulk cleanup
- ✅ **MP3 Metadata** - ID3 tags, artwork embedding, track numbers, readable folder names
- ✅ **Device Sync** - Sync downloads to MP3 players with metadata-based comparison
- ✅ **Keyboard Navigation** - Intuitive keybindings for efficient navigation
- ✅ **Command Auto-completion** - Intelligent command completion in minibuffer
- ✅ **Buffer Management** - Multiple buffers for different views
- ✅ **Playlist Support** - User playlists plus auto-generated `Today` (last 24h) playlist
- ✅ **Search & Filter** - Text search, status filter, date range filter
- ✅ **Theme System** - Multiple themes (dark, light, high-contrast, solarized)
- ✅ **Cross-platform Build** - Windows and Linux build support
- ✅ **Audio Playback** - Play downloaded episodes with rodio backend or external player

**⏳ In Progress / Planned:**
- ⏳ **Episode Notes** - Add personal notes to episodes (not yet implemented)
- ⏳ **Statistics Tracking** - Listen time and download statistics (not yet implemented)

## 🚀 Quick Start

> 📖 **Documentation:**
> - 📚 **Complete guide**: [**GETTING_STARTED.md**](GETTING_STARTED.md) - Detailed platform-specific instructions and quick start
> - 🏗️ **Architecture**: [**ARCHITECTURE.md**](docs/ARCHITECTURE.md) - System design and technical documentation

### Prerequisites
- Rust 1.75 or later
- Git

**⚠️ Important Build Notes:**
- **Windows ARM64**: Requires LLVM/Clang toolchain (see [scripts/INSTALL-LLVM.md](scripts/INSTALL-LLVM.md))
- **Windows x64**: Requires MSVC Build Tools (see [scripts/INSTALL-MSVC-TOOLS.md](scripts/INSTALL-MSVC-TOOLS.md))
- **Linux**: Standard build tools (gcc/clang) required

The application is currently **in active development** with core RSS/download features and audio playback complete.

### Installation

**🚧 Development Status**: Pre-built binaries are available for testing core features (RSS subscriptions, downloads, audio playback, and UI).

#### Windows (winget)
```powershell
winget install lqdev.PodcastTUI
```

#### Pre-built Binaries
Download the latest release for your platform from the [releases page](https://github.com/lqdev/podcast-tui/releases).

**Windows (manual):**
```powershell
# Download and extract podcast-tui-vX.X.X-windows-x86_64.zip
# Run podcast-tui.exe
```

**Linux:**
```bash
# Download and extract podcast-tui-vX.X.X-linux-x86_64.tar.gz
tar -xzf podcast-tui-vX.X.X-linux-x86_64.tar.gz
cd podcast-tui-vX.X.X-linux-x86_64
./podcast-tui
```

#### From Source
```bash
git clone https://github.com/lqdev/podcast-tui.git
cd podcast-tui
cargo build --release
./target/release/podcast-tui

# Optional: Install icon and desktop entry on Linux
./scripts/install-icon-linux.sh
```

#### Building Cross-Platform Releases

**Linux/macOS:**
```bash
# Install build dependencies (one-time setup)
./scripts/install-build-deps.sh

# Quick local build
./scripts/build-linux.sh
```

**Windows:**
```powershell
# Verify dependencies
.\scripts\install-build-deps.ps1

# Quick local build
.\scripts\build-windows.ps1
```

See [BUILD_SYSTEM.md](docs/BUILD_SYSTEM.md) for detailed build documentation and cross-platform build instructions.

#### Using DevContainer (Recommended for Development)
1. Install [Docker](https://docker.com) and [VS Code](https://code.visualstudio.com)
2. Install the [Remote-Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
3. Clone the repository and open in VS Code
4. Click "Reopen in Container" when prompted
5. Run `cargo run` to start the application

### Application Icon

Podcast TUI features a custom icon combining a cassette tape and RSS feed symbol, representing the audio content and subscription management capabilities.

**Linux:** After installing from a release package, run the included `install-icon-linux.sh` script to add the application icon to your system's application menu and file manager.

**Windows:** The icon is automatically embedded in the executable and will appear in the taskbar, Task Manager, and file explorer.

See [assets/README.md](assets/README.md) for more details about the icon design and installation.

### First Run
1. Start the application: `podcast-tui`
2. Press `a` to add your first podcast
3. Enter an RSS feed URL (try: `https://feeds.simplecast.com/54nAGcIl`)
4. Navigate with arrow keys or Up/Down to browse episodes
5. Press `D` to download episodes, `F1` or `?` for help
6. Press `Shift+Enter` on a downloaded episode to play it

## 🎹 Keybindings

### Navigation
- `↑` / `↓` - Move up/down
- `←` / `→` - Move left/right
- `Page Up` / `Page Down` - Scroll by page
- `Home` / `End` - Jump to top/bottom
- `Enter` - Select/activate item
- `Space` - Select/activate item
- `Tab` - Next buffer
- `Shift+Tab` - Previous buffer
- `Ctrl+Page Up` - Previous buffer (alternative)
- `Ctrl+Page Down` - Next buffer (alternative)

### Podcast Management
- `a` - Add new podcast subscription
- `d` - Delete selected podcast
- `r` - Refresh selected podcast feed
- `Shift+R` - Refresh all podcast feeds
- `Ctrl+r` - Hard refresh (re-parse all episodes)

### Episode Management  
- `Enter` - Open episode detail / navigate into playlist
- `Shift+D` - Download episode (works in episode list and episode detail)
- `Shift+X` or `X` - Delete downloaded file for selected episode
- `p` - Add selected episode to a playlist
- `Ctrl+x` - Delete ALL downloaded episodes and clean up
- `:clean-older-than <duration>` - Delete downloads older than duration (e.g., `7d`, `2w`, `1m`)
- `:cleanup <duration>` - Alias for clean-older-than

### Playlist Commands
- `:playlists` - Open playlist buffer
- `:playlist-create [name]` - Create playlist
- `:playlist-delete <name>` - Delete playlist
- `:playlist-refresh` - Refresh `Today` auto-playlist
- `:playlist-sync` - Sync podcasts + playlists to device

### Buffer Management
- `F2` - Switch to podcast list
- `F3` - Switch to help
- `F4` - Switch to downloads
- `F5` - Refresh current buffer
- `F7` - Switch to playlists
- `Ctrl+b` - Show buffer list / Switch buffer
- `Ctrl+k` - Close current buffer
- `Ctrl+l` - List all buffers

### Search & Filter Commands
- `/` - Open search (filter by text, matches title + description)
- `:filter-status <new|downloaded|played|downloading|failed>` - Filter by status
- `:filter-date <today|7d|2w|1m>` - Filter by date range
- `:clear-filters` - Clear all active filters

### Application
- `F1` - Show help
- `h` or `?` - Show help
- `:` - Command prompt
- `Esc` - Cancel/hide minibuffer
- `q` - Quit application
- `F10` - Quit application

### Audio Playback
- `Shift+P` - Toggle play/pause
- `Shift+Enter` - Play selected downloaded episode
- `Ctrl+Left` - Seek backward 10s
- `Ctrl+Right` - Seek forward 10s
- `+` / `=` - Volume up
- `-` - Volume down
- `F9` - Open now playing buffer

See [complete keybinding reference](docs/KEYBINDINGS.md) for all shortcuts.

## ⚙️ Configuration

Configuration is stored in JSON format at:
- Linux: `~/.config/podcast-tui/config.json`
- Windows: `%APPDATA%/podcast-tui/config.json`

### Example Configuration
```json
{
  "downloads": {
    "directory": "~/Downloads/Podcasts",
    "concurrent_downloads": 3,
    "cleanup_after_days": 30,
    "sync_device_path": "/mnt/mp3player",
    "sync_delete_orphans": true,
    "sync_preserve_structure": true,
    "sync_dry_run": false,
    "sync_include_playlists": true,
    "use_readable_folders": true,
    "embed_id3_metadata": true,
    "assign_track_numbers": true,
    "download_artwork": true,
    "include_episode_numbers": true,
    "include_dates": false,
    "max_filename_length": 100
  },
  "playlist": {
    "today_refresh_policy": "daily",
    "auto_download_on_add": true,
    "download_retries": 3
  },
  "audio": {
    "volume": 0.8,
    "seek_seconds": 10,
    "external_player": null,
    "auto_play_next": false,
    "remember_position": true
  },
  "ui": {
    "theme": "dark",
    "show_progress_bar": true,
    "whats_new_episode_limit": 50
  }
}
```

### Device Sync Configuration

The device sync feature allows you to sync downloaded episodes and playlists to external MP3 players or USB devices:

- `sync_device_path`: Default path to your device (can be overridden at runtime)
- `sync_delete_orphans`: Remove files on device that aren't on PC (default: true)
- `sync_preserve_structure`: Keep podcast folder structure on device (default: true)  
- `sync_dry_run`: Preview changes without applying them (default: false)
- `sync_include_playlists`: Include playlists in sync (default: true)

**Usage:**
```bash
# Sync to device (prompts for path or uses config default)
:sync

# Sync to specific parent path (creates Podcasts/ and Playlists/)
:sync /mnt/usb/Music

# Preview changes without applying
:sync-dry-run /mnt/usb/Music

# View sync history
:buffer sync
```

See [configuration documentation](docs/CONFIGURATION.md) for all options.

## 📁 Data Storage

Podcast TUI uses JSON files for data storage:

```
~/.local/share/podcast-tui/
├── config.json                 # Application configuration  
├── podcasts/                   # Podcast subscriptions
│   ├── {podcast-id}.json
├── episodes/                   # Episode metadata and notes
│   ├── {podcast-id}/
│   │   ├── {episode-id}.json
├── playlists/                  # Playlists metadata + audio copies
│   ├── Morning Commute/
│   │   ├── playlist.json
│   │   └── audio/
│   │       ├── 001-episode.mp3
│   ├── Today/
│   │   ├── playlist.json
│   │   └── audio/
└── stats.json                  # Usage statistics
```

This design allows for:
- Easy manual editing of data
- Simple backup (copy directory)
- Version control friendly
- Future storage backend options

## 🔧 Development

### Architecture
The application follows a modular architecture with clear separation of concerns:

- **Storage Layer** - Trait-based abstraction with JSON implementation
- **Domain Logic** - Podcast, episode, and download management
- **UI Layer** - Buffer-based terminal interface using Ratatui
- **Network Layer** - Async HTTP with connection pooling

See [**ARCHITECTURE.md**](docs/ARCHITECTURE.md) for comprehensive technical documentation including:
- Core architectural principles and design patterns
- Module structure and dependencies
- Storage abstraction design
- UI component patterns
- Data flow diagrams

### Contributing
We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Development setup instructions
- Code style guidelines (also in [.github/copilot-instructions.md](.github/copilot-instructions.md))
- Sprint process and project management
- Pull request requirements
- Architecture guidelines and best practices
- Testing strategy and guidelines (see [docs/TESTING.md](docs/TESTING.md))

### Project Management
- **PRD**: [Product Requirements Document](docs/PRD.md)
- **Implementation Plan**: [8-week sprint plan](docs/IMPLEMENTATION_PLAN.md)
- **Project Board**: Track progress and current sprint
- **Issues**: Bug reports and feature requests

## 🏗️ Technology Stack

- **Language**: Rust 2021
- **TUI Framework**: [Ratatui](https://ratatui.rs/) + [Crossterm](https://github.com/crossterm-rs/crossterm)
- **Audio**: [Rodio](https://github.com/RustAudio/rodio)
- **HTTP**: [Reqwest](https://github.com/seanmonstar/reqwest)
- **RSS Parsing**: [feed-rs](https://github.com/feed-rs/feed-rs)
- **Serialization**: [Serde](https://serde.rs/)
- **Async Runtime**: [Tokio](https://tokio.rs/)

## 📋 Roadmap

### Completed

- [x] **Project Setup** - Rust project structure, dependencies, and tooling
- [x] **Storage Layer** - JSON-based storage with abstraction trait
- [x] **Data Models** - Podcast, Episode, and configuration models
- [x] **Core UI Framework** - TUI with Emacs-style buffers and keybindings
- [x] **Theme System** - Multiple themes with dynamic switching
- [x] **RSS Subscription Management** - Subscribe to podcasts via RSS feeds
- [x] **OPML Import/Export** - Non-destructive import and export of subscriptions
- [x] **Episode Parsing** - RSS feed parsing and episode extraction
- [x] **Download System** - Parallel episode downloads with progress tracking
- [x] **Episode Management UI** - Browse and manage episodes
- [x] **Download Cleanup** - Auto-cleanup on startup + manual `:clean-older-than`
- [x] **Application Icon** - Custom cassette+RSS icon, embedded in Windows exe
- [x] **Device Sync** - Metadata-based sync to MP3 players/USB devices
- [x] **MP3 Metadata** - ID3 tags, artwork, track numbers, readable filenames
- [x] **Search & Filter** - Text search, status filter, date range filter
- [x] **Playlist Management** - User playlists + auto-generated "Today" playlist
- [x] **Winget Publishing** - Available on Windows Package Manager
- [x] **Audio Playback** - Rodio backend with external player fallback

### In Progress / Planned

- [ ] **Episode Notes** - Add personal notes to episodes
- [ ] **Statistics Tracking** - Listen time and download statistics
- [ ] **Duration Filter** - Filter episodes by duration (deferred pending RSS duration data)

### Post-MVP (v2.0+)
- [ ] SQLite storage backend option
- [ ] Advanced smart playlists
- [ ] Plugin architecture
- [ ] Cloud synchronization (optional)

## 🐛 Known Issues

**Not Yet Implemented:**
- ⚠️ **Episode notes** - planned for a future release
- ⚠️ **Statistics tracking** - planned for a future release
- ⚠️ **Duration filter** - deferred until episode duration is reliably populated from RSS feeds

**Build Requirements:**
- Windows ARM64 builds require LLVM/Clang (see scripts/INSTALL-LLVM.md)
- Windows x64 builds require MSVC Build Tools
- Linux requires `libasound2-dev` and `pkg-config`

**Current Limitations:**
- RSS feed parsing works with most standard feeds; some non-standard audio URL formats may not parse correctly
- Terminal compatibility tested on Windows Terminal, GNOME Terminal, and similar

See [GitHub Issues](https://github.com/lqdev/podcast-tui/issues) for current bugs and feature requests.

## 📜 License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- [Ratatui](https://ratatui.rs/) community for excellent TUI framework
- [feed-rs](https://github.com/feed-rs/feed-rs) for robust RSS parsing
- Terminal UI community for design inspiration
- Rust community for excellent tooling and libraries

---

**Status**: 🚀 Active Development (v1.6.0)  
**Maintainer**: [@lqdev](https://github.com/lqdev)  
**Version**: 1.6.0
