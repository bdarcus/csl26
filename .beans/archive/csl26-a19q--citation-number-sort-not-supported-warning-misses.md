---
# csl26-a19q
title: citation_number_sort_not_supported warning misses inherited bibliography.sort
status: completed
type: bug
priority: normal
tags:
    - engine
    - schema
created_at: 2026-08-06T12:43:41Z
updated_at: 2026-09-02T13:55:53Z
parent: csl26-ccdt
---

The citation_number_sort_not_supported style-load warning
(crates/citum-engine/src/api/warnings.rs) inspects only
processing.config().sort, not style.bibliography.sort. A style that
inherits bibliography.sort: citation-number from a numeric base (e.g. an
author-date leaf extending a numeric base, as
gb-t-7714-2025-author-date.yaml still does — see the follow-up bean tracking
its own explicit sort) silently renders its bibliography in registry order
with no warning at all, even though citation-number has no group-sort
equivalent for a non-numeric processing family.

Needs a family-aware check: warn when an author-date/note-family style
resolves to bibliography.sort: citation-number, since that's legitimate only
for numeric styles.

Discovered while tracing csl26-m8la's root cause: the migrated
gb-t-7714-2025-author-date.yaml had no bibliography.sort of its own and
silently inherited the numeric base's citation-number sort, going unnoticed
specifically because this warning didn't fire.

## Summary of Changes

Made `scan_bibliography_sort_for_citation_number` (renamed from
`scan_bibliography_config_sort_for_citation_number`) family-aware:

- Exposed `Processor::resolved_bibliography_sort` as `pub(crate)` — it already
  implements the correct precedence (style-level `bibliography.sort` override
  -> processing-family preset default -> config-level explicit sort), so the
  warning now reuses the exact resolution the renderer uses instead of
  re-deriving it.
- Kept the original explicit-config-key check (citation-number listed among
  several keys, silently dropped by `Sort::group_sort`) unchanged.
- Added a second check: when the *effective* bibliography sort resolves to an
  empty group-sort template (the `citation-number` preset's shape) for a
  non-numeric processing family (author-date/note/label), warn — this is the
  gap the bean reported, and it now fires for
  `gb-t-7714-2025-author-date.yaml`, surfacing the defect csl26-q67h tracks.
  Guarded to skip when the first check already warned, since both read the
  same config-level step.

5 new `#[rstest]` cases in `warnings.rs` cover: own citation-number sort on an
author-date family (warns), note family (warns), numeric family (silent —
legitimate no-sort idiom), no bibliography.sort declared (silent), and an
author-date family's own non-citation-number sort (silent). The pre-existing
integration test for the explicit-key case still passes unchanged.

Deliberately not fixed here: csl26-q67h (restoring
gb-t-7714-2025-author-date's own bibliography.sort) — that was attempted and
reverted in csl26-m8la because it regressed
american-medical-association-alphabetical's exact parity from 21/67 to 1/67.
This change only makes the existing silent defect visible.
