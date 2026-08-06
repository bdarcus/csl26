---
# csl26-92kv
title: 'entomological-society-of-america: year-suffix order regressed for a same-title-differing collision pair'
status: todo
type: bug
priority: low
created_at: 2026-08-06T13:31:14Z
updated_at: 2026-08-06T13:31:27Z
parent: csl26-ccdt
---

csl26-m8la's registry-order year-suffix fix caused one single-entry regression in the full 35-style exemplar corpus check: entomological-society-of-america's 'disambiguate-year-suffix' citation case flipped from exact-parity pass to fail (exactParity 9/67 -> 8/67). Root cause not isolated within a bounded investigation; documented as a known residual per explicit accept-and-move-on decision.

## Evidence

- Fixture: `disambiguate-year-suffix` in `tests/fixtures/citations-expanded.json`, citing `ITEM-31` then `ITEM-32` (both "Garcia", both issued 2019, same container-title, differing only in article title: "...Robust Climate Attribution" vs "...Probabilistic Climate Attribution").
- Oracle: `(Garcia 2019a, 2019b)` — ITEM-31 (Robust) gets `a`.
- citum before csl26-m8la: matches oracle.
- citum after csl26-m8la's engine fix: `(Garcia 2019b, 2019a)` — swapped.
- `entomological-society-of-america` extends `elsevier-harvard`, which
  resolves `bibliography.sort: author-date-title`
  (`SortPreset::AuthorDateTitle` — Author, Issued, Title, all ascending).
  `elsevier-harvard` itself *improved* under the same fix (40/67 -> 45/67),
  so this isn't a universal regression in the preset — just this one pair.

## Why the obvious explanation doesn't hold

The two references' Title keys genuinely differ ("Probabilistic" vs
"Robust"), so `sort_by_keys`'s real Title comparison should govern the tie
regardless of what the disambiguator's pre-sort does before it — the
pre-sort (registry index, or full issued date) only matters when *all*
template keys tie. That should make csl26-m8la's pre-sort change irrelevant
to this specific pair, but the render order changed anyway. Not root-caused
within the time-boxed investigation; needs a closer look at
`Disambiguator::sort_group_for_year_suffix`'s interaction with
`ReferenceSorter::sort_by_keys`'s stability guarantees for this case,
ideally with a minimal reproduction outside the full corpus harness.
