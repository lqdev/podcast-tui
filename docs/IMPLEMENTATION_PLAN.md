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
ratatui = "0.24"
crossterm = "0.27"

# Async Runtime
tokio = { version = "1.0", features = ["full"] }

# HTTP and RSS
reqwest = { version = "0.11", features = ["json", "stream"] }
feed-rs = "1.3"

# Audio
rodio = "0.17"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# CLI
clap = { version = "4.0", features = ["derive"] }

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

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

### Sprint 4: Playback System (Week 5)  
**Goal**: Implement basic audio playback functionality

#### Day 1-2: Audio Backend
- [ ] Integrate rodio for cross-platform audio
- [ ] Basic playback controls (play/pause/stop)
- [ ] Volume control implementation
- [ ] Audio file format support

#### Day 3-4: Playback UI
- [ ] Playback status display
- [ ] Progress bar for currently playing episode
- [ ] Playback controls in episode view
- [ ] Currently playing indicator

#### Day 5-6: Advanced Controls
- [ ] Seek functionality (forward/backward 30s)
- [ ] Chapter navigation support
- [ ] Playback queue/next episode functionality
- [ ] External player integration option

#### Day 7: Polish
- [ ] Playback error handling
- [ ] Resume playback from last position
- [ ] Keyboard shortcuts for playback control
- [ ] Integration with episode status tracking

**Deliverables**:
- [ ] Working audio playback system
- [ ] UI integration for playback controls
- [ ] Chapter support where available
- [ ] External player fallback option

### Sprint 5: Enhanced Features (Week 6)
**Goal**: Implement notes, filtering, and playlist functionality

#### Day 1-2: Episode Notes
- [ ] Note data models and storage
- [ ] Note editing UI (simple text input)
- [ ] Note display in episode details
- [ ] Note search functionality

#### Day 3-4: Filtering and Search
- [ ] Episode filtering by status (downloaded/played)
- [ ] Date range filtering
- [ ] Duration-based filtering  
- [ ] Basic text search across episodes

#### Day 5-6: Playlist Management
- [ ] Playlist data models and storage
- [ ] Create/delete playlist functionality
- [ ] Add/remove episodes from playlists
- [ ] Playlist UI buffer

#### Day 7: Integration
- [ ] Playlist playback functionality
- [ ] Filter integration with episode lists
- [ ] Search result display and navigation
- [ ] Playlist management shortcuts

**Deliverables**:
- [ ] Episode notes functionality
- [ ] Comprehensive filtering system
- [ ] Basic playlist creation and management
- [ ] Search capabilities across content

### Sprint 6: Statistics and Cleanup (Week 7)
**Goal**: Implement statistics tracking and episode cleanup

#### Day 1-2: Statistics Collection
- [ ] Listening time tracking
- [ ] Play count statistics
- [ ] Download statistics
- [ ] Statistics data models and storage

#### Day 3-4: Statistics UI
- [ ] Statistics display buffer
- [ ] Most played podcasts/episodes
- [ ] Storage usage information
- [ ] Listening habits insights

#### Day 5-6: Episode Cleanup
- [ ] Automatic cleanup based on age/status
- [ ] Manual episode deletion
- [ ] Cleanup configuration options
- [ ] Storage space management

#### Day 7: Advanced Features
- [ ] Transcript display (when available)
- [ ] Chapter information display
- [ ] Metadata viewing and basic editing
- [ ] Export functionality improvements

**Deliverables**:
- [ ] Statistics tracking and display
- [ ] Episode cleanup functionality  
- [ ] Transcript support
- [ ] Enhanced metadata handling

### Sprint 7: Polish and Cross-Platform (Week 8)
**Goal**: Final polish, testing, and cross-platform validation

#### Day 1-2: Cross-Platform Testing
- [ ] Windows compatibility testing and fixes
- [ ] Linux distribution testing
- [ ] Terminal emulator compatibility
- [ ] Audio system testing across platforms

#### Day 3-4: Performance Optimization
- [ ] Startup time optimization
- [ ] Memory usage profiling and optimization
- [ ] UI responsiveness improvements
- [ ] Large library performance testing

#### Day 5-6: Documentation and UX
- [ ] Complete help system
- [ ] User documentation
- [ ] Keyboard shortcut reference
- [ ] Installation and setup guide

#### Day 7: Release Preparation
- [ ] Final bug fixes and testing
- [ ] Release build optimization
- [ ] Package preparation
- [ ] MVP feature completeness verification

**Deliverables**:
- [ ] Fully cross-platform compatible application
- [ ] Complete documentation
- [ ] Performance-optimized build
- [ ] MVP ready for release

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

### Current Sprint
- 🚧 **Sprint 4**: Audio Playback (Next Up)

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