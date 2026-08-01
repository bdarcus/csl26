---
# csl26-t0m4
title: Chicago broadcast variant missing Episode literal and Television medium
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

chicago-author-date-18th.yaml's broadcast variant diverges from citeproc-js (Delta49, row 37 in the chicago-shared-corpus benchmark): oracle renders 'Episode 1, ...Aired September 28. Television.' but citum renders '1, ...Aired September 28.' -- missing the literal 'Episode' term (styles-legacy/chicago-author-date.csl uses text-case=capitalize-first value=episode) and the 'Television' medium label. Independent of the punctuation-in-quote fix in csl26-1hya, which fixes only the quote-placement byte in this row.
