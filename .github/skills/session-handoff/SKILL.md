---
name: session-handoff
description: Write a checkpoint file at session end so the next session can pick up exactly where you left off. Captures what was done, what's next, key decisions, and reminders.
---

# Skill: Session Handoff

## When to use
- At the **end of a session**, before the user starts a new one
- When the user says "wrap up", "hand off", "save context", "new session", or similar
- After all PRs are merged or parked and no more active work remains in this session

## Steps

### 1. Determine the checkpoint number

Look for existing checkpoint files in the session folder:

```
C:\Users\lqdev\.copilot\session-state\<current-session-id>\checkpoints\
```

If the folder doesn't exist, create it. Name the file with the next sequential number:
- No existing files → `001-session-handoff.md`
- Highest is `003-*` → `004-session-handoff.md`

### 2. Gather what was accomplished

Collect these facts (do NOT guess — verify from Git and GitHub):

```powershell
# Recent commits on main since session started
git log --oneline -10

# Any open PRs from this session
gh pr list --state all --author @me --limit 5
```

For each PR merged or opened this session, note:
- PR number and title
- Issue it closed
- Merge commit SHA
- Key decisions or code review outcomes

### 3. Gather what's next

Use the `next-issue` skill (steps 1-4 only) to get the current top-5 stack rank. Include the full table.

### 4. Collect key reminders

Review the session for anything the next session should know:
- **Conventions learned or established** (e.g., new skill rules, project patterns discovered)
- **Decisions made** (e.g., "chose approach X over Y because Z")
- **Gotchas** (e.g., "test_opml_local_file is a pre-existing failure — ignore it")
- **In-flight work** (e.g., "PR #120 is open, awaiting review — check for comments before continuing")

### 5. Write the checkpoint file

Use this exact template. Every section is **required** — write `None` if a section genuinely has nothing.

```markdown
# Checkpoint NNN — Session Handoff

## What was accomplished this session

### [PR Title] — MERGED ✅ / OPEN 🔄 / DRAFT 📝
- One-line summary of what changed
- Merge commit: `SHA` on main (or: branch `name`, awaiting review)
- Code review: N comments — all ACCEPTED/REJECTED/DEFERRED (list highlights)
- Key files: `src/path/file.rs` (new), `src/path/other.rs` (modified)

(Repeat for each PR)

### Other changes (not PR-gated)
- Skill updates, doc fixes, etc.

## What's next (confirmed from project board)

| # | Issue | Priority | Phase | Effort |
|---|-------|----------|-------|--------|
| **N** | **Title** ← next | P1 | Phase 2 | M |
| ... | ... | ... | ... | ... |

## Key reminders for next session

1. Numbered list of things the next session must know
2. Include skill usage patterns, conventions, gotchas
3. Reference issue/PR numbers where relevant

## Recent main branch commits

```
SHA  commit message
SHA  commit message
SHA  commit message
```
```

### 6. Save the file

Write the checkpoint to:

```
C:\Users\lqdev\.copilot\session-state\<current-session-id>\checkpoints\NNN-session-handoff.md
```

### 7. Confirm to the user

Tell the user:
- The checkpoint file path
- A 2-3 sentence summary of what it contains
- Remind them to use the `session-resume` skill in the new session:

> Checkpoint saved. In your new session, say:
> "Read checkpoint `<path>` and resume using the `session-resume` skill."

## Rules

- **Verify, don't recall** — use `git log`, `gh pr view`, and `gh api graphql` to confirm facts. Don't reconstruct from memory.
- **Every section required** — even if it's `None`. A missing section in a checkpoint is a lost handoff.
- **Stack rank must be current** — always re-query the project board; don't copy from earlier in the session (issues may have been closed).
- **Include the checkpoint path in your response** — the user needs it to resume.
- **One checkpoint per handoff** — don't overwrite previous checkpoints. They're a history.

## Anti-patterns

- ❌ Writing the checkpoint from memory without verifying Git/GitHub state
- ❌ Omitting the "What's next" section because "they know"
- ❌ Skipping the stack rank query because "it hasn't changed"
- ❌ Writing vague reminders like "remember to check things" — be specific
