# Style Workflow Execution

**Status:** Active
**Version:** 1.2
**Date:** 2026-08-11
**Related:** [STYLE_WORKFLOW_DECISION_RULES.md](../policies/STYLE_WORKFLOW_DECISION_RULES.md),
[MIGRATE_RESEARCH_RICH_INPUTS.md](../specs/MIGRATE_RESEARCH_RICH_INPUTS.md),
[STYLE_TEMPLATE_EXPRESSIVENESS_AND_PARITY.md](../specs/STYLE_TEMPLATE_EXPRESSIVENESS_AND_PARITY.md),
[MIGRATION_STRATEGY_ANALYSIS.md](../architecture/MIGRATION_STRATEGY_ANALYSIS.md)

## Purpose
This guide defines the shared execution flow for Citum style workflows so Claude skills and Codex agents can reference the same process without duplicating it.

## Scope
In scope:
- style-oriented routing and verification loops
- evidence order for migration and QA passes
- output contract shape for shared workflow roles
- common escalation boundaries and stop conditions

Out of scope:
- host-specific frontmatter and model settings
- one-off mode deltas that only apply to a single wrapper
- Rust implementation details

## Design
### Shared execution order
1. Establish the workflow mode and target scope.
2. Classify the target on **three** axes before editing:
   - semantic class (`base`, `profile`, `journal`, `independent`)
   - implementation form (`alias`, `config-wrapper`, `structural-wrapper`, `standalone`)
   - portfolio tier (`embedded-core` or `dependent`) — see the decision rules for
     the predicate (`citum style list --source embedded`)
3. Establish source authority before reading implementation artifacts:
   publisher guide first, then publisher house rules, then parent-style guidance.
4. Run `node scripts/check-style-coverage-audits.js --status <style-id>` and
   report `current`, `stale`, or `not registered`.
   - For `current`, inspect the human adjudication record and packet before
     editing, then select one bounded output cluster.
   - For `stale`, stop and regenerate valid evidence. If the cause is an
     edit to the audited style or one of its `style.chain` ancestors (the
     expected case — the manifest pins file hashes that do not self-heal),
     regenerate with `node scripts/style-coverage-review.js --manifest
     <path> --json-out <path> --markdown-out <path> --update-manifest`
     first to re-pin the hashes and observation count, then regenerate the
     packet normally on a clean, committed tree before it can be
     baseline-eligible again.
   - For `not registered`, continue without creating an audit.
   - If the edit touches a `style.chain` file shared by other embedded-core
     styles (check `citum style list --source embedded` plus each
     candidate's `extends`), a registered packet only speaks for its own
     leaf style — the shared-ancestor rule below is the mechanism that
     protects and credits the rest of that family.
5. Capture the smallest trustworthy evidence surface first. A registered
   audit's uncovered fields are structural leads, not causal claims.
6. Use reduced-cluster evidence before broad supplemental reruns.
7. Classify each failure using the shared policy.
8. Apply at most one tightly scoped fix per bounded cluster pass.
9. Re-run the reduced evidence set, then the broader oracle or report surface.
10. Before final QA on a registered style, regenerate the baseline-eligible
    packet (`--update-manifest` first if a chain file changed, then a plain
    regeneration on a clean, committed tree), rerun the checker, and explain
    disposition and parity count deltas. If a shared ancestor changed, also
    regenerate `embedded-parity-baseline.json` from the full portfolio and
    report any sibling deltas.
11. Stop when the cluster is reclassified, converged, or proven out of scope.

### Shared verification logic
- Fidelity to the declared primary authority is a hard gate for all tiers.
  Fidelity is lenient by design (see
  [2026-07-31_EXACT_PARITY_REFOCUS.md](../architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md)) —
  it catches structural breakage, not formatting quality.
- **Exact parity is a hard gate for `embedded-core` styles**, and is the
  primary tuning objective once fidelity is green — see the `tune` loop
  below. A style's exact-parity `passed` count may never drop below its
  recorded floor in `scripts/report-data/embedded-parity-baseline.json`. For
  `dependent` styles, exact parity is diagnostic only.
- **SQI is a hard gate for `embedded-core` styles**, ordered after exact
  parity (both fidelity and exact parity must be green before SQI work
  starts). For `dependent` styles, SQI is advisory and a tie-breaker only.
- QA must reject regressions and formatting defects.
- Supplemental rich-input evidence is confirmation, not the first debugging surface.
- Registered audit packets must be schema-valid, hash-current,
  baseline-eligible, count-consistent, and byte-reproducible. Count changes are
  evidence requiring explanation, not an automatic pass or failure.
- **Shared-ancestor edits require a full-portfolio floor check.** A registered
  coverage packet is scoped to one leaf style; it gives no field-level
  evidence for siblings that extend the same `style.chain` ancestor (Chicago's
  shared bases are the current example). The pre-existing, portfolio-wide
  safety net is `scripts/report-data/embedded-parity-baseline.json`, which
  tracks a per-style exact-parity floor for every `embedded-core` style
  individually. Before closing any pass that edited a file shared by other
  embedded-core styles, run the full-portfolio `report-core.js` (not
  `--style`-scoped) and regenerate that baseline file — this is what catches
  a sibling regression and credits a sibling improvement that the packet
  itself cannot see.
- CSL structure is verification evidence, not the source of truth for wrapper thickness.
- For `profile` targets, verify that the file still satisfies the config-wrapper
  contract: no local templates, no local `type-variants`, and no
  template-clearing `null`.
- For `journal` targets, accept `structural-wrapper` as a legitimate endpoint
  when guide-backed deltas or current merge mechanics prevent a meaningful thin
  reduction.

### Shared output shape
Every workflow should report:
- target or cluster chosen
- semantic class, implementation form, and portfolio tier
- classification and rationale
- before/after evidence
- registered coverage-audit status (`current`, `stale`, or `not registered`)
- disposition and joined exact-parity deltas when a registered packet applies
- exact change made, if any
- whether the pass should continue, stop, or escalate

### Shared escalation
- `migration-artifact` stays in migration work until the converter is fixed or disproven.
- Do not classify a cluster as `migration-artifact` from a hand-authored
  coverage packet. Require a fresh candidate generated by `citum-migrate` from
  the relevant CSL source and reproduce the cluster against that candidate.
- `style-defect` routes to style-local YAML repair.
- `processor-defect` routes to processor or engine follow-up.
- `intentional divergence` that generalizes beyond one style is recorded in
  [DIVERGENCE_REGISTER.md](../adjudication/DIVERGENCE_REGISTER.md) and excluded from fix counts.
- An exact-parity residual an agent cannot classify as `style-defect` or
  `processor-defect` is recorded in `scripts/report-data/parity-adjudication.json`
  as `citeproc-correct` (fix required, counts against the gate) or `unclear`
  (excluded, escalates to the user). An agent must never record `citum-correct`
  — that state requires the user and a cited authority. See
  [2026-07-31_EXACT_PARITY_REFOCUS.md](../architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md)
  for the full rationale.
- If parentage is guide-backed but current merge semantics still force a bulky
  wrapper, escalate as an infrastructure constraint rather than preserving or
  reintroducing duplicated structure as if it were authority.

## Waves

A style wave is a bounded cohort executed through repeated `upgrade`, `migrate`,
`create`, or `tune` passes under this same execution flow.

- Keep one wave to one family or one clearly related cohort per PR.
- For profile-family work, it is valid to use `create` to author a hidden family
  root first and then `upgrade` to reduce the public handles.
- Do not add a separate public "wave" command surface; waves are an execution
  pattern, not a new mode.

## The `tune` loop (embedded-core styles)

`tune` is the correct mode whenever the goal is to bring an `embedded-core`
style to **100% fidelity, its exact-parity floor raised, and clean SQI**.
Deterministic migration (`citum-migrate`) cannot reliably reach this bar on
its own — see
[MIGRATION_STRATEGY_ANALYSIS.md](../architecture/MIGRATION_STRATEGY_ANALYSIS.md). The converter's output is a
**seed**: a starting candidate whose oracle score and SQI baseline ground the
first iteration.

Exact parity is the primary tuning objective — it is where the actual
punctuation, casing, and spacing work happens. Fidelity gates entry into the
loop; SQI is a final structural pass once the visible text is correct. See
[2026-07-31_EXACT_PARITY_REFOCUS.md](../architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md)
for why fidelity alone is not sufficient evidence of correct rendering.

### Execution order
1. **Seed:** run `citum-migrate` (or accept the existing Citum YAML) to produce
   a concrete candidate. Record oracle fidelity baseline and exact-parity
   baseline (`scripts/report-data/embedded-parity-baseline.json`). Report the
   registered coverage-audit status and, when current, choose one bounded
   packet cluster before editing. Editing the audited style or any of its
   `style.chain` ancestors stales the packet's pinned hashes immediately —
   regenerating with `--update-manifest` (see the shared execution order,
   step 4) is the expected recovery, not a sign something went wrong.
2. **Fidelity loop:**
   a. Run the oracle (`node scripts/oracle.js <legacy-style> --json`).
   b. Classify each failure per the shared decision rules; for type- or
      field-population-shaped failures, run the conversion-layer pre-flight
      (Decision Rules → "Conversion-layer pre-flight") first.
   c. Apply the smallest correct YAML change toward the target reference output.
   d. Re-run oracle. Repeat until fidelity is 100%.
   e. If a residual is clearly a `processor-defect` or `intentional divergence`,
      reclassify and exclude — do not keep iterating.
3. **Exact-parity loop (begins once fidelity is green):**
   a. Run `node scripts/report-core.js --style <name> --all-features` and read
      `exactParity` for the style.
   b. For each residual: classify as `style-defect` (fix the YAML),
      `processor-defect` (escalate to Rust workflow), a generalizable
      `intentional divergence` (add to
      [DIVERGENCE_REGISTER.md](../adjudication/DIVERGENCE_REGISTER.md)), or —
      when none of those fit — record `citeproc-correct` (still a required
      fix) or `unclear` (excludes and escalates to the user) in
      `scripts/report-data/parity-adjudication.json`. Never record
      `citum-correct`; that state is the user's call, made with a cited
      authority.
   c. Apply the smallest correct fix. Re-run and confirm the `passed` count
      rose and fidelity did not regress.
   d. Continue until no further residual is classifiable without escalation,
      or the floor is raised as far as this pass supports; regenerate
      `embedded-parity-baseline.json` to ratchet the new floor in. Always
      regenerate it from the full portfolio, not `--style`-scoped — this is
      the only mechanism that protects (and credits) sibling styles when a
      fix landed in a shared `style.chain` ancestor.
4. **SQI loop (begins only when fidelity and exact parity are stable):**
   a. Run `node scripts/report-core.js --style <name>` to get the SQI score.
   b. Apply SQI improvements — hoist shared options, use presets, introduce
      diff-based `type-variants`, prune redundant defaults — without regressing
      fidelity or exact parity. Re-run oracle after each SQI change to confirm.
   c. Continue until SQI is clean (no actionable SQI findings remain).
5. **QA gate:** hand off to `style-qa` with tier = `embedded-core`.
   Regenerate and validate any registered coverage packet first, then include
   disposition and joined exact-parity deltas in the handoff.

### Stop conditions (same as all shared workflows)
- Two distinct approaches fail on the same cluster → reclassify.
- Residual explained by a registered divergence → record ID, do not count as failure.
- Residual is a `processor-defect` → escalate to Rust workflow; do not keep
  cycling YAML.
- Migrate cannot produce a usable seed → hand-author from guide evidence
  directly (the `create` path).

## Implementation Notes
- Use this guide as the canonical place for the evidence ladder and convergence language currently repeated across style workflows.
- Keep host wrappers short and refer back here instead of restating the loop.

## Acceptance Criteria
- [x] Shared style workflows reference this guide instead of duplicating the same loop text.
- [x] The evidence ladder is defined here exactly once.
- [x] The shared output contract is expressed here in host-neutral terms.
- [x] Portfolio tier is part of the classification and output contract.
- [x] `tune` loop is defined here once, not in individual skill files.

## Changelog
- 2026-08-11: Documented `style-coverage-review.js --update-manifest` as the
  recovery path when a coverage-audit pre-flight goes stale from an edit to
  the audited style or a shared `style.chain` ancestor. Added the
  shared-ancestor / full-portfolio-baseline rule: a registered packet is
  leaf-scoped, so sibling embedded-core styles sharing an edited ancestor are
  protected and credited only by regenerating `embedded-parity-baseline.json`
  from the full portfolio, not a `--style`-scoped run.
- 2026-08-09: Added coverage-audit status reporting, current-packet cluster
  selection, stale-packet rejection, final regeneration and delta reporting,
  the unaudited-style opt-out, and fresh migrated-candidate evidence for
  converter attribution.
- 2026-07-31: Promoted exact parity to a hard gate for `embedded-core` styles,
  ordered between fidelity and SQI in the tier table and the `tune` loop
  (fidelity → exact parity → SQI). Added the parity-adjudication escalation
  path (`citeproc-correct` / `unclear` / user-only `citum-correct`) to shared
  escalation. Cross-linked
  [2026-07-31_EXACT_PARITY_REFOCUS.md](../architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md).
  Fidelity's gate mechanics and scope are unchanged.
- 2026-06-24: Added portfolio tier to the three-axis classification and output
  shape. Replaced the universal "SQI is advisory" rule with a tier-aware rule
  (embedded-core promotes SQI to a hard gate; dependent stays advisory). Added
  the `tune` execution loop section, including seed, fidelity loop, SQI loop,
  and stop conditions. Cross-linked `MIGRATION_STRATEGY_ANALYSIS.md`. Added
  `tune` to the wave pass types.
- 2026-04-23: Added explicit semantic-class vs implementation-form
  classification, profile-contract verification, journal structural-wrapper
  acceptance, and bounded-wave guidance.
- 2026-04-04: Initial version.
