---
# csl26-4xr6
title: 'Chicago: title case not applied to bibliography titles'
status: todo
type: task
priority: high
tags:
    - style
    - chicago
    - fidelity
    - title
created_at: 2026-08-23T20:40:25Z
updated_at: 2026-08-23T20:40:25Z
parent: csl26-h7oc
---

Leverage class from docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md wave 1. 182 entries where a type-variant simply omits the title-case transform (YAML wiring), e.g. 'Mesopotamia: between two rivers' vs oracle 'Mesopotamia: Between Two Rivers'. Bundles the sibling over-capitalization sub-cause (31 entries: real stop-word gaps against citeproc-js's skipWordsRex -- in/into/via -- plus post/article-newspaper headlines that should stay sentence case). Single largest measured defect in the family, no prior bean. Do NOT touch docs/policies/TEXT_CASE_PROTECTION.md's internal-caps preservation mechanism (csl26-4kt3) while fixing this -- the 2-entry acronym/mixed-case sub-class (PhD->Phd) is adjacent but out of scope here. Touches all four Chicago variants. Use node scripts/analyze-parity-residuals.js to re-measure.
