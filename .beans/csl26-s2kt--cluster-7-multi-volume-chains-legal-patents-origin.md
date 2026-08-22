---
# csl26-s2kt
title: 'Cluster 7: multi-volume chains, legal, patents, original/reprint trailers'
status: in-progress
type: task
priority: normal
created_at: 2026-08-07T13:21:11Z
updated_at: 2026-08-21T19:40:44Z
parent: csl26-h7oc
---

Per docs/specs/CHICAGO_FAMILY_STRATEGY.md cluster ordering. Bundles four structural clusters repeatedly deferred in csl26-giun across multiple sessions as 'explicitly out of scope': multi-volume chains ('Bk. 3 of..., vol. 2 of...'), hearings/legal citation grammar, patents (missing 'filed' date / country prefix), and original/reprint trailer text ('Reprint'/'Originally published as X (publisher)' — blocked historically on TemplateConditionField not exposing original-publisher/original-publisher-place for render-when; re-verify that blocker still holds before starting, since csl26-ifhx found several adjacent 'missing' accessors were already implemented). Split into separate beans if investigation shows they don't share a fix.


## Fresh counts (2026-08-08, node scripts/report-core.js, references-expanded + chicago-18th corpora)

| type | author-date-18th | T&F | shortened |
|---|---|---|---|
| legal_case | 9/9 | 9/9 | 8/8 |
| legislation | 11/11 | 11/11 | 10/10 |
| treaty | 2/2 | 2/2 | 1/1 |
| regulation | 2/2 | 2/2 | 1/1 |
| hearing | 3/3 | 3/3 | 2/2 |
| bill | 5/7 | 5/7 | 6/6 |
| standard | 5/5 | 5/5 | 4/4 |
| patent | 1/3 | 1/3 | 2/2 |
| report | 5/8 | 5/8 | 6/6 |

Every legal/regulatory sub-type is at or near 100% failure across all three styles -- legal_case, legislation, treaty, regulation, hearing, and standard are fully failing everywhere they're comparable. This confirms the "explicitly out of scope, repeatedly deferred" history is a real, large gap, not a minor one, and that these read as unauthored or badly wrong type-variant templates rather than punctuation-level drift -- consistent with this bean's suggestion to split into separate beans, since patent and report show partial (not total) failure and may have a different, narrower cause than legal_case/legislation/treaty/regulation/hearing/standard's wholesale gap.

author-date-18th and T&F are byte-identical on every sub-type here, so any fix on the shared ancestor clears both at once; multi-volume chains weren't separately isolated in this pass (no clean type signal for that sub-cluster in the available data) and still need the original investigation.
