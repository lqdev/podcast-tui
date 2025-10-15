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

### Project Management

- **[docs/PRD.md](docs/PRD.md)** - Product requirements and scope
- **[docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md)** - 8-week sprint roadmap
- **[CHANGELOG.md](CHANGELOG.md)** - Version history and changes

### Code Guidelines

- **[.github/copilot-instructions.md](.github/copilot-instructions.md)** - Comprehensive code style, architecture patterns, and development best practices

---

## 🏗️ Project Structure

```
podcast-tui/
├── src/
│   ├── main.rs              # Application entry point
│   ├── constants.rs         # Centralized configuration constants
│   ├── storage/             # Data persistence abstraction
│   │   ├── mod.rs          # Storage trait definition
│   │   └── json.rs         # JSON storage implementation
│   ├── podcast/             # Domain models and RSS logic
│   │   ├── models.rs       # Podcast, Episode data models
│   │   └── feed.rs         # RSS feed parsing
│   ├── download/            # Download management
│   ├── ui/                  # Terminal UI
│   │   ├── app.rs          # Main application state
│   │   ├── buffers/        # UI buffers (podcast list, episodes, etc.)
│   │   └── components/     # Reusable UI components
│   └── utils/               # Shared utilities
├── tests/                   # Integration tests
├── examples/                # Example code
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md     # System architecture
│   ├── TESTING.md          # Testing strategy
│   └── archive/            # Historical documentation
├── scripts/                 # Build and automation scripts
├── Cargo.toml              # Rust project configuration
├── .cargo/config.toml      # Cargo build settings
└── .github/
    ├── workflows/          # CI/CD workflows
    └── copilot-instructions.md  # Detailed code guidelines
```

---

## 🔄 Development Workflow

### Before Starting Work

1. **Understand the issue**: Read the issue description and comments
2. **Review documentation**: Check [ARCHITECTURE.md](docs/ARCHITECTURE.md) for relevant design patterns
3. **Check sprint status**: Review [IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) for current priorities
4. **Set up branch**: Create a feature branch from `main`

```bash
git checkout -b feature/description
```

### During Development

1. **Follow TDD where appropriate**: Write tests first for business logic
2. **Run tests frequently**: `cargo test` after each change
3. **Check code quality**: `cargo clippy` to catch common issues
4. **Format code**: `cargo fmt` before commits

### Before Committing

```bash
# Required quality checks
cargo fmt --check              # Verify formatting
cargo clippy -- -D warnings    # Check for warnings
cargo test                     # Run all tests
cargo build --release          # Verify release build
```

### Commit Message Format

```
type(scope): brief description

[optional body explaining what and why]

[optional footer with breaking changes or issue refs]
```

**Types**: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `style`, `perf`

**Examples**:
```bash
feat(ui): add episode filtering buffer
fix(download): handle network timeout gracefully
docs: update ARCHITECTURE.md with storage patterns
refactor: extract constants to centralized module
test: add property-based tests for validation
```

### Creating a Pull Request

1. **Push your branch**: `git push origin feature/description`
2. **Create PR** with:
   - Descriptive title and description
   - Link to related issues
   - Screenshots/demos for UI changes
   - Test coverage for new functionality
3. **Address review feedback**
4. **Ensure CI passes** (formatting, linting, tests)

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

# Integration tests: Separate files
tests/
    test_opml_local_file.rs
    test_episode_detail_feeds.rs
    unsubscribe_integration_test.rs
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

- `ratatui` - TUI framework
- `crossterm` - Cross-platform terminal manipulation
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `feed-rs` - RSS parsing
- `rodio` - Audio playback
- `serde` / `serde_json` - Serialization

### Development Dependencies

- `mockall` - Mocking framework for tests
- `tokio-test` - Testing utilities for async code
- `tempfile` - Temporary directories for tests

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

## 🎯 Current Development Status

**Version**: 1.0.0-mvp (in development)  
**Progress**: 37.5% complete (3/8 sprints done)

### Completed (Sprints 0-3)
- ✅ Project setup and foundation
- ✅ Storage layer with JSON implementation
- ✅ Core UI framework with buffer management
- ✅ RSS subscription management
- ✅ OPML import/export
- ✅ Episode downloading with progress tracking

### In Progress (Sprint 4)
- ⏳ Audio playback with rodio
- ⏳ Playback controls and chapter navigation

### Upcoming (Sprints 5-7)
- Playlist creation and management
- Episode notes
- Search and filtering
- Statistics tracking
- Final polish and documentation

See [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) for detailed sprint information.

---

## 🔗 Helpful Resources

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

**Last Updated**: October 2025  
**For Questions**: See [CONTRIBUTING.md](CONTRIBUTING.md) or open an issue on GitHub
