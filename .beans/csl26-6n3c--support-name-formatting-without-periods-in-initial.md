---
# csl26-6n3c
title: Support name formatting without periods in initials
status: todo
type: feature
priority: high
created_at: 2026-02-07T06:53:07Z
updated_at: 2026-02-07T06:53:07Z
---

Some styles want initials without periods.

Current: Kuhn, T. S. (with periods)
Expected: Kuhn TS (no periods)

Fix:
- Handle initialize-with='' (no period after initials)
- Handle initialize-with=' ' (space only)
- Distinguish from initialize-with='.'
- Update name rendering in csln_processor
- Test against styles using no-period format

Refs: GitHub #132, TIER3_PLAN.md Issue 2.2