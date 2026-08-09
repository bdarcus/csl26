---
name: style-maintain
type: agent-invocable
description: "Fast targeted maintenance for an existing Citum style. Use for punctuation/layout bugs, missing type-variants entries, or syntax modernization. Not for migrations or batch waves."
model: haiku
---

# Style Maintain

Authoritative shared process docs:
- `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md`
- `docs/guides/STYLE_WORKFLOW_EXECUTION.md`

## Use This Skill When
- Updating one style for punctuation/layout bugs.
- Adding a missing `type-variants` entry.
- Modernizing style syntax without changing rendered output intent.

## Input Contract
- Existing style path in `styles/`.
- One focused objective.
- Optional reference oracle style in `styles-legacy/`.

## Autonomous Operation

Run the full fix loop without pausing for approval. Use the shared docs for the common decision and execution flow. Only interrupt for `Cargo.toml`/`Cargo.lock` changes or `git push origin main`.

## Workflow

1. Read `docs/adjudication/DIVERGENCE_REGISTER.md` before the first oracle run.
2. Run `node scripts/check-style-coverage-audits.js --status <style-id>` and
   report `current`, `stale`, or `not registered`. For a current packet, select
   one bounded output cluster; reject stale evidence; do not create a packet
   for an unregistered style.
3. Run the correct oracle with all failures visible.
4. Classify each failure using the shared decision rules.
5. Apply the smallest YAML fix needed for the selected defect.
6. Re-run the oracle.
7. If configured, capture supplemental rich-input evidence after the main oracle pass.
8. Before final QA, regenerate and validate any registered packet and report
   disposition plus joined-parity deltas.
9. Stop iterating on scenarios that have converged or belong to another layer.
10. QA gate, then commit.

## Fix Ordering
1. Component-level type variations and punctuation/wrap controls.
2. Shared bibliography spine improvements.
3. `type-variants` only for true structural outliers.
4. Processor/schema changes only after planner escalation.

## Co-Evolution

If the defect is actually a processor or engine issue, use the shared decision rules and then route it to the appropriate Rust workflow.

## Verification
- Oracle per routing table in the shared docs and this skill.
- `cargo run --bin citum -- render refs -b tests/fixtures/references-expanded.json -s <style-path>`
- QA handoff to `../style-qa/SKILL.md`
