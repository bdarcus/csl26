---
# csl26-rpza
title: 'Cluster 6: broadcast and episode grammar'
status: todo
type: task
priority: normal
created_at: 2026-08-07T13:21:11Z
updated_at: 2026-08-08T13:36:41Z
parent: csl26-h7oc
---

Per docs/specs/CHICAGO_FAMILY_STRATEGY.md cluster ordering. Broadcast/episode citations ('Episode 6, "...," written by...') need a dedicated grammar distinct from generic article/serial templates; both author-date-18th and notes-18th currently reuse raw variable:/number: components with no semantic distinction between a TV episode number and a journal issue number (2026-06-30 audit Section C item 2 — accessor exists per csl26-ifhx findings, this is template wiring). Deferred repeatedly in csl26-giun as 'explicitly out of scope' across multiple sessions — this bean exists so it stops being silently dropped.


## Fresh counts (2026-08-08, node scripts/report-core.js, references-expanded + chicago-18th corpora)

| type | author-date-18th | T&F | shortened |
|---|---|---|---|
| broadcast | 8/8 | 8/8 | 7/7 |
| motion_picture | 8/8 | 8/8 | 7/7 |

Both fail 100% across all three styles -- confirms "no dedicated grammar exists" rather than a formatting-edge defect, and confirms this cluster's premise with hard numbers instead of the 2026-06-30 audit's qualitative description.

Two adjacent types show the identical 100%-fail shape and aren't currently in this bean's scope -- worth checking whether they share the same template macro as broadcast/episode before scoping them separately:

| type | author-date-18th | T&F | shortened |
|---|---|---|---|
| speech | 8/8 | 8/8 | 8/8 |
| song | 8/9 | 8/9 | 9/9 |

author-date-18th and T&F are byte-identical on all four types, so a fix on the shared ancestor clears both in one pass.
