# Style Migrate+Enhance

**Type:** User-Invocable, Agent-Invocable
**LLM Access:** Yes
**Purpose:** High-throughput migration waves for priority parent styles with fidelity as the hard gate and SQI as secondary optimization.

## Use This Skill When
- The task is portfolio migration (for example: next 5 or 10 styles).
- You need repeatable before/after/rerun metrics.
- You want concrete recommendations for `csln_migrate` improvements from observed gaps.

## Input Contract
- Legacy style path(s) under `styles-legacy/`.
- Target CSLN style path(s) under `styles/`.
- Batch size and priority source (`docs/reference/STYLE_PRIORITY.md`, `docs/TIER_STATUS.md`).
- Optional target metric (for example: bibliography >= 24/28).

## Output Contract
- Updated style YAML file(s).
- Metrics table per style:
  - baseline (seeded)
  - enhanced (edited)
  - rerun (fresh `csln-migrate` for comparison)
- List of migration-pattern gaps and recommended converter/preset follow-up.

## Workflow
1. Select next priority wave.
2. Seed with migration baseline (`scripts/prep-migration.sh` or `csln-migrate`).
3. Capture baseline metrics (`node scripts/report-core.js`, `node scripts/oracle.js ... --json`).
4. Run iterative style fixes with fidelity-first ordering.
5. Re-run migration for apples-to-apples comparison.
6. Produce metrics + follow-up recommendations.

## Hard Gates
- Never accept a fidelity regression.
- SQI is tie-breaker and optimization only.
- Stop and escalate if iteration 1 bibliography is below 50%.
- Stop after bounded retries and return a redesign request to planner.

## Required Artifacts
- Iteration log (what changed, what improved, what remains).
- Final wave summary table.
- Suggested migration-engine improvements only when repeated across styles.

## Verification
- Structured oracle: `node scripts/oracle.js <legacy-style> --json`
- Core quality report: `node scripts/report-core.js`
- Optional full workflow impact: `./scripts/workflow-test.sh <legacy-style>`

## Related
- Public router: `../style-evolve/SKILL.md`
- QA gate: `../style-qa/SKILL.md`
- Planning templates: `../styleauthor/templates/migrate-enhance-checklist.md`
