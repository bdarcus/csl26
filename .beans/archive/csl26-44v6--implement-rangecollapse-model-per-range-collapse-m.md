---
# csl26-44v6
title: Implement range/collapse model per RANGE_COLLAPSE_MODEL.md v1.0
status: completed
type: task
priority: high
tags:
    - engine
    - schema
    - style
    - chicago
created_at: 2026-08-28T17:15:04Z
updated_at: 2026-08-28T18:31:53Z
parent: csl26-awlo
---

PR 2 of the range/collapse stack (docs/range-collapse-config-audit-spec is PR 1, merging soon). Implements the resolved spec: rename PageRangeFormat->generic type + range-format/range-delimiter YAML naming; LocatorConfig::range_format becomes Option, applies to all locator kinds by default (Decision 2) with per-kind opt-out; separate configurable delimiter chains for page/locator and identifier/suffix ranges, retiring 3 hardcoded literals; remove CitationOptions/BibliographyOptions scoped page-range-format field entirely; delete dead pattern.page-range locale message from 5 locales; style-authoring guide page; just schema-gen; oracle-verify blast radius (chicago-author-date-18th, chicago-shortened-notes-bibliography-core, chicago-notes-bibliography-17th-edition, elsevier-vancouver-core, springer-vancouver-brackets-core, modern-language-association) + full check-core-quality gate; register any oracle divergence from the Decision-2 all-kinds default in DIVERGENCE_REGISTER.md.

## Todo

- [x] Rename `PageRangeFormat` -> generic type name (e.g. `RangeFormat`); update all Rust call sites
- [x] `LocatorConfig::range_format` -> `Option<RangeFormat>`; 3 `LocatorPreset::config()` arms set `None`
- [x] Style-level YAML: `page-range-format`/`page-range-delimiter` -> `range-format`/`range-delimiter` (with `chicago16` value spelling retained)
- [x] Locator resolution chain applies style-wide default to every kind (Decision 2); `locators.kinds.<kind>.range-format` is the opt-out
- [x] Shared delimiter resolution: page variables and locators use `range-delimiter`; citation-number/compound/suffix ranges use configurable `identifier-range-delimiter` with an independent en-dash default -- retired 3 hardcoded literals
- [x] Remove `page_range_format` from `CitationOptions`/`BibliographyOptions` + their `to_config()` conversions
- [x] Delete dead `pattern.page-range` locale message from 5 locale files
- [x] `just schema-gen`
- [x] Update embedded style YAML field names portfolio-wide (page-range-format -> range-format)
- [x] docs/guides/style-authoring/ page for the model (options.html, Range and number formatting section)
- [x] Oracle-verify blast-radius styles + check-core-quality gate
- [x] Register any Decision-2 divergence in DIVERGENCE_REGISTER.md -- N/A, no new divergence appeared (gate warnings unchanged before/after)
- [x] just pre-commit (fmt, clippy, nextest) -- green at every commit

## Summary of Changes

Implemented the full RANGE_COLLAPSE_MODEL.md v1.0 spec as 4 jj/git commits on feat/range-collapse-engine, stacked on docs/range-collapse-config-audit-spec:

1. Mechanical rename (PageRangeFormat->RangeFormat, page-range-format/page-range-delimiter->range-format/range-delimiter across Rust + 21 style YAML files + citum-migrate + docs; removed CitationOptions/BibliographyOptions' scoped field per Decision 3). Oracle-neutral -- verified zero corpus drift.
2. Locator all-kinds inheritance (Decision 2): LocatorConfig::range_format -> Option<RangeFormat>, render_locator() threads the style-wide default through. chi-chapter-locator fixed (exact-parity passed 209->210 on chicago-author-date-18th).
3. Configurable identifier-range delimiter (Decision 1 mechanism): citation-number/compound/suffix ranges resolve `options.identifier-range-delimiter` with an independent en-dash default instead of using 3 hardcoded literals or the page/locator delimiter. Added options.citation-numbers.range-format (independent default, per spec's architecture-test example).
4. Docs cleanup: deleted dead pattern.page-range locale message (5 locales), documented the model in docs/guides/style-authoring/options.html, regenerated schemas.

Every commit individually passes just pre-commit and a full-corpus report-core.js + check-core-quality.js gate with zero new regressions or unregistered divergence.
