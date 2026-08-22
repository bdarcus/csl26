---
# csl26-t0m4
title: Chicago broadcast variant missing Episode literal and Television medium
status: completed
type: bug
priority: normal
tags:
    - style
    - chicago
    - fidelity
created_at: 2026-08-01T11:58:13Z
updated_at: 2026-08-21T18:13:17Z
parent: csl26-h7oc
---

chicago-author-date-18th.yaml's broadcast variant diverges from citeproc-js (Delta49, row 37 in the chicago-shared-corpus benchmark): oracle renders 'Episode 1, ...Aired September 28. Television.' but citum renders '1, ...Aired September 28.' -- missing the literal 'Episode' term (styles-legacy/chicago-author-date.csl uses text-case=capitalize-first value=episode) and the 'Television' medium label. Independent of the punctuation-in-quote fix in csl26-1hya, which fixes only the quote-placement byte in this row.

\n\nOutcome (Chicago style-only wave, 2026-08-21): Added the citeproc-confirmed Episode message and television-medium wiring to the author-date and notes broadcast variants. Fresh family reports verify the broadcast examples; author-date and Taylor & Francis are both 179/542, notes 24/72.

\n\n## Summary of Changes\n\nAdded the citeproc-confirmed Episode message and television-medium wiring to the author-date and notes broadcast variants. Fresh family reports verify the broadcast examples; author-date and Taylor & Francis are both 179/542, notes 24/72.
