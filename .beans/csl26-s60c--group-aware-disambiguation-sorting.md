---
# csl26-s60c
title: Group-aware disambiguation sorting
status: completed
type: task
priority: normal
created_at: 2026-02-16T14:06:45Z
updated_at: 2026-02-16T16:40:00Z
---

Phase 7: Add group sort support to year suffix assignment.

Files:
- crates/csln_processor/src/processor/disambiguation.rs
- crates/csln_processor/src/processor/mod.rs
- crates/csln_core/src/lib.rs

Tasks:
1. [x] Add group_sort field to Disambiguator struct
2. [x] Add with_group_sort() constructor
3. [x] Update apply_year_suffix() to use GroupSorter
4. [x] Add unit tests for group-aware suffix assignment
5. [x] Integrate global bibliography sort into processor initialization
6. [x] Verify localized sorting in Disambiguator

Acceptance:
- Legal citations sorted by case name, not title
PR: [179](https://github.com/bdarcus/csl26/pull/179)
- Type-order grouping sorts by reference type first
- Year suffix respects group sort order

Refs: docs/architecture/DISAMBIGUATION_MULTILINGUAL_GROUPING.md Phase 2
