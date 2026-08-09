---
name: migrate-research
description: Autonomous research loop for Citum migration fidelity gaps and converter-focused Rust fixes.
---

# Migrate Research

Use this skill when multiple styles share a migration failure pattern or when a rich
input pass needs classification before changing `citum_migrate`.

Read first:
- `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md`
- `docs/guides/STYLE_WORKFLOW_EXECUTION.md`
- `docs/specs/MIGRATE_RESEARCH_RICH_INPUTS.md`
- `docs/guides/AGENT_SKILLS.md`
- `docs/guides/JJ_AI_CHANGE_STACK.md` when `.jj` is present and the research
  pass needs an isolated local change

## Use When

- The goal is converter improvement, not a style YAML patch.
- Several styles fail for the same reason.
- You need bounded evidence before deciding whether to fix migration, style, processor,
  or adjudication.

## Operating Rules

- Start with the smallest trustworthy evidence surface.
- Report the target's coverage-audit status as `current`, `stale`, or
  `not registered` using `node scripts/check-style-coverage-audits.js --status
  <style-id>`. Use a current packet to choose one bounded cluster, reject a
  stale packet, and do not create one for an unregistered style.
- State the target semantic class and implementation form before proposing a fix.
- Classify each cluster as `migration-artifact`, `style-defect`, `processor-defect`,
  or `intentional divergence`. For type- or field-population-shaped clusters,
  run the conversion-layer pre-flight from the shared decision rules
  (`docs/policies/STYLE_WORKFLOW_DECISION_RULES.md`) before classifying.
- Record the selected parent style when the target is a known wrapper.
- Preserve the config-wrapper contract for `profile + config-wrapper` targets.
- Treat `journal + structural-wrapper` as a valid endpoint; do not force thin-wrapper reduction.
- If a migration-side fix would require local templates or local `type-variants` in a profile,
  stop and reroute or escalate instead of breaking the profile contract.
- Stop when the cluster is clearly out of scope or converged.
- Keep the loop bounded to one cluster per pass.
- Never attribute a cluster to `citum-migrate` from a hand-authored coverage
  packet. Require a fresh migrated candidate from the relevant CSL source and
  reproduce the cluster before calling it a converter defect.

## Output

Report the chosen cluster, coverage-audit status, semantic class,
implementation form, selected parent if any, classification, before/after
evidence, disposition and parity deltas when registered, exact change made if
any, and whether the pass should continue, stop, or escalate.

## Self-Improvement

When you reach a dead end not covered by the operating rules — a cluster pattern that
doesn't fit the four classifications, a stop condition missing from the guides, or a
migration artifact the evidence ladder doesn't anticipate — record the insight as a
concrete bullet here and include the file update in the same commit or PR. Future
research passes start with a richer surface.

- (2026-06-10) Corpus-level cluster selection: before opening a pass, run
  `node scripts/report-migrate-sqi.js --corpus random --seed 20260610` and pick
  the cluster by reach from its fidelity headline + failure taxonomy, not from
  the curated lab corpus alone. The random baseline and current cluster table
  live in `docs/architecture/audits/2026-06-10_MIGRATE_RANDOM_SAMPLE_BASELINE.md`.
- (2026-06-10) A processor hard-fail on migrated output (e.g. a `type-variants`
  op matching no component) is always `migration-artifact` with an emit-time
  validation fix in `citum-migrate` plus a regression test — the converter must
  never emit YAML the processor rejects. No evidence ladder needed.
- (2026-06-10) Note-class run-on output (delimiters/affixes lost wholesale in
  full-note citation templates) is one systemic cluster, not per-style defects;
  do not open per-style passes for it.
