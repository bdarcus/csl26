# Rendering Fidelity Workflow Analysis & Improvements

> **Historical document** — This analysis reflects the pre-strict-oracle state (2026-02-06)
> when oracle scoring used 5-scenario citation sets. Current oracle uses the strict 8-scenario
> set (`tests/fixtures/citations-expanded.json`). Numbers here are not directly comparable to
> current reporting. See `docs/TIER_STATUS.md` for live metrics.

**Date:** 2026-02-06 (Updated)
**Status:** Phase 1 Complete, Phases 2-4 In Progress

## Executive Summary

The rendering fidelity workflow infrastructure is now in place. Phase 1 (Quick Wins) has been completed with structured oracle integration, batch analysis tooling, and workflow documentation. The focus now shifts to regression detection, component-first iteration strategy, and migration debugging capabilities to efficiently reach full fidelity across the style corpus.

## Completed Work (Phase 1) ✅

- **Structured oracle as default**: `oracle.js` provides component-level diffs
- **Batch aggregator**: `oracle-batch-aggregate.js` analyzes multiple styles simultaneously
- **Workflow wrapper**: `workflow-test.sh` combines single-style testing with batch analysis
- **Documentation**: `RENDERING_WORKFLOW.md` provides step-by-step guidance

## Current Fidelity Status

### Metrics (as of 2026-02-05)
- **APA 7th**: 5/5 citations ✅, 5/5 bibliography ✅ (100% match)
- **Academy of Management Review**: 5/5 citations ✅, 0/5 bibliography (requires style-specific work)
- **Batch (50 styles)**: 74% achieve 5/5 citation match

### Fidelity Targets

| Tier | Goal | Status |
|------|------|--------|
| **Tier 1** | 100% citation match for top 10 styles | ✅ **COMPLETE** (74% across 50 styles) |
| **Tier 2** | 8/15+ bibliography for APA, Elsevier Harvard, Chicago | 🔄 **IN PROGRESS** (APA: 5/5) |
| **Tier 3** | 8/15+ bibliography for IEEE, Nature, Elsevier Vancouver | ⏳ PENDING |
| **Tier 4** | 12/15+ bibliography across top 20 styles | ⏳ PENDING |
| **Full Fidelity** | 15/15 citation AND bibliography for top 20 parent styles | 🎯 TARGET |

**Impact**: Top 20 parent styles cover **75%+ of the 7,987 dependent styles** in the CSL repository.

## Component-First Convergence Strategy

**Key Insight**: The most efficient path to full fidelity is fixing common component failures across styles, not debugging styles individually.

### Iteration Loop

1. **Run batch analysis** across top 20 styles using `oracle-batch-aggregate.js`
2. **Identify the most common component failure** (e.g., "year formatting" appears in 15 styles)
3. **Fix that ONE issue** in processor or migration
4. **Re-run batch** and measure improvement
5. **Repeat** with next most common failure

### Why This Works

- Each fix potentially improves **10-20 styles simultaneously**
- Converges faster than style-by-style debugging
- The batch aggregator's `componentSummary` output provides exactly this data
- Prevents duplicate work fixing the same issue in multiple contexts

### When to Move On

- Once a style reaches **12/15+ bibliography matches**, move to next priority style
- Diminishing returns on perfecting one style vs improving many

## Test Data Status

### Current Coverage
- **Items**: 15 reference items
- **Types**: 8 reference types (article-journal, book, chapter, report, thesis, conference, webpage, edited-volume)
- **Status**: ✅ Adequate for Tier 1/2 work

### Expansion Needed (Tier 3+)
- article-magazine (2 items)
- article-newspaper (1 item)
- software (2 items - increasingly important)
- dataset (2 items - increasingly important)
- legal_case (1 item)
- legislation (1 item)
- Edge cases: no author, no date, very long title, multilingual data

**Target**: 25 items covering 15+ reference types

## Known Acceptable Differences

Some differences between citeproc-js and CSLN are intentional or acceptable:

- **HTML entity encoding**: `&#38;` vs `&`
- **Whitespace normalization**: Extra spaces collapsed
- **Unicode vs ASCII**: Em-dash (`—`) vs double-hyphen (`--`) in page ranges
- **Quote normalization**: Smart quotes vs straight quotes (when not specified by style)

Agents should not spend time investigating these documented differences.

## Remaining Phases (Prioritized)

### Phase 2: Regression Detection (HIGH PRIORITY)

**Goal**: Prevent fixes from breaking previously passing styles

**Implementation**:
```bash
# Save baseline
oracle-batch-aggregate.js --top 20 --save baselines/baseline-2026-02-06.json

# Compare against baseline
oracle-batch-aggregate.js --top 20 --compare baselines/baseline-2026-02-06.json
```

**Output Example**:
```
Regression detected:
  - APA: 15/15 → 14/15 bibliography (ITEM-3 now failing)

New passing:
  + Nature: 0/15 → 5/15 bibliography

Net impact: -1 passing entries
```

**Benefits**:
- Catch regressions immediately (same commit)
- No wasted work from unknowingly breaking styles
- Baseline updates track progress over time

**Effort**: Medium (2-3 days to implement `--save` and `--compare` flags)
**Impact**: HIGH - prevents costly backtracking

### Phase 3: Migration Debugger (MEDIUM PRIORITY)

**Goal**: Fast root-cause identification for migration issues

**Implementation**: Add `--debug-variable` flag to `csln_migrate`

**Example Usage**:
```bash
csln_migrate styles-legacy/apa.csl --debug-variable volume

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

**Benefits**:
- Faster debugging of complex migration issues
- Visibility into compilation pipeline
- Useful for understanding how CSL → CSLN works

**Effort**: High (1-2 days to implement provenance tracking)
**Impact**: MEDIUM - valuable for stubborn issues, but not all issues require this

### Phase 4: Test Data Expansion (LOW PRIORITY)

**Goal**: Better coverage for edge cases and additional reference types

**Implementation**:
- Expand `references-expanded.json` to 25 items
- Create `generate-test-item.js` tool for easy test addition
- Add edge cases (no author, no date, long titles, multilingual)

**Benefits**:
- Catch more edge cases
- Better tier 3+ coverage
- Easier test expansion for contributors

**Effort**: Low-Medium (2-3 hours to add items, 4 hours for generator tool)
**Impact**: MEDIUM - Important but not blocking current tier 2/3 work

## Critical Issues Resolved

### ✅ Fixed: Batch Aggregator Script Bug
**Issue**: `oracle-batch-aggregate.js` referenced `oracle-structured.js` which doesn't exist
**Fix**: Updated to reference `oracle.js` (the current structured oracle)
**Impact**: Batch workflow now functional

### ✅ Fixed: Stale Documentation References
**Issue**: Multiple documents referenced `oracle-structured.js` as separate from `oracle.js`
**Fix**: Updated all references to reflect that `oracle.js` IS the structured oracle
**Impact**: Documentation accuracy, reduced confusion

## Bottlenecks Addressed

### 1. Manual Failure Inspection ✅ RESOLVED
**Was**: Agents manually comparing long strings
**Now**: `oracle.js` provides component-level diffs by default
**Impact**: Significant reduction in token usage for debugging

### 2. No Batch Progress Tracking ✅ RESOLVED
**Was**: Unknown if fixes help 1 style or 10 styles
**Now**: `oracle-batch-aggregate.js` shows cross-style impact
**Impact**: Better prioritization, visible progress

### 3. Limited Root Cause Visibility ⏳ PENDING (Phase 3)
**Issue**: CSL → YAML migration is a black box
**Solution**: Migration debugger with `--debug-variable` flag
**Status**: Planned for Phase 3

### 4. No Regression Detection ⏳ PENDING (Phase 2)
**Issue**: Fixes can break other styles unknowingly
**Solution**: Baseline tracking with `--save`/`--compare` flags
**Status**: Next priority

## Success Metrics

### Before Phase 1
- Token usage per style debugging: High (manual string comparison)
- Batch impact visibility: None
- Workflow documentation: Missing
- Agent experience: Manual, token-intensive

### After Phase 1 (Current)
- Token usage per style debugging: Significantly reduced (structured diffs)
- Batch impact visibility: Available via `oracle-batch-aggregate.js`
- Workflow documentation: Complete (`RENDERING_WORKFLOW.md`)
- Agent experience: Guided, efficient

### After Phase 2 (Regression Detection)
- Regression detection: Immediate (same commit)
- Baseline tracking: Automatic
- Confidence in changes: High (no silent breakage)

### After Phase 3 (Migration Debugger)
- Migration debugging time: Fast (provenance tracking)
- Root cause identification: Minutes vs hours
- Migration confidence: High (transparent pipeline)

### After Phase 4 (Test Expansion)
- Test coverage: 25 items, 15+ types
- Edge case coverage: Comprehensive
- Contributor onboarding: Easy (generator tool)

## Agent Performance Benchmarks

**Component-First Iteration** (recommended):
- Fix common component → Improve 10-20 styles/fix
- 5-10 fixes → Reach tier 2/3 targets
- Convergence: Fast

**Style-by-Style** (not recommended):
- Fix one style → Improve 1 style/session
- 20+ sessions → Reach tier 2/3 targets
- Convergence: Slow, potential duplicate work

## Next Steps

1. ✅ **Fix critical batch aggregator bug** (COMPLETE)
2. **Implement regression detection** (`--save`/`--compare` flags)
3. **Run batch analysis** to identify most common component failures
4. **Apply component-first iteration** to reach Tier 2 targets
5. **Implement migration debugger** for stubborn issues
6. **Expand test data** as needed for Tier 3+

## Related Documentation

- **[RENDERING_WORKFLOW.md](./RENDERING_WORKFLOW.md)**: Step-by-step agent workflow guide
- **[../reference/STYLE_PRIORITY.md](./../reference/STYLE_PRIORITY.md)**: Parent style impact ranking

## Conclusion

Phase 1 workflow improvements are complete and functional. The infrastructure now supports efficient iterative development. The next high-priority improvement is regression detection to prevent fixes from breaking other styles. With component-first iteration strategy and baseline tracking, the path to full fidelity is clear and measurable.
