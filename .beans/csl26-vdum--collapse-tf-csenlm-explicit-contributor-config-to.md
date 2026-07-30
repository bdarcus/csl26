---
# csl26-vdum
title: Collapse T&F CSE/NLM explicit contributor config to vancouver preset
status: todo
type: task
priority: low
created_at: 2026-07-30T13:02:00Z
updated_at: 2026-07-30T13:02:00Z
parent: csl26-s2rw
blocked_by:
    - csl26-edjj
---

Follow-up from the 2026-07-30 Elsevier/T&F overlap audit (docs/architecture/audits/2026-07-30_ELSEVIER_TF_CORE_OVERLAP.md). taylor-and-francis-council-of-science-editors-author-date-core and taylor-and-francis-national-library-of-medicine-core independently hand-write the same Vancouver-style bibliography.options.contributors.* block (display-as-sort, name-form: initials, initialize-with, delimiter, delimiter-precedes-last, sort-separator) instead of using the existing contributors: vancouver preset. Collapse both to the preset shorthand field-by-field, verify no fidelity/SQI regression via report-core.js --styles for the two styles, and just check-core-quality.
