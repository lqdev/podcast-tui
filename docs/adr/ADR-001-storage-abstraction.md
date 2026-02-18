# ADR-001: JSON Storage with Trait Abstraction

**Status**: Accepted  
**Date**: 2025-09  
**Deciders**: @lqdev

## Context

The application needs persistent storage for podcast subscriptions, episode metadata, configuration, and download state. The primary use case is a single-user local application where data volumes are modest (hundreds of podcasts, thousands of episodes).

## Decision

Use a **trait-based storage abstraction** (`Storage` trait in `src/storage/traits.rs`) with a **JSON file implementation** (`src/storage/json.rs`) as the initial backend.

- One JSON file per podcast (in `podcasts/` directory)
- One JSON file per episode grouped by podcast (in `episodes/{podcast-id}/`)
- Atomic writes via temp file + rename pattern
- `serde` / `serde_json` for serialization

## Consequences

**Positive:**
- Human-readable data files — easy to inspect and manually edit
- Simple backup: copy the data directory
- No database dependency to install or manage
- Version control friendly for small datasets
- Easy to migrate to a different backend by swapping the `Storage` impl

**Negative:**
- No query capability (no filtering/sorting at storage layer)
- Performance degrades with very large episode counts
- No transactions across multiple files

**Future options enabled by the trait abstraction:**
- SQLite backend (for better query performance)
- Remote/cloud storage
- Encrypted storage

## References
- `src/storage/traits.rs` — Storage trait definition
- `src/storage/json.rs` — JSON implementation
- `docs/STORAGE_DESIGN.md` — Detailed storage design documentation
