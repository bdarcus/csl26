---
id: 1
subject: Update post-processing passes for new bibliography architecture
status: pending
blocks: []
blocked_by: []
priority: high
impact: medium
phase: 2
parent_task: 30
depends_on_pr: 117
created: 2026-02-06T10:30:00Z
content_hash: a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2w3x4y5z6
---

Follow-up to PR #117: Post-processing passes in main.rs were written for the old architecture and are creating duplicate components or conflicts with the new occurrence-based compilation.

## Problem Analysis

- Post-processing in main.rs still expects old architecture
- Creating extra volume/issue entries (duplicates)
- May be interfering with correct component extraction

## Expected Outcome

- No duplicate component entries
- Bibliography extraction matches template_compiler output
- Simpler main.rs without obsolete processing

## Implementation Notes

1. Review current post-processing in main.rs
2. Identify which passes are now handled by template_compiler
3. Remove redundant logic
4. Test with multiple bibliography styles to ensure correctness
5. Update tests if needed

## Related Issues

- Blocks: #2, #3
- Depends on: PR #117
