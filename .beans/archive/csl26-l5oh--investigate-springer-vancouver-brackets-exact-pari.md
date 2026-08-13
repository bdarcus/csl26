---
# csl26-l5oh
title: 'Bibliography reference-marker gap: springer/T&F-NLM numeric styles missing label-wrap/label-separator'
status: completed
type: bug
priority: normal
tags:
    - scorecard
    - styles
    - fidelity
    - regression
created_at: 2026-07-31T13:24:53Z
updated_at: 2026-08-13T11:21:56Z
parent: csl26-ccdt
---

Springer-basic-brackets, springer-vancouver-brackets, and taylor-and-francis-national-library-of-medicine each declared `label-mode: numeric` with no `label-wrap`/`label-separator`, so bibliography markers rendered flush against entry text (`1Smith` not `1. Smith`). Added the correct affixes per each style's CSL source; also fixed springer-vancouver's incorrect sentence-case title preset.

Exact-parity: 20→53/67, 11→40/67, 0→13/67. No regressions elsewhere (full-portfolio diff). Baseline regenerated.

Engine-side follow-up (undeclared label-wrap/separator should warn) tracked separately: csl26-tw46 / PR #1178.
