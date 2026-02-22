# Implementation Plan - Podcast TUI MVP

## Project Overview
- **Duration**: 8 weeks
- **Approach**: Agile with weekly sprints
- **Focus**: MVP delivery over perfect implementation
- **Testing**: Continuous integration with automated testing

## Technology Stack

### Core Technologies
```toml
[dependencies]
# UI Framework
ratatui = "0.29"
crossterm = "0.29"

# Async Runtime
tokio = { version = "1.0", features = ["full"] }

# HTTP and RSS
reqwest = { version = "0.12", features = ["rustls-tls", "stream", "json"] }
feed-rs = "2.0"

# Audio playback (rodio backend + external player fallback)
rodio = "0.21"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# CLI
clap = { version = "4.0", features = ["derive"] }

# Error Handling
anyhow = "1.0"
thiserror = "2.0"
async-trait = "0.1"

# Identifiers & Time
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
directories = "5.0"

# Media metadata
id3 = "1.9"
image = "0.24"

# Text & XML
quick-xml = "0.31"
regex = "1.10"

[dev-dependencies]
mockall = "0.11"
tokio-test = "0.4"
tempfile = "3.0"
```

### Development Tools
- **IDE**: VS Code with Rust Analyzer
- **Container**: DevContainer for consistent environment
- **Testing**: `cargo test` with integration tests
- **Linting**: `clippy` with strict settings
- **Formatting**: `rustfmt` with project config

## Architecture Overview

```
src/
├── main.rs                    # Application entry point
├── cli.rs                     # Command line interface
├── config/                    # Configuration management
│   ├── mod.rs
│   ├── settings.rs           # JSON config loading/saving
│   └── defaults.rs           # Default configuration values
├── app/                       # Application state and logic
│   ├── mod.rs
│   ├── state.rs              # Global application state
│   ├── events.rs             # Event handling system
│   └── actions.rs            # Application actions/commands
├── ui/                        # User interface components
│   ├── mod.rs
│   ├── app.rs                # Main UI orchestration
│   ├── keybindings.rs        # Emacs-style key handling
│   ├── buffers/              # Emacs-style buffer system
│   │   ├── mod.rs
│   │   ├── podcast_list.rs   # Podcast subscription buffer
│   │   ├── episode_list.rs   # Episode listing buffer
│   │   ├── episode_detail.rs # Episode details buffer
│   │   ├── playlist.rs       # Playlist management buffer
│   │   └── help.rs           # Help system buffer
│   ├── components/           # Reusable UI components
│   │   ├── mod.rs
│   │   ├── list.rs           # Generic list component
│   │   ├── detail.rs         # Detail view component
│   │   ├── progress.rs       # Progress indicators
│   │   ├── minibuffer.rs     # Command input area
│   │   └── statusbar.rs      # Status line component
│   └── themes.rs             # Color schemes and styling
├── podcast/                   # Podcast domain logic
│   ├── mod.rs
│   ├── models.rs             # Data models (Podcast, Episode, etc.)
│   ├── feed.rs               # RSS feed parsing and management
│   ├── subscription.rs       # Subscription management
│   ├── episode.rs            # Episode operations
│   ├── metadata.rs           # Episode metadata handling
│   ├── notes.rs              # Episode notes functionality
│   ├── chapters.rs           # Chapter support
│   └── transcripts.rs        # Transcript handling
├── download/                  # Download management
│   ├── mod.rs
│   ├── manager.rs            # Download queue and orchestration
│   ├── downloader.rs         # Individual download handling
│   ├── progress.rs           # Download progress tracking
│   └── cleanup.rs            # Episode cleanup functionality
├── playback/                  # Audio playback
│   ├── mod.rs
│   ├── player.rs             # Audio playback engine
│   ├── controls.rs           # Playback control logic
│   ├── external.rs           # External player integration
│   └── events.rs             # Playback event handling
├── playlist/                  # Playlist management
│   ├── mod.rs
│   ├── manager.rs            # Playlist CRUD operations
│   └── models.rs             # Playlist data structures
├── storage/                   # Data persistence layer
│   ├── mod.rs
│   ├── traits.rs             # Storage trait definitions
│   ├── json.rs               # JSON file storage implementation
│   ├── migration.rs          # Data format migration
│   └── backup.rs             # Backup and restore functionality
├── stats/                     # Statistics and analytics
│   ├── mod.rs
│   ├── collector.rs          # Statistics collection
│   └── models.rs             # Statistics data models
├── import_export/             # OPML and data portability
│   ├── mod.rs
│   ├── opml.rs               # OPML import/export
│   └── formats.rs            # Support for different formats
└── utils/                     # Utility functions
    ├── mod.rs
    ├── http.rs               # HTTP utilities
    ├── fs.rs                 # File system helpers
    ├── time.rs               # Time/date utilities
    └── validation.rs         # Data validation helpers
```

## Sprint Planning

### Sprint 0: Project Setup (Week 1) - ✅ **COMPLETE**
**Goal**: Establish development environment and project foundation

#### Day 1-2: Environment Setup
- [x] Create GitHub repository with proper structure
- [x] Set up DevContainer configuration
- [x] Initialize Cargo project with dependencies
- [x] Configure CI/CD pipeline (GitHub Actions)
- [x] Set up development tooling (rustfmt, clippy)

#### Day 3-4: Storage Foundation  
- [x] Define storage traits and interfaces
- [x] Implement JSON storage backend
- [x] Create basic data models (Podcast, Episode, Config)
- [x] Write storage layer tests
- [x] Set up configuration system

#### Day 5-7: Basic App Structure
- [x] Create main application entry point
- [x] Implement event system foundation
- [x] Set up basic TUI framework integration
- [x] Create simple key handling system
- [x] Basic app state management

**Deliverables**:
- ✅ Working development environment
- ✅ Storage abstraction with JSON implementation
- ✅ Basic application skeleton that compiles and runs
- ✅ Constants module for configuration defaults

### Sprint 1: Core UI Framework (Week 2) - ✅ **COMPLETE**
**Goal**: Implement buffer-based UI foundation

#### Day 1-3: Buffer System
- [x] Implement buffer management (create, switch, destroy)
- [x] Create basic buffer rendering system
- [x] Implement buffer navigation and focus management
- [x] Basic minibuffer for command input

#### Day 4-5: Keybindings
- [x] Implement core navigation (arrow keys, Page Up/Down, Home/End)
- [x] Buffer switching commands (Ctrl+B, Tab, F-keys)
- [x] Universal keybindings that work across terminals
- [x] Command execution with auto-completion

#### Day 6-7: Core Components
- [x] Status bar implementation
- [x] Help system foundation (F1, ?)
- [x] List component for data display
- [x] Progress indicator components
- [x] Multi-theme support system

**Deliverables**:
- [x] Functional buffer-based navigation
- [x] Buffer system that can display different content
- [x] Complete help system
- [x] Core UI components ready for content
- [x] Theme switching capability

### Sprint 2: RSS and Podcasts (Week 3) - ✅ **COMPLETE**
**Goal**: Implement podcast subscription and RSS handling

#### Day 1-2: RSS Parsing
- [x] Integrate feed-rs for RSS/Atom parsing
- [x] Create feed validation and error handling
- [x] Implement feed metadata extraction
- [x] Episode parsing from RSS feeds

#### Day 3-4: Subscription Management  
- [x] Podcast subscription CRUD operations
- [x] Feed refresh functionality
- [x] Subscription persistence using storage layer
- [x] Advanced duplicate detection (multi-strategy)

#### Day 5-6: Podcast UI
- [x] Podcast list buffer implementation
- [x] Add/delete subscription UI flow
- [x] Feed refresh UI and progress indication
- [x] Podcast detail view

#### Day 7: OPML Foundation
- [x] OPML parsing with opml crate
- [x] Non-destructive OPML import functionality
- [x] OPML export implementation

**Deliverables**:
- [x] Working RSS feed parsing and subscription management
- [x] UI for managing podcast subscriptions  
- [x] Full OPML import/export
- [x] Persistent subscription storage

### Sprint 3: Episodes and Downloads (Week 4) - ✅ **COMPLETE**
**Goal**: Implement episode management and download system

#### Day 1-2: Episode Management
- [x] Episode data models and persistence
- [x] Episode list UI buffer
- [x] Episode detail view with metadata
- [x] Episode status tracking (new/played/downloaded)

#### Day 3-4: Download System
- [x] HTTP download client implementation
- [x] Download queue with concurrent handling
- [x] Concurrent download manager (configurable 2-3 parallel)
- [x] Download progress tracking and UI

#### Day 5-6: File Management
- [x] Download directory organization by podcast
- [x] File naming with sanitization
- [x] Download status integration with episode list
- [x] Advanced cleanup functionality (age-based, bulk delete)

#### Day 7: Integration
- [x] Episodes to downloads UI integration
- [x] Error handling for network failures
- [x] Resume interrupted downloads capability
- [x] Download manager UI integration

**Deliverables**:
- [x] Complete episode browsing interface
- [x] Working download system with progress tracking
- [x] File organization and management
- [x] Full integration between subscriptions, episodes, and downloads

### Post-Sprint 3: Shipped Features (out-of-band sprints)

The following were shipped between the original sprint plan and the current state:

#### ✅ Application Icon
- Custom SVG/PNG/ICO icon (cassette + RSS symbol)
- Embedded in Windows exe via `build.rs` + `winres`
- Linux desktop entry + `install-icon-linux.sh`

#### ✅ Device Sync (v1.4.0-mvp)
- `DownloadManager` sync methods — metadata-based comparison (filename + size)
- Dry-run mode, orphan deletion, structure preservation
- New `Sync` buffer with operation history
- `:sync [path]`, `:sync-dry-run [path]` commands
- 7 unit tests

#### ✅ Download Cleanup (v1.5.0-mvp)
- Auto-cleanup on startup (`cleanup_after_days` config)
- `:clean-older-than <duration>` command (h/d/w/m suffixes)
- `parse_cleanup_duration()` + `format_cleanup_duration()` utilities
- 11 unit tests

#### ✅ Search & Filter (v1.6.0)
- `src/ui/filters.rs`: `EpisodeFilter`, `EpisodeStatus`, `DateRange`
- `/` text search, `:filter-status`, `:filter-date`, `:clear-filters`
- AND-combined filter logic
- Duration filter deferred (Design Decision #13 in `docs/rfcs/RFC-001-search-and-filter.md`)

#### ✅ Playlist Management (post-v1.6.0 / Unreleased)
- `src/playlist/` module (5 files)
- User playlists + `Today` auto-playlist (rolling 24h, configurable refresh policy)
- 4 new UI buffers: playlist_list, playlist_detail, playlist_picker, sync
- `p` add to playlist, `c` create, `F7` open, `:playlist-*` commands
- Audio file copying for device compatibility

### Current / Upcoming Work

#### ✅ Audio Playback
- [x] Wire up `rodio` for playback (rodio backend + external player fallback)
- [x] Playback controls (play/pause/stop/seek)
- [x] Volume control
- [x] Progress display for currently playing episode (NowPlaying buffer, F9)
- [ ] Resume from last position

#### ⏳ Episode Notes
- [ ] Note data models and storage
- [ ] Note editing UI in episode detail
- [ ] Note display

#### ⏳ Statistics Tracking
- [ ] Listening time tracking
- [ ] Play count statistics
- [ ] Statistics display buffer

## Testing Strategy

### Unit Tests
- Storage layer operations
- RSS parsing and validation  
- Data model serialization
- Business logic functions
- Utility functions

### Integration Tests
- End-to-end subscription workflows
- Download and playback workflows
- OPML import/export
- Cross-component interactions
- Error handling scenarios

### Manual Testing
- Cross-platform compatibility
- Terminal emulator compatibility
- User workflow validation
- Performance benchmarking
- Accessibility testing

## Risk Mitigation

### Technical Risks
1. **Audio playback issues**: Early prototype, fallback to external players
2. **Cross-platform bugs**: Continuous cross-platform testing
3. **Performance problems**: Regular profiling and optimization
4. **RSS parsing edge cases**: Comprehensive test feed collection

### Project Risks  
1. **Scope creep**: Strict MVP focus, feature parking lot
2. **Timeline pressure**: Aggressive prioritization, feature dropping
3. **Technical debt**: Regular refactoring, code quality focus
4. **Single developer**: Clear documentation, modular design

## Definition of Done

### Feature Complete
- [ ] Functionality implemented according to requirements
- [ ] Unit tests written and passing
- [ ] Integration tests cover main workflows
- [ ] Cross-platform compatibility verified
- [ ] Documentation updated

### Code Quality
- [ ] Passes all lints (clippy) with no warnings
- [ ] Formatted with rustfmt
- [ ] No unwrap() or expect() in user-facing code
- [ ] Proper error handling and user feedback
- [ ] Code reviewed (self-review for solo project)

### User Experience
- [ ] Emacs keybindings work as expected
- [ ] Help system covers new functionality
- [ ] Error messages are clear and actionable
- [ ] Performance meets MVP targets
- [ ] Graceful degradation on limited terminals

---

---

## Progress Summary

### Completed Sprints (3/8 = 37.5%)
- ✅ **Sprint 0**: Project Setup - Foundation, storage, constants
- ✅ **Sprint 1**: Core UI Framework - Buffers, themes, navigation
- ✅ **Sprint 2**: RSS and Podcasts - Subscriptions, OPML, feeds
- ✅ **Sprint 3**: Episodes and Downloads - Download manager, file organization

### Completed
- ✅ **Sprint 4**: Audio Playback

### Upcoming Sprints
- ⏳ **Sprint 5**: Enhanced Features (Playlists, notes, search)
- ⏳ **Sprint 6**: Statistics and Cleanup
- ⏳ **Sprint 7**: Polish and Cross-Platform

---

**Document Version**: 1.1
**Last Updated**: 2025-10-07  
**Sprint Reviews**: Weekly on Fridays
**Retrospectives**: End of each sprint
**Daily Standups**: Personal daily check-ins with progress tracking