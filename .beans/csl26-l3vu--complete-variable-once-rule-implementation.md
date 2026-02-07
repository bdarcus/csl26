---
# csl26-l3vu
title: Complete variable-once rule implementation
status: todo
type: feature
priority: critical
created_at: 2026-02-07T12:11:49Z
updated_at: 2026-02-07T12:11:49Z
parent: csl26-u1in
---

Ensure variable-once rule is fully implemented across all migration scenarios.

Tasks:
- Prevent duplicate list variables during migration
- Add suppress overrides automatically where needed
- Handle edge cases (contributor+date, title+container-title, etc.)
- Verify no silent duplication in rendered output

Recent work: Commits on duplicate list variable prevention (csl26-6whe fix)

Impact: Critical for bibliography accuracy