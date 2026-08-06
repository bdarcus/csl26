---
# csl26-0h05
title: Type titles.type-mapping keys and categories
status: completed
type: bug
priority: high
tags:
    - schema
    - engine
created_at: 2026-08-05T15:00:51Z
updated_at: 2026-08-06T12:45:38Z
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

## Summary of Changes

Landed across four commits on 2026-08-05/06:

- `005f0741` (docs): defined the typed title-mapping spec.
- `b996fb69` (fix(schema)!): replaced `titles.type-mapping`'s unchecked
  string keys/values with typed `TitleCategory` parsing; normalized
  underscore keys with warnings for unknown future reference types;
  rejected duplicate keys after normalization; made engine
  (`render/component.rs`) and migration (`passes/sqi_refinement.rs`) logic
  consume the typed categories exhaustively; regenerated schemas.
- `613ffb94` (refactor(styles)): migrated `ieee.yaml` to the new typed
  category config as the first real-world consumer.
- `81555bba` / `b460bc6f` (docs/fix(schema)!): removed the superseded
  "all" type-selector keyword from the spec and schema now that typed
  category mappings cover its use cases.

All six acceptance criteria verified checked; schema regenerated and
pre-commit gate passed as part of `b996fb69`.
