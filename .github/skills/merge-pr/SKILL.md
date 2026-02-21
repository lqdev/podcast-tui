---
name: merge-pr
description: Check if a PR is ready, squash-merge it into main, and delete the branch. The standard completion workflow after a PR is approved and all checks pass.
---

# Skill: Merge a Pull Request

## When to use
When a PR is ready to land: review comments have been addressed, CI is green, and the branch is mergeable. Use this skill to complete the lifecycle of a work-on-issue branch.

## Pre-flight Checks

Before merging, verify all of the following:

### 1. Confirm PR is mergeable
```bash
gh pr view <PR_NUMBER> --json mergeable,mergeStateStatus,isDraft,reviews
```

Required state:
- `mergeable: MERGEABLE` (no conflicts)
- `mergeStateStatus: CLEAN` or `BLOCKED` only by missing approvals (not by failing CI)
- `isDraft: false`

If any check fails, **stop** and report the blocker. Do not merge with conflicts or failing CI.

### 2. Confirm all review comments are resolved

```bash
gh pr view <PR_NUMBER> --json reviews
```

First verify that `copilot-pull-request-reviewer` is present in the reviews list. **If the bot has not yet posted its review, stop — do not merge.** The bot runs asynchronously and an absent review means it hasn't finished, not that the PR is clean.

Then check inline threads:
- Check PR review threads via the `get_review_comments` MCP tool or the GitHub UI
- All `ACCEPT` items must be implemented and pushed
- All `DEFER` items must have a corresponding GitHub issue
- No unresolved blocking comments from human reviewers

### 3. Confirm CHANGELOG is updated
- If the PR is user-facing, verify `[Unreleased]` in `CHANGELOG.md` contains an entry for this change
- If missing, add it now using the `update-changelog` skill before merging

## Merge

Use squash merge to keep main history linear:

```bash
gh pr merge <PR_NUMBER> --squash --delete-branch
```

Flags:
- `--squash` — squashes all branch commits into one clean commit on main
- `--delete-branch` — deletes the remote branch immediately after merge (GitHub also deletes the local tracking ref)

The squash commit title defaults to the PR title. This is correct — ensure the PR title follows conventional commit format (`fix(scope): description`) before merging.

## Post-merge

### 1. Pull main locally
```bash
git checkout main
git pull origin main
```

### 2. Verify the merge commit is present
```bash
git log --oneline -3
```

Confirm the squash commit appears at the tip of main with the correct message and `Closes #N` in the body.

### 3. Confirm issue is closed
The `Closes #N` in the commit message automatically closes the linked issue. Verify:
```bash
gh issue view <ISSUE_NUMBER> --json state
```

Should return `"state": "CLOSED"`.

## Decision Table

| Condition | Action |
|-----------|--------|
| `mergeStateStatus: CLEAN` | ✅ Merge |
| `mergeStateStatus: DIRTY` (conflicts) | 🚫 Rebase branch first, re-run checks |
| `mergeStateStatus: BLOCKED` (failing CI) | 🚫 Fix CI failures first |
| `isDraft: true` | 🚫 Mark ready for review first |
| Unresolved review comments | 🚫 Address comments first |
| CHANGELOG missing for user-facing change | 🚫 Add changelog entry first |

## Tips

- Always squash — never merge commits or rebase-merge. Squash keeps `git log` on main clean and bisectable.
- Never force-push to main. If main has diverged, update the branch instead.
- If `--delete-branch` fails (branch already deleted), that's fine — it's idempotent.
- After merging, sync your local main immediately to avoid stale state on the next task.
