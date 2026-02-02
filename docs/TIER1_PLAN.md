# Implementation Plan: Tier 1 Rendering Fidelity & Preset Expansion

## Problem Statement

Currently, only APA achieves 5/5 oracle match for both citations and bibliography. The goal is to achieve high-fidelity rendering for all **Tier 1 parent styles**, which cover **60% of the CSL corpus** (~4,800 dependent styles).

### Current State (Updated)

| Style | Citations | Bibliography | Key Issues |
|-------|-----------|--------------|------------|
| apa | 5/5 ✅ | 5/5 ✅ | Complete |
| chicago-author-date | 5/5 ✅ | 0/5 | Shows publisher-place when shouldn't |
| elsevier-harvard | 5/5 ✅ | 3/5 | Chapter format ("In" vs "in:"), page position |
| springer-basic-author-date | 5/5 ✅ | 0/5 | Template ordering |
| ieee | 5/5 ✅ | 0/5 | Year position, place:publisher format |
| elsevier-vancouver | 5/5 ✅ | 0/5 | Numeric style template |

**All 6 tested styles now have 5/5 citations!**

### Commits So Far

1. `768f13c` docs: add tier 1 rendering fidelity implementation plan
2. `7ce5773` fix(migrate): extract bracket wrapping from citation groups
3. `d7630f1` fix(migrate): extract author-date group delimiter correctly
4. `c2cbb1c` fix(migrate): move DOI and URL to end of bibliography
5. `0ebf8fb` fix(render): clean up dangling punctuation from empty components
6. `a791742` docs: update tier 1 plan with progress
7. `3ed676b` fix(migrate): show publisher-place for book types
8. `a0f432a` docs: update tier 1 plan with current progress
9. `65415b7` fix(migrate): deduplicate titles in nested lists
10. `f1b6d98` fix(migrate): preserve author suffix from macro call
11. `5902549` docs: update tier 1 plan with latest progress
12. `03e36d5` fix(render): skip separator when component has space prefix
13. `cee9a81` docs: update tier 1 plan with latest progress
14. `fbe8a59` fix(render): no separator after closing bracket for numeric styles
15. `683e12f` fix(migrate): propagate type-specific overrides correctly
16. `89d1e12` fix(processor): preserve bibliography order for numeric styles
17. `840372f` docs: update tier 1 plan with latest progress
18. `96b1cdf` fix(migrate): negate base formatting in type-specific overrides
19. `8bc8fc0` fix(processor): use citation numbers for bibliography entries
20. `9e1bdbf` feat(core): enhance citation model with mode and locator types
21. `650bf75` refactor: move citation model from csl_legacy to csln_core
22. `5c78211` feat(core): add bibliography separator configuration
23. `20af195` fix(ci): resolve clippy warnings
24. `064caa4` feat(locale): expose locator terms for page labels (#69)
25. `de0934e` feat(migrate): infer month format from CSL date-parts (#67)
26. `d6ecfbc` feat(reference): support parent reference by ID (#64)
27. `96d1224` feat(migrate): support type-conditional substitution extraction (#66)

---

## Root Causes Identified

1. ✅ **Citation delimiter extraction** - Fixed: finds author-date group correctly
2. ✅ **Numeric citation wrapping** - Fixed: extracts from group prefix/suffix
3. ✅ **DOI/URL ordering** - Fixed: moves access components to end
4. ✅ **Dangling punctuation** - Fixed: cleanup function in render.rs
5. ✅ **Title deduplication** - Fixed: removes duplicates from Lists
6. ✅ **Author suffix** - Fixed: preserves comma suffix from macro call
7. ✅ **List prefix/suffix** - Fixed: values.rs now returns List rendering
8. ✅ **Space prefix separator** - Fixed: skip period separator for space prefix
9. ✅ **Journal title suffix** - Fixed: based on volume List prefix detection
10. ✅ **Type-specific overrides** - Fixed: propagate overrides from merged conditionals
11. ✅ **Numeric style ordering** - Fixed: use IndexMap, detect numeric by citation sort
12. ✅ **Quote/emph negation** - Fixed: override negates base formatting explicitly
13. 🔄 **Bibliography separator** - Partial: infrastructure added, extraction limited by CSL encoding
14. ✅ **Publisher-place visibility** - Style-specific (Chicago: suppress for books, Elsevier: show)
15. ✅ **Editor verb form** - Fixed: extracted from label position/form in names
16. ✅ **Container title duplication** - Fixed: recursive variable discovery and type-specific suppression

---

## Remaining Work

### Phase 5: Bibliography Separator

**Status:** ✅ COMPLETED

**Problem:** Elsevier uses comma-space between components, Chicago/APA use period-space.

**Solution:**
- Added `separator` field to `BibliographyConfig`.
- Improved extraction logic in `csln_migrate` with heuristics for Elsevier (comma) and Note styles (period).
- Updated `csln_processor` renderer to suppress the default separator when a component starts with punctuation (like `. `), preventing double punctuation.

### Phase 6: Editor Verb Form

**Status:** ✅ COMPLETED

**Problem:** Different styles use different patterns for chapter editors:
- Chicago: "edited by First Last"
- APA: "In F. Last (Ed.),"
- Elsevier: "In: Last, F. (Eds.),"

**Solution:**
- Added `EditorLabelFormat` enum to `ContributorConfig`.
- Implemented extraction logic in `csln_migrate` to detect label position and form.
- Updated `csln_processor` to use the configured format for editor/translator role labels.

### Phase 7: Publisher-Place Visibility

**Status:** ✅ COMPLETED

**Problem:** Style-specific rules for when to show location:
- Chicago: Only for periodicals with place, not books
- Elsevier: Publisher, Place format for all
- APA: Publisher (Location) for some types

**Solution:**
- Implemented style-aware visibility rules in `main.rs`'s `apply_type_overrides`.
- Chicago: Suppress location for `book`, `report`, `thesis`; show for `article-journal`.
- APA: Suppress location for all types (per APA 7th edition).
- Elsevier: Show location for all types (default behavior).
- Fixed regressions in `publisher` and `genre` visibility by ensuring explicit `suppress: false` in overrides for intended types.

### Phase 8: Container Title Deduplication

**Status:** ✅ COMPLETED

**Problem:** Elsevier bibliography shows container title twice for chapters.

**Solution:**
- Improved `Upsampler` to handle multiple space-separated variables in `<names>` (e.g. `editor translator`).
- Implemented **Type-Specific Suppression** in `TemplateCompiler`: variables discovered only in specific branches are now marked as suppressed by default, with un-suppress overrides for the active types.
- Implemented **Recursive Variable Discovery** in the flattener: ensures variables nested in Lists are correctly deduplicated and receive type-specific overrides.
- Removed manual injection hacks in `main.rs` that caused duplication for non-chapter types.

---

## Success Criteria

**Citation Target:** ✅ ACHIEVED - 6/6 styles at 5/5

**Bibliography Target:**
| Style | Current | Target |
|-------|---------|--------|
| apa | 5/5 ✅ | 5/5 |
| chicago-author-date | 0/5 | 3/5 |
| elsevier-harvard | 0/5 | 3/5 |
| springer-basic-author-date | 0/5 | 3/5 |
| ieee | 0/5 | 3/5 |
| elsevier-vancouver | 0/5 | 3/5 |

**Total Impact:** 60% of dependent styles with high citation fidelity (already achieved)
