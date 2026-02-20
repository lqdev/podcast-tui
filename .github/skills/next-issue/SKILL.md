---
name: next-issue
description: Query the GitHub project board to find the next stack-ranked issue to work on. Returns the top actionable Todo item by Priority then Phase, checking issue dependencies, filtering out epics, closed issues, and blocked items.
---

# Skill: Find the Next Issue

## When to use
- Before picking up new work — always check the board, not just the issue list
- When `work-on-issue` Step 2 tells you to check the project board
- When the user asks "what's next?" or "what should we work on?"

## Steps

### 1. Fetch the project board

Use the GraphQL API to get all items with their field values:

```powershell
gh api graphql -f query='
{
  user(login: "lqdev") {
    projectV2(number: 1) {
      items(first: 50) {
        nodes {
          fieldValues(first: 8) {
            nodes {
              ... on ProjectV2ItemFieldSingleSelectValue {
                name
                field { ... on ProjectV2SingleSelectField { name } }
              }
            }
          }
          content {
            ... on Issue { number title state }
          }
        }
      }
    }
  }
}'
```

### 2. Filter to actionable items

Exclude:
- `state: CLOSED` — already done
- `Status: Done` or `Status: In Progress` — not next
- Epics (titles starting with `[Epic]` or `[Meta-Epic]`) — not directly implementable

Keep only: `Status: Todo` and `state: OPEN` and not an epic.

### 3. Check dependencies

Before ranking, scan each issue body for **"Depends on: … #N"** lines. Build a dependency map:
- If issue A depends on issue B, and B is **not** in the Done/Closed set, then A is **blocked**.
- A blocked item cannot be "next up" regardless of its rank fields.

Mark each item as either **READY** (all deps done) or **BLOCKED** (at least one open dep).

### 4. Stack rank the results

Sort **READY** items by the following fields in order:

| Field | Rank order |
|-------|-----------|
| **Priority** | P0 > P1 > P2 > P3 |
| **Phase** | Phase 1 > Phase 2 > Phase 3 |
| **Effort** | XS > S > M > L > XL (prefer smaller work when all else equal) |

Append **BLOCKED** items at the bottom (same sort order within blocked), noting what they're waiting on.

### 5. Report the top item

Present a short table of the top 5 actionable items:

| # | Issue | Priority | Phase | Effort | Status |
|---|-------|----------|-------|--------|--------|
| **N** | Title (next up) | P1 | Phase 1 | S | ✅ READY |
| N | Title | P1 | Phase 2 | M | ⛔ blocked by #X |
| ... | ... | ... | ... | ... | ... |

Call out the **top READY item** explicitly as "Next up: #N — Title".

### 6. Cross-reference with work-on-issue

After identifying the next issue, use the `work-on-issue` skill to implement it.

## Priority reference

| Board label | Meaning |
|-------------|---------|
| `P0 - Critical` | Blocking — must fix immediately |
| `P1 - High` | Core feature or quality issue — standard next work |
| `P2 - Medium` | Valuable but not urgent |
| `P3 - Low` | Nice to have |

## Notes

- The board is the source of truth for ordering — not issue numbers or creation dates
- If two items have the same Priority + Phase, prefer lower effort (quicker wins unblock higher-effort work)
- Epics are planning items — always work on their sub-issues, not the epic itself
