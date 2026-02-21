---
name: session-resume
description: Bootstrap a new session from a previous session's checkpoint. Reads the handoff file, confirms the project board state, and prepares to continue work.
---

# Skill: Session Resume

## When to use
- At the **start of a new session** when the user provides a checkpoint file path or asks you to "pick up where we left off"
- When the user pastes a handoff summary and asks you to continue
- When the user says "resume", "continue", "read checkpoint", or similar

## Steps

### 1. Read the checkpoint

The user will provide one of:
- **A file path** to a checkpoint: read it with the `view` tool
- **Pasted text** from a previous handoff: parse it directly

Extract:
- What was accomplished last session (PRs, commits, decisions)
- What's next (the stack-ranked table)
- Key reminders

### 2. Verify the codebase state

Confirm the checkpoint matches reality:

```powershell
# Verify we're on main and up to date
git checkout main
git pull origin main

# Check recent commits match what the checkpoint claims
git log --oneline -5

# Check for any open PRs from previous session that might need attention
gh pr list --state open --limit 5
```

If there are **discrepancies** (e.g., a PR the checkpoint says is merged but isn't, or unexpected new commits), flag them to the user before continuing.

### 3. Confirm the stack rank is current

The **Task List** view ([views/1](https://github.com/users/lqdev/projects/1/views/1)) is physically ordered by stack rank. Use `gh project item-list` (which returns items in view order) to check the current state:

```powershell
gh project item-list 1 --owner lqdev --format json --limit 100
```

Use the `next-issue` skill (steps 1-4) to filter and check dependencies. Compare the top actionable item against the checkpoint's "What's next" table.

**If the top item changed** (someone else closed it, priorities shifted, board was reordered), tell the user:
> "Checkpoint said #97 was next, but it's now closed. Current top of stack is #98."

**If it matches**, confirm briefly:
> "Stack rank confirmed — #97 is still next."

**Important:** The physical board order is the source of truth — not a re-sort by Priority/Phase/Effort. If the board order looks wrong (e.g., a newly triaged item is at the bottom), flag it to the user.

### 4. Surface key reminders

Print the "Key reminders" from the checkpoint so they're visible in the new session context. This ensures conventions and decisions carry forward.

### 5. Check for in-flight work

If the checkpoint mentions open PRs or draft branches:

```powershell
# Check if the PR is still open
gh pr view {N} --json state,mergeStateStatus,reviews

# Check for new review comments since the checkpoint
gh pr view {N} --json reviews --jq '.reviews[] | {author: .author.login, state: .state, submittedAt: .submittedAt}'
```

If there are **new review comments**, alert the user:
> "PR #N has new review comments since the checkpoint. Use the `code-review-validation` skill to action them before starting new work."

### 6. Prepare for next work

Based on the confirmed next issue, tell the user what's ready:

> **Ready to go:** Issue #N — Title (Priority, Phase, Effort)
>
> When you're ready, say "work on it" and I'll use the `work-on-issue` skill to pick it up.

## Rules

- **Always re-verify** — never trust the checkpoint blindly. Git state and the project board are the source of truth; the checkpoint is a hint.
- **Flag discrepancies** — if the checkpoint doesn't match reality, stop and tell the user. Don't silently proceed with stale context.
- **Check for open PRs first** — unfinished work from last session takes priority over new issues (review comments, failed CI, etc.).
- **Print reminders visibly** — key reminders exist to prevent the new session from repeating mistakes. Make them prominent.
- **Use skills for sub-tasks** — `next-issue` for stack rank, `code-review-validation` for review comments, `work-on-issue` to start.

## What if there's no checkpoint?

If the user says "resume" but there's no checkpoint file:

1. Ask: "I don't have a checkpoint file. Can you paste the handoff context from the previous session, or point me to the checkpoint path?"
2. If they can't provide one, fall back to:
   - `git log --oneline -10` to see recent activity
   - `gh pr list --state all --limit 10` to find recent PRs
   - `next-issue` skill to find the top of the stack
   - Present what you find and ask the user to confirm before proceeding

## Anti-patterns

- ❌ Starting work without verifying the checkpoint against Git/GitHub
- ❌ Ignoring open PRs with pending review comments
- ❌ Skipping the stack rank confirmation ("it probably hasn't changed")
- ❌ Not printing key reminders in the new session context
- ❌ Proceeding without user confirmation when discrepancies are found
