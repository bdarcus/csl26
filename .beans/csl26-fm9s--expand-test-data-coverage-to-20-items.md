---
# csl26-fm9s
title: Expand test data coverage to 20+ items
status: completed
type: task
priority: normal
created_at: 2026-02-07T06:53:29Z
updated_at: 2026-02-08T22:34:47Z
blocking:
    - csl26-e6v4
    - csl26-r6fn
---

Current oracle tests use only 15 reference items. Expand to 25+ items covering more diverse reference types and edge cases.

Phase 1 additions (10 items to reach 25 total):
- 2 article-magazine
- 1 article-newspaper
- 2 software citations
- 2 dataset citations
- 1 legal_case
- 1 legislation
- 1 webpage with access date

Edge cases:
- No author (use title for sorting)
- No date ('n.d.' handling)
- Very long title (>200 characters)
- Multilingual data (future: csln#66)

Effort: 2-3 days (or 4 hours with #137 generator)

Refs: GitHub #138, WORKFLOW_ANALYSIS.md Phase 3