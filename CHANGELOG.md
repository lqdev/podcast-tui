# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - Sprint 1 Complete

### Added
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

#### Sprint 2: RSS & Podcasts (Week 3) - 📋 **NEXT**
- [ ] RSS feed parsing integration
- [ ] Subscription management
- [ ] Podcast listing UI
- [ ] OPML import/export

#### Sprint 3: Episodes & Downloads (Week 4) - 📋 Planned
- [ ] Episode management system
- [ ] Download queue implementation
- [ ] File organization
- [ ] Progress tracking UI

#### Sprint 4: Playback (Week 5) - 📋 Planned
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