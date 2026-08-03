# Unified Scoped Options Specification

**Status:** Active
**Date:** 2026-04-22
**Supersedes:** `CONFIG_ONLY_PROFILE_OVERRIDES.md`
**Related:** `STYLE_TAXONOMY.md`, `STYLE_PRESET_ARCHITECTURE.md`, bean `csl26-xt7k`, bean `csl26-nrkn`, bean `csl26-rwgi`

## Purpose

This specification replaces the author-facing `options.profile` contract with
normal typed options that live at the scope they actually affect. Profile
styles remain a registry/taxonomy concept, but profile-specific configuration
does not. Authors configure wrappers and standalone styles with the same
schema surface.

## Scope

In scope:

- removal of `options.profile` from the public schema
- new citation-scoped and bibliography-scoped option fields for inherited and
  standalone styles
- resolver changes required to apply these fields during normal style
  resolution
- migration of embedded profile wrappers and documentation

Out of scope:

- stringly typed variable/parameter systems
- preserving compatibility with `options.profile`
- widening inheritance so profile wrappers can override template-bearing fields

## Design

### 1. Author-Facing Model

The schema no longer exposes a dedicated profile-only namespace.

- profile styles still use `extends:` to select a structural base
- profile styles may still not override template-bearing fields
- style configuration uses normal typed options in the scope they affect

The initial replacement fields are:

- top-level `options.contributors`
- `citation.options.label-mode`
- `citation.options.label-wrap`
- `citation.options.group-delimiter`
- `bibliography.options.label-mode`
- `bibliography.options.label-wrap`
- `bibliography.options.date-position`
- `bibliography.options.title-terminator`
- `bibliography.options.repeated-author-rendering`
- existing `bibliography.options.volume-pages-delimiter`

### 2. Resolution Model

Style resolution keeps the current structural rule for profile wrappers:

- profile wrappers inherit templates intact from their base
- profile wrappers may not override template-bearing fields

After a style is resolved, the engine applies structural scoped options to the
effective specs and retains label mode/wrap as runtime presentation metadata.
The renderer materializes a semantic label slot only after locale and type
variant selection; authored templates are not rewritten. This happens for both
profile wrappers and standalone styles, so the option semantics are uniform.

`citation.options.label-mode` supports `numeric` and `none`. When omitted,
numeric processing implies `numeric`; other processing modes preserve existing
template behavior. `none` suppresses inherited or legacy `citation-number`
components. `citation.options.label-wrap` presents the generated label itself;
`citation.wrap` continues to wrap the citation cluster.

`bibliography.options.label-mode` and `label-wrap` are likewise runtime-owned.
Numeric bibliography labels are inserted as a leading semantic group after
type-variant resolution, preserving label-only separator behavior for entries
whose content is otherwise empty. Existing explicit label components remain
accepted and prevent duplicate insertion.

### 2a. Runtime Scope Cascade Merge Semantics

At render time the engine derives an effective per-scope configuration by
cascading the global `options` block into each scope: global →
`citation.options` and global → `bibliography.options`. Scope values win.

Nested option blocks (`dates`, `titles`, `contributors`, `substitute`,
`multilingual`, `locators`, `links`, `notes`, `sorting`, …) merge
**field-by-field**, mirroring STYLE_INHERITANCE.md rule 1: a scope block
that sets `dates.note-wrap` inherits every other field of the global
`dates` block. Scalars and arrays replace whole.

Field presence comes from the authored document, not from typed structs —
deserialized serde defaults are indistinguishable from authored defaults.
Each parsed style captures its authored `citation.options` /
`bibliography.options` mappings (preset names expanded to their resolved
mappings), and `extends` resolution chain-merges these captures alongside
the typed overlay, so a wrapper's scope-level authorship accumulates
across the chain while global-scope changes still flow into scopes the
chain never re-authored.

The captures are advisory, never load-bearing: before the cascade uses a
capture it verifies the typed scope options still round-trip from it (the
same post-parse mutation guard as the overlay). When no trustworthy
capture exists — programmatic construction, post-parse mutation, or a
non-mapping value — the cascade falls back to the pre-existing typed
whole-block merge. Non-engine consumers of the typed merge (lint,
citum-migrate SQI refinement) intentionally stay on the typed path.

The verifying case is `gb-t-7714-2025-base`: its bibliography scope
authors only `dates.note-wrap`, and the previously duplicated copy of the
global `dates` block is removed with byte-identical rendered output
across the embedded and in-repo corpus (bean `csl26-yz4w`).

### 3. Schema Rules

`options.profile` is removed completely.

- parsing a style with `options.profile` is a hard error
- the error message must point authors to the new scoped fields
- capability-gated profile-axis validation is removed

## Implementation Notes

The new fields remain strongly typed Rust schema items. This preserves the
current code-as-schema model while removing the separate profile vocabulary.

## Acceptance Criteria

- [ ] Styles using the new scoped fields parse and resolve.
- [ ] Styles using `options.profile` fail with a migration-oriented error.
- [ ] Embedded profile wrappers use only the new scoped fields.
- [ ] Standalone styles can use the same fields without `extends:`.
- [ ] Numeric labels are materialized at render time without mutating authored templates.

## Changelog

- 2026-07-30: §2a — the runtime scope cascade merges nested option blocks
  field-by-field from chain-merged authored scope captures, with a typed
  whole-block fallback (bean `csl26-yz4w`).
- 2026-08-03: Numeric citation and bibliography labels became declarative
  runtime presentation settings; template mutation was removed.
- 2026-04-22: Activated alongside the schema and embedded-style migration.
- 2026-04-22: Initial version.
