---
name: create-rfc
description: Create a new RFC (Request for Comments / design document) for a planned feature or significant change. Saves to docs/rfcs/ following the RFC-NNN naming convention.
---

# Skill: Create an RFC

## When to use
Before implementing:
- A new major feature (new module, new command category, new data model)
- A significant behavior change
- A cross-cutting technical decision (new dependency, new abstraction)
- Anything that warrants design review before coding begins

## Steps

### 1. Determine the next RFC number
List files in `docs/rfcs/` and find the highest RFC number, then increment.

### 2. Create the RFC file
Create `docs/rfcs/RFC-<NNN>-<short-title>.md`:

```markdown
# RFC-<NNN>: <Title>

**Status**: Draft | Accepted | Implemented | Superseded  
**Date**: YYYY-MM-DD  
**Author**: @lqdev

## Summary

One paragraph: what is this RFC proposing?

## Motivation

Why is this change needed? What problem does it solve?
What is the current limitation?

## Detailed Design

### Data Model Changes (if any)
```rust
// New or modified structs/enums
```

### API Changes (if any)
```rust
// New or modified functions/traits
```

### UI Changes (if any)
- New keybindings
- New commands
- New buffers

### Config Changes (if any)
```json
{
  "section": {
    "new_field": "default_value"
  }
}
```

### Implementation Steps
1. Step 1
2. Step 2
3. ...

## Alternatives Considered

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| Option A | ... | ... | Chosen |
| Option B | ... | ... | Rejected because ... |

## Design Decisions

Document non-obvious choices inline as numbered design decisions:
> **Design Decision #N**: We chose X over Y because...

## Open Questions

- [ ] Question 1 (resolved: answer)
- [ ] Question 2 (still open)

## References
- Related issues/PRs
- Prior art or inspiration
```

### 3. Update the status when implemented
Change `**Status**: Draft` to `**Status**: Implemented` and add a reference to the implementing PR/commit.
