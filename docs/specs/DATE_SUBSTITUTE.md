# Date Substitute Specification

**Status:** Superseded
**Version:** 1.0
**Date:** 2026-08-12
**Superseded by:** [`DATE_FALLBACK.md`](./DATE_FALLBACK.md)
**Related:** csl26-qbmd, PR #1171, [`DISAMBIGUATION.md`](./DISAMBIGUATION.md), [`PRIMARY_CONTRIBUTOR_SUBSTITUTION.md`](./PRIMARY_CONTRIBUTOR_SUBSTITUTION.md)

## Purpose

> This v1 contract is retained as a historical record. The active v2 contract
> is [`DATE_FALLBACK.md`](./DATE_FALLBACK.md); it removes template fallbacks,
> adds first/later issued lanes and explicit `none` semantics, and renames the
> option to `date-fallback`.

Define an options-level policy for the fallback candidates used when a
reference's identity date is missing. The policy removes repeated inline
`TemplateDate.fallback` chains, gives style families reusable named presets,
and guarantees that rendering and disambiguation resolve the same candidate
source under the same effective citation or bibliography configuration.

A pre-migration survey of the 32 embedded style YAML files found 64
`fallback:` keys, all in three GB/T files. Fifty were date fallbacks: 25 in
`gb-t-7714-2025-base.yaml`, 22 in `gb-t-7714-2025-author-date.yaml`, and 3 in
`gb-t-7714-2025-note.yaml`. The other 14 are terminal author fallbacks in the
author-date style: after its options-level author-substitute chain is
exhausted, each `contributor: author` component renders `term.anonymous`.
Those author fallbacks belong to the existing author-substitute contract, not
this date-substitute contract. Layer 4 moved the identity-date policy into the
two GB/T presets; the author-date style retains eight inline fallbacks only on
later display dates. The concentration showed that the repeated behavior was
a style-family policy rather than a generic template default.

## Scope

In scope:

- a `date-substitute` option accepting a named preset or an explicit ordered
  selector map;
- three v1 presets: `standard`, `gb-t-7714-2025`, and
  `gb-t-7714-2025-author-date`;
- citation, bibliography, inheritance, and selector-map merge semantics;
- one shared candidate-resolution contract for rendering and disambiguation;
- the first identity-bearing date component in the effective template.

Out of scope:

- changing the primary date variable or the identity-slot component's own
  form, affixes, or wrapping;
- moving terminal author fallback into the options-level author-substitute
  policy; `TemplateContributor.fallback` remains part of author substitution;
- replacing inline fallback on later or display-only date components;
- migrating styles in the specification or implementation layers;
- changing the rule that an accessed date does not distinguish work identity.

## Public YAML Contract

`date-substitute` is optional and accepts either a preset name:

```yaml
date-substitute: standard
```

or a flat, insertion-ordered map from `TypeSelector` to complete candidate
lists:

```yaml
date-substitute:
  default:
  - message: term.no-date
    form: short
  book,thesis,map:
  - date: copyright
    form: year
    prefix: c
  article-journal,article-magazine: []
```

The map is deliberately flat. It does not add `template:` and `overrides:`
nesting around a structure that is already expressed by selector keys.
Selectors use the existing validated `TypeSelector` syntax and vocabulary.
`default` is the fallback selector and is handled specially; it is not a
wildcard mixed into normal selector matching.

### Candidate shape

Each candidate is either a date or a locale message. A date candidate carries
its own date form, rendering, and note policy independently:

```yaml
- date: copyright
  form: year
  suppress-note: true
  prefix: c
```

A message candidate carries the existing locale-message form and rendering
fields:

```yaml
- message: term.no-date
  form: short
```

The closed candidate type supports only these two variants. It does not accept
arbitrary `TemplateComponent` values.

Candidate rendering fields have the same semantics as on ordinary date and
message components. Candidate values pass through the central component
renderer; suppression, emphasis, quotes, strong, small caps, vertical
alignment, wrapping, affixes, text case, and period stripping must not be
silently ignored.

`suppress-note` remains available on date candidates. Omitting it preserves
the normal date-note behavior from the effective `dates.note-wrap` option;
`suppress-note: true` hides the note for that candidate.

## Presets

Presets are authoring conveniences, not hidden engine branches. Each preset
expands eagerly to the explicit selector map shown below before inheritance or
scope cascading.

### `standard`

`standard` is the generic baseline:

```yaml
default:
- message: term.no-date
  form: short
```

It is the default named value when an API requires a
`DateSubstitutePreset`, but it is not an implicit default for the optional
`Config.date_substitute` field.

### `gb-t-7714-2025`

This preset is shared by the numeric base and note styles. Missing identity
dates are blank by default; publication-like works and container chapters add
the GB/T fallbacks they require:

```yaml
default: []
chapter,entry-dictionary,entry-encyclopedia:
- date: accessed
  form: year
  wrap:
    punctuation: brackets
book,thesis,map:
- date: copyright
  form: year
  prefix: c
- date: printing
  form: year
  suffix: 印刷
- date: accessed
  form: year
  wrap:
    punctuation: brackets
```

The selector spellings are the existing authored reference types. There is no
`monograph` placeholder.

### `gb-t-7714-2025-author-date`

This preset starts with the standard no-date term, blanks journal and magazine
articles, uses the access year for web publications, and preserves the GB/T
publication-year chain:

```yaml
default:
- message: term.no-date
  form: short
article-journal,article-magazine: []
webpage,post,post-weblog:
- date: accessed
  form: year
  wrap:
    punctuation: brackets
- message: term.no-date
  form: short
book,thesis,map:
- date: copyright
  form: year
  prefix: c
- date: printing
  form: year
  suffix: 印刷
- date: accessed
  form: year
  wrap:
    punctuation: brackets
- message: term.no-date
  form: short
```

## Omission and Default Semantics

Omission is semantically distinct from naming `standard`:

- `date-substitute` omitted preserves the selected component's inline
  `fallback`, or its existing implicit issued/no-date behavior when no inline
  fallback exists;
- `date-substitute: standard` explicitly selects the standard options-level
  policy;
- an explicit selector map selects only the entries it contains.

Implementation must not use a `resolve_or_default(None)` path that silently
injects `standard`. `DateSubstitutePreset::default()` may return `standard`
for contexts that already require a preset value; an absent optional config
must remain absent.

## Selector Resolution

For a reference and an effective selector map, resolve candidates in this
order:

1. the first non-`default` selector, in insertion order, whose existing
   `TypeSelector::matches` behavior matches the reference;
2. the `default` selector, if present;
3. the component's existing inline fallback or implicit behavior when neither
   selector exists.

The first match wins. A matched empty list is a real match and intentionally
renders the identity date position blank. It must not fall through to
`default` or inline behavior.

The resolver must retain the distinction among these cases. One suitable
internal model is:

```rust
enum EffectiveDateCandidateSource<'a> {
    /// No options selector matched; resolve the component's existing behavior.
    Unmatched,
    /// An options selector matched. The slice may intentionally be empty.
    Options(&'a [DateSubstituteCandidate]),
    /// The selected inline source; `None` retains implicit issued/no-date.
    Inline(Option<&'a [TemplateComponent]>),
}
```

The exact internal names are non-normative. The three-state behavior is
normative: unmatched options, matched options (including matched-empty), and
inline/implicit fallback cannot collapse into one defaulted slice.

## Inheritance and Scope Cascading

Preset expansion happens before style inheritance and before citation or
bibliography scope cascading. After expansion, every value is the same ordered
selector-map representation.

Maps merge per selector key:

- a selector present only in the base remains;
- a selector present only in the child or narrower scope is appended in that
  layer's authored order;
- when both layers contain the same selector, the later candidate list
  replaces the earlier list as a whole, including replacement by `[]`;
- candidate lists never merge element-by-element.

The merged map must preserve deterministic selector order. Replacing an
existing key keeps that key's position; newly introduced keys append in the
later layer's order. This makes “first matching non-default selector” stable
across inheritance.

Citation and bibliography scopes resolve independently through the existing
effective configuration cascade. A bibliography-owned identity slot consumes
effective bibliography options; the citation fallback slot consumes effective
citation options.

## Identity Slot

`date-substitute` applies only to the first `TemplateDate` encountered in
effective template order, recursively descending into groups, whose
`suppress-disamb-suffix` is not `true`. This is the slot already selected by
`first_date_component_for_bibliography` and
`first_date_component_for_citation` for collision grouping.

Later dates, and dates marked `suppress-disamb-suffix: true`, keep their inline
fallback behavior. This includes archive dates, access-date footers, reprint
dates, and other display-only occurrences. A style may therefore use an
options-level policy for its identity slot and inline fallback for later date
components in the same template.

## Shared Rendering and Disambiguation Contract

Rendering and disambiguation must consume the same resolved candidate source,
the same selected identity slot, and the same effective scope configuration.
They must not independently re-resolve selectors or default omission.

Layer 1, PR #1171, supplies the required candidate-neutral foundation:
`fallback_candidate_discriminant` receives date form, rendering, and
`suppress-note` separately, so both inline `TemplateDate` candidates and
options-level candidates can use it. It also pairs a bibliography-selected
slot with effective bibliography date options and a citation-selected slot
with effective citation date options.

Layer 3 must provide one resolver result to both consumers:

- the renderer formats the first resolving candidate in order;
- the disambiguator derives its identity discriminant from that same selected
  candidate and render inputs;
- a message candidate uses its localized message identity;
- an accessed candidate may render but continues to contribute no work
  identity;
- a matched empty list renders the identity slot blank;
- year-suffix placement follows the behavior documented in
  [`DISAMBIGUATION.md`](./DISAMBIGUATION.md).

The invariant is semantic rather than textual: two references whose selected
date candidates render distinguishable identity content must not share a date
discriminant, and two references whose selected candidates render the same
identity content must not be split by unrendered raw precision.

## Implementation Notes

- Use an insertion-ordered map such as `IndexMap<TypeSelector,
  Vec<DateSubstituteCandidate>>`; a `HashMap` cannot implement the normative
  first-match rule.
- Reuse the schema's existing `TypeSelector` parser, validation, and reference
  type normalization.
- Include candidates from global, citation, and bibliography policies in the
  shared hard template-component resource budget, and emit the existing
  `UnknownTypeName` warning for unrecognized selectors at every scope.
- The config representation can be an untagged enum of preset scalar and
  explicit map. Preset expansion should produce an owned explicit map at the
  cascade boundary.
- Adding the public schema surface requires `just schema-gen` in Layer 3.
- Keep candidate resolution separate from candidate rendering so one resolved
  source can be shared by rendering and disambiguation.

## Acceptance Criteria

### Layer 2 — Draft specification

- [x] Three v1 presets and their exact expansions are defined.
- [x] Omission, explicit `standard`, unmatched maps, and matched-empty maps are
      distinct.
- [x] Selector resolution, ordered map merging, candidate-list replacement,
      identity-slot scope, and shared-consumer requirements are normative.
- [x] Survey counts and GB/T selectors reflect the current embedded styles.

### Layer 3 — Schema and engine implementation

- [x] The schema accepts the three preset names and flat ordered selector maps,
      and generated schemas are current.
- [x] `Config.date_substitute: None` preserves inline or implicit behavior;
      no defaulting path injects `standard`.
- [x] Presets expand before inheritance and scope cascading; selector maps
      merge per key and replace complete lists.
- [x] Tests cover first-match ordering, `default`, unmatched fallback,
      matched-empty blanking, preset expansion, and scope inheritance.
- [x] Rendering and disambiguation consume one resolved candidate source and
      effective scope configuration, including `suppress-note` behavior.
- [x] Options-level candidates participate in resource limits and selector
      validation, and use ordinary component rendering semantics.
- [x] Later and `suppress-disamb-suffix: true` dates retain inline fallback.
- [x] This spec changes from Draft to Active in the implementation commit.

### Layer 4 — GB/T style migration and fidelity

- [x] GB/T numeric and note-base styles use `gb-t-7714-2025`.
- [x] GB/T author-date uses `gb-t-7714-2025-author-date`.
- [x] Migrated identity slots remove redundant inline fallback while
      display-only date fallbacks remain inline.
- [x] GB/T oracle results meet or exceed the pre-migration baseline, and the
      exemplar/core-quality sweep reports no unrelated regression.

## Changelog

- 2026-08-12: Migrated the GB/T family to the named presets in Layer 4.
- 2026-08-12: Clarified candidate rendering, resource-budget, and selector-warning requirements after implementation review.
- 2026-08-12: Activated with the Layer 3 schema and engine implementation.
- 2026-08-12: Initial Draft for Layer 2 of the date-substitute stack.
