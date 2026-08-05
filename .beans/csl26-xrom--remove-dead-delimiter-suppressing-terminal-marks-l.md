---
# csl26-xrom
title: Remove dead delimiter_suppressing_terminal_marks locale config
status: todo
type: task
priority: low
tags:
    - engine
    - config
created_at: 2026-08-05T18:01:56Z
updated_at: 2026-08-05T18:04:33Z
---

The locale grammar-options field delimiter-suppressing-terminal-marks (default "?!...") is threaded through processor/setup.rs into Config and read by no renderer. It names the exact concept that commit 05bfcf89 reimplemented via resolve_punctuation_collision, so it is dead config that actively misleads.

Removal surface (~25 files, breaking locale-schema change): 15 embedded locale YAMLs plus 2 overrides, locale/types.rs:621, options/mod.rs:509, processor/setup.rs:59, both generated JSON schemas, and 5 docs including a PUNCTUATION_NORMALIZATION.md spec section.

Wants a docs-first PR per the repo's schema-change rule.
