---
name: work-on-issue
description: Pick up a GitHub issue, implement it, and submit a PR. The standard workflow for any assigned task.
---

# Skill: Work on a GitHub Issue

## When to use
When you are assigned a GitHub issue to implement (feature, bug fix, refactoring, or documentation task).

## Steps

### 1. Read the issue
- Read the full issue body: description, acceptance criteria, implementation notes, file references
- If the issue is a sub-issue of an epic, read the epic for architectural context
- Check for dependencies (`Depends on #X`) — do not start if dependencies are unresolved

### 2. Check the project board
- Open the [GitHub Projects board](https://github.com/users/lqdev/projects/1)
- Note the issue's Priority, Phase, and Effort fields for context
- Confirm the issue is in `Todo` status (not blocked or already in progress)

### 3. Create a branch

```bash
# From main, create a branch tied to the issue number
git checkout main
git pull origin main
git checkout -b feat/issue-{N}-short-description
```

Use the appropriate type prefix: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `chore/`

### 4. Implement the change
- Follow the project's code standards (see `AGENTS.md` and `.github/copilot-instructions.md`)
- Use existing skills where applicable:
  - Adding a buffer? → See the `add-new-buffer` skill
  - Adding a command? → See the `add-new-command` skill
  - Architectural decision? → See the `create-adr` skill
  - Design document needed? → See the `create-rfc` skill
- Make minimal, surgical changes — don't refactor unrelated code
- Write tests for new functionality
- Filing a new issue along the way? → See the `create-issue` skill
- Need to triage / label? → See the `triage-issue` skill

### 5. Run quality checks

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

All three must pass before committing.

### 6. Commit with conventional format

```bash
git add .
git commit -m "feat(scope): brief description

Detailed explanation of what changed and why.

Closes #N"
```

- Use the conventional commit types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
- Reference the issue in the commit footer with `Closes #N`
- For partial work toward an epic, use `Part of #N` instead

### 7. Push and create a PR

```bash
git push origin feat/issue-{N}-short-description
```

Then create the PR. **Always use a PowerShell here-string for the body** — backtick is PowerShell's escape character, so any backtick-quoted code in a markdown body will corrupt or crash if passed as an inline string literal.

```powershell
# Single-quoted here-string: backticks, $vars, and quotes are all literal.
# The closing '@ MUST be at column 0 (no leading spaces).
$body = @'
## Summary

Closes #N — one-paragraph description.

## Changes

- `src/ui/app.rs`: what changed
- `src/ui/keybindings.rs`: what changed

## Tests

- N unit tests, N integration tests

## Quality

- `cargo fmt` ✅
- `cargo clippy -- -D warnings` ✅
- `cargo test` ✅
'@

gh pr create --title "feat(scope): description" --body $body --base main
```

PR requirements:
- Title in conventional commit format (`feat(scope): description`)
- `Closes #N` in the body
- Screenshots or terminal output for UI changes
- Note any testing done beyond automated tests

### 8. Update changelog (if user-facing)
If the change is user-facing, update `CHANGELOG.md` following the `update-changelog` skill **before pushing**. Add the entry to `[Unreleased]` and include it in the same commit as the code change. This is what prevents changelog gaps when multiple releases are cut quickly.

## Tips
- One issue per branch — don't combine unrelated changes
- If you discover a bug or improvement while working, file a new issue for it
- If the issue is unclear or missing acceptance criteria, ask for clarification before coding
- Check `src/constants.rs` before hardcoding any values
