---
applyTo: "**/*.md"
---

# Documentation Conventions — podcast-tui

## Voice and Tone

- Write for developers and technical power users
- Be direct and concise — no fluff
- Use present tense ("Press `q` to close")
- Use active voice

## Required Accuracy

Before writing or updating documentation:
1. Verify config field names in `src/config.rs`
2. Verify keybindings in `src/ui/keybindings.rs`
3. Verify command names in `src/ui/app.rs` command dispatch
4. Check `src/constants.rs` for any threshold/limit values

## Substitutions (always apply)

| ❌ Never write | ✅ Always write |
|---|---|
| `yourusername` | `lqdev` |
| Sprint 3 Complete / Sprint X | v1.6.0 or current version |
| 37.5% / 75% complete | Feature list or "active development" |
| October 2025 | Current date (February 2026 or later) |

## Status Markers

Use these consistently for feature status:
- ✅ **COMPLETE** — feature is shipped and working
- ⏳ **PENDING** — not yet implemented
- 🚧 **IN PROGRESS** — actively being developed

## Code Blocks

Always specify the language for syntax highlighting:
````markdown
```rust
// Rust code
```
```json
// JSON config example
```
```powershell
# PowerShell command
```
````

## Links

Use relative links for cross-references within the repo:
```markdown
See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for details.
See [ADR-002](docs/adr/ADR-002-buffer-based-ui.md) for the buffer system decision.
```

## Headers

- H1 (`#`) — document title only (one per file)
- H2 (`##`) — major sections
- H3 (`###`) — subsections
- H4 (`####`) — rarely; prefer bullet points

## What NOT to Document

Do not document as complete or available:
- Episode notes (not implemented)
- Statistics tracking (not implemented)
- Duration-based filtering (deferred — see `docs/rfcs/RFC-001-search-and-filter.md`)

## Archive vs Delete

Never delete documentation. If content is stale:
- Move to `docs/archive/` with a header note
- Add `> **Note**: This document is archived. It was accurate as of [date].`

## Footer Format

Use this footer for docs with version context:
```markdown
---
*Last Updated: Month Year | Version: vX.Y.Z | Maintainer: [@lqdev](https://github.com/lqdev)*
```
