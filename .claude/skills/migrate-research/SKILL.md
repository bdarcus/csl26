---
name: migrate-research
type: user-invocable, agent-invocable
description: Autonomous research loop that improves citum-migrate converter fidelity by iterating on Rust code in crates/citum_migrate/src/. Measures fidelity across a corpus of priority styles, clusters failure patterns, attempts targeted Rust fixes, validates with cargo nextest, keeps improvements, reverts failures. Does NOT modify style YAML.
model: sonnet
---

# Migrate Research

## Use This Skill When
- Systematic converter improvement is the goal.
- Multiple styles share the same migration failure pattern.
- You want to close the gap between `citum-migrate` output and reference fidelity.
- A style-fidelity follow-up needs bounded rich-input evidence to determine whether the next pass belongs in migration, style, processor, or adjudication.

## Do Not Use When
- Fixing a single style's YAML.
- The failure is an engine rendering bug, not a converter bug.
- The failure requires new schema fields.
- The only new signal is a full supplemental corpus rerun with no bounded cluster, hypothesis, or classification.

## Entry Points

```
/migrate-research
/migrate-research --top 20
/migrate-research --styles "apa,nature"
/migrate-research --resume
```

## Setup (read first)

Before doing anything else, read:
- `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md` — failure classification and stop conditions
- `docs/guides/STYLE_WORKFLOW_EXECUTION.md` — evidence ladder, cluster reduction, session loop

## Autonomous Operation

Run the full research loop without pausing for approval. Only interrupt for `Cargo.toml`/`Cargo.lock` changes or `git push origin main`.

## Workflow
- Use the shared execution guide for the evidence ladder, cluster reduction, and stop conditions.
- Run `node scripts/check-style-coverage-audits.js --status <style-id>` and
  report `current`, `stale`, or `not registered`. A current packet may select
  one bounded cluster; stale evidence is rejected; an unregistered style does
  not need a new packet.
- Do not attribute a cluster to `citum-migrate` from a hand-authored packet.
  Require a fresh migrated candidate from the relevant CSL source and
  reproduce the cluster first.
- Keep the file focused on the converter-specific loop and migration session state.

## Session State

Keep the existing `lab/` scratch structure under `.claude/skills/migrate-research/`.

## Output Contract

Report the chosen cluster, coverage-audit status, classification, before/after
evidence, disposition and joined-parity deltas when registered, exact change
made if any, and the final verdict on whether the pass should continue, stop,
or escalate.
