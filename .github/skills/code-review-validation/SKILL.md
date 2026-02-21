---
name: code-review-validation
description: Validate and action code review feedback from Copilot or other reviewers. Accept improvements, reject with reasoning, or defer with detailed issues.
---

# Skill: Validate Code Review Feedback

When Copilot or other reviewers provide feedback on a PR, you must carefully evaluate each suggestion. Don't accept blindly—validate against the codebase, project conventions, and implementation best practices.

## Prerequisite: Confirm the Copilot review has been posted

**Before starting**, fetch the live PR state:

```bash
gh pr view <PR_NUMBER> --json reviews
```

Check that `copilot-pull-request-reviewer` appears in the `reviews` list. If it is absent, **stop and wait** — the bot runs asynchronously after PR creation and may take several minutes.

```bash
# Also fetch all inline review threads
gh api graphql -f query='{ repository(owner: "lqdev", name: "podcast-tui") { pullRequest(number: N) { reviewThreads(first: 50) { nodes { isResolved comments(first: 1) { nodes { body path line } } } } } } }'
```

Do **not** substitute an internal code reviewer for a missing Copilot review. The two are complementary but the Copilot review is the authoritative one that must be actioned on the PR threads.

**Only proceed once the Copilot review is confirmed posted.**

## Review Validation Process

### 1. Understand the Suggestion

- Read the full comment in context
- Identify what the reviewer is suggesting (fix, improvement, clarification)
- Note the file/line where feedback was given
- Check if the suggestion applies to current code (reviews can become outdated)

### 2. Evaluate Against Project Conventions

Before deciding, check:
- Does it align with existing patterns in the codebase?
- Is it consistent with AGENTS.md, copilot-instructions.md, and architecture docs?
- Does it match the project's style (naming, structure, error handling)?
- Are there counter-examples in the codebase suggesting a different approach?

**Reference these files:**
- `AGENTS.md` — architecture patterns, code standards, best practices
- `.github/copilot-instructions.md` — coding style and common patterns
- `CONTRIBUTING.md` — development workflow and testing requirements
- `docs/ARCHITECTURE.md` — system design and module organization
- Recent merged PRs (#70, #71, #72) for real-world examples of accepted patterns

### 3. Make a Decision

You have three options:

#### Option A: ACCEPT
The suggestion improves code quality, aligns with conventions, and fixes a real issue.

**Process:**
- Apply the fix using the exact code from the suggestion (if provided)
- Test immediately: `cargo test` (if code change) or manual verification (if doc change)
- Commit the change with a clear message explaining the fix
- Push to the branch
- Reply on the review thread: "Accepted — fixed in commit [SHA]"

**Example reply:**
```
Accepted. The old filenames were outdated after the skill migration. Updated all four references to use skill names instead of file paths, per the new structure.
```

#### Option B: REJECT
The suggestion doesn't align with the project, creates inconsistency, or introduces unnecessary complexity.

**Process:**
- Don't implement the change
- Reply on the review thread with clear reasoning:
  - Why the current approach is better
  - What pattern or convention it follows
  - A code example or reference if helpful
- If the reviewer's concern is valid but the suggestion isn't, offer an alternative

**Example reply:**
```
Rejected. The suggestion adds a new config field, but our pattern (as seen in src/constants.rs) is to centralize all defaults there, not scatter them across multiple files. The current approach keeps configuration in one place. See config.rs:45 for the existing pattern.
```

#### Option C: DEFER
The suggestion is valid but out of scope for this PR, requires architectural discussion, or needs a separate issue.

**Process:**
1. **Create a detailed GitHub issue** with:
   - Clear title tied to the feedback
   - Full context (what the reviewer suggested, why it matters)
   - Code references (file:line numbers, existing patterns)
   - Proposed solution (sketch if unclear)
   - Acceptance criteria (what "done" looks like)
   - Link to the original PR/review thread
   
2. **Make the issue shovel-ready** — an agent should be able to pick it up and implement immediately without asking questions
   
3. **Reply on the review thread:**
   ```
   Deferred to #NNN. This is outside the scope of the current PR but deserves dedicated attention. See issue for full context.
   ```

## Common Review Patterns

### "This file is too long / module should be split"
- **Action**: Usually DEFER — requires architectural discussion and testing
- **Decision factors**: Does the PR add to existing bloat or create new issues? If new, consider accepting a refactor. If existing, file a separate issue.

### "This pattern is inconsistent with X"
- **Action**: Usually ACCEPT if clear — consistency matters
- **Decision factors**: Check the referenced pattern exists in recent code. If it's historical but superseded, provide evidence.

### "Missing error handling / edge case"
- **Action**: Evaluate scope. ACCEPT if within current PR scope, DEFER if it's a systematic issue
- **Decision factors**: Is this PR introducing the risk or exposing existing risk? If introducing, fix now.

### "This should use Y instead of X"
- **Action**: Check project conventions first. ACCEPT if X was a mistake, REJECT if both are valid patterns used elsewhere
- **Decision factors**: Reference existing code showing the pattern. Ask: is there a documented reason for the current approach?

### "Add a test for this code path"
- **Action**: Almost always ACCEPT — testing is non-negotiable per Implementation Best Practices
- **Exception**: If the code path is already covered by integration tests, link to them and discuss scope

## Tips

1. **Link to evidence**: Reference specific files/commits when rejecting or deferring
2. **Be specific**: "Accepted" is weaker than "Accepted — fixes off-by-one in line 42"
3. **Stay collaborative**: Frame rejects/defers as "this approach is better because..." not "you're wrong"
4. **Check the date**: Code review comments can become stale. Verify the issue still exists before acting
5. **Test everything**: Even "obvious" fixes can break things. Test after each change
