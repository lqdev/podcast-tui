---
applyTo: "**/*.rs"
---

# Rust Code Conventions — podcast-tui

## Error Handling

Always use `thiserror` for domain errors, `anyhow` for application-level propagation:

```rust
// Domain error: use thiserror
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Podcast not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Application code: propagate with context
let data = storage.load_podcast(&id)
    .await
    .with_context(|| format!("Failed to load podcast: {id}"))?;
```

**Never** use `unwrap()` or `expect()` in production code paths without a comment proving it's infallible.

## Constants

All hardcoded strings, thresholds, and magic numbers must live in `src/constants.rs`:

```rust
// ✅ Correct
use crate::constants::MAX_CONCURRENT_DOWNLOADS;

// ❌ Wrong — hardcoded in caller
let max = 3;
```

## Storage Access

Always use the `Storage` trait — never import `JsonStorage` directly in non-storage modules:

```rust
// ✅ Correct — via trait
pub async fn refresh(&self, storage: &dyn Storage) -> Result<()>

// ❌ Wrong — concrete type
pub async fn refresh(&self, storage: &JsonStorage) -> Result<()>
```

Atomic writes: always write to `.tmp` then rename:
```rust
tokio::fs::write(&temp_path, &content).await?;
tokio::fs::rename(&temp_path, &final_path).await?;
```

## Async

Use `tokio::fs` (not `std::fs`) inside `async fn`. Never call blocking I/O on the async executor:

```rust
// ✅ Correct
tokio::fs::read_to_string(&path).await?

// ❌ Blocks the async executor
std::fs::read_to_string(&path)?
```

## Buffer Trait Pattern

All UI buffers must implement:
```rust
fn as_any(&self) -> &dyn std::any::Any { self }
fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
```

Downcasting: always use safe downcast, never unsafe pointer casts:
```rust
buffer.as_any_mut().downcast_mut::<ConcreteBufferType>()
```

## Naming Conventions

- Functions / variables / modules: `snake_case`
- Types / structs / enums: `PascalCase`
- Constants: `UPPER_SNAKE_CASE`
- Test functions: `test_<what>_<scenario>_<expected>` (e.g., `test_filter_by_status_returns_downloaded`)

## Device Sync Scoping

Any file operations during device sync must be scoped to the managed subdirectories only:
```rust
// Scope to Podcasts/ and Playlists/ under sync root — never touch parent dirs
let podcasts_root = sync_path.join("Podcasts");
let playlists_root = sync_path.join("Playlists");
```
