---
# csl26-4q7v
title: Chicago interview variant leads with title instead of author
status: completed
type: bug
priority: normal
tags:
    - style
    - chicago
    - fidelity
created_at: 2026-08-01T11:58:13Z
updated_at: 2026-08-21T18:13:16Z
parent: csl26-h7oc
---

chicago-author-date-18th.yaml's interview variant diverges from citeproc-js (Delta1, row 39 in the chicago-shared-corpus benchmark): oracle leads 'Bengio, Yoshua. 2023....' but citum leads with the quoted title instead, because the variant's first group is gated render-when: field-present: title and outranks the author-first group. Independent of the punctuation-in-quote fix in csl26-1hya, which fixes only the quote-placement byte in this row.

\n\nOutcome (Chicago style-only wave, 2026-08-21): Re-routed the author-first interview bibliography variant and matched genre/no-genre interviewer grammar against citeproc-js. Fresh author-date exact parity improved to 179/542 with fidelity preserved at 0.890.

\n\n## Summary of Changes\n\nRe-routed the author-first interview bibliography variant and matched genre/no-genre interviewer grammar against citeproc-js. Fresh author-date exact parity improved to 179/542 with fidelity preserved at 0.890.
