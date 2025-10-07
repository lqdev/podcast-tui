# GitHub Copilot Instructions for Podcast TUI Development

## Project Overview
This is a cross-platform terminal user interface (TUI) application for podcast management written in Rust. The application uses simple, universal keybindings that work reliably across all terminal emulators and prioritizes MVP delivery over production-scale features.

## Code Style and Architecture Guidelines

### Rust Style
- Follow standard Rust conventions with `rustfmt` and `clippy`
- Use `snake_case` for functions, variables, and modules
- Use `PascalCase` for types and structs
- Prefer explicit error handling with `Result<T, E>` over panicking
- Use `async/await` for I/O operations
- Implement proper resource cleanup with RAII patterns
- Write comprehensive error messages for user-facing errors

### Architecture Patterns
- **Storage Abstraction**: Always code against the `Storage` trait, never directly against JSON implementation
- **Component Separation**: Maintain clear separation between UI, business logic, and data persistence
- **Event-Driven**: Use event-driven patterns for UI updates and user interactions
- **Buffer-Based UI**: Use buffers, windows, and minibuffer patterns for organizing different views
- **Async-First**: Design for async operations, especially for network I/O and file operations

### Error Handling
```rust
// Prefer custom error types
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
let podcast = storage.load_podcast(&id)
    .await
    .with_context(|| format!("Failed to load podcast with id: {}", id))?;
```

### Testing Patterns
- Write unit tests for business logic
- Use mock implementations of storage trait for testing
- Test error conditions and edge cases
- Create integration tests for key user workflows
- Mock network requests in tests

## Specific Implementation Guidelines

### UI Components (Ratatui)
- Create reusable components that encapsulate rendering and event handling
- Use proper focus management for keyboard navigation
- Implement responsive layouts that adapt to terminal size
- Always provide keyboard navigation alternatives to mouse actions

```rust
// Example component structure
pub struct EpisodeList {
    episodes: Vec<Episode>,
    selected: Option<usize>,
    scroll_offset: usize,
}

impl EpisodeList {
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.select_next();
                Some(Action::Render)
            }
            // ... other keybindings
        }
    }
    
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Rendering logic
    }
}
```

### Storage Implementation
- Always implement against the Storage trait
- Use proper serialization with serde for JSON
- Handle file I/O errors gracefully
- Implement atomic writes for data consistency
- Provide meaningful progress feedback for long operations

```rust
#[async_trait]
impl Storage for JsonStorage {
    type Error = JsonStorageError;
    
    async fn save_podcast(&self, podcast: &Podcast) -> Result<(), Self::Error> {
        let path = self.podcast_path(&podcast.id);
        let json = serde_json::to_string_pretty(podcast)?;
        
        // Atomic write pattern
        let temp_path = format!("{}.tmp", path.display());
        tokio::fs::write(&temp_path, json).await?;
        tokio::fs::rename(temp_path, path).await?;
        
        Ok(())
    }
}
```

### Network Operations
- Use connection pooling with reqwest
- Implement proper timeouts and retry logic
- Handle HTTP errors gracefully
- Respect rate limiting and server resources
- Provide progress feedback for downloads

```rust
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub async fn download_episode(&self, url: &str, path: &Path, 
                                  progress_tx: mpsc::Sender<DownloadProgress>) 
                                  -> Result<(), NetworkError> {
        let response = self.client.get(url).send().await?;
        let total_size = response.content_length();
        
        let mut file = tokio::fs::File::create(path).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded = 0u64;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            
            let _ = progress_tx.send(DownloadProgress {
                downloaded,
                total: total_size,
            }).await;
        }
        
        Ok(())
    }
}
```

### Audio Integration
- Use rodio for cross-platform audio playback
- Implement proper resource management for audio streams
- Handle audio format variations gracefully
- Provide fallback to external players when needed

## Common Patterns to Follow

### Configuration Management
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub audio: AudioConfig,
    pub downloads: DownloadConfig,
    pub keybindings: KeybindingConfig,
    pub storage: StorageConfig,
}

impl Config {
    pub fn load_or_default() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            let default_config = Self::default();
            default_config.save(&config_path)?;
            Ok(default_config)
        }
    }
}
```

### Event System
```rust
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Download(DownloadEvent),
    Playback(PlaybackEvent),
    Storage(StorageEvent),
    Quit,
}

pub struct EventHandler {
    tx: mpsc::UnboundedSender<AppEvent>,
    rx: mpsc::UnboundedReceiver<AppEvent>,
}
```

## What NOT to Do

- ❌ Don't use unwrap() or expect() in user-facing code
- ❌ Don't block the UI thread with synchronous I/O
- ❌ Don't implement storage operations directly in UI components
- ❌ Don't hardcode file paths or configuration values
- ❌ Don't ignore errors or fail silently
- ❌ Don't create deeply nested match statements (use helper methods)
- ❌ Don't mix UI concerns with business logic

## MVP Focus Reminders

- Prioritize working functionality over perfect architecture
- Keep features simple and focused
- Don't over-engineer for future requirements
- Focus on user experience over performance optimization
- Build incrementally with working checkpoints
- Document decisions and trade-offs made for MVP

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    
    #[tokio::test]
    async fn test_podcast_subscription() {
        let mut mock_storage = MockStorage::new();
        mock_storage
            .expect_save_podcast()
            .with(predicate::always())
            .times(1)
            .returning(|_| Ok(()));
            
        let manager = PodcastManager::new(Arc::new(mock_storage));
        let result = manager.subscribe("https://example.com/feed.xml").await;
        
        assert!(result.is_ok());
    }
}
```

## Documentation Expectations

- Write doc comments for public APIs
- Include usage examples in complex functions
- Document error conditions and edge cases
- Keep README updated with build and usage instructions
- Update CHANGELOG.md for significant changes

## Progress Tracking and Documentation

### Tracking Development Progress

**Use CHANGELOG.md as the Primary Record**
- ✅ Document all significant changes in `CHANGELOG.md` using Keep a Changelog format
- ✅ Group changes by category: Added, Changed, Deprecated, Removed, Fixed, Security
- ✅ Include dates and context for major milestones
- ✅ Keep it user-facing and concise

**Avoid Root-Level Progress Documents**
- ❌ Don't create progress tracking files in root directory (creates clutter)
- ❌ Don't create `*_COMPLETE.md`, `*_PROGRESS.md`, or `*_SUMMARY.md` files in root
- ✅ If detailed progress tracking is needed, create in `docs/inprogress/` while actively working
- ✅ Move completed initiative docs from `docs/inprogress/` to `docs/archive/` when work is completed.

**For Multi-Phase Initiatives**
```
1. Plan: Document plan in docs/ (e.g., PROJECT_CLEANUP.md)
2. Track: Create detailed progress docs in docs/inprogress/ if needed (NOT in root!)
3. Work: Use git commits with clear messages for day-to-day progress
4. Complete: Update CHANGELOG.md with comprehensive summary
5. Archive: Move docs/inprogress/ files to docs/archive/{initiative}/ when done
6. Clean: Keep root directory focused on current, actionable docs only
```

### Documentation File Lifecycle

**Active Documentation** (Root or docs/)
- `README.md` - Project overview (keep current)
- `GETTING_STARTED.md` - Quick start guide (keep current)
- `CONTRIBUTING.md` - Contribution guidelines (keep current)
- `CHANGELOG.md` - Change history (keep current, grows over time)
- `docs/ARCHITECTURE.md` - System design (keep current)
- `docs/TESTING.md` - Testing strategy (keep current)
- `docs/IMPLEMENTATION_PLAN.md` - Sprint roadmap (keep current)

**In-Progress Documentation** (docs/inprogress/)
- Active sprint/initiative progress tracking (temporary location)
- Detailed multi-phase work documentation while in progress
- Move to archive when work completes

**Archived Documentation** (docs/archive/)
- Historical bug fixes → `docs/archive/fixes/`
- Implementation notes → `docs/archive/implementation_notes/`
- Completion summaries → `docs/archive/summaries/`
- Completed initiatives → `docs/archive/{initiative-name}/`

### When to Use docs/inprogress/

Use `docs/inprogress/` for active work that needs detailed tracking:
- Multi-phase initiatives with multiple completion milestones
- Complex refactoring or cleanup projects
- Sprint work that needs more documentation than just git commits
- Anything that would otherwise clutter the root directory

### When to Archive Documentation

Move from `docs/inprogress/` to `docs/archive/` immediately when:
- Feature/sprint is complete and announced in CHANGELOG
- Bug fix is resolved and documented in CHANGELOG
- Initiative/cleanup is finished
- Work is no longer active and document becomes historical reference

**Documentation Structure Example**:
```
docs/
├── inprogress/                  # Active work (temporary)
│   ├── sprint-4/               # Current sprint detailed tracking
│   │   ├── README.md           # Sprint overview
│   │   └── PROGRESS.md         # Daily/weekly progress notes
│   └── refactor-audio/         # Active multi-phase initiative
│       └── PHASE1.md
│
└── archive/                     # Completed work (permanent)
    ├── README.md               # Index of all archived docs
    ├── cleanup/                # Completed initiative (moved from inprogress)
    │   ├── README.md          # Summary of initiative
    │   └── *.md               # Detailed tracking docs
    ├── fixes/
    │   └── {ISSUE}_FIX.md
    ├── implementation_notes/
    │   └── {FEATURE}_NOTES.md
    └── summaries/
        └── {SPRINT}_COMPLETE.md
```

### Git Commit Best Practices

**Commit Message Format**
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
chore: archive cleanup progress documents
```

### CHANGELOG.md Format

Follow [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
## [Unreleased] - Sprint X Complete

### Added
- **Feature Name**: Brief description
  - Sub-feature 1 with details
  - Sub-feature 2 with details
  - Integration with existing features

### Changed
- **Component**: What changed and why
  - Impact on users or developers
  - Migration notes if breaking

### Fixed
- **Bug**: What was fixed
  - Root cause (optional)
  - Impact (optional)

### Removed
- **Old Feature**: What was removed and why
  - Migration path if applicable
```

### Documentation Anti-Patterns

**❌ Don't Do This**:
```
Root directory with:
- FEATURE_COMPLETE.md
- BUGFIX_SUMMARY.md  
- PHASE1_DONE.md
- SPRINT_RETROSPECTIVE.md
- PROJECT_STATUS.md
- IMPLEMENTATION_NOTES.md
```

**✅ Do This Instead**:
```
Root directory with:
- README.md (current)
- CHANGELOG.md (comprehensive history)
- CONTRIBUTING.md (current guidelines)

docs/inprogress/ while actively working:
- sprint-4/PROGRESS.md
- feature-x/IMPLEMENTATION.md

docs/archive/ when work completes:
- cleanup/README.md (moved from inprogress)
- cleanup/PHASE1_COMPLETE.md
- cleanup/PHASE2_COMPLETE.md
```

### Rationale

The project experienced "organic growth clutter" where progress tracking documents accumulated in the root directory. This made navigation difficult and created the very problem they were meant to solve.

**Key Principles**:
1. **CHANGELOG.md is the source of truth** for what happened
2. **Root directory stays clean** with only essential, current documentation
3. **Use docs/inprogress/ for active detailed tracking** - not root, not archive yet
4. **Archive immediately** after completion - move from inprogress to archive
5. **Git history preserves everything** - don't fear archiving
6. **Documentation has a lifecycle** - inprogress → complete → archive

This project emphasizes rapid MVP development while maintaining code quality, extensibility, and a **clean, navigable documentation structure**.