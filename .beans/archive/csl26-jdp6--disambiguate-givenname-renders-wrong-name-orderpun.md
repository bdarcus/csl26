---
# csl26-jdp6
title: disambiguate-givenname renders wrong name order/punctuation vs oracle
status: completed
type: bug
priority: normal
tags:
    - rendering
    - disambiguation
    - engine
    - citation
created_at: 2026-08-16T18:26:30Z
updated_at: 2026-08-17T18:11:33Z
---

Found while investigating csl26-8nrt (by-cite disambiguation scope) via
taylor-and-francis-council-of-science-editors-author-date's citations-expanded.json
fixture. For citation id `disambiguate-givenname`:

- Oracle (citeproc-js): (A. Johnson 2020; B. Johnson 2020)
- Citum:                (Johnson A 2020; Johnson B 2020)

Two divergences bundled together:
1. Name order — oracle puts the given-name initial before the family name;
   Citum puts family first. Style sets `display-as-sort: all` (sort order for
   all contributor positions, not just first).
2. Initial punctuation — oracle's initial has a trailing period ("A."); Citum's
   does not ("A").

## Investigation needed
- [x] Real CSL scopes `name-as-sort-order="all"` only to the
      bibliography-side `author` macro (`styles-legacy/…author-date.csl:21,28`).
      The citation-side `author-short` macro carries no `name-as-sort-order`
      attribute at all. Migration hoisted the bibliography-only attribute to
      style-global `options.contributors.display-as-sort: all`, wrongly
      applying it to citation scope too. Citum's inversion logic
      (`is_inverted_name_order`, `crates/citum-engine/src/values/contributor/names.rs:892`)
      is correct given the config it was handed — the bug is in the migrated
      style data, not the engine.
- [x] Same hoisting mechanism: real CSL's `author-short` macro sets
      `initialize-with="."`; the bibliography-side `author` macro sets
      `initialize-with=""`. Migration collapsed both to a single style-global
      `initialize-with: ""`, so `initialize_given_name`
      (`crates/citum-engine/src/values/contributor/names.rs:593`) had no
      separator available at citation scope. Not a bypass — the config never
      carried the citation-scoped value.
- [x] `resolve_given_part` / `format_single_name`
      (`crates/citum-engine/src/values/contributor/names.rs:967,1013`) escalate
      the short-form slot to `Initials`/`Full` on
      `disambiguate-add-givenname` and read `ctx.display_as_sort` /
      `ctx.initialize_with` from the scope config passed in — confirmed
      correct against real CSL's per-macro `<name>` semantics once the
      style carries citation-scoped values.
- [x] Added a citation-scoped `contributors: {display-as-sort: none,
      initialize-with: "."}` override to
      `crates/citum-schema-style/embedded/styles/taylor-and-francis-council-of-science-editors-author-date-core.yaml`
      and two `#[rstest]` regression cases in
      `crates/citum-engine/src/processor/tests.rs`. `report-core.js --style
      taylor-and-francis-council-of-science-editors-author-date`: citations
      19/20 -> 20/20, exactParity 23/67 -> 30/67 (family total unchanged at
      67), bibliography unaffected at 44/47. `just pre-commit` and `just
      check-core-quality` both pass.

## Summary of Changes

Root cause was in the migrated style data, not the engine: `citum-migrate`
hoisted per-macro `<name>` attributes (`name-as-sort-order`,
`initialize-with`) that real CSL scopes only to the bibliography-side
`author` macro up to style-global `options.contributors`, silently applying
them to the citation-side `author-short` macro too. Fixed by adding a
citation-scoped `contributors` override that restores the real CSL
citation-only values (`display-as-sort: none`, `initialize-with: "."`).

Whether `citum-migrate` still does this hoisting systemically (any style
whose citation/bibliography name elements differ would be affected) is
unverified and out of scope here — flagged as a possible follow-up bean.
