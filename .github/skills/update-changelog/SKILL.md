---
name: update-changelog
description: Add a new entry to CHANGELOG.md for a feature, fix, or release. Handles both [Unreleased] additions and splitting a release section.
---

# Skill: Update CHANGELOG.md

## When to use
- After implementing a feature, bug fix, or significant change
- When creating a new version release (splitting [Unreleased] into a versioned section)

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

## Rules
- Keep entries user-facing (not implementation details)
- Include date context for major sections
- Reference commands, keybindings, and config fields
- List test counts for significant features
- Never delete existing entries
