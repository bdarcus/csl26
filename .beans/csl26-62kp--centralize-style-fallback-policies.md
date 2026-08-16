---
# csl26-62kp
title: Centralize style fallback policies
status: in-progress
type: feature
priority: high
tags:
    - schema
    - engine
    - migrate
    - styles
created_at: 2026-08-16T12:23:45Z
updated_at: 2026-08-16T12:23:45Z
---

Centralize missing-author and missing-date policy in style options so rendering templates remain presentational.

Specification: docs/specs/DATE_FALLBACK.md

## Acceptance Criteria

- [ ] Draft and merge the date-fallback and substitution contract.
- [ ] Remove author/date fallback behavior from template components.
- [ ] Implement processing-derived author substitution and explicit date fallback.
- [ ] Update migration and all tracked Citum styles.
- [ ] Update the style author guide and generated schema.
- [ ] Pass schema, production-style, quality, and Rust gates.
- [ ] Submit the two-PR stack and verify CI.
