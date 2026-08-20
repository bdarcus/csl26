---
# csl26-lirf
title: Contributor name sort keys carry raw Djot markup
status: completed
type: bug
priority: normal
tags:
    - sorting
    - contributor
    - rendering
    - engine
created_at: 2026-08-20T22:20:33Z
updated_at: 2026-08-20T22:33:58Z
---

multilingual_string_sort_text() in crates/citum-engine/src/sort_support.rs
(used by contributor_sort_key() -> structured_name_sort_text() ->
multilingual_string_sort_text()) returns the raw MultilingualString::Simple
value or complex.original verbatim for name sort-key purposes, just like
title_sort_text() did before csl26-4wts. An institutional/literal author
authored as e.g. "[IBM]{.nocase}" (a literal Contributor::SimpleName or a
StructuredName family field carrying Djot markup) would sort under '[' instead
of 'I'.

Follow-up to csl26-4wts (fix(engine): strip djot markup from title sort keys),
which fixed the identical leak for title sort keys only. Surfaced during that
fix's review rather than folded in, to keep the PR bounded.

Fix should likely reuse the same strip_markup_for_sort() helper added in
csl26-4wts (crates/citum-engine/src/sort_support.rs), applied at
multilingual_string_sort_text()'s return, plus structured_name_original_text()
if that path is affected too.

## Summary of Changes

Folded into the same PR as csl26-4wts (#1213) rather than left as a
follow-up, per instruction not to defer bounded fixes that are trivial once
the infrastructure exists.

- `multilingual_string_sort_text()` in `crates/citum-engine/src/sort_support.rs`
  now strips markup via the `strip_markup_for_sort()` helper added for
  csl26-4wts — covers both `Contributor::SimpleName` (e.g. `[IBM]{.nocase}`)
  and `StructuredName` family/given (via `structured_name_sort_text`).
- `structured_name_original_text()` (the `is_romanized()` transliteration
  fallback path) also strips family/given before composing, since it
  bypasses `multilingual_string_sort_text` and does its own `.to_string()`.

Tests: `given_a_simple_name_with_djot_markup_when_building_the_contributor_sort_key_then_markup_is_stripped`
(rstest, 2 cases) and
`given_a_structured_name_with_djot_markup_in_family_when_building_the_contributor_sort_key_then_markup_is_stripped`
in `sort_support.rs`. Full pre-commit gate (2662/2662) and `report-core.js
--all-features` (zero drift vs `main`) verified together with csl26-4wts.
