---
name: update-changelog
description: Add a new entry to CHANGELOG.md for a feature, fix, or release. Handles both [Unreleased] additions and splitting a release section.
---

# Skill: Update CHANGELOG.md

## When to use
- After implementing a feature, bug fix, or significant change
- When creating a new version release (splitting [Unreleased] into a versioned section)
- When backfilling missing entries for already-published releases

## Steps

### 1. Read the current CHANGELOG.md
Open `CHANGELOG.md` and find the `## [Unreleased]` section at the top.

### 2. Determine the change type
Select the appropriate subsection:
- **Added** — new feature or capability
- **Changed** — change in existing behavior
- **Fixed** — bug fix
- **Removed** — removed feature
- **Security** — security fix

### 3. Write the entry
Add under the correct subsection in `[Unreleased]`. Follow this format:

```markdown
## [Unreleased]

### Added

**Feature Name — Month Year**
- **Main thing**: One-line summary
  - Sub-detail with more context
  - Another sub-detail
  - Commands or keybindings introduced: `:command`, `Shift+D`
  - Config fields added: `downloads.new_field` (default: `true`)
  - Tests added: N unit tests
```

### 4. For a release: split [Unreleased] into a version

When tagging a release, rename `[Unreleased]` to the new version:

```markdown
## [1.7.0] - YYYY-MM-DD

### Added
<content from [Unreleased]>
```

Then add a fresh empty `## [Unreleased]` above it.

---

## Reconstructing historical entries (backfill)

Use this when multiple releases were cut without the changelog being maintained and you need to attribute features to the correct version.

> ⚠️ **Never infer version attribution from feature names, "Phase N" labels, or narrative alone.** Features named "Phase 2" and "Phase 3" may ship in different releases. Always verify with timestamps.

### Algorithm

**Step 1 — Get release timestamps**
```bash
gh api repos/lqdev/podcast-tui/releases --jq '.[] | "\(.tag_name) published: \(.published_at)"'
```

**Step 2 — Get the canonical PR list for each release**

GitHub auto-generates a PR list in every release body. This is the ground truth:
```bash
gh release view vX.Y.Z --repo lqdev/podcast-tui --json body
```
The body contains every PR merged between the previous and current tag, with links.

**Step 3 — Verify individual PR timing (if release body is ambiguous)**
```bash
gh pr view N --repo lqdev/podcast-tui --json number,title,mergedAt
```
A PR belongs to the **earliest release** whose `publishedAt` timestamp is >= the PR's `mergedAt`.

**Step 4 — Write entries per release**
For each release, group the PRs from its body by change type (Added / Changed / Fixed) and write entries following the format above.

### Example

```
v1.9.0 published: 2026-02-20T15:51:52Z
v1.8.0 published: 2026-02-19T17:15:41Z

PR #80 mergedAt: 2026-02-20T03:53:20Z  → in v1.9.0 (after v1.8.0 cutoff)
PR #58 mergedAt: 2026-02-18T18:36:38Z  → in v1.8.0 (after v1.7.0, before v1.8.0)
```

---

## Rules
- Keep entries user-facing (not implementation details)
- Include date context for major sections
- Reference commands, keybindings, and config fields
- List test counts for significant features
- Never delete existing entries
- When backfilling, the GitHub release body is the ground truth — not the existing CHANGELOG content or feature grouping names
