# Podcast TUI

A cross-platform terminal user interface for podcast management built with Rust and Emacs-style keybindings.

![Build Status](https://github.com/yourusername/podcast-tui/workflows/CI/badge.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.75+-red.svg)

## 🎧 Features

### MVP Release (v1.0.0-mvp)
- ✅ **RSS Subscription Management** - Subscribe to podcasts via RSS feeds
- ✅ **OPML Import/Export** - Non-destructive import and export of subscriptions  
- ✅ **Episode Management** - Browse, search, and filter episodes
- ✅ **Download System** - Parallel episode downloads with progress tracking
- ✅ **Audio Playback** - Basic playback controls with chapter support
- ✅ **Playlist Creation** - Create and manage custom episode playlists
- ✅ **Episode Notes** - Add personal notes to episodes
- ✅ **Statistics Tracking** - Listen time and download statistics
- ✅ **Emacs-style Navigation** - Familiar keybindings for Emacs users
- ✅ **Cross-platform** - Windows and Linux support

## 🚀 Quick Start

### Prerequisites
- Rust 1.75 or later
- Git

### Installation

#### From Source
```bash
git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui
cargo build --release
./target/release/podcast-tui
```

#### Using DevContainer (Recommended for Development)
1. Install [Docker](https://docker.com) and [VS Code](https://code.visualstudio.com)
2. Install the [Remote-Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
3. Clone the repository and open in VS Code
4. Click "Reopen in Container" when prompted
5. Run `cargo run` to start the application

### First Run
1. Start the application: `podcast-tui`
2. Press `a` to add your first podcast subscription
3. Enter an RSS feed URL (try: `https://feeds.simplecast.com/54nAGcIl`)
4. Press `r` to refresh the feed and load episodes
5. Navigate with `C-n`/`C-p`, press `RET` to play an episode

## 🎹 Keybindings

Podcast TUI uses Emacs-style keybindings for efficient keyboard navigation:

### Navigation
- `C-n` / `C-p` - Next/previous item
- `C-f` / `C-b` - Move right/left  
- `C-a` / `C-e` - Beginning/end of line
- `RET` - Select/activate item

### Buffer Management
- `C-x b` - Switch between buffers (podcasts, episodes, playlists)
- `C-x 1` - Focus current buffer
- `C-x 2` - Split horizontally
- `C-x 3` - Split vertically
- `C-x o` - Switch window

### Podcast Management
- `a` - Add new podcast subscription
- `d` - Delete selected podcast
- `r` - Refresh selected podcast feed
- `R` - Refresh all podcast feeds

### Episode Management  
- `RET` - Play selected episode
- `D` - Download episode
- `X` - Delete downloaded file
- `N` - Add/edit episode note
- `m` - Mark as played/unplayed

### Playback Controls
- `SPC` - Play/pause
- `s` - Stop playback
- `f` / `b` - Seek forward/backward (30s)
- `+` / `-` - Volume up/down
- `n` / `p` - Next/previous episode

### Help System
- `C-h ?` - Help overview
- `C-h k` - Describe key
- `C-h b` - Show all keybindings

See [complete keybinding reference](docs/EMACS_KEYBINDINGS.md) for all shortcuts.

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

- **Storage Layer** - Abstracted JSON-based persistence
- **Domain Logic** - Podcast, episode, and playlist management
- **UI Layer** - Emacs-style terminal interface using Ratatui
- **Audio System** - Cross-platform playback with Rodio

See [architecture documentation](docs/ARCHITECTURE.md) for details.

### Contributing
We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Development setup instructions
- Code style guidelines  
- Sprint process and project management
- Pull request requirements

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
- [x] Basic subscription and episode management
- [x] Download system with progress tracking
- [x] Audio playback with basic controls
- [x] OPML import/export
- [x] Episode notes and filtering
- [ ] Cross-platform testing and optimization
- [ ] Documentation and release preparation

### Post-MVP (v1.1+)
- [ ] SQLite storage backend option
- [ ] Advanced playlist features (smart playlists)
- [ ] Plugin architecture
- [ ] Enhanced statistics and reporting
- [ ] Cloud synchronization (optional)
- [ ] Web interface companion

See [full roadmap](docs/ROADMAP.md) for detailed planning.

## 🐛 Known Issues

Current limitations in the MVP:
- Limited to 2-3 concurrent downloads
- Basic audio format support (MP3, M4A)
- Windows audio system may require additional setup
- Terminal compatibility varies across emulators

See [GitHub Issues](https://github.com/yourusername/podcast-tui/issues) for current bugs and feature requests.

## 📜 License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- [Ratatui](https://ratatui.rs/) community for excellent TUI framework
- [feed-rs](https://github.com/feed-rs/feed-rs) for robust RSS parsing
- Emacs community for keybinding inspiration
- Rust community for excellent tooling and libraries

---

**Status**: 🚧 Active Development (MVP Phase)  
**Maintainer**: [@yourusername](https://github.com/yourusername)  
**Version**: 1.0.0-mvp (in development)