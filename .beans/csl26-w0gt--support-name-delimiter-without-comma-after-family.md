---
# csl26-w0gt
title: Support name delimiter without comma after family
status: todo
type: feature
priority: high
created_at: 2026-02-07T06:53:10Z
updated_at: 2026-02-07T06:53:10Z
---

Some styles use space-only separator between family and initials.

Current: Smith, J, Anderson, M
Expected: Smith J, Anderson M

Fix:
- Extract sort-separator=' ' from CSL name element
- Apply to bibliography contributor config
- Test against styles using space-only separator
- Ensure family-given delimiter is configurable per style

Refs: GitHub #133, TIER3_PLAN.md Issue 2.3