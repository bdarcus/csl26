---
# csl26-tc4x
title: Fix Chicago author-date given-name disambiguation + bibliography given-name sort
status: completed
type: bug
priority: high
tags:
    - chicago
    - disambiguation
    - contributors
    - sorting
    - fixtures
created_at: 2026-08-14T13:51:29Z
updated_at: 2026-08-14T14:18:28Z
parent: csl26-40n4
---

citation.options.contributors: name-form: initials was missing alongside initialize-with, so
disambiguation's given-name expansion never activated initials (Short->Long flip reads
ctx.name_form, which defaulted to Full). Also found and fixing: bibliography sort's
structured_name_sort_text ignores given names entirely (both NameSortOrder arms identical),
causing family-name ties to fall through to title order instead of given-name order.

## Todo
- [x] Fix chicago-author-date-18th.yaml: name-form: initials + initialize-with: '. ' in citation.options.contributors
- [x] Add name-form: full override to personal-communication type-variant (regression guard)
- [x] Fix misleading ContributorConfig::initialize_with doc comment
- [x] just schema-gen (doc comment change affects generated schema)
- [x] Fix structured_name_sort_text/structured_name_sort_as_text given-name blindness in sort_support.rs
- [x] Add rstest coverage for sort key fix
- [x] Add pairs-with field to coverage-manifest.json citations entries
- [x] Enforce pairs-with in check-testing-infra.js + update its test
- [x] Add fixture-pairing section to RENDERING_WORKFLOW.md
- [x] Verify: render refs command from user, oracle exact-parity, report-core, check-oracle-regression, pre-commit gate

## Summary of Changes

Root cause: `initialize-with` only sets the initials separator string; `name-form: initials`
is what activates the initials form. The style's citation-scope contributors config was
missing `name-form: initials`, so the given-name-expansion disambiguation pass flipped
Short->Long form but rendered full names anyway.

- `chicago-author-date-18th.yaml`: added `name-form: initials` alongside `initialize-with: '. '`
  in `citation.options.contributors`; added `name-form: full` override on the
  `personal-communication` type-variant's long-form author node to guard against a regression
  there (CMOS 18 14.111 first-mention case).
- Fixed the misleading `ContributorConfig::initialize_with` doc comment that described CSL
  semantics ("if None, full names used") rather than Citum's actual split; regenerated schemas.
- Found and fixed a second, related bug while verifying: `sort_support::structured_name_sort_text`
  / `structured_name_original_text` ignored given names entirely (both `NameSortOrder` arms
  identical), so bibliography ties on family name fell through to title order instead of given
  name. Fixed via a shared `compose_family_given_key` helper (careful to preserve empty-key
  fallback behavior for anonymous/no-author entries). Updated 3 existing test expectations that
  encoded the old (wrong) tiebreak order; added dedicated rstest + integration coverage.
- Added a `pairs-with` field to every `citations`-kind entry in `coverage-manifest.json`,
  pointing at the references fixture it must render against — verified empirically via
  citation-id/reference-id overlap for all 10 citations fixtures (uncovered that
  `citations-compound-numeric.json` actually pairs with `references-compound-numeric-family.json`,
  not the more obviously-named `compound-numeric-refs.json`). Enforced in
  `check-testing-infra.js` + tests. Added a "Picking a Fixture Pair" section to
  RENDERING_WORKFLOW.md leading with `--show-keys`.

Verification: oracle exact citation parity 16/20 -> 17/20 with `disambiguate-givenname` now
`exactMatch: true`; bibliography byte-identical except the two Johnson entries reordering
correctly (also fixes a previously-undetected `Smith, Jane`/`Smith, John` mis-ordering); full
corpus `report-core.js --all-features` + `check-core-quality.js` gate passed (one pre-existing,
unrelated `ieee` preset-usage warning); `check-oracle-regression.js` clean; full workspace
`cargo nextest run` 2526/2526 passed; `cargo fmt --check` + `clippy -D warnings` clean.

Deferred as separate beans: `GivennameRule`'s `*WithInitials` variants carry an inert form hint
(scope/form conflation) — needs a spec PR per schema-change policy. Oracle's `orderingIssues`
check pairs bibliography entries by id, not position, so it missed the sort bug above.
