# Style Maintain

**Type:** User-Invocable, Agent-Invocable
**LLM Access:** Yes
**Purpose:** Fast, targeted maintenance for an existing CSLN style file.

## Use This Skill When
- Updating one style for punctuation/layout bugs.
- Adding a missing type override.
- Modernizing style syntax without changing rendered output intent.

## Input Contract
- Existing style path in `styles/`.
- One focused objective (formatting bug, missing type, or modernization).
- Optional reference oracle style in `styles-legacy/`.

## Workflow
1. Reproduce mismatch with one oracle snapshot.
2. Apply smallest YAML-first fix.
3. Recheck oracle metrics.
4. Run QA gate.
5. Stop when target is reached.

## Fix Ordering
1. Component overrides and punctuation/wrap controls.
2. Shared bibliography spine improvements.
3. `type-templates` only for true structural outliers.
4. Processor/schema changes only after planner escalation.

## Hard Gates
- Preserve or improve fidelity.
- No unnecessary template explosion.
- Keep fallback behavior for non-explicit types reasonable.

## Verification
- `node scripts/oracle.js <legacy-style> --json`
- `cargo run --bin csln -- render refs -b tests/fixtures/references-expanded.json -s <style-path>`
- QA handoff to `../style-qa/SKILL.md`

## Related
- Public router: `../style-evolve/SKILL.md`
- QA gate: `../style-qa/SKILL.md`
- Update checklist: `../styleauthor/templates/update-checklist.md`
