---
# csl26-9zn6
title: Define style template expressiveness and parity coverage workflow
status: completed
type: task
priority: high
tags:
    - docs
    - architecture
    - qa
created_at: 2026-08-09T14:42:00Z
updated_at: 2026-08-09T14:46:04Z
---

Produce the docs-only maintainer adjudication for style-template
expressiveness and parity coverage. The deliverable is a Draft specification,
not implementation.

Scope:

- decide whether current evidence justifies macros or general conditionals;
- specify fallback-template diff semantics without carrying pilot code;
- define coverage state, provenance, denominator, and QA contracts;
- separate observed pilot facts from inference and preserve unresolved
  questions;
- split implementation into the existing fallback bean and a dedicated
  auditable-packet bean.

Explicitly excluded: generated pilot packets, individual model review records,
Rust or JavaScript changes, style changes, and workflow control-surface edits.

- [x] Draft and index the specification.
- [x] Adjudicate the pilot against row and repository evidence.
- [x] Rescope csl26-4xg9 to the accepted fallback design.
- [x] Create the auditable and reproducible packet follow-up bean.
- [x] Keep implementation and workflow changes out of this task.

## Summary of Changes

Published the Draft specification as a docs-only adjudication. It retains
bounded template reuse, narrows fallback patching to the existing `template`
field, separates render disposition from comparability, and requires an audit
manifest before pilot metrics can become baselines. Implementation remains in
`csl26-4xg9` and `csl26-02yg`.
