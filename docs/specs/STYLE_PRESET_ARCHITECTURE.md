# Style Base Architecture

**Status:** Active
**Version:** 2.0
**Date:** 2026-04-20
**Bean:** `csl26-v961`
**Related:** bean `csl26-zy07` (unblocked by this spec), bean `csl26-wp6y` (follow-up policy work), `LOCALE_MESSAGES.md`, `STYLE_TAXONOMY.md`, `STYLE_EDITIONS_AND_FAMILIES.md`

## Purpose

Citum styles have two levels of configuration reuse:

**Level 1 — Options presets (existing).** Named bundles of formatting
configuration (`ContributorPreset`, `DatePreset`, `TitlePreset`, etc.) applied
at the `options` key of a style file. Already implemented in
`crates/citum-schema-style/src/presets.rs`.

**Level 2 — Style base inheritance (new).** A named, compiled-in `Style` struct
representing a complete well-known style. A YAML file (or wizard output) can
reference a style base by name using the `extends:` key and express only the delta
between itself and the base, rather than duplicating the full style definition.

The motivating case is **behavioral dependents** — styles that differ from a
parent by 2–5 rules (e.g. Turabian ≈ Chicago Notes + no ibid + footnote
punctuation tweaks). In CSL these are full standalone files. In Citum they
become an `extends:` declaration plus a few top-level field overrides.

For config-only profile wrappers, those deltas now live under the normal typed
options surface. Wrapper styles use the same `options.*`, `citation.options.*`,
and `bibliography.options.*` blocks that standalone styles use.

---

## §1 Problem Statement

The CSL corpus mixes two kinds of dependent styles:

1. **Locale/title-only dependents** — same formatting, different display name or
   default locale. Already handled by the Citum alias system (no YAML duplication).

2. **Behavioral dependents** — small formatting deviations from a parent. Today
   these become full standalone YAML files in `styles/`, creating maintenance
   burden and obscuring the relationship to the parent.

Additionally, the top well-known styles (`apa-7th`, `chicago-notes`, etc.) are
candidates for compiled-in embedding so that the wizard and CLI can reference
them without loading a YAML file from disk.

---

## §2 `StyleBase` Enum

A `StyleBase` identifies a compiled-in `Style` struct by a stable key.

```yaml
# Minimal base-extended style — inherits everything from the base
extends: chicago-notes-18th
```

```yaml
# Behavioral variant — override specific fields at the top level
extends: chicago-notes-18th
citation:
  ibid: ~       # null disables ibid — Turabian 9th ed.
options:
  bibliography:
    entry-suffix: "."
```

```yaml
# Locale variant — override locale rendering options at the top level
extends: chicago-author-date-18th
options:
  locale-override: de-DE-chicago
  default-locale: de-DE
```

The authoring rule is uniform: declare `extends:`, then override any fields you
need at the top level of the style file. There is no separate `variant` layer —
top-level fields are the only override mechanism, keeping the mental model
simple for style authors.

### Naming convention

Style base keys use `kebab-case` composed of `{short-name}-{edition}`:

| Base key | `info.short_name` | `info.edition` |
|----------|-------------------|----------------|
| `chicago-notes-18th` | Chicago Notes | 18th |
| `chicago-author-date-18th` | Chicago Author-Date | 18th |
| `apa-7th` | APA | 7th |

The key is the canonical identity. `info.short_name` and `info.edition`
(csl26-zy07) are the human-readable metadata surface for UIs.

### Initial set

| Base key | Location |
|----------|----------|
| `chicago-notes-18th` | `styles/embedded/chicago-notes-18th.yaml` |
| `chicago-author-date-18th` | `styles/embedded/chicago-author-date-18th.yaml` |
| `apa-7th` | `styles/embedded/apa-7th.yaml` |

Additional bases are added as top-priority styles in the style portfolio are
embedded. The `StyleBase` enum in `crates/citum-schema-style/src/style_base.rs`
lists all currently available bases.

---

## §3 `Style.extends` Field

The `extends` field on `Style` holds an optional `StyleBase` value — a plain
enum that serializes to/from a kebab-case string:

```rust
pub struct Style {
    // … existing fields …
    pub extends: Option<StyleBase>,
}
```

When `extends` is present, the engine produces the final `Style` during
processor construction by:

1. Loading the base style.
2. Merging any explicit top-level fields from the local file on top (with local
   fields taking ultimate precedence).

This two-step resolution makes the engine the canonical runtime resolution
boundary for all callers (CLI, tests, FFI, and embedded integrations).

**Merge semantics:** object fields merge structurally and recursively. Scalars,
arrays, and explicit `null` values replace inherited values. Arrays replace
wholesale — there is no per-element merging. The normative statement of these
rules, including field-level deep merge of nested option blocks, now lives in
[STYLE_INHERITANCE.md](./STYLE_INHERITANCE.md); this section defers to it.

This preserves a simple and predictable contract, but it also means some
evidence-backed child styles remain bulky until more granular override
mechanisms exist for bibliography/template structures.

---

## §4 Registry Location

The `StyleBase` enum lives in `crates/citum-schema-style` as compiled-in
data, while the engine owns runtime resolution. Rationale:

- Keeping the enum in the schema crate preserves the declarative contract:
  a `Style` is a `Style` regardless of how it was produced.
- Letting the engine resolve base-extended styles guarantees that every runtime
  entry point observes the same effective style, even when callers load raw
  YAML or embedded styles without pre-resolving them.
- The wizard and CLI both depend on `citum-schema-style` already; no new
  dependency edges are introduced.

---

## §5 Locale Complement

Style bases interact cleanly with the locale override system (LOCALE_MESSAGES.md):

```yaml
extends: chicago-author-date-18th
options:
  locale-override: de-DE-chicago
```

The `de-DE-chicago` locale override encodes the locale-specific deviations from
`chicago-author-date-de.csl` (German conjunctions, date order, editor verb form)
without duplicating the style's structural template. This is expressed as a
regular top-level `options` override — the same mechanism used for any other
behavioral difference from the base.

This replaces the CSL pattern of language-variant style files like
`chicago-author-date-de.csl`. German-speaking users select
`extends: chicago-author-date-18th` and set their locale to `de-DE` (or apply
`locale-override: de-DE-chicago` for style-specific refinements); no separate
German style file is needed.

See `locales/overrides/de-DE-chicago.yaml` and `LOCALE_MESSAGES.md §4` for the
worked example.

---

## §6 Wizard & CLI Implications

**Style Navigator** (citum-hub): The 'Closest match' banner names a base key
(e.g. `chicago-notes-18th`), not a file path. "Use this" loads the compiled
base directly into `WizardState`. A YAML file is only produced when the user
deviates from the base — and even then the output contains `extends:` plus
only the overriding fields, not the full style.

**CLI**: `citum render` resolves `extends` before rendering. A future
`--base <key>` flag would make the registry directly addressable without a
style file argument, but that is out of scope for this bean.

**File naming** (csl26-zy07): Once style bases exist, YAML filenames for
well-known styles should derive from `short_name` + `edition`. The rename wave
is tracked separately and blocks on this bean's base key shape being stable.

---

## §7 Circular Dependency Prevention

To prevent infinite recursion during base resolution, Citum implements two
levels of protection:

1. **Resolution loop protection:** `into_resolved()` tracks visited
   `StyleBase` variants in a `HashSet`. If a base is encountered twice in
   the same resolution chain, the recursive call is aborted and the style is
   returned as-is for that branch.

2. **Base style invariant:** A `Style` that serves as a base for the
   `StyleBase` enum (e.g. `styles/embedded/apa-7th.yaml`) **must not** itself
   contain an `extends` field. Enforced by the `all_bases_resolve_cleanly` unit
   test and documented as an authoring constraint.

If a circular dependency is detected at runtime, the engine stops at the first
repetition and renders using any fields present beyond the `preset` key,
effectively treating the circular reference as a no-op. Base styles (Tier 1) must
not contain an `extends:` field; this is enforced by the `all_bases_resolve_cleanly`
unit test.

---

## §8 Corpus Impact

Base-extended styles enable significant corpus savings when combined with the
registry alias system (see `STYLE_TAXONOMY.md`). On the current CSL snapshot,
the top opportunities are families like APA (900+ potential aliases), Elsevier
variants (1,500+ total), and Springer templates (475+ dependent styles).


---

## §9 Known Tensions and Follow-Up Questions

This design is intentionally shippable before every policy question is fully
settled.

- **Benchmark identity.** When a base-extended style overrides part of its
  parent, the project still needs a stable rule for what comparator or oracle
  identity should govern fidelity claims.
- **Wrapper vs independent style.** A compact override is valuable; a large
  override layer may become harder to understand than an explicit standalone
  style. The project should define when bases are encouraged and when they
  should be avoided.
- **Replace-only arrays.** Structural deep merge works well for nested objects,
  but arrays and template lists still replace wholesale. This is the correct
  merge rule, but it creates an authoring footgun unless the docs and examples
  stay explicit.

Broader policy and authoring guidance are tracked in `csl26-wp6y`. The taxonomy
and four-tier classification are documented in `STYLE_TAXONOMY.md`.

---

## §10 Acceptance Criteria

- [x] `StyleBase::ChicagoNotes18th.base()` returns a `Style` that round-trips
      through serde without error.
- [x] A style YAML with `extends: chicago-notes-18th` and no other fields
      deserializes and resolves to the same effective `Style` as the base.
- [x] A style YAML with `extends: chicago-notes-18th` + `citation.ibid: ~`
      produces a `Style` where `citation.ibid` is `None`.
- [x] A style YAML with `extends: chicago-author-date-18th` +
      `options.page-range-format: expanded` preserves inherited option fields
      while overriding only the page-range behavior.
- [x] `options.locale-override: de-DE-chicago` paired with
      `extends: chicago-author-date-18th` renders `"und"` (not `"and"`) for a
      multi-author citation.
- [x] `citum.schema.json` updated to include `StyleBase`.
- [x] All existing oracle tests pass without regression.
---

## Changelog

- v2.1 (2026-07-28): Deferred merge-semantics ownership to
  [STYLE_INHERITANCE.md](./STYLE_INHERITANCE.md), which resolves the
  nested-option whole-value-replace divergence (`csl26-svfg`).
- v2.0 (2026-04-20): Renamed `StylePreset` → `StyleBase` and `preset:` →
  `extends:` throughout the schema (breaking rename; all YAML files updated).
  Expanded the base enum to 16 entries with full publisher and numeric styles.
  Added `StyleKind` enum to `RegistryEntry` for four-tier taxonomy classification.
  New spec `STYLE_TAXONOMY.md` documents the four tiers (base, profile, journal, independent).
- v1.5 (2026-03-19): Removed `StyleVariantDelta` and `StylePresetSpec`.
  Top-level style fields are now the sole override mechanism; `Style.preset`
  holds `Option<StylePreset>` directly. Unified authoring model: declare
  `preset:`, then override any fields at top level.
- v1.4 (2026-03-18): Updated merge semantics to structural deep merge, moved
  canonical runtime resolution into the engine, documented concrete preset
  base files, and added follow-up design tensions explicitly.
- v1.3 (2026-03-18): Marked Active after implementation and CI-tooling verification.
- v1.2 (2026-03-18): Added §7 Circular Dependency Prevention.
- v1.1 (2026-03-18): Updated to include Apa7th and ChicagoAuthorDate18th.
- v1.0 (2026-03-18): Initial Draft.
