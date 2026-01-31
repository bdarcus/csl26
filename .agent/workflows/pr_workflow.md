---
description: Standard workflow for AI agent to create and merge PRs
---

# AI PR Workflow

This workflow defines the standard process for an AI agent to implement changes and submit PRs.

## When to Use

Use this workflow when:
- Implementing a feature or fix that requires code changes
- Creating documentation that should be reviewed
- Any change that needs to go through the PR process

## Context Window Management

Large PRs can consume significant context. To maximize efficiency:

1. **Use targeted reads**: Read specific files rather than exploring broadly
2. **Use Task agents**: Delegate exploration to subagents when searching
3. **Avoid re-reading**: Don't re-read files already in context
4. **Batch operations**: Combine independent tool calls in single messages
5. **Track progress**: After completing a PR, report remaining context percentage

**At the end of each PR workflow**, report:
```
Remaining context: ~XX%
```

## Workflow Steps

### 1. Setup

```bash
# Ensure on latest main
git checkout main && git pull --rebase

# Create feature branch
git checkout -b <type>/<short-description>
```

**Branch types**: `feat/`, `fix/`, `refactor/`, `docs/`, `test/`

### 2. Implement Changes

Make the necessary code changes. Follow guidelines in [AGENTS.md](../AGENTS.md).

### 3. Pre-Commit Verification

**CRITICAL**: Run these commands and fix ALL issues before committing.

```bash
# Format code (REQUIRED - CI fails without this)
cargo fmt --all

# Lint check (CI treats warnings as errors)
cargo clippy

# Run tests
cargo test
```

**For rendering changes**, also run oracle verification:

```bash
cd scripts && node oracle-e2e.js ../styles/apa.csl
```

### 4. Commit

```bash
git add <specific-files>  # Never use -A blindly

git commit -m "<type>(<scope>): <description>

<body explaining what and why>"
```

**Commit rules**:
- Use conventional commit format
- Lowercase subject line
- **Plain text body only**: No Markdown in the commit body (e.g., use `Result`, not `` `Result` ``).
- **No escaped backticks**: Do not escape backticks in messages (use `code`, never \`code\`)
- NO `Co-Authored-By` footer
- Exclude debug files, `.env`, temp files

### 5. Push and Create PR

```bash
git push -u origin <branch-name>

# NOTE: When generating the body, do NOT escape backticks.
# Correct: "Added `MyStruct`"
# Incorrect: "Added \`MyStruct\`"


gh pr create --title "<type>(<scope>): <description>" --body "$(cat <<'EOF'
## Summary
- Key changes as bullet points

## Test Results
- Relevant test results

EOF
)"
```

### 6. Wait for CI

```bash
# Check CI status (may need to wait/retry)
gh pr checks <pr-number>
```

**If CI fails**:
1. Get failure details: `gh run view <run-id> --log-failed`
2. Fix the issue locally
3. Amend: `git commit --amend --no-edit`
4. Force push: `git push --force`
5. Repeat until CI passes

### 7. Await Maintainer Review

**IMPORTANT: Do NOT merge the PR yourself.** 

After CI passes:
1. Notify the user that the PR is ready for review
2. **Stop and wait** for a maintainer to approve and merge
3. Only proceed to cleanup after the PR has been merged by a maintainer

```
PR #XX is ready for review: <URL>
CI passed. Awaiting maintainer approval before merge.
```

**Exception**: The maintainer may explicitly instruct you to merge a specific PR. 
In that case, use rebase merge:

```bash
gh pr merge <pr-number> --rebase
```

### 8. Cleanup

```bash
git checkout main && git pull --rebase
git branch -d <branch-name>
```

### 9. Report Status

After completing the PR workflow, report to the user:
- PR URL and CI status
- Brief summary of changes
- **Remaining context window percentage**

Example:
```
PR #31 merged: https://github.com/user/repo/pull/31

Changes: Added X, modified Y

Remaining context: ~45%
```

## Quick Reference

Full workflow in one block:

```bash
# Setup
git checkout main && git pull --rebase
git checkout -b feat/my-feature

# ... make changes ...

# Verify
cargo fmt --all && cargo clippy && cargo test

# Commit
git add <files>
git commit -m "feat(scope): description"

# PR
git push -u origin feat/my-feature
gh pr create --title "feat(scope): description" --body "## Summary
..."

# Wait for CI
gh pr checks <pr-number>  # Repeat until pass

# STOP HERE - await maintainer approval
# "PR #XX ready for review. CI passed. Awaiting maintainer approval."

# After maintainer merges:
git checkout main && git pull --rebase
git branch -d feat/my-feature

# Report
# "Remaining context: ~XX%"
```

## Common CI Failures

| Error | Fix |
|-------|-----|
| Clippy: large enum variant | Box the large variant: `Variant(Box<T>)` |
| Clippy: unused import | Remove the import |
| Format check failed | Run `cargo fmt --all` |
| Test failure | Fix the failing test |

## Decision Points

### When to Run Oracle Tests

Run oracle tests when changes affect:
- `csln_processor/src/render.rs`
- `csln_processor/src/values.rs`
- `csln_migrate/src/` (any file)
- Template or options structures

### When to Amend vs New Commit

- **Amend**: Fixing CI failures, typos, forgotten files
- **New commit**: Additional functionality, responding to review feedback

### What Files to Stage

**Always stage explicitly**. Never use `git add -A` without checking status first.

**Include**:
- Source files (`.rs`)
- Config files (`Cargo.toml`, `Cargo.lock`)
- Documentation (`.md` in `docs/` or `.agent/`)

**Exclude**:
- Debug files (`debug_*.yaml`, `*_analysis.txt`)
- Temporary files
- IDE config (`.idea/`, `.vscode/`)
- Environment files (`.env`)
