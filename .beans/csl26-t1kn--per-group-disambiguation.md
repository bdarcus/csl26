---
# csl26-t1kn
title: Per-group disambiguation
status: complete
type: task
priority: normal
created_at: 2026-02-16T14:06:48Z
updated_at: 2026-02-16T14:07:08Z
blocked_by:
    - csl26-tuk7
    - csl26-s60c
---

Phase 3: Run disambiguation separately per group with suffix restart.

Files:
- crates/csln_processor/src/render/bibliography.rs

Tasks:
1. Modify render_grouped() to run disambiguation per group
2. Ensure suffix sequence restarts per group
3. Add integration tests for grouped bibliographies

Acceptance:
- Legal citations group shows (2020a), books group shows (2020a) independently
- No suffix collisions within a group
- Suffix assignment stable across re-renders

Dependencies: Phases 1 and 2

Refs: docs/architecture/DISAMBIGUATION_MULTILINGUAL_GROUPING.md Phase 3
