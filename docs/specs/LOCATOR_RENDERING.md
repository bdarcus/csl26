# Locator Rendering Specification

**Status:** Active
**Version:** 1.1
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

**In scope (v1.1):**
- **`attach` inside a grouped multi-item citation** (e.g. MLA's
  `multi-item-with-locators` case, `(A, Title 10; see also B et al 440)`).
  A grouped citation with more than one item renders each item's
  remaining template through `filter_author_from_template` /
  `render_item_from_template_with_format`
  (`crates/citum-engine/src/processor/rendering/grouped/core.rs`), which
  computes its own "leading affix" between the externally-rendered author
  heading and the item's own content — a join point separate from
  `citation_to_string_with_format`'s per-part loop, and one that guesses
  the delimiter from the item's *structurally* first remaining template
  component rather than what actually renders. See Engine semantics below
  for `leading_join_delimiter_override`, which corrects that guess using
  what actually rendered. Both single-item and multi-item grouped
  citations pick up `attach` correctly.

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

- **`attach` is not a prefix — it is a dedicated join-override channel.**
  An earlier draft of this section proposed writing the resolved `attach`
  into the locator's `Rendering.prefix`, reasoning that
  `prefix_supplies_own_leading_separator`
  (`crates/citum-engine/src/render/component.rs`) already suppresses the
  shared delimiter for a component with its own leading separator. That
  draft was wrong in a way only caught by the full test suite: `Rendering`
  affixes are baked into the component's own rendered text unconditionally
  (`realized_component_affixes`, applied inside
  `render_component_detailed_with_format_and_renderer`), regardless of the
  component's eventual position in the outer part list. When a *preceding*
  component collapses to empty at render time (e.g. `suppress_author` on a
  citation whose template is `[group[author,...], variable: locator]`), the
  locator becomes the first rendered part — and a baked-in `attach` prefix
  then renders with nothing to join to (`( 10)` instead of `(10)`).
  `ProcTemplateComponent` instead carries a separate
  `locator_attach: Option<DelimiterPunctuation>` field, realized into
  `RenderedComponent::join_delimiter_override: Option<String>` alongside
  (not merged into) the component's own affixes. The join loop in
  `citation_to_string_with_format`
  (`crates/citum-engine/src/render/citation.rs`) uses this override in
  place of the shared delimiter **only** for parts after the first —
  `for (i, part) in parts.iter().enumerate()` skips the join step entirely
  when `i == 0`. This makes "never appears when there's nothing to join to"
  structural rather than a rule to remember, and needs no change to
  `prefix_supplies_own_leading_separator` or `realized_component_affixes`.
- **Where `attach` is resolved.** The locator's `LocatorType` kind (and
  which `LocatorPattern`, if any, matches) is only known where the raw
  `CitationLocator` is still available — the `SimpleVariable::Locator` arm
  of value resolution (`crates/citum-engine/src/values/variable.rs`), which
  already loads the effective `LocatorConfig` and calls `render_locator`
  (`crates/citum-engine/src/values/locator.rs`). The renderer resolves
  `locator_attach` separately, at `ProcTemplateComponent` construction time
  (`processor/rendering/grouped/core.rs`'s
  `render_template_component_with_format`, via a small
  `resolve_locator_attach` helper), reusing the same
  kind/pattern-matching logic as `render_locator` through a shared
  `values::locator::effective_attach()` function — this is a different
  point in the pipeline from `get_effective_rendering`
  (`crates/citum-engine/src/render/component.rs`), which layers
  per-component-type config into `Rendering` but only sees the already-built
  `ProcTemplateComponent`, by which point the raw locator kind is gone.
- **Template `prefix` wins, structurally.** `resolve_locator_attach` returns
  `None` (no override) whenever the `variable: locator` component's own
  `TemplateVariable.rendering.prefix` is already set. The template's prefix
  is therefore rendered exactly as today — through the ordinary affix path,
  never through `locator_attach` — with no separate precedence rule for the
  classifier or realization path to implement.
- **Scope of the top-level join: `citation_to_string_with_format`'s own
  part list.** `join_delimiter_override` (like
  `supplies_own_leading_separator`) is read exactly once, inside that
  function's join over the *outer* list of a citation or integral
  template's top-level items (`crates/citum-engine/src/render/citation.rs`).
  The nested group-item join in
  `crates/citum-engine/src/processor/rendering/grouped/core.rs`
  (`render_group_child_values` / `join_with_quote_movement`, joining a
  `group:` node's own children, e.g. author+title) does not consult it — a
  comment there confirms per-part suppression was deliberately moved to
  the outer join only (csl26-475u). **`attach` inside a nested `group:` is
  out of scope for v1.1** and not implemented; if a future style needs it,
  that join needs the same classifier wired in first — a separate, larger
  change. Neither target style needs it: MLA's locator is a top-level
  item in both its citation and integral templates, never nested inside a
  `group:`; APA's locator is likewise always top-level.
- **A second join point: the author-heading-to-item join in grouped
  multi-item citations.** A citation with more than one cite item renders
  each item's remaining (author-stripped) template through
  `filter_author_from_template` /
  `render_item_from_template_with_format`
  (`crates/citum-engine/src/processor/rendering/grouped/core.rs`,
  `crates/citum-engine/src/processor/rendering/mod.rs`) — itself calling
  `citation_to_string_with_format` for the item's own top-level list, then
  separately splicing the externally-rendered author heading onto the
  front using a delimiter guessed by `filter_author_from_template` from
  the item's *structurally* first remaining `TemplateComponent`. That
  guess is wrong whenever the structural guess renders empty at runtime
  (e.g. a `disambiguate-only` title not needed for this reference) and a
  later component — a locator with its own `attach` — becomes the item's
  true first visible content: the stale guess (the title's own `", "`
  prefix) still gets spliced on, producing e.g. `"LeCun et al, 440"`
  instead of `"LeCun et al 440"`.

  Fixed by `leading_join_delimiter_override()`
  (`crates/citum-engine/src/render/citation.rs`): given the item's own
  `ProcTemplate`, it renders components in order and returns the
  `join_delimiter_override` of the first one that renders non-empty text
  — i.e. `citation_to_string_with_format`'s own answer to "what actually
  renders first," rather than a structural guess. `render_item_from_template_with_format`
  computes this alongside the rendered string; `render_group_item_parts_with_format`
  prefers it over the static `leading_affix` when both are present, only
  for the item establishing `group_delimiter` (index 0's contribution;
  subsequent same-author-collapse items join to each other via a
  different, unaffected delimiter). This makes the fix general — it
  corrects the same class of stale-guess bug for any component with a
  `join_delimiter_override`, not only locators, and the full corpus sweep
  below found seven *additional* pre-existing MLA rows it also fixes.
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
- **v1.1:** `attach` is not schema-only. It required new engine plumbing:
  `ProcTemplateComponent::locator_attach` and
  `RenderedComponent::join_delimiter_override`, resolved separately from
  (never baked into) the component's own affixes, so it only ever surfaces
  in `citation_to_string_with_format`'s join loop for parts after the
  first — see Engine semantics above for why the naively simpler "write it
  into `Rendering.prefix`" approach renders a stray separator when a
  preceding component collapses to empty (e.g. `suppress_author`). Scoped
  to top-level template items only; nested-`group:` attachment is
  unimplemented.

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

- [x] `label_case`, `attach`, and `strip_label_periods` present on both
      `LocatorConfig` and the new `LocatorOverrides`.
- [x] `LocatorConfigEntry` gains `PresetWithOverrides`, ordered
      `Preset, PresetWithOverrides, Explicit` (untagged — order is load
      bearing, see Schema additions). A parse-and-resolve test asserts the
      exact MLA YAML below deserializes as `PresetWithOverrides`, not
      `Explicit`, and resolves `page` to `LabelForm::None` (inherited from
      the `note` preset, not lost) —
      `given_a_preset_with_a_per_kind_attach_override_when_parsed_then_resolves_as_preset_with_overrides`.
- [x] `render_locator()` applies `label_case` to label terms and resolves
      the effective `attach` for the matched kind/pattern.
- [x] The resolved `attach` is threaded from locator value resolution
      (`values/locator.rs::effective_attach()`, called from
      `processor/rendering/grouped/core.rs::resolve_locator_attach`) into
      `ProcTemplateComponent::locator_attach` — a dedicated field, **not**
      `Rendering.prefix` and **not** `ProcTemplateComponent.prefix` — only
      realized at the join site
      (`RenderedComponent::join_delimiter_override`, consumed by
      `citation_to_string_with_format`) and only when the template did not
      already set its own `prefix`.
- [x] Test coverage includes: MLA single-item citation locator, MLA
      integral locator, and MLA *multi-item grouped* citation locator
      (`with-locator`, `suppress-author-with-locator`,
      `multi-item-with-locators` in `scripts/report-data` fixtures — all
      three flip to exact parity), and the regression the design avoids
      (`example_documents::mla_plain_text_shows_integral_name_memory`,
      `suppress_author` collapsing the preceding component to empty must
      not surface a stray leading separator on the locator).
- [x] `apa-7th.yaml` and `modern-language-association.yaml` updated to the
      YAML shown above; both flip to exact parity on the affected rows
      (APA 106/146 → 108/146; MLA 62/115 → 72/115 exact-parity rows,
      `node scripts/report-core.js --all-features --styles apa-7th,modern-language-association`).
- [x] Follow-up beans filed for: legal type-class label overrides
      (csl26-bg3f), self-identifying non-numeric locator values
      (csl26-swk3), `timestamp` as a `LocatorType` variant (csl26-nn2t).
- [x] No fidelity or exact-parity regression anywhere: full
      `cargo nextest run` (2767/2767) and a **full-corpus**
      `report-core.js --all-features` sweep (all 35 embedded styles, not
      only `locators:`-using ones — `leading_join_delimiter_override`
      touches the shared grouped-citation join path) show zero regressions
      and eight total exact-parity improvements, all in
      modern-language-association; `fidelityScore` unchanged for every
      style.

## Changelog

- v1.0 (2026-03-17): Initial version.
- v1.1 (2026-09-06): Added `label-case`, `attach`, and `strip_label_periods`
  on the new `LocatorOverrides`, and a `preset`-plus-`overrides` form for
  `Config.locators`. Fixes APA label-case and MLA locator-attachment
  fidelity gaps (bean csl26-7652). Rejected a template-level `LocatorKind`
  condition field; deferred a locator-class abstraction. `attach` routes
  through a dedicated `ProcTemplateComponent::locator_attach` /
  `RenderedComponent::join_delimiter_override` channel — never baked into
  a component's own text — so it only surfaces at the true join site in
  `citation_to_string_with_format`, both for single-item and (via
  `leading_join_delimiter_override`) multi-item grouped citations. Landed
  with `apa-7th.yaml` and `modern-language-association.yaml` updated,
  three follow-up beans filed for explicitly out-of-scope work
  (csl26-bg3f, csl26-swk3, csl26-nn2t), full `cargo nextest run` green,
  and a full-corpus `report-core.js` sweep showing eight exact-parity
  improvements and zero regressions across all 35 embedded styles.
