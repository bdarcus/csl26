---
# csl26-ctkb
title: Implement year-suffix merged/ranged collapse rendering
status: todo
type: feature
priority: normal
tags:
    - rendering
    - citation
    - engine
created_at: 2026-08-19T13:47:45Z
updated_at: 2026-08-19T13:48:02Z
---

`SameAuthorCollapse::year_suffix` supports `Merged` (`Smith (2020a, b)`) and `Ranged` (`Smith (2020a–c)`) — CSL's `collapse="year-suffix"` / `collapse="year-suffix-ranged"`. Both parse and round-trip through the schema (landed in csl26-ecfn's implementation, `docs/specs/SAME_AUTHOR_COLLAPSE.md`) but the renderer doesn't implement either degree yet — it falls back to `Separate`, with a one-time `SchemaWarning::UnimplementedCollapseDegree` at `citum style validate` time (not yet at render time — see below) and a migrate-time `tracing::warn!`.

Two embedded/tracked styles already declare the merged degree and are silently rendering the wrong output today: `springer-basic-author-date-core` and `international-journal-of-wildland-fire` (both `collapse: { same-author: { year-suffix: merged } }`).

## Scope
- [ ] Implement `Merged" rendering: join adjacent same-year suffixed tokens sharing a year, e.g. `2020a, 2020b` → `2020a, b`.
- [ ] Implement `Ranged" rendering: collapse a contiguous run of suffixes into a range, e.g. `2020a, 2020b, 2020c` → `2020a–c`.
- [ ] Wire the render-time warning (currently only surfaced via `citum style validate`, not at actual render time) or confirm the validate-time channel is sufficient.
- [ ] Re-run `report-core.js --style springer-basic-author-date` and `--style international-journal-of-wildland-fire` to confirm the fix moves exactParity (currently both render `Separate` despite declaring `merged`).

See `docs/specs/SAME_AUTHOR_COLLAPSE.md` §1, §4, Scope section.
