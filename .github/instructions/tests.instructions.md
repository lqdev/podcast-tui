---
applyTo: "tests/**,src/**/*test*"
---

# Test Conventions — podcast-tui

## Structure

Follow Arrange → Act → Assert with clear comments:

```rust
#[test]
fn test_<what>_<scenario>_<expected_outcome>() {
    // Arrange
    let input = create_test_data();

    // Act
    let result = function_under_test(input);

    // Assert
    assert_eq!(result.unwrap(), expected);
}
```

## Async Tests

Use `#[tokio::test]` for any test that uses `.await`:

```rust
#[tokio::test]
async fn test_storage_save_and_reload_roundtrip() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = JsonStorage::new(dir.path()).await.unwrap();
    // ...
}
```

## File System

Always use `tempfile::TempDir` for tests that touch the file system. Never use hardcoded paths:

```rust
let dir = tempfile::TempDir::new().unwrap();
let path = dir.path().join("test-data.json");
```

## Mocking

Use `mockall` for mocking the `Storage` trait:

```rust
use mockall::predicate::*;
mock! {
    StorageImpl {}
    #[async_trait]
    impl Storage for StorageImpl {
        async fn save_podcast(&self, podcast: &Podcast) -> Result<(), StorageError>;
        // ...
    }
}
```

## Rules

- Test names must describe what, scenario, and expected outcome
- Test one thing per test function
- **Never** `unwrap()` on `Result` in assertions without a comment
- Test both success path AND at least one error/edge case
- Integration tests (`tests/`) get their own file per feature area

## Running Tests

```powershell
cargo test                           # all
cargo test --lib                     # unit only
cargo test --test test_playlist      # one integration test file
cargo test -- --nocapture            # see println! output
```
