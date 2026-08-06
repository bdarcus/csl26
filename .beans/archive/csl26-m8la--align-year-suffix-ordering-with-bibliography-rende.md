---
# csl26-m8la
title: Align year-suffix ordering with bibliography render order for constant-fallback collision groups
status: completed
type: bug
priority: high
tags:
    - style
    - fidelity
    - engine
created_at: 2026-07-23T17:05:46Z
updated_at: 2026-08-06T13:33:36Z
parent: csl26-ccdt
---

Disambiguator::sort_group_for_year_suffix (crates/citum-engine/src/processor/disambiguation.rs) has a no-group_sort fallback that hardcodes a title-alphabetical tiebreak (a_title.cmp(b_title)) as the DEFAULT year-suffix ordering whenever a style doesn't configure an explicit bibliography.sort:/group_sort. This is wrong for styles (confirmed: gb-t-7714-2025-author-date) whose actual bibliography render order is NOT title-alphabetical.

## Evidence (csl26-6eak, 2026-07-23 session)

Confirmed via direct evidence, not assumption: citum's own bibliography RENDER position for a large anonymous-author collision group (the 佚名+无日期 bucket, ~25 items) does not correlate at all with the LETTERS it assigns them — e.g. render position 0 gets letter 'k', position 1 gets 'w', position 3 gets 'a'. Oracle (citeproc-js), by contrast, assigns a/b/c... in exact bibliography render order.

This means citum's own suffix order disagrees with citum's own render order — an internal inconsistency, not (necessarily) a deeper bibliography-sort divergence from the oracle. Fixing sort_group_for_year_suffix's no-group_sort fallback to follow the ACTUAL resolved bibliography order (rather than a hardcoded title-alphabetical assumption) should fix this bucket AND the smaller `2011a/2011b`, `2012a/2012b`, `2023a/b/c`, `2024a/b`, `2000b/c` swapped-pair cases seen elsewhere in the same corpus (all traceable to the same root cause).

## Scope / risk

This is a SHARED code path: the no-group_sort branch is the DEFAULT ordering for every style that doesn't set an explicit group_sort — likely the common case across the embedded corpus, not rare. Any fix needs a full `just check-core-quality` (157 styles) pass to confirm no regression, in addition to the GB/T author-date oracle corpus (tests/fixtures/test-items-library/gb-t-7714-2025.json, --scope bibliography).

## Recommended approach

Investigate what the CORRECT default ordering should be when no explicit group_sort is configured — likely "the order references actually appear in the rendered bibliography" (which may already be available via some existing render-order/index the Disambiguator doesn't currently have access to at hint-calculation time), rather than a fresh title-alphabetical computation. Do not assume; verify against the oracle for at least gb-t-7714-2025-author-date plus a broader spot-check of other author-date styles with real (non-anonymous) same-author-year collisions.

Part of csl26-6eak (Tune gb-t-7714-2025-author-date to full fidelity) — the single highest-leverage remaining item, ~28 of 41 residual adjusted failures trace to this.

## Corrected diagnosis

The bean as originally filed blamed `Disambiguator::sort_group_for_year_suffix`'s
*no-`group_sort`* fallback branch (its title-alphabetical tiebreak). That branch is
never reached by `gb-t-7714-2025-author-date` — it has always resolved a
`group_sort` (via inherited `citation-number` from its numeric base). The actual
defect is in the *other* branch: when a resolved `group_sort`'s template doesn't
fully order a collision group, `sort_group_for_year_suffix` pre-sorted by
title-alphabetical order regardless — diverging from the renderer
(`ReferenceSorter::sort_references_impl`), which falls back to **registry order**
(or id order, when the resolved sort carries the opt-in id tiebreak). Wherever the
resolved sort didn't fully determine order, citum's own suffix letters disagreed
with citum's own render order.

## What shipped

`Disambiguator::sort_group_for_year_suffix`'s resolved-`group_sort` branch now
mirrors the renderer's actual fallback chain: an empty template (e.g. the
`citation-number` preset) leaves registry order untouched — matching
`sort_references_impl`'s early return — and a non-empty template's ties break by
id (when the resolved sort's id tiebreak is set), then by the full issued date
(not just the year an `Issued` sort key compares — needed to fix a same-year,
no-title collision pair that a year-only comparison couldn't distinguish), then by
registry index. `Disambiguator` gained an `id_tiebreak` flag threaded from
`Processor::resolved_bibliography_sort()` (previously discarded) so this mirroring
is possible at all.

Also investigated and **reverted before shipping**: restoring
`gb-t-7714-2025-author-date.yaml`'s own `bibliography.sort` (it currently still
inherits `citation-number`, registry order, from its numeric base) plus a
companion `sorting.rs` Author-key fix needed to keep that restoration net-neutral
for this style. Measured together they were net-negative — no improvement over the
engine fix alone on this style's own oracle, and a severe regression elsewhere
(`american-medical-association-alphabetical` exact-parity collapsed from 21/67 to
1/67, since the Author-key change touches a shared code path across all 157
styles). Filed as `csl26-q67h` with the measurements and root-cause detail, not
bundled into this fix.

## Numbers

`gb-t-7714-2025-author-date` (`--refs-fixture gb-t-7714-2025.json --scope
bibliography`): adjusted bibliography oracle failures **42 -> 30** (out of 203;
173 passed). This is the engine fix alone — the style's bibliography still renders
in registry order (not a real author+date order), so this is not full parity with
the oracle; it makes citum's suffix letters internally consistent with citum's own
(registry-order) render, which is the failure mode this bean actually reported. A
residual ~9-entry gap in the English anonymous-undated bucket traces to a separate
architectural mismatch (variable-based vs. render-text-based collision grouping),
tracked in `csl26-huuz`, not fixed here.

Full 35-style exemplar corpus (`report-core.js --all-features --parallelism 1`)
vs. clean `main`: 4 styles improved (gb-t-7714-2025-author-date +4,
elsevier-harvard +5, springer-basic-author-date +8,
taylor-and-francis-council-of-science-editors-author-date +2 exact-parity
entries), 1 single-entry regression
(entomological-society-of-america 9/67 -> 8/67, a swapped-suffix pair whose root
cause wasn't isolated in a bounded investigation — tracked in `csl26-92kv`). Zero
regressions in `citum-engine`'s test suite (1236 tests, including 19
new/updated unit tests in `disambiguation.rs` and 2 new integration test cases
in `citations.rs` for this fix).

The `citation_number_sort_not_supported` warning missing this exact
silently-inherited-`citation-number` case (which is why the internal
letter/render-order mismatch this bean reported went unnoticed) is tracked
separately in `csl26-a19q`.
