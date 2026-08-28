---
# csl26-awlo
title: Range/collapse config coherence
status: todo
type: epic
priority: high
tags:
    - engine
    - schema
    - style
    - chicago
created_at: 2026-08-26T22:30:01Z
updated_at: 2026-08-26T22:30:06Z
parent: csl26-w0hf
---

The number-range/collapse configuration is 12+ uncoordinated surfaces (page-range-format, locators.range-format, dates.range-format, citation-number collapse markers, compound sub-label collapse, delimiters) with no shared model, inconsistent optionality, hardcoded literals in some paths, and zero docs/guides/style-authoring coverage. Triggered by chi-chapter-locator: (Wilson 2019, 112-118) vs oracle's 112-18 — the locator preset hardcodes PageRangeFormat::Expanded, shadowing the style's page-range-format: chicago16. Full audit in docs/architecture/audits/2026-08-26_RANGE_COLLAPSE_CONFIG.md; design questions in docs/specs/RANGE_COLLAPSE_MODEL.md (Draft). Related: csl26-dfq0 (localization precedent), csl26-boha (migrate-side message emission, distinct), csl26-cl4q (open-ended ranges, must fit the new model).
