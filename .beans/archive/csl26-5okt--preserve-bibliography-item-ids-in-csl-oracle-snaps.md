---
# csl26-5okt
title: Preserve bibliography item IDs in CSL oracle snapshots
status: completed
type: bug
priority: high
tags:
    - oracle
    - snapshots
    - fidelity
created_at: 2026-07-28T13:46:03Z
updated_at: 2026-08-02T15:01:57Z
parent: csl26-6th8
blocking:
    - csl26-6th8
---

The CSL oracle snapshots currently preserve rendered bibliography text but not
the source item ID associated with each output row. As a result, snapshot-based
reports must pair bibliography rows by text similarity and cannot prove whether
leftover rows are real omissions or additions.

Implement ID-preserving bibliography snapshots so the report can replace
heuristic pairing with authoritative item-ID pairing wherever the source
processor exposes IDs.

Acceptance criteria:
- [ ] Extend the CSL snapshot contract and generator to preserve the source item
  ID for every bibliography entry.
- [ ] Update snapshot loading and fast-oracle comparison to pair complete
  ID-bearing outputs by ID.
- [ ] Preserve the neutral heuristic fallback when either side lacks complete
  item IDs.
- [ ] Regenerate affected CSL snapshots and document any format-version or
  compatibility implications.
- [ ] Test ID-paired rows, ID-proven oracle-only rows, ID-proven Citum-only rows,
  incomplete-ID fallback, and grouped evidence-run reporting.
- [ ] Audit existing heuristic-unpaired observations and remove them from the
  unresolved category wherever regenerated snapshots provide authoritative IDs.

Specification:
`docs/specs/STYLE_COMPATIBILITY_INHERITANCE_REPORT.md`

This blocks parity triage because unpaired output must be classified by
authoritative identity before it can be treated as a rendering defect.
