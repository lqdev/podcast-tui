# Podcast TUI - Product Requirements Document

## Project Information
- **Project Name**: Podcast TUI
- **Version**: 1.6.0
- **Created**: 2025-09-28
- **Last Updated**: 2026-02-18
- **Status**: Active Development
- **Team**: Solo Development

## Executive Summary

A cross-platform terminal user interface (TUI) application for podcast management built in Rust. The application provides subscription management, episode downloading, basic playback, and playlist creation through an Emacs-style keyboard interface, prioritizing MVP delivery.

## Problem Statement

Current podcast management solutions are either:
- Heavy GUI applications that consume significant resources
- Limited command-line tools without interactive features
- Web-based solutions requiring internet connectivity
- Missing integration between subscription management and local file organization

## Target Users

**Primary**: Developers and power users who:
- Prefer keyboard-driven interfaces
- Work primarily in terminal environments
- Want offline podcast management
- Need efficient podcast organization and playback

**Secondary**: Podcast enthusiasts who:
- Manage large podcast libraries
- Want fine-grained control over downloads and metadata
- Prefer lightweight, fast applications

## Goals and Success Criteria

### Primary Goals
1. **MVP Delivery**: Working application within 8 weeks
2. **Cross-Platform**: Runs reliably on Windows and Linux
3. **Emacs-Style UX**: Familiar keybindings for Emacs users
4. **Offline-First**: Full functionality without internet (after initial sync)

### Success Metrics
- ✅ Manage 100+ podcast subscriptions
- ✅ Download 2-3 episodes concurrently
- ✅ < 5 second application startup
- ✅ < 200MB memory usage during normal operation
- ✅ Basic playback functionality working
- ✅ OPML import/export compatibility

## Core Features (MVP Scope)

### Must Have (P0)
- [x] RSS feed subscription management ✅ **COMPLETE** (Sprint 2)
- [x] Episode listing and metadata display ✅ **COMPLETE** (Sprint 3)
- [x] Basic episode downloading (2-3 concurrent) ✅ **COMPLETE** (Sprint 3)
- [x] OPML import/export (non-destructive) ✅ **COMPLETE** (Sprint 2)
- [x] Emacs-style keyboard navigation ✅ **COMPLETE** (Sprint 1)
- [x] JSON-based configuration and data storage ✅ **COMPLETE** (Sprint 0)
- [x] Cross-platform compatibility (Windows/Linux) ✅ **COMPLETE** (Sprint 0-3, build scripts)

### Should Have (P1)
- [ ] Basic audio playback (play/pause/stop/seek) ⏳ **PENDING**
- [ ] Episode notes functionality ⏳ **PENDING**
- [x] Simple playlist creation and management ✅ **COMPLETE** (user playlists + Today auto-playlist)
- [x] Episode filtering (status, date, text search) ✅ **COMPLETE** (duration deferred pending RSS data)
- [ ] Chapter support and navigation ⏳ **PENDING**
- [ ] Basic statistics tracking ⏳ **PENDING**
- [x] Episode cleanup (manual and automatic) ✅ **COMPLETE**
- [x] Device sync to MP3 players/USB drives ✅ **COMPLETE**

### Could Have (P2)
- [ ] Transcript display (when available) ⏳ **PENDING**
- [x] Basic metadata management (ID3 tags) ✅ **COMPLETE** (embed_id3_metadata, artwork, track numbers)
- [ ] External player integration ⏳ **PENDING**
- [x] Episode artwork embedding ✅ **COMPLETE**

### Won't Have (This Version)
- Advanced smart playlists
- Cloud synchronization
- Advanced statistics and analytics
- Plugin system
- Advanced audio processing
- Multi-user support
- Web interface

## Technical Requirements

### Architecture
- **Language**: Rust 2021 edition
- **TUI Framework**: Ratatui + crossterm
- **Audio**: rodio for playback
- **Storage**: JSON files with trait abstraction
- **HTTP**: reqwest with connection pooling
- **RSS**: feed-rs parser
- **Config**: serde_json

### Performance
- **Startup**: < 5 seconds (MVP target)
- **Memory**: < 200MB normal operation
- **Storage**: Local JSON files in organized structure
- **Concurrency**: 2-3 simultaneous downloads
- **Responsiveness**: Non-blocking UI during I/O operations

### Cross-Platform
- **Primary**: Windows 10+, Ubuntu 20.04+
- **Terminal**: Windows Terminal, GNOME Terminal, other major emulators
- **Audio**: Cross-platform audio libraries with fallback options
- **Files**: Platform-appropriate file paths and permissions

## User Experience Requirements

### Navigation Model
- **Emacs-style**: C-n/C-p for navigation, C-x for commands
- **Buffers**: Switch between podcasts, episodes, playlists views
- **Minibuffer**: Command input area for text commands
- **Help**: C-h help system with keybinding discovery

### Interface Design
- **Responsive**: Adapts to terminal size gracefully
- **Information Dense**: Efficient use of screen space
- **Clear Focus**: Always visible focus indicators
- **Status Communication**: Clear progress and status messages

### Accessibility
- **Keyboard-Only**: Full functionality without mouse
- **Screen Reader**: Compatible with terminal screen readers
- **Color**: Graceful degradation for limited color terminals
- **Font**: Works with standard terminal fonts

## Data Management

### Storage Design
```
data/
├── config.json                 # Application configuration
├── podcasts/                   # Podcast definitions
│   ├── {podcast-id}.json
├── episodes/                   # Episode metadata and notes
│   ├── {podcast-id}/
│   │   ├── {episode-id}.json
├── playlists/                  # User-created playlists
│   ├── {playlist-id}.json
├── stats.json                  # Usage statistics
└── downloads/                  # Downloaded audio files
    ├── {podcast-name}/
    │   ├── {episode-name}.mp3
```

### Data Models
- **Podcast**: RSS URL, metadata, last refresh, settings overrides
- **Episode**: Metadata, download status, play progress, user notes
- **Playlist**: Name, episode list, play order, creation date
- **Statistics**: Play counts, listening time, download stats

## Risk Assessment

### Technical Risks
- **Audio Compatibility**: Different audio formats/codecs across platforms
  - *Mitigation*: Use proven cross-platform libraries, provide external player fallback
- **Terminal Compatibility**: Varying terminal emulator capabilities
  - *Mitigation*: Test on major emulators, graceful feature degradation
- **Performance**: Large podcast libraries causing slowdowns
  - *Mitigation*: Lazy loading, efficient data structures, performance monitoring

### Project Risks
- **Scope Creep**: Adding non-MVP features during development
  - *Mitigation*: Strict MVP focus, feature parking lot for future versions
- **Cross-Platform Issues**: Windows/Linux differences causing problems
  - *Mitigation*: Early cross-platform testing, use of proven libraries
- **Time Constraints**: 8-week timeline being too aggressive
  - *Mitigation*: Aggressive feature prioritization, early prototype validation

## Dependencies

### External Libraries
- `ratatui` - TUI framework
- `crossterm` - Cross-platform terminal handling  
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `feed-rs` - RSS/Atom parsing
- `rodio` - Audio playback
- `serde` + `serde_json` - Serialization
- `clap` - Command line argument parsing
- `anyhow` - Error handling
- `thiserror` - Custom error types

### Development Dependencies
- `cargo-watch` - Development file watching
- `cargo-audit` - Security auditing
- `mockall` - Test mocking

## Non-Functional Requirements

### Reliability
- Graceful handling of network failures
- Data corruption prevention through atomic writes
- Recovery from partial downloads
- Crash resistance with proper error handling

### Usability
- Intuitive keybindings following Emacs conventions
- Clear error messages and recovery suggestions
- Comprehensive help system
- Responsive interface during long operations

### Maintainability
- Modular architecture with clear separation of concerns
- Comprehensive test coverage for business logic
- Clear documentation for setup and contribution
- Consistent code style and formatting

## Future Considerations

### Version 1.1 Candidates
- SQLite storage backend
- Advanced playlist features
- Plugin architecture foundation
- Enhanced statistics and reporting

### Version 2.0 Vision
- Optional cloud synchronization
- Web interface companion
- Advanced audio processing
- Multi-user support

---

**Document Version**: 1.0
**Last Updated**: 2025-09-28
**Next Review**: Weekly during development
**Approver**: Project Lead