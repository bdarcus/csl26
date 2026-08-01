---
# csl26-4q7v
title: Chicago interview variant leads with title instead of author
status: todo
type: bug
priority: normal
tags:
    - style
    - chicago
    - fidelity
created_at: 2026-08-01T11:58:13Z
updated_at: 2026-08-01T11:58:13Z
---

chicago-author-date-18th.yaml's interview variant diverges from citeproc-js (Delta1, row 39 in the chicago-shared-corpus benchmark): oracle leads 'Bengio, Yoshua. 2023....' but citum leads with the quoted title instead, because the variant's first group is gated render-when: field-present: title and outranks the author-first group. Independent of the punctuation-in-quote fix in csl26-1hya, which fixes only the quote-placement byte in this row.
