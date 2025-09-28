# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - MVP Development

### Added
- Initial project structure and DevContainer setup
- Storage abstraction layer with JSON implementation
- Emacs-style keybinding system foundation
- Basic application architecture and event system
- GitHub issue templates and project management structure
- Comprehensive documentation (PRD, Implementation Plan, Contributing)

### Changed
- N/A (Initial release)

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A

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