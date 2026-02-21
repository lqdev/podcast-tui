---
name: next-issue
description: Query the GitHub project board to find the next stack-ranked issue to work on. Returns the top actionable Todo item from the Task List view, checking issue dependencies and filtering out epics, closed issues, and blocked items.
---

# Skill: Find the Next Issue

## When to use
- Before picking up new work — always check the board, not just the issue list
- When `work-on-issue` Step 2 tells you to check the project board
- When the user asks "what's next?" or "what should we work on?"

## Board view reference

The **Task List** view ([views/1](https://github.com/users/lqdev/projects/1/views/1)) is the canonical stack rank. Items are **physically ordered** top-to-bottom by priority — the first actionable item in the list is the next to work on. Do **not** re-sort by field values; the physical order captures nuances (like "finish this epic before starting that one") that Priority/Phase/Effort sorting cannot.

`gh project item-list` returns items in this physical order.

## Steps

### 1. Fetch the project board (in view order)

Use `gh project item-list` which returns items in their physical board position:

```powershell
gh project item-list 1 --owner lqdev --format json --limit 100
```

This returns items in **stack rank order** — the physical order of the Task List view.

For dependency checking, you also need issue bodies. Fetch those for the top candidates:

```powershell
gh issue view <N> --json number,title,state,body --jq '{number,title,state,body}'
```

### 2. Filter to actionable items

Walk the list **in order** and exclude:
- `state: CLOSED` — already done
- `Status: Done` or `Status: In Progress` — not next
- Epics (titles starting with `[Epic]` or `[Meta-Epic]`) — not directly implementable

Keep only: `Status: Todo` and `state: OPEN` and not an epic.

**Preserve the list order** — do not re-sort.

### 3. Check dependencies

For the top ~10 items from Step 2, scan each issue body for **"Depends on: … #N"** lines. Build a dependency map:
- If issue A depends on issue B, and B is **not** in the Done/Closed set, then A is **blocked**.
- A blocked item cannot be "next up" regardless of its position.

Mark each item as either **READY** (all deps done) or **BLOCKED** (at least one open dep).

### 4. Report the results

Present a short table of the top ~10 actionable items **in board order** (not re-sorted):

| # | Issue | Priority | Phase | Effort | Status |
|---|-------|----------|-------|--------|--------|
| **N** | Title ← NEXT | P1 | Phase 1 | S | ✅ READY |
| N | Title | P1 | Phase 2 | M | ⛔ blocked by #X |
| ... | ... | ... | ... | ... | ... |

Call out the **first READY item** explicitly as "Next up: #N — Title".

### 5. Cross-reference with work-on-issue

After identifying the next issue, use the `work-on-issue` skill to implement it.

## Priority reference

| Board label | Meaning |
|-------------|---------|
| `P0 - Critical` | Blocking — must fix immediately |
| `P1 - High` | Core feature or quality issue — standard next work |
| `P2 - Medium` | Valuable but not urgent |
| `P3 - Low` | Nice to have |

## Notes

- **The board's physical order is the source of truth** — not Priority/Phase/Effort field values alone. The physical order on the Task List view (views/1) is maintained by stack rank operations and captures strategic decisions (e.g., "finish half-done epics first").
- Epics are planning items — always work on their sub-issues, not the epic itself
- If the physical order seems stale or wrong (e.g., a newly triaged issue is at the bottom but should be higher), flag it to the user before proceeding
