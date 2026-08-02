---
# csl26-2nv1
title: Honor omitted optional bibliography in citation-only styles
status: completed
type: bug
priority: high
tags:
    - style
    - engine
created_at: 2026-07-31T12:13:32Z
updated_at: 2026-08-02T17:19:48Z
parent: csl26-h7oc
---

## Problem

`Style.bibliography` is already optional, and `chicago-notes-18th` is the
bibliography-free base inherited by the shortened-notes-and-bibliography
variant. The base nevertheless authors the semantic placeholder
`bibliography.template: []`, while engine and bindings entry points assume a
bibliography is present instead of honoring `Style.bibliography == None`.

## Contract

- `chicago-notes-18th` omits the optional `bibliography` key entirely.
- `chicago-shortened-notes-bibliography-core` continues to extend Chicago
  notes and adds its own bibliography mapping.
- Engine, document pipeline, FFI, and CLI render no bibliography content or
  heading when the resolved style has no bibliography.
- No top-level null-disable inheritance behavior or schema change is added.

## Acceptance checklist

- [x] Remove the fake empty bibliography from `chicago-notes-18th` and prove
  the authored and resolved styles both remain bibliography-free.
- [x] Add engine/CLI or document-pipeline regression coverage proving
  bibliography output is absent while citations still render.
- [x] Verify the bibliography-bearing descendant still resolves its authored
  bibliography through the existing parent-before-dependent hierarchy.
- [x] Run script regression tests, the Rust pre-commit gate, and relevant
  exact-parity/oracle checks.

## Summary of Changes

- Removed the fake empty bibliography from Chicago notes; its
  bibliography-bearing descendant still inherits the notes style and adds its
  own bibliography.
- Made engine and bindings bibliography surfaces preserve and honor an absent
  `Style.bibliography`.
- Added embedded-family, document, bindings, materialization, and report-script
  regression coverage.
- Verified 245 script tests, 2,337 Rust tests, and the all-features quality
  gate across 35 styles. All 19 embedded exact-parity floors pass with no
  unclear adjudications; Chicago notes remains 22/72 exact and its
  bibliography-bearing descendant remains 13/473, with unchanged fidelity and
  SQI.
