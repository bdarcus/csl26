---
# csl26-group
title: Implement configurable bibliography grouping
status: in-progress
type: feature
priority: normal
created_at: 2026-02-15T00:00:00Z
updated_at: 2026-02-16T15:50:00Z
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

Phase 1 complete ✅: Schema extension in csln_core committed (f58f013).
Starting Phase 2: Selector logic implementation.

Phase 2 complete ✅: Selector evaluation logic (8 tests passing).

Starting Phase 3: Group sorting implementation.

Phase 3 complete ✅: Group sorting (type-order, name-order, composite).

Starting Phase 4: Processor integration.

Phase 4 complete ✅: Processor integration (custom groups + legacy fallback).

Starting Phase 5: Legal use case validation.

Phase 5 complete ✅: Legal hierarchy infrastructure (style + fixtures).

Starting Phase 6: Multilingual use case validation.

Phase 6 complete ✅: Multilingual sorting infrastructure (style + fixtures).

Phases 1-6 complete. Ready for Phase 7 (documentation).

Note: CLI testing reveals that bibliography grouping requires citation
data (cited_ids/silent_ids) to be populated via document processing.
Direct `csln process` command doesn't provide citations, so all
references fall into ungrouped fallback. Need integration tests that
use process_document() API for proper validation.
