#!/bin/bash
# tasks-to-issues.sh - Migrate TASKS.md to GitHub Issues
#
# Usage:
#   ./scripts/tasks-to-issues.sh --dry-run    # Preview without creating
#   ./scripts/tasks-to-issues.sh              # Create issues

set -euo pipefail

DRY_RUN=false
TASKS_FILE="docs/TASKS.md"

if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
    echo "🔍 DRY RUN MODE - No issues will be created"
    echo ""
fi

# Check dependencies
if ! command -v gh &> /dev/null; then
    echo "Error: gh CLI not found. Install from https://cli.github.com"
    exit 1
fi

# Verify authentication
if ! gh auth status &> /dev/null; then
    echo "Error: Not authenticated with GitHub. Run: gh auth login"
    exit 1
fi

# Task mapping: priority -> GitHub labels
map_priority() {
    case "$1" in
        HIGHEST|HIGH) echo "priority-high" ;;
        MEDIUM) echo "priority-medium" ;;
        LOW) echo "priority-low" ;;
        *) echo "priority-medium" ;;
    esac
}

# Task mapping: category -> GitHub labels
map_category() {
    local title="$1"
    local body="$2"
    local labels=""

    # Determine labels from title and content
    if [[ "$title" =~ [Ff]ix|[Bb]ug|[Rr]egression ]]; then
        labels="bug,rendering"
    elif [[ "$title" =~ [Rr]efactor|[Dd]ebt ]]; then
        labels="tech-debt,refactor"
    elif [[ "$title" =~ [Ss]upport|[Ii]mplement|[Aa]dd ]]; then
        labels="feature"
    else
        labels="feature"
    fi

    # Add category-specific labels
    if [[ "$body" =~ numeric.style ]]; then
        labels="$labels,numeric-styles"
    fi
    if [[ "$body" =~ multilingual|language ]]; then
        labels="$labels,i18n"
    fi
    if [[ "$body" =~ workflow|test|debug ]]; then
        labels="$labels,dx"
    fi

    echo "$labels"
}

# Extract task from markdown section
create_issue() {
    local task_num="$1"
    local title="$2"
    local body="$3"
    local priority="$4"
    local blocks="$5"
    local blocked_by="$6"

    # Build GitHub issue body
    local issue_body="## Task Details

$body

---

**Original Task:** #$task_num from TASKS.md
**Priority:** $priority"

    # Add dependencies
    if [[ -n "$blocked_by" ]]; then
        issue_body="$issue_body
**Blocked by:** $blocked_by"
    fi

    if [[ -n "$blocks" ]]; then
        issue_body="$issue_body
**Blocks:** $blocks"
    fi

    # Determine labels
    local priority_label=$(map_priority "$priority")
    local category_labels=$(map_category "$title" "$body")
    local all_labels="$priority_label,$category_labels"

    # Create or preview issue
    if [[ "$DRY_RUN" == true ]]; then
        echo "📋 Would create issue #$task_num:"
        echo "   Title: $title"
        echo "   Labels: $all_labels"
        echo ""
    else
        echo "Creating issue #$task_num: $title"
        gh issue create \
            --title "$title" \
            --body "$issue_body" \
            --label "$all_labels" \
            || echo "⚠️  Failed to create issue #$task_num"
    fi
}

# Parse TASKS.md and create issues
parse_tasks() {
    local in_task=false
    local task_num=""
    local title=""
    local body=""
    local priority="MEDIUM"
    local blocks=""
    local blocked_by=""

    while IFS= read -r line; do
        # Detect task start
        if [[ "$line" =~ ^###[[:space:]]+#([0-9]+):[[:space:]]+(.+)$ ]]; then
            # Save previous task if exists
            if [[ -n "$task_num" ]]; then
                create_issue "$task_num" "$title" "$body" "$priority" "$blocks" "$blocked_by"
            fi

            # Start new task
            task_num="${BASH_REMATCH[1]}"
            title="${BASH_REMATCH[2]}"
            body=""
            priority="MEDIUM"
            blocks=""
            blocked_by=""
            in_task=true
            continue
        fi

        # Skip if not in a task
        if [[ "$in_task" == false ]]; then
            continue
        fi

        # End of task (separator or next section)
        if [[ "$line" =~ ^---$ || "$line" =~ ^##[[:space:]] ]]; then
            in_task=false
            continue
        fi

        # Extract metadata
        if [[ "$line" =~ ^\*\*Priority:\*\*[[:space:]]+(.+)$ ]]; then
            priority="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^\*\*Blocked[[:space:]]by:\*\*[[:space:]]+(.+)$ ]]; then
            blocked_by="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^\*\*Blocks:\*\*[[:space:]]+(.+)$ ]]; then
            blocks="${BASH_REMATCH[1]}"
        else
            # Add to body
            body="$body$line"$'\n'
        fi
    done < "$TASKS_FILE"

    # Save last task
    if [[ -n "$task_num" ]]; then
        create_issue "$task_num" "$title" "$body" "$priority" "$blocks" "$blocked_by"
    fi
}

# Main execution
echo "🔄 Migrating tasks from $TASKS_FILE to GitHub Issues"
echo ""

# Only migrate active tasks (exclude completed)
echo "📊 Task Summary:"
grep -c "^### #[0-9]" "$TASKS_FILE" | xargs echo "   Total tasks found:"
grep -c "^### #[0-9].*📋" "$TASKS_FILE" | xargs echo "   Active tasks (📋):"
echo ""

# Run migration
parse_tasks

if [[ "$DRY_RUN" == true ]]; then
    echo ""
    echo "✨ Dry run complete. Run without --dry-run to create issues."
else
    echo ""
    echo "✅ Migration complete! View issues at:"
    echo "   https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/issues"
fi
