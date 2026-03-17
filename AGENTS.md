# AGENTS.md - Development Guide for Code Assistants

> **Purpose**: This file provides code assistants (AI agents, IDEs, and developers) with accurate, repo-specific setup instructions, code standards, and development workflows for the Podcast TUI project.

---

## 🚀 Quick Start

### Prerequisites

- **Rust**: 1.75 or later
- **Git**: For version control
- **ALSA libraries** (Linux): `libasound2-dev` for audio support
- **Build tools**: Standard C/C++ build tools (gcc/clang on Linux, MSVC on Windows)

### Installation & Setup

```bash
# Clone the repository
git clone https://github.com/lqdev/podcast-tui.git
cd podcast-tui

# Install dependencies (handled by cargo)
# Cargo will automatically download and build all Rust dependencies

# Build the project
cargo build

# Run tests to verify setup
cargo test

# Run the application in development mode
cargo run

# Build optimized release binary
cargo build --release
```

### First-Time Setup Notes

**Linux:**
```bash
# Install ALSA development libraries (required for audio)
sudo apt-get install libasound2-dev pkg-config

# Build the project
cargo build
```

**Windows:**
- Requires MSVC Build Tools (see [scripts/INSTALL-MSVC-TOOLS.md](scripts/INSTALL-MSVC-TOOLS.md))
- For ARM64: Requires LLVM/Clang (see [scripts/INSTALL-LLVM.md](scripts/INSTALL-LLVM.md))

**NixOS:**
```bash
# Enter the dev shell (all dependencies pre-installed)
nix develop

# Or just build and run directly
nix build && ./result/bin/podcast-tui
```
- `Cargo.lock` is tracked in git — always commit changes to it
- `flake.lock` must be regenerated on a Nix-capable system when `flake.nix` inputs change (`nix flake lock`)
- See [docs/NIX_PACKAGING.md](docs/NIX_PACKAGING.md) for full details

**DevContainer (Recommended):**
- Install [Docker](https://docker.com) and [VS Code](https://code.visualstudio.com)
- Install [Remote-Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
- Open in VS Code and select "Reopen in Container"
- All dependencies pre-installed, ready to develop

---

## 🔧 Development Commands

### Building

```bash
# Development build (fast compilation, no optimizations)
cargo build

# Release build (optimized, slower to compile)
cargo build --release

# Check compilation without building binary (fast)
cargo check

# Build documentation
cargo doc --no-deps --open
```

### Testing

```bash
# Run all tests (unit + integration)
cargo test

# Run only unit tests
cargo test --lib

# Run specific test
cargo test test_name

# Run tests with output (show println! messages)
cargo test -- --nocapture

# Run tests in parallel (default)
cargo test

# Run tests with specific number of threads
cargo test -- --test-threads=1
```

### Code Quality

```bash
# Format code (required before committing)
cargo fmt

# Check formatting without modifying files
cargo fmt --check

# Run linter (required before committing)
cargo clippy

# Run clippy with warnings as errors
cargo clippy -- -D warnings

# Run all quality checks (recommended before PR)
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

### Running the Application

```bash
# Run in development mode
cargo run

# Run with release optimizations
cargo run --release

# Run with specific log level
RUST_LOG=debug cargo run

# Run with custom arguments (if applicable)
cargo run -- --help
```

### Cross-Platform Builds

**Linux:**
```bash
# Install build dependencies (one-time)
./scripts/install-build-deps.sh

# Build all Linux targets
./scripts/build-linux.sh
```

**Windows:**
```powershell
# Verify dependencies (one-time)
.\scripts\install-build-deps.ps1

# Build all Windows targets
.\scripts\build-windows.ps1
```

See [docs/BUILD_SYSTEM.md](docs/BUILD_SYSTEM.md) for detailed cross-platform build instructions.

---

## 📋 Code Standards

### Language & Framework

- **Language**: Rust 2021 Edition
- **Package Manager**: Cargo (standard Rust toolchain)
- **Async Runtime**: Tokio
- **TUI Framework**: Ratatui + Crossterm
- **Audio**: Rodio
- **HTTP**: Reqwest
- **Serialization**: Serde (JSON format)

### Code Style

Follow standard Rust conventions:

```rust
// Use snake_case for functions, variables, modules
fn process_episode() { }
let episode_count = 10;

// Use PascalCase for types and structs
struct EpisodeList { }
enum BufferType { }

// Prefer explicit error handling with Result<T, E>
fn load_podcast(id: &str) -> Result<Podcast, PodcastError> {
    // Never use unwrap() or expect() in production code
    let data = load_from_storage(id)?;
    parse_podcast(&data)
}

// Use async/await for I/O operations
async fn download_episode(url: &str) -> Result<(), DownloadError> {
    let response = http_client.get(url).await?;
    // ...
}
```

### Error Handling Patterns

```rust
// Create custom error types with thiserror
#[derive(Debug, thiserror::Error)]
pub enum PodcastError {
    #[error("Feed parsing failed: {0}")]
    FeedParsing(String),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

// Always provide context in error chains
use anyhow::Context;

let podcast = storage.load_podcast(&id)
    .await
    .with_context(|| format!("Failed to load podcast with id: {}", id))?;
```

### Architecture Principles

1. **Storage Abstraction**: Always code against the `Storage` trait, never directly against JSON implementation
2. **Component Separation**: Clear separation between UI, business logic, and data persistence
3. **Event-Driven**: Event-driven patterns for UI updates and user interactions
4. **Buffer-Based UI**: Buffers, windows, and minibuffer patterns for organizing views
5. **Async-First**: Design for async operations, especially for network I/O and file operations

### Testing Requirements

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use tempfile::TempDir;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let input = create_test_data();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }

    #[tokio::test]
    async fn test_async_function() {
        // Use tokio::test for async tests
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

---

## 📚 Essential Documentation

### Primary Documentation (Start Here)

- **[README.md](README.md)** - Project overview, features, and quick start
- **[GETTING_STARTED.md](GETTING_STARTED.md)** - Detailed setup and platform-specific instructions
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development workflow, sprint process, PR requirements
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** - System architecture, design patterns, module structure

### Technical Documentation

- **[docs/TESTING.md](docs/TESTING.md)** - Comprehensive testing strategy and guidelines
- **[docs/BUILD_SYSTEM.md](docs/BUILD_SYSTEM.md)** - Cross-platform build instructions
- **[docs/STORAGE_DESIGN.md](docs/STORAGE_DESIGN.md)** - Storage abstraction architecture
- **[docs/OPML_SUPPORT.md](docs/OPML_SUPPORT.md)** - OPML import/export implementation
- **[docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)** - Complete keyboard shortcuts reference
- **[docs/WINGET_PUBLISHING.md](docs/WINGET_PUBLISHING.md)** - Windows Package Manager (winget) publishing workflow

### Project Management

- **[GitHub Projects Board](https://github.com/users/lqdev/projects/1)** - Issue tracking, priorities, and phase planning
- **[docs/PRD.md](docs/PRD.md)** - Product requirements and scope
- **[CHANGELOG.md](CHANGELOG.md)** - Version history and changes

### Code Guidelines

- **[.github/copilot-instructions.md](.github/copilot-instructions.md)** - Comprehensive code style, architecture patterns, and development best practices

---

## 🏗️ Project Structure

```
podcast-tui/
├── src/
│   ├── main.rs              # CLI entry point (clap argument parsing)
│   ├── app.rs               # Application state and startup
│   ├── config.rs            # Config structs (Audio, Download, Playlist, UI, Storage)
│   ├── constants.rs         # Centralized constants (network, downloads, ui, storage, etc.)
│   ├── lib.rs               # Library root
│   ├── storage/             # Data persistence abstraction
│   │   ├── mod.rs           # Module root
│   │   ├── traits.rs        # Storage trait definition
│   │   ├── json.rs          # JSON storage implementation
│   │   └── models.rs        # Shared storage models
│   ├── podcast/             # Domain models and RSS logic
│   │   ├── models.rs        # Podcast, Episode data models
│   │   ├── feed.rs          # RSS feed parsing (feed-rs)
│   │   ├── opml.rs          # OPML import/export
│   │   ├── subscription.rs  # Subscription management
│   │   └── mod.rs
│   ├── download/            # Download management + device sync + cleanup
│   │   ├── manager.rs       # DownloadManager (downloads, sync, cleanup)
│   │   └── mod.rs
│   ├── playlist/            # Playlist management
│   │   ├── models.rs        # Playlist, PlaylistType, AutoPlaylistKind, RefreshPolicy
│   │   ├── manager.rs       # PlaylistManager (CRUD, ordering)
│   │   ├── file_manager.rs  # Audio file copying for device compatibility
│   │   ├── auto_generator.rs # Today auto-playlist generation
│   │   └── mod.rs
│   ├── ui/                  # Terminal UI (ratatui + crossterm)
│   │   ├── app.rs           # UIApp main loop and event dispatch
│   │   ├── mod.rs           # UI module root
│   │   ├── events.rs        # Event types and handling
│   │   ├── keybindings.rs   # KeyChord binding registry
│   │   ├── themes.rs        # Theme definitions (dark/light/high-contrast/solarized)
│   │   ├── filters.rs       # EpisodeFilter (text, status, date range)
│   │   ├── buffers/         # 12 buffer implementations
│   │   │   ├── mod.rs           # Buffer trait + BufferManager
│   │   │   ├── podcast_list.rs  # Podcast subscription list
│   │   │   ├── episode_list.rs  # Episode list with filter support
│   │   │   ├── episode_detail.rs # Single episode view
│   │   │   ├── downloads.rs     # Active downloads progress
│   │   │   ├── help.rs          # Help keybinding reference
│   │   │   ├── buffer_list.rs   # Buffer switcher overlay
│   │   │   ├── playlist_list.rs # Playlist management view
│   │   │   ├── playlist_detail.rs # Single playlist view
│   │   │   ├── playlist_picker.rs # Add-to-playlist picker overlay
│   │   │   ├── sync.rs          # Device sync history view
│   │   │   └── whats_new.rs     # Rolling new episodes view
│   │   └── components/      # Reusable UI components
│   └── utils/               # Shared utilities (filesystem, text, validation)
├── tests/                   # Integration tests (6 files)
│   ├── test_episode_detail_feeds.rs
│   ├── test_opml_live_url.rs
│   ├── test_opml_local_file.rs
│   ├── test_playlist.rs
│   ├── test_sync_commands.rs
│   └── unsubscribe_integration_test.rs
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md      # System architecture
│   ├── TESTING.md           # Testing strategy
│   ├── KEYBINDINGS.md       # Complete keybinding reference
│   ├── BUILD_SYSTEM.md      # Cross-platform build instructions
│   ├── STORAGE_DESIGN.md    # Storage abstraction design
│   ├── OPML_SUPPORT.md      # OPML import/export
│   ├── SEARCH_AND_FILTER.md # Search/filter design (incl. Design Decision #13)
│   ├── WINGET_PUBLISHING.md # Windows Package Manager publishing
│   └── archive/             # Historical documentation
├── scripts/                 # Build and automation scripts
├── assets/                  # Application icons (SVG, PNG, ICO)
├── manifests/               # Winget package manifests
├── Cargo.toml               # Rust project configuration
└── .github/
    ├── workflows/           # CI/CD workflows
    └── copilot-instructions.md  # Lean code style supplement
```

---

## 🔄 Development Workflow

> **Full workflow details are in skills.** This section is a quick reference — see the linked skills for step-by-step procedures.

| Task | Skill |
|------|-------|
| Pick up an issue and submit a PR | `work-on-issue` |
| File a new issue | `create-issue` |
| Triage / label an issue | `triage-issue` |
| Cut a release | `prepare-release` |
| Update CHANGELOG.md | `update-changelog` |
| Validate code review feedback | `code-review-validation` |
| Create an ADR | `create-adr` |
| Create an RFC | `create-rfc` |
| Add a new UI buffer | `add-new-buffer` |
| Add a new minibuffer command | `add-new-command` |

### Quick Reference: Quality Checks

```bash
# Required before every commit
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Required before every PR
cargo build --release
```

### Quick Reference: Commit Format

```
type(scope): brief description

[optional body explaining what and why]

[optional footer: Closes #N or Part of #N]
```

**Types**: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `style`, `perf`

### Quick Reference: Branch Naming

```
{type}/issue-{number}-{short-description}
```

Examples: `feat/issue-74-fix-sync-foundation`, `fix/issue-99-download-timeout`

---

## 🎯 Project Management Reference

> **Procedural details are in skills.** Use the `create-issue` skill to file issues and the `triage-issue` skill to apply labels and project board fields.

### Project Board

All work is tracked on the [GitHub Projects board](https://github.com/users/lqdev/projects/1).

**Custom fields on every issue:**

| Field | Values | Meaning |
|-------|--------|---------|
| **Stack Rank** | Number (10, 20, 30…) | Canonical work order. Lower = higher priority. Gaps of 10 for easy insertion. |
| **Priority** | P0, P1, P2, P3 | P0 = blocker, P1 = high, P2 = medium, P3 = low |
| **Phase** | Phase 1, Phase 2, Phase 3, Backlog | Implementation phase or backlog |
| **Effort** | XS, S, M, L, XL | XS = trivial, S = half-day, M = full day, L = 2–3 days, XL = 3+ days |

**Status columns:** `Todo` → `In Progress` → `Done`

### Issue Hierarchy

- **Epics** use the `[Epic]` title prefix and contain linked sub-issues (GitHub sub-issues feature)
- **Sub-issues** are standalone issues linked to their parent epic
- **Standalone issues** for bugs, small features, or chores that don't need an epic

### Labels

| Category | Labels | Usage |
|----------|--------|-------|
| **Priority** | `P0`, `P1`, `P2`, `P3` | Severity / importance |
| **Type** | `bug`, `enhancement`, `documentation` | What kind of work |
| **Component** | `ui`, `downloads`, `sync`, `storage`, `rss`, `audio`, `performance` | Which module |
| **Status** | `needs-triage`, `blocked`, `help-wanted` | Workflow state |

---

## 🚫 Common Pitfalls to Avoid

### ❌ Don't Do This

```rust
// ❌ Never use unwrap() in production code
let value = some_option.unwrap();

// ❌ Don't block the UI thread
let data = blocking_io_operation();

// ❌ Don't hardcode configuration values
const MAX_DOWNLOADS: usize = 3;

// ❌ Don't ignore errors
let _ = operation_that_might_fail();

// ❌ Don't mix UI concerns with business logic
fn render_and_save_podcast() { }
```

### ✅ Do This Instead

```rust
// ✅ Use proper error handling
let value = some_option
    .ok_or(MyError::MissingValue)?;

// ✅ Use async for I/O operations
let data = async_io_operation().await?;

// ✅ Use centralized constants
use crate::constants::downloads::MAX_CONCURRENT;

// ✅ Handle errors properly
operation_that_might_fail()
    .with_context(|| "Failed to perform operation")?;

// ✅ Separate concerns
fn render_podcast(podcast: &Podcast) { }
fn save_podcast(podcast: &Podcast) -> Result<()> { }
```

---

## 🧪 Testing Strategy

### Test Coverage Requirements

- **Unit tests** for all business logic
- **Integration tests** for user workflows
- **Mock external dependencies** (network, filesystem)
- **Target**: 80%+ code coverage for production code

### Test Organization

```bash
# Unit tests: In the same file as the code
src/podcast/models.rs
    #[cfg(test)]
    mod tests { }

# Integration tests: Separate files in tests/
tests/
    test_episode_detail_feeds.rs       # Feed parsing end-to-end
    test_opml_live_url.rs              # OPML import from live URLs
    test_opml_local_file.rs            # OPML import from local files
    test_playlist.rs                   # Playlist CRUD and sync workflows
    test_sync_commands.rs              # Device sync command integration
    unsubscribe_integration_test.rs    # Subscribe/unsubscribe workflow
```

### Running Specific Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test test_opml_local_file

# Run tests matching a pattern
cargo test opml

# Run tests with logging output
RUST_LOG=debug cargo test -- --nocapture
```

See [docs/TESTING.md](docs/TESTING.md) for comprehensive testing guidelines.

---

## 🐛 Debugging

### Logging

```rust
// Add to your code
use log::{debug, info, warn, error};

debug!("Detailed debugging information");
info!("General information");
warn!("Warning messages");
error!("Error messages");
```

```bash
# Run with logging enabled
RUST_LOG=debug cargo run
RUST_LOG=podcast_tui=trace cargo run
RUST_LOG=info cargo test -- --nocapture
```

### Common Issues

**Issue**: Build fails with "alsa not found"
```bash
# Solution (Linux):
sudo apt-get install libasound2-dev pkg-config
```

**Issue**: Tests fail with file permission errors
```bash
# Solution: Tests use tempfile crate which should handle this
# Check that /tmp is writable
ls -la /tmp
```

**Issue**: Clippy warnings in CI
```bash
# Note: The codebase may have some existing clippy warnings
# Focus on not introducing NEW warnings in your changes
# Run clippy to check your changes:
cargo clippy -- -D warnings

# To see only warnings in files you modified:
git diff --name-only | xargs -I {} cargo clippy --quiet -- -D warnings
```

---

## 📦 Dependencies

### Core Dependencies

- `ratatui 0.29` - TUI framework
- `crossterm 0.29` - Cross-platform terminal manipulation
- `tokio` (full) - Async runtime
- `reqwest 0.12` (rustls-tls, stream, json) - HTTP client
- `feed-rs 2.0` - RSS/Atom feed parsing
- `rodio 0.21` - Audio playback (rodio backend + external player fallback)
- `serde` / `serde_json` - Serialization
- `quick-xml 0.31` - XML parsing (OPML)
- `regex 1.10` - Pattern matching
- `clap 4.0` - CLI argument parsing
- `anyhow 1.0` - Error context chaining
- `thiserror 2.0` - Custom error types
- `async-trait 0.1` - Async trait methods
- `uuid 1.0` (v4 + serde) - Unique identifiers
- `chrono 0.4` (serde) - Date/time handling
- `directories 5.0` - Platform-appropriate config/data paths
- `id3 1.9` - MP3 ID3 tag reading/writing
- `image 0.24` - Artwork image processing

### Development Dependencies

- `mockall 0.11` - Mocking framework for tests
- `tokio-test 0.4` - Testing utilities for async code
- `tempfile 3.0` - Temporary directories for tests

### Adding Dependencies

```bash
# Add a new dependency
cargo add dependency-name

# Add a development dependency
cargo add --dev dependency-name

# Add with specific features
cargo add dependency-name --features feature1,feature2
```

---

## 📊 Current Development Status

**Version**: 1.6.0  
**Status**: Active Development (February 2026)

### Completed Features
- ✅ Project setup and foundation
- ✅ Storage layer with JSON implementation (trait-based abstraction)
- ✅ Core UI framework with Emacs-style buffer management
- ✅ RSS subscription management (subscribe/unsubscribe/refresh/hard-refresh)
- ✅ OPML import/export (non-destructive, local files + URLs)
- ✅ Episode downloading with parallel progress tracking
- ✅ MP3 metadata (ID3 tags, artwork embedding, track numbers, readable filenames)
- ✅ Device sync to MP3 players/USB drives (metadata-based comparison, dry-run, orphan deletion)
- ✅ Download cleanup (auto on startup + manual `:clean-older-than`)
- ✅ Search & filter (text, status, date range — `src/ui/filters.rs`)
- ✅ Playlists (user playlists + auto-generated `Today` rolling 24h playlist)
- ✅ Theme system (dark/light/high-contrast/solarized)
- ✅ What's New buffer (rolling recent episodes across all podcasts)
- ✅ Winget publishing (Windows Package Manager)
- ✅ Audio playback (rodio backend + external player fallback)

### Not Yet Implemented
- ⏳ Episode notes
- ⏳ Statistics tracking
- ⏳ Duration filter (deferred — see `docs/SEARCH_AND_FILTER.md` Design Decision #13)

---

## 🗺️ Feature Map (Code → Functionality)

| Feature | Key Files | Commands / Keys |
|---------|-----------|-----------------|
| Subscribe/Unsubscribe | `src/podcast/subscription.rs`, `src/ui/buffers/podcast_list.rs` | `a` add, `d` delete, `r` refresh, `R` refresh all, `Ctrl+r` hard refresh |
| Episode List | `src/ui/buffers/episode_list.rs`, `src/ui/filters.rs` | Arrow keys navigate, `Enter` open detail |
| Downloads | `src/download/manager.rs`, `src/ui/buffers/downloads.rs` | `Shift+D` download, `F4` downloads buffer |
| Device Sync | `src/download/manager.rs` (sync methods) | `:sync [path]`, `:sync-dry-run [path]`, `F4`→sync buffer |
| Download Cleanup | `src/download/manager.rs` (`cleanup_old_downloads*`) | `:clean-older-than <dur>`, `:cleanup <dur>` |
| Search & Filter | `src/ui/filters.rs`, `src/ui/buffers/episode_list.rs` | `/` search, `:filter-status`, `:filter-date`, `:clear-filters` |
| Playlists | `src/playlist/` (5 files), `src/ui/buffers/playlist_*.rs` | `c` create, `F7` list, `p` add episode, `:playlist-*` commands |
| OPML | `src/podcast/opml.rs` | `Shift+A` import, `Shift+E` export, `:import-opml`, `:export-opml` |
| Themes | `src/ui/themes.rs` | `:theme <dark|light|high-contrast|solarized>` |
| Config | `src/config.rs` | `~/.config/podcast-tui/config.json` (Linux) |
| Constants | `src/constants.rs` | All default values centralized here |
| Buffer Mgmt | `src/ui/buffers/mod.rs` | `Tab`/`Shift+Tab`, `F2-F7`, `Ctrl+b` list, `Ctrl+k` close |

---



### Official Documentation
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Ratatui Documentation](https://ratatui.rs/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

### Project-Specific
- [GitHub Repository](https://github.com/lqdev/podcast-tui)
- [Issue Tracker](https://github.com/lqdev/podcast-tui/issues)
- [Project Board](https://github.com/lqdev/podcast-tui/projects)

---

## 💡 Tips for AI Code Assistants

1. **Always reference existing documentation** before suggesting changes
2. **Follow the Storage trait pattern** - never hardcode JSON implementations
3. **Use centralized constants** from `src/constants.rs` - never hardcode values
4. **Prefer small, focused changes** over large refactorings
5. **Write tests alongside code changes** to ensure correctness
6. **Check ARCHITECTURE.md** for established patterns before creating new ones
7. **Maintain consistency** with existing code style and patterns
8. **Document non-obvious decisions** in code comments
9. **Update relevant documentation** when making architectural changes
10. **Test cross-platform compatibility** when changing build or file system code

### When Suggesting Changes

✅ **Do**:
- Reference specific files and line numbers
- Explain the "why" behind suggestions
- Provide complete, working code examples
- Include test cases for new functionality
- Consider error handling and edge cases

❌ **Don't**:
- Suggest breaking changes without strong justification
- Introduce new dependencies without discussing alternatives
- Skip error handling for "quick fixes"
- Suggest platform-specific solutions without fallbacks
- Ignore existing architecture patterns

---

**Last Updated**: February 2026  
**Version**: 1.6.0
**For Questions**: See [CONTRIBUTING.md](CONTRIBUTING.md) or open an issue on GitHub
