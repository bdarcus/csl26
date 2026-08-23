---
# csl26-rrsb
title: 'Chicago: year-suffix letter leaks or resolves wrong'
status: todo
type: task
priority: high
tags:
    - engine
    - chicago
    - fidelity
created_at: 2026-08-23T20:40:45Z
updated_at: 2026-08-23T20:40:45Z
parent: csl26-h7oc
---

Leverage class from the 2026-08-23 audit. 97 entries where a disambiguation year-suffix letter (2019a/2019b) is wrong, missing, or leaks into an adjacent date range. Flagged as an engine-layer defect, not YAML -- classify with the conversion-layer pre-flight (docs/policies/STYLE_WORKFLOW_DECISION_RULES.md) before touching any style file. Touches all four Chicago variants.
