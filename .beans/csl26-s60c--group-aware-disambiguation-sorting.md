---
# csl26-s60c
title: Group-aware disambiguation sorting
status: todo
type: task
priority: normal
created_at: 2026-02-16T14:06:45Z
updated_at: 2026-02-16T14:06:45Z
---

Phase 2: Add group sort support to year suffix assignment.

Files:
- crates/csln_processor/src/processor/disambiguation.rs

Tasks:
1. Add group_sort field to Disambiguator struct
2. Add with_group_sort() constructor
3. Update apply_year_suffix() to use GroupSorter
4. Add unit tests for group-aware suffix assignment

Acceptance:
- Legal citations sorted by case name, not title
- Type-order grouping sorts by reference type first
- Year suffix respects group sort order

Refs: docs/architecture/DISAMBIGUATION_MULTILINGUAL_GROUPING.md Phase 2
