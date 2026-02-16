---
# csl26-n4x6
title: 'Phase 2: Year suffix rendering'
status: completed
type: task
priority: high
created_at: 2026-02-16T12:18:47Z
updated_at: 2026-02-16T12:38:06Z
blocking:
    - csl26-h2b0
---

Implement year suffix rendering in date templates.

**Implementation**: See docs/architecture/DISAMBIGUATION_IMPLEMENTATION_PLAN.md Phase 2

**Tasks**:
1. Add int_to_letter() converter (0→'a', 25→'z', 26→'aa')
2. Update date renderer to check hints.year_suffix
3. Append suffix to year output
4. Update Sorter to include year suffix in sort key

**Acceptance Criteria**:
- Citations with same author+year get a/b/c suffixes
- Suffixes render after year: '(1990a)'
- Bibliography sorting respects suffixes
- Test test_disambiguate_yearsuffixandsort_native passes

**Effort**: 3-4 hours

**@builder candidate**: Yes - clear implementation with test
