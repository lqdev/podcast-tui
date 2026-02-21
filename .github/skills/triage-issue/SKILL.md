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

### 9. Position the item on the board

After setting all fields, assign a **Stack Rank** value and position the item in the correct location on the Task List view. The Stack Rank field is the canonical work order — new items land at the bottom by default and need to be assigned a proper rank.

1. Query the current board and sort by Stack Rank:
   ```powershell
   gh project item-list 1 --owner lqdev --format json --limit 200
   ```

2. Sort items by Stack Rank ascending. Find the correct position based on the new item's Priority → Phase → Effort relative to existing items. The item should be placed:
   - **After** the last item with higher or equal priority (lower or equal Stack Rank number)
   - **Before** the first item with lower priority (higher Stack Rank number)
   - **Above** all epic/meta-epic items (features sort before their parent epics)

3. Compute a Stack Rank value between the surrounding items. For example, if inserting between rank 30 and 40, use 35.
   - If there is a gap (e.g., 30 and 40), pick a value in the gap (e.g., 35).
   - If there is **no** gap (the surrounding ranks are consecutive, e.g., 30 and 31), handle gap exhaustion explicitly:
     - Prefer: trigger the `rerank-board` skill to rebalance ranks for the affected section. It implements the gap-compaction and renumbering procedure.
     - For a small set of items only: increase the Stack Rank of the lower item **and all items below it** by `+10` using `gh project item-edit`, then choose a value in the new gap (for example, between 30 and 40, use 35).

4. Set the Stack Rank field:
   ```powershell
   gh project item-edit --project-id "PVT_kwHOAKnYPM4BPqK6" --id "<NEW_ITEM_ID>" --field-id "PVTF_lAHOAKnYPM4BPqK6zg-Gc20" --number <RANK>
   ```

5. Optionally sync physical board position with `updateProjectV2ItemPosition`:
   ```powershell
   gh api graphql -f query='mutation {
     updateProjectV2ItemPosition(input: {
       projectId: "PVT_kwHOAKnYPM4BPqK6"
       itemId: "<NEW_ITEM_ID>"
       afterId: "<ITEM_THAT_SHOULD_COME_BEFORE>"
     }) { items(first:1) { nodes { id } } }
   }'
   ```

   If the item should be first on the board, omit `afterId`.

See the `rerank-board` skill for project constants and full reordering mechanics.

### 10. Apply status label (if needed)

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
- [ ] **Stack Rank assigned** (not left at default / bottom)
- [ ] `needs-triage` label removed
- [ ] `blocked` label added if dependencies exist

## Tips

- When in doubt about priority, check: "If we shipped tomorrow without this, would a user notice?" P0/P1 = yes, P2/P3 = probably not.
- Effort estimates are for a developer familiar with the codebase. Double it for a first-time contributor.
- Phase assignment follows the epic's phasing when the issue is a sub-issue. Standalone issues default to `Backlog` unless explicitly prioritized.
