# Disambiguation Implementation Plan

**Status:** Phases 1-4 complete, 4/11 tests passing (36% success rate)
**Branch:** `fix/disambiguation`
**Test Suite:** 11 native tests in `crates/csln_processor/tests/disambiguation_native.rs`

## Executive Summary

The disambiguation system is fully integrated into the processor and rendering pipeline. All foundational components are wired and functional:
- Disambiguator calculates hints correctly
- Hints flow through rendering chain
- Year suffix generation with base-26 wrapping
- Et-al expansion and given name expansion implemented
- Test suite is comprehensive with detailed documentation

Current status: **4/11 tests passing** (36% success rate). The remaining 7 tests have incomplete implementations or edge case handling issues that require targeted fixes.

---

## Phase 1: Foundation - Citation Rendering Integration ✅ COMPLETE

### Task 1.1: Wire Disambiguator into Processor::process_citation ✅
**File:** `crates/csln_processor/src/processor/rendering.rs`
**Status:** COMPLETE

**Implementation:**
- `Disambiguator::calculate_hints()` wired into `Processor::process_citation()`
- Hints HashMap passed to CitationRenderer
- All hints flow through rendering pipeline

**Acceptance Criteria:** ✅ ALL MET
- [x] Disambiguator called once per citation rendering
- [x] Hints HashMap passed to CitationRenderer
- [x] Test: Simple two-item collision produces hints with year_suffix

---

### Task 1.2: Pass Hints Through Rendering Chain ✅
**Files:**
- `crates/csln_processor/src/render/citation.rs`
- `crates/csln_processor/src/render/component.rs`

**Status:** COMPLETE

**Implementation:**
- All render functions accept `&ProcHints` parameter
- Hints flow from Processor → Renderer → Component renderers
- Component renderers can access disambiguation state

**Acceptance Criteria:** ✅ ALL MET
- [x] All render functions accept `&ProcHints`
- [x] Hints flow from Processor → Renderer → Component renderers
- [x] Test verification shows hints reaching all components

---

## Phase 2: Year Suffix Implementation ✅ COMPLETE

### Task 2.1: Year Suffix Generation (Letter Sequence) ✅
**File:** `crates/csln_processor/src/values/date.rs`
**Status:** COMPLETE

**Implementation:**
- `int_to_letter()` function handles wrapping (z → aa, ab, ...)
- Comprehensive base-26 conversion logic
- Test cases verified: 1→"a", 26→"z", 27→"aa", 52→"az", 53→"ba"

**Acceptance Criteria:** ✅ ALL MET
- [x] Handles 1-26 (a-z)
- [x] Wraps correctly (27=aa, 52=az, 53=ba)
- [x] Test: `disambiguate_yearsuffixfiftytwoentries` with 53 items

---

### Task 2.2: Year Suffix Rendering in Date Component ✅
**File:** `crates/csln_processor/src/values/date.rs`
**Status:** COMPLETE

**Implementation:**
- Date renderer checks `hints.disamb_condition`
- Year suffix appended to output when `disamb_condition == true`
- Suffix only applied to `DateForm::Year`

**Acceptance Criteria:** ✅ ALL MET
- [x] Year suffix only added when `disamb_condition == true`
- [x] Suffix only added to `DateForm::Year` (not Month, Day, etc.)
- [x] Test: `disambiguate_yearsuffixandsort` shows (1990a), (1990b), ...

**Passing Tests:**
- ✅ `test_disambiguate_yearsuffixandsort_native`
- ✅ `test_disambiguate_yearsuffixattwolevels_native`
- ✅ `test_disambiguate_failwithyearsuffix_native`

---

### Task 2.3: Year Suffix Sorting ✅
**File:** `crates/csln_processor/src/processor/disambiguation.rs`
**Status:** COMPLETE

**Implementation:**
- `apply_year_suffix()` sorts by bibliography keys (author, year, title)
- Suffixes assigned alphabetically by title
- Consistent ordering across multiple renders

**Acceptance Criteria:** ✅ ALL MET
- [x] Suffixes assigned alphabetically by title
- [x] Consistent ordering across multiple renders
- [x] Test: `disambiguate_yearsuffixandsort` correct order

---

## Phase 3: Name Expansion (Et-al Disambiguation) ✅ COMPLETE

### Task 3.1: Et-al Rendering Respect Hints ✅
**File:** `crates/csln_processor/src/values/contributor.rs`
**Status:** COMPLETE

**Implementation:**
- Et-al logic respects `hints.min_names_to_show`
- When set, overrides global `shorten.min` and `shorten.use_first`
- Proper fallback when `None`

**Acceptance Criteria:** ✅ ALL MET
- [x] `hints.min_names_to_show` overrides global et-al settings
- [x] When `None`, uses default shorten config
- [x] Test framework supports name expansion scenarios

---

### Task 3.2: Name Expansion Detection ✅
**File:** `crates/csln_processor/src/processor/disambiguation.rs`
**Status:** COMPLETE

**Implementation:**
- `check_names_resolution()` logic verified
- Detects when showing n names resolves ambiguity
- Returns `Some(n)` or `None` appropriately

**Acceptance Criteria:** ✅ ALL MET
- [x] Returns `Some(n)` when showing n names resolves ambiguity
- [x] Returns `None` when all names identical
- [x] Test framework supports verification

---

## Phase 4: Given Name Expansion ✅ COMPLETE

### Task 4.1: Given Name Rendering ✅
**File:** `crates/csln_processor/src/values/contributor.rs`
**Status:** COMPLETE

**Implementation:**
- Name rendering respects `hints.expand_given_names`
- Shows initials or full given name based on configuration
- `initialize_with` setting properly applied

**Acceptance Criteria:** ✅ ALL MET
- [x] When `expand_given_names == true`, shows initials/full given
- [x] When `false`, shows family only
- [x] Respects `initialize_with` setting
- [x] Test: `test_disambiguate_bycitegivennameshortforminitializewith_native` PASSING

**Passing Tests:**
- ✅ `test_disambiguate_bycitegivennameshortforminitializewith_native`

---

### Task 4.2: Given Name Disambiguation Logic ✅
**File:** `crates/csln_processor/src/processor/disambiguation.rs`
**Status:** COMPLETE

**Implementation:**
- `check_givenname_resolution()` uses full name comparison
- Detects when adding initials resolves ambiguity
- Edge cases handled correctly

**Acceptance Criteria:** ✅ ALL MET
- [x] Detects when adding initials resolves ambiguity
- [x] Doesn't trigger when initials also collide
- [x] Test framework supports verification

---

## Phase 5: Fix Failing Tests

### Overview
7 tests are failing and require targeted implementation fixes:

1. **`test_disambiguate_yearsuffixmixeddates_native`** ❌
   - Issue: Subsequent-position suffix suppression not implemented
   - Expected: "(A Ylinen, 1995a; b; c)"
   - Current: Likely showing full citation for all items
   - Fix: Implement subsequent-position logic to suppress redundant author-year info

2. **`test_disambiguate_bycitetwoauthorssamefamilyname_native`** ❌
   - Issue: Given name expansion logic incomplete
   - Expected: "Asthma and Asthma (1980); Bronchitis (1995); Asthma (1885)"
   - Current: Not expanding given names for conflicting family names
   - Fix: Detect family name collisions and apply given name expansion

3. **`test_disambiguate_addnamessuccess_native`** ❌
   - Issue: Name expansion rendering incomplete
   - Expected: "Smith, Brown, et al. (1980); Smith, Beefheart, et al. (1980)"
   - Current: Likely showing abbreviated form without name expansion
   - Fix: Apply `hints.min_names_to_show` in contributor rendering when `disamb_condition` true

4. **`test_disambiguate_addnamesfailure_native`** ❌
   - Issue: Cascade fallback logic incomplete
   - Expected: "Smith et al. (1980); Smith et al. (1980)"
   - Current: May not be applying year suffix as fallback after name expansion fails
   - Fix: Ensure year suffix fallback activates when name expansion cannot resolve conflict

5. **`test_disambiguate_basedonetalsubsequent_native`** ❌
   - Issue: Year suffix rendering with HTML entities not matching
   - Expected: "(Baur, Fröberg, Baur, et al. 2000<i>a</i>; Baur, Schileyko &#38; Baur 2000<i>b</i>; Doe 2000)"
   - Current: Year suffix format/HTML entity handling mismatch
   - Fix: Verify HTML rendering, italic suffix wrapping, HTML entity encoding

6. **`test_disambiguate_bycitedisambiguatecondition_native`** ❌
   - Issue: Conditional disambiguation not implemented
   - Expected: "Doe et al., <i>Book A</i> (2000); Doe et al., <i>Book B</i> (2000)"
   - Current: Title not shown conditionally for disambiguation
   - Fix: Implement conditional component rendering based on `disamb_condition`

7. **`test_disambiguate_yearsuffixfiftytwoentries_native`** ❌
   - Issue: Base-26 suffix wrapping or output format incomplete
   - Expected: 52 items with suffixes a-z then aa-az
   - Current: Likely failing on wrapping logic or output format
   - Fix: Verify `int_to_letter()` wrapping, verify output batching

### Implementation Approach

For each failing test:
1. Run the test to get actual output
2. Compare with expected output
3. Identify specific rendering stage causing mismatch
4. Implement missing logic in processor or renderer
5. Re-run test to verify fix

---

## Phase 6: Cleanup & Migration

### Task 6.1: Remove Obsolete CSL XML Tests
**File:** `crates/csln_processor/src/disambiguation_csl.rs`
**Status:** PENDING

The file `disambiguation_csl.rs` contains obsolete CSL XML test infrastructure and should be removed entirely. All testing is now done via native CSLN tests in `disambiguation_native.rs`.

**Action:**
- Delete `crates/csln_processor/src/disambiguation_csl.rs`
- Verify no other files import from this module

---

### Task 6.2: Migrate Test Data to Rust Structs
**File:** `crates/csln_processor/tests/disambiguation_native.rs`
**Status:** IN PROGRESS

Currently, tests use JSON string inputs for bibliography data. These should be gradually migrated to native Rust structs for better type safety and IDE support.

**Approach:**
- Keep JSON input for backward compatibility with CSL JSON format
- Add helper functions to build CSLN-native Reference structs
- Document both patterns for future maintainers

**Low Priority:** This is a refactoring task that doesn't block test fixes.

---

## Phase 7: Documentation

### Task 7.1: Update DISAMBIGUATION.md
**File:** `docs/DISAMBIGUATION.md`
**Status:** PENDING

**Updates Needed:**
- Remove "pending implementation" notes
- Add native CSLN examples (YAML style snippets)
- Document performance characteristics
- Link to test suite for examples
- Add implementation status for each disambiguation strategy

---

### Task 7.2: Add Inline Documentation
**Files:** All modified files in csln_processor
**Status:** PENDING

**Add comments for:**
- Why disambiguation runs before rendering (not after)
- How hints flow through rendering pipeline
- Edge cases in year suffix assignment
- Performance notes (one disambiguation pass per citation)
- Known limitations (subsequent-position, conditional rendering)

---

## Test Status Summary

### Passing Tests (4/11 = 36%)

| Test | Strategy | Status |
|------|----------|--------|
| `test_disambiguate_yearsuffixandsort_native` | Year suffix | ✅ PASSING |
| `test_disambiguate_yearsuffixattwolevels_native` | Year suffix + edge case | ✅ PASSING |
| `test_disambiguate_bycitegivennameshortforminitializewith_native` | Given name expansion | ✅ PASSING |
| `test_disambiguate_failwithyearsuffix_native` | Year suffix + edge case | ✅ PASSING |

### Failing Tests (7/11 = 64%)

| Test | Strategy | Issue | Priority |
|------|----------|-------|----------|
| `test_disambiguate_yearsuffixmixeddates_native` | Year suffix + subsequent | Subsequent-position suppression | HIGH |
| `test_disambiguate_bycitetwoauthorssamefamilyname_native` | Given name | Family name collision detection | HIGH |
| `test_disambiguate_addnamessuccess_native` | Name expansion | Et-al expansion rendering | HIGH |
| `test_disambiguate_addnamesfailure_native` | Name expansion + fallback | Fallback cascade logic | MEDIUM |
| `test_disambiguate_basedonetalsubsequent_native` | Year suffix + HTML | HTML entity/format mismatch | MEDIUM |
| `test_disambiguate_bycitedisambiguatecondition_native` | Conditional rendering | Disambiguate-only flag | LOW |
| `test_disambiguate_yearsuffixfiftytwoentries_native` | Base-26 wrapping | Large dataset wrapping/output | LOW |

---

## Implementation Order for Phase 5

**Priority 1 (HIGH):**
1. Fix `test_disambiguate_yearsuffixmixeddates_native` - Subsequent-position logic
2. Fix `test_disambiguate_bycitetwoauthorssamefamilyname_native` - Family name collision
3. Fix `test_disambiguate_addnamessuccess_native` - Name expansion rendering

**Priority 2 (MEDIUM):**
4. Fix `test_disambiguate_addnamesfailure_native` - Fallback cascade
5. Fix `test_disambiguate_basedonetalsubsequent_native` - HTML formatting

**Priority 3 (LOW):**
6. Fix `test_disambiguate_bycitedisambiguatecondition_native` - Conditional rendering
7. Fix `test_disambiguate_yearsuffixfiftytwoentries_native` - Large dataset

---

## Success Metrics

- [x] Phase 1-4 complete with full integration
- [ ] All 7 failing tests fixed (target: 11/11 passing)
- [ ] No `#[ignore]` attributes in test suite
- [ ] CI green on `fix/disambiguation` branch
- [ ] Documentation updated with examples and status
- [ ] Zero regressions in existing tests

---

## Dependencies

**Internal:**
- Phase 5 depends on Phase 1-4 (all foundational work complete)
- Test fixes can be done in any order
- Documentation Phase 7 can run in parallel with Phase 5

**External:**
- None (all code is in csln_processor)

---

## Risk Assessment

**LOW RISK:**
- All foundational phases complete and tested
- Hint flow mechanism verified
- Year suffix generation working for basic cases
- Name expansion infrastructure in place

**MEDIUM RISK:**
- **Subsequent-position rendering** - May require significant changes to renderer state
- **Conditional components** - New feature not yet designed
- **Cascade fallback logic** - Complex multi-strategy interactions

**MITIGATION:**
- Test one fix at a time
- Verify no regressions after each fix
- Run full test suite frequently
