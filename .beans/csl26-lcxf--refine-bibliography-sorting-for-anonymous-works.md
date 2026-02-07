---
# csl26-lcxf
title: Refine bibliography sorting for anonymous works
status: todo
type: bug
priority: normal
created_at: 2026-02-07T06:53:16Z
updated_at: 2026-02-07T06:53:16Z
---

Chicago Author-Date shows entries out of order for anonymous works.

Issues:
- Anonymous work sorting not respecting 'The' article stripping
- Year fallback not working correctly for same-author entries

Fix:
- Review anonymous work sorting logic
- Ensure article stripping ('The', 'A', 'An') works for all styles
- Verify year-based secondary sort for same-name entries
- Test against Chicago Author-Date

Target: Chicago bibliography improves from 4/15 to 5/15+

Refs: GitHub #134, TIER2_PLAN.md Phase 5