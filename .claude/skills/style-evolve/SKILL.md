---
name: style-evolve
type: user-invocable, agent-invocable
description: "Single human-facing command for all Citum style work. Use whenever someone asks to fix, improve, convert, create, or tune a citation style. Routes to upgrade, migrate, create, or tune. Always use this rather than calling internal skills directly."
model: sonnet
routes-to: style-maintain, style-migrate-enhance, style-tune, style-qa
---

# Style Evolve

## Human UX (Public Entry Point)

```
/style-evolve upgrade <style-path>
/style-evolve migrate <csl-path>
/style-evolve create
/style-evolve tune <embedded-style-id>
```

Do not ask users to call internal skills directly.

## Setup (read first)

Before doing anything else, read:
- `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md` — failure classification and stop conditions
- `docs/guides/STYLE_WORKFLOW_EXECUTION.md` — decision flow, evidence ladder, shared gates
- `docs/guides/JJ_AI_CHANGE_STACK.md` when `.jj` is present and local change-stack
  curation would help isolate a style pass

## Autonomous Operation

Run the full pipeline without stopping to ask questions between steps. Only interrupt for the explicit permission gates in `CLAUDE.md`.

## Modes

### 1. upgrade
Route to `../style-maintain/SKILL.md`.

### 2. migrate
Route to `../style-migrate-enhance/SKILL.md`.

### 3. create
Build a new Citum style from source evidence. Escalate to `@dplanner` for design.

### 4. tune
Route to `../style-tune/SKILL.md`. Use when the target is an `embedded-core`
style and the goal is 100% fidelity, a raised exact-parity floor, **and**
clean SQI. All three are hard gates for this mode. The migrate output is the
seed, not the deliverable.

## Co-Evolution Rule

Use `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md` for failure classification and stop conditions. Keep the router focused on dispatch and host entrypoint behavior.

## Shared Gates

- Compatibility fidelity regression is never allowed unless the task explicitly chooses a documented semantic divergence from legacy CSL behavior.
  Fidelity is a lenient structural check, not proof of correct rendering — see
  `docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md`.
- **Exact parity is a hard gate for `embedded-core` styles**: the `passed`
  count may never drop below the floor recorded in
  `scripts/report-data/embedded-parity-baseline.json`. It is the primary
  tuning objective for `tune` mode. For `dependent` styles, exact parity is
  diagnostic only.
- **SQI is a hard gate for `embedded-core` styles** (fidelity, exact parity,
  AND SQI all required, in that order).
  For `dependent` styles, SQI is optimization-only after fidelity is stable.
  See `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md` for the tier definition and quality bar.
- An exact-parity residual that cannot be classified as `style-defect`,
  `processor-defect`, or a generalizable registered divergence is recorded in
  `scripts/report-data/parity-adjudication.json`. Agents may record
  `citeproc-correct` or `unclear` only; `citum-correct` requires the user.
- Before editing a style, classify it by semantic class, implementation form, and
  portfolio tier using the shared workflow docs and `docs/specs/STYLE_TAXONOMY.md`.
- Before editing, run
  `node scripts/check-style-coverage-audits.js --status <style-id>` and report
  `current`, `stale`, or `not registered`. Follow the shared workflow for
  bounded cluster selection, `--update-manifest` recovery from a stale
  packet, the shared-ancestor / full-portfolio-baseline rule, and final
  packet regeneration.
- Profile-family work may require a `create` pass for a hidden family root
  followed by `upgrade` reduction of the public handles.
- Journal/profile reductions must choose parents from guide-backed authority,
  not nearest CSL or template similarity.
- Keep waves bounded to one family or one clearly related cohort per PR.
- For styles with configured `benchmark_runs`, run `node scripts/report-core.js --style <name>` after the primary oracle pass and treat the rich-input results as official supplemental evidence.
- All modes must pass `../style-qa/SKILL.md` before completion.
- If docs or beans are changed: `./scripts/check-docs-beans-hygiene.sh` must pass.

## Output Contract

Every completed task delivers:
- fidelity metrics
- exact-parity metrics (embedded-core styles)
- SQI delta
- authority basis
- rich input evidence summary
- coverage-audit status and, when registered, disposition plus joined-parity deltas
- divergences
- code opportunities table
- QA verdict
- research value
- commit SHA and message, or `none`

Include a routed sub-skill header line in the final output.
