---
# csl26-cz0p
title: 'Cluster 5: archival / manuscript / document-routed refs'
status: todo
type: task
priority: normal
created_at: 2026-08-07T13:21:11Z
updated_at: 2026-08-08T13:36:32Z
parent: csl26-h7oc
---

Per docs/specs/CHICAGO_FAMILY_STRATEGY.md cluster ordering. Author-date-18th's manuscript type-variant lacks archive-collection that notes-18th's has; CSL-document-routed archival refs (Purcell map, Agassiz, Henshaw, Johnson, Concerning-a-court-of-arbitration) need the same archival treatment as the merged manuscript/collection type-variant but 'document' is also used by ~30 unrelated placeholder items in the shared corpus — adding it naively dropped total entry count 400->397 (csl26-giun 2026-07-02 note, reverted as a regression). Needs a narrower selector than the bare type, not a blanket document merge. All Section-C accessor facts already exist per csl26-ifhx (scrapped, findings absorbed here) — this is YAML template wiring only, no Rust work.


## Fresh counts (2026-08-08, node scripts/report-core.js, references-expanded + chicago-18th corpora)

By reference type, current exactParity bibliography failures (fail/total), identical across chicago-author-date-18th and taylor-and-francis-chicago-author-date since T&F inherits these type-variants unchanged, plus chicago-shortened-notes-bibliography for comparison:

| type | author-date-18th | T&F | shortened |
|---|---|---|---|
| document | 35/35 | 35/35 | 35/35 |
| manuscript | 13/15 | 13/15 | 13/14 |

document is failing 100% across all three styles -- consistent with "no archival treatment exists yet," not scattered drift. Per this bean's own caution, don't treat 35 as a clean target count: it still likely mixes real archival/document-routed refs with the ~30 unrelated placeholder items that caused the prior document->397 regression (csl26-giun 2026-07-02). manuscript at 13/15 (87-93%) is a cleaner, narrower type to land first and validate the selector approach against before touching document's broader, contaminated pool.

Because author-date-18th and T&F show byte-identical fail/total counts on both types, a fix landed on the shared ancestor template clears both styles in one pass -- confirms the "fix once, land across the family" framing this bean and the epic already assume.
