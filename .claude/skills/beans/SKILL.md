# Beans Task Management

**Type:** User-Invocable, Agent-Invocable
**LLM Access:** Yes
**Purpose:** Local-only task management using beans CLI for rapid development

## Overview

The `/beans` skill provides a streamlined interface to the beans task management system. Beans stores tasks as markdown files in `.beans/` with dependency tracking, priorities, and rich metadata. This skill is optimized for rapid development cycles where GitHub sync overhead is unnecessary.

## Commands

### List Tasks
```
/beans list [--status STATUS] [--priority PRIORITY] [--type TYPE]
```
Show all beans, optionally filtered by status/priority/type.

**Examples:**
- `/beans list` - Show all tasks
- `/beans list --status todo` - Show only todo tasks
- `/beans list --priority high` - Show high priority tasks
- `/beans list --type bug` - Show only bugs

### Show Next Task
```
/beans next
```
Recommend the next best task to work on based on:
1. Blockers (tasks blocking milestones)
2. Critical bugs in-progress
3. High priority todos

### Show Task Details
```
/beans show BEAN_ID
```
Display full bean contents including body and metadata.

### Create Task
```
/beans create "Title" [--type TYPE] [--priority PRIORITY] [--status STATUS] [--body TEXT]
```
Create a new bean.

**Types:** task (default), bug, feature, milestone, epic (only these 5 are valid)
**Priorities:** critical, high, normal (default), low, deferred
**Statuses:** todo (default), in-progress, draft, completed, scrapped

**Note:** For categorization beyond core types (e.g., tech-debt, refactor), use `--tag` instead:
```
/beans create "Title" --type task --tag tech-debt --tag refactor
```

**Examples:**
- `/beans create "Fix parser bug" --type bug --priority critical`
- `/beans create "Add new feature" --type feature --priority high --body "Detailed description here"`

### Update Task
```
/beans update BEAN_ID [--status STATUS] [--priority PRIORITY] [--blocking BLOCKER_ID]
```
Update bean properties.

**Examples:**
- `/beans update csl26-abc1 --status in-progress` - Mark as started
- `/beans update csl26-abc1 --status completed` - Mark as done
- `/beans update csl26-abc1 --blocking csl26-xyz2` - Add blocker relationship

### Delete Task
```
/beans delete BEAN_ID
```
Delete a bean permanently.

## Workflow Patterns

### Starting Work
```
/beans next              # Find recommended task
/beans show csl26-abc1   # Review details
/beans update csl26-abc1 --status in-progress
```

### Completing Work
```
/beans update csl26-abc1 --status completed
/beans next              # Find next task
```

### Creating Related Tasks
```
/beans create "Parent task" --type milestone
# Note the ID from output: csl26-parent
/beans create "Subtask 1" --blocking csl26-parent
/beans create "Subtask 2" --blocking csl26-parent
```

## Integration with AI Agents

When delegating to `@builder` or `@planner`, reference bean IDs in the task description:

```
@builder: Implement csl26-abc1 - Fix delimiter handling
```

The agent can then query bean details using `/beans show csl26-abc1`.

## Advantages Over GitHub Issues

- **Instant queries:** No API rate limits or network latency
- **Dependency visualization:** Built-in blocking/parent relationships
- **Simple workflow:** Create → Work → Complete (no sync overhead)
- **Git-friendly:** Tasks are markdown files, easy to diff/review
- **Offline-first:** Works without internet connection

## Limitations

- **Local only:** No automatic GitHub sync (by design during rapid development)
- **Single user:** Not designed for team collaboration
- **No notifications:** Manual checking required for updates

## Future Extensions

If GitHub sync becomes necessary later:
- Add `/beans sync` command for manual push/pull
- Implement issue template mapping
- Add conflict resolution for divergent updates

## See Also

- `beans help` - Official CLI documentation
- `.beans.yml` - Project configuration
- `.beans/*.md` - Task markdown files