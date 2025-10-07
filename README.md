# Podcast TUI

A cross-platform terminal u✅ **Completed Features:**
- ✅ **RSS Subscription Management** - Subscribe to podcasts via RSS feeds
- ✅ **OPML Import/Export** - Import/export subscriptions with duplicate detection ([docs](docs/OPML_SUPPORT.md))
- ✅ **Episode Management** - Browse and manage episodesinterface for podcast management built with Rust.

![Build Status](https://github.com/yourusername/podcast-tui/workflows/CI/badge.svg)
![License](https://img.shiel### Architecture
The application follows a modular architecture with clear separation of concerns:

- **Storage Layer** - Abstracted JSON-based persistence
- **Domain Logic** - Podcast, episode, and playlist management
- **UI Layer** - Terminal interface using Ratatui with buffer-based UI
- **Audio System** - Cross-platform playback with Rodioadge/license-MIT-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.75+-red.svg)
![Development Status](https://img.shields.io/badge/status-Sprint%203%20Complete-blue)
![Progress](https://img.shields.io/badge/MVP%20Progress-37.5%25-yellow)

> 📚 **Documentation:** For comprehensive architecture and design patterns, see [**ARCHITECTURE.md**](docs/ARCHITECTURE.md)

## 📊 Current Status (October 2025)

**🎉 Sprint 3 Complete!** The application has completed its first 3 sprints (37.5% of MVP):

✅ **Working Features:**
- RSS feed subscription management with OPML import/export
- Episode browsing with comprehensive metadata
- Parallel episode downloads (2-3 concurrent, configurable)
- Intuitive keyboard navigation and buffer management
- Multi-theme support (dark, light, high-contrast, solarized)
- Cross-platform builds (Windows x64/ARM64, Linux x64/ARM64)

🚧 **In Progress (Sprint 4 - Next Up):**
- Audio playback with rodio
- Playback controls and progress tracking
- Chapter navigation

📋 **Planned (Sprints 5-7):**
- Playlists, episode notes, search & filtering
- Statistics tracking and reporting
- Final polish and documentation

**⚠️ Audio playback is not yet implemented.** The current release is suitable for managing subscriptions and downloading episodes.

## 🎧 Features

### MVP Release (v1.0.0-mvp) - In Progress

**✅ Completed Features:**
- ✅ **RSS Subscription Management** - Subscribe to podcasts via RSS feeds
- ✅ **OPML Import/Export** - Non-destructive import and export of subscriptions  
- ✅ **Episode Management** - Browse and manage episodes
- ✅ **Download System** - Parallel episode downloads with progress tracking and bulk cleanup
- ✅ **Keyboard Navigation** - Intuitive keybindings for efficient navigation
- ✅ **Command Auto-completion** - Intelligent command completion in minibuffer
- ✅ **Buffer Management** - Multiple buffers for different views
- ✅ **Theme System** - Multiple themes (dark, light, high-contrast, solarized)
- ✅ **Cross-platform Build** - Windows and Linux build support

**🚧 In Progress / Planned:**
- ⏳ **Audio Playback** - Basic playback controls (not yet implemented)
- ⏳ **Playlist Creation** - Create and manage custom episode playlists (not yet implemented)
- ⏳ **Episode Notes** - Add personal notes to episodes (not yet implemented)
- ⏳ **Statistics Tracking** - Listen time and download statistics (not yet implemented)
- ⏳ **Search & Filtering** - Episode search and filtering (not yet implemented)

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

The application is currently **in active development** with core RSS/download features complete but audio playback not yet implemented.

### Installation

**🚧 Development Status**: Pre-built binaries are available for testing core features (RSS subscriptions, downloads, and UI). Audio playback is not yet functional.

#### Pre-built Binaries
Download the latest release for your platform from the [releases page](https://github.com/yourusername/podcast-tui/releases).

**Windows:**
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
git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui
cargo build --release
./target/release/podcast-tui
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

### First Run
**Note:** Audio playback is not yet implemented. Current features include subscription management, episode browsing, and downloading.

1. Start the application: `podcast-tui`
2. Press `a` to add your first podcast
3. Enter an RSS feed URL (try: `https://feeds.simplecast.com/54nAGcIl`)
4. Navigate with arrow keys or Up/Down to browse episodes
5. Press `D` to download episodes, `F1` or `?` for help

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
- `Enter` - Play selected episode (when playback implemented)
- `Shift+D` - Download episode
- `Shift+X` - Delete downloaded file for selected episode
- `Ctrl+x` - Delete ALL downloaded episodes and clean up

### Buffer Management
- `F2` - Switch to podcast list
- `F3` - Switch to help
- `F4` - Switch to downloads
- `F5` - Refresh current buffer
- `Ctrl+b` - Show buffer list / Switch buffer
- `Ctrl+k` - Close current buffer
- `Ctrl+l` - List all buffers

### Application
- `F1` - Show help
- `h` or `?` - Show help
- `:` - Command prompt
- `Esc` - Cancel/hide minibuffer
- `q` - Quit application
- `F10` - Quit application

### Future Playback Controls (Not Yet Implemented)
- `Space` - Play/pause
- Audio controls coming in Sprint 4

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
    "concurrent": 3,
    "cleanup_after_days": 30
  },
  "audio": {
    "volume": 0.8,
    "seek_seconds": 30
  },
  "keybindings": {
    "play_pause": "SPC",
    "next_episode": "n",
    "prev_episode": "p"
  },
  "ui": {
    "theme": "default",
    "show_progress_bar": true
  }
}
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
├── playlists/                  # User playlists
│   ├── {playlist-id}.json
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

### MVP (Current Focus)

**Completed (Sprints 0-3):**
- [x] **Sprint 0: Project Setup** - Rust project structure, dependencies, and tooling
- [x] **Sprint 0: Storage Layer** - JSON-based storage with abstraction trait
- [x] **Sprint 0: Data Models** - Podcast, Episode, and configuration models with tests
- [x] **Sprint 1: Core UI Framework** - Complete Emacs-style TUI with buffers and keybindings
- [x] **Sprint 1: Theme System** - Multiple themes with dynamic switching
- [x] **Sprint 1: Buffer Management** - Emacs-style buffer system with switching
- [x] **Sprint 2: RSS Subscription Management** - Subscribe to podcasts via RSS feeds  
- [x] **Sprint 2: OPML Import/Export** - Non-destructive import and export of subscriptions
- [x] **Sprint 2: Episode Parsing** - RSS feed parsing and episode extraction
- [x] **Sprint 3: Download System** - Parallel episode downloads with progress tracking
- [x] **Sprint 3: Episode Management UI** - Browse and manage episodes
- [x] **Sprint 3: File Organization** - Download directory organization and cleanup

**In Progress (Sprints 4-7):**
- [ ] **Sprint 4: Audio Playback** - Basic playback controls with rodio
- [ ] **Sprint 5: Playlist Creation** - Create and manage custom episode playlists
- [ ] **Sprint 5: Episode Notes** - Add personal notes to episodes
- [ ] **Sprint 5: Search & Filtering** - Episode search and filtering
- [ ] **Sprint 6: Statistics Tracking** - Listen time and download statistics
- [ ] **Sprint 7: Cross-platform Testing** - Windows and Linux compatibility verification
- [ ] **Sprint 7: Polish & Documentation** - Final polish and complete documentation

### Post-MVP (v1.1+)
- [ ] SQLite storage backend option
- [ ] Advanced playlist features (smart playlists)
- [ ] Plugin architecture
- [ ] Enhanced statistics and reporting
- [ ] Cloud synchronization (optional)
- [ ] Web interface companion

See [full roadmap](docs/ROADMAP.md) for detailed planning.

## 🐛 Known Issues

**Current Development Status:**
- ⚠️ **Audio playback not yet implemented** - Sprint 4 feature (rodio integration pending)
- ⚠️ **Playlists not yet implemented** - Sprint 5 feature
- ⚠️ **Episode notes not yet implemented** - Sprint 5 feature
- ⚠️ **Statistics tracking not yet implemented** - Sprint 6 feature
- ⚠️ **Search/filtering limited** - Advanced features pending Sprint 5

**Build Requirements:**
- Windows ARM64 builds require LLVM/Clang (see scripts/INSTALL-LLVM.md)
- Windows x64 builds require MSVC Build Tools
- The `ring` dependency (used by reqwest) has specific compiler requirements

**Current Limitations:**
- Download concurrency configurable (default 2-3)
- RSS feed parsing works with most standard feeds
- Terminal compatibility tested on Windows Terminal, GNOME Terminal, and similar
- Some feeds with non-standard audio URL formats may not parse correctly

See [GitHub Issues](https://github.com/yourusername/podcast-tui/issues) for current bugs and feature requests.

## 📜 License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- [Ratatui](https://ratatui.rs/) community for excellent TUI framework
- [feed-rs](https://github.com/feed-rs/feed-rs) for robust RSS parsing
- Terminal UI community for design inspiration
- Rust community for excellent tooling and libraries

---

**Status**: 🚧 Active Development (Sprint 3 Complete - 37.5% of MVP)  
**Completed**: Sprints 0-3 (Foundation, UI, RSS/Podcasts, Downloads)  
**Next Up**: Sprint 4 (Audio Playback)  
**Maintainer**: [@yourusername](https://github.com/yourusername)  
**Version**: 1.0.0-mvp (in development)