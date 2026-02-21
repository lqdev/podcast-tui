---
name: rerank-board
description: Reorder items on the Task List project board to reflect current priorities. Supports full strategic reranks and targeted insertions. Always shows proposed order for user confirmation before executing.
---

# Skill: Rerank the Project Board

## When to use

- After a **strategic shift** — epic completed, new theme started, priorities changed
- When **multiple new issues** were triaged and need proper positioning
- When `next-issue` or `session-resume` flags the board order as **stale**
- When the user explicitly asks to reprioritize or reorder the board

## Prerequisites

The **Task List** view ([views/1](https://github.com/users/lqdev/projects/1/views/1)) is the canonical stack rank. Physical item order = work order. This skill maintains that ordering.

### Project constants

| Constant | Value |
|----------|-------|
| Project number | `1` |
| Project GraphQL node ID | `PVT_kwHOAKnYPM4BPqK6` |
| Task List view ID | `PVTV_lAHOAKnYPM4BPqK6zgJZwCc` |
| Stack Rank field ID | `PVTF_lAHOAKnYPM4BPqK6zg-Gc20` |
| Priority field ID | `PVTSSF_lAHOAKnYPM4BPqK6zg-ATAw` |
| Phase field ID | `PVTSSF_lAHOAKnYPM4BPqK6zg-ATA0` |
| Effort field ID | `PVTSSF_lAHOAKnYPM4BPqK6zg-AT-c` |

### Priority option IDs

| Option | ID |
|--------|----|
| P0 - Critical | `c3a04e10` |
| P1 - High | `d84695a4` |
| P2 - Medium | `f53b544b` |
| P3 - Low | `d397cd3c` |

### Phase option IDs

| Option | ID |
|--------|----|
| Phase 1 - Foundation | `8ba9b788` |
| Phase 2 - Core UX | `a3d2da29` |
| Phase 3 - Advanced | `48b8f89a` |
| Backlog | `cb1ac7bf` |

### Effort option IDs

| Option | ID |
|--------|----|
| XS - 1-2 hours | `8518dfa1` |
| S - Half day | `fe0793b9` |
| M - Full day | `6821e868` |
| L - 2-3 days | `e248f7c7` |
| XL - More than 3 days | `8e2ce8c1` |

## Steps

### 1. Query current board state

```powershell
gh project item-list 1 --owner lqdev --format json --limit 200
```

This returns all items including their `"stack rank"` field (a number). **Sort items by Stack Rank ascending** to get the current canonical work order. Parse each item for: issue number, title, status, priority, phase, effort, stack rank, and project item ID.

### 2. Assess what needs reordering

Analyze the current order and identify issues:

- **New items at the bottom** that should rank higher based on Priority/Phase
- **Completed/closed items** still positioned above open items
- **Blocked items** positioned above ready items with similar priority
- **Epic completion gaps** — if an epic is nearly done, its remaining items should be grouped near the top
- **Field values vs position mismatch** — a P1/Phase 1 item sitting below P2/Phase 3 items

Report findings to the user:
> "Found 3 items that appear out of position: #152 (P1) is at the bottom, #104 is above #103 but depends on it, etc."

### 3. Propose the new order

Build the proposed order using these principles (in priority order):

1. **User's strategic direction** — if the user said "finish UX epics first" or "audio is the priority", that overrides field-based sorting
2. **Epic completion** — epics that are >50% done should have their remaining items grouped near the top
3. **Priority → Phase → Effort** — the default field-based sort as a starting heuristic
4. **Features before epics** — actionable feature issues above their parent epic/meta-epic items
5. **Blocked items below ready items** at the same priority level

Present a **before vs after** comparison showing what moved, using Stack Rank values:

```
Proposed reorder (24 items):

 Rank 10:  #99   P1/Ph1/S  Keybinding Conflict Detection          (was: 80)  ⬆
 Rank 20:  #100  P1/Ph1/L  Keybinding Presets & Help Text          (was: 90)  ⬆
 Rank 30:  #103  P1/Ph2/M  Load User Themes from Filesystem        (unchanged)
 ...
```

### 4. Get user confirmation

**Always ask for confirmation before executing.** Show the proposed order and ask:
> "This will reposition N items on the Task List view. Proceed?"

The user may:
- **Approve** — execute the reorder
- **Modify** — "move #134 above #109" → adjust and re-confirm
- **Cancel** — no changes

### 5. Execute the reranking

Update the **Stack Rank field** for each item to its new value. Use gaps of 10 between items (10, 20, 30…) to allow future insertions without renumbering.

```powershell
$projId = "PVT_kwHOAKnYPM4BPqK6"
$fieldId = "PVTF_lAHOAKnYPM4BPqK6zg-Gc20"
$rank = 10
foreach ($item in $orderedItems) {
    gh project item-edit --project-id $projId --id $item.Id --field-id $fieldId --number $rank 2>&1 | Out-Null
    $rank += 10
}
```

Each `item-edit` call is **independent** — no chaining required. Partial failures leave the board in a valid (if imperfect) state, unlike the old `afterId` chaining approach.

After updating Stack Rank values, **also sync the physical board position** so the visual order matches:

```powershell
# First item: move to top (no afterId)
gh api graphql -f query='
mutation {
  updateProjectV2ItemPosition(input: {
    projectId: "PVT_kwHOAKnYPM4BPqK6"
    itemId: "<FIRST_ITEM_ID>"
  }) { items(first:1) { nodes { id } } }
}'

# Subsequent items: place after the previous item
gh api graphql -f query='
mutation {
  updateProjectV2ItemPosition(input: {
    projectId: "PVT_kwHOAKnYPM4BPqK6"
    itemId: "<ITEM_ID>"
    afterId: "<PREVIOUS_ITEM_ID>"
  }) { items(first:1) { nodes { id } } }
}'
```

**Batch both operations in a single loop:**

```powershell
$projId = "PVT_kwHOAKnYPM4BPqK6"
$fieldId = "PVTF_lAHOAKnYPM4BPqK6zg-Gc20"
$prevId = $null
$rank = 10
foreach ($item in $orderedItems) {
    # Set the Stack Rank field (source of truth)
    gh project item-edit --project-id $projId --id $item.Id --field-id $fieldId --number $rank 2>&1 | Out-Null

    # Sync the physical board position to match
    if ($prevId) {
        $afterClause = "afterId: `"$prevId`""
    } else {
        $afterClause = ""
    }
    $mutation = "mutation { updateProjectV2ItemPosition(input: { projectId: `"$projId`", itemId: `"$($item.Id)`" $afterClause }) { items(first:1) { nodes { id } } } }"
    gh api graphql -f query="$mutation" 2>&1 | Out-Null
    $prevId = $item.Id
    $rank += 10
}
```

### 6. Update field values (if needed)

If the reorder involves changing Priority or Phase values (not just physical position), use `gh project item-edit`:

```powershell
gh project item-edit --project-id "PVT_kwHOAKnYPM4BPqK6" --id "<ITEM_ID>" --field-id "<FIELD_ID>" --single-select-option-id "<OPTION_ID>"
```

**Note:** `gh project item-edit` uses the project GraphQL node ID for `--project-id`, not the project number. It does NOT take `--owner`.

### 7. Verify the result

Re-query the board and display the final order:

```powershell
gh project item-list 1 --owner lqdev --format json --limit 200
```

Confirm the top 10 items have correct Stack Rank values and that the physical order matches. If any items are mispositioned, flag and fix.

## Modes

### Full rerank

Reorder all non-Done items on the board. Use when priorities have shifted significantly or after a planning session. Follows all steps above. Reassigns Stack Rank values with fresh gaps of 10.

### Targeted insert

Position one or a few specific items into the existing board order. Faster than a full rerank — only the new items get Stack Rank values.

1. Query the board and sort by Stack Rank to find the correct position based on Priority/Phase/Effort
2. Compute a Stack Rank value between the surrounding items (e.g., if inserting between rank 30 and 40, use 35)
3. Set the Stack Rank field with `gh project item-edit`
4. Optionally sync physical position with `updateProjectV2ItemPosition`
5. No user confirmation needed for single-item inserts (the triage-issue skill handles this)

**Gap exhaustion:** If there's no integer gap between two adjacent ranks (e.g., 30 and 31), renumber starting from the insertion point: shift all items below down by 10, then insert at the freed slot.

## Rules

- **Never reorder without showing the proposed order first** (full rerank mode)
- **Stack Rank field is the source of truth** — physical board position is kept in sync but is secondary
- **Always use the project GraphQL node ID** (`PVT_kwHOAKnYPM4BPqK6`), not the project number, for `updateProjectV2ItemPosition` and `gh project item-edit --project-id`
- **Use gaps of 10** when assigning Stack Rank values (10, 20, 30…) to allow easy insertion
- **Features above epics** — actionable items should always appear above their parent epic/meta-epic
- **Preserve intentional ordering** — if the user manually positioned something, don't override it unless asked
- **Done items stay in place** — don't waste API calls repositioning completed items; they'll scroll off naturally

## Anti-patterns

- ❌ Re-sorting purely by Priority/Phase/Effort without considering strategic context (epic completion, user direction)
- ❌ Executing a reorder without user confirmation
- ❌ Setting Stack Rank values without gaps (1, 2, 3 instead of 10, 20, 30) — makes future insertions require full renumbering
- ❌ Using project number (`1`) instead of GraphQL node ID for position mutations
- ❌ Using `--owner` with `gh project item-edit` (it doesn't support that flag)
- ❌ Updating physical position without also updating Stack Rank field (causes source-of-truth drift)
