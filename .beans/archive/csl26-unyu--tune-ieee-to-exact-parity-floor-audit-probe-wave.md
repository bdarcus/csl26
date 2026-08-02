---
# csl26-unyu
title: Tune ieee to exact-parity floor (audit probe wave)
status: completed
type: task
priority: high
tags:
    - migrate
    - engine
    - fidelity
    - style
created_at: 2026-08-01T20:25:14Z
updated_at: 2026-08-02T13:17:42Z
---

Ran the ieee tuning wave the interpretation audit (csl26-2jls) called for: parity-targeted tuning on ieee, cost recorded, decision rule fixed in advance (90%+ at sane cost = translation is fine; stalling in the 50s-60s = justifies scoping a CSL interpreter).

Result: 84/149 -> 88/149 exact parity (56.4% -> 59.1%), fidelity 100% throughout. Landed as commit 12865760 on style/ieee-exact-parity-wave.

What shipped: a general engine bug fix (bibliography numeric labels wrongly counted as content when their sibling is empty -- affects every numeric-processing style, not just ieee) plus three small ieee.yaml fixes (motion-picture date parens, patent number, removed a redundant label component).

What was tried and reverted: a shared role-label preset fix that helped ieee but silently regressed chicago-author-date-18th and american-medical-association -- caught before landing, filed as csl26-g6bi with the regression evidence attached.

Verdict: 59.1% is inside the "stall" band the decision rule named, but one general bug fix improving multiple styles for free complicates a simple yes/no read -- see the audit addendum (docs/architecture/audits/2026-08-01_CSL_DIRECT_INTERPRETATION_ANALYSIS.md) for the full discussion.

Follow-ups, all under csl26-ccdt: csl26-ww77 (start here), csl26-g6bi, csl26-y49d, csl26-3az5, csl26-lhrl.
