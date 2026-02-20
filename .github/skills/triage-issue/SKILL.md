---
name: triage-issue
description: Triage a GitHub issue by applying labels, setting project board fields, linking to epics, and identifying blockers.
---

# Skill: Triage a GitHub Issue

## When to use
After an issue is created (or discovered untriaged), apply the correct metadata so it's properly prioritized and discoverable on the [project board](https://github.com/users/lqdev/projects/1).

## Steps

### 1. Read the issue fully

Understand the scope, impact, and intent before triaging. Check for:
- Is it a duplicate of an existing open issue?
- Does it belong to an existing epic?
- Are there unresolved dependencies?

### 2. Apply type label

Every issue gets exactly one type label:

| Label | When to use |
|-------|-------------|
| `bug` | Something is broken or behaves incorrectly |
| `enhancement` | New feature, improvement, or capability |
| `documentation` | Documentation-only change |
| `testing` | Test infrastructure or coverage improvement |

### 3. Apply component labels

Add one or more component labels based on which modules are affected:

| Label | Modules |
|-------|---------|
| `ui` | `src/ui/`, buffers, keybindings, themes |
| `downloads` | `src/download/manager.rs` download operations |
| `sync` | `src/download/manager.rs` sync operations |
| `storage` | `src/storage/`, JSON persistence |
| `rss` | `src/podcast/feed.rs`, subscription management |
| `audio` | Rodio integration (future) |
| `performance` | Optimization, async efficiency |

### 4. Set priority

| Priority | Criteria | Examples |
|----------|----------|---------|
| `P0` | Blocker — data loss, crash, security vulnerability | Unsafe memory leak (#63) |
| `P1` | High — broken core workflow, bad UX for common path | Sync buffer non-functional (#74) |
| `P2` | Medium — improvement to existing feature, consistency fix | Normalize feedback messages (#64) |
| `P3` | Low — nice-to-have, cosmetic, future enhancement | Named sync targets |

### 5. Set effort estimate

| Effort | Criteria |
|--------|----------|
| `XS` | < 30 min. Trivial one-liner or config change |
| `S` | Half-day. Small feature or focused bug fix |
| `M` | Full day. Feature with tests, or multi-file bug fix |
| `L` | 2–3 days. Significant feature with design work |
| `XL` | 3+ days. Major feature, likely needs phased breakdown |

If effort is `XL`, consider whether the issue should be an **epic** with sub-issues instead.

### 6. Set phase

| Phase | Criteria |
|-------|----------|
| `Phase 1` | Foundation — must be done first, unblocks other work |
| `Phase 2` | Core functionality — the main feature work |
| `Phase 3` | Polish — advanced features, config, docs |
| `Backlog` | Not scheduled — valid but not prioritized yet |

### 7. Link to epic (if applicable)

If the issue is part of a larger effort:
- Add `**Parent**: <Epic title>` to the issue body
- Use GitHub's sub-issues feature to link it under the epic
- Add `**Depends on**: #N` if there's a sequencing constraint

### 8. Check for blockers

If the issue depends on unfinished work:
- Add `**Depends on**: #N` to the body
- Apply the `blocked` label if the dependency is not yet in progress
- Set the project board status to `Todo` (not `In Progress`)

### 9. Apply status label (if needed)

| Label | When |
|-------|------|
| `needs-triage` | Remove this label once triage is complete |
| `blocked` | Issue cannot proceed until dependency is resolved |
| `help-wanted` | Good for external contributors |

## Triage checklist

After triaging, the issue should have:
- [ ] Exactly one type label (`bug`, `enhancement`, `documentation`, or `testing`)
- [ ] One or more component labels (`ui`, `storage`, `sync`, etc.)
- [ ] Priority set on project board (`P0`–`P3`)
- [ ] Effort set on project board (`XS`–`XL`)
- [ ] Phase set on project board (or `Backlog`)
- [ ] Epic linkage if part of a larger effort
- [ ] `needs-triage` label removed
- [ ] `blocked` label added if dependencies exist

## Tips

- When in doubt about priority, check: "If we shipped tomorrow without this, would a user notice?" P0/P1 = yes, P2/P3 = probably not.
- Effort estimates are for a developer familiar with the codebase. Double it for a first-time contributor.
- Phase assignment follows the epic's phasing when the issue is a sub-issue. Standalone issues default to `Backlog` unless explicitly prioritized.
