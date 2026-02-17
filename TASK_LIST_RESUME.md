# Portfolio: Resuming csl26-group Implementation

## Context
- **Bean:** `csl26-group` (Implement configurable bibliography grouping)
- **Phase:** 7 (Documentation & Group-Aware Disambiguation)
- **Active Task:** `csl26-s60c` (Group-aware disambiguation sorting)

## Task List

- [x] **Infrastructure: Update `Disambiguator`**
    - [x] Add `group_sort` field to `Disambiguator` struct in `disambiguation.rs`
    - [x] Update `new()` and add `with_group_sort()` constructors
- [x] **Logic: Enhance `apply_year_suffix`**
    - [x] Integrate `GroupSorter` from `csln_processor::grouping::sorting`
    - [x] Update sorting logic to use `group_sort` specification if available
    - [x] Ensure title-based fallback remains functional
- [x] **Validation: Unit Tests**
    - [x] Add test for legal citation sorting (case name priority)
    - [x] Add test for type-order grouping (sorting by reference type)
    - [x] Verify year suffix reflects the new sort order
- [x] **Integration: Bibliography Renderer**
    - [x] Update `render_grouped()` in `bibliography.rs` to pass `GroupSort` to `Disambiguator`
- [x] **Documentation**
    - [x] Update `csl26-group` and `csl26-s60c` beans with progress
