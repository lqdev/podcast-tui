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

The **Task List** view ([views/1](https://github.com/users/lqdev/projects/1/views/1)) is the canonical stack rank. Each item has an explicit **Stack Rank** number field — lower number = higher priority. The view is sorted by Stack Rank ascending, so the first actionable item is the next to work on.

Stack Rank captures strategic nuances (like "finish this epic before starting that one") that Priority/Phase/Effort sorting alone cannot.

### Project constants

| Constant | Value |
|----------|-------|
| Project number | `1` |
| Project GraphQL node ID | `PVT_kwHOAKnYPM4BPqK6` |
| Stack Rank field ID | `PVTF_lAHOAKnYPM4BPqK6zg-Gc20` |

## Steps

### 1. Fetch the project board

```powershell
gh project item-list 1 --owner lqdev --format json --limit 100
```

Each item includes a `"stack rank"` field (number). Treat the JSON output as unsorted. **Explicitly sort the items by this field ascending** to establish the canonical work order.

For dependency checking, you also need issue bodies. Fetch those for the top candidates:

```powershell
gh issue view <N> --json number,title,state,body --jq '{number,title,state,body}'
```

### 2. Filter to actionable items

Sort by Stack Rank ascending, then exclude:
- `state: CLOSED` — already done
- `Status: Done` or `Status: In Progress` — not next
- Epics (titles starting with `[Epic]` or `[Meta-Epic]`) — not directly implementable

Keep only: `Status: Todo` and `state: OPEN` and not an epic.

**Sort by Stack Rank** — the numeric field is the source of truth.

### 3. Check dependencies

For the top ~10 items from Step 2, scan each issue body for **"Depends on: … #N"** lines. Build a dependency map:
- If issue A depends on issue B, and B is **not** in the Done/Closed set, then A is **blocked**.
- A blocked item cannot be "next up" regardless of its position.

Mark each item as either **READY** (all deps done) or **BLOCKED** (at least one open dep).

### 4. Report the results

Present a short table of the top ~10 actionable items **sorted by Stack Rank**:

| Rank | Issue | Priority | Phase | Effort | Status |
|------|-------|----------|-------|--------|--------|
| **10** | Title ← NEXT | P1 | Phase 1 | S | ✅ READY |
| 20 | Title | P1 | Phase 2 | M | ⛔ blocked by #X |
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

- **The Stack Rank field is the source of truth** — not physical board position or Priority/Phase/Effort field values alone. Stack Rank is maintained by the `rerank-board` and `triage-issue` skills and captures strategic decisions (e.g., "finish half-done epics first").
- Stack Ranks use gaps of 10 (10, 20, 30…) to allow easy insertion without renumbering
- Epics are planning items — always work on their sub-issues, not the epic itself
- If the Stack Rank ordering seems stale or wrong (e.g., a newly triaged issue has a high rank but should be lower), flag it to the user before proceeding
