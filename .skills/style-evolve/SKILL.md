---
name: style-evolve
description: "Public entrypoint for Citum style work. Activate on: 'upgrade', 'migrate', 'create', 'tune', any style authoring request, or any request to fix/improve/convert a Citum or CSL citation style. Route to the shared workflow docs and internal roles."
---

# Style Evolve

Use this skill for any Citum style request that should be handled through the shared
style workflow.

Read first:
- `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md`
- `docs/guides/STYLE_WORKFLOW_EXECUTION.md`
- `docs/guides/AGENT_SKILLS.md`
- `docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md` — why fidelity
  alone is not sufficient evidence of correct rendering
- `docs/guides/JJ_AI_CHANGE_STACK.md` when `.jj` is present and local stack
  curation would help isolate the work

## Public Modes

- `upgrade`: improve an existing Citum style.
- `migrate`: convert CSL 1.0 source into Citum style YAML.
- `create`: author a new Citum style from source evidence.
- `tune`: drive an embedded-core style to 100% fidelity, a raised exact-parity
  floor, + clean SQI via iterative LLM authoring, seeded by migrate evidence.
  Fidelity, exact parity, and SQI are all hard gates for this mode, applied in
  that order. Loop defined in `docs/guides/STYLE_WORKFLOW_EXECUTION.md`.

## Routing

- Use `spec-planner` when the request needs architecture or schema decisions.
- Use `migration-researcher` when the evidence points to `citum_migrate`.
- Use `rust-implementer` for bounded Rust fixes.
- Use `style-qa-reviewer` for the final style QA gate.

## Operating Rules

- Fidelity is the hard gate for all tiers. It is a lenient, punctuation/case-insensitive
  comparison — necessary but not sufficient evidence of correct rendering.
- Exact parity is a **hard gate for embedded-core styles**: `passed` may never drop
  below the floor in `scripts/report-data/embedded-parity-baseline.json`, and it is
  the primary tuning objective for `tune`. Diagnostic only for dependent styles.
- SQI is a **hard gate for embedded-core styles**, ordered after fidelity and exact
  parity; advisory/tie-breaker for dependent styles. See
  `docs/policies/STYLE_WORKFLOW_DECISION_RULES.md` for the tier definition and quality bar.
- For embedded-core targets, use `tune` rather than treating migrate output as final.
- An exact-parity residual that isn't a `style-defect`/`processor-defect`/registered
  divergence is recorded in `scripts/report-data/parity-adjudication.json` as
  `citeproc-correct` or `unclear`; only the user may record `citum-correct`.
- Before editing a style, classify it by semantic class and implementation
  form using `docs/specs/STYLE_TAXONOMY.md` and the shared workflow docs.
- Profile-family work may require a `create` pass for a hidden family root
  followed by `upgrade` reduction of the public handles.
- Journal/profile reductions must choose parents from guide-backed authority,
  not nearest CSL or template similarity.
- Keep waves bounded to one family or one clearly related cohort per PR.
- Do not duplicate the shared decision rules or evidence ladder here.
- Keep this skill focused on routing and host-facing behavior.

## Self-Improvement

When you encounter a routing case, semantic class, or implementation form not covered
by the existing operating rules or the shared workflow docs, record the pattern here as
a new bullet under "Operating Rules" and include the file update in the same commit.
The goal is that each time style-evolve handles an edge case it couldn't resolve
cleanly, that edge case becomes a covered case going forward.
