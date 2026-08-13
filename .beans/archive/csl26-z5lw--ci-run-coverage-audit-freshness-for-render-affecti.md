---
# csl26-z5lw
title: 'CI: run coverage audit freshness for render-affecting changes'
status: completed
type: bug
priority: high
tags:
    - ci
    - coverage-audit
created_at: 2026-08-13T14:49:29Z
updated_at: 2026-08-13T15:16:33Z
parent: csl26-hk3u
---

Investigate and fix the CI path filter so registered coverage-audit freshness runs before merge for render-affecting changes.

- [x] Widen the workflow trigger to all render-affecting inputs
- [x] Add an explanatory workflow comment
- [x] Validate representative changed-path cases
- [x] Run workflow and freshness checks
- [x] Record the implementation summary

## Summary of Changes

Widened the CI coverage-audit trigger to all render-affecting source and input paths and documented why any Rust crate can affect rendered audit output. YAML parsing, representative path cases, checker regression tests, and clean-main packet regeneration passed; the committed main packet required no update.

The direct checker invocation in this sandbox reports Node child-process EPERM for Git probes, so the clean-main regeneration was validated with an equivalent piped-stdio shim.
