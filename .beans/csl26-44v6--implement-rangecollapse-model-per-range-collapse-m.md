---
# csl26-44v6
title: Implement range/collapse model per RANGE_COLLAPSE_MODEL.md v1.0
status: in-progress
type: task
priority: high
tags:
    - engine
    - schema
    - style
    - chicago
created_at: 2026-08-28T17:15:04Z
updated_at: 2026-08-28T18:23:24Z
parent: csl26-awlo
---

PR 2 of the range/collapse stack (docs/range-collapse-config-audit-spec is PR 1, merging soon). Implements the resolved spec: rename PageRangeFormat->generic type + range-format/range-delimiter YAML naming; LocatorConfig::range_format becomes Option, applies to all locator kinds by default (Decision 2) with per-kind opt-out; shared delimiter chain across page/locator/citation-number/compound/same-author-suffix ranges, retiring 3 hardcoded literals; remove CitationOptions/BibliographyOptions scoped page-range-format field entirely; delete dead pattern.page-range locale message from 5 locales; style-authoring guide page; just schema-gen; oracle-verify blast radius (chicago-author-date-18th, chicago-shortened-notes-bibliography-core, chicago-notes-bibliography-17th-edition, elsevier-vancouver-core, springer-vancouver-brackets-core, modern-language-association) + full check-core-quality gate; register any oracle divergence from the Decision-2 all-kinds default in DIVERGENCE_REGISTER.md.

## Todo

- [x] Rename `PageRangeFormat` -> generic type name (e.g. `RangeFormat`); update all Rust call sites
- [x] `LocatorConfig::range_format` -> `Option<RangeFormat>`; 3 `LocatorPreset::config()` arms set `None`
- [x] Style-level YAML: `page-range-format`/`page-range-delimiter` -> `range-format`/`range-delimiter` (with `chicago16` value spelling retained)
- [x] Locator resolution chain applies style-wide default to every kind (Decision 2); `locators.kinds.<kind>.range-format` is the opt-out
- [x] Shared delimiter chain: page var/locators already existed; citation-number/compound/suffix ranges now share one IDENTIFIER_RANGE_DELIMITER constant (deliberately NOT chained through the page-scoped style/locale option -- AMA/ACS corpus evidence: hyphen page-delimiter + en-dash citation-number ranges coexist) -- retired 3 hardcoded literals
- [x] Remove `page_range_format` from `CitationOptions`/`BibliographyOptions` + their `to_config()` conversions
- [ ] Delete dead `pattern.page-range` locale message from 5 locale files
- [ ] `just schema-gen`
- [x] Update embedded style YAML field names portfolio-wide (page-range-format -> range-format)
- [ ] docs/guides/style-authoring/ page for the model
- [ ] Oracle-verify blast-radius styles + check-core-quality gate
- [ ] Register any Decision-2 divergence in DIVERGENCE_REGISTER.md
- [ ] just pre-commit (fmt, clippy, nextest)
