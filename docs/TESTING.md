# Testing Documentation - Podcast TUI

**Version**: 1.1  
**Last Updated**: February 2026  
**Status**: Living Document

---

## 📋 Overview

This document outlines the comprehensive testing strategy for Podcast TUI. It serves as a guide for implementing and maintaining tests across the codebase, ensuring reliability, maintainability, and code quality.

### Testing Philosophy

Following the project's MVP-focused approach:
- **Pragmatic Testing**: Focus on critical paths and high-risk areas
- **Test What Matters**: Prioritize business logic and user-facing functionality
- **Fast Feedback**: Keep test suites fast for rapid iteration
- **Clear Intent**: Tests should serve as documentation of behavior

---

## 🎯 Testing Goals

### Current Status
- ✅ **Unit tests** passing (storage, models, download manager, utilities, filters)
- ✅ **6 integration tests** (OPML, episode detail feeds, unsubscribe, playlist, sync commands)
- ⚠️ **Coverage**: Estimated 65-75% (storage and core logic well-covered; UI buffers lower)
- ⏳ **Target**: 80%+ coverage for production code

### Priority Areas for Additional Testing

**High Priority:**
1. Playlist edge cases (duplicate adds, ordering, Today refresh)
2. Device sync edge cases (orphan deletion, dry-run mode, path override)
3. Episode filter combinations (multi-filter AND logic)
4. Download cleanup duration parsing edge cases

**Medium Priority:**
5. UI buffer state management (filter persistence across buffer switches)
6. Concurrent download edge cases
7. Configuration loading/saving with all new fields
8. ID3 metadata embedding edge cases

**Low Priority (Post-MVP):**
9. Performance benchmarking
10. Fuzzing for RSS parser robustness
11. UI rendering tests
12. Cross-platform compatibility matrix tests

---

## 🧪 Test Categories

### 1. Unit Tests

**Purpose**: Test individual functions and methods in isolation

**Location**: `src/*/mod.rs` and dedicated test modules

**Coverage Areas**:
- ✅ Storage trait implementations (`src/storage/json.rs`)
- ✅ Data model serialization (`src/podcast/models.rs`)
- ✅ Utility functions (`src/utils/*.rs`)
- ✅ Configuration loading (`src/config.rs`)
- ✅ Constants validation (`src/constants.rs`)
- ✅ Download cleanup duration parsing (`src/utils/`)
- ✅ Download manager cleanup logic (`src/download/manager.rs`)
- ⏳ Episode filter logic (`src/ui/filters.rs`)
- ⏳ Playlist auto-generator (`src/playlist/auto_generator.rs`)

**Example Structure**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_function_name_with_clear_scenario() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }

    #[test]
    fn test_error_case_descriptive_name() {
        let invalid_input = create_invalid_data();
        let result = function_under_test(invalid_input);
        assert!(result.is_err());
    }
}
```

**Best Practices**:
- Use descriptive test names: `test_<function>_<scenario>_<expected>`
- Follow Arrange-Act-Assert pattern
- Use `tempfile` for file system tests
- Mock external dependencies
- Test both success and error paths

### 2. Integration Tests

**Purpose**: Test component interactions and workflows

**Location**: `tests/` directory (separate from `src/`)

**Current Tests** (6 integration test files):
- ✅ `test_opml_local_file.rs` - OPML file import workflow
- ✅ `test_opml_live_url.rs` - OPML URL import workflow
- ✅ `test_episode_detail_feeds.rs` - Feed parsing end-to-end
- ✅ `unsubscribe_integration_test.rs` - Unsubscribe workflow
- ✅ `test_playlist.rs` - Playlist CRUD and sync workflows
- ✅ `test_sync_commands.rs` - Device sync command integration

**Planned Integration Tests**:

```rust
// tests/test_download_workflow.rs
#[tokio::test]
async fn test_complete_download_workflow() {
    // Setup: Create storage, add podcast, fetch episodes
    // Act: Download episode
    // Assert: File exists, metadata updated, progress tracked
}

// tests/test_subscription_workflow.rs
#[tokio::test]
async fn test_subscribe_refresh_unsubscribe() {
    // Test complete subscription lifecycle
}

// tests/test_opml_edge_cases.rs
#[tokio::test]
async fn test_opml_with_malformed_xml() {
    // Test error handling for invalid OPML
}

#[tokio::test]
async fn test_opml_with_duplicate_feeds() {
    // Test duplicate detection logic
}

#[tokio::test]
async fn test_opml_with_large_file() {
    // Test handling of OPML with 1000+ feeds
}
```

**Best Practices**:
- Use realistic test data
- Test complete user workflows
- Use `tokio::test` for async tests
- Clean up test artifacts
- Use test fixtures for common setups

### 3. Property-Based Tests

**Purpose**: Test properties that should hold for many inputs

**Tool**: [proptest](https://github.com/proptest-rs/proptest) or [quickcheck](https://github.com/BurntSushi/quickcheck)

**Planned Tests**:

```rust
// Example: URL validation properties
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_url_validation_idempotent(url in "https?://[a-z0-9.]+/.*") {
        let validated = validate_url(&url);
        let revalidated = validate_url(&validated.unwrap());
        prop_assert_eq!(validated, revalidated);
    }

    #[test]
    fn test_sanitize_filename_safe(s in ".*") {
        let sanitized = sanitize_filename(&s);
        prop_assert!(!sanitized.contains('/'));
        prop_assert!(!sanitized.contains('\\'));
        prop_assert!(sanitized.len() <= MAX_FILENAME_LENGTH);
    }
}
```

**Target Areas**:
- Filename sanitization
- URL parsing and validation
- RSS feed parsing with varied inputs
- Path manipulation utilities
- Configuration validation

### 4. Mock-Based Tests

**Purpose**: Test components that depend on external systems

**Tool**: [mockall](https://github.com/asomers/mockall) (already in use)

**Current Usage**:
- Storage trait mocking for business logic tests

**Example Pattern**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;

    // Define mock storage
    mock! {
        Storage {}
        
        #[async_trait]
        impl Storage for Storage {
            type Error = StorageError;
            
            async fn save_podcast(&self, podcast: &Podcast) 
                -> Result<(), Self::Error>;
            async fn load_podcast(&self, id: &str) 
                -> Result<Podcast, Self::Error>;
        }
    }

    #[tokio::test]
    async fn test_with_mocked_storage() {
        let mut mock_storage = MockStorage::new();
        mock_storage
            .expect_save_podcast()
            .times(1)
            .returning(|_| Ok(()));
            
        // Test logic that uses storage
        let result = some_function(&mock_storage).await;
        assert!(result.is_ok());
    }
}
```

**Mock Targets**:
- Storage operations (already mocked)
- HTTP client for network requests
- Audio player interface
- File system operations (for CI environments)

### 5. Error Handling Tests

**Purpose**: Verify graceful error handling and user feedback

**Critical Scenarios**:
```rust
#[tokio::test]
async fn test_network_timeout_handling() {
    // Simulate network timeout
    // Verify appropriate error returned
    // Verify user-friendly error message
}

#[tokio::test]
async fn test_disk_full_scenario() {
    // Simulate disk full during download
    // Verify cleanup of partial file
    // Verify error propagation
}

#[tokio::test]
async fn test_corrupted_storage_recovery() {
    // Create corrupted JSON file
    // Verify graceful degradation or recovery
    // Verify user notified appropriately
}

#[tokio::test]
async fn test_invalid_feed_url() {
    // Test various invalid URL formats
    // Verify clear error messages
}
```

---

## 📊 Test Organization

### Directory Structure

```
tests/
├── common/
│   ├── mod.rs                    # Shared test utilities
│   ├── fixtures.rs               # Test data fixtures
│   └── helpers.rs                # Helper functions
├── integration/
│   ├── download_workflow.rs
│   ├── subscription_workflow.rs
│   ├── opml_edge_cases.rs
│   └── buffer_navigation.rs
├── property/
│   ├── validation_properties.rs
│   └── sanitization_properties.rs
└── test_*.rs                     # Top-level integration tests

src/
├── storage/
│   ├── json.rs                   # Contains #[cfg(test)] mod tests
│   └── mod.rs
├── podcast/
│   ├── models.rs                 # Contains unit tests
│   ├── feed.rs                   # Contains feed parsing tests
│   └── ...
└── utils/
    ├── fs.rs                     # Contains utility tests
    └── validation.rs             # Contains validation tests
```

### Test Fixtures

**Location**: `tests/fixtures/`

**Contents**:
```
tests/fixtures/
├── feeds/
│   ├── valid_feed.xml
│   ├── malformed_feed.xml
│   ├── empty_feed.xml
│   └── large_feed.xml
├── opml/
│   ├── valid_subscriptions.opml
│   ├── duplicate_feeds.opml
│   ├── invalid_xml.opml
│   └── large_subscription_list.opml
└── config/
    ├── valid_config.json
    ├── minimal_config.json
    └── invalid_config.json
```

---

## 🚀 Running Tests

### Basic Commands

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in specific module
cargo test storage::

# Run integration tests only
cargo test --test '*'

# Run with specific features
cargo test --features "feature-name"
```

### Continuous Integration

```bash
# Pre-commit checks
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check

# Coverage report (requires cargo-tarpaulin)
cargo tarpaulin --out Html --output-dir coverage/
```

### Test Configuration

```toml
# Cargo.toml - Test configuration
[dev-dependencies]
tempfile = "3.0"
mockall = "0.11"
tokio-test = "0.4"
proptest = "1.0"          # Add for property-based testing
test-case = "3.0"         # Add for parameterized tests

[[test]]
name = "integration"
path = "tests/integration/mod.rs"
harness = true
```

---

## 📈 Coverage Goals

### Current Coverage (Estimated)

| Module | Coverage | Status |
|--------|----------|--------|
| `storage/` | ~90% | ✅ Excellent |
| `podcast/models` | ~85% | ✅ Good |
| `utils/` | ~80% | ✅ Good |
| `config` | ~75% | ✅ Good |
| `constants` | 100% | ✅ Complete |
| `podcast/feed` | ~60% | ⚠️ Needs work |
| `download/` | ~50% | ⚠️ Needs work |
| `ui/` | ~30% | ⚠️ Minimal |

### Target Coverage (MVP)

- **Overall**: 80%+ for production code
- **Critical Paths**: 95%+ (storage, feed parsing, downloads)
- **UI Components**: 60%+ (focus on state management)
- **Utilities**: 90%+ (reusable code needs high confidence)

### Tracking Coverage

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/ --exclude-files 'src/main.rs' 'src/ui/*'

# View report
open coverage/index.html  # macOS
start coverage/index.html # Windows
```

---

## ✅ Test Quality Guidelines

### Writing Good Tests

**DO**:
- ✅ Write descriptive test names
- ✅ Test one thing per test
- ✅ Use Arrange-Act-Assert pattern
- ✅ Make tests independent
- ✅ Clean up test artifacts
- ✅ Test error cases
- ✅ Use realistic test data

**DON'T**:
- ❌ Use `unwrap()` in tests (use `?` or explicit asserts)
- ❌ Depend on external services
- ❌ Share mutable state between tests
- ❌ Write flaky tests
- ❌ Test implementation details
- ❌ Ignore failing tests
- ❌ Skip error path testing

### Test Naming Convention

```rust
// Pattern: test_<function>_<scenario>_<expected_outcome>

#[test]
fn test_save_podcast_valid_data_succeeds() { }

#[test]
fn test_save_podcast_invalid_path_returns_error() { }

#[test]
fn test_parse_feed_malformed_xml_handles_gracefully() { }
```

### Assertion Helpers

```rust
// Custom assertions for better error messages
macro_rules! assert_podcast_eq {
    ($left:expr, $right:expr) => {
        assert_eq!($left.id, $right.id, "Podcast IDs don't match");
        assert_eq!($left.title, $right.title, "Podcast titles don't match");
        // ... other fields
    };
}

// Use in tests
assert_podcast_eq!(loaded_podcast, expected_podcast);
```

---

## 🔍 Testing Specific Components

### Storage Layer

**Test Coverage**: ✅ ~90% (Excellent)

**Key Tests**:
- ✅ Save/load podcast roundtrip
- ✅ Atomic writes (temp file → rename)
- ✅ Error handling (I/O errors, permission errors)
- ✅ Serialization/deserialization
- ✅ Directory creation

**Additional Tests Needed**:
- ⏳ Concurrent access scenarios
- ⏳ Large dataset performance
- ⏳ Backup and recovery

### RSS Feed Parsing

**Test Coverage**: ⚠️ ~60% (Needs improvement)

**Current Tests**:
- ✅ Basic RSS feed parsing
- ✅ Episode extraction
- ✅ Metadata handling

**Additional Tests Needed**:
```rust
#[tokio::test]
async fn test_feed_with_missing_enclosure() { }

#[tokio::test]
async fn test_feed_with_invalid_date_format() { }

#[tokio::test]
async fn test_feed_with_html_entities() { }

#[tokio::test]
async fn test_feed_with_multiple_enclosures() { }

#[tokio::test]
async fn test_atom_feed_parsing() { }
```

### Download Manager

**Test Coverage**: ⚠️ ~50% (Needs improvement)

**Current Tests**:
- ✅ Basic download functionality
- ✅ Progress tracking

**Additional Tests Needed**:
```rust
#[tokio::test]
async fn test_concurrent_download_limit() { }

#[tokio::test]
async fn test_download_retry_on_failure() { }

#[tokio::test]
async fn test_download_cancellation() { }

#[tokio::test]
async fn test_disk_space_check() { }

#[tokio::test]
async fn test_network_timeout_handling() { }

#[tokio::test]
async fn test_partial_download_cleanup() { }
```

### OPML Operations

**Test Coverage**: ✅ ~75% (Good, but edge cases needed)

**Current Tests**:
- ✅ Local file import
- ✅ URL import
- ⚠️ Basic duplicate detection (1 test failing)

**Additional Tests Needed**:
```rust
#[tokio::test]
async fn test_opml_with_malformed_xml() { }

#[tokio::test]
async fn test_opml_with_invalid_feed_urls() { }

#[tokio::test]
async fn test_opml_with_1000_feeds() { }

#[tokio::test]
async fn test_opml_with_special_characters() { }

#[tokio::test]
async fn test_opml_export_import_roundtrip() { }
```

### UI Buffers

**Test Coverage**: ⚠️ ~30% (Minimal)

**Focus Areas**:
```rust
#[test]
fn test_buffer_navigation() { }

#[test]
fn test_buffer_selection_state() { }

#[test]
fn test_buffer_scroll_bounds() { }

#[test]
fn test_buffer_focus_management() { }

#[test]
fn test_minibuffer_command_parsing() { }
```

### Utilities

**Test Coverage**: ✅ ~80% (Good)

**Well-Covered**:
- ✅ Filename sanitization
- ✅ Path manipulation
- ✅ URL validation
- ✅ Tilde expansion

**Additional Property-Based Tests Recommended**:
```rust
proptest! {
    #[test]
    fn prop_sanitize_filename_always_valid(s in ".*") {
        let result = sanitize_filename(&s);
        prop_assert!(is_valid_filename(&result));
    }
}
```

---

## 🎯 Test Implementation Roadmap

### Phase 5: Add Missing Tests (Planned)

**Week 1: OPML Edge Cases**
- [ ] Malformed XML handling
- [ ] Invalid URL handling
- [ ] Large file performance
- [ ] Special character handling
- [ ] Roundtrip export/import

**Week 2: Feed Parsing**
- [ ] Missing enclosure handling
- [ ] Invalid date formats
- [ ] HTML entity decoding
- [ ] Multiple enclosures
- [ ] Atom feed support

**Week 3: Download Manager**
- [ ] Concurrent download limits
- [ ] Retry logic
- [ ] Cancellation handling
- [ ] Disk space checks
- [ ] Network timeouts

**Week 4: Property-Based Tests**
- [ ] Validation properties
- [ ] Sanitization properties
- [ ] Path manipulation properties

**Week 5: Integration Tests**
- [ ] Complete workflows
- [ ] Error recovery scenarios
- [ ] Cross-component interactions

---

## 📝 Test Maintenance

### Keeping Tests Healthy

**Regular Tasks**:
1. Run full test suite before committing
2. Fix failing tests immediately
3. Update tests when refactoring
4. Remove obsolete tests
5. Add tests for bug fixes

### Test Debt Tracking

**Current Test Debt**:
- ⚠️ 1 failing integration test (OPML live URL)
- ⏳ Feed parsing edge cases
- ⏳ Download error scenarios
- ⏳ UI buffer state tests
- ⏳ Property-based validation tests

**Addressing Test Debt**:
- Track in GitHub issues with `test-debt` label
- Prioritize based on risk and frequency
- Allocate time in each sprint for test improvements

---

## 🔧 Testing Tools

### Required Dependencies

```toml
[dev-dependencies]
tempfile = "3.0"           # Temporary directories/files
mockall = "0.11"           # Mocking framework
tokio-test = "0.4"         # Async test utilities
```

### Recommended Additions

```toml
[dev-dependencies]
proptest = "1.0"           # Property-based testing
test-case = "3.0"          # Parameterized tests
criterion = "0.5"          # Benchmarking
cargo-tarpaulin = "0.27"   # Coverage (install globally)
```

### Useful Cargo Commands

```bash
# Watch tests (requires cargo-watch)
cargo watch -x test

# Run tests with timing
cargo test -- --show-output --test-threads=1

# Run specific test pattern
cargo test opml

# Test with backtrace
RUST_BACKTRACE=1 cargo test

# Update test snapshots (if using insta)
cargo insta review
```

---

## 📚 Additional Resources

### Testing References
- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Rust By Example - Testing](https://doc.rust-lang.org/rust-by-example/testing.html)
- [mockall Documentation](https://docs.rs/mockall/)
- [proptest Documentation](https://docs.rs/proptest/)

### Project-Specific
- [ARCHITECTURE.md](ARCHITECTURE.md) - System design for test planning
- [copilot-instructions.md](../.github/copilot-instructions.md) - Testing patterns and anti-patterns
- [STORAGE_DESIGN.md](STORAGE_DESIGN.md) - Storage testing considerations

---

## ✅ Definition of Done - Testing

For a feature to be considered complete:
- [ ] Unit tests for business logic (80%+ coverage)
- [ ] Integration tests for workflows
- [ ] Error cases tested
- [ ] Edge cases identified and tested
- [ ] Documentation updated with test examples
- [ ] All tests passing in CI
- [ ] No test-related clippy warnings

---

**Document Version**: 1.0  
**Last Updated**: October 7, 2025  
**Next Review**: After Sprint 5 (when test implementation begins)  
**Maintained By**: Development Team
