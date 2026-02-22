# PR Workflow Fast

**Type:** User-Invocable, Agent-Invocable
**LLM Access:** Yes
**Purpose:** Efficient branch/PR workflow with minimal ceremony and explicit quality gates.

## Use This Skill When
- You want a branch + PR quickly.
- You need consistent, reviewable PR descriptions from oracle/test outputs.
- You want checks selected by change type.

## Branch Policy
- Branch prefix: `codex/`.
- Name pattern: `codex/<scope>-<short-goal>`.
- Keep PR scope narrow and mergeable.

## Change-Type Gates
1. Docs/styles only (`.md`, `styles/*.yaml`):
   - syntax sanity + targeted rendering/oracle checks
2. Rust-touching (`.rs`, `Cargo.toml`, `Cargo.lock`):
   - `cargo fmt`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo nextest run` (fallback: `cargo test`)
3. Hot path/perf claims:
   - benchmark baseline/after via `./scripts/bench-check.sh`

## PR Body Template
- Summary: what changed and why.
- Scope: files and affected workflows.
- Validation: exact commands run + key results.
- Risk: potential regressions and mitigation.
- Follow-ups: non-blocking next steps.

## Efficiency Rules
- One oracle snapshot per iteration unless structure changes materially.
- Stop at target metric; avoid unbounded polish loops.
- Escalate to planner when style-only fixes stall.
- Prefer smallest diff that achieves gate pass.

## Merge Readiness Checklist
- Checks passed for touched change type.
- PR body includes objective evidence.
- No unresolved high-severity findings.
