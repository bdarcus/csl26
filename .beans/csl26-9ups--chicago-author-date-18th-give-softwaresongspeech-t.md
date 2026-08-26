---
# csl26-9ups
title: 'chicago-author-date-18th: give software/song/speech title formatting (substitute style-content gap)'
status: todo
type: task
priority: normal
tags:
    - style
    - chicago
    - title
    - fidelity
created_at: 2026-08-26T14:50:53Z
updated_at: 2026-08-26T14:50:56Z
parent: csl26-h7oc
---

Style-content-only fix (no engine/schema change): software and song type-variants in chicago-author-date-18th.yaml have no formatting on their title: primary node (should be emph: true, per CMOS 18); speech has no dedicated bibliography type-variant at all and falls to the default template (also missing emph). Once title-rendering: from-template ships (csl26-0u0f), these three types' author-less substituted titles still render plain because there is nothing on the resolved template node to derive from -- confirmed by hand-simulation in docs/specs/SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md §4.3/§5 (5 of the 40-row taxonomy: software x3, song x2, speech x1). Fix: add emph: true to software/song's title: primary nodes; give speech its own type-variant with emph: true. Independent of and non-blocking for csl26-0u0f's mechanism.
