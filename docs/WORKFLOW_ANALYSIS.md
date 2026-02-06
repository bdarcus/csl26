# Rendering Fidelity Workflow Analysis & Improvements

**Date:** 2026-02-05
**Status:** Critical path for project completion

## Executive Summary

The rendering fidelity workflow is the bottleneck preventing faster progress. While excellent tooling has been built (structured oracle, batch aggregation), these tools are not yet integrated into the regular agent workflow. This analysis identifies concrete improvements that could significantly accelerate agent-driven development.

## Current State Analysis

### Test Data (✅ Adequate, 🔶 Needs Expansion)
- **Current:** 15 reference items in `tests/fixtures/references-expanded.json`
- **Coverage:** 8 reference types (article-journal, book, chapter, report, thesis, conference, webpage, edited-volume)
- **Missing:** article-magazine, article-newspaper, software, dataset, legal citations, multilingual items
- **Status:** ✅ Adequate for current tier 1/2 work, 🔶 Needs expansion before tier 3

### Oracle Scripts (✅ Implemented, ❌ Not Integrated)
| Script | Purpose | Status | Integration |
|--------|---------|--------|-------------|
| `oracle.js` | Basic string comparison | ✅ Working | ✅ Used regularly |
| `oracle-e2e.js` | End-to-end migration test | ✅ Working | ✅ Used regularly |
| `oracle-structured.js` | Component-level diff | ✅ Implemented | ❌ **Not integrated** |
| `oracle-batch-aggregate.js` | Multi-style failure analysis | ✅ Implemented | ❌ **Not integrated** |

**Key Finding:** The structured oracle tools exist but agents are still using the basic string comparison workflow. This wastes tokens on manual failure inspection.

### Rust Test Infrastructure (🔶 Basic, Needs Expansion)
- **Current:** `oracle_comparison.rs` (basic test), `subsequent_author_substitute.rs` (feature test)
- **Coverage:** Minimal - mostly relies on Node.js oracle scripts
- **CI Integration:** Not running oracle comparisons in CI
- **Status:** 🔶 Long-term need, not blocking short-term progress

### Workflow Documentation (❌ Missing)
- No documented workflow for using structured oracle tools
- No regression detection process
- No guidance on when to run batch analysis
- Agents default to basic oracle.js out of habit

## Identified Bottlenecks (Ranked by Impact)

### 1. Manual Failure Inspection ⚠️ **HIGHEST IMPACT**
**Problem:** Agents must manually compare long strings to find differences, wasting tokens.

**Evidence from TIER3_PLAN.md:**
> "Manual failure inspection - Comparing strings by eye is slow and error-prone"
> "No structured diff - Hard to see which *component* is wrong vs just different punctuation"

**Impact:** High token consumption per style debugging session

**Solution:** Integrate `oracle-structured.js` as the default comparison tool

**Benefit:** Significant reduction in token usage for debugging

### 2. No Batch Progress Tracking ⚠️ **HIGH IMPACT**
**Problem:** Can't see if a fix helps 1 style or 10 styles without running manually.

**Evidence from TIER3_PLAN.md:**
> "No batch progress tracking - Can't see if a fix helps 10 styles or just 1"

**Impact:** Changes made without understanding broader impact

**Solution:** Run `oracle-batch-aggregate.js` after each fix to see impact

**Benefit:** Better prioritization, avoid regressions

### 3. Limited Root Cause Visibility ⚠️ **MEDIUM IMPACT**
**Problem:** CSL → YAML migration is a black box - hard to debug why a variable ends up in the wrong place.

**Evidence from TIER3_PLAN.md:**
> "Limited root cause visibility - CSL → YAML migration is a black box"
> Tool 3: Migration Debugger (PENDING)

**Impact:** Significant debugging effort spent tracing migration issues

**Solution:** Implement `--debug-variable VAR` flag in csln_migrate

**Benefit:** Faster root cause identification

### 4. Test Data Expansion Overhead 🔶 **MEDIUM IMPACT**
**Problem:** Adding new test items requires manual work across multiple files.

**Current Process:**
1. Add item to `references-expanded.json`
2. Run oracle scripts manually
3. Update expected outputs if needed
4. No automated validation

**Impact:** Discourages comprehensive test coverage expansion

**Solution:** Test data generator + automated validation workflow

**Benefit:** Enables faster test expansion

### 5. No Regression Detection ⚠️ **MEDIUM IMPACT**
**Problem:** No automated way to detect if a change breaks previously passing styles.

**Impact:** Regressions discovered late, require backtracking

**Solution:** CI integration with baseline tracking

**Benefit:** Catch regressions immediately

## Recommended Improvements (Prioritized)

### Priority 1: Workflow Integration (Quick Wins)

#### 1.1 Make Structured Oracle the Default
**Current:** `node scripts/oracle.js styles/apa.csl`
**New:** `node scripts/oracle-structured.js styles/apa.csl`

**Changes:**
- Rename `oracle.js` → `oracle-simple.js` (backup)
- Rename `oracle-structured.js` → `oracle.js` (new default)
- Update CLAUDE.md test commands
- Add `--simple` flag if old behavior needed

**Impact:** Immediate improvement in debugging efficiency

#### 1.2 Add Batch Analysis to Workflow
**Current:** Run manually when remembered
**New:** Run after every significant change

**Changes:**
- Create `scripts/workflow-test.sh` wrapper:
  ```bash
  #!/bin/bash
  # Test a single style with structured diff
  node scripts/oracle.js "$1"

  # Show impact across top 10 styles
  echo "Running batch analysis..."
  node scripts/oracle-batch-aggregate.js styles/ --top 10 --json > batch-results.json
  ```
- Add to CLAUDE.md autonomous commands whitelist
- Document in workflow guide

**Impact:** Better prioritization, regression detection

#### 1.3 Create Workflow Documentation
**New File:** `docs/RENDERING_WORKFLOW.md`

**Content:**
- Step-by-step guide for fixing rendering issues
- When to use each oracle script
- How to interpret structured diff output
- Batch analysis interpretation guide
- Common failure patterns and fixes

**Impact:** Reduces agent setup overhead, standardizes process

### Priority 2: Migration Debugger

#### 2.1 Implement `--debug-variable` Flag
**Location:** `crates/csln_migrate/src/main.rs`

**Features:**
- Track variable provenance through compilation pipeline
- Show CSL source nodes → intermediate representation → final YAML
- Display deduplication decisions
- Show override propagation

**Example Usage:**
```bash
csln_migrate styles/apa.csl --debug-variable volume

# Output:
Variable: volume
Source CSL nodes:
  1. <text variable="volume"/> in macro "label-volume" (line 142)
  2. <text variable="volume"/> in macro "source-serial" (line 187)

Compiled to:
  Template component at index 4 in bibliography.template
  - rendering.prefix: " "
  - rendering.suffix: None
  - overrides: {article-journal: {suppress: false}}

Deduplication: Node 1 merged into Node 2 (same variable)
Ordering: Placed after container-title by reorder_serial_components()
```

**Impact:** Faster migration debugging

### Priority 3: Test Data Expansion

#### 3.1 Add Missing Reference Types
**Target:** 25 items (up from 15)

**New Items Needed:**
- article-magazine (2 items)
- article-newspaper (1 item)
- software (2 items - increasingly important)
- dataset (2 items - increasingly important)
- legal_case (1 item)
- legislation (1 item)
- webpage with access date (1 item)

**Edge Cases:**
- No author (use title for sorting)
- No date ("n.d." handling)
- Very long title (>200 chars)
- Multilingual data (future: csln#66)

**Impact:** Better coverage, catch more edge cases

#### 3.2 Create Test Data Generator
**Tool:** `scripts/generate-test-item.js`

**Features:**
- Interactive prompt for reference type
- Validates required fields
- Auto-assigns next ITEM-N number
- Runs oracle comparison automatically

**Impact:** Makes test expansion easier

### Priority 4: Regression Detection

#### 4.1 Baseline Tracking System
**Approach:** Store baseline results, compare on each run

**Implementation:**
```bash
# Save baseline
node scripts/oracle-batch-aggregate.js styles/ --top 20 --json > baselines/baseline-2026-02-05.json

# Compare against baseline
node scripts/oracle-batch-aggregate.js styles/ --top 20 --json --compare baselines/baseline-2026-02-05.json
```

**Output:**
```
Regression detected:
  - APA: 15/15 → 14/15 bibliography (ITEM-3 now failing)
  - IEEE: 15/15 → 15/15 citations (no change)

New passing:
  + Nature: 0/15 → 5/15 bibliography

Net impact: -1 passing entries
```

**Impact:** Catch regressions immediately

### Priority 5: Rust Test Integration (Low Priority - Future)

#### 5.1 Move Oracle Logic to Rust
**Rationale:** Long-term, CI should run Rust tests, not Node.js scripts

**Approach:**
- Port oracle comparison logic to Rust
- Use `insta` crate for snapshot testing
- Run in CI on every PR

**Impact:** Better CI integration, faster tests

**Priority:** Low - Node.js scripts work well, this is optimization

## Implementation Phases

### Phase 1: Quick Wins
- [ ] Rename oracle scripts (make structured default)
- [ ] Create `workflow-test.sh` wrapper
- [ ] Write `docs/RENDERING_WORKFLOW.md`
- [ ] Update CLAUDE.md with new commands

**Expected Impact:** Significant reduction in token usage for debugging

### Phase 2: Migration Debugger
- [ ] Implement `--debug-variable` flag in csln_migrate
- [ ] Add provenance tracking infrastructure
- [ ] Test on common failure cases (volume, year, pages)
- [ ] Document usage in workflow guide

**Expected Impact:** Faster root cause identification

### Phase 3: Test & Regression
- [ ] Expand test data to 25 items
- [ ] Create test data generator
- [ ] Implement baseline tracking in batch aggregator
- [ ] Add regression detection to workflow

**Expected Impact:** Better coverage, catch regressions early

### Phase 4: Documentation & Process (Ongoing)
- [ ] Create troubleshooting guide for common failure patterns
- [ ] Document migration debugging workflow
- [ ] Add examples to workflow guide
- [ ] Update README with new workflow

## Agent Performance Benchmarks

**Before Improvements:**
- Token usage per style debugging: High (manual string comparison)
- Regressions discovered: Late (after multiple commits)
- Test data coverage: 15 items, 8 types
- Agent experience: Manual, token-intensive

**After Phase 1 (Quick Wins):**
- Token usage per style debugging: Significantly reduced (structured diffs)
- Batch impact visible immediately
- Workflow documented and standardized

**After Phase 2 (Migration Debugger):**
- Migration debugging tokens: Significantly reduced
- Root cause identification: Fast
- Migration confidence: High

**After Phase 3 (Test & Regression):**
- Test data coverage: 25 items, 15+ types
- Regressions caught: Immediately (same commit)
- Test expansion: Easy (generator tool)

## Open Questions

1. **Should we run batch analysis in CI?**
   - Pro: Catch regressions automatically
   - Con: Adds overhead to CI time
   - **Recommendation:** Add as optional check, run on-demand initially

2. **Should we move to Rust tests now or later?**
   - **Recommendation:** Later - Node.js scripts work well, this is optimization

3. **How often should we update baselines?**
   - **Recommendation:** After each significant milestone (tier completion)

## Related Tasks

- Task #11: Expand test data coverage to 20+ items (covers Phase 3.1)
- Task #14: Fix year positioning for numeric styles (high priority fix)
- Task #12: Fix conference paper template formatting (medium priority)

## Conclusion

The rendering fidelity workflow can be significantly improved with **quick, targeted changes**:

1. **Phase 1:** Make structured oracle the default → More efficient debugging
2. **Phase 2:** Add migration debugger → Fast root cause identification
3. **Phase 3:** Expand tests + regression detection → Catch issues early

The highest ROI improvements are **workflow integration** (Priority 1) which can be implemented with immediate impact.
