# Plan: Add Project Management Documentation for AI Agents

## Problem
Agent instruction files (AGENTS.md, copilot-instructions.md, skills/) have thorough code guidance but zero project management workflow. Agents don't know how to discover work, name branches, update issue status, or interact with the GitHub Projects board.

## Approach
Three targeted, minimal additions — keep each file focused on its purpose. **UPDATE**: Expanded to include implementation best practices and code review validation skill.

## Completed Tasks

✅ **branch-setup** — Created `feat/agent-project-mgmt-docs` branch from main  
✅ **agents-md-section** — Added "🎯 Issue Workflow & Project Management" section to AGENTS.md  
✅ **agents-best-practices** — Added "🔨 Implementation Best Practices" section to AGENTS.md  
✅ **skill-work-on-issue** — Created `.github/skills/work-on-issue/SKILL.md`  
✅ **skill-code-review-validation** — Created `.github/skills/code-review-validation/SKILL.md`  
✅ **copilot-instructions-update** — Added minimal "Issue Workflow" section to `.github/copilot-instructions.md`  
✅ **skills-restructure** — Migrated all 6 skills from flat .md to subdirectory/SKILL.md format  
✅ **review-validation** — Validated code review feedback (1 comment) using new skill → ACCEPTED + fixed  
✅ **quality-checks** — Verified doc-only changes, no regressions  
✅ **commit-and-push** — Committed and pushed to origin

## Branch Status

**Current**: `feat/agent-project-mgmt-docs` (3 commits, all pushed)  
**PR**: #77 (Ready for merge — all review feedback addressed)

### Commits
1. `3342ad6` - docs: add project management guidance for AI agents
2. `74ebeaa` - docs: add implementation best practices to AGENTS.md
3. `2b4eddf` - docs: add code-review-validation skill and fix skill references

## Out of Scope (flag as future work)
- `sprint_task.yml` modernization (Sprint 0–7 → Phase-based)
- `CONTRIBUTING.md` stale sprint/develop-branch references
- `docs/IMPLEMENTATION_PLAN.md` historical sprint roadmap
