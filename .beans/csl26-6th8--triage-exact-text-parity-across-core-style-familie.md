---
# csl26-6th8
title: Reassess fidelity claims and triage exact-text parity
status: todo
type: task
priority: high
tags:
    - scorecard
    - styles
    - fidelity
created_at: 2026-07-28T12:17:54Z
updated_at: 2026-07-28T14:01:12Z
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
- [ ] Ratchet exact parity only where authority has been verified; do not change lenient compatibility gates implicitly.

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
- [ ] Recommend durable public terminology for compatibility versus textual fidelity, including the legacy JSON field.
- [ ] Propose a staged exact-parity ratchet and CI migration plan by family after adjudication.
- [ ] Quantify unique root causes separately from repeated affected rows so prioritization reflects both defect breadth and impact.


Pairing correction:
- 162 one-sided, ID-less bibliography observations are now explicitly
  not-comparable and excluded from both compatibility and exact-parity totals.
- The current snapshots contain no ID-proven bibliography omissions; csl26-5okt
  blocks this triage until CSL snapshots preserve item IDs and authoritative
  pairing can replace the heuristic fallback.
