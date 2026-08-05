# Template Schema v3 Specification

**Status:** Active
**Version:** 0.6
**Date:** 2026-08-05
**Supersedes:** `docs/specs/TEMPLATE_V2.md`
**Related:** csl26-t3v1, csl26-zaxo, `docs/specs/DISTRIBUTED_RESOLVER.md`

## Purpose

**Audience:** Engine implementers and style authors.

The "hard-fork" nature of V2's `type-variants` (complete replacement with no inheritance) is incompatible with a distributed style ecosystem. V3 reintroduces **Structural Template Inheritance** using a pure diff-based model, so that every type-variant can be deterministically derived from the base template at resolution time.

This design explicitly **rejects "Macros"** to avoid the complexity and fragmentation of CSL 1.0. Instead, it relies on two pillars:
1.  **Surgical Diffs:** Type-variants that modify, add, or remove components from a base template.
2.  **Logic-Heavy Options:** Moving shared formatting logic (contributor lists, date fallbacks) into style-level configuration rather than template structures.

## Scope

**In Scope:**
- `extends` keyword within `type-variants` (defaulting to the base `template`).
- List-diff operations: `modify`, `add`, `remove`.
- `message:` template components for locale-authored phrase realization.
- Expansion of `options` (e.g., `contributor-config`, `date-config`) to absorb shared logic.
- Impact on `DistributedResolver` style-merging.

**Out of Scope:**
- Named templates or Macros (Forbidden).
- Cross-section references to named reusable template fragments (Forbidden).
- YAML Anchors (MAY be used locally for authoring convenience but MUST NOT be relied upon for cross-style reuse).

Family roots MAY share a complete citation or bibliography section through
style inheritance. When a family head changes ordering-sensitive structure,
the head replaces that section explicitly. Some component sequences may
therefore repeat across heads; that duplication is preferred to reintroducing
CSL-like macro calls and does not by itself identify a schema gap.

## Terminology

- **Template:** An ordered list of components.
- **Component:** A single rendering instruction (e.g., `contributor: author`, `date: issued`, alongside rendering hints like `prefix` or `form`).
- **Type-variant:** A named diff that transforms the base template into a specialized template for a specific reference type (e.g., `article-journal`).

---

## Design

### §1 — The Structural Diff Model

In V3, every `type-variant` is a transformation of a parent template. By recording the **intent of the change** rather than a copy of the result, we ensure that updates to the parent style flow through to the variant.

```yaml
bibliography:
  template:
    - contributor: author
    - date: issued
      form: year
    - title: primary
    - variable: publisher
    - variable: url

  type-variants:
    article-journal:
      # If `extends` is omitted, the variant implicitly extends the base `template`.
      modify:
        - match: { variable: publisher }
          suppress: true
      add:
        - after: { title: primary }
          component: { title: parent-serial, emph: true }
```

If `extends` is omitted, the variant implicitly extends the base `template`. Optionally, `extends` MAY reference another type-variant of the same template, in which case the parent variant's diffs are applied before the child's.

### §2 — Absorbing Macros into Style Options

The primary reason authors use macros in other systems is to ensure consistent formatting for complex entities (like a list of 10 authors). Citum solves this by moving that logic into `options`.

#### 2.1 Contributor Configuration
Instead of a "Macro" for author formatting, authors configure the `contributor-config` once. Rendering of any component with `contributor: <role>` MUST be governed by `options.contributors.<role>` unless that component explicitly overrides one of these policies with a local hint (e.g., a local `delimiter`).

```yaml
options:
  contributors:
    author:
      shorten: { min: 3, use-first: 1 }
      and: "symbol"
      delimiter: ", "
      et-al-use-last: true
```

#### 2.2 Date Configuration
Templates SHOULD reference logical date roles (e.g., `date: issued`) while
`options.dates` centralizes their formatting policy. A date component MAY use
`fallback` to define its missing-value behavior. An absent fallback preserves
the engine default (`issued` uses the locale no-date term); an explicit fallback
list is authoritative. If every fallback component is empty, including when the
list itself is empty, the date is omitted.

```yaml
- date: issued
  form: year
  fallback: [] # Omit when issued is unavailable.
```

CSL and CSL-M date elements render nothing when their variable is unavailable.
Migration therefore emits an explicit empty fallback for `issued` dates unless
the source style supplies another fallback, such as a localized no-date term.

```yaml
- date: issued
  form: year
  fallback:
- message: term.no-date
```

### §2.3 Declarative Reference Markers

Processor-generated markers are style semantics rather than authored data
selection. A style MUST declare its marker policy in scoped options; the
marker cannot appear in a template. See
[REFERENCE_MARKERS](REFERENCE_MARKERS.md).

```yaml
options:
  processing: numeric

citation:
  options:
    label-mode: numeric
    label-wrap: brackets
  collapse: citation-number
```

The engine resolves locale and type variants first, then materializes a semantic
numeric-label slot before collapse and final formatting. Non-integral labels
lead the item; explicit integral templates receive the label after authored
content using the effective integral delimiter. An omitted citation label mode
implies `numeric` only for numeric processing. `label-mode: none` suppresses an
inherited marker, and a declared mode strips a marker of any other kind
inherited from a parent.

`number: citation-number` and `number: citation-label` are **not template
components** and are rejected at parse time. Wrapping operates at three scopes:
`citation.wrap` is cluster-level, `citation.options.item-wrap` encloses the
marker together with the item body (IEEE's `[1, p. 737]`), and
`citation.options.label-wrap` encloses the marker alone (AMA's `[1](p737)`).

Bibliographies use the canonical `bibliography.options.label-mode` setting. The
renderer materializes the marker as a leading slot after type-variant
resolution and applies wrapping at runtime, without rewriting the style
template. `bibliography.options.label-separator` supplies the gap between
marker and entry body — empty by default, which renders flush and matches
citeproc-js `second-field-align` output flattened to text. The separator is
carried on the marker itself, so it survives when the entry body renders empty
(an entry with no author).

### §2.4 MF2 Message Components

Templates MAY call an MF2 phrase with `message:`. Message bodies normally come
from the active locale, but a style MAY define specialized messages in
`options.messages`. A style-owned message takes precedence over a locale
message with the same ID, and inherited message maps merge by ID. This lets a
hidden family root own standard-specific textual classifications without
putting them into every locale.

```yaml
- message: pattern.accessed-date
  args:
    date: { date: accessed, form: day-month-abbr-year }

- message: pattern.in-container
  args:
    container: { title: parent-monograph, emph: true }
  text-case: capitalize-first

options:
  messages:
    standard.type-code: |-
      .match {$type :select} {$carrier :select}
      when book - {M}
      when book * {M/{$carrier}}
      when * * {Z}

bibliography:
  template:
    - message: standard.type-code
      args:
        type: { reference-type: key }
        carrier: { carrier: { online: OL, absent: '-' } }
```

Each `args` entry is rendered through the normal component pipeline before MF2
evaluation. Supported argument sources are `literal`, `variable`, `date`,
`title`, `contributor`, `number`, `term`, `group`, `reference-type`, and
`carrier`. `reference-type: key` supplies the canonical Citum reference-type
key. `carrier` supplies the reference's explicit medium when present, otherwise
the configured `online` value for URL, DOI, or CSTR resources, and the
configured `absent` value for offline resources. The resulting strings become
MF2 named variables (`{$date}`, `{$container}`, etc.).

Style-owned messages are a textual-realization and classification mechanism.
They MUST NOT be used to recreate general template control flow, and this spec
does not add a generic literal template component or CSL-style conditional
language. Structural selection remains in typed templates and type variants.

The style owns phrase and argument selection; the locale normally owns word
order and glue text. `term:` components remain readable for compatibility, but
new localized phrase work SHOULD use `message:` and `pattern.*` locale IDs.
`term.*` and `role.*` message IDs remain valid for lexical labels,
abbreviations, and inflected role forms; role-plus-name phrases can move to
`pattern.*` when the locale needs to control placement around rendered names.

### §2.4 Conditional Number Labels (`when-numeric`)

Some numbering vocabulary is only labeled when the resolved value is a bare
number; free-text values are already a complete label and render unwrapped.
GB/T 7714 renders a numeric edition as `2 版` but a free-text edition (`修订版`,
`Rev. ed.`) bare, and a numeric volume as `第4卷` but an already-labeled volume
(`美国卷`) bare. CSL-M expresses the same rule with `<if is-numeric="…">`.

A `number:` component MAY set `when-numeric: <label-form>` (`long`, `short`, or
`symbol`, the same vocabulary as `label-form`). When set, and the resolved
value is numeric — one or more digit runs, optionally joined by the
punctuation of ranges and lists (`-`, `,`, `&`, space) — the engine resolves
that number variable's general locale term at the given form and wraps the
value with it. Non-numeric values render bare, with no label.

The term itself is locale-owned, not style-owned, and follows the CSL-M
convention: a term containing a literal `%s` (e.g. zh-CN's `第%s卷`, matching
the pinned upstream term) wraps the value at that position; a term without
`%s` (e.g. `版`) follows the value as a space-separated suffix, matching
GB/T 7714's `<number/> <label/>` ordering for numeric editions.

```yaml
# style (gb-t-7714-2025-base.yaml)
- number: edition
  when-numeric: short
- group:
  - title: primary
  - number: volume
    when-numeric: short
  delimiter: ：
```

```yaml
# locale (zh-CN.yaml)
terms:
  edition:
    short: { singular: 版, plural: 版 }
  volume:
    short: { singular: 第%s卷, plural: 第%s卷 }
```

**Boundary with §2.3:** `when-numeric` is deliberately a typed component field,
not an MF2 message, even though both resolve locale text. The dividing line is
what is being decided, not what text results. §2.3's `message:` messages are a
**total function over closed, structural enums** (reference-type × carrier →
a fixed classification code) — textual realization of a selection that has
already been made elsewhere. `when-numeric` is a **conditional on the resolved
value's form** (numeric or not) — exactly the kind of structural selection
§2.3 already reserves for typed templates ("MF2 MUST NOT recreate general
template control flow"). A future style need MUST NOT reach for an MF2
message to express a presence or numeric conditional; that stays a typed
component field.

### §3 — Merge Operations (Formalized)

The engine MUST process each operation list (`modify`, `remove`, `add`) in the order provided. The order of these keys (`modify`, `remove`, `add`) within a variant has no semantic effect.

1.  **Identify Anchor (Match):** A `match` selector is a partial match: a component matches if it contains all key-value pairs specified in `match`, with equal values, regardless of any additional keys on the component.
    *   If no component matches, the operation MUST be ignored or treated as a validation error (implementation-defined, but validators SHOULD treat this as an error).
    *   If multiple components match, engines MUST treat this as a validation error or select the first match deterministically; style authors SHOULD avoid ambiguous `match` selectors.
2.  **Apply Operation:**
    *   `modify`: Overwrites rendering hints, and MAY clear inherited ones via `unset` (§5.1). If a `modify` operation attempts to change the component’s kind or primary value (e.g., `contributor: author` to `contributor: editor` or `variable: publisher` to `variable: url`), the style is invalid and must be rejected by validators or ignored by non-validating engines (implementation-defined).
    *   `remove`: Deletes the anchor from the list.
    *   `add`: Inserts a new component `before` or `after` the anchor. An `add` operation MAY specify either `before` or `after`, but not both. If both are present, the style is invalid. If the anchor in `before`/`after` does not match any component, the engine MUST append the new component to the end of the list.

### §4 — Distributed Merging

Resolution (`try_into_resolved_with`) follows the URI chain. Resolution is recursive: if the parent style itself `extends` another style, the engine MUST fully resolve that ancestor chain before applying the child’s diffs.

When a child style `extends` a remote parent:
1.  Fetches and fully resolves the remote parent's templates.
2.  Applies the parent's `type-variants` (if any).
3.  Applies the child's `type-variants` diffs to the fully resolved parent template.

**Example: Subscriber Style (`university-apa.yaml`)**
```yaml
extends: https://hub.citum.org/styles/apa.yaml

bibliography:
  # No local 'template:' is defined; it is inherited from the parent.
  type-variants:
    article-journal:
      # Inherits the article-journal from APA, then adds a localized label:
      add:
        - before: { variable: doi }
          component: { term: doi, suffix: ": " }
```

Engines SHOULD treat unreachable or invalid parent URIs as resolution errors; style authors MUST NOT assume offline resolution if remote parents are unavailable.

### §5 — Variant Reuse Without Duplication

§1–§4 give a diff model but leave two shapes inexpressible, and authors have
been paying for both by pasting whole templates. This section closes them
**without** relaxing the macro prohibition: neither addition introduces a named
fragment that a template can call, and neither is reachable from a template
body. They act only on the variant-resolution graph, which already exists.

#### 5.1 Clearing an inherited field (`unset`)

A `modify` operation can set a rendering field but not clear one. `Rendering`
fields are all `Option`, and the merge assigns only where the incoming value is
`Some`, so there is no value an author can write that means "the parent set
this; I don't want it."

The practical consequence is severe. A base template whose `title: primary`
carries `wrap: { punctuation: quotes }` cannot be specialized into a variant
that renders the title bare. The author's only recourse is `remove` + `add`,
which restates the component *and its position* — reintroducing exactly the
fragility the diff model exists to remove, because a later change to the base
template's ordering silently stops flowing through.

A `modify` operation MAY therefore carry `unset`, a list of rendering field
names to clear on the anchor:

```yaml
type-variants:
  book:
    modify:
      - match: { title: primary }
        unset: [wrap]
```

Semantics:

- Field names use the schema's kebab-case rendering vocabulary: `text-case`,
  `emph`, `quote`, `strong`, `small-caps`, `vertical-align`, `prefix`,
  `suffix`, `wrap`, `suppress`, `initialize-with`, `name-form`,
  `strip-periods`, and `label-form` (the last valid only on a `number:`
  anchor, matching where `modify` already accepts it).
- An unrecognized field name is a validation error. It MUST NOT be silently
  ignored: a typo that quietly leaves inherited quotes in place is precisely
  the failure this operation exists to prevent.
- Clearing a field the parent never set is a **no-op, not an error**. A diff
  must stay valid when an upstream parent stops setting a field.
- Within one operation, `unset` applies **before** that operation's own
  rendering merge. `{ unset: [wrap], wrap: { punctuation: parentheses } }` is
  therefore a replace, not a self-cancelling no-op. Ordering across separate
  operations remains authored order, per §3.
- `unset` MUST NOT change the component's kind or primary value; the §3
  `modify` rule applies unchanged. `unset: [title]` is invalid.

Alternative considered and rejected: an explicit YAML `null` (`wrap: null`).
Serde cannot distinguish an absent key from an explicit null on `Option<T>`
without lifting every field to `Option<Option<T>>` or hand-writing a
deserializer for `Rendering` — a large, invasive change to the most-touched
struct in the schema, in exchange for a spelling that reads as ambiguous
(`prefix: null` vs `prefix: ""`). `unset` is self-describing, additive, costs
one new field on the modify operation, and leaves `Rendering` untouched.

#### 5.2 Abstract variants (`abstract-variants`)

`extends` can only name something that is itself a reference type. When several
types share a specialization, the author must either repeat it or pick one
arbitrary type as the donor. `ieee.yaml` does the latter today —
`entry-encyclopedia` extends `broadcast`, which asserts a relationship between
encyclopedia entries and broadcasts that does not exist. Nothing in the file
records that both are really "a standalone work, whose title is not quoted."

A section MAY therefore declare `abstract-variants`, a sibling map to
`type-variants` holding named, non-type bases:

```yaml
bibliography:
  abstract-variants:
    standalone-work:
      modify:
        - match: { title: primary }
          unset: [wrap]

  type-variants:
    [broadcast, dataset, report]:
      extends: standalone-work
    book:
      extends: standalone-work
      modify:
        - match: { variable: publisher-place }
          prefix: ". "
```

Semantics:

- An `abstract-variants` entry has the same value shape as a `type-variants`
  entry: a full template or a diff. Omitting `extends` implicitly extends the
  section's base `template`, exactly as in §1.
- Its key is a **name, not a type selector**. It is never compared against a
  reference type, so it can never be selected for rendering. Type-name
  validation (`unknown reference type "…"`) MUST NOT fire for these keys.
- `extends: X` resolves in order: local `type-variants[X]`, local
  `abstract-variants[X]`, inherited `type-variants[X]`, inherited
  `abstract-variants[X]`. A name defined in both local maps is a validation
  error rather than a silent precedence win.
- An abstract variant MAY extend another abstract variant. Cycles are detected
  by the same mechanism as type-variant cycles and are a resolution error.
- Abstract variants are a **resolution-time construct only**. A fully resolved
  style contains concrete `type-variants` and no `abstract-variants`; the
  renderer never sees them.
- An abstract variant that nothing extends is dead weight, not an error.
  Linters SHOULD warn.
- Under §4, a child style MAY extend a remote parent's abstract variant, and
  abstract variants merge by name on the same terms as type-variants.

Alternative considered and rejected: a reserved sigil inside the existing
`type-variants` key space (`_standalone-work`). It keeps one map but forces
`TypeSelector` to mean two different things depending on a leading character,
and every consumer — `matches`, `unknown_type_names`, validation, lint, the
JSON schema — needs a carve-out. A separate map keeps `TypeSelector` meaning
"reference type" and requires no carve-outs.

#### 5.3 Worked example

`ieee.yaml` today spells out six near-identical bibliography variants. Measured
against its own base `template`, `personal_communication` is byte-identical;
`broadcast`, `dataset`, and `report` are identical to each other and differ from
the base only in dropping the title's quote wrap; `book` and `motion-picture`
add one field each on top of that. Roughly 150 lines encode about four real
decisions.

Before (abridged — this shape repeats six times):

```yaml
type-variants:
  broadcast:
  - contributor: author
    form: long
    and: text
    name-order: given-first
    shorten: { min: 7, use-first: 1 }
  - title: primary
  - title: parent-monograph
    emph: true
  # … nine more components, copied verbatim from the base template …
```

After:

```yaml
abstract-variants:
  standalone-work:
    modify:
      - match: { title: primary }
        unset: [wrap]

type-variants:
  [broadcast, dataset, report]:
    extends: standalone-work
  book:
    extends: standalone-work
    modify:
      - match: { variable: publisher-place }
        prefix: ". "
  entry-encyclopedia:
    extends: standalone-work
    modify:
      - match: { number: pages }
        label-form: short
  # `personal_communication` is deleted: it was the base template.
```

Each variant now states its one real difference, and a change to the base
template flows into all of them.

#### 5.4 What remains forbidden

This section MUST NOT be read as reopening macros. Unchanged from §"Scope":

- There is still no named fragment a **template body** can invoke. Both
  additions act on the variant graph; neither is reachable from a component
  list.
- `abstract-variants` entries are diffs over one section's base template, not
  reusable component sequences. They cannot be shared across `citation` and
  `bibliography`, and they take no arguments.
- Resolution stays total and static: every variant is still derivable to a
  concrete template before rendering, with no runtime dispatch.

#### 5.5 Schema and tooling impact

- `TemplateModifyOperation` gains `unset: Vec<RenderingField>`, defaulting to
  empty and skipped when empty on serialize. `Rendering` is unchanged.
- Each template section gains
  `abstract-variants: Option<IndexMap<String, TemplateVariant>>`, skipped when
  absent. `TypeSelector` is unchanged.
- `just schema-gen` regenerates `docs/schemas/style.json`; `unset` surfaces as
  an enum-constrained string array, `abstract-variants` as an object with
  free-form keys and the existing `TemplateVariant` value schema.
- Both fields are additive. Existing styles parse and resolve unchanged, so no
  style migration is required.
- `citum-migrate` SHOULD NOT start emitting either form on this change.
  Converter output is compared against a pinned corpus, and changing its shape
  would move migration-fidelity numbers for reasons unrelated to converter
  quality. Teaching the converter to factor shared specializations out is
  separate work, worth its own bean once hand-authored styles have exercised
  the syntax.

---

## Acceptance Criteria

- [x] Macros are absent from the spec.
- [ ] `type-variants` schema supports `extends`, `modify`, `add`, and `remove` with defined matching and ordering semantics.
- [ ] A `modify` operation can clear an inherited rendering field without restating the component's position (§5.1).
- [ ] Several reference types can share one specialization through a named non-type base, without one of them being designated the arbitrary donor (§5.2).
- [ ] A fully resolved style contains no `abstract-variants`, and abstract names never reach reference-type validation.
- [ ] Style-level `options` expanded to handle contributor and date formatting policies, with clear precedence rules vs local component hints.
- [ ] Engine resolution logic supports cross-URI template diffing, including recursive parent chains and error handling for missing parents.

---

## Changelog

- v0.6 (2026-08-05): Add §5 — `unset` on `modify` for clearing an inherited
  rendering field, and `abstract-variants` for named non-type bases. Both act
  on the variant graph only; the macro prohibition is restated in §5.4.
- v0.5 (2026-07-15): Clarify family inheritance, forbid named cross-section
  fragments, define authoritative date-fallback omission semantics, and allow
  narrowly scoped style-owned MF2 messages for textual classification.
- v0.4 (2026-06-24): Add `message:` components for locale-authored MF2
  phrase realization, including grouped argument sources, and deprecate
  template `term:` as the long-term phrase realization surface.
- v0.3 (2026-05-05): Clarified terminology, matching semantics, order of operations, and validation rules. Added subscriber style example using localized terms instead of literal affixes.
- v0.2 (2026-05-05): Pivoted to Pure Diff model. Removed Macros/Named Templates. Expanded role of style-level options.
- v0.1 (2026-05-05): Initial draft (Macro-based).
