---
# csl26-0h05
title: Type titles.type-mapping keys and categories
status: in-progress
type: bug
priority: high
tags:
    - schema
    - engine
created_at: 2026-08-05T15:00:51Z
updated_at: 2026-08-05T15:35:08Z
---

`titles.type-mapping` currently stores both reference-type keys and title-category values as unchecked strings. Typos silently miss or fall through to default rendering.

Spec: `docs/specs/TYPED_TITLE_MAPPING.md`
Related: PR #1142 (superseded), PR #1143 (style-layering evidence)

## Acceptance Criteria

- [x] Specify typed mapping, normalization, validation, inheritance, and forward-compatibility behavior.
- [x] Parse category values through `TitleCategory` and reject invalid values.
- [x] Normalize underscore keys and warn for unknown future reference types.
- [x] Reject duplicate keys after normalization.
- [x] Make engine and migration logic consume typed categories exhaustively.
- [x] Regenerate schemas and pass the repository pre-commit gate.
