# Podcast TUI

A cross-platform terminal user interface for podcast management built with Rust.

![Build Status](https://github.com/lqdev/podcast-tui/workflows/CI/badge.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.75+-red.svg)
![Version](https://img.shields.io/badge/version-1.12.0-green)
![Development Status](https://img.shields.io/badge/status-Active%20Development-blue)

> 📚 **Documentation:** For comprehensive architecture and design patterns, see [**ARCHITECTURE.md**](docs/ARCHITECTURE.md)

## 📊 Current Status (February 2026)

**v1.12.0** — The application has a fully working feature set for podcast subscription management, downloading, device sync, playlist management, audio playback, discovery, tagging, and optional scrobbling.

✅ **Working Features:**
- RSS feed subscription management with OPML import/export
- Episode browsing with comprehensive metadata
- Parallel episode downloads (configurable concurrent, default 3)
- MP3 metadata embedding (ID3 tags, artwork, track numbers, readable filenames)
- Device sync via interactive Sync buffer (F8) with dry-run preview and live progress
- Playlist management (user playlists, `Today` auto-playlist, smart playlists)
- Search & filter by text, status, date range, favorites, and tags
- Download cleanup (auto on startup + manual `:clean-older-than`)
- Audio playback with NowPlaying buffer (F9), rodio backend, external player fallback
- Podcast discovery via PodcastIndex API (`:discover`, `:trending`)
- Favorites (`*`), mark played (`m`) / unplayed (`u`), podcast tagging
- 10+ themes: 4 built-in + 5 community bundled + user TOML themes with `extends`
- Keybinding presets (default, vim, emacs)
- Optional ListenBrainz scrobbling (disabled by default)
- Cross-platform builds (Windows x64/ARM64, Linux x64/ARM64) + NixOS flake

⏳ **Not Yet Implemented:**
- Episode notes
- Statistics tracking

**⚠️ Episode notes and statistics tracking are not yet implemented.** The current release is suitable for managing subscriptions, downloading episodes, playing audio, syncing to devices, managing playlists, and discovering new podcasts.

## 🎧 Features

### MVP Release (v1.6.0)

**✅ Completed Features:**
- ✅ **RSS Subscription Management** - Subscribe to podcasts via RSS feeds
- ✅ **OPML Import/Export** - Non-destructive import and export of subscriptions  
- ✅ **Episode Management** - Browse and manage episodes
- ✅ **Download System** - Parallel episode downloads with progress tracking and bulk cleanup
- ✅ **MP3 Metadata** - ID3 tags, artwork embedding, track numbers, readable folder names
- ✅ **Device Sync** - Sync downloads to MP3 players via interactive Sync buffer (F8) with dry-run preview and live progress
- ✅ **Keyboard Navigation** - Intuitive keybindings for efficient navigation (default/vim/emacs presets)
- ✅ **Command Auto-completion** - Intelligent command completion in minibuffer
- ✅ **Buffer Management** - Multiple buffers for different views
- ✅ **Playlist Support** - User playlists, auto-generated `Today` (last 24h) playlist, and smart playlists
- ✅ **Search & Filter** - Text search, status filter (including favorites), date range filter, tag filter
- ✅ **Theme System** - 4 built-in themes + 5 community themes (catppuccin-mocha, dracula, nord, gruvbox-dark, tokyo-night) + user TOML themes
- ✅ **Cross-platform Build** - Windows and Linux build support + NixOS flake
- ✅ **Audio Playback** - Play downloaded episodes with NowPlaying buffer (F9), rodio backend or external player
- ✅ **Podcast Discovery** - Search and browse podcasts via PodcastIndex API (`:discover`, `:trending`)
- ✅ **Tagging** - Organize podcasts with tags (`:tag`, `:untag`, `:filter-tag`)
- ✅ **Favorites & Playback State** - Favorite episodes (`*`), mark played/unplayed (`m`/`u`)
- ✅ **Optional Scrobbling** - ListenBrainz scrobbling support (disabled by default)

**⏳ In Progress / Planned:**
- ⏳ **Episode Notes** - Add personal notes to episodes (not yet implemented)
- ⏳ **Statistics Tracking** - Listen time and download statistics (not yet implemented)

## 🎨 Customization

Podcast TUI exposes three customization surfaces in `config.json`. See [`docs/KEYBINDINGS.md`](docs/KEYBINDINGS.md) and [`docs/DEVICE_PROFILES.md`](docs/DEVICE_PROFILES.md) for the full reference.

- **Startup buffer** — set `ui.startup_buffer` to one of `help`, `podcast-list`, `downloads`, `sync`, `playlist-list`, `whats-new`, `now-playing` to control which buffer opens on launch. Default: `help`.
- **Keybinding presets** — set `keybindings.preset` to `default`, `vim`, or `emacs`. Per-action overrides go under `keybindings.bindings`.
- **Device profiles** — for MP3 players that show the literal filename (e.g. Innioasis Y1, generic USB DACs), define a profile under `device_profiles[]` with a `filename_template` and select it via `active_device_profile`. The local downloads directory is untouched; only the device-side filenames are rewritten during sync. Give a profile a `sync_path` and switching to it points the active sync target at that device automatically. The `:set-device-profile` minibuffer command switches profiles at runtime.

The storage layer also caches all podcasts, episodes, and playlists in memory and persists the index to `cache_index.json`, making warm launches near-instant. Toggle with `storage.cache_enabled` (default `true`); rebuild with `:cache-rebuild`. See [`docs/STORAGE_DESIGN.md`](docs/STORAGE_DESIGN.md) for benchmark numbers.

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

#### NixOS / Nix
```bash
# Try it (zero commitment)
nix run github:lqdev/podcast-tui

# Install to profile
nix profile install github:lqdev/podcast-tui

# Or add to your NixOS configuration (flake-based):
# inputs.podcast-tui.url = "github:lqdev/podcast-tui";
# environment.systemPackages = [ podcast-tui.packages.${system}.default ];
```

> 📖 See [docs/NIX_PACKAGING.md](docs/NIX_PACKAGING.md) for declarative config, Home Manager, development shell, and more.

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
- `*` - Toggle episode as favorite
- `m` - Mark episode as played
- `u` - Mark episode as unplayed
- `Ctrl+x` - Delete ALL downloaded episodes and clean up
- `:clean-older-than <duration>` - Delete downloads older than duration (e.g., `7d`, `2w`, `1m`)
- `:cleanup <duration>` - Alias for clean-older-than

### Playlist Commands
- `:playlists` - Open playlist buffer
- `:playlist-create [name]` - Create playlist
- `:playlist-delete <name>` - Delete playlist
- `:playlist-refresh` - Refresh `Today` auto-playlist
- `:playlist-sync` - Sync podcasts + playlists to device
- `:smart-playlist <name>` - Create a smart playlist with auto-generated rules (`⚡` prefix)

### Podcast Discovery
- `:discover <query>` - Search for new podcasts via PodcastIndex API
- `:trending` - Browse trending podcasts

### Tagging
- `:tag <tag>` - Add a tag to the selected podcast
- `:untag <tag>` - Remove a tag from the selected podcast
- `:filter-tag <tag>` - Filter podcast list by tag

### Themes
- `:theme <name>` - Switch theme
  - Built-in: `dark`, `light`, `high-contrast`, `solarized`
  - Community: `catppuccin-mocha`, `dracula`, `nord`, `gruvbox-dark`, `tokyo-night`
  - Custom: any name from `~/.config/podcast-tui/themes/<name>.toml`

### Buffer Management
- `F2` - Switch to podcast list
- `F3` - Search (same as `/`)
- `F4` - Switch to downloads
- `F5` - Refresh current buffer
- `F6` - Clear all active filters
- `F7` - Switch to playlists
- `F8` - Switch to sync buffer
- `F9` - Open NowPlaying buffer
- `Ctrl+b` - Show buffer list / Switch buffer
- `Ctrl+k` - Close current buffer
- `Ctrl+l` - List all buffers

### Search & Filter Commands
- `/` or `F3` - Open search (filter by text, matches title + description)
- `:filter-status <new|downloaded|played|downloading|failed|favorited>` - Filter by status
- `:filter-date <today|12h|7d|2w|1m>` - Filter by date range
- `:filter-tag <tag>` - Filter podcasts by tag
- `:clear-filters` or `F6` - Clear all active filters

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
    "max_filename_length": 150
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
  },
  "discovery": {
    "podcastindex_api_key": "",
    "podcastindex_api_secret": "",
    "max_results": 20
  },
  "scrobbling": {
    "enabled": false,
    "server_url": "https://api.listenbrainz.org",
    "token": ""
  }
}
```

### Device Sync Configuration

The device sync feature allows you to sync downloaded episodes and playlists to external MP3 players or USB devices via the interactive **Sync buffer** (`F8`):

- `sync_device_path`: Default path to your device (can be overridden at runtime via `p` in the Sync buffer)
- `sync_delete_orphans`: Remove files on device that aren't on PC (default: true)
- `sync_preserve_structure`: Keep podcast folder structure on device (default: true)  
- `sync_dry_run`: Preview changes without applying them (default: false)
- `sync_include_playlists`: Include playlists in sync (default: true)

**Usage (via Sync buffer — `F8`):**

Press `F8` to open the Sync buffer, then:
- `s` - Start sync to configured device path
- `d` - Run a dry-run preview (shows what would change)
- `p` - Open directory picker to choose a different target path

The Sync buffer shows live progress during sync and a history of past sync operations.

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
└── themes/                     # Optional user TOML theme files
    └── my-theme.toml
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
- [x] **Theme System** - 4 built-in + 5 community themes + user TOML themes with `extends`
- [x] **Keybinding Presets** - Default, vim, and emacs preset support
- [x] **RSS Subscription Management** - Subscribe to podcasts via RSS feeds
- [x] **OPML Import/Export** - Non-destructive import and export of subscriptions
- [x] **Episode Parsing** - RSS feed parsing and episode extraction
- [x] **Download System** - Parallel episode downloads with progress tracking
- [x] **Episode Management UI** - Browse and manage episodes
- [x] **Download Cleanup** - Auto-cleanup on startup + manual `:clean-older-than`
- [x] **Application Icon** - Custom cassette+RSS icon, embedded in Windows exe
- [x] **Device Sync** - Interactive Sync buffer (F8) with dry-run, directory picker, live progress
- [x] **MP3 Metadata** - ID3 tags, artwork, track numbers, readable filenames
- [x] **Search & Filter** - Text search, status filter (including favorites), date range filter, tag filter
- [x] **Playlist Management** - User playlists + auto-generated `Today` playlist
- [x] **Smart Playlists** - Rule-based playlists via `:smart-playlist`
- [x] **Winget Publishing** - Available on Windows Package Manager
- [x] **Audio Playback** - Rodio backend with NowPlaying buffer (F9) and external player fallback
- [x] **Podcast Discovery** - Search and browse via PodcastIndex API (`:discover`, `:trending`)
- [x] **Tagging** - Organize podcasts with custom tags
- [x] **Favorites & Playback State** - Mark favorites (`*`), played (`m`) / unplayed (`u`)
- [x] **ListenBrainz Scrobbling** - Optional listen tracking (disabled by default)
- [x] **NixOS Flake** - Nix packaging for NixOS and nix-based systems

### In Progress / Planned

- [ ] **Episode Notes** - Add personal notes to episodes
- [ ] **Statistics Tracking** - Listen time and download statistics
- [ ] **Duration Filter** - Filter episodes by duration (deferred pending RSS duration data)

### Post-MVP (v2.0+)
- [ ] SQLite storage backend option
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

**Status**: 🚀 Active Development (v1.12.0)  
**Maintainer**: [@lqdev](https://github.com/lqdev)  
**Version**: 1.12.0
