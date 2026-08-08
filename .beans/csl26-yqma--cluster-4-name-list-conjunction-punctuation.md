---
# csl26-yqma
title: 'Cluster 4: name-list conjunction punctuation'
status: todo
type: task
priority: normal
created_at: 2026-08-07T13:20:49Z
updated_at: 2026-08-08T13:36:20Z
parent: csl26-h7oc
---

Per docs/specs/CHICAGO_FAMILY_STRATEGY.md cluster ordering. Missing comma before 'and' in a 2-author name list ('Smith, Jane and Robert Williams' vs oracle's 'Smith, Jane, and Robert Williams'). ~62 residual observations, subset of a broader punctuation-only diff class (2026-07-30 clustering, csl26-giun evidence). Fix across all four Chicago-family styles at once.


## Re-check against current data (2026-08-08)

Re-verified against `node scripts/report-core.js` rather than the 2026-07-30 clustering this bean's ~62 estimate came from. Scanning chicago-author-date-18th + taylor-and-francis-chicago-author-date's currently-failing bibliography entries for "oracle has a comma before 'and' in a name list, citum doesn't" found only 2 hits total, and both look like a different defect (a missing "In ..." container wrapper), not name-list punctuation. The ~62 estimate looks stale -- likely already fixed by intervening work (contributor-role/name-list work has landed in clusters 1-2 and elsewhere since 2026-07-30).

Don't scope this cluster against chicago-shortened-notes-bibliography's failures without adjustment: its bibliography output currently shows a much bigger, unrelated, single-cause defect -- 326 of 422 failing bibliography entries (77%) match a signature where the whole entry is comma-delimited where it should be period-delimited (`bibliography.options.separator: ", "` instead of `". "`, tracked as csl26-zl7f). That signature will produce false positives against any "missing comma"-style pattern match run against shortened's raw diffs, including this cluster's. Re-derive this cluster's actual current scope from a fresh sample after csl26-zl7f lands, across all three styles, rather than trusting the original 2026-07-30 estimate or scanning shortened before that fix.
