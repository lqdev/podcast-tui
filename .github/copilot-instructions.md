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

## Issue Workflow

- Check the [GitHub Projects board](https://github.com/users/lqdev/projects/1) for priority and context before starting work
- Create branches tied to issues: `feat/issue-{N}-short-desc`, `fix/issue-{N}-short-desc`
- Reference issues in commits and PRs: `Closes #N` or `Part of #N`
- Read acceptance criteria and implementation notes in the issue body before coding

## What NOT to Do

- ❌ Don't use unwrap() or expect() in user-facing code
- ❌ Don't block the UI thread with synchronous I/O
- ❌ Don't implement storage operations directly in UI components
- ❌ Don't hardcode file paths or configuration values — use `src/constants.rs`
- ❌ Don't ignore errors or fail silently
- ❌ Don't create deeply nested match statements (use helper methods)
- ❌ Don't mix UI concerns with business logic
- ❌ Don't add new buffers without registering them in `BufferManager`
- ❌ Don't hardcode magic numbers — add to `src/constants.rs`

## Documentation

- Write doc comments for public APIs
- Include usage examples in complex functions
- Document error conditions and edge cases
- Update CHANGELOG.md for significant changes
- Git commit format: `type(scope): description` (types: feat, fix, docs, refactor, test, chore)