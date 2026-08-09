---
name: style-migrate-enhance
type: agent-invocable
description: High-throughput migration waves converting priority parent CSL 1.0 styles to Citum with repeatable before/after metrics and migration-engine gap recommendations. Fidelity is the hard gate; exact parity is diagnostic for dependent styles.
model: sonnet
---

# Style Migrate+Enhance

Authoritative shared process docs:
- `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md`
- `docs/guides/STYLE_WORKFLOW_EXECUTION.md`
- `docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md` — converter role and limits
- `docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md` — why fidelity
  alone is not sufficient evidence of correct rendering, and why exact parity
  is a hard gate for embedded-core but stays diagnostic here

## Use This Skill When
- The task is portfolio migration (long-tail / dependent styles).
- You need repeatable before/after/rerun metrics.
- You want concrete recommendations for `citum_migrate` improvements from observed gaps.
- You need to produce a migrate **seed** for a subsequent `tune` pass on an
  embedded-core style.

## Role of Migrate Output

For **dependent** styles, the migrate output is the deliverable (subject to the
fidelity hard gate; exact parity is captured but stays diagnostic-only for this
tier). For **embedded-core** styles, the migrate output is a **seed
and evidence source** feeding a subsequent `style-tune` pass — it is not a
finished embedded style, and its exact-parity number is a starting point, not
a gate this skill enforces (`style-tune` owns that gate). See
`docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md` for the strategic basis:
the converter is an evidence tool, not the canonical authoring path for
high-impact styles.

## Input Contract
- Legacy style path(s) under `styles-legacy/`.
- Target Citum style path(s) under `styles/`.
- Batch size and priority source.
- Optional target metric.

## Output Contract
- Updated style YAML file(s).
- Shared metrics and rerun evidence in the format described by the shared execution guide.
- Migration-pattern gaps and recommended converter/preset follow-up when observed.
  Every CSL 1.0.2 type reaches styles as its real `ref_type` under the
  conversion contract (`docs/specs/CSL_TYPE_CONVERSION_CONTRACT.md`), so when
  the source CSL style handles a type like `collection`, `review`,
  `performance`, or `figure`, a missing `type-variants` entry in the migrated
  YAML is a reportable migration gap, not converter noise.

## Autonomous Operation

Run the full wave without pausing between styles. Use the shared docs for the common evidence order, decision rules, and output shape. Only interrupt for `Cargo.toml`/`Cargo.lock` changes or `git push origin main`.

## Workflow
1. Select the next priority wave.
2. Report each target's coverage-audit status as `current`, `stale`, or
   `not registered` with
   `node scripts/check-style-coverage-audits.js --status <style-id>`. Use a
   current packet to select one bounded cluster, reject a stale packet, and do
   not create one for an unregistered style.
3. Seed the baseline with the smallest trustworthy evidence surface.
4. Apply the fix according to the shared policy and execution guide.
5. Re-run apples-to-apples comparison evidence.
6. Treat supplemental rich-input evidence as confirmation when configured.
7. Commit each passing style and produce final metrics plus follow-up recommendations.

## Hard Gates
- Never accept a fidelity regression.
- Never classify a registered divergence as a migration or engine bug without updating adjudication first.
- Never call a cluster a `citum-migrate` defect from a hand-authored coverage
  packet. Require a fresh migrated candidate from the relevant CSL source and
  reproduce the cluster against that candidate.
- Exact parity is diagnostic-only in this skill for dependent styles, and not
  a gate here at all for embedded-core targets — the `tune` pass owns the
  exact-parity gate. Still capture the number as seed evidence when the
  target is embedded-core.
- SQI is tie-breaker and optimization only for dependent styles. For embedded-core
  targets, SQI is not a gate here — the `tune` pass owns SQI to green.
- After bounded retries with no progress, note it in the wave summary and move to the next style rather than halting the entire wave.

## Required Artifacts
- Iteration log.
- Final wave summary table.
- Code Opportunities table in the same shape as the router skill when engine gaps are observed.

## Verification
- Structured oracle: `node scripts/oracle.js <legacy-style> --json`
- Core quality report: `node scripts/report-core.js`
- Supplemental official style report for configured rich-input styles: `node scripts/report-core.js --style <name>`
- Optional full workflow impact: `./scripts/workflow-test.sh <legacy-style>`
