# csl-tasks

A general-purpose Rust CLI tool for managing project tasks with GitHub Issues synchronization, drift detection, and flexible task lifecycle management.

## Features

- **Markdown Storage**: Tasks stored as individual `.md` files with YAML frontmatter
- **GitHub Sync**: Bidirectional synchronization with GitHub Issues
- **Drift Detection**: Identify conflicts between local and remote state
- **Dependency Management**: Track task dependencies with `blocks` and `blocked_by`
- **Task Lifecycle**: Pending → InProgress → Completed workflow
- **Graph Visualization**: Visualize task dependencies in ASCII or DOT format
- **Archiving**: Move completed tasks to archive directory
- **LLM-Friendly**: Works with any LLM that has file system access

## Installation

```bash
cargo build --package csl-tasks --release
```

## Quick Start

```bash
# Create a new task
csl-tasks create \
  --subject "Fix bibliography rendering" \
  --description "Update post-processing passes for new architecture"

# List all tasks
csl-tasks list

# View task details
csl-tasks get 1

# Update task status
csl-tasks claim 1      # Set to in_progress
csl-tasks complete 1   # Set to completed

# Find next available task
csl-tasks next

# Visualize dependencies
csl-tasks graph --format ascii
csl-tasks graph --format dot > tasks.dot

# Archive completed tasks
csl-tasks archive --dry-run  # Preview what would be archived
csl-tasks archive            # Actually archive

# Validate task dependencies
csl-tasks validate
```

## GitHub Synchronization

```bash
# Set environment variables
export GITHUB_TOKEN="ghp_..."
export GITHUB_REPO="owner/repo"

# Sync local tasks to GitHub (create/update issues)
csl-tasks sync --direction to-gh

# Check drift between local and remote
csl-tasks sync-status

# Dry run (preview changes)
csl-tasks sync --dry-run
```

## Task File Format

Tasks are stored as Markdown files with YAML frontmatter in the `tasks/` directory:

```markdown
---
id: 1
subject: Fix bibliography rendering
status: pending
blocks: []
blocked_by: []
priority: high
phase: 2
content_hash: abc123...
---

Update post-processing passes in main.rs for new architecture.

## Problem

- Post-processing expects old architecture
- Creating duplicate volume/issue entries

## Solution

- Remove obsolete post-processing
- Update component extraction logic
```

## Dependency Management

Tasks can depend on each other using `blocks` and `blocked_by`:

```bash
# Task 2 cannot start until task 1 is completed
csl-tasks update 2 --add-blocked-by 1

# Task 1 blocks task 2
csl-tasks update 1 --add-blocks 2
```

The system automatically detects circular dependencies:

```bash
csl-tasks validate
# Error: Circular dependency detected: 1 -> 2 -> 3 -> 1
```

## LLM Integration

LLMs can work with tasks by directly reading/writing Markdown files:

```python
# Read a task
content = read_file("tasks/1.md")

# Update task status
edit_file("tasks/1.md", old="status: pending", new="status: in_progress")

# Find next task
files = glob("tasks/*.md")
tasks = [parse_task(f) for f in files]
next_task = min(
    (t for t in tasks if t.status == "pending" and not t.blocked_by),
    key=lambda t: t.id
)
```

## CLI Reference

### Task Management

- `list [--status <status>] [--format <json|table>]` - List all tasks
- `get <id> [--format <json|text>]` - Get task details
- `create --subject "..." --description "..." [--metadata key=value]` - Create task
- `update <id> [--status <status>] [--subject "..."] [--add-blocks <id>]` - Update task
- `delete <id>` - Delete task
- `next [--format <json|text>]` - Find next unblocked pending task
- `claim <id>` - Set status to in_progress
- `complete <id>` - Set status to completed
- `validate` - Validate task dependencies

### GitHub Sync

- `sync [--dry-run] [--direction <to-gh|from-gh|both>]` - Sync with GitHub
- `sync-status` - Show drift between local and remote

### Visualization

- `graph [--format <ascii|dot>]` - Visualize task dependency graph

### Archiving

- `archive [--dry-run]` - Archive completed tasks to `tasks/archive/`

## Configuration

Create `.csl-tasks.toml` or `tasks/config.toml`:

```toml
[github]
repo = "owner/repo"
label = "task"
sync_metadata = true

[local]
task_dir = "tasks"
archive_completed = true

[sync]
auto_sync = false
conflict_strategy = "prompt"  # prompt|local|remote
preserve_github_labels = true
```

## Implementation Status

- ✅ Phase 1: Core task management
- ✅ Phase 2: GitHub synchronization (to-gh direction)
- ✅ Phase 3: Drift detection
- ✅ Phase 4: Workflow enhancements (graph, archive, config)
- ⏳ Phase 5: Polish & production

## License

Part of the CSL Next project.
