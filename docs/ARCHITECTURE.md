# Podcast TUI Architecture

**Last Updated**: October 7, 2025  
**Version**: 1.0.0-MVP  
**Status**: Sprint 3 Complete

## Overview

Podcast TUI is a cross-platform terminal user interface (TUI) application for podcast management written in Rust. The architecture emphasizes:

- **Storage Abstraction**: All data operations go through a trait-based storage layer
- **Event-Driven UI**: User interactions trigger events processed asynchronously
- **Buffer-Based Navigation**: Emacs-inspired buffer system for organizing views
- **Async-First Design**: Network and I/O operations use tokio async runtime
- **Cross-Platform**: Runs on Windows (x64/ARM64) and Linux (x64/ARM64)

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         Terminal UI Layer                        │
│                           (Ratatui)                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐           │
│  │   Buffers   │  │ Components  │  │ Keybindings  │           │
│  │             │  │             │  │              │           │
│  │ • Podcast   │  │ • Lists     │  │ • Emacs-     │           │
│  │   List      │  │ • Details   │  │   style      │           │
│  │ • Episodes  │  │ • Minibuf   │  │ • Universal  │           │
│  │ • Downloads │  │ • Status    │  │   Keys       │           │
│  │ • Help      │  │ • Theme     │  │              │           │
│  └─────────────┘  └─────────────┘  └──────────────┘           │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                      Application Layer                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌────────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   App State    │  │  Event Loop  │  │  Config Mgmt │       │
│  │                │  │              │  │              │       │
│  │ • Buffers      │  │ • Key Events │  │ • JSON       │       │
│  │ • Active View  │  │ • Downloads  │  │ • Defaults   │       │
│  │ • Command      │  │ • Network    │  │ • Validation │       │
│  │   History      │  │ • Storage    │  │              │       │
│  └────────────────┘  └──────────────┘  └──────────────┘       │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                      Business Logic Layer                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  Podcast    │  │   Download   │  │   Feed       │          │
│  │  Manager    │  │   Manager    │  │   Parser     │          │
│  │             │  │              │  │              │          │
│  │ • Subscribe │  │ • Queue      │  │ • RSS 2.0    │          │
│  │ • Refresh   │  │ • Download   │  │ • Atom       │          │
│  │ • OPML      │  │ • Progress   │  │ • Podcast    │          │
│  │ • Episodes  │  │ • Cleanup    │  │   Namespace  │          │
│  └─────────────┘  └──────────────┘  └──────────────┘          │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                      Storage Abstraction                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────┐          │
│  │              Storage Trait (async)               │          │
│  │                                                  │          │
│  │  • save_podcast()    • load_podcast()           │          │
│  │  • list_podcasts()   • delete_podcast()         │          │
│  │  • save_episode()    • load_episode()           │          │
│  │  • list_downloads()  • update_download()        │          │
│  └──────────────────────────────────────────────────┘          │
│                           │                                     │
│                           ▼                                     │
│  ┌──────────────────────────────────────────────────┐          │
│  │         JSON Storage Implementation              │          │
│  │                                                  │          │
│  │  • Atomic writes    • Directory structure       │          │
│  │  • Serde JSON       • Error handling            │          │
│  └──────────────────────────────────────────────────┘          │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                      External Dependencies                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐        │
│  │ Tokio    │  │ Reqwest  │  │ Ratatui  │  │ Feed-rs │        │
│  │ (Async)  │  │ (HTTP)   │  │ (TUI)    │  │ (Parse) │        │
│  └──────────┘  └──────────┘  └──────────┘  └─────────┘        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Core Principles

### 1. Storage Abstraction

**Why**: Enables testing with mocks, easy migration to different storage backends (SQLite, cloud, etc.)

**How**: All storage operations go through the `Storage` trait defined in `src/storage/traits.rs`. The current implementation uses JSON files (`JsonStorage`), but this can be swapped without changing business logic.

**Example**:
```rust
#[async_trait]
pub trait Storage: Send + Sync {
    type Error;
    
    async fn save_podcast(&self, podcast: &Podcast) -> Result<(), Self::Error>;
    async fn load_podcast(&self, id: &str) -> Result<Option<Podcast>, Self::Error>;
    // ... more methods
}
```

**Benefits**:
- ✅ Easy unit testing with mock storage
- ✅ Can switch to SQLite/database later without code changes
- ✅ Clear separation of concerns
- ✅ Atomic operations with temporary files

### 2. Event-Driven UI

**Why**: Keeps UI responsive during long-running operations (downloads, network requests)

**How**: User interactions generate events that are processed asynchronously. The UI renders based on application state, not directly from events.

**Event Flow**:
```
User Input → KeyEvent → App::handle_key() → State Update → UI Render
                                         ↓
                                    Side Effects (async)
                                         ↓
                                    Event Queue → State Update
```

**Benefits**:
- ✅ Non-blocking UI
- ✅ Progress feedback for long operations
- ✅ Clean separation of input handling and rendering
- ✅ Easier testing of state transitions

### 3. Buffer-Based UI

**Why**: Provides Emacs-style navigation familiar to power users, allows multiple views of data

**How**: Each view (podcast list, episode list, downloads, help) is a "buffer". Users switch between buffers with keyboard shortcuts.

**Buffer Types**:
- **Podcast List** (`F2`): Main subscription list
- **Episode List**: Episodes for selected podcast
- **Downloads** (`F4`): Download queue and status
- **Help** (`F1`, `?`): Keybinding reference
- **What's New**: Recently updated episodes

**Benefits**:
- ✅ Familiar workflow for Emacs users
- ✅ Efficient keyboard navigation
- ✅ Multiple views without complex window management
- ✅ Extensible: easy to add new buffer types

### 4. Async-First Design

**Why**: Podcast operations involve network I/O (RSS feeds, downloads) and file I/O (storage)

**How**: Built on tokio async runtime. All I/O operations are `async`, allowing concurrent operations without blocking.

**Async Operations**:
- RSS feed parsing and fetching
- Episode downloads (2-3 concurrent by default)
- Storage operations (read/write JSON files)
- OPML import/export

**Benefits**:
- ✅ Responsive UI during network operations
- ✅ Concurrent downloads
- ✅ Better resource utilization
- ✅ Scalable to many podcasts

## Module Structure

### Core Modules

#### `src/app.rs`
**Purpose**: Application state and coordination

**Key Components**:
- `App`: Main application state (buffers, downloads, podcasts)
- Event loop coordination
- Command execution (`:quit`, `:add-podcast`, etc.)

**Dependencies**: `ui`, `storage`, `podcast`, `download`, `config`

#### `src/ui/`
**Purpose**: Terminal UI rendering and interaction

**Key Files**:
- `app.rs`: UI application wrapper
- `buffers/`: Buffer implementations (podcast list, episodes, downloads, help)
- `components/`: Reusable UI components (lists, status bar, minibuffer)
- `events.rs`: Event types and handling
- `keybindings.rs`: Key mapping and command dispatch
- `themes.rs`: Color schemes

**Dependencies**: `ratatui`, `crossterm`

#### `src/podcast/`
**Purpose**: Podcast and feed management

**Key Files**:
- `models.rs`: `Podcast` and `Episode` data structures
- `feed.rs`: RSS/Atom feed parsing
- `subscription.rs`: Subscription management
- `opml.rs`: OPML import/export

**Dependencies**: `feed-rs`, `reqwest`

#### `src/storage/`
**Purpose**: Data persistence abstraction

**Key Files**:
- `traits.rs`: `Storage` trait definition
- `json.rs`: JSON file-based implementation
- `models.rs`: Storage-specific models

**Dependencies**: `tokio::fs`, `serde`, `serde_json`

#### `src/download/`
**Purpose**: Episode download management

**Key Files**:
- `manager.rs`: Download queue, progress tracking, concurrent downloads
- `mod.rs`: Public API

**Dependencies**: `reqwest`, `tokio::fs`

#### `src/config.rs`
**Purpose**: Application configuration

**Key Components**:
- `Config`: Main configuration structure
- Default values and validation
- JSON serialization/deserialization
- Path expansion (`~/` handling)

#### `src/utils/`
**Purpose**: Shared utilities

**Current Utilities**:
- File path manipulation
- String utilities
- (More to be added in cleanup phase)

### Data Flow

#### Subscribing to a Podcast

```
1. User Input: Press 'a' or ':add-podcast <URL>'
   ↓
2. UI: Parse command, validate URL
   ↓
3. Podcast Manager: Fetch RSS feed (async)
   ↓
4. Feed Parser: Parse XML → Podcast + Episodes
   ↓
5. Storage: Save podcast and episodes (async, atomic writes)
   ↓
6. App State: Update podcast list
   ↓
7. UI: Re-render with new podcast
```

#### Downloading an Episode

```
1. User Input: Press 'Shift+D' on episode
   ↓
2. Download Manager: Add to queue
   ↓
3. HTTP Request: Stream episode file (async)
   ↓
4. Progress Events: Update download status
   ↓
5. File System: Write to downloads directory
   ↓
6. Storage: Update episode metadata (downloaded: true)
   ↓
7. UI: Update download status in real-time
```

#### Importing OPML

```
1. User Input: ':import-opml <path>'
   ↓
2. OPML Parser: Parse XML, extract feed URLs
   ↓
3. For each feed (concurrent):
   a. Fetch RSS feed
   b. Parse feed
   c. Save podcast
   ↓
4. Progress Feedback: Show import status
   ↓
5. App State: Reload podcast list
   ↓
6. UI: Display imported podcasts
```

## Key Design Patterns

### 1. Repository Pattern (Storage Abstraction)

**Where**: `src/storage/traits.rs`, `src/storage/json.rs`

**Pattern**: Abstract data access behind trait, concrete implementations are interchangeable

**Benefits**: Testing, flexibility, separation of concerns

### 2. Builder Pattern

**Where**: Configuration, some UI components

**Pattern**: Fluent API for constructing complex objects

**Example**:
```rust
let config = Config::default()
    .with_downloads(DownloadConfig { concurrent: 5, ... })
    .with_theme("dark");
```

### 3. Command Pattern

**Where**: UI keybindings, minibuffer commands

**Pattern**: Encapsulate actions as objects/enums

**Example**:
```rust
enum Command {
    AddPodcast(String),
    DeletePodcast(String),
    Quit,
}
```

### 4. Observer Pattern (Event System)

**Where**: Download progress, UI updates

**Pattern**: Async channels (mpsc) for event notification

**Example**:
```rust
let (progress_tx, progress_rx) = mpsc::channel(100);
// Download manager sends progress events
// UI receives and displays them
```

### 5. Factory Pattern

**Where**: Buffer creation, storage initialization

**Pattern**: Centralized object creation based on type/config

## Dependencies

### Major Dependencies and Rationale

#### `tokio` (v1.x)
**Purpose**: Async runtime  
**Why**: Industry standard, excellent performance, mature ecosystem  
**Used For**: All async operations (network, file I/O, concurrent tasks)

#### `ratatui` (v0.x)
**Purpose**: Terminal UI framework  
**Why**: Modern, well-maintained, excellent documentation  
**Used For**: All UI rendering, event handling, layouts

#### `crossterm` (v0.x)
**Purpose**: Cross-platform terminal manipulation  
**Why**: Works on Windows and Unix, used by ratatui  
**Used For**: Terminal control, raw mode, key input

#### `reqwest` (v0.x)
**Purpose**: HTTP client  
**Why**: Async, feature-rich, widely used  
**Used For**: RSS feed fetching, episode downloads

#### `feed-rs` (v1.x)
**Purpose**: RSS/Atom parsing  
**Why**: Comprehensive format support, podcast namespace  
**Used For**: Parsing podcast RSS feeds

#### `serde` + `serde_json` (v1.x)
**Purpose**: Serialization  
**Why**: De facto standard in Rust  
**Used For**: Config files, JSON storage, OPML parsing

#### `anyhow` (v1.x)
**Purpose**: Error handling  
**Why**: Convenient error propagation, context chaining  
**Used For**: Application-level error handling

#### `thiserror` (v1.x)
**Purpose**: Custom error types  
**Why**: Ergonomic error type definitions  
**Used For**: Domain-specific errors (StorageError, FeedError, etc.)

#### `quick-xml` (v0.x)
**Purpose**: XML parsing  
**Why**: Fast, low-level control for OPML  
**Used For**: OPML import/export

#### `dirs` (v5.x)
**Purpose**: Platform directories  
**Why**: Cross-platform config/data directories  
**Used For**: Finding config directory (`~/.config/podcast-tui/`)

## Testing Strategy

### Unit Tests

**Where**: Alongside implementation code (`#[cfg(test)]` modules)

**What to Test**:
- Data model transformations
- Configuration parsing and validation
- Path manipulation utilities
- OPML parsing and generation
- Feed parsing (with mock data)

**Example**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.downloads.concurrent, 3);
    }
    
    #[tokio::test]
    async fn test_storage_save_load() {
        let storage = JsonStorage::new_temp();
        let podcast = Podcast::new("Test", "http://test.com/feed");
        
        storage.save_podcast(&podcast).await.unwrap();
        let loaded = storage.load_podcast(&podcast.id).await.unwrap();
        
        assert_eq!(loaded, Some(podcast));
    }
}
```

### Integration Tests

**Where**: `tests/` directory

**What to Test**:
- OPML import with real files
- Subscription workflow (mocked network)
- Download manager (mocked HTTP)
- End-to-end user workflows

**Current Tests**:
- `test_opml_live_url.rs`: Real-world OPML URL handling
- `test_opml_local_file.rs`: Local OPML file import
- `unsubscribe_integration_test.rs`: Unsubscribe workflow

### Mock Strategy

**Storage Mocking**: Use in-memory or temporary directory storage for tests

**Network Mocking**: Use `mockito` or similar for HTTP mocking (planned)

**UI Testing**: Manual testing (TUI testing is challenging to automate)

## Performance Considerations

### Memory Usage

**Target**: < 200MB during normal operation

**Strategies**:
- Lazy loading: Don't load all episodes into memory
- Pagination: Load episodes in chunks if needed
- Streaming downloads: Don't buffer entire files

### Startup Time

**Target**: < 5 seconds

**Current**:
- Config loading: < 100ms
- Storage initialization: < 500ms
- Podcast loading: Depends on count (~10ms per podcast)
- UI initialization: < 100ms

**Optimizations**:
- Parallel podcast loading (tokio tasks)
- Splash screen with progress indicator
- Lazy episode loading

### Network Efficiency

**Strategies**:
- Connection pooling (reqwest default)
- Conditional requests (ETag, Last-Modified headers)
- Rate limiting to respect servers
- Retry with exponential backoff

**Current**:
- 2-3 concurrent downloads (configurable)
- 30s timeout for feed fetches
- 300s timeout for episode downloads

## Security Considerations

### Input Validation

**RSS URLs**: Validate URL format before fetching

**File Paths**: Sanitize filenames, prevent directory traversal

**OPML Import**: XML sanitization to prevent XXE attacks

**Configuration**: Validate all config values, use safe defaults

### Network Security

**HTTPS**: Prefer HTTPS URLs, warn on HTTP

**Certificate Validation**: Enabled by default (reqwest)

**Timeouts**: Prevent hanging on malicious servers

**Rate Limiting**: Prevent accidental DoS

### File System Security

**Atomic Writes**: Use temporary files + rename to prevent corruption

**Permissions**: Use appropriate file permissions (user-only on Unix)

**Path Traversal**: Validate and sanitize all file paths

## Future Architecture Changes

### Planned Improvements

#### 1. Constants Module (Phase 3 of Cleanup)

**Purpose**: Centralize magic numbers and configuration defaults

**Structure**:
```rust
// src/constants.rs
pub mod network {
    pub const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
    pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
}

pub mod filesystem {
    pub const MAX_FILENAME_LENGTH: usize = 255;
}

pub mod downloads {
    pub const DEFAULT_CONCURRENT_DOWNLOADS: usize = 3;
}
```

#### 2. Enhanced Utils Module

**Purpose**: Reduce code duplication, provide reusable utilities

**Planned**:
- `utils/fs.rs`: Path manipulation (`expand_tilde`, safe writes)
- `utils/validation.rs`: Input validation (URLs, paths, ranges)
- `utils/string.rs`: String utilities (sanitization, truncation)

#### 3. Audio Playback (Sprint 4)

**Approach**: Use `rodio` for cross-platform audio

**Architecture Addition**:
```
┌──────────────────┐
│  Audio Manager   │
│                  │
│  • Play/Pause    │
│  • Seek          │
│  • Volume        │
│  • Queue         │
└──────────────────┘
        ↓
   rodio crate
```

**Integration**: New buffer for "Now Playing", event-driven playback control

#### 4. Statistics (Sprint 6)

**Data to Track**:
- Listen history
- Completion rates
- Popular podcasts
- Time listened

**Storage**: Extend `Storage` trait with statistics methods

#### 5. Database Migration (Post-MVP)

**Rationale**: Better performance with large libraries (1000+ podcasts)

**Approach**: 
- Implement `Storage` trait with SQLite backend
- Migration tool from JSON to SQLite
- Keep JSON as option for simplicity

**Benefits**:
- Faster queries (filtering, searching)
- Better concurrency
- Smaller storage footprint
- Full-text search

## Contributing to Architecture

### When to Update This Document

- Adding new modules or major features
- Changing core design patterns
- Adding significant dependencies
- Refactoring storage or UI layers
- Making performance optimizations

### Architecture Decision Records (ADRs)

For major decisions, consider creating an ADR in `docs/adr/`:

```markdown
# ADR-001: Use Ratatui for TUI Framework

## Status: Accepted

## Context
Need a TUI framework for cross-platform terminal UI.

## Decision
Use Ratatui instead of alternatives (tui-rs, cursive, termion).

## Consequences
- Positive: Modern, maintained, excellent docs
- Negative: Smaller ecosystem than cursive
- Neutral: Need to learn ratatui patterns
```

## Questions?

For questions about architecture:
1. Check this document and `STORAGE_DESIGN.md`
2. Review code in `src/` with doc comments
3. See `.github/copilot-instructions.md` for coding patterns
4. Open an issue for architectural discussions

---

**Document Version**: 1.0  
**Covers**: Sprint 0-3 (Foundation through Downloads)  
**Next Review**: After Sprint 4 (Audio Playback)
