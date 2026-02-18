# Project Cleanup Plan

**Date**: October 7, 2025  
**Status**: Planning Phase  
**Branch**: `repo-cleanup`

## Overview

This document outlines the comprehensive cleanup plan for the Podcast TUI repository to improve maintainability, reduce confusion, and ensure adherence to Rust and software engineering best practices.

---

## 📋 Documentation Cleanup

### Files to Archive

Create `docs/archive/` structure with subdirectories:
- `docs/archive/fixes/` - Bug fix documentation
- `docs/archive/implementation_notes/` - Historical implementation details
- `docs/archive/summaries/` - Completion summaries

#### Root-Level Files to Archive

| File | Destination | Reason |
|------|-------------|--------|
| `BUFFER_SCROLLING_FIX.md` | `docs/archive/fixes/` | Implementation detail, historical |
| `COLON_KEYBINDING_FIX.md` | `docs/archive/fixes/` | Bug fix documentation, historical |
| `OPML_AUTOCOMPLETE_FIX.md` | `docs/archive/fixes/` | Bug fix documentation, historical |
| `OPML_BUG_FIX_COMPLETE.md` | `docs/archive/summaries/` | Obsolete, covered in main docs |
| `OPML_IMPROVEMENTS.md` | `docs/archive/implementation_notes/` | Merged into main documentation |
| `OPML_FEATURE_COMPLETE.md` | `docs/archive/summaries/` | Obsolete, feature is documented elsewhere |
| `IMPLEMENTATION_COMPLETE.md` | `docs/archive/summaries/` | Vague title, content covered elsewhere |
| `ISSUE_FIXES_SUMMARY.md` | `docs/archive/summaries/` | Historical, issues should be in git history |
| `DOCUMENTATION_UPDATE_SUMMARY.md` | `docs/archive/summaries/` | Historical, archive it |
| `SETUP_COMPLETE.md` | `docs/archive/summaries/` | Duplicates `BUILD_SYSTEM_FINAL.md` |
| `WINDOWS_BUILD_COMPLETE.md` | `docs/archive/summaries/` | Duplicates `scripts/README-WINDOWS.md` |
| `WHATS_NEW_BUFFER_IMPLEMENTATION.md` | `docs/archive/implementation_notes/` | Historical, in CHANGELOG |

#### Files to Delete

| File | Reason |
|------|--------|
| `GIT_COMMIT_INFO.md` | Temporary file, should be removed |

#### `docs/` Files to Archive

| File | Destination | Reason |
|------|-------------|--------|
| `docs/OPML_IMPLEMENTATION_SUMMARY.md` | `docs/archive/implementation_notes/` | Implementation detail |
| `docs/OPML_REAL_WORLD_FIX_SUMMARY.md` | `docs/archive/fixes/` | Bug fix summary |
| `docs/OPML_XML_SANITIZATION_FIX.md` | `docs/archive/fixes/` | Bug fix detail |
| `docs/TESTING_OPML_URL_FIX.md` | `docs/archive/fixes/` | Test documentation for fix |
| `docs/BUGFIX_OPML_URL_HANDLING.md` | `docs/archive/fixes/` | Bug fix detail |
| `docs/FEATURE-OPML.md` | `docs/archive/implementation_notes/` | Implementation notes |

### Files to Consolidate

#### Build Documentation (Too Many Overlapping Files)

**Current Files:**
- `BUILD_COMMANDS.md`
- `BUILD_SYSTEM_FINAL.md`
- `docs/BUILD_SYSTEM.md`
- `docs/BUILD_SYSTEM_SUMMARY.md`
- `scripts/README.md`
- `scripts/README-WINDOWS.md`

**Recommendation:**
- **Keep**: `docs/BUILD_SYSTEM.md` (comprehensive guide)
- **Keep**: `scripts/README.md` (platform-specific quick reference)
- **Delete**: `BUILD_COMMANDS.md`, `BUILD_SYSTEM_FINAL.md`, `docs/BUILD_SYSTEM_SUMMARY.md`

#### OPML Documentation

**Current Files:**
- `docs/OPML_SUPPORT.md` ✅ Keep (user guide)
- All other OPML files → Archive (see table above)

#### Getting Started Documentation

**Current Files:**
- `GETTING_STARTED.md`
- `QUICKSTART.md`

**Recommendation:**
- Merge `QUICKSTART.md` into `GETTING_STARTED.md`
- Delete `QUICKSTART.md` after merge

---

## 📚 Recommended Documentation Structure

### Root Level (User-Facing)

```
README.md                    # Project overview, quick start, features
GETTING_STARTED.md          # Detailed setup for all platforms
CHANGELOG.md                # Version history
CONTRIBUTING.md             # Contribution guidelines
LICENSE                     # MIT license
```

### docs/ (Detailed Documentation)

```
docs/
├── PRD.md                  # Product requirements (✅ exists)
├── IMPLEMENTATION_PLAN.md  # Sprint planning (⚠️ needs update)
├── ARCHITECTURE.md         # System architecture (➕ create new)
├── BUILD_SYSTEM.md        # Build instructions (✅ exists)
├── KEYBINDINGS.md         # Keybinding reference (✅ exists)
├── EMACS_KEYBINDINGS.md   # Emacs-style bindings (✅ exists)
├── OPML_SUPPORT.md        # OPML user guide (✅ exists)
├── STORAGE_DESIGN.md      # Storage architecture (✅ exists)
├── PROJECT_CLEANUP.md     # This document (➕ create new)
└── archive/               # Historical documents (➕ create new)
    ├── fixes/
    ├── implementation_notes/
    └── summaries/
```

### scripts/ (Build Scripts)

```
scripts/
├── README.md              # Platform-specific build guide (✅ exists)
├── build-linux.sh         # (✅ exists)
├── build-windows.ps1      # (✅ exists)
├── build-releases.sh      # (✅ exists)
└── install-build-deps.sh  # (✅ exists)
```

---

## 🔄 Documentation Updates Needed

### 1. README.md
- [ ] Update progress percentage (currently shows 37.5%, but more features completed)
- [ ] Reflect latest feature status
- [ ] Add link to `ARCHITECTURE.md`

### 2. IMPLEMENTATION_PLAN.md
- [ ] Mark Sprint 3 as complete
- [ ] Update Sprint 4 status
- [ ] Add Sprint 5 planning if needed

### 3. ARCHITECTURE.md (New File)
- [ ] Create comprehensive architecture documentation
- [ ] Include system diagrams
- [ ] Document data flow
- [ ] Explain design patterns used
- [ ] Reference from `CONTRIBUTING.md`

### 4. GETTING_STARTED.md
- [ ] Merge content from `QUICKSTART.md`
- [ ] Ensure all platforms covered
- [ ] Add troubleshooting section

### 5. CONTRIBUTING.md
- [ ] Add reference to `ARCHITECTURE.md`
- [ ] Update with new documentation structure
- [ ] Add code style guidelines

---

## 🔍 Code Quality Issues

### 1. Unused Imports and Dead Code

**Action Items:**
- [ ] Run `cargo clippy` to identify unused imports
- [ ] Run `cargo-udeps` to find unused dependencies
- [ ] Remove dead code flagged by compiler

### 2. Repeated Code Patterns

#### Issue: Path Expansion Logic Duplicated

**Locations:**
- `src/config.rs` (line ~43)
- `src/podcast/opml.rs`
- `src/podcast/subscription.rs`

**Solution:**
Create `src/utils/fs.rs` with reusable function:

```rust
/// Expand tilde (~) in file paths to home directory
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = dirs::home_dir() {
            if path == "~" {
                return home;
            }
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}
```

**Action Items:**
- [ ] Create utility function in `src/utils/fs.rs`
- [ ] Replace duplicated logic in `src/config.rs`
- [ ] Replace duplicated logic in `src/podcast/opml.rs`
- [ ] Replace duplicated logic in `src/podcast/subscription.rs`
- [ ] Add unit tests for `expand_tilde()`

### 3. Error Handling Improvements

#### Issue: Manual XML Sanitization

**Current**: Custom regex-based sanitization in `src/podcast/opml.rs`

**Recommendation**: Use dedicated library

**Action Items:**
- [ ] Evaluate `html-escape` or similar crates
- [ ] Update `Cargo.toml` with new dependency
- [ ] Refactor `sanitize_xml()` function
- [ ] Add tests for edge cases

### 4. Magic Numbers and Hardcoded Values

**Issue**: Magic numbers scattered throughout codebase

**Locations:**
- `src/config.rs` - Default values (3, 255, 50, 30)
- `src/download/manager.rs` - Timeout values
- Various modules - File size limits, retry counts

**Solution:**
Create `src/constants.rs` module:

```rust
//! Application-wide constants

/// Network timeouts
pub mod network {
    use std::time::Duration;
    
    pub const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
    pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
    pub const FEED_REFRESH_TIMEOUT: Duration = Duration::from_secs(60);
}

/// File system limits
pub mod filesystem {
    pub const MAX_FILENAME_LENGTH: usize = 255;
    pub const MAX_PATH_LENGTH: usize = 4096;
}

/// Download configuration
pub mod downloads {
    pub const DEFAULT_CONCURRENT_DOWNLOADS: usize = 3;
    pub const MAX_CONCURRENT_DOWNLOADS: usize = 10;
    pub const CHUNK_SIZE: usize = 8192;
}

/// UI configuration
pub mod ui {
    pub const DEFAULT_WHATS_NEW_LIMIT: usize = 50;
    pub const MAX_WHATS_NEW_LIMIT: usize = 200;
}
```

**Action Items:**
- [ ] Create `src/constants.rs`
- [ ] Export module from `src/lib.rs`
- [ ] Replace magic numbers in `src/config.rs`
- [ ] Replace magic numbers in `src/download/manager.rs`
- [ ] Replace magic numbers in other modules
- [ ] Update tests to use constants

### 5. Consolidate Similar Test Patterns

**Issue**: Duplicated test setup code

**Solution**: Create test fixtures and helper functions

**Action Items:**
- [ ] Create `tests/fixtures/` directory
- [ ] Add sample OPML files
- [ ] Add sample podcast feed XMLs
- [ ] Create test helper functions
- [ ] Refactor existing tests to use fixtures

---

## 🏗️ Structural Improvements

### 1. Create Constants Module

**File**: `src/constants.rs`

**Action Items:**
- [ ] Create module with nested submodules
- [ ] Define network constants
- [ ] Define filesystem constants
- [ ] Define download constants
- [ ] Define UI constants
- [ ] Export from `src/lib.rs`
- [ ] Add documentation comments

### 2. Enhance Utils Module

#### `src/utils/validation.rs` (Create or Enhance)

```rust
//! Input validation utilities

use anyhow::Result;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Invalid file path: {0}")]
    InvalidPath(String),
    #[error("Value out of range: {0}")]
    OutOfRange(String),
}

/// Check if string is a valid HTTP(S) URL
pub fn is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Validate and parse URL
pub fn validate_url(s: &str) -> Result<Url, ValidationError> {
    Url::parse(s)
        .map_err(|e| ValidationError::InvalidUrl(e.to_string()))
}

/// Validate file path exists and is readable
pub fn validate_file_path(path: &Path) -> Result<(), ValidationError> {
    if !path.exists() {
        return Err(ValidationError::InvalidPath(
            format!("Path does not exist: {}", path.display())
        ));
    }
    if !path.is_file() {
        return Err(ValidationError::InvalidPath(
            format!("Path is not a file: {}", path.display())
        ));
    }
    Ok(())
}
```

**Action Items:**
- [ ] Create or enhance `src/utils/validation.rs`
- [ ] Add URL validation functions
- [ ] Add path validation functions
- [ ] Add range validation functions
- [ ] Export from `src/utils/mod.rs`
- [ ] Add comprehensive unit tests

#### `src/utils/fs.rs` (Create or Enhance)

**Action Items:**
- [ ] Create or enhance `src/utils/fs.rs`
- [ ] Add `expand_tilde()` function
- [ ] Add path manipulation utilities
- [ ] Add safe file operations
- [ ] Export from `src/utils/mod.rs`
- [ ] Add unit tests

### 3. Improve Error Types

**Action Items:**
- [ ] Review all error types for consistency
- [ ] Ensure proper error context chains
- [ ] Add `anyhow::Context` where missing
- [ ] Document error conditions in doc comments

---

## 📋 Action Plan

### Phase 1: Documentation Cleanup (Priority: High)

#### Step 1: Create Archive Structure

```powershell
# Create archive directories
New-Item -ItemType Directory -Force -Path "docs\archive\fixes"
New-Item -ItemType Directory -Force -Path "docs\archive\implementation_notes"
New-Item -ItemType Directory -Force -Path "docs\archive\summaries"
```

#### Step 2: Move Root-Level Files

```powershell
# Move fix documentation
Move-Item "BUFFER_SCROLLING_FIX.md" "docs\archive\fixes\"
Move-Item "COLON_KEYBINDING_FIX.md" "docs\archive\fixes\"
Move-Item "OPML_AUTOCOMPLETE_FIX.md" "docs\archive\fixes\"

# Move summaries
Move-Item "OPML_BUG_FIX_COMPLETE.md" "docs\archive\summaries\"
Move-Item "OPML_FEATURE_COMPLETE.md" "docs\archive\summaries\"
Move-Item "IMPLEMENTATION_COMPLETE.md" "docs\archive\summaries\"
Move-Item "ISSUE_FIXES_SUMMARY.md" "docs\archive\summaries\"
Move-Item "DOCUMENTATION_UPDATE_SUMMARY.md" "docs\archive\summaries\"
Move-Item "SETUP_COMPLETE.md" "docs\archive\summaries\"
Move-Item "WINDOWS_BUILD_COMPLETE.md" "docs\archive\summaries\"

# Move implementation notes
Move-Item "OPML_IMPROVEMENTS.md" "docs\archive\implementation_notes\"
Move-Item "WHATS_NEW_BUFFER_IMPLEMENTATION.md" "docs\archive\implementation_notes\"
```

#### Step 3: Remove Temporary Files

```powershell
Remove-Item "GIT_COMMIT_INFO.md"
```

#### Step 4: Archive docs/ Files

```powershell
# Move OPML implementation details
Move-Item "docs\OPML_IMPLEMENTATION_SUMMARY.md" "docs\archive\implementation_notes\"
Move-Item "docs\FEATURE-OPML.md" "docs\archive\implementation_notes\"

# Move OPML fixes
Move-Item "docs\OPML_REAL_WORLD_FIX_SUMMARY.md" "docs\archive\fixes\"
Move-Item "docs\OPML_XML_SANITIZATION_FIX.md" "docs\archive\fixes\"
Move-Item "docs\TESTING_OPML_URL_FIX.md" "docs\archive\fixes\"
Move-Item "docs\BUGFIX_OPML_URL_HANDLING.md" "docs\archive\fixes\"
```

#### Step 5: Consolidate Build Documentation

```powershell
# Remove redundant build docs (after verifying content is in docs/BUILD_SYSTEM.md)
Remove-Item "BUILD_COMMANDS.md"
Remove-Item "BUILD_SYSTEM_FINAL.md"
Remove-Item "docs\BUILD_SYSTEM_SUMMARY.md"
```

#### Step 6: Merge Getting Started Docs

- [ ] Manually merge `QUICKSTART.md` into `GETTING_STARTED.md`
- [ ] Remove `QUICKSTART.md` after merge

**Estimated Time**: 2-3 hours

---

### Phase 2: Create Missing Documentation (Priority: High)

#### Step 1: Create ARCHITECTURE.md

- [ ] Create `docs/ARCHITECTURE.md`
- [ ] Add system overview
- [ ] Add architecture diagrams (ASCII or Mermaid)
- [ ] Document data flow
- [ ] Explain design patterns
- [ ] Add module structure
- [ ] Document dependencies rationale
- [ ] Add security considerations
- [ ] Document testing strategy
- [ ] Add performance considerations

**Template**: See Architecture.md template below

**Estimated Time**: 4-6 hours

#### Step 2: Create archive/README.md

- [ ] Create `docs/archive/README.md`
- [ ] Explain purpose of archive
- [ ] Index archived documents
- [ ] Add dates and reasons for archival

**Estimated Time**: 30 minutes

---

### Phase 3: Code Refactoring (Priority: Medium)

#### Step 1: Create Constants Module

- [ ] Create `src/constants.rs`
- [ ] Define network constants
- [ ] Define filesystem constants
- [ ] Define download constants
- [ ] Define UI constants
- [ ] Export from `src/lib.rs`
- [ ] Add documentation

**Estimated Time**: 1-2 hours

#### Step 2: Enhance Utils Module

- [ ] Create/enhance `src/utils/validation.rs`
- [ ] Create/enhance `src/utils/fs.rs`
- [ ] Add `expand_tilde()` function
- [ ] Add URL validation
- [ ] Add path validation
- [ ] Export from `src/utils/mod.rs`

**Estimated Time**: 2-3 hours

#### Step 3: Refactor Using New Utilities

- [ ] Update `src/config.rs` to use constants
- [ ] Update `src/config.rs` to use `expand_tilde()`
- [ ] Update `src/podcast/opml.rs` to use validation utils
- [ ] Update `src/podcast/subscription.rs` to use `expand_tilde()`
- [ ] Update `src/download/manager.rs` to use constants
- [ ] Run tests after each change

**Estimated Time**: 3-4 hours

#### Step 4: Add Tests

- [ ] Add tests for `src/constants.rs` (documentation tests)
- [ ] Add tests for `src/utils/validation.rs`
- [ ] Add tests for `src/utils/fs.rs`
- [ ] Run full test suite

**Estimated Time**: 2-3 hours

---

### Phase 4: Update Existing Documentation (Priority: Medium)

#### Step 1: Update README.md

- [ ] Review current progress percentage
- [ ] Update feature status
- [ ] Add link to `ARCHITECTURE.md`
- [ ] Update build instructions if needed
- [ ] Verify all links work

**Estimated Time**: 1 hour

#### Step 2: Update IMPLEMENTATION_PLAN.md

- [ ] Mark completed sprints
- [ ] Update current sprint status
- [ ] Add Sprint 5 if needed
- [ ] Update progress indicators

**Estimated Time**: 1 hour

#### Step 3: Update CONTRIBUTING.md

- [ ] Add reference to `ARCHITECTURE.md`
- [ ] Update documentation structure info
- [ ] Add code style guidelines
- [ ] Update development workflow

**Estimated Time**: 1-2 hours

#### Step 4: Merge QUICKSTART into GETTING_STARTED

- [ ] Review both documents
- [ ] Merge unique content
- [ ] Ensure all platforms covered
- [ ] Add troubleshooting section
- [ ] Delete `QUICKSTART.md`

**Estimated Time**: 1-2 hours

---

### Phase 5: Add Missing Tests (Priority: Low)

- [ ] Add integration tests for OPML edge cases
- [ ] Add property-based tests for validation
- [ ] Add tests for new utility functions
- [ ] Increase code coverage

**Estimated Time**: 4-6 hours

---

## 📊 Success Metrics

### Documentation
- [ ] All historical docs moved to `docs/archive/`
- [ ] Build documentation consolidated to 2 files
- [ ] OPML documentation consolidated to 1 user-facing file
- [ ] `ARCHITECTURE.md` created with comprehensive coverage
- [ ] All root-level docs serve clear user-facing purpose

### Code Quality
- [ ] No magic numbers in main codebase
- [ ] No code duplication for path expansion
- [ ] Centralized validation logic
- [ ] All new utilities have tests
- [ ] `cargo clippy` passes with no warnings

### Maintainability
- [ ] Clear documentation structure
- [ ] Easy onboarding for new contributors
- [ ] Reduced cognitive load
- [ ] Better adherence to Rust idioms

---

## 🔄 Review Checklist

Before considering this cleanup complete, verify:

- [ ] All tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Code formatted: `cargo fmt`
- [ ] Documentation builds: `cargo doc`
- [ ] README accurate and up-to-date
- [ ] CHANGELOG updated with cleanup notes
- [ ] All internal links in docs work
- [ ] Archive has README explaining contents

---

## 📝 Architecture.md Template

```markdown
# Podcast TUI Architecture

## Overview

[High-level description of the application]

## Architecture Diagram

[ASCII or Mermaid diagram showing layers]

## Core Principles

### 1. Storage Abstraction
[Explanation]

### 2. Event-Driven UI
[Explanation]

### 3. Async-First Design
[Explanation]

### 4. Buffer-Based UI
[Explanation]

## Module Structure

### Core Modules
[List and describe each module]

### Data Flow
[Explain how data flows through the system]

## Key Design Patterns

[List and explain patterns used]

## Dependencies

[Rationale for major dependencies]

## Testing Strategy

[Explain testing approach]

## Performance Considerations

[Document performance decisions]

## Security

[Document security measures]

## Future Architecture Changes

[Planned improvements]
```

---

## 📅 Timeline Estimate

| Phase | Estimated Time | Priority |
|-------|---------------|----------|
| Phase 1: Documentation Cleanup | 2-3 hours | High |
| Phase 2: Create Missing Docs | 4-6 hours | High |
| Phase 3: Code Refactoring | 8-12 hours | Medium |
| Phase 4: Update Existing Docs | 4-6 hours | Medium |
| Phase 5: Add Missing Tests | 4-6 hours | Low |
| **Total** | **22-33 hours** | |

---

## 🎯 Next Steps

1. Review and approve this cleanup plan
2. Create GitHub issues for tracking (optional)
3. Begin Phase 1: Documentation Cleanup
4. Commit changes incrementally with clear messages
5. Update CHANGELOG.md as you progress

---

## 📞 Questions or Concerns?

If you have questions about any part of this cleanup plan, please:
1. Review the `.github/copilot-instructions.md`
2. Check existing architecture in `docs/STORAGE_DESIGN.md` and `docs/PRD.md`
3. Ask for clarification before making major changes

---

**Last Updated**: October 7, 2025  
**Document Status**: ✅ Ready for Execution
