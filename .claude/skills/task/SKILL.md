# Task Skill

Comprehensive task management for CSLN project. Query, create, update, and sync tasks with GitHub Issues.

## Description

This skill provides a unified interface to the local task system (`csl-tasks` CLI). It supports:
- Listing and filtering tasks by status, priority, or search
- Creating and updating tasks locally
- Claiming tasks and marking them complete
- Syncing with GitHub Issues (two-way)
- Visualizing task dependencies
- Checking task-GitHub drift status

All operations are local-first (instant queries on markdown files), with optional GitHub synchronization.

## Usage

### List tasks
```
/task list                          # All tasks
/task list --status pending         # Filter by status
/task list --priority high          # Filter by priority (highest, high, medium, low)
/task list --search "year positioning"  # Search by subject/description
/task list --with-drift             # Include GitHub drift status
```

### Get task details
```
/task get 18                        # Show task #18 full details
```

### Find next actionable task
```
/task next                          # Recommend best task to work on next
/task next --priority high          # Recommend from high-priority tasks only
```

### Create task
```
/task create \
  --subject "Fix year positioning for numeric styles" \
  --description "Years should appear after volume/issue. Affects 10,000+ dependent styles." \
  --priority highest
```

### Update task
```
/task update 18 \
  --status in_progress \
  --description "Fixed year positioning. Testing with top 50 styles..."
```

### Claim task (mark in_progress)
```
/task claim 18
```

### Mark task complete
```
/task complete 18
```

### Sync with GitHub
```
/task sync                          # Sync local changes → GitHub
/task sync --direction from-gh      # Import updates from GitHub
/task sync --direction both         # Two-way sync
/task sync --dry-run                # Preview changes without applying
```

### Check sync status
```
/task sync-status                   # Show local-GitHub drift
```

### Visualize dependencies
```
/task graph                         # ASCII task dependency tree
/task graph --format dot            # Graphviz DOT format (for drawing tools)
```

## Output Format

### List Output (Markdown Table)
```
| Task | Subject | Priority | Status | GitHub |
|------|---------|----------|--------|--------|
| #18 | Fix year positioning | HIGHEST | pending | #127 ✓ |
| #17 | Support superscript | HIGH | in_progress | #128 ⚠ drift |
| #16 | Fix volume/issue | HIGH | pending | #129 ✓ |
| #15 | Debug Springer | HIGH | pending | ✗ none |
```

Drift indicators:
- `✓` = synced (no differences with GitHub)
- `⚠ drift` = content, status, or dependencies differ
- `✗ none` = no associated GitHub issue

### Next Output (Recommendation)
```
💡 **Recommended Task: #18 (Fix year positioning)**

Priority: HIGHEST
Impact: ~10,000+ dependent styles
Status: pending
Blockers: none
Blocked by: none

Reasoning: Highest priority, no blockers, affects largest portion of corpus.
This task unblocks #17 and #16.

👉 Run: /task claim 18
```

### Get Output (Full Details)
```
Task #18: Fix year positioning for numeric styles

Status: pending
Priority: HIGHEST
Impact: ~10,000+ dependent styles

Description:
Years should appear after volume/issue. Current implementation has issues
with numeric styles (Nature, Cell, etc.). Affects majority of corpus.

Blocks: #17, #16
Blocked by: none

GitHub Issue: https://github.com/bdarcus/csl26/issues/127 (synced)

Task Dir: tasks/
Last Modified: 2025-02-06 14:32:00
```

## Task Status Values

- `pending` - Not started
- `in_progress` - Currently being worked on
- `blocked` - Waiting for something else
- `completed` - Done
- `archived` - Historical (hidden by default)

## Priority Values

- `lowest` - Nice to have
- `low` - Eventually
- `medium` - Soon
- `high` - Needed soon
- `highest` - Critical path

## Requirements

- `csl-tasks` CLI installed: `cargo install --path crates/csl_tasks`
- Initialized: `csl-tasks init` (creates `tasks/` directory)
- Optional: GitHub token for sync: `export GITHUB_TOKEN=ghp_...`

## Setup

First time:
```bash
cd ~/Code/csl26

# Install CLI
cargo install --path crates/csl_tasks

# Initialize
csl-tasks init

# Optional: sync existing GitHub issues
csl-tasks sync --direction from-gh
```

## CLI Tool Used

- `csl-tasks` CLI with JSON output for structured parsing

## Important Notes

### Local-First Architecture
- Task queries are instant (local markdown files)
- Sync to GitHub is optional
- Create/update tasks locally, sync later when convenient
- GitHub token only needed for `sync` and `sync-status` commands

### Sync Workflow
The skill recommends local-first workflow:
1. Create tasks locally: `/task create --subject "..."`
2. Work on tasks: `/task claim`, `/task update`
3. Mark done: `/task complete`
4. Sync to GitHub when ready: `/task sync --direction to-gh`

### Dependencies
Tasks can block each other:
- `/task update 18 --add-blocks 17` (task 18 blocks task 17)
- Use `/task next` to find tasks with no blockers
- Use `/task graph` to visualize dependency chains

## Related Skills

- No deprecation: task-next skill still available but `/task next` is preferred
- See `CLAUDE.md` Task Management Workflow section for GitHub Issues conventions
