# ADR-002: Emacs-Style Buffer-Based UI

**Status**: Accepted  
**Date**: 2025-09  
**Deciders**: @lqdev

## Context

The TUI needs to support multiple views (podcast list, episode list, episode detail, downloads, playlists, help, etc.) with fluid navigation between them. The design must work reliably across all terminal emulators without requiring mouse support.

## Decision

Implement an **Emacs-style buffer-based UI** where:
- Each view is a "buffer" implementing the `Buffer` trait
- `BufferManager` maintains a stack of open buffers
- Users switch buffers with `Tab`/`Shift+Tab`, `F2`-`F7`, or `Ctrl+b`
- A **minibuffer** at the bottom handles command input and status messages
- A **status bar** shows current buffer name and key hints

Key conventions:
- `Buffer` trait requires `render()`, `handle_key()`, `title()`, `as_any()`, `as_any_mut()`
- Buffers are registered with typed getter methods in `BufferManager`
- Background refresh via `tokio::spawn` to keep UI responsive

## Consequences

**Positive:**
- Familiar pattern for keyboard-centric power users
- Clean separation of concerns — each buffer is self-contained
- Easy to add new views without refactoring existing ones
- Works in any terminal without mouse support

**Negative:**
- Buffer lifecycle management adds complexity
- Typed getter methods in `BufferManager` need updating when new buffer types are added
- Context switching between many buffers can feel disorienting for new users

## References
- `src/ui/buffers/mod.rs` — Buffer trait and BufferManager
- `src/ui/app.rs` — Main application event loop
- `docs/ARCHITECTURE.md` — UI layer documentation
