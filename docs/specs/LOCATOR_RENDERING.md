# Locator Rendering Specification

**Status:** Active (v1.0); **v1.1 addition below: Draft, pending review**
**Version:** 1.1-draft
**Date:** 2026-09-06
**Related:** bean csl26-3he9, bean csl26-7652, bean csl26-t1hh,
`crates/citum-schema-style/src/options/locators.rs`

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

**In scope (v1.1 addition, bean csl26-7652):**
- `label-case` on `LocatorConfig` and `LocatorKindConfig`: a text-case
  transform applied to the rendered label term.
- `attach` on `LocatorConfig` and `LocatorKindConfig`: a per-kind override of
  the delimiter joining the locator to its preceding sibling.
- A `preset`-plus-`overrides` form for `Config.locators`, so a style can keep
  a shorthand preset (e.g. `note`) while overriding individual fields —
  today `LocatorConfigEntry` is only `Preset` or fully `Explicit`, and
  `Config::merge` treats `locators` as an atomic replace.

**Out of scope (v1.1):**
- Per-kind label overrides gated on reference type-class beyond the existing
  `LocatorPattern.type_class` gate (e.g. Bluebook-style symbol labels for
  legal types). Deferred to a follow-up bean.
- Self-identifying non-numeric locator values (a canonical pinpoint like
  `2.14.3` that carries no label regardless of kind). Deferred to a
  follow-up bean.
- Adding `timestamp` to the `LocatorType` enum. Deferred to a follow-up bean.
- A named "locator class" abstraction (a third precedence tier grouping
  multiple kinds under one label). See Rejected Alternatives below.

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

## Label Case and Attachment (v1.1)

**Status of this section: Draft.** Pending review; flips to Active alongside
`v1.1` (dated) once implementation lands.

### Problem

Two embedded styles have a live fidelity gap that the v1.0 schema cannot
express, because both gaps are about a *locator kind*, not a locator's label
form or range:

**MLA** (`modern-language-association.yaml`) joins the locator to the
preceding author-short/title differently by kind. mla.csl
(`styles-legacy/modern-language-association.csl:1144-1158`) picks the
delimiter procedurally:

```xml
<if locator="line page timestamp" match="any">
  <group delimiter=" ">
    <text macro="author-short"/>
    <text variable="locator"/>
  </group>
</if>
<else>
  <group delimiter=", ">
    <text macro="author-short"/>
    <text macro="label-locator"/>
  </group>
</else>
```

Citum's citation template renders `variable: locator` as a template item
after the author/title group, so it always takes the citation's own `", "`
delimiter — correct for the labelled branch, wrong for `page`/`line`, which
covers 10 of 34 residual N-punctuation rows:

```
rendered: (Kuhn, The Structure of Scientific Revolutions, 23)
oracle:   (Kuhn, The Structure of Scientific Revolutions 23)
```

(This gap was independently tracked in bean csl26-t1hh, which reached the
same options/preset-over-`render-when` conclusion via the doctrine recorded
in csl26-qyub before this spec existed; csl26-7652 and csl26-t1hh are now
cross-linked rather than duplicated.)

**APA** (`apa-7th.yaml`) has no `locators:` block, so every kind takes the
schema default `default-label-form: short`. apa.csl's `label-locator` macro
(`styles-legacy/apa.csl:204-236`) abbreviates only `page`/`paragraph`; every
other kind gets a long, capitalized label:

```
rendered: (Hawking, 1988, sec. 12, esp. discussion)
oracle:   (Hawking, 1988, Section 12, esp. discussion)
```

### Why this belongs on the locator, not the template

A locator's join behavior is a property of *the locator*, not of where the
template happens to place it: a bare pinpoint (`23`) is an appositive that
binds tight to what precedes it; a labelled phrase (`sec. 12`) reads as a
separate clause needing its own comma. `options.locators` already owns label
form and range format per kind — attachment and label case are two more
facets of "how this locator kind is spelled," not new template logic.

### Schema additions

```rust
/// Per-kind configuration overrides (v1.1: two new fields).
pub struct LocatorKindConfig {
    pub label_form: Option<LabelForm>,
    pub range_format: Option<RangeFormat>,
    pub strip_label_periods: Option<bool>,
    /// Text-case transform applied to this kind's rendered label term.
    /// `AsIs` opts a kind out of a config-level `label_case`.
    pub label_case: Option<crate::options::titles::TextCase>,
    /// Overrides the delimiter joining this locator to its preceding
    /// sibling, when the locator is a top-level item in the citation or
    /// integral template (see Engine semantics: nested `group:` items are
    /// out of scope for v1.1).
    pub attach: Option<DelimiterPunctuation>,
}

/// Top-level locator rendering configuration (v1.1: two new fields; existing
/// `strip_label_periods` field shown for contrast with `LocatorOverrides`
/// below, which must also carry it).
pub struct LocatorConfig {
    pub default_label_form: LabelForm,
    pub range_format: Option<RangeFormat>,
    pub kinds: HashMap<LocatorType, LocatorKindConfig>,
    pub patterns: Vec<LocatorPattern>,
    pub fallback_delimiter: String,
    /// Strip trailing periods from labels globally. Existing v1.0 field.
    pub strip_label_periods: Option<bool>,
    /// Default label-case transform for all kinds unless overridden.
    pub label_case: Option<crate::options::titles::TextCase>,
    /// Default attachment delimiter for all kinds unless overridden.
    pub attach: Option<DelimiterPunctuation>,
}

/// All-`Option` overlay applied on top of a resolved preset.
///
/// `LocatorConfig` itself cannot serve as the overlay: its
/// `default_label_form` and `fallback_delimiter` fields are non-`Option`
/// with serde defaults, so a flattened `LocatorConfig` overlay could not
/// distinguish "field not set, inherit the preset" from "field explicitly
/// set to the schema default." Every field here must mirror a field on
/// `LocatorConfig` — `strip_label_periods` is included for that reason;
/// omitting it would silently make it un-overridable for a preset-based
/// style (e.g. `numeric`, which sets it via the preset).
pub struct LocatorOverrides {
    pub default_label_form: Option<LabelForm>,
    pub range_format: Option<RangeFormat>,
    pub strip_label_periods: Option<bool>,
    pub label_case: Option<crate::options::titles::TextCase>,
    pub attach: Option<DelimiterPunctuation>,
    pub fallback_delimiter: Option<String>,
    /// Merged into the preset's `kinds` map per key, not replaced wholesale.
    pub kinds: HashMap<LocatorType, LocatorKindConfig>,
    pub patterns: Option<Vec<LocatorPattern>>,
}

/// Preset-or-explicit wrapper. `#[serde(untagged)]`, same as v1.0.
///
/// Variant order matters for an untagged enum: `Preset` is tried first (a
/// bare string), then `PresetWithOverrides` — its `preset` field is
/// required, so it only matches a mapping that has that key, and falls
/// through otherwise. `Explicit` must come **last**: `LocatorConfig`
/// carries a `#[serde(flatten)] unknown_fields` catch-all with no
/// `deny_unknown_fields`, so `Explicit` will happily accept (and silently
/// ignore into `unknown_fields`) a stray `preset` key. Trying `Explicit`
/// before `PresetWithOverrides` would make `{preset: note, kinds: {...}}`
/// resolve as a bare `Explicit` with `preset` discarded and no preset
/// behavior applied at all — the v1.0 draft of this enum had this bug.
pub enum LocatorConfigEntry {
    Preset(LocatorPreset),
    /// A preset resolved to a `LocatorConfig`, then overlaid with `overrides`.
    PresetWithOverrides {
        preset: LocatorPreset,
        #[serde(flatten)]
        overrides: LocatorOverrides,
    },
    Explicit(LocatorConfig),
}
```

`label_case` reuses `crate::options::titles::TextCase`
(`crates/citum-schema-style/src/options/titles.rs:22-41`) rather than a new
enum — the same transforms (`capitalize-first`, `as-is`, etc.) apply to a
label term as to a title. `attach` reuses `DelimiterPunctuation`, the same
type as `prefix`/`suffix`/the template's own `delimiter`.

### YAML: MLA

```yaml
options:
  locators:
    preset: note
    kinds:
      page: { attach: " " }
      line: { attach: " " }
# citation "(Kuhn, The Structure of Scientific Revolutions 23)"
# citation "(Kuhn, The Structure of Scientific Revolutions, sec. 12)"
```

No config-level `attach` here, deliberately. Labelled kinds already receive
the citation's own `", "` (the default resolved by
`Processor::resolve_citation_delimiters`,
`crates/citum-engine/src/processor/citation.rs:146-182`, when no explicit
`delimiter` is set), and a style-wide `attach: ", "` would also apply inside
MLA's *integral* template, where the locator is wrapped in parentheses —
producing `Kuhn, (sec. 12)` instead of `Kuhn (sec. 12)`. Per-kind `attach: " "`
on `page`/`line` is correct in both the citation and integral templates. The
example strings above show the disambiguation-title branch; in the ordinary
case (no disambiguation needed) MLA renders `(Kuhn 23)`.

### YAML: APA

```yaml
options:
  locators:
    default-label-form: long
    label-case: capitalize-first
    kinds:
      page:      { label-form: short, label-case: as-is }
      paragraph: { label-form: short, label-case: as-is }
# "(Hawking, 1988, Section 12, esp. discussion)"
# "(Hawking, 1988, p. 33)"
```

APA's "every kind except page and paragraph" is expressed by flipping the
config-level default to `long` + `capitalize-first`, then opting the two
short-form kinds back out with `label-case: as-is` — not by enumerating the
other ~30 `LocatorType` variants individually.

### Engine semantics

- **What actually classifies a join-delimiter suppression.**
  `ProcTemplateComponent` (`crates/citum-engine/src/render/component.rs:13-44`)
  carries *two* separate prefix-like fields, and only one of them matters
  here. `component.prefix: Option<String>` ("prefix from value extraction")
  is rendered as an **inner** affix — inside any `wrap`, next to the raw
  value (`total_inner_prefix`,
  `crates/citum-engine/src/render/component.rs:317-330`) — and is never
  consulted by the join-delimiter logic. The field that matters is
  `Rendering.prefix: Option<DelimiterPunctuation>` (the template-authored,
  semantic prefix): it is realized as an **outer** affix
  (`realized_component_affixes`,
  `crates/citum-engine/src/render/component.rs:183-201`) and is the sole
  input to `prefix_supplies_own_leading_separator`
  (`crates/citum-engine/src/render/component.rs:363-403`), whose result
  (`RenderedComponent::supplies_own_leading_separator`) the join loop in
  `citation_to_string_with_format` consults to skip the shared delimiter
  ahead of a part (`crates/citum-engine/src/render/citation.rs:150-176`).
  **`attach` must resolve into `Rendering.prefix` for the locator's
  component, not into `component.prefix`.**
- **Where `attach` is resolved.** The locator's `LocatorType` kind (and
  which `LocatorPattern`, if any, matches) is only known where the raw
  `CitationLocator` is still available — the `SimpleVariable::Locator` arm
  of value resolution (`crates/citum-engine/src/values/variable.rs:335-355`),
  which already loads the effective `LocatorConfig` and calls
  `render_locator` (`crates/citum-engine/src/values/locator.rs:26-62`). This
  is a different point in the pipeline from `get_effective_rendering`
  (`crates/citum-engine/src/render/component.rs:455-484`), which layers
  per-component-type config into `Rendering` but only sees the already-built
  `ProcTemplateComponent` — by then the raw locator kind is gone, leaving
  only the rendered value string. The resolved `attach`, once computed
  during value resolution, must be carried forward and set as the locator
  component's `Rendering.prefix` at `ProcTemplateComponent` construction
  time — new plumbing, not a reuse of the existing per-type
  `get_effective_rendering` arms (those key on static config/`ref_type`
  only). **This is new engine wiring, not solely a schema addition** — the
  Files section below is corrected accordingly.
- **Template `prefix` wins, structurally.** The resolved `attach` is written
  into `Rendering.prefix` **only when the template's own `TemplateVariable`
  did not already set one**. This makes precedence a fallback check at
  construction time rather than a separate rule the classifier or the
  realization path needs to know about — the existing merge/realize/classify
  code is otherwise untouched.
- **Scope of this addition: top-level template items only.** Confirmed by
  reading every consumer of `supplies_own_leading_separator`: it is read
  exactly once, inside `citation_to_string_with_format`'s join over the
  *outer* list of a citation or integral template's top-level items
  (`crates/citum-engine/src/render/citation.rs:150-176`). The nested
  group-item join in
  `crates/citum-engine/src/processor/rendering/grouped/core.rs` (around
  line 695) does not consult it — a comment there confirms per-part
  suppression was deliberately moved to the outer join only (csl26-475u).
  Both target styles need only this scope: MLA's citation template is
  `[group[author, title], variable: locator]` and its integral template is
  `[contributor: author, variable: locator]` — in both, `variable: locator`
  is a top-level item, not nested inside a `group:`. APA's locator is
  likewise always a top-level array item. **`attach` inside a nested
  `group:` is out of scope for v1.1** and not implemented; if a future style
  needs it, the nested-group join in `grouped/core.rs` needs the same
  classifier wired in first — a separate, larger change.
- **Compound locators.** When a `LocatorPattern` matches, the `attach` of
  the first kind in `pattern.order` governs the join for the whole rendered
  locator.
- **`label-case` ordering.** Applies to the rendered label term only (never
  the locator's value), and after `strip-label-periods`.
- **Precedence, all locator knobs:** `kinds.<kind>.<field>` overrides
  `LocatorConfig.<field>` (or `LocatorOverrides.<field>`) overrides the
  resolved preset overrides the schema default. `strip_label_periods`
  follows the same chain: `Some(false)` at any level is an explicit clear,
  distinct from an omitted field (which inherits).
- **`PresetWithOverrides` merge.** The preset resolves to a `LocatorConfig`
  first; then each `Some` field in `overrides` overlays it, field by field
  (`strip_label_periods` included). `overrides.kinds` merges per
  `LocatorType` key into the preset's `kinds` map — it does not replace the
  map wholesale.

### Rejected alternatives

- **`TemplateConditionField::LocatorKind` + `render_when` on a wrapping
  group.** Considered because `TemplateGroupCondition` / `render_when`
  already exist for other field-presence branching. Rejected: this
  transliterates CSL's `<choose locator="...">` into the Citum template
  layer, the one place this project has deliberately kept declarative. Every
  style needing kind-dependent joining would grow a two-branch template
  duplicating structure on each side, and the branching logic would live far
  from the `LocatorConfig` that already owns every other per-kind decision.
- **Derive `attach` from `label-form: none`.** Considered as a way to avoid
  a new field: "no label" and "tight join" often co-occur. Rejected: not
  derivable in general. Chicago author-date renders a bare page number
  (`label-form: none`) joined with a comma (`Smith 2020, 33`), while MLA
  joins the same bare page number with a space (`Kuhn 23`). Label form and
  join delimiter are independent facts about a locator kind.
- **Named locator classes** (a `classes:` map grouping several `LocatorType`
  values under one shared config, as a precedence tier between
  `default_label_form`/`label_case`/`attach` and `kinds.<kind>`). Deferred,
  not rejected outright: both APA and MLA need at most three per-kind
  entries each, so the DRY benefit of a class abstraction doesn't yet
  outweigh a third precedence tier's complexity. Revisit if a future style
  needs the same override across five or more kinds.

## Implementation Notes

- Follow `deny_unknown_fields` and `#[cfg_attr(feature = "schema", derive(JsonSchema))]`
  on all new structs.
- The `LocatorPreset::Note` preset suppresses page labels to match the
  existing engine behaviour for `Processing::Note`.
- `citum-migrate` fixup code that sets `show_label`/`strip_label_periods`
  should be updated to emit `locators` config onto the style's `Config`
  instead (or removed if migration always generates config-level locators).
- **v1.1:** `attach` is not schema-only. It requires new engine plumbing to
  carry the resolved value from locator value-resolution time (where the
  `LocatorType` kind is known) into the built `ProcTemplateComponent`'s
  `Rendering.prefix` (where the join-suppression classifier reads it) — see
  Engine semantics above for the exact fields and why `get_effective_rendering`
  cannot host this by itself. Scope the v1.1 implementation to top-level
  template items only; do not attempt to also wire nested-`group:` support
  in the same change.

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

**v1.1 additions:**

- [ ] `label_case`, `attach`, and `strip_label_periods` present on both
      `LocatorConfig` and the new `LocatorOverrides`.
- [ ] `LocatorConfigEntry` gains `PresetWithOverrides`, ordered
      `Preset, PresetWithOverrides, Explicit` (untagged — order is load
      bearing, see Schema additions). A parse-and-resolve test asserts the
      exact MLA YAML below deserializes as `PresetWithOverrides`, not
      `Explicit`, and resolves `page` to `LabelForm::None` (inherited from
      the `note` preset, not lost).
- [ ] `render_locator()` applies `label_case` to label terms and resolves
      the effective `attach` for the matched kind/pattern.
- [ ] The resolved `attach` is threaded from locator value resolution
      (`values/variable.rs`'s `SimpleVariable::Locator` arm) into the
      locator's `ProcTemplateComponent.rendering.prefix` — **not** into
      `ProcTemplateComponent.prefix` (a different, inner-affix field) — and
      only when the template did not already set its own `prefix`. Tests
      confirm `Rendering.prefix` (not `component.prefix`) carries the
      value and that `supplies_own_leading_separator` is exercised, not
      bypassed.
- [ ] Test coverage includes: MLA citation top-level locator, MLA integral
      top-level locator (both flip to `Kuhn 23` / `Kuhn (sec. 12)`), and an
      explicit non-goal test/note that `attach` inside a nested `group:` is
      unimplemented in v1.1 (out of scope, not silently broken).
- [ ] `apa-7th.yaml` and `modern-language-association.yaml` updated to the
      YAML shown above; both flip to exact parity on the affected rows.
- [ ] Follow-up beans filed for: legal type-class label overrides,
      self-identifying non-numeric locator values, `timestamp` as a
      `LocatorType` variant.
- [ ] No fidelity or exact-parity regression elsewhere
      (`node scripts/report-core.js --all-features`).

## Changelog

- v1.0 (2026-03-17): Initial version.
- v1.1-draft (2026-09-06): Added `label-case`, `attach`, and
  `strip_label_periods` on the new `LocatorOverrides`, and a
  `preset`-plus-`overrides` form for `Config.locators`. Fixes APA
  label-case and MLA locator-attachment fidelity gaps (bean csl26-7652).
  Rejected a template-level `LocatorKind` condition field; deferred a
  locator-class abstraction. Revised after adversarial review: corrected
  `LocatorConfigEntry` variant order (untagged-enum shadowing was silently
  losing the preset), corrected the join mechanism to target
  `Rendering.prefix` (not the unrelated `component.prefix` inner-affix
  field) and named the new engine wiring this requires, narrowed the join
  claim to top-level template items only, and added the missing
  `strip_label_periods` overlay field.
