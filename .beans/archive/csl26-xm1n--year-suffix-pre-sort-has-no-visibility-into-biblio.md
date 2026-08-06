---
# csl26-xm1n
title: Year-suffix pre-sort has no visibility into bibliography.groups partitioning
status: scrapped
type: bug
priority: normal
created_at: 2026-08-06T14:27:30Z
updated_at: 2026-08-06T16:19:16Z
parent: csl26-ccdt
---

Disambiguator::sort_group_for_year_suffix's registry-index/date pre-sort assumes final bibliography render order is a function of registry order and the resolved bibliography.sort template alone. Styles with an explicit bibliography.groups partition (e.g. chicago-author-date-18th's primary-sources/archival/secondary buckets) can determine final render position through group membership and per-group sorting the Disambiguator has no access to at hint-calculation time.

## Evidence (csl26-m8la PR #1150 review, 2026-08-06)

A GitHub Copilot review comment on PR #1150 correctly flagged that
`ReferenceSorter`'s `Issued` sort key only compares the year
(`CachedSortValue::Issued(Option<i32>)`, compared via `compare_optional_years`
in `sorting.rs`) — there is no month/day comparison anywhere in the compiled
sort-key path. This raised the question of how
`Disambiguator::sort_group_for_year_suffix`'s `year_suffix_date_key`
pre-sort step (added to fix a `chicago-author-date-18th` regression) could
possibly be "mirroring the renderer" when the renderer's own Issued key
can't see months.

Direct verification (comparing rendered bibliography entries for two
same-year, no-title Gourmet magazine references, ids `6188419/9TVQWT5I`
(May 2000) and `6188419/UMQEEZLJ` (September 2000)):

- Registry order in the source fixture
  (`tests/fixtures/test-items-library/chicago-18th.json`): UMQEEZLJ
  (September) appears before 9TVQWT5I (May).
- Actual rendered bibliography position: 9TVQWT5I (May) at index 139,
  UMQEEZLJ (September) at index 140 — May first, reversed from registry
  order.
- Tested directly: pre-sorting by `cached.data.index` alone (registry order,
  no date step) still renders May first at the same position (139) — so the
  *text* render order doesn't depend on the Disambiguator's pre-sort at all
  — but assigns it the *wrong* suffix letter ('b' instead of 'a'), because
  `cached.data.index` predicts September first when the real render puts May
  first.
- `chicago-author-date-18th.yaml` has an explicit `bibliography.groups`
  block (`primary-sources`/`archival`/`secondary`, selected by a `note`
  field) with no per-group `sort` override. `citum-refs`'s reference loading
  has no sort calls anywhere (confirmed via grep), so registry order should
  equal file order — the divergence must originate in how group partitioning
  determines final concatenated render position, not in reference loading.

## Why comparing full issued date "works" without explaining the mechanism

`year_suffix_date_key`'s full-date comparison happens to predict the correct
render order for this pair and hasn't caused a regression in the
gb-t-7714-2025-author-date corpus (173/203, unchanged) or the full 35-style
exemplar corpus diff done for csl26-m8la. But it is not a mirror of any
actual renderer code path — no comparator in `sorting.rs` compares full
issued dates. It is currently shipped as an empirically-verified
approximation, not a principled fix, per PR #1150 review discussion.

## Scope

Understanding this properly needs tracing exactly how `BibliographyGroup`
partitioning (`docs/specs/DISAMBIGUATION.md` §5) determines final entry
position — whether groups are rendered as independently-sorted sub-lists
concatenated in group-declaration order (which would explain the divergence
if, e.g., a per-group stable sort receives its input in a different order
than the flat registry), and whether `Disambiguator` can be given visibility
into that same group-partition-aware ordering instead of relying on either
raw registry index or a date heuristic.

## Reasons for Scrapping

The premise was wrong. `bibliography.groups` partitioning was a coincidental
correlation, not the cause. Direct root-causing (PR #1150 review,
2026-08-06) found the actual mechanism: `ReferenceSorter`'s `Issued` sort
key only ever compared the year (`CachedSortValue::Issued(Option<i32>)`,
via `compare_optional_years`) — never month or day. This was a real,
independent engine bug affecting every style with an Issued sort key, not
something specific to `chicago-author-date-18th`'s group partition.

Fixed directly: widened `CachedSortValue::Issued` to `Option<(i32, u32,
u32)>` (year, month, day) and `compare_by_issued`/the cached-value
comparator to compare the full tuple (`sorting.rs`), added
`DateValue::month()` alongside the existing `.year()`/`.day()`
(`citum-schema-data`). This is the single shared comparator used by both
the real bibliography renderer and `Disambiguator::sort_group_for_year_suffix`
(via `ReferenceSorter::sort_by_keys`), so fixing it at the source made the
Disambiguator's separate `year_suffix_date_key` pre-sort step entirely
redundant — removed it, eliminating the duplicated/drifting logic this
bean was implicitly worried about.

Verified: chicago-author-date-18th's May/September Gourmet pair now
resolves correctly through the real Issued key with no special-casing;
gb-t-7714-2025-author-date unchanged (173/203, matching the target);
full engine test suite green (1236 tests).
