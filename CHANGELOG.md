# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - Device Sync + Application Icon + Download Cleanup

### Added

**Download Cleanup — Auto-Cleanup on Startup and Manual Command - February 2026**
- **Auto-cleanup on Startup**: Automatically delete downloaded episodes older than the configured `cleanup_after_days` threshold when the app launches
  - Wires up the previously-dead `cleanup_after_days` config field (default: 30 days)
  - Silent when nothing to clean; shows count when episodes are removed
  - Disabled when `cleanup_after_days` is `null` or `0` in config
- **Manual `:clean-older-than <duration>` Command**: Delete downloads older than a specified duration
  - Supports flexible duration syntax: `12h` (hours), `7d` (days), `2w` (weeks), `1m` (months)
  - Bare numbers default to days (e.g., `30` = 30 days)
  - Confirmation prompt before deletion to prevent accidental data loss
  - Alias `:cleanup` for convenience
  - Tab-completion support for both command names
- **Duration Parser**: New `parse_cleanup_duration()` and `format_cleanup_duration()` utility functions
  - Case-insensitive, validates range (>= 1h, <= 365d)
  - Singular/plural formatted output (e.g., "1 week", "2 months")
  - 11 comprehensive unit tests
- **Download Manager**: New `cleanup_old_downloads()` and `cleanup_old_downloads_hours()` methods
  - Uses file modification time to determine download age (no schema migration needed)
  - Follows the same pattern as `delete_all_downloads()` for consistency
  - Cleans up empty directories after deletion
- **Help Buffer**: Added DOWNLOAD CLEANUP section with command reference
- **Documentation**: Updated `docs/KEYBINDINGS.md` with cleanup command reference

### Fixed
- **Minibuffer prompt race condition**: Commands invoked via `:` command prompt (e.g., `:clean-older-than`, `:delete-all-downloads`) had their confirmation prompts immediately wiped by an unconditional `minibuffer.clear()` — prompts now persist correctly
- **AllEpisodeBuffers refresh error**: `trigger_background_refresh(AllEpisodeBuffers)` was a stub that always sent an error ("Buffer refresh failed: Use individual episode buffer refresh") — now properly iterates open episode buffers and refreshes each one individually

**Device Sync for MP3 Players - November 2025**
- **Metadata-Based Device Sync**: Sync downloaded episodes to external MP3 players or USB devices
  - Compare files using metadata only (filename + file size) for fast, reliable sync
  - Runtime device path override - specify sync target when initiating sync
  - Preserves podcast folder structure on device for easy navigation
  - Dry-run mode for safe preview of sync changes before execution
  - Orphan file deletion - removes episodes on device that are no longer on PC
  - Atomic operations with comprehensive error handling and reporting
  - New Sync buffer with visual history of last 10 sync operations
  - Shows sync status, file counts, timestamps, and success/failure indicators
  - Commands: `sync <path>` for full sync, `sync-dry-run <path>` for preview
  - Buffer aliases: `sync`, `device-sync` for quick navigation
  - Configuration options in `config.json`:
    - `sync_device_path`: Optional default device path
    - `sync_delete_orphans`: Auto-delete orphaned files (default: true)
    - `sync_preserve_structure`: Keep folder hierarchy (default: true)
    - `sync_dry_run`: Default mode for safety (default: false)
  - 7 comprehensive unit tests covering all sync scenarios
  - Built for MP3 player compatibility with existing ID3 metadata features

**Application Icon - November 2025**
- **Custom Application Icon**: Added icon for easy identification in system UI
  - Created SVG icon combining cassette tape and RSS feed symbols
  - Generated PNG versions in multiple sizes (16x16, 32x32, 48x48, 64x64, 128x128, 256x256)
  - Created Windows ICO file with multi-resolution support
  - Implemented Windows icon embedding via `build.rs` using `winres` crate
  - Icon automatically appears in Windows taskbar, Task Manager, and file explorer
  - Created Linux desktop entry file for application launcher integration
  - Added `install-icon-linux.sh` script for Linux icon installation
  - Icons installed to `~/.local/share/icons/hicolor/` following XDG standards
  - Desktop entry installed to `~/.local/share/applications/`
  - Added `regenerate-icons.sh` utility script for rebuilding icons from SVG
  - Updated build scripts to include icon assets in release packages
  - Comprehensive documentation in `assets/README.md`
  - Cross-platform support: Windows (embedded) and Linux (system icons)

### Fixed

**Omny.fm Podcast Downloads - November 2025**
- **Corrupted Download Files**: Fixed episodes from Omny.fm hosted podcasts not downloading correctly
  - **Root Cause**: Servers were returning HTTP 200 OK with HTML error pages instead of audio files
  - Added HTTP status validation in `download_file()` method using `error_for_status()`
  - **Added Content-Type validation** to reject HTML responses with clear error messages
  - Validates `Content-Type` header before downloading (accepts audio/*, video/*, octet-stream)
  - Rejects downloads when Content-Type contains "html" even with 200 OK status
  - Added HTTP status validation in `download_artwork()` method
  - Standardized User-Agent string to match feed parser for consistent server behavior
  - Fixes corruption issues with Desert Oracle, Better Offline, and other Omny.fm podcasts
  - Ensures podcast cover art downloads correctly
  - No impact on existing functionality - all 97 unit tests pass
  - Provides clear error: "Server returned HTML instead of audio file"

**UI Thread Blocking - October 10, 2025**
- **Responsive UI During Background Operations**: Fixed UI thread blocking during podcast refresh and downloads
  - Moved all buffer refresh operations to background tasks using `tokio::spawn`
  - Created `BufferRefreshType` enum to categorize different refresh operations
  - Implemented `BufferDataRefreshed` app event to send pre-loaded data back to UI thread
  - Added `set_downloads()`, `set_episodes()`, and `set_podcasts()` methods for non-blocking buffer updates
  - Refactored podcast list, downloads, What's New, and episode buffer refreshes to use background loading
  - Fixed F5 refresh action to use background refresh system instead of blocking `.await` calls
  - UI now remains fully responsive during podcast refreshes, downloads, and data loading operations
  - Users can scroll, navigate, and switch buffers while background operations are running
  - Eliminates UI freezing and weird character artifacts during intensive operations

**Episode Description Rendering - October 2025**
- **HTML Content in Descriptions**: Fixed rendering of episode descriptions containing HTML/CDATA
  - Added `utils::text` module with HTML stripping functionality
  - Implemented `strip_html()` function to remove HTML tags from RSS feed content
  - Added HTML entity decoding for common entities (&amp;, &lt;, &gt;, etc.)
  - Added smart whitespace cleanup to handle excessive newlines and spaces
  - Applied sanitization to both episode and podcast descriptions in feed parser
  - Resolves issue with feeds like Audioboom that include raw HTML in descriptions
  - Clean text feeds (like Libsyn) remain unchanged
  - Comprehensive test coverage with 10 unit tests

**GitHub Actions Release Build - October 2025**
- **Release Build Workflow**: Fixed failing GitHub Actions release build workflow
  - Fixed Zig installation PATH issue where `zig` binary wasn't accessible after pip install
  - Created symlink from `~/.local/bin/zig` to the ziglang package binary
  - Added missing `aarch64-unknown-linux-gnu` Rust target to installation script
  - Improved PATH handling to ensure zig is available for cargo-zigbuild
  - Updated `scripts/install-build-deps.sh` with proper GitHub Actions PATH integration
- **GitHub Release Creation**: Fixed 403 error when creating GitHub releases
  - Added `permissions: contents: write` to `create-release` job in `.github/workflows/release.yml`
  - GitHub Actions now requires explicit permissions for GITHUB_TOKEN to create releases
  - Resolves "⚠️ GitHub release failed with status: 403" error

### Added

**Code Quality & Documentation Improvements - October 2025**
- **Constants Module**: Centralized configuration defaults in `src/constants.rs`
  - 8 organized categories (network, filesystem, downloads, ui, storage, feed, audio, opml)
  - 240 lines with comprehensive documentation
  - Validated in unit tests
  - Eliminated all magic numbers from codebase
- **Architecture Documentation**: Created comprehensive `docs/ARCHITECTURE.md` (500+ lines)
  - Core architectural principles and design patterns
  - Module structure and dependencies
  - Storage abstraction design
  - UI component patterns
  - Data flow diagrams
- **Testing Documentation**: Created comprehensive `docs/TESTING.md` (450+ lines)
  - Testing philosophy and goals
  - Test categories and organization
  - Component-specific testing strategies
  - Test implementation roadmap
  - Quality guidelines and best practices

### Changed

**Documentation Organization - October 2025**
- Reorganized root directory from 27 files to ~12 essential files
- Archived 19 historical documents to `docs/archive/` structure
- Created `docs/archive/cleanup/` for cleanup progress tracking documents
- Enhanced `CONTRIBUTING.md` with architecture and testing references
- Updated `README.md` with clear documentation hierarchy
- Updated `IMPLEMENTATION_PLAN.md` with accurate sprint status (3/8 complete = 37.5%)
- Added comprehensive cross-references between documentation files

### Removed

**Documentation Cleanup - October 2025**
- Removed 5 redundant documentation files:
  - `BUILD_COMMANDS.md` (consolidated into `docs/BUILD_SYSTEM.md`)
  - `BUILD_SYSTEM_FINAL.md` (consolidated into `docs/BUILD_SYSTEM.md`)
  - `GIT_COMMIT_INFO.md` (outdated)
  - `QUICKSTART.md` (merged into `GETTING_STARTED.md`)
  - `docs/BUILD_SYSTEM_SUMMARY.md` (consolidated into main build docs)

### Fixed

**Code Quality - October 2025**
- Fixed 8 clippy warnings (unused imports, visibility issues, dead code markers)
- Removed all magic numbers by introducing constants module
- Improved code maintainability with single source of truth for defaults
- Enhanced error handling with proper field prefixes for intentional unused values

### Added

**OPML Import/Export Support - ✅ COMPLETE**
- **OPML Import**: Import podcast subscriptions from OPML files or URLs
  - Support for local files and remote URLs
  - Non-destructive import (skips duplicates)
  - Sequential processing with progress callbacks
  - Detailed logging with timestamped import reports
  - Real-world OPML compatibility (handles malformed XML)
  - Robust XML sanitization for unescaped entities
  - Flexible `@text`/`@title` attribute handling
- **OPML Export**: Export current subscriptions to OPML 2.0 format
  - Configurable export directory
  - Timestamped filenames for version tracking
  - Atomic writes for data safety
  - Full OPML 2.0 compliance with both `@text` and `@title` attributes
- **UI Integration**: Seamless OPML workflows
  - Keybindings: `Shift+A` (import), `Shift+E` (export)
  - Commands: `:import-opml`, `:export-opml`
  - Real-time progress feedback
  - Detailed import/export summaries
- **Error Handling**: Comprehensive error reporting
  - Validation errors with clear messages
  - Network error handling with retries
  - Per-feed error tracking during import
  - Detailed log files for troubleshooting

### Fixed
- **OPML Real-World Compatibility**: Fixed parsing of OPML files from popular services
  - Handle OPML files missing required `@text` attribute (fall back to `@title`)
  - Sanitize unescaped ampersands and special characters in XML attributes
  - Support OPML files that violate strict XML/OPML specifications
- **Minibuffer Context Preservation**: Fixed race condition in minibuffer input handling
  - Prompt context now captured before submit() clears state
  - OPML import from URLs no longer misidentified as podcast subscription

**Sprint 3: Downloads & Episode Management (Week 4) - ✅ COMPLETE**
- **Download System**: Full parallel download implementation with progress tracking
  - Concurrent download manager supporting 2-3 parallel downloads (configurable)
  - Progress tracking with byte-level granularity
  - Resume capability for interrupted downloads
  - Automatic cleanup of downloaded episodes (configurable age-based)
  - Bulk delete functionality for podcast downloads
- **Episode Management**: Complete episode browsing and organization
  - Episode list buffer with status indicators (new/downloaded/played)
  - Episode metadata display with comprehensive information
  - Download status integration throughout UI
  - File organization by podcast with sanitized filenames
- **File Management**: Robust file handling
  - Configurable download directory with expansion support
  - Safe file naming with special character handling
  - Atomic file operations for reliability
  - Year-based organization option

**Sprint 2: RSS & Podcast Functionality (Week 3) - ✅ COMPLETE**
- **RSS Feed Parsing**: Full RSS/Atom feed support with feed-rs integration
  - Multi-strategy audio URL extraction (6 different strategies)
  - Comprehensive feed validation and error handling
  - Support for various feed formats and quirks
  - Metadata extraction (title, description, author, artwork)
- **Subscription Management**: Complete podcast subscription system
  - Subscribe/unsubscribe functionality with duplicate prevention
  - Podcast list with sorted display (by last updated)
  - Feed refresh with smart episode detection
  - Hard refresh option to update existing episodes
- **OPML Support**: Import/export podcast subscriptions
  - Non-destructive OPML import with duplicate detection
  - Import from local files or HTTP(S) URLs
  - Sequential processing with real-time progress updates
  - Detailed error logging for failed imports
  - Standard OPML 2.0 compliant export format
  - Configurable export directory with timestamped filenames
  - Batch subscription handling
  - Keyboard shortcuts: `Shift+A` (import), `Shift+E` (export)
  - Commands: `:import-opml` and `:export-opml`
- **Episode Detection**: Intelligent episode management
  - Deterministic episode IDs based on GUID for deduplication
  - Multi-strategy duplicate detection (GUID, URL, title+date)
  - Track number assignment for episodes
  - Episode status tracking (new/downloaded/played)

**Sprint 1: Core UI Framework (Week 2) - ✅ COMPLETE**
- **Core UI Framework**: Complete Emacs-style TUI framework implementation
  - Comprehensive UIAction system with 20+ action types for navigation and control
  - Full event handling system with crossterm integration and async support
  - Sophisticated keybinding system with prefix key support (C-x, C-h, C-c sequences)
  - Emacs-style navigation keys (C-n/C-p/C-f/C-b) with arrow key alternatives
- **Buffer Management System**: True Emacs-style buffer paradigm
  - Buffer trait for extensible content types with proper lifecycle management
  - BufferManager with buffer switching, next/previous navigation
  - Help buffer with scrollable keybinding documentation and custom content support
  - Placeholder podcast list and episode list buffers for upcoming RSS functionality
- **UI Components**: Professional-grade terminal UI components
  - **Minibuffer**: Full input handling, command history, cursor movement, message display
  - **Status Bar**: Real-time buffer name display, key sequence feedback, contextual help hints
  - **Theme System**: 4 complete themes (dark, light, high-contrast, solarized) with consistent styling
- **Main Application Loop**: Robust async application framework
  - Complete UIApp with 60fps event loop, efficient rendering, comprehensive action handling
  - Command execution system supporting M-x Emacs-style commands (quit, help, theme, buffer)
  - Dynamic theme switching, buffer switching, integrated help system
  - Proper async event handling with terminal cleanup and error recovery
- **Sprint 0 Foundation**: Complete Rust project structure and backend systems
  - **Storage Layer**: Comprehensive JSON-based storage with abstraction trait
  - **Data Models**: Rich domain models for Podcast, Episode, and configuration  
  - **Configuration System**: JSON-based configuration with sensible defaults
  - **Utilities**: Helper modules for file system, time formatting, and validation

### Changed
- Updated implementation plan to reflect completed Sprint 0 and Sprint 1 objectives
- Enhanced README with current MVP progress status showing Sprint 1 completion
- Improved git repository hygiene with proper .gitignore for Rust projects

### Fixed  
- Episode `is_played()` logic now correctly respects status vs historical play count
- Storage layer properly handles serialization errors without anyhow dependencies
- Git repository size issues by removing build artifacts and adding comprehensive .gitignore
- All compilation errors in UI framework with proper async/trait implementations
- Theme system now supports "default" theme alias for better backwards compatibility

## Sprint Progress

### Sprint 0: Project Setup (Week 1) - ✅ **COMPLETE**
**Completed Objectives:**
- ✅ Project structure with modern Rust tooling
- ✅ Storage abstraction layer with comprehensive JSON backend
- ✅ Rich data models with full test coverage
- ✅ Configuration management system
- ✅ Application architecture foundation
- ✅ Development environment and CI setup

**Key Achievements:**
- 19 passing unit tests covering core functionality
- Comprehensive error handling following Rust best practices  
- Clean separation of concerns (Storage trait abstraction)
- Atomic file operations for data consistency
- Rich domain models supporting MVP feature requirements
- Proper async/await implementation throughout

### Sprint 1: Core UI Framework (Week 2) - ✅ **COMPLETE**
**Completed Objectives:**
- ✅ Complete Emacs-style TUI framework with ratatui and crossterm
- ✅ Comprehensive event handling system with async support
- ✅ Sophisticated keybinding system with prefix keys (C-x, C-h, C-c)
- ✅ Buffer management system following Emacs paradigms
- ✅ Professional UI components (minibuffer, status bar, themes)
- ✅ Main application loop with 60fps rendering and proper cleanup
- ✅ Command execution system (M-x commands)
- ✅ Multi-theme support with dynamic switching

**Key Achievements:**
- **60 passing unit tests** covering all UI framework components
- Complete Emacs-style navigation (C-n, C-p, C-f, C-b) with alternatives
- Robust async event loop with proper terminal management
- Extensible buffer system ready for RSS/podcast content
- Professional theming system with accessibility considerations
- Comprehensive error handling and recovery throughout UI stack
- Full integration testing of UI workflows and interactions

**Next Up: Sprint 2 - RSS & Podcast Functionality**

## [1.0.0-mvp] - TBD

Target release date: 8 weeks from project start

### Planned Features
- RSS podcast subscription management
- Episode browsing and management
- Parallel episode downloading (2-3 concurrent)
- Basic audio playback with controls
- OPML import/export functionality
- Episode notes and filtering
- Simple playlist management
- Cross-platform support (Windows/Linux)
- Emacs-style keyboard navigation
- JSON-based configuration and data storage
- Basic statistics tracking

---

## Release Planning

### Sprint Milestones

#### Sprint 0: Foundation (Week 1) - ✅ Complete
- [x] Project setup and DevContainer
- [x] Storage abstraction design
- [x] Basic application structure
- [x] Documentation framework

#### Sprint 1: Core UI (Week 2) - ✅ **COMPLETE**
- [x] Emacs-style navigation implementation
- [x] Buffer management system
- [x] Professional UI components (minibuffer, status bar, themes)
- [x] Help system foundation
- [x] Main application loop with event handling
- [x] Command execution system (M-x commands)
- [x] Multi-theme support with dynamic switching

#### Sprint 2: RSS & Podcasts (Week 3) - ✅ **COMPLETE**
- [x] RSS feed parsing integration with feed-rs
- [x] Subscription management (subscribe/unsubscribe/list)
- [x] Podcast listing UI with status display
- [x] OPML import/export functionality
- [x] Feed refresh with smart episode detection
- [x] Duplicate prevention and deduplication

#### Sprint 3: Episodes & Downloads (Week 4) - ✅ **COMPLETE**
- [x] Episode management system with full metadata
- [x] Download queue implementation (concurrent)
- [x] File organization by podcast
- [x] Progress tracking UI integration
- [x] Bulk cleanup and delete functionality
- [x] Episode status tracking throughout UI

#### Sprint 4: Playback (Week 5) - 📋 **NEXT**
- [ ] Audio playback integration
- [ ] Playback controls
- [ ] Chapter navigation
- [ ] External player fallback

#### Sprint 5: Enhanced Features (Week 6) - 📋 Planned
- [ ] Episode notes functionality
- [ ] Filtering and search
- [ ] Playlist management
- [ ] Statistics collection

#### Sprint 6: Statistics & Cleanup (Week 7) - 📋 Planned
- [ ] Statistics display
- [ ] Episode cleanup automation
- [ ] Transcript support
- [ ] Metadata enhancements

#### Sprint 7: Polish & Release (Week 8) - 📋 Planned
- [ ] Cross-platform testing
- [ ] Performance optimization
- [ ] Documentation completion
- [ ] MVP release preparation

---

## Version History Format

Each release will include:
- **Added**: New features
- **Changed**: Changes in existing functionality  
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security improvements

## Breaking Changes Policy

For MVP and 1.x releases:
- Configuration format changes will include migration tools
- Storage format changes will include automatic migration
- Major breaking changes will increment the major version number
- Deprecated features will be supported for at least one minor version

## Release Schedule

- **MVP Release**: End of Week 8
- **Patch Releases**: As needed for critical bugs
- **Minor Releases**: Monthly after MVP for new features
- **Major Releases**: When significant breaking changes are needed