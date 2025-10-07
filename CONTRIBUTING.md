# Contributing to Podcast TUI

Thank you for your interest in contributing to Podcast TUI! This document outlines the process for contributing to this project.

## Development Setup

### Prerequisites
- Git
- Docker (for DevContainer)
- VS Code (recommended) or your preferred editor

### Getting Started

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/podcast-tui.git
   cd podcast-tui
   ```

2. **Open in DevContainer**
   - Open the project in VS Code
   - Click "Reopen in Container" when prompted
   - Or use Command Palette: "Remote-Containers: Reopen in Container"

3. **Verify setup**
   ```bash
   cargo --version
   cargo test
   cargo clippy
   ```

## Development Workflow

### Branch Strategy
- `main` - Stable, releasable code
- `develop` - Integration branch for features
- `feature/description` - Feature development branches
- `sprint/X` - Sprint-specific branches for coordinated development

### Commit Messages
Follow conventional commit format:
```
type(scope): description

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Examples:
- `feat(ui): add episode filtering buffer`
- `fix(download): handle interrupted download resume`
- `docs(readme): update installation instructions`

### Code Quality Standards

#### Rust Code Standards
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- No `unwrap()` or `expect()` in user-facing code paths
- Use proper error handling with `Result<T, E>`
- Write doc comments for public APIs
- Follow the project's architecture patterns (see copilot-instructions.md)

#### Testing Requirements
- Unit tests for business logic
- Integration tests for user workflows
- Mock external dependencies in tests
- Maintain test coverage above 80%

#### Performance Guidelines
- Profile performance-critical code
- Avoid blocking the UI thread
- Use async/await for I/O operations
- Implement proper resource cleanup

## Sprint Process

### Sprint Planning
1. Review current sprint board
2. Estimate effort for new tasks (XS/S/M/L/XL)
3. Assign tasks based on priority and dependencies
4. Update sprint milestone

### Daily Development
1. Check sprint board for current tasks
2. Move tasks through workflow: `Todo → In Progress → Review → Done`
3. Update progress in task comments
4. Ask for help if blocked

### Sprint Review
1. Demo completed functionality
2. Update sprint documentation
3. Move incomplete tasks to next sprint
4. Document lessons learned

## Architecture Guidelines

> 📖 **Complete Architecture Documentation**: See [**docs/ARCHITECTURE.md**](docs/ARCHITECTURE.md) for comprehensive technical documentation

### Core Principles (Summary)

**Storage Abstraction**
- Always code against the `Storage` trait, never directly against JSON implementation
- Write storage-agnostic tests using mocks
- Use proper serialization with serde
- Implement atomic writes for data consistency

**UI Components**
- Follow buffer-based UI patterns (buffers, windows, minibuffer)
- Create reusable components that encapsulate rendering and event handling
- Implement proper focus management
- Use responsive layouts that adapt to terminal size

**Error Handling**
- Create custom error types with `thiserror`
- Provide user-friendly error messages
- Always provide context in error chains
- Never use `unwrap()` or `expect()` in user-facing code

**Configuration**
- Use centralized constants module (`src/constants.rs`)
- All default values defined in one place
- JSON for human-readable config files
- Provide sensible defaults for all options

### Code Style

Follow the project's comprehensive code style guidelines in [**.github/copilot-instructions.md**](.github/copilot-instructions.md), which includes:
- Rust style conventions (snake_case, PascalCase)
- Architecture patterns (Storage, Event-Driven, Buffer-Based UI, Async-First)
- Error handling patterns with examples
- Testing strategies
- Common patterns to follow and anti-patterns to avoid

### Module Organization

The project follows a clear module structure documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md):
- `src/storage/` - Data persistence abstraction
- `src/podcast/` - Domain models and RSS logic
- `src/download/` - Download management
- `src/ui/` - Terminal UI with buffers and components
- `src/constants.rs` - Centralized configuration constants
- `src/utils/` - Shared utilities (filesystem, validation, etc.)

## Pull Request Process

### Before Creating PR
1. Ensure all tests pass: `cargo test`
2. Run linting: `cargo clippy -- -D warnings`
3. Format code: `cargo fmt`
4. Update documentation if needed
5. Test cross-platform compatibility (if applicable)

### PR Requirements
- [ ] Descriptive title and description
- [ ] Link to related issues
- [ ] Screenshots/demos for UI changes
- [ ] Tests for new functionality
- [ ] Documentation updates
- [ ] Changelog entry (if user-facing)

### Review Process
1. Automated checks must pass
2. Code review by maintainer
3. Manual testing of changes
4. Cross-platform verification (if needed)
5. Merge to develop branch

## Release Process

### MVP Release Criteria
- All P0 features implemented and tested
- Cross-platform compatibility verified
- Documentation complete
- Performance targets met
- No critical or high-severity bugs

### Version Numbers
- MVP: `1.0.0-mvp`
- Pre-releases: `1.0.0-beta.1`, `1.0.0-rc.1`
- Stable releases: `1.0.0`, `1.1.0`, `2.0.0`

### Release Checklist
- [ ] All tests passing
- [ ] Documentation updated
- [ ] Changelog updated
- [ ] Cross-platform builds tested
- [ ] Performance benchmarked
- [ ] Release notes prepared

## Issue Triage

### Labels
- **Priority**: `P0` (critical), `P1` (high), `P2` (medium), `P3` (low)
- **Type**: `bug`, `enhancement`, `documentation`, `question`
- **Status**: `needs-triage`, `blocked`, `help-wanted`, `good-first-issue`
- **Sprint**: `sprint-0`, `sprint-1`, etc.
- **Component**: `ui`, `audio`, `downloads`, `rss`, `storage`, `performance`

### Triage Process
1. New issues get `needs-triage` label
2. Maintainer reviews and adds appropriate labels
3. Critical bugs get immediate attention
4. Enhancements evaluated against MVP scope
5. Issues assigned to milestones/sprints

## Getting Help

### Documentation

**Essential Reading:**
- [**ARCHITECTURE.md**](docs/ARCHITECTURE.md) - Complete system architecture and design patterns
- [**PRD**](docs/PRD.md) - Product requirements and scope
- [**Implementation Plan**](docs/IMPLEMENTATION_PLAN.md) - Technical roadmap and sprint status
- [**GETTING_STARTED.md**](GETTING_STARTED.md) - Quick start and platform-specific setup

**Technical Documentation:**
- [**Storage Design**](docs/STORAGE_DESIGN.md) - Storage abstraction architecture
- [**OPML Support**](docs/OPML_SUPPORT.md) - OPML import/export implementation
- [**Keybindings**](docs/KEYBINDINGS.md) - Complete keyboard shortcuts reference
- [**Build System**](docs/BUILD_SYSTEM.md) - Cross-platform build instructions

**Code Guidelines:**
- [**.github/copilot-instructions.md**](.github/copilot-instructions.md) - Comprehensive code style and architecture patterns

**Historical Documentation:**
- [**docs/archive/**](docs/archive/) - Historical implementation notes and bug fixes

### Communication
- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and general discussion
- **PR Comments**: Code-specific discussions
- **Project Board**: Current sprint status and planning

### Common Questions

**Q: How do I add a new storage backend?**
A: Implement the `Storage` trait in `src/storage/`. See `json.rs` for reference. Read [docs/STORAGE_DESIGN.md](docs/STORAGE_DESIGN.md) for architecture details.

**Q: How do I add a new UI buffer?**
A: Create a new module in `src/ui/buffers/` following the pattern in existing buffers. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#ui-layer) for buffer component patterns.

**Q: How do I test cross-platform compatibility?**
A: Use the DevContainer for Linux testing. For Windows, test in Windows Terminal and PowerShell. See [docs/BUILD_SYSTEM.md](docs/BUILD_SYSTEM.md) for platform-specific build instructions.

**Q: What's the MVP scope?**
A: See the [PRD](docs/PRD.md) for detailed scope. Focus on core functionality over advanced features. Current progress: 37.5% (3/8 sprints complete).

**Q: Where are magic numbers defined?**
A: All configuration defaults are centralized in `src/constants.rs` organized by category (network, downloads, ui, storage, audio, etc.). Never use hard-coded values.

**Q: What documentation should I read first?**
A: Start with [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for system design, then [.github/copilot-instructions.md](.github/copilot-instructions.md) for code patterns, and [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) for sprint status.

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful and inclusive in all interactions.

---

Thank you for contributing to Podcast TUI! 🎧