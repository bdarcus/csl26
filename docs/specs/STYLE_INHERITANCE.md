# Style Inheritance Specification

**Status:** Draft
**Version:** 1.0
**Date:** 2026-07-28
**Supersedes:** [STYLE_ALIASING.md](./STYLE_ALIASING.md) (mechanism-selection
survey; its preset design survives via
[STYLE_PRESET_ARCHITECTURE.md](./STYLE_PRESET_ARCHITECTURE.md))
**Related:** beans `csl26-s2rw`, `csl26-svfg`, `csl26-8x90`;
[2026-07-28_STYLE_INHERITANCE_PORTFOLIO_AUDIT.md](../architecture/audits/2026-07-28_STYLE_INHERITANCE_PORTFOLIO_AUDIT.md);
[STYLE_PRESET_ARCHITECTURE.md](./STYLE_PRESET_ARCHITECTURE.md),
[STYLE_REGISTRY.md](./STYLE_REGISTRY.md),
[UNIFIED_SCOPED_OPTIONS.md](./UNIFIED_SCOPED_OPTIONS.md),
[STYLE_TAXONOMY.md](./STYLE_TAXONOMY.md),
[STYLE_EDITIONS_AND_FAMILIES.md](./STYLE_EDITIONS_AND_FAMILIES.md)

## Purpose

Style reuse in Citum grew across four spec layers (presets, `extends`
inheritance, scoped-option profiles, registry aliases) that were never
reconciled; the 2026-07-28 audit found spec–spec and spec–code
contradictions. This spec is the single authoritative statement of how a
style inherits, what merge semantics apply, when to alias versus wrap, and
which styles live where in the portfolio.

## Scope

In scope:

- the three-layer resolution model and which spec owns each layer
- normative merge semantics for `extends`, including nested option blocks
- decision rules: registry alias vs config-wrapper vs structural-wrapper vs
  standalone
- hidden base/core style conventions
- portfolio policy: embedded set, in-repo exemplars, community repo
- guarantees per audience (maintainers, agents, style authors)

Out of scope:

- registry file format and lookup order (owned by
  [STYLE_REGISTRY.md](./STYLE_REGISTRY.md))
- the scoped-options surface itself (owned by
  [UNIFIED_SCOPED_OPTIONS.md](./UNIFIED_SCOPED_OPTIONS.md))
- edition evolution and retirement workflow (owned by
  [STYLE_EDITIONS_AND_FAMILIES.md](./STYLE_EDITIONS_AND_FAMILIES.md))
- semantic classification vocabulary (owned by
  [STYLE_TAXONOMY.md](./STYLE_TAXONOMY.md))

## Design

### Resolution model

A style name resolves to rendered output through exactly three layers, in
order. Each layer has one owning spec; no other mechanism may introduce
reuse.

| Layer | Mechanism | Answers | Owning spec |
|---|---|---|---|
| 1. Identity | Registry `aliases:` | "Which style is this name?" | STYLE_REGISTRY.md |
| 2. Structure | `extends:` chain | "Which templates and variants apply?" | This spec |
| 3. Configuration | Scoped options + presets | "With which behavioral settings?" | UNIFIED_SCOPED_OPTIONS.md |

An alias contributes no YAML and no behavior delta — it is a name. A style
document contributes structure and/or configuration. Layer 2 resolution
happens once, at load time, in `citum-schema-style` (`style/overlay.rs`);
the engine never sees an unresolved `extends`.

### Merge semantics (normative)

When a child declares `extends: <parent>`, the child document is overlaid
onto the fully resolved parent:

1. **Nested option and config structs merge field-by-field, recursively.**
   A child that sets `bibliography.options.dates.note-wrap` inherits every
   other field of the parent's `dates` block. This applies to all scoped
   option blocks (`contributors`, `dates`, `titles`, `locators`, …) at
   every scope (global, `citation`, `bibliography`).
2. **Scalars and arrays replace whole.** There is no per-element array
   merge; a child that touches a list owns the whole list.
3. **Explicit `null` clears optional fields.** Writing `key: null` removes
   the inherited value entirely (distinct from omitting the key, which
   inherits). Non-optional fields with schema defaults reject `null` at
   parse time; reset one by authoring its default value explicitly.
   A preset reference (e.g. `dates: numeric`) layers its resolved settings
   like an authored block: fields the preset defines apply — non-optional
   fields are always fully determined by the preset — while optional fields
   the preset leaves unset inherit from the parent.
4. **`type-variants` and `type-templates` merge per-key.** A child variant
   for `book` replaces the parent's `book` variant and leaves the parent's
   other variants intact. Within a replaced variant, no deeper merge
   occurs.
5. **Chains resolve root-first.** Grandparent → parent → child, each step
   applying rules 1–4. Cycles and missing parents are load errors. Base
   styles must not themselves declare `extends`
   (STYLE_PRESET_ARCHITECTURE.md §7).

Rule 1 is the resolution of `csl26-svfg`: STYLE_PRESET_ARCHITECTURE.md §3
already stated structural deep merge, but the implementation replaced
nested option structs whole-value, forcing the three GB/T 7714-2025 leaf
styles to each carry a duplicated `bibliography.options.dates` block
(PR #1068). Under this spec those duplicates collapse into
`gb-t-7714-2025-base`. Until the overlay implementation lands, rule 1 is
aspirational and this spec remains Draft.

### Choosing a form

Decision rules for how a new style enters the portfolio, refining
STYLE_TAXONOMY.md's Profile Rule:

| The style… | Form | Mechanism |
|---|---|---|
| renders identically to an existing style (verified on raw output by a human, not fixture similarity) | **alias** | registry `aliases:` entry, no YAML |
| differs only in configuration (delimiters, et-al, disambiguation, date forms…) | **config-wrapper** | `extends:` + scoped options only; no template-bearing fields |
| differs in templates or type variants for some types | **structural-wrapper** | `extends:` + local templates/variants |
| shares no useful parent | **standalone** | full document; consider extracting a hidden core if siblings emerge |

Aliasing requires declared-variant evidence (style metadata, publisher
documentation) plus a human raw-output comparison. Fixture-bounded
behavioral similarity alone is never sufficient: the 2026-07-17 audit
addendum found 0 of 90 ≥0.98-similar candidate pairs safely
auto-registrable.

### Hidden layers

A family may factor shared structure into hidden styles:

- **Naming:** `<family>-core` for extracted shared structure of one public
  style's siblings (`elsevier-harvard-core`), `<family>-base` for a
  family-wide root (`chicago-18-base`, `gb-t-7714-2025-base`).
- Hidden styles are embedded but **not enumerated** in
  `registry/default.yaml`; they are unreachable from CLI and hub pickers
  and reachable only via `extends:`.
- Hidden styles never declare `extends` and never appear as standalone rows
  in the compatibility report (they participate in chain and family-root
  discovery only).

### Portfolio policy

Three tiers, with movement between them by measured criteria:

| Tier | Location | Contract |
|---|---|---|
| **Embedded** | `crates/citum-schema-style/embedded/styles/` | Product surface. Compiled in, registry-enumerated, tuned toward exact-text parity; regressions gate CI. |
| **Exemplar** | `styles/` (this repo) | Kept because it is a Rust-test fixture, the wrapper exemplar for an embedded parent, or unique-coverage (legal, label, 17th-edition notes, humanities notes, chemistry, superscript numeric). Reported as a secondary tier. |
| **Community** | `citum-styles` repo | Everything else. Resolved at runtime via the registry's filesystem layer; may `extends:` embedded parents. Parity status is advisory, not gating. |

`styles/embedded` remains a symlink into the crate; the crate directory is
authoritative. `styles/experimental/` holds pre-spec explorations and is
exempt from tier rules.

**Promotion to embedded** requires: high dependent-style reach or strategic
coverage, a tuned style meeting the exact-parity bar for its fixtures, and
a registry entry with curated aliases. **Demotion to community** requires
only that the keep-exemplar criteria no longer hold. The initial
disposition of all 141 checked-in styles is recorded in
`scripts/report-data/style-disposition-2026-07-28.tsv`.

### Guarantees by audience

- **Maintainers:** one merge algorithm (rules 1–5) implemented in one place
  (`style/overlay.rs`); the compiled-in `StyleBase` set is closed and CI
  rejects `extends:` to a nonexistent base.
- **LLM agents:** resolution is statically decidable from the YAML corpus —
  layer table above; no runtime conditionals select parents; a style's
  effective configuration is derivable by applying rules 1–5 along its
  chain.
- **Style authors/editors:** to change one behavior of an existing style,
  write a config-wrapper with only the fields that differ; you never need
  to copy a block to preserve its siblings (rule 1); `null` is the explicit
  "remove inherited" operator.

## Implementation Notes

- The deep-merge change (`csl26-svfg`) is implemented in the `extends`
  overlay (`crates/citum-schema-style/src/style/overlay.rs`), not in the
  runtime `Options::merge` impls; the GB/T dates deduplication is the
  verifying case (rendered output must not change). A working spike lives
  on local branch `wip/svfg-deep-merge`; it verified the GB/T case
  byte-identical and surfaced four constraints the final implementation
  must satisfy:
  1. **Raw-document basis.** Field presence must come from the authored raw
     `options` mapping — struct-level merges cannot distinguish authored
     defaults from serde defaults (e.g. `DateConfig.month`). `month` also
     needs `#[serde(default)]` or partial `dates` blocks cannot parse.
     Presence is a property of any serialized document, not of YAML:
     JSON and CBOR inputs carry the same key-set and explicit-null
     information and transcode losslessly into the same generic value
     tree (style documents use string-keyed maps only).
  2. **Uniform raw-preserving ingest (`csl26-j3zy`, fixed).** Every style
     load path now populates the raw tree through one of the
     raw-preserving constructors: `Style::from_document_bytes(bytes, format)`
     for load paths that detect YAML/JSON/CBOR from the source (YAML/JSON
     parse directly; CBOR is decoded the same way, then rejected if any map
     uses a non-string key), or `Style::from_yaml_bytes`/`from_yaml_str` for
     the remote and embedded paths (HTTP, Git, CID, `embedded/styles.rs`)
     that are YAML-only by construction. The store resolver, the
     schema-layer `file://` `extends` fallback, and the CLI's
     `convert style` command previously deserialized typed structs directly
     and so never populated `raw_yaml` in any format — the fix and a
     turbofish-bypass guard
     (`crates/citum-schema-style/tests/raw_ingest_guard.rs`) land together
     with regression coverage in `citum_store`'s resolver tests.
  3. **Post-parse mutation guard.** Styles mutated programmatically after
     parse (tests, server overrides) carry stale `raw_yaml`; the raw path
     must verify the typed overlay still round-trips from its raw options
     and fall back to the typed merge otherwise, because resolution re-runs
     on already-resolved styles (`extends` is preserved).
  4. **Wrapper-compat pass.** Existing wrappers were tuned under
     whole-block replace and may rely on a partial block *suppressing*
     parent fields — `taylor-and-francis-chicago-author-date`'s partial
     `titles:` block drops the parent's `type-mapping` and title-class
     entries today. Each such wrapper needs explicit `~` clears added in
     the same change, verified against the full test suite and the
     compatibility report.
- The community-repo split, report refocus, and registry alias review are
  tracked as separate beans under epic `csl26-s2rw`.
- STYLE_PRESET_ARCHITECTURE.md remains authoritative for `StyleBase`
  compilation, circular-dependency guards, and wizard integration; its §3
  merge summary defers to this spec.

## Acceptance Criteria

- [ ] `extends` deep-merges nested option structs field-by-field (rule 1),
  verified by a test extending a parent's `dates` block with one field.
- [ ] All style load paths populate the raw document tree via one
  constructor (`csl26-j3zy`), and a deep-merge test proves a JSON-authored
  child produces the same resolved style as its YAML equivalent.
- [ ] The three duplicated GB/T `bibliography.options.dates` blocks are
  removed with byte-identical rendered output.
- [ ] Scalars, arrays, and explicit `null` behavior are covered by tests
  matching rules 2–3.
- [ ] STYLE_ALIASING.md is marked Superseded pointing here; the related
  specs cross-reference this spec in their changelogs.
- [ ] The disposition TSV exists and covers all 141 checked-in styles.
- [ ] Status flips to Active in the same commit as the overlay
  implementation.

## Changelog

- v1.0 (2026-07-28): Initial version — resolution model, normative merge
  semantics, form decision rules, hidden-layer conventions, three-tier
  portfolio policy.
