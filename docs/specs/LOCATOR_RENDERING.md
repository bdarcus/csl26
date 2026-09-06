# Locator Rendering Specification

**Status:** Active
**Version:** 1.0
**Date:** 2026-03-17
**Related:** bean csl26-3he9, `crates/citum-schema-style/src/options/locators.rs`,
`docs/specs/LOCATOR_INPUT.md`

## Purpose

Replace the per-template `show-label` / `strip-label-periods` fields on
`TemplateVariable` with a style-level `LocatorConfig` block. The citation
template decides *where* a locator appears; the locator rendering subsystem
decides *how* it is spelled, ranged, and labelled. This removes ad-hoc
label logic from the engine's hot path and makes compound-locator formatting
fully configurable by styles.

## Scope

**In scope:**
- New `LocatorConfig` and supporting types in `citum-schema-style`.
- A `render_locator()` function in `citum-engine` that is the sole consumer
  of those types.
- Removal of `show_label` and `strip_label_periods` from `TemplateVariable`
  and all call sites.
- Presets: `note` and `author-date`.
- Per-kind label-form and range-format overrides.
- Compound-locator patterns keyed by a set of `LocatorType` values plus an
  optional reference-type-class gate.

**Out of scope:**
- Backward compatibility with styles using the old fields.
- Full per-reference custom locator formatting beyond type-class distinctions.
- Changes to the fixed `CitationLocator` / `LocatorSegment` / `LocatorType`
  / `LocatorValue` data model.

## Design

### Schema types (`citum-schema-style/src/options/locators.rs`)

```rust
/// How a locator label should be displayed.
pub enum LabelForm {
    None,   // bare value: "33"
    Short,  // "p. 33"  (locale short term)
    Long,   // "page 33" (locale long term)
    Symbol, // locale symbol term if available
}

/// Whether labels are rendered on every segment, only the first, or none.
pub enum LabelRepeat {
    All,   // each segment gets its own label
    First, // only the first segment is labelled
    None,  // no labels (value only)
}

/// Per-kind configuration overrides.
pub struct LocatorKindConfig {
    pub label_form: Option<LabelForm>,       // overrides LocatorConfig::default_label_form
    pub range_format: Option<RangeFormat>,
    pub strip_label_periods: Option<bool>,
}

/// A pattern that matches a specific combination of locator kinds.
///
/// Patterns are tested in declaration order; first match wins.
pub struct LocatorPattern {
    /// The set of LocatorType values this pattern matches (order-insensitive).
    pub kinds: Vec<LocatorType>,
    /// Optional gate on reference type class.
    pub type_class: Option<TypeClass>,
    /// Rendering order of segments when pattern matches.
    pub order: Vec<LocatorType>,
    /// Delimiter between segments. Default: ", "
    pub delimiter: String,
    pub label_repeat: LabelRepeat,
}

/// Top-level locator rendering configuration.
pub struct LocatorConfig {
    pub default_label_form: LabelForm,     // Default: Short
    // `None` inherits the style-wide `options.range-format` default for
    // every locator kind (RANGE_COLLAPSE_MODEL.md Decision 2); `Some`
    // overrides it here.
    pub range_format: Option<RangeFormat>,
    pub kinds: HashMap<LocatorType, LocatorKindConfig>,
    pub patterns: Vec<LocatorPattern>,
    pub fallback_delimiter: String,        // Default: ", "
}

/// Preset-or-explicit wrapper (same pattern as DateConfigEntry).
pub enum LocatorConfigEntry {
    Preset(LocatorPreset),
    Explicit(LocatorConfig),
}

pub enum LocatorPreset {
    /// Note style: bare page numbers, no labels.  Other locator kinds show
    /// short labels.  Expanded ranges.
    Note,
    /// Author-date / numbered: short labels for all kinds, expanded ranges.
    AuthorDate,
}
```

`TypeClass` is a small closed enum covering the broad genre distinctions
needed for locator rendering (e.g. `Legal`, `Classical`, `Standard`).
It is not the same as the reference `ReferenceType` — it is a coarser
grouping that a style can specify in a pattern.

### `Config` integration

```yaml
# In a style's config block:

# Shorthand preset
locators: note

# Explicit
locators:
  default-label-form: short
  range-format: expanded
  patterns:
    - kinds: [page, line]
      delimiter: ", "
      label-repeat: all
    - kinds: [page]
      type-class: legal
      delimiter: ""
      label-repeat: none
```

`Config::merge` treats `locators` as an atomic replace (same as
`range_format`).

### Engine API (`citum-engine/src/values/locator.rs`)

```rust
/// Render a citation locator to a display string.
///
/// All label, range, and delimiter decisions are driven by `config`.
/// Returns an empty string when the locator is absent.
pub fn render_locator(
    locator: &CitationLocator,
    ref_type: &str,
    config: &LocatorConfig,
    locale: &Locale,
    style_range_format: Option<&RangeFormat>,
) -> String;
```

Template authors write:

```yaml
- variable: locator
  prefix: ", "
```

No label-control fields on the component. The engine resolves
`options.locator_raw` and calls `render_locator`.

### Rendering algorithm

1. Collect `LocatorType` set from the locator's segments.
2. Scan `config.patterns` in order; find first whose `kinds` set ⊆ active
   kinds, and whose `type_class` (if set) matches `ref_type`.
3. If pattern found: render segments in `pattern.order`; apply per-kind
   `LocatorKindConfig`; join with `pattern.delimiter`; honour
   `pattern.label_repeat`.
4. If no pattern: render each segment with its per-kind `LocatorKindConfig`
   (or `default_label_form`); join with `config.fallback_delimiter`.
5. Apply range formatting to any value that looks like a range (contains
   `–`, `-`, or `–`) per the kind's `range_format` or `config.range_format`.

### `RenderOptions` change

Add:

```rust
pub locator_raw: Option<&'a CitationLocator>,
```

The existing `locator: Option<&'a str>` and `locator_label: Option<LocatorType>`
fields are removed. `resolve_item_locator` is deleted; the renderer passes
the raw `CitationLocator` directly.

### Migration of affected styles

Styles that used `show-label: false` → set `locators: note` (or explicit
`default-label-form: none`).
Styles that used `show-label: true` → set `locators: author-date` (or
explicit `default-label-form: short`).
Styles that used `strip-label-periods: true` → set
`locators.kinds.page.strip-label-periods: true` (or global preset that does
so).

## Implementation Notes

- Follow `deny_unknown_fields` and `#[cfg_attr(feature = "schema", derive(JsonSchema))]`
  on all new structs.
- The `LocatorPreset::Note` preset suppresses page labels to match the
  existing engine behaviour for `Processing::Note`.
- `citum-migrate` fixup code that sets `show_label`/`strip_label_periods`
  should be updated to emit `locators` config onto the style's `Config`
  instead (or removed if migration always generates config-level locators).

## Acceptance Criteria

- [ ] `show_label` and `strip_label_periods` removed from `TemplateVariable`.
- [ ] `LocatorConfig`, `LocatorKindConfig`, `LocatorPattern`, `LabelForm`,
      `LabelRepeat`, `LocatorConfigEntry`, `LocatorPreset` defined in
      `citum-schema-style/src/options/locators.rs`.
- [ ] `Config.locators` field wired with preset-or-explicit deserializer.
- [ ] `render_locator()` in `citum-engine/src/values/locator.rs`; old
      `format_locator_value` and `collapse_compound_locator` deleted.
- [ ] `RenderOptions.locator_raw` replaces `locator` + `locator_label`.
- [ ] All existing styles updated to use `locators:` config or preset.
- [ ] `citum-migrate` fixup code updated to remove references to deleted fields.
- [ ] Oracle tests pass at existing fidelity levels.
- [ ] BDD integration tests added for: bare page, short-label page,
      compound page+line, fallback compound, type-class-gated pattern.

## Changelog

- v1.0 (2026-03-17): Initial version.
