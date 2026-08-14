---
# csl26-7u16
title: oracle.js orderingIssues check is blind to positional bibliography divergence
status: todo
type: bug
priority: normal
tags:
    - oracle
    - sorting
    - bibliography
    - testing
created_at: 2026-08-14T14:17:53Z
updated_at: 2026-08-14T14:18:17Z
---

scripts/oracle.js's orderingIssues check pairs bibliography entries by id, not by position, so
it cannot detect that Citum renders two entries in a different order than citeproc even when
every individual entry's rendered text matches exactly (component-level and even exact-string
match per entry). This let a real bug through undetected: chicago-author-date-18th's
bibliography sort ordered "Johnson, Brian" before "Johnson, Alice" (citeproc: Alice first) and
"Smith, John" before "Smith, Jane" (citeproc: Jane first) — orderingIssues reported 0 in both
the buggy and fixed state.

Root cause of the underlying bug (fixed in csl26-tc4x): sort_support::structured_name_sort_text
ignored given names when building the bibliography author sort key, so family-name ties fell
through to title order instead of breaking on given name.

## Todo
- [ ] Change orderingIssues (or add a new check) to compare oracle vs citum bibliography
      entry order positionally, not just per-id text match
- [ ] Re-run the full corpus with the new check to see how many other styles have undetected
      ordering divergence
