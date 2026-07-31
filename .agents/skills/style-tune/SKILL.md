---
name: style-tune
type: agent-invocable
description: Iterative LLM hand-tuning loop for embedded-core styles. Drives a style to 100% fidelity, a raised exact-parity floor, and clean SQI. All three are hard gates. Seeded from migrate evidence, not from the converter output as a terminal deliverable.
model: sonnet
---

# Style Tune

Authoritative shared process docs:
- `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md` — tier definition, quality bar,
  failure classification
- `docs/guides/STYLE_WORKFLOW_EXECUTION.md` — the `tune` loop definition (seed →
  fidelity loop → exact-parity loop → SQI loop → QA), stop conditions, and shared
  escalation rules
- `docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md` — why fidelity
  alone (a lenient, punctuation/case-insensitive comparison) is not sufficient
  evidence of correct rendering, and how the exact-parity gate and adjudication
  ledger work

## Use This Skill When
- The target is an `embedded-core` style (verified via `citum style list --source embedded`
  or by checking `crates/citum-schema-style/src/embedded/styles.rs`).
- The goal is 100% oracle fidelity, a raised exact-parity floor, **and** clean SQI.
- Migrate output is available (or can be generated) as the starting seed.

## What This Skill Is NOT
- Not for long-tail or dependent styles (use `style-maintain` or `style-migrate-enhance`).
- Not a batch wave tool — one embedded style per run.
- Not a converter fix tool — `citum-migrate` issues are escalated separately.

## Input Contract
- Embedded style ID (e.g. `apa-7th`, `ieee`).
- Legacy CSL path in `styles-legacy/` for oracle comparison.
- Citum YAML path in `crates/citum-schema-style/embedded/styles/`.
- Authority basis: publisher guide or style manual (primary authority first).

## Hard Gates
- Fidelity: 100% oracle pass rate (`node scripts/oracle.js <legacy> --json`).
- Exact parity: the style's `exactParity.passed` count in
  `node scripts/report-core.js --style <name> --all-features` may never drop
  below its recorded floor in `scripts/report-data/embedded-parity-baseline.json`.
  This is the primary tuning objective — where the real punctuation, casing,
  and spacing work happens.
- SQI: clean score (`node scripts/report-core.js --style <name>`).
- A `tune` pass is not complete until fidelity is 100% and SQI is clean; a
  pass may still land with residual exact-parity gap if every residual is
  classified (fixed, escalated as `unclear`, or excluded via a registered
  divergence) — see Failure Classification below.
- Never accept a fidelity regression as a tradeoff for exact-parity or SQI
  improvement, and never accept an exact-parity regression as a tradeoff for
  SQI improvement.

## Execution Loop

Follow the full `tune` loop from `docs/guides/STYLE_WORKFLOW_EXECUTION.md`:

1. **Seed** — run `citum-migrate` or accept the existing YAML. Record baseline
   oracle fidelity and exact-parity floor.
2. **Fidelity loop** — oracle → classify failure → smallest correct YAML fix →
   re-run. Repeat until 100%.
3. **Exact-parity loop** (begins once fidelity is green) — `report-core --style
   --all-features` → classify each residual → smallest correct fix or ledger
   entry → re-run. Continue until no further residual is classifiable without
   escalation; regenerate `embedded-parity-baseline.json` to ratchet the new
   floor in.
4. **SQI loop** (only after fidelity and exact parity are stable) —
   `report-core` → hoist/preset/prune → oracle re-check → repeat until clean.
5. **QA gate** — hand off to `../style-qa/SKILL.md` with `tier: embedded-core`.

## Failure Classification
Use the shared decision rules for all mismatches. For type- or
field-population-shaped mismatches, run the conversion-layer pre-flight
(Decision Rules → "Conversion-layer pre-flight") before classifying —
never iterate YAML against a reference that converted wrongly.
- `style-defect` → fix in YAML.
- `migration-artifact` → note gap, do not cycle YAML to compensate; fix the seed
  if a converter improvement is available, otherwise hand-author around it.
- `processor-defect` → escalate to Rust workflow; stop YAML iteration on that cluster.
- `intentional divergence` that generalizes beyond this style → record in
  `docs/adjudication/DIVERGENCE_REGISTER.md`, exclude from counts.
- Exact-parity residual that fits none of the above → record
  `citeproc-correct` (still a required fix) or `unclear` (excludes, escalates
  to the user) in `scripts/report-data/parity-adjudication.json`. Never write
  `citum-correct` — that state requires the user and a cited authority.

## Stop Conditions
- Two distinct approaches fail on the same cluster → reclassify.
- Residual explained by a registered divergence → record, do not count.
- Residual is `processor-defect` → escalate; move on.
- Migrate cannot produce a usable seed → switch to pure `create` path (hand-author
  from guide evidence directly).

## Output Contract
Every completed tune pass delivers:
- embedded style ID and authority basis
- tier: `embedded-core`
- seed baseline: oracle fidelity %, exact-parity passed/total, SQI score
- final: oracle fidelity %, exact-parity passed/total, SQI score
- fidelity changes made (per mismatch cluster)
- exact-parity changes made (per residual class), and any new
  `parity-adjudication.json` entries with their state
- SQI changes made (hoisting, presets, type-variant compression)
- residuals reclassified (processor-defect / divergence IDs / adjudication states)
- QA verdict
- commit SHA and message

## Verification
- Oracle: `node scripts/oracle.js styles-legacy/<name>.csl --json`
- Exact parity: `node scripts/report-core.js --style <name> --all-features`
  (read `styles[0].exactParity`)
- SQI: `node scripts/report-core.js --style <name>`
- Render smoke-check: `cargo run --bin citum -- render refs -b tests/fixtures/references-expanded.json -s crates/citum-schema-style/embedded/styles/<name>.yaml`
- QA handoff: `../style-qa/SKILL.md` with `tier: embedded-core`
