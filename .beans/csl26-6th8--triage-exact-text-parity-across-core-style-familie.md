---
# csl26-6th8
title: Reassess fidelity claims and triage exact-text parity
status: in-progress
type: task
priority: high
tags:
    - scorecard
    - styles
    - fidelity
created_at: 2026-07-28T12:17:54Z
updated_at: 2026-07-31T14:41:05Z
parent: csl26-zik7
---

The corrected unadjudicated oracle-text snapshot introduced by csl26-zik7
reports 3,198/12,712 (25.2%) across 157 core styles while the existing
baseline gate remains green for every style. There are 8,747 paired rows that pass
lenient compatibility but differ from the oracle text. These are repeated
row-level observations, not a unique-defect count or a verdict that Citum is
wrong. Initial clusters include punctuation/brackets (at least 627),
contributor/role-label differences (at least 462), case-only drift, date
formatting, and broader text/order differences. Every cluster needs
authoritative classification before fixes.

Acceptance criteria:
- [ ] Cluster exact-parity drift by stable semantic cause and style family.
- [ ] Attribute each high-volume cluster to style YAML, shared renderer/schema, benchmark carrier, or intentional divergence.
- [ ] Prioritize by aggregate CSL reach and open bounded implementation beans for actionable families/shared causes.
- [ ] Add representative regression coverage before changing behavior.
- [x] Ratchet exact parity only where authority has been verified; do not change lenient compatibility gates implicitly.

Evidence: `/tmp/core-report-tristate.json` from the csl26-zik7 implementation run; canonical dashboard is `docs/compat.html`.



Strategic clarification:
- The legacy `fidelityScore` is a lenient compatibility score, not sufficient evidence of textual fidelity.
- The 25.2% portfolio result is a row-weighted oracle comparison that may be dominated by repeated shared causes; it is explicitly unadjudicated and non-gating.
- Oracle-text drift is symmetric evidence. A difference may be a Citum defect, an oracle defect, an intentional divergence, or an unresolved authority question.
- The `Merriam-Webster.com dictionary` fixture demonstrates why adjudication is required: citeproc changes `.com` to `.Com`, while Citum preserves the fixture's domain casing, but the same row contains other punctuation differences that must be assessed independently.
- Every visible punctuation, label, case, bracket, numbering, and ordering difference remains part of the comparison and requires authority-based adjudication.
- The current compatibility baseline remains useful as a regression guard; it must not be presented as proof of exact style fidelity.
- Dashboard previews must expose the first differing span. The original 100-character truncation made 233 mismatches appear identical during spot checks.
- Missing alignment sides must be written out rather than shown with an easily misread empty-set glyph, and render-only native smoke tests must be distinguished from oracle comparisons.

Additional acceptance criteria:
- [x] Recommend durable public terminology for compatibility versus textual fidelity, including the legacy JSON field.
- [x] Propose a staged exact-parity ratchet and CI migration plan by family after adjudication.
- [ ] Quantify unique root causes separately from repeated affected rows so prioritization reflects both defect breadth and impact.


Pairing correction:
- 162 one-sided, ID-less bibliography observations are now explicitly
  not-comparable and excluded from both compatibility and exact-parity totals.
- The current snapshots contain no ID-proven bibliography omissions; csl26-5okt
  blocks this triage until CSL snapshots preserve item IDs and authoritative
  pairing can replace the heuristic fallback.

## Summary of Changes (2026-07-31)

Delivered the gate infrastructure and terminology this bean's strategic-clarification section called for:

- Fixed `summarizeExactParity` (`scripts/report-core.js`) to read divergence-adjusted oracle sections — it previously ignored registered divergences (div-004/005/008/009/010/011) entirely, understating parity for every style with one. AMA went 23.9% -> 71.6% once the fix is applied.
- Added a hard, per-style, monotonic exact-parity gate to `scripts/check-core-quality.js` (`--parity-baseline`) and `.github/workflows/fidelity.yml` (both `mode=all` and the previously-ungated `mode=selected` path). Floor = current `passed` count per embedded-core style (regenerated at HEAD in `scripts/report-data/embedded-parity-baseline.json`); fixture-drift guard on `total`. The lenient fidelity gate is unchanged, per this bean's own acceptance criterion.
- Added `scripts/report-data/parity-adjudication.json`, a lightweight ledger (distinct from `docs/adjudication/DIVERGENCE_REGISTER.md`) with three states — `citeproc-correct`/`unclear` (agent-writable) and `citum-correct` (user-only, requires a cited authority) — so agents have a defined escalation path instead of unilaterally excluding hard residuals.
- Full writeup: `docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md`.
- Updated `STYLE_WORKFLOW_EXECUTION.md`, `STYLE_WORKFLOW_DECISION_RULES.md`, and the style-tune/style-qa/style-migrate-enhance/style-evolve skills (both `.claude/skills/` and `.skills/` copies) to the fidelity -> exact-parity -> SQI ordering.

**Not done here** (remains open, this bean stays in-progress): the actual clustering/triage of accumulated exact-parity residuals by semantic cause and style family — the original acceptance criteria this bean was created for. The ledger and gate now exist to record that work's output; the triage pass itself is future work.


## Follow-up hardening (2026-07-31)

- Made the exact-parity baseline and adjudication-ledger inputs fail closed on missing or malformed JSON, with CLI regression coverage.
- Added the quality-gate tests and parity data files to Fidelity CI coverage.
- Synchronized exact-parity workflow guidance across Claude, public, and Codex agent skill surfaces.
