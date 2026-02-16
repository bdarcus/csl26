---
# csl26-29ti
title: 'Phase 3: Name expansion (et-al disambiguation)'
status: completed
type: task
priority: high
created_at: 2026-02-16T12:18:51Z
updated_at: 2026-02-16T12:44:23Z
blocking:
    - csl26-h2b0
---

Implement et-al expansion for disambiguation.

**Implementation**: See docs/architecture/DISAMBIGUATION_IMPLEMENTATION_PLAN.md Phase 3

**Tasks**:
1. Update contributor renderer to check hints.min_names_to_show
2. Override et-al settings when disambiguation requires more names
3. Test with references that need name expansion to disambiguate

**Acceptance Criteria**:
- Et-al expands when needed for disambiguation
- Normal et-al rules apply when no conflict
- Test validates correct name count in output

**Effort**: 2-3 hours

**@builder candidate**: Yes - builds on Phase 1 infrastructure
