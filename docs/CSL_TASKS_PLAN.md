# CSL Tasks Implementation Plan

## Overview

A general-purpose Rust CLI tool for managing project tasks with GitHub Issues synchronization, drift detection, and flexible task lifecycle management. Designed to work with any LLM or human workflow, not tied to specific AI assistants.

## 1. Core Architecture

### Task Model
```rust
pub struct Task {
    pub id: u32,
    pub subject: String,
    pub description: String,
    pub active_form: String,
    pub status: TaskStatus,
    pub blocks: Vec<u32>,
    pub blocked_by: Vec<u32>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub github_issue: Option<u32>,
    pub content_hash: String,
}

pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Deleted,
}
```

### Storage Format
- **Location**: `tasks/` directory in project root
- **Format**: Individual Markdown files with YAML frontmatter per task (`{id}.md`)
- **Benefits**:
  - Human-readable and editable (plain text Markdown)
  - Git-friendly (merge conflicts only affect individual tasks, meaningful diffs)
  - Matches GitHub Issues format (seamless sync)
  - Self-documenting (task files are their own documentation)
  - No serialization overhead (edit in any text editor)

**Example Task File** (`tasks/31.md`):
```markdown
---
id: 31
subject: Update post-processing passes for new bibliography architecture
status: pending
blocks: []
blocked_by: []
priority: high
impact: medium
phase: 2
parent_task: 30
depends_on_pr: 117
created: 2026-02-06T10:30:00Z
content_hash: abc123def456...
---

Follow-up to PR #117: Post-processing passes in main.rs were written for the old architecture and are creating duplicate components or conflicts with the new occurrence-based compilation.

## Problem Analysis

- Post-processing in main.rs still expects old architecture
- Creating extra volume/issue entries (duplicates)
- May be interfering with correct component extraction

## Expected Outcome

- No duplicate component entries
- Bibliography extraction matches template_compiler output
- Simpler main.rs without obsolete processing
```

### Content Hashing
- **Purpose**: Detect changes to task definitions
- **Algorithm**: SHA-256 of `{subject}:{description}:{metadata}`
- **Use Cases**:
  - Drift detection between local and GitHub
  - Change tracking across sessions
  - Selective sync (only changed tasks)

## 2. CLI Interface

### Basic Commands
```bash
# Task Management
csl-tasks list [--status <status>] [--format <json|table>]
csl-tasks get <id>
csl-tasks create --subject "..." --description "..." [--metadata key=value]
csl-tasks update <id> [--status <status>] [--subject "..."] [--add-blocks <id>]
csl-tasks delete <id>

# GitHub Sync
csl-tasks sync [--dry-run] [--force] [--direction <to-gh|from-gh|both>]
csl-tasks sync-status  # Show drift summary
csl-tasks resolve-drift <id> [--use local|remote]

# Workflow Helpers
csl-tasks next  # Find next unblocked pending task
csl-tasks claim <id>  # Set status to in_progress
csl-tasks complete <id>  # Set status to completed
csl-tasks graph [--format dot|ascii]  # Visualize task dependencies
```

### Configuration
```toml
# tasks/config.toml (or .csl-tasks.toml in project root)
[github]
repo = "owner/repo"
label = "task"  # Label to identify synced issues
sync_metadata = true  # Sync custom metadata as YAML frontmatter

[local]
task_dir = "tasks"  # Directory for task markdown files
archive_completed = true  # Move completed to tasks/archive/

[sync]
auto_sync = false  # Prompt before sync
conflict_strategy = "prompt"  # prompt|local|remote
preserve_github_labels = true  # Don't remove non-task labels
```

## 3. GitHub Synchronization

### Sync Model
**Bidirectional**: Local tasks ↔ GitHub Issues

**Mapping**:
- `Task.id` → Issue custom field or label (`task-id:42`)
- `Task.subject` → Issue title
- `Task.description` → Issue body (Markdown content after frontmatter)
- `Task.status` → Issue state + labels
- `Task.metadata` → YAML frontmatter in issue body
- `Task.blocks` → Footer references ("**Blocks**: #123, #456")
- Local `tasks/31.md` ↔ GitHub Issue #142 (bidirectional sync)

**Example Issue Body** (synced from `tasks/31.md`):
```markdown
---
task_id: 31
priority: high
impact: medium
phase: 2
parent_task: 30
depends_on_pr: 117
content_hash: abc123def456...
---

Follow-up to PR #117: Post-processing passes in main.rs were written for the old architecture and are creating duplicate components or conflicts with the new occurrence-based compilation.

## Problem Analysis

- Post-processing in main.rs still expects old architecture
- Creating extra volume/issue entries (duplicates)
- May be interfering with correct component extraction

## Expected Outcome

- No duplicate component entries
- Bibliography extraction matches template_compiler output
- Simpler main.rs without obsolete processing

---
**Blocks**: #124, #125
**Blocked By**: None
```

### Sync Algorithm
```rust
async fn sync_bidirectional() -> Result<SyncReport> {
    let local_tasks = load_local_tasks()?;
    let github_issues = fetch_github_issues().await?;

    let mut changes = Vec::new();

    // Match by csl_task_id
    for task in &local_tasks {
        match find_github_issue(task.id, &github_issues) {
            Some(issue) => {
                // Drift detection
                if task.content_hash != issue.content_hash {
                    changes.push(Conflict::ContentDrift { task_id, issue_num });
                }
                if task.status != map_issue_state(&issue) {
                    changes.push(Conflict::StatusMismatch { task_id, issue_num });
                }
            }
            None => changes.push(Change::CreateIssue(task.clone())),
        }
    }

    // Find GitHub issues not in local
    for issue in &github_issues {
        if let Some(task_id) = extract_task_id(&issue) {
            if !local_tasks.contains_id(task_id) {
                changes.push(Change::CreateLocal(issue.clone()));
            }
        }
    }

    // Apply changes (with user confirmation)
    apply_changes(changes).await
}
```

### Conflict Resolution Strategies
1. **Prompt** (default): Show diff, ask user to choose
2. **Local Wins**: Always prefer local task data
3. **Remote Wins**: Always prefer GitHub issue data
4. **Last Modified**: Use timestamp to determine winner

## 4. Drift Detection

### Drift Types
1. **Content Drift**: Task description/metadata changed in one location
2. **Status Mismatch**: Task marked completed locally but issue still open
3. **Orphaned Tasks**: Local task with no matching GitHub issue
4. **Orphaned Issues**: GitHub issue with csl-task label but no local task
5. **Dependency Drift**: `blocks`/`blocked_by` differ from GitHub references

### Detection Report
```bash
$ csl-tasks sync-status

Drift Summary:
  Content Drift: 2 tasks
  Status Mismatch: 1 task
  Orphaned Tasks: 0
  Orphaned Issues: 1

Details:
  Task 31 ↔ Issue #142
    Local:  priority: high, phase: 2
    Remote: priority: medium, phase: 2
    Hash:   local=abc123 remote=def456

  Task 32 ↔ Issue #143
    Local:  status=completed
    Remote: state=open

  Issue #144 (orphaned)
    Title: "Fix delimiter extraction"
    No matching local task found
```

### Resolution Commands
```bash
# Resolve individual conflict
csl-tasks resolve-drift 31 --use local   # Update GitHub from local
csl-tasks resolve-drift 31 --use remote  # Update local from GitHub

# Bulk resolution
csl-tasks sync --force --direction to-gh    # Push all local changes
csl-tasks sync --force --direction from-gh  # Pull all remote changes
```

## 5. Task Lifecycle

### State Transitions
```
Pending → InProgress → Completed
         ↓
      Deleted (soft delete, preserves history)
```

### Dependency Management
- **Blocks**: Tasks that cannot start until this task completes
- **Blocked By**: Tasks that must complete before this task can start
- **Validation**: Circular dependency detection
- **Auto-Unblock**: When task moves to `Completed`, remove from `blocked_by` of dependent tasks

### Querying Available Work
```rust
pub fn find_next_task(tasks: &[Task]) -> Option<&Task> {
    tasks.iter()
        .filter(|t| t.status == TaskStatus::Pending)
        .filter(|t| t.blocked_by.is_empty())
        .min_by_key(|t| t.id)  // Lowest ID = highest priority
}
```

## 6. Metadata System

### Core Fields (built-in)
- `id`, `subject`, `description`, `status`, `blocks`, `blocked_by`

### Extension Fields (in `metadata`)
- `priority`: "critical" | "high" | "medium" | "low"
- `impact`: "100%" | "high" | "medium" | "low"
- `phase`: "1" | "2" | "3" (development phase)
- `parent_task`: Reference to parent task ID
- `depends_on_pr`: GitHub PR number
- `created`: ISO 8601 timestamp
- `completed`: ISO 8601 timestamp

### Custom Metadata
Users can add arbitrary key-value pairs:
```bash
csl-tasks create \
  --subject "Fix bibliography" \
  --metadata priority=high \
  --metadata assignee=alice \
  --metadata estimated_hours=8
```

## 7. LLM Integration

### Direct File Access (Primary Interface)

LLMs access tasks by reading/writing Markdown files in `tasks/` directory:

```markdown
# LLM reads a task
Read: tasks/31.md

# LLM updates task status
Edit: tasks/31.md (change status: pending -> in_progress)

# LLM creates new task
Write: tasks/42.md (with YAML frontmatter)

# LLM finds next task
Glob: tasks/*.md + filter by status=pending, blocked_by=[]
```

**Works with any LLM that has file system access** (Claude Code, Cursor, Aider, etc.)

### CLI Tool (Secondary - for Sync & Validation)

The `csl-tasks` CLI is primarily for:

1. **GitHub Sync**: `csl-tasks sync`
2. **Validation**: Detect circular dependencies, validate references
3. **Convenience**: `csl-tasks next` (find unblocked work)
4. **Conflict Resolution**: `csl-tasks resolve-drift`

LLMs can call CLI for these operations if needed:
```bash
csl-tasks sync --dry-run
csl-tasks next  # Alternative to manual file scanning
```

### Workflow Example

```
1. LLM: Glob tasks/*.md, filter status=pending
2. LLM: Read tasks/31.md
3. LLM: Edit tasks/31.md (status: in_progress)
4. LLM: Do the work...
5. LLM: Edit tasks/31.md (status: completed)
6. Human: csl-tasks sync (push to GitHub)
```

## 8. Error Handling

### Network Failures
- Retry with exponential backoff (3 attempts)
- Cache last successful sync timestamp
- Offline mode: Queue changes for later sync

### GitHub API Limits
- Rate limit detection (X-RateLimit-Remaining header)
- Graceful degradation: Skip sync if rate-limited
- Use conditional requests (ETag) to save quota

### Data Validation
- Validate task references (`blocks`/`blocked_by` point to real tasks)
- Circular dependency detection
- JSON schema validation for metadata

### Conflict Resolution
- Never overwrite without user confirmation (unless `--force`)
- Always show diff before destructive operations
- Preserve both versions in case of unsolvable conflicts

## 9. Testing Strategy

### Unit Tests
- Task CRUD operations
- Dependency graph validation
- Content hash computation
- Status transition logic

### Integration Tests
- GitHub API mock (octocrab + wiremock)
- Sync scenarios (create, update, conflict)
- Drift detection accuracy
- Markdown parsing/serialization round-trips

### End-to-End Tests
- Real GitHub repo (test organization)
- Full sync workflow
- Conflict resolution flow
- CLI command parsing

### Test Data
- Fixture tasks in `tests/fixtures/tasks/`
- Sample GitHub responses in `tests/fixtures/github/`
- Dependency graphs for validation testing

## 10. Implementation Phases

### Phase 1: Core Task Management (Week 1)
- [x] Define `Task` struct and `TaskStatus` enum
- [ ] Implement Markdown+YAML frontmatter parsing/serialization
- [ ] File I/O for `tasks/*.md` files
- [ ] CLI commands: `list`, `get`, `create`, `update`, `delete`
- [ ] Dependency validation (circular detection)
- [ ] Unit tests for core logic

### Phase 2: GitHub Integration (Week 2)
- [ ] GitHub API client (octocrab)
- [ ] Issue ↔ Task mapping
- [ ] Sync algorithm (unidirectional: to GitHub first)
- [ ] YAML frontmatter parsing/generation
- [ ] Integration tests with API mocks

### Phase 3: Drift Detection (Week 3)
- [ ] Content hashing
- [ ] Drift detection algorithm
- [ ] Conflict reporting
- [ ] Resolution strategies (prompt, local, remote)
- [ ] `sync-status` and `resolve-drift` commands

### Phase 4: Workflow Enhancements (Week 4)
- [ ] Dependency graph visualization (`graph` command)
- [ ] `next` command (find unblocked work)
- [ ] Session archiving
- [ ] Configuration file support
- [ ] Documentation and examples

### Phase 5: Polish & Production (Week 5)
- [ ] Error handling improvements
- [ ] Performance optimization (parallel API calls)
- [ ] User-friendly error messages
- [ ] Shell completion scripts
- [ ] CI/CD integration

## Dependencies

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
tokio = { version = "1.0", features = ["full"] }
octocrab = "0.38"      # GitHub API client
sha2 = "0.10"          # Content hashing
anyhow = "1.0"         # Error handling
thiserror = "1.0"      # Custom error types
gray_matter = "0.2"    # YAML frontmatter parsing (or similar)
pulldown-cmark = "0.9" # Markdown parsing/rendering
```

## References

- GitHub Issues API: https://docs.github.com/en/rest/issues
- YAML Frontmatter: https://jekyllrb.com/docs/front-matter/
- Task Management Best Practices: Todoist, Linear, Jira workflows
- Markdown + Frontmatter: Jekyll, Hugo, Obsidian patterns
