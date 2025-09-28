# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - Sprint 0 Complete

### Added
- **Project Foundation**: Complete Rust project structure with Cargo.toml and all dependencies
- **Storage Layer**: Comprehensive JSON-based storage implementation with abstraction trait
  - Podcast and Episode CRUD operations  
  - Atomic file operations for data integrity
  - Proper error handling and comprehensive test coverage
- **Data Models**: Rich domain models for Podcast, Episode, and configuration
  - Episode status tracking (New, Downloading, Downloaded, Played)
  - Chapter support and metadata handling
  - User notes and playback position tracking
- **Configuration System**: JSON-based configuration with sensible defaults
  - Audio, download, keybinding, storage, and UI settings
  - Automatic config file creation and validation
- **Utilities**: Helper modules for file system, time formatting, and validation
- **Testing**: 19 unit tests covering storage operations, models, and utilities
- **Development Environment**: DevContainer with all necessary tools and dependencies

### Changed
- Updated implementation plan to reflect completed Sprint 0 objectives
- Enhanced README with current MVP progress status

### Fixed
- Episode `is_played()` logic now correctly respects status vs historical play count
- Storage layer properly handles serialization errors without anyhow dependencies
- Documentation formatting for compatibility with doctests

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

**Next Up: Sprint 1 - Core UI Framework**

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

#### Sprint 1: Core UI (Week 2) - 🚧 In Progress
- [ ] Emacs-style navigation implementation
- [ ] Buffer management system
- [ ] Basic UI components
- [ ] Help system foundation

#### Sprint 2: RSS & Podcasts (Week 3) - 📋 Planned
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