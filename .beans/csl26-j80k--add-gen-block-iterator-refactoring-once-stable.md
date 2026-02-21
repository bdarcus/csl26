---
# csl26-j80k
title: Add gen block iterator refactoring once stable
status: todo
type: feature
priority: low
created_at: 2026-02-21T01:11:20Z
updated_at: 2026-02-21T01:11:20Z
---

Rust `gen {}` blocks (lazy zero-allocation iterators) are experimental as of Rust 1.85
(tracking issue: https://github.com/rust-lang/rust/issues/117078). Once stabilized, refactor
the following hot-path sites to eliminate intermediate Vec allocations:

- `crates/csln_processor/src/processor/rendering.rs` lines 1029-1083:
  `process_template_with_number_internal_with_format` -- the `filter_map(...).collect::<Vec<_>>()`
  pipeline can become a `gen` block that yields `ProcTemplateComponent` items lazily while
  mutating the `rendered_vars` / `substituted_bases` HashSets in place.

- `strip_author_component` and related filter pipelines in `rendering.rs`.

- Group membership iteration in `crates/csln_processor/src/grouping/`.

Rationale and design context: docs/architecture/EDITION_2024_MIGRATION.md
