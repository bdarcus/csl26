---
# csl26-ctkb
title: Implement year-suffix merged/ranged collapse rendering
status: completed
type: feature
priority: normal
tags:
    - rendering
    - citation
    - engine
created_at: 2026-08-19T13:47:45Z
updated_at: 2026-08-20T15:07:21Z
---

`SameAuthorCollapse::year_suffix` supports `Merged` (`Smith (2020a, b)`) and `Ranged` (`Smith (2020a–c)`) — CSL's `collapse="year-suffix"` / `collapse="year-suffix-ranged"`. Both parse and round-trip through the schema (landed in csl26-ecfn's implementation, `docs/specs/SAME_AUTHOR_COLLAPSE.md`) but the renderer doesn't implement either degree yet — it falls back to `Separate`, with a one-time `SchemaWarning::UnimplementedCollapseDegree` at `citum style validate` time (not yet at render time — see below) and a migrate-time `tracing::warn!`.

Two embedded/tracked styles already declare the merged degree and are silently rendering the wrong output today: `springer-basic-author-date-core` and `international-journal-of-wildland-fire` (both `collapse: { same-author: { year-suffix: merged } }`).

## Scope
- [x] Implement `Merged" rendering: join adjacent same-year suffixed tokens sharing a year, e.g. `2020a, 2020b` → `2020a, b`.
- [x] Implement `Ranged" rendering: collapse a contiguous run of suffixes into a range, e.g. `2020a, 2020b, 2020c` → `2020a–c`.
- [x] Removed `SchemaWarning::UnimplementedCollapseDegree` and its validate-time check entirely instead of wiring a render-time warning — the gap it flagged is closed, so the warning describes nothing anymore.
- [x] Re-ran `report-core.js`: springer-basic-author-date-core 53/67 → 54/67; international-journal-of-wildland-fire 14/67 → 15/67.

See `docs/specs/SAME_AUTHOR_COLLAPSE.md` §1, §4, Scope section.

## Summary of Changes

Implemented via a 3-PR stack (each stacked on the previous, per repo's
schema-changes-need-a-docs-PR-first rule):

1. **#1209** `docs(spec): design year-suffix collapse rendering` — revised
   `SAME_AUTHOR_COLLAPSE.md` to v1.2, adding §13 with the full design.
   Key finding: citeproc-js's merged-suffix join delimiter and the
   ordinary year-to-year join are two distinct mechanisms with different
   fallback chains, both fed by the same `cite-group-delimiter` CSL
   attribute — traced directly from citeproc-js source
   (`scripts/node_modules/citeproc/citeproc_commonjs.js`), not guessed.
2. **#1210** `feat(schema): map same-author collapse delimiters` — adds
   `SameAuthorCollapse::delimiter` / `::year_suffix_delimiter`; parses
   CSL's `cite-group-delimiter` / `year-suffix-delimiter` (new fields on
   `csl_legacy::model::Citation`, which didn't capture them before) and
   maps them in migrate; declares `delimiter: ", "` on
   `springer-basic-author-date-core`.
3. **#1211** `feat(engine)!: render year-suffix collapse degrees` — new
   `grouped/year_suffix.rs` merges/ranges same-year suffix tokens after
   item-part rendering, gated on a group-level locator bound. Removes
   `SchemaWarning::UnimplementedCollapseDegree` (breaking change to
   `citum-schema-style`'s public API) now that the gap it flagged is
   closed.

Results: `springer-basic-author-date-core` 53/67 → 54/67 exactParity;
`international-journal-of-wildland-fire` 14/67 → 15/67. Full workspace
suite 2650/2650, zero regressions. All three PRs green on CI.

Mid-implementation correction: an early draft of §13 got the
merged-suffix delimiter precedence backwards (claimed
`year_suffix_delimiter` overrides `delimiter`; citeproc-js's exec-time
override means the reverse). Caught before merge by re-deriving the
precedence from source against both target styles' oracle snapshots;
fixed via `--amend` + `--force-with-lease` on the still-open PR1/PR2
before stacking PR3 on the corrected base.
