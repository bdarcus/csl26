# Task Next Skill

Fetch and present high-priority tasks from local task storage (instant).

## Description

This skill queries the local `csl-tasks` database to find the most important tasks to work on next. Uses local markdown files for instant queries (no API calls).

## Usage

```
/task-next
```

## What It Does

1. Queries local task database (`tasks/*.md`)
2. Filters by status (pending) and priority (high/medium)
3. Sorts by priority (high → medium → low)
4. Presents up to 5 actionable tasks
5. Shows task number, subject, priority, and impact
6. Links to GitHub issue if synced

## Output Format

```
📋 Top Priority Tasks:

1. Task #14: Fix year positioning for numeric styles
   Priority: HIGHEST | Impact: ~10,000+ issues
   GitHub: https://github.com/bdarcus/csl26/issues/127

2. Task #15: Support superscript citation numbers
   Priority: HIGH | Impact: Nature, Cell journals
   GitHub: https://github.com/bdarcus/csl26/issues/128

3. Task #17: Debug Springer citation regression
   Priority: HIGH | Impact: 460 dependent styles
   GitHub: https://github.com/bdarcus/csl26/issues/130

Which task would you like to work on?
```

## CLI Tool Used

- `csl-tasks list`: Fast local query (instant response)

## Requirements

- `csl-tasks` CLI installed: `cargo install --path crates/csl_tasks`
- Initialized in project: `csl-tasks init`
- Optional: Synced with GitHub: `csl-tasks sync pull`

## Setup

First time setup per project:
```bash
cd ~/Code/csl26
csl-tasks init
csl-tasks sync pull  # Import existing GitHub issues
```

## Advantages

- **Instant**: Reads local files (no API calls)
- **Offline**: Works without internet
- **Two-way sync**: Create tasks locally, sync to GitHub later
