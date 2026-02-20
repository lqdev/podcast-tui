---
name: create-issue
description: Draft a well-structured GitHub issue for a feature, bug, or chore. Encodes the project's quality bar for shovel-ready issues.
---

# Skill: Create a GitHub Issue

## When to use
When you need to file a new issue — whether from discovering a bug, scoping a feature, splitting work from an epic, or deferring feedback from a code review.

## Issue quality bar

Every issue must be **shovel-ready**: an agent or developer should be able to pick it up and start implementing without asking clarifying questions.

## Steps

### 1. Choose the title format

| Type | Title format | Example |
|------|-------------|---------|
| Feature | `[Feature] Brief description` | `[Feature] Sync Buffer Phase 2: Core UX` |
| Bug | `[Bug] Brief description` | `[Bug] Closing Help buffer prevents reopening` |
| Epic | `[Epic] Brief description` | `[Epic] Sync Buffer Overhaul` |
| Chore | `chore: Brief description` | `chore: Sync Cargo.toml versions after release` |
| Test | `test: Brief description` | `test: Add failure-tracking test for cleanup` |
| Docs | `docs: Brief description` | `docs: Update KEYBINDINGS.md for sync buffer` |
| Refactor | `refactor: Brief description` | `refactor: Extract constants to centralized module` |

### 2. Write the issue body

Use this structure (omit sections that don't apply):

```markdown
## Summary

One paragraph: what this issue is about and why it matters.

**Parent**: <Epic name> (if this is a sub-issue)
**Depends on**: #N (if there are blocking dependencies)
**Origin**: <PR link or review comment> (if deferred from a review)

## Problem / Background

What's broken or missing? Be specific:
- Current behavior (with code references: `file.rs:line`)
- Expected behavior
- User impact

## Proposed Solution / Fix

What should be done? Include:
- Code changes with snippets showing the approach
- Which pattern to follow (reference existing code)
- Alternatives considered and why they were rejected

## Files to Modify

| File | Change |
|------|--------|
| `src/ui/app.rs` | Add match arm for new action |
| `src/ui/buffers/help.rs` | Update help text |

## Acceptance Criteria

- [ ] Criterion 1 (specific, testable)
- [ ] Criterion 2
- [ ] All existing tests pass (`cargo test`)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Testing Strategy

### Unit Tests
- What to test and expected behavior

### Integration Tests
- End-to-end workflows to verify

### Manual Tests
- Steps to manually verify (for UI changes)

## Implementation Notes

Non-obvious technical details, gotchas, or constraints.

## Definition of Done

- [ ] Functionality implemented according to requirements
- [ ] Unit tests written and passing
- [ ] Code passes linting (clippy) with no warnings
- [ ] Code formatted with rustfmt
- [ ] Documentation updated (if applicable)
```

### 3. Calibrate detail level by effort

| Effort | Detail level | Example |
|--------|-------------|---------|
| **XS** (trivial) | Summary + Files + Acceptance Criteria | Issue #64: 5 `show_error` → `show_message` changes |
| **S** (half-day) | Full body, light on implementation notes | Issue #51: Add F3 keybinding |
| **M** (full day) | Full body with code snippets and testing strategy | Issue #48: MockStorage + failure test |
| **L–XL** (multi-day) | Full body with detailed design, phase breakdown | Issue #74: Sync buffer Phase 1 |
| **Epic** | Summary + problem + phased solution + sub-issue links | Issue #73: Sync buffer overhaul |

### 4. Reference real code

Always verify before writing:
- Check `src/ui/keybindings.rs` for keybinding claims
- Check `src/ui/app.rs` for command dispatch
- Check `src/config.rs` for config field names
- Check `src/constants.rs` for threshold values
- Include approximate line numbers (e.g., `app.rs:~1212`)

### 5. Link to context

- **Sub-issue of an epic?** Add `**Parent**: <Epic title>` in summary
- **Deferred from a review?** Add `**Origin**: <PR link>` with the review comment URL
- **Depends on another issue?** Add `**Depends on**: #N`
- **Related to prior work?** Add `**Related**: #N, PR #M`

### 6. Apply labels

Use the `triage-issue` skill to apply the correct labels after creation.

## Anti-patterns

- ❌ **Vague acceptance criteria**: "It should work correctly" → be specific
- ❌ **Missing code references**: "Fix the sync buffer" → say which file, which function, which line
- ❌ **No testing guidance**: Always include what to test and how
- ❌ **Scope creep**: One issue = one logical unit of work. If it's multiple things, split into sub-issues
- ❌ **Implementation-free**: Don't just describe the problem — sketch the solution with code snippets

## Examples of well-written issues in this repo

- **XS/S**: Issue #64 — concise, 5 specific changes, clear before/after
- **M**: Issue #48 — two options with code, constraints, compatibility notes
- **L**: Issue #74 — root cause analysis, fix per root cause, line references
- **Epic**: Issue #73 — phased breakdown, keybinding table, design decisions
