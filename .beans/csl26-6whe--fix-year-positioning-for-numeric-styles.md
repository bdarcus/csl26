---
# csl26-6whe
title: Fix year positioning for numeric styles
status: completed
type: bug
priority: critical
created_at: 2026-02-07T06:52:59Z
updated_at: 2026-02-07T07:26:30Z
---

Year appears after contributors in numeric styles, but should appear at end.

Evidence: 10,878 year position issues across 2,844 styles

Current: [1]T. S. Kuhn, 1962. 'The Structure...'
Expected: [1]T. S. Kuhn, 'The Structure...', vol. 2, no. 2, 1962

Fix:
- Detect numeric style class from CSL citation element
- Move date:issued component to end of template
- Preserve year position for author-date styles
- Test against IEEE, Nature, Elsevier Vancouver

Refs: GitHub #127, TIER3_PLAN.md Issue 1.0