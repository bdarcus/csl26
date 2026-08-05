# Typed Title Mapping Specification

**Status:** Active
**Version:** 1.0
**Date:** 2026-08-05
**Supersedes:** The `titles.type-mapping` open question in PR #1142
**Related:** bean `csl26-0h05`, PR #1143,
[`TYPE_CLASSIFICATION_CENTRALIZATION.md`](./TYPE_CLASSIFICATION_CENTRALIZATION.md),
[`FORWARD_COMPATIBILITY.md`](./FORWARD_COMPATIBILITY.md)

## Purpose

Make `options.titles.type-mapping` a checked boundary between reference types
and title-rendering categories. The current `HashMap<String, String>` accepts
misspelled keys and values, then silently either misses the lookup or falls
back to default rendering. The mapping began as a mechanical replacement for
processor hardcoding before `TitleCategory` existed; that historical shape is
not an extensibility contract.

## Scope

In scope:

- A normalized reference-type key that preserves forward-compatible unknown
  types while making lookup spelling consistent.
- A closed `TitleCategory` value vocabulary.
- Validation, inheritance, and serialization behavior for the mapping.
- Shared typed consumption by the engine and migration refinement.

Out of scope:

- Changing the default reference-type classification tables.
- Changing rendered output for valid existing styles.
- Template type-variant precedence, locale vocabulary overrides, or new title
  categories.

## Design

### Typed shape

`TitlesConfig.type_mapping` has this public Rust shape:

```rust
Option<HashMap<ReferenceTypeName, TitleCategory>>
```

`ReferenceTypeName` is an owned string newtype because reference inputs can
carry classes introduced by a newer schema. It normalizes `_` to `-` during
construction and deserialization, exposes its canonical spelling for lookup,
and serializes in canonical kebab-case. It is not a closed enum.

`TitleCategory` is the closed rendering-policy vocabulary:

- `component`
- `monograph`
- `periodical`
- `serial`
- `container-monograph`
- `default`

The legacy input spelling `collection` deserializes as
`container-monograph`; serialization always emits `container-monograph`.
Any other category value is a parse error rather than a silent fallback.

### Key validation and lookup

Known reference-type names share one authoritative vocabulary with template
type-selector validation. `default` remains a selector keyword and is not a
reference-type name. `all` was also such a keyword and is removed; see
[`TEMPLATE_V3.md`](./TEMPLATE_V3.md) §6.

Unknown canonical keys remain loadable for forward compatibility, but
`Style::validate` emits `SchemaWarning::UnknownTypeName` at
`options.titles.type-mapping`. This distinguishes an intentional future type
from a likely typo without preventing an older engine from loading the style.

#### The key vocabulary belongs in the published schema

The paragraph above is a statement about the **engine**: an unknown name loads,
warns, and matches nothing. That tolerance is deliberate, and it is not a reason
for `docs/schemas/style.json` to be silent about the vocabulary — but silent is
what it has been. `type-mapping` is `HashMap<ReferenceTypeName, TitleCategory>`,
and the published schema constrains only the value half. schemars generates
`additionalProperties` for map types and discards the key type, so the key half —
a validated, normalizing Rust type — reaches the schema as a free string.

This is not specific to `type-mapping`. `propertyNames` appears **nowhere** in
any of the eight published schemas, so every map-keyed vocabulary in Citum is an
open string space to a schema consumer.

The rule, stated once: **the published schema is the authoring contract and
carries the key vocabulary; the engine is the runtime and stays
forward-compatible.** A style using a reference type Citum does not yet know
fails schema validation in an editor and still renders correctly.

Applied to reference types, which are a closed vocabulary:

```json
"type-variants": {
  "type": "object",
  "propertyNames": { "enum": ["article-journal", "book", "…", "default"] },
  "additionalProperties": { "$ref": "#/$defs/TemplateVariant" }
}
```

The enum is **hyphenated only**. Authored underscore spellings are dropped
rather than enumerated alongside their canonical forms: normalizing the
authored side let two spellings of one name coexist, which is the ambiguity the
duplicate-key rule below already rejects within a single mapping. Normalization
of *incoming reference data* is unaffected — that is input tolerance, not an
authoring surface, and a hyphenated selector still matches a reference whose
type arrives with underscores.

Vocabularies that are genuinely open-ended — locale codes, message keys, field
names — take `propertyNames: { "pattern": … }` rather than an enum, so a typo is
caught without freezing an extensible namespace.

Scope note: this spec change lands the four reference-type maps
(`type-mapping` plus the three `type-variants` sites). The remaining nine
vocabularies are tracked separately; they span locales, messages, and
contributor roles and want their own review.

Lookup compares canonical spellings. Consequently, an authored
`personal_communication` key matches the engine's
`personal-communication` reference type. Two keys in one authored mapping
that normalize to the same spelling are a parse error; authored order must not
silently choose a winner.

### Inheritance

Existing `TitlesConfig` inheritance is unchanged:

- omission leaves the inherited mapping untouched;
- `type-mapping: ~` clears it;
- a non-null child mapping merges by canonical reference-type key, with child
  values replacing matching parent values.

Normalization occurs before merge, so underscore and kebab spellings cannot
create two effective entries for one reference type.

### Rendering and migration

The engine and `citum-migrate` consume `TitleCategory` values directly and
match them exhaustively. They must not convert the value back to a string.
Existing per-title-position fallback behavior remains unchanged: a category
that does not apply to the current title position uses that position's current
fallback.

## Implementation Notes

- Place `ReferenceTypeName` and the shared known-name vocabulary in
  `citum-schema-style`; re-export the type through the `citum-schema` facade.
- Keep `TypeSelector`'s public shape unchanged, but route its validation and
  normalization through the same helpers.
- Use a custom map deserializer so normalization collisions are detected before
  a `HashMap` overwrites an earlier entry.
- Regenerate the checked-in style schemas in the implementation commit.

## Acceptance Criteria

- [x] Invalid title-category values fail style parsing with the offending value.
- [x] Underscore-spelled mapping keys resolve against canonical kebab-case
  reference types.
- [x] Unknown reference-type keys load and produce a location-specific
  validation warning.
- [x] Duplicate keys after normalization fail style parsing.
- [x] Valid existing embedded styles render identically before and after the
  change.
- [x] Engine rendering and migration refinement agree for every title category
  and title position.

## Changelog

- v1.0 (2026-08-05): Initial specification.
- v1.0 implementation (2026-08-05): Activated after schema, engine, migration,
  and embedded-style regression validation.
