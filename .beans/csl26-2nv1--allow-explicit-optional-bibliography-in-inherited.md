---
# csl26-2nv1
title: Allow explicit optional bibliography in inherited styles
status: todo
type: feature
priority: high
tags:
    - schema
    - engine
    - inheritance
    - style
created_at: 2026-07-31T12:13:32Z
updated_at: 2026-07-31T12:13:32Z
parent: csl26-h7oc
---

## Problem

`Style.bibliography` is already optional for standalone styles, but an extending child cannot suppress an inherited bibliography. `bibliography: null` currently leaves the parent bibliography in place, so `chicago-notes-18th` uses the semantic placeholder `bibliography.template: []`. The engine and CLI also process bibliography output even when a style has no bibliography spec.

## Contract

- `bibliography: null` is an explicit disable in an extending style; an absent key continues to inherit and a mapping continues to merge.
- Resolved `Style.bibliography` is `None` after that explicit disable.
- Engine, document pipeline, FFI, and CLI render no bibliography content or heading when the resolved style has no bibliography.
- Replace Chicago notes’ empty template placeholder with the explicit disable.

## Acceptance checklist

- [ ] Add schema/overlay tests covering inherit, merge, and explicit-null disable semantics.
- [ ] Add engine/CLI or document-pipeline regression coverage proving bibliography output is absent while citations still render.
- [ ] Update generated schema/reference documentation if the public schema output changes.
- [ ] Convert `chicago-notes-18th` from `bibliography.template: []` to `bibliography: null` and validate its rendered document behavior.
- [ ] Run the Rust pre-commit gate and relevant exact-parity/oracle checks.
