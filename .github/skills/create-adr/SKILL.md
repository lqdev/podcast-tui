---
name: create-adr
description: Create a new Architecture Decision Record (ADR) documenting a significant design decision. Saves to docs/adr/ and updates the ADR index.
---

# Skill: Create an ADR

## When to use
When making a significant architectural choice that:
- Has non-obvious trade-offs
- Will affect how future code is written
- Supersedes a previous decision
- Would be confusing without documented rationale

## Steps

### 1. Determine the next ADR number
Read `docs/adr/README.md` to find the highest existing ADR number, then increment by 1.

### 2. Create the ADR file
Create `docs/adr/ADR-<NNN>-<short-title>.md`:

```markdown
# ADR-<NNN>: <Title>

**Status**: Proposed | Accepted | Deprecated | Superseded by ADR-XXX  
**Date**: YYYY-MM-DD  
**Deciders**: @lqdev

## Context

What situation, problem, or constraint required a decision?
What forces are at play (technical, organizational, timeline)?

## Decision

What was decided? Be specific and direct.
"We will use X for Y because Z."

## Consequences

**Positive:**
- Benefit 1
- Benefit 2

**Negative / Trade-offs:**
- Cost 1
- Cost 2

**Future options enabled:**
- What this makes easier later

## References
- Relevant source files
- Related issues or PRs
- External resources
```

### 3. Update the ADR index
Add a row to the table in `docs/adr/README.md`:

```markdown
| [ADR-NNN](ADR-NNN-short-title.md) | Title | Accepted | YYYY-MM-DD |
```

### 4. If superseding an existing ADR
Update the old ADR's **Status** line to:
```
**Status**: Superseded by [ADR-NNN](ADR-NNN-new-title.md)
```

## Good ADRs answer
1. What was the context / what problem were we solving?
2. What did we decide?
3. Why did we choose this over alternatives?
4. What are the trade-offs we accepted?
