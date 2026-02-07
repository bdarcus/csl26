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

## ID Management

### ID Strategy

The csl-tasks system uses GitHub issue numbers as task IDs for proper cross-referencing:

- **GitHub-backed tasks**: IDs 1-9999, matched to their GitHub issue number
- **Local-only tasks**: IDs 10000+, for tasks not yet synced to GitHub
- **Benefits**: Task references like "blocks task 14" now mean the same thing in local and GitHub contexts

### Migration to GitHub-Aligned IDs

When syncing with GitHub, task IDs should align with their issue numbers. Run migration once:

```
/task migrate-ids --dry-run         # Preview what would change
/task migrate-ids                   # Apply the migration (creates backup)
```

The migration:
1. Creates backup: `tasks-backup-YYYYMMDD-HHMMSS/`
2. Renumbers files to match GitHub issue numbers
3. Updates all blocker/blocks references atomically
4. Idempotent: safe to run multiple times

### Validation

Check for ID conflicts and invalid references:

```
/task validate                      # Check all tasks for issues
```

Reports:
- Duplicate task IDs
- Dangling references (blockers that don't exist)
- Corrupted state (mismatched ID/github_issue)

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

- `csl-tasks` CLI installed: `cargo install --path crates/csl-tasks`
- Initialized: `csl-tasks init` (creates `tasks/` directory)
- Optional: GitHub token for sync: `export GITHUB_TOKEN=ghp_...`

## Setup

First time:
```bash
cd ~/Code/csl26

# Install CLI
cargo install --path crates/csl-tasks

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

### Sync Error Handling
The sync command now gracefully handles errors:
- **Created**: New GitHub issues created successfully for tasks without github_issue
- **Updated**: Existing GitHub issues updated successfully
- **Skipped**: Tasks skipped due to:
  - Missing GitHub issues (deleted or inaccessible)
  - Permission errors (insufficient token permissions)
  - Deleted tasks that can't be synced
- **Failed**: Tasks that encountered unexpected errors (details shown in report)

Use `--dry-run` to preview what will be synced without making changes:
```
/task sync --direction to-gh --dry-run
/task sync --direction from-gh --dry-run
```

The sync command will:
1. Continue processing all tasks even if some fail
2. Print a summary report showing created/updated/skipped/failed tasks
3. Exit with status 1 if any tasks failed (to alert CI/automation)
4. Show clear reasons for skipped or failed tasks

### Dependencies
Tasks can block each other:
- `/task update 18 --add-blocks 17` (task 18 blocks task 17)
- Use `/task next` to find tasks with no blockers
- Use `/task graph` to visualize dependency chains

## Related Skills

- No deprecation: task-next skill still available but `/task next` is preferred
- See `CLAUDE.md` Task Management Workflow section for GitHub Issues conventions
