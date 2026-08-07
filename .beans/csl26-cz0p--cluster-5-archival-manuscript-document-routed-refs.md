---
# csl26-cz0p
title: 'Cluster 5: archival / manuscript / document-routed refs'
status: todo
type: task
priority: normal
created_at: 2026-08-07T13:21:11Z
updated_at: 2026-08-07T13:21:11Z
parent: csl26-h7oc
---

Per docs/specs/CHICAGO_FAMILY_STRATEGY.md cluster ordering. Author-date-18th's manuscript type-variant lacks archive-collection that notes-18th's has; CSL-document-routed archival refs (Purcell map, Agassiz, Henshaw, Johnson, Concerning-a-court-of-arbitration) need the same archival treatment as the merged manuscript/collection type-variant but 'document' is also used by ~30 unrelated placeholder items in the shared corpus — adding it naively dropped total entry count 400->397 (csl26-giun 2026-07-02 note, reverted as a regression). Needs a narrower selector than the bare type, not a blanket document merge. All Section-C accessor facts already exist per csl26-ifhx (scrapped, findings absorbed here) — this is YAML template wiring only, no Rust work.
