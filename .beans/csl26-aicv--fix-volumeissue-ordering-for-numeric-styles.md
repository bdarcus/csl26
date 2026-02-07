---
# csl26-aicv
title: Fix volume/issue ordering for numeric styles
status: todo
type: bug
priority: high
created_at: 2026-02-07T06:44:03Z
updated_at: 2026-02-07T07:40:14Z
blocking:
    - csl26-l2hg
---

Numeric styles like Vancouver show volume incorrectly.

Current: volume(issue)
Expected: volume(issue) or volume: issue depending on style

Refs: GitHub #129