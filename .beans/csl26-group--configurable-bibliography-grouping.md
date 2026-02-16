---
# csl26-group
title: Implement configurable bibliography grouping
status: todo
type: feature
priority: normal
created_at: 2026-02-15T00:00:00Z
updated_at: 2026-02-16T00:36:17Z
---

Implement configurable bibliography grouping system.

Based on comprehensive architectural analysis in BIBLIOGRAPHY_GROUPING.md design document.

**Key Features:**
- Per-group sorting (critical for multilingual bibliographies)
- Predicate-based selectors (type, field, cited status)
- First-match semantics (no duplication)
- Graceful fallback for ungrouped items

**Use Cases:**
- Legal hierarchy (Bluebook: Constitutions → Statutes → Cases)
- Multilingual sorting (Vietnamese given-family vs Western family-given)
- Topical grouping (keywords, types, custom metadata)

**Implementation Plan:**
1. Schema extension in csln_core (BibliographyGroup, GroupSelector, GroupSort)
2. Selector logic in csln_processor/src/grouping/selector.rs
3. Group sorting with type-order and name-order variants
4. Processor integration in render_grouped_bibliography_with_format
5. Legal use case validation (bluebook-legal.yaml)
6. Multilingual use case validation (multilingual-academic.yaml)
7. Documentation (style authoring guide, migration from hardcoded)

**Design Principles:**
- Explicit over magic (all grouping in YAML)
- User-defined groups override style-defined groups
- Backward compatible (omitting groups field produces flat bibliography)

See docs/architecture/design/BIBLIOGRAPHY_GROUPING.md for full design.
