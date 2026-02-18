# ADR-003: Metadata-Based Device Sync

**Status**: Accepted  
**Date**: 2025-10  
**Deciders**: @lqdev

## Context

The application needs to sync downloaded podcast episodes to external MP3 players and USB devices. These devices may not support checksums or timestamps reliably, and full file comparison would be slow for large libraries.

## Decision

Use **metadata-based comparison** (filename + file size) to determine whether a file needs to be synced:
- If a file with the same name and size exists on device → skip
- If missing or size differs → copy
- If present on device but no longer on PC → optionally delete (orphan deletion)

Device output layout uses **sibling directories** from the sync root:
- `<sync_path>/Podcasts/<podcast-name>/` — downloaded episodes
- `<sync_path>/Playlists/<playlist-name>/` — playlist audio copies

Scoped to managed directories only — never touches unmanaged user content.

## Consequences

**Positive:**
- Fast sync (no checksum computation)
- Works with FAT32/exFAT filesystems common on MP3 players
- Dry-run mode allows preview before applying changes
- Orphan deletion is opt-in (safe default)

**Negative:**
- File size comparison can produce false matches if a file is corrupted but same size
- No support for partial transfers or resume
- Requires podcast folder structure to be consistent

## References
- `src/download/manager.rs` — Sync implementation (methods prefixed `sync_`)
- `src/ui/buffers/sync.rs` — Sync history buffer
- `docs/KEYBINDINGS.md` — `:sync` and `:sync-dry-run` commands
