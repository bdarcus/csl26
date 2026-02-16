# Claude Code Native Tasks Migration

## Overview

This project has been migrated to use Claude Code native tasks for work tracking and planning. This document explains the new structure and how to work with it.

## What Changed

### Documentation Structure

**Before:**
```
.agent/
  ├── AGENTS.md (symlinked as CLAUDE.md)
  ├── PERSONAS.md
  ├── PRIOR_ART.md
  ├── design/
  ├── todo/         # Markdown TODO files
  ├── sessions/     # Old session logs
  └── workflows/    # Workflow docs
```

**After:**
```
./architecture/
  ├── PERSONAS.md
  ├── PRIOR_ART.md
  └── design/

.agent/
  └── skills/       # Project-specific skills only

CLAUDE.md          # Standalone project instructions
```

### Task Management

**Before:** TODO items scattered across:
- `.agent/todo/*.md` files
- GitHub issues
- `./TIER*_PLAN.md` files

**After:** All work tracked as native Claude tasks:
- Use `TaskList` to see all tasks
- Use `TaskCreate` to add new work items
- Use `TaskUpdate` to mark progress
- Proper dependency chains with `blockedBy`/`blocks`

## Current Tasks

Run `TaskList` to see current status. As of migration:

### High Priority (from corpus analysis)
- **Task #14**: Fix year positioning for numeric styles (HIGHEST - affects 10k+ entries)
- **Task #17**: Debug Springer citation regression

### Medium Priority
- **Task #12**: Fix conference paper template formatting
- **Task #13**: Refine bibliography sorting for anonymous works
- **Task #10**: Refactor delimiter handling with hybrid enum
- **Task #11**: Expand test data coverage to 20+ items

### Feature Requests (from GitHub issues)
- **Task #5**: Implement full document processing (#99)
- **Task #6**: Support language-dependent title formatting (#97)
- **Task #7**: Create style authors guide (#96)
- **Task #8**: Evaluate ICU library for date/time internationalization (#93)
- **Task #9**: Support automatic foot/endnoting of citations (#88)

### Blocked Tasks
- **Task #15**: Superscript citations (blocked by #14)
- **Task #16**: Volume/issue ordering (blocked by #14)

## Working with Global Agents

This project integrates with global `~/.claude/` agents:

### Available Agents

- **@planner**: Quick planning (≤3 questions with defaults)
- **@dplanner**: Deep planning with research capabilities
- **@builder**: Implementation specialist (2-retry cap, no questions)
- **@reviewer**: QA specialist with conflict detection

### Project-Specific Context

When agents are invoked on this project, they automatically receive:
- CSL domain knowledge
- Rust engineering standards
- Citation processing requirements
- Oracle verification workflows

This is handled through `CLAUDE.md` which acts as a context layer on top of global agent behavior.

### Specialized Style Agents

In addition to global agents, this project utilizes three specialized specialists for citation style authoring (accessible via the `/styleauthor` skill):

- **@dstyleplan**: The **Deep Architect**. Prioritizes correctness and holistic design. Conducts research and designs component trees using `sequential-thinking`.
- **@styleplan**: The **Architect**. Threshold: Maintenance & Simple Gaps. Provides technical build plans and Rust code snippets for the builder.
- **@styleauthor**: The **Builder** (Haiku). Implementation specialist with a 2-retry cap. Executes the plan without asking questions.

## Task Dependencies

The migration set up proper dependency chains:

```
Task #14 (Year positioning)
  ├─ blocks #15 (Superscript citations)
  └─ blocks #16 (Volume/issue ordering)
```

When working on tasks, always check `blockedBy` to ensure prerequisites are complete.

## Best Practices

1. **Create tasks proactively**: When you discover new work, create a task instead of documenting in markdown
2. **Use dependencies**: Set up `blockedBy`/`blocks` relationships to track order
3. **Update status**: Mark tasks `in_progress` when starting, `completed` when done
4. **Reference tasks**: Use `Task #N` in commit messages to link work to tasks
5. **Keep tasks focused**: Break large work into multiple tasks with dependencies

## Benefits of Native Tasks

1. **Structured tracking**: Tasks have metadata (status, dependencies, owner)
2. **Persistent state**: Tasks survive across sessions
3. **Queryable**: Can filter/sort tasks programmatically
4. **Integrated**: Works with Claude Code's planning and execution flow
5. **No duplication**: Single source of truth vs scattered markdown files

## Migration Notes

All previous TODO content has been preserved in task descriptions:
- GitHub issue context → Task description
- TODO analysis → Task implementation details
- TIER plan work → Broken into granular tasks

Historical session logs and workflows were archived (removed from repo) as they're now superseded by native task tracking.
