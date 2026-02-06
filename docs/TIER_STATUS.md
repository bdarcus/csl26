# TIER Status

## Tier 1: Citations ✅ COMPLETED

All 6 priority styles achieve 15/15 citation match.

## Tier 2: Author-Date Bibliography ✅ ACHIEVED

- **APA**: 14/15 ✅ (target: 8/15)
- **Elsevier Harvard**: 8/15 ✅ (target: 8/15)
- **Chicago Author-Date**: 6/15 ✅ (target: 5/15)

## Tier 3: Numeric Styles (ACTIVE)

### Current Status

| Style | Citations | Bibliography |
|-------|-----------|--------------|
| ieee | 15/15 ✅ | 0/15 |
| nature | 15/15 ❌ | 0/15 |
| elsevier-vancouver | 15/15 ✅ | 0/15 |

### Active Tasks

See task list for current work:
- #14: Fix year positioning for numeric styles
- #15: Support superscript citation numbers
- #16: Fix volume/issue ordering for numeric styles
- #17: Debug Springer citation regression

### Tooling

- ✅ Structured diff oracle (`scripts/oracle.js`)
- ✅ Batch aggregator (`scripts/oracle-batch-aggregate.js`)
- ⏳ Migration debugger (pending - #24)

## Success Metrics

### Citations
- **Target**: 100% match for top 10 parent styles
- **Current**: 9/10 (90%) - Springer regression needs fix

### Bibliography
- **Target**: 8/15+ for numeric styles (IEEE, Nature, Elsevier Vancouver)
- **Current**: 0/15 - Year positioning is blocker

### Corpus Impact
- Top 10 parent styles cover **60%** of dependent styles (4,792/7,987)
- Numeric styles cover **57%** of corpus
