---
# csl26-m8la
title: Align year-suffix ordering with bibliography render order for constant-fallback collision groups
status: todo
type: bug
priority: high
tags:
    - style
    - fidelity
    - engine
created_at: 2026-07-23T17:05:46Z
updated_at: 2026-08-02T14:19:28Z
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
