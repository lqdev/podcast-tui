# Feature Implementation: OPML Import/Export Support

## Objective

Implement comprehensive OPML (Outline Processor Markup Language) import and export functionality for podcast subscriptions, enabling users to:

- Import podcast feeds from OPML files or URLs (non-destructive - skip duplicates)
- Export all subscriptions to OPML format with configurable location
- Use simple keyboard shortcuts and commands for both operations
- Track progress and errors with detailed feedback

---

## 1. OPML Import Functionality

### Goal

Allow users to import an OPML file (local file path or URL) containing RSS feed URLs and add them to their podcast list.

### Key Requirements

#### Input Handling

- Accept both local file paths and HTTP(S) URLs via minibuffer
- Download OPML from URL if provided (using existing HTTP client patterns)
- Validate OPML format before attempting to parse

#### OPML Validation

- Check XML structure is valid
- Verify root element is `<opml>` with version attribute
- Confirm presence of `<body>` element
- Provide clear error message if validation fails
- Early validation before attempting full parse

#### Parsing Strategy

- Parse standard OPML 2.0 format (flat structure, no nested categories)
- Extract RSS feed URLs from `<outline>` elements
  - Primary: `xmlUrl` attribute
  - Fallback: `url` attribute
  - Skip outlines without feed URLs
- Extract metadata (text/title) for reference in logs

#### Non-Destructive Import

- Use existing `is_subscribed()` method from `SubscriptionManager`
- Skip feeds that already exist in user's subscriptions
- Track statistics: total, imported, skipped, failed
- **Sequential processing**: Import one feed at a time with progress updates

#### Progress Feedback

Update minibuffer with current status:

- "Validating OPML file..."
- "Importing feed 3 of 10: [Podcast Title]..."
- "Parsing feed: [Podcast Title]..."
- "Skipped (duplicate): [Podcast Title]"
- "Failed: [Podcast Title] - [Error]"
- Show running count of imported/skipped/failed

#### Error Handling

- Log all errors to file: `~/.local/share/podcast-tui/logs/opml-import-YYYY-MM-DD-HHmmss.log`
- Track failed imports with feed URL and error message
- Continue processing remaining feeds if one fails
- Show detailed error summary at end:

```text
Import complete:
- Total feeds: 15
- Imported: 10
- Skipped (duplicates): 3
- Failed: 2

Failed imports:
- https://example.com/feed1.xml: Network timeout
- https://example.com/feed2.xml: Invalid feed format

See log: ~/.local/share/podcast-tui/logs/opml-import-2025-10-06-143022.log
```

#### Error Scenarios to Handle

- Invalid OPML file format / XML parse error
- File not found / permission errors
- Network errors when downloading from URL
- Network errors when fetching individual feeds
- Feed parsing failures (malformed RSS/Atom)
- Storage errors during save

---

## 2. OPML Export Functionality

### Goal

Export all current podcast subscriptions to a standard OPML file at a configurable location.

### Key Requirements

#### Configuration

Add to `StorageConfig` in `src/config.rs`:

```rust
pub struct StorageConfig {
    // ... existing fields ...
    #[serde(default = "default_opml_export_directory")]
    pub opml_export_directory: String,
}

fn default_opml_export_directory() -> String {
    "~/Documents/podcast-exports".to_string()
}
```

#### Export Location Logic

- Default: Use `config.storage.opml_export_directory`
- Allow override: User can specify custom path via minibuffer
- Minibuffer prompt: "Export to (default: [configured path]): "
- If user enters nothing, use default
- If user enters path, use that path
- Expand `~` to user home directory

#### Filename Generation

- Default filename format: `podcasts-export-YYYY-MM-DD-HHmmss.opml`
- Example: `podcasts-export-2025-10-06-143022.opml`
- If user provides directory, append default filename
- If user provides full path with filename, use as-is
- Ensure `.opml` extension is present

#### OPML Generation

- Generate OPML 2.0 compliant XML structure (flat, no categories)
- Include all subscribed podcasts
- Use standard OPML attributes:
  - `xmlUrl`: RSS feed URL (required)
  - `text`: Podcast title
  - `title`: Podcast title (duplicate for compatibility)
  - `type`: "rss"
  - `description`: Podcast description (if available)

#### OPML Structure

```xml
<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head>
    <title>Podcast Subscriptions</title>
    <dateCreated>Sun, 06 Oct 2025 14:30:22 GMT</dateCreated>
    <docs>http://opml.org/spec2.opml</docs>
  </head>
  <body>
    <outline type="rss" 
             text="Example Podcast" 
             title="Example Podcast"
             description="A great podcast about things"
             xmlUrl="https://example.com/feed.xml"/>
    <!-- More feeds... -->
  </body>
</opml>
```

#### File Writing

- Create parent directory if it doesn't exist
- Use atomic write pattern (temp file + rename)
- Handle permission errors gracefully
- Show success message with full path

#### Progress Feedback

Update minibuffer:

- "Loading subscriptions..."
- "Generating OPML (15 feeds)..."
- "Writing to file..."
- "Exported 15 feeds to /path/to/podcasts-export-2025-10-06-143022.opml"

#### Error Handling

- Directory creation failures
- Permission errors
- Disk space issues
- Invalid path errors
- Show clear error message if export fails

---

## 3. Module Structure

### File Organization

**Create new module**: `src/podcast/opml.rs`

#### Module Components

```rust
use crate::podcast::{Podcast, SubscriptionManager, SubscriptionError};
use crate::storage::{PodcastId, Storage};
use anyhow::Result;
use std::path::Path;
use quick_xml;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// OPML parser for importing podcast subscriptions
pub struct OpmlParser {
    client: reqwest::Client,
}

impl OpmlParser {
    /// Create new OPML parser
    pub fn new() -> Self;
    
    /// Parse OPML from file path or URL
    /// Returns parsed OPML document or validation error
    pub async fn parse(&self, source: &str) -> Result<OpmlDocument, OpmlError>;
    
    /// Validate OPML structure before parsing
    fn validate_opml(xml: &str) -> Result<(), OpmlError>;
    
    /// Download OPML from URL
    async fn download_opml(&self, url: &str) -> Result<String, OpmlError>;
}

/// OPML exporter for podcast subscriptions
pub struct OpmlExporter;

impl OpmlExporter {
    /// Create new OPML exporter
    pub fn new() -> Self;
    
    /// Export podcasts to OPML file
    pub async fn export(
        &self, 
        podcasts: &[Podcast], 
        path: &Path
    ) -> Result<(), OpmlError>;
    
    /// Generate OPML XML from podcast list
    fn generate_opml(&self, podcasts: &[Podcast]) -> Result<String, OpmlError>;
}

/// Parsed OPML document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpmlDocument {
    pub version: String,
    pub head: OpmlHead,
    pub outlines: Vec<OpmlOutline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpmlHead {
    pub title: Option<String>,
    pub date_created: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpmlOutline {
    pub text: String,
    pub title: Option<String>,
    pub xml_url: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub outline_type: Option<String>,
}

/// Result of OPML import operation
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub total_feeds: usize,
    pub imported: usize,
    pub skipped: usize,
    pub failed: Vec<FailedImport>,
}

#[derive(Debug, Clone)]
pub struct FailedImport {
    pub url: String,
    pub title: Option<String>,
    pub error: String,
}

impl ImportResult {
    /// Format summary for display
    pub fn summary(&self) -> String;
    
    /// Check if any imports failed
    pub fn has_failures(&self) -> bool;
}

/// OPML operation errors
#[derive(Debug, thiserror::Error)]
pub enum OpmlError {
    #[error("Failed to read OPML file: {0}")]
    FileRead(#[from] std::io::Error),
    
    #[error("Failed to download OPML: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Invalid OPML format: {0}")]
    InvalidFormat(String),
    
    #[error("Failed to parse OPML XML: {0}")]
    ParseError(String),
    
    #[error("OPML validation failed: {0}")]
    ValidationError(String),
    
    #[error("No feeds found in OPML file")]
    NoFeeds,
    
    #[error("Failed to create directory: {0}")]
    DirectoryCreation(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
```

#### Add to Module Exports

Add to `src/podcast/mod.rs`:

```rust
pub mod opml;
pub use opml::{OpmlParser, OpmlExporter, OpmlError, OpmlDocument, ImportResult};
```

---

## 4. Integration with SubscriptionManager

### New Methods

Add to `src/podcast/subscription.rs`:

```rust
impl<S: Storage> SubscriptionManager<S> {
    /// Import podcasts from OPML file or URL
    /// Returns detailed statistics about the import
    pub async fn import_opml(
        &self,
        source: &str,
        progress_callback: impl Fn(String) + Send + Sync,
    ) -> Result<ImportResult, SubscriptionError> {
        // 1. Parse and validate OPML
        // 2. Extract feed URLs
        // 3. Import each feed sequentially
        // 4. Call progress_callback for each step
        // 5. Track statistics and failures
        // 6. Write log file
        // 7. Return ImportResult
    }
    
    /// Export all subscriptions to OPML file
    pub async fn export_opml(
        &self,
        output_path: &Path,
        progress_callback: impl Fn(String) + Send + Sync,
    ) -> Result<usize, SubscriptionError> {
        // 1. Load all podcasts
        // 2. Generate OPML
        // 3. Write to file (atomic)
        // 4. Call progress_callback for each step
        // 5. Return count of exported feeds
    }
}
```

#### Add to SubscriptionError enum

```rust
#[error("OPML error: {0}")]
Opml(#[from] OpmlError),
```

### Logging Implementation

#### Log File Structure

- Location: `~/.local/share/podcast-tui/logs/`
- Filename: `opml-import-YYYY-MM-DD-HHmmss.log`
- Content format:

```text
OPML Import Log
Started: 2025-10-06 14:30:22
Source: /path/to/subscriptions.opml

=== Processing ===
[14:30:23] Validating OPML...
[14:30:23] Found 15 feeds
[14:30:24] [1/15] Importing: Example Podcast (https://example.com/feed.xml)
[14:30:25] [1/15] ✓ Success
[14:30:26] [2/15] Importing: Another Podcast (https://another.com/feed.xml)
[14:30:26] [2/15] ⊘ Skipped (already subscribed)
[14:30:27] [3/15] Importing: Failed Podcast (https://broken.com/feed.xml)
[14:30:29] [3/15] ✗ Failed: Network timeout after 30s
...

=== Summary ===
Completed: 2025-10-06 14:31:05
Duration: 43 seconds
Total feeds: 15
Imported: 10
Skipped: 3
Failed: 2

=== Failed Imports ===
1. Failed Podcast (https://broken.com/feed.xml)
   Error: Network timeout after 30s

2. Invalid Podcast (https://invalid.com/feed.xml)
   Error: Failed to parse RSS feed: Invalid XML
```

---

## 5. UI Integration

### Update UIAction Enum

Add to `src/ui/mod.rs`:

```rust
pub enum UIAction {
    // ... existing actions ...
    
    /// Import podcasts from OPML file or URL
    ImportOpml,
    
    /// Export subscriptions to OPML file
    ExportOpml,
    
    /// Trigger async OPML import with source path
    TriggerOpmlImport { source: String },
    
    /// Trigger async OPML export with output path
    TriggerOpmlExport { path: Option<String> },
    
    // ... rest of actions ...
}
```

### Update AppEvent Enum

Add to `src/ui/events.rs`:

```rust
pub enum AppEvent {
    // ... existing events ...
    
    /// OPML import started
    OpmlImportStarted {
        source: String,
    },
    
    /// OPML import progress update
    OpmlImportProgress {
        current: usize,
        total: usize,
        status: String,
    },
    
    /// OPML import completed
    OpmlImportCompleted {
        result: ImportResult,
        log_path: String,
    },
    
    /// OPML import failed
    OpmlImportFailed {
        source: String,
        error: String,
    },
    
    /// OPML export started
    OpmlExportStarted {
        path: String,
    },
    
    /// OPML export progress update
    OpmlExportProgress {
        status: String,
    },
    
    /// OPML export completed
    OpmlExportCompleted {
        path: String,
        feed_count: usize,
    },
    
    /// OPML export failed
    OpmlExportFailed {
        path: String,
        error: String,
    },
}
```

### Keybindings

Add to `src/ui/keybindings.rs`:

```rust
fn setup_default_bindings(&mut self) {
    // ... existing bindings ...
    
    // OPML Import/Export
    self.bind_key(
        KeyChord::shift(KeyCode::Char('A')), 
        UIAction::ImportOpml
    );
    self.bind_key(
        KeyChord::shift(KeyCode::Char('E')), 
        UIAction::ExportOpml
    );
}
```

### Command System

Add commands (in app state handler):

#### `import-opml` command

1. Show minibuffer prompt: "Import OPML from (file path or URL): "
2. User enters path or URL
3. Validate input (not empty)
4. Trigger `UIAction::TriggerOpmlImport { source }`
5. Handle async import in background
6. Update minibuffer with progress via `AppEvent::OpmlImportProgress`
7. Show final summary via `AppEvent::OpmlImportCompleted`
8. If failures, show detailed error list

#### `export-opml` command

1. Load default path from config: `config.storage.opml_export_directory`
2. Show minibuffer prompt: "Export to (default: {default_path}): "
3. User can press Enter (use default) or enter custom path
4. If custom path, validate it
5. Generate timestamped filename if directory provided
6. Trigger `UIAction::TriggerOpmlExport { path }`
7. Handle async export in background
8. Update minibuffer with progress via `AppEvent::OpmlExportProgress`
9. Show success message with full path via `AppEvent::OpmlExportCompleted`

### Minibuffer Updates

#### Progress Display Pattern

```rust
// During import
app.update_minibuffer("Validating OPML file...");
app.update_minibuffer("Found 15 feeds in OPML");
app.update_minibuffer("Importing [3/15]: Example Podcast...");
app.update_minibuffer("Skipped [4/15]: Duplicate Podcast (already subscribed)");
app.update_minibuffer("Failed [5/15]: Broken Podcast - Network timeout");

// Final summary
app.show_message(format!(
    "Import complete: {} imported, {} skipped, {} failed. See log: {}",
    result.imported, result.skipped, result.failed.len(), log_path
));

// During export
app.update_minibuffer("Loading subscriptions...");
app.update_minibuffer("Generating OPML (15 feeds)...");
app.update_minibuffer("Writing to file...");
app.show_message(format!(
    "Exported {} feeds to {}",
    feed_count, path
));
```

---

## 6. Dependencies

### Add to Cargo.toml

```toml
[dependencies]
# Existing dependencies...

# For OPML XML parsing/generation
quick-xml = { version = "0.31", features = ["serialize"] }

# For logging (if not already present)
chrono = { version = "0.4", features = ["serde"] }

# HTTP client (likely already present for feed fetching)
reqwest = { version = "0.11", features = ["json"] }
```

---

## 7. Implementation Phases

### Phase 1: Core OPML Module (2-3 hours)

1. Create `src/podcast/opml.rs` with structs and error types
2. Implement `OpmlParser`:
   - XML validation logic
   - Parse OPML structure with quick-xml
   - Download from URL support
3. Implement `OpmlExporter`:
   - Generate OPML XML
   - Format with proper structure
4. Write unit tests:
   - Valid OPML parsing
   - Invalid OPML rejection
   - URL vs file path handling
   - Export generation
   - Round-trip test (export → import)

### Phase 2: SubscriptionManager Integration (2-3 hours)

1. Add `import_opml()` method:
   - Sequential feed processing
   - Progress callbacks
   - Duplicate detection with `is_subscribed()`
   - Error tracking and logging
   - Generate log file
2. Add `export_opml()` method:
   - Load all subscriptions
   - Generate OPML
   - Atomic file write
   - Progress callbacks
3. Add configuration for export directory
4. Write integration tests:
   - Import with duplicates
   - Import with failures
   - Export all subscriptions
   - Export with empty list

### Phase 3: UI Integration (2-3 hours)

1. Add `UIAction` variants for import/export
2. Add `AppEvent` variants for progress and completion
3. Implement keybindings (Shift+A, Shift+E)
4. Add command handlers:
   - `import-opml` with minibuffer prompt
   - `export-opml` with minibuffer prompt and config default
5. Implement async task spawning for import/export
6. Wire up progress updates to minibuffer
7. Implement final summary display
8. Handle error display with details

### Phase 4: Polish & Testing (1-2 hours)

1. Test with real-world OPML files
2. Test error scenarios:
   - Invalid OPML files
   - Network failures
   - Permission errors
   - Malformed feeds in OPML
3. Refine progress messages
4. Refine error messages
5. Update documentation

---

## 8. Testing Strategy

### Unit Tests

#### OPML Parsing

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_parse_valid_opml() {
        // Test with valid OPML 2.0 file
    }
    
    #[tokio::test]
    async fn test_parse_invalid_xml() {
        // Should return ValidationError
    }
    
    #[tokio::test]
    async fn test_parse_missing_xmlurl() {
        // Should skip outlines without xmlUrl
    }
    
    #[tokio::test]
    async fn test_download_from_url() {
        // Mock HTTP request
    }
    
    #[test]
    fn test_validate_opml_structure() {
        // Test validation logic
    }
}
```

#### OPML Export

```rust
#[test]
fn test_generate_opml() {
    // Generate OPML from podcast list
    // Verify XML structure
}

#[test]
fn test_export_empty_list() {
    // Should generate valid empty OPML
}

#[test]
fn test_round_trip() {
    // Export → Import → Compare
}
```

### Integration Tests

#### Import Scenarios

- Import valid OPML with 10 feeds
- Import OPML with duplicates (should skip)
- Import OPML with some invalid feeds (should continue)
- Import from URL vs local file
- Import empty OPML

#### Export Scenarios

- Export subscriptions to default location
- Export to custom directory
- Export to custom filename
- Export with no subscriptions
- Export with many subscriptions (100+)

#### Error Scenarios

- Invalid OPML format
- Network timeout during URL download
- Network timeout during feed fetch
- File permission errors
- Invalid output path
- Disk space issues (harder to test)

### Manual Testing Checklist

- [ ] Press Shift+A, enter file path, verify import
- [ ] Press Shift+A, enter URL, verify download and import
- [ ] Import OPML with duplicates, verify skipping
- [ ] Import OPML with broken feeds, verify error logging
- [ ] Check log file contains detailed error information
- [ ] Press Shift+E with default config, verify export
- [ ] Press Shift+E, override path, verify custom location
- [ ] Verify exported OPML can be imported into other apps (Apple Podcasts, Overcast, etc.)
- [ ] Test `:import-opml` command
- [ ] Test `:export-opml` command
- [ ] Verify minibuffer shows progress during operations
- [ ] Verify error messages are clear and actionable
- [ ] Test with large OPML files (50+ feeds)
- [ ] Test with invalid OPML files
- [ ] Test error summary display

---

## 9. Documentation Updates

### Update Files

**README.md**:

- Add OPML import/export to feature list
- Add quick example in "Quick Start" section

**KEYBINDINGS.md** (or docs/EMACS_KEYBINDINGS.md):

- Document Shift+A for import
- Document Shift+E for export
- Document `:import-opml` command
- Document `:export-opml` command

**GETTING_STARTED.md**:

- Add section on importing existing subscriptions
- Add section on exporting for backup/portability

**CHANGELOG.md**:

- Add new feature entry with details

**Config Example**:

- Document `opml_export_directory` setting

### Code Documentation

- Add module-level documentation to `opml.rs`
- Document all public functions with examples
- Explain OPML validation logic
- Document error handling strategy
- Add examples of progress callbacks

---

## 10. Success Criteria

### Functional Requirements

- ✅ User can press Shift+A to import OPML from file or URL
- ✅ User can press Shift+E to export to configured or custom location
- ✅ User can use `:import-opml` and `:export-opml` commands
- ✅ OPML files are validated before parsing
- ✅ Duplicate feeds are automatically skipped during import
- ✅ Import processes feeds sequentially with progress updates
- ✅ Minibuffer shows current progress during operations
- ✅ Failed imports are logged to file with detailed errors
- ✅ Import shows final summary with statistics
- ✅ Export uses timestamped filenames by default
- ✅ Export location is configurable in config.json
- ✅ User can override export location via minibuffer
- ✅ Exported OPML files are valid OPML 2.0 format
- ✅ Exported OPML can be imported into other podcast apps

### Non-Functional Requirements

- ✅ All operations handle errors gracefully without crashes
- ✅ Clear, actionable error messages for all failure modes
- ✅ Progress feedback is smooth and informative
- ✅ Large OPML files (100+ feeds) import successfully
- ✅ Network timeouts don't hang the UI
- ✅ Log files are created in appropriate location
- ✅ Atomic file writes prevent corruption
- ✅ Code follows project architecture guidelines
- ✅ Comprehensive tests cover happy path and errors
- ✅ Documentation is complete and accurate

---

## 11. Architecture Guidelines

### Follow Existing Patterns

#### Storage Abstraction

- Don't access storage directly
- Use `SubscriptionManager` methods
- Code against `Storage` trait

#### Async Operations

- Spawn background tasks for long operations
- Use progress callbacks to update UI
- Don't block the UI thread
- Handle task cancellation gracefully

#### Error Handling

- Use `thiserror` for error types
- Provide context with error messages
- Chain errors appropriately
- User-facing errors should be actionable

#### Event-Driven UI

- Use `AppEvent` for async results
- Update minibuffer via events
- Keep UI responsive during operations
- Show progress, not just start/end states

#### Resource Cleanup

- Use RAII patterns
- Close files/connections properly
- Clean up temp files on error
- Use atomic operations for data safety

### Code Style

- Follow Rust conventions (rustfmt, clippy)
- Write comprehensive doc comments
- Add usage examples to complex functions
- Test error conditions thoroughly
- Keep functions focused and small
- Prefer explicit over implicit
- Handle edge cases gracefully

---

## 12. Implementation Notes

### OPML Validation Details

Check for these required elements:

1. Root `<opml>` element with `version` attribute
2. `<head>` element (can be empty)
3. `<body>` element
4. At least one `<outline>` element
5. Each outline should have `xmlUrl` or `url` attribute

### URL Detection

Distinguish between file path and URL:

```rust
fn is_url(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}
```

### Path Expansion

Expand `~` to home directory:

```rust
fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}
```

### Atomic File Write Pattern

```rust
async fn write_atomic(path: &Path, content: &str) -> Result<()> {
    let temp_path = path.with_extension("tmp");
    tokio::fs::write(&temp_path, content).await?;
    tokio::fs::rename(temp_path, path).await?;
    Ok(())
}
```

### Progress Callback Pattern

```rust
let progress_callback = |msg: String| {
    // Send event to UI
    let _ = event_tx.send(AppEvent::OpmlImportProgress {
        status: msg,
        ...
    });
};

subscription_manager.import_opml(source, progress_callback).await?;
```

---

## 13. Known Limitations & Future Enhancements

### MVP Limitations

- Flat OPML structure only (no nested categories)
- Sequential import (not parallel)
- Basic progress feedback
- Log files not automatically cleaned up
- No import history tracking
- No OPML auto-sync functionality

### Future Enhancements (Post-MVP)

- Support nested OPML categories/folders
- Parallel import with concurrency limit
- Import preview before committing
- OPML auto-export on schedule
- Cloud sync integration
- Import from common podcast services (Apple Podcasts, Spotify)
- OPML file picker dialog (native file browser)
- Progress bar visualization
- Undo failed imports
- Import diff (show what will change)

---

## 14. Example Usage Flows

### Import Flow

```text
User: Shift+A
App: "Import OPML from (file path or URL): "
User: ~/podcasts.opml
App: "Validating OPML file..."
App: "Found 15 feeds in OPML"
App: "Importing [1/15]: Tech Podcast..."
App: "✓ Imported [1/15]: Tech Podcast"
App: "Importing [2/15]: News Podcast..."
App: "⊘ Skipped [2/15]: News Podcast (already subscribed)"
...
App: "Import complete: 10 imported, 3 skipped, 2 failed"
App: "Failed imports: see log at ~/.local/share/podcast-tui/logs/opml-import-2025-10-06-143022.log"
```

### Export Flow

```text
User: Shift+E
App: "Export to (default: ~/Documents/podcast-exports): "
User: [Enter] (uses default)
App: "Loading subscriptions..."
App: "Generating OPML (15 feeds)..."
App: "Writing to file..."
App: "Exported 15 feeds to ~/Documents/podcast-exports/podcasts-export-2025-10-06-143022.opml"
```

### Command Flow

```text
User: : (colon to enter command)
App: Shows command prompt
User: import-opml
App: "Import OPML from (file path or URL): "
[Same as Shift+A flow]
```

---

## 15. Requirements Summary

### Confirmed Requirements

- ✅ **File Selection**: Text input in minibuffer
- ✅ **Import Strategy**: Sequential processing
- ✅ **Failed Imports**: Log to file + detailed error list at end
- ✅ **Export Location**: Configurable default + minibuffer override
- ✅ **OPML Categories**: Flat structure only
- ✅ **Progress Feedback**: Update minibuffer with status messages
- ✅ **Default Filename**: Timestamped format
- ✅ **OPML Validation**: Yes, validate before parsing

### Key Features

- Non-destructive import (skip duplicates)
- Sequential feed processing with progress
- Detailed error logging and reporting
- Configurable export location with override
- Timestamped export filenames
- OPML validation before parsing
- Support for both file paths and URLs
- Atomic file writes for data safety

---

This document serves as the complete specification for implementing OPML import/export functionality in the podcast-tui application. Follow the implementation phases sequentially and refer to the architecture guidelines to ensure consistency with the existing codebase.
