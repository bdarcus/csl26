# Citum Data Model

This is a conceptual tour of Citum's native bibliographic data model —
`InputReference` and its supporting types in `crates/citum-schema-data`. It
explains the shape of the model and why it looks the way it does; field-level
detail lives in the generated reference pages this document links out to.

**Status:** descriptive reference, not a normative spec. It carries no
stability promise beyond the rules in [SCHEMA_VERSIONING.md](SCHEMA_VERSIONING.md).

## Ingest architecture

Citum accepts three input formats today, with different hop counts to the
native model:

```
BibLaTeX (.bib)  ─────────────────────────────►  InputReference
CSL-JSON          ──►  csl_legacy::csl_json::Reference  ──►  InputReference
RIS               ──►  csl_legacy::csl_json::Reference  ──►  InputReference
```

BibLaTeX maps directly to `InputReference`: `crates/citum-refs/src/formats/biblatex/`
dispatches on BibLaTeX entry type via the declarative tables in `tables.rs`
(see [BIBLATEX_MAPPING.md](BIBLATEX_MAPPING.md)) and builds the target struct
without an intermediate representation.

CSL-JSON and RIS both convert through `csl_legacy::csl_json::Reference` first —
CSL-JSON because that *is* its native shape, and RIS because the existing RIS
reader already emits `csl_legacy` structures rather than a dedicated RIS
intermediate type. `citum-schema-data`'s `reference::conversion` module then
maps that legacy shape onto `InputReference` (see
[generated/CSL_JSON_MAPPING.md](generated/CSL_JSON_MAPPING.md) and
[CSL_TYPE_CONVERSION_CONTRACT.md](../specs/CSL_TYPE_CONVERSION_CONTRACT.md) for
the type-level rules that hop follows). A dedicated RIS mapping reference is
not currently documented.

## Reference classes

`InputReference` is a class-discriminated union (see
[INPUT_REFERENCE_CLASS_DISCRIMINATOR.md](../specs/INPUT_REFERENCE_CLASS_DISCRIMINATOR.md)
for the discriminator's exact deserialization mechanics). Its variants split
into two families:

- **Structural classes** — `Monograph`, `Collection`, `CollectionComponent`,
  `Serial`, `SerialComponent`. These model bibliographic structure generically:
  a component belongs to a container (`Collection` or `Serial`), and that
  relationship is what the renderer walks to build citations and
  bibliography entries. Most everyday references (books, journal articles,
  book chapters) are one of these five.
- **Flat classes** — `LegalCase`, `Statute`, `Treaty`, `Hearing`, `Regulation`,
  `Brief`, `Classic`, `Patent`, `Dataset`, `Standard`, `Software`, `Event`,
  `AudioVisual`. These model domains (law, patents, datasets, software,
  events, AV works) whose citation conventions don't decompose into the
  generic container/component relationship — a patent's defining fields are
  `patent-number` and `jurisdiction`, not a place in a monograph/collection
  hierarchy.

Both families exist because forcing every domain into the structural model
would either lose domain-specific fields (a legal case's `reporter`/`volume`/
`page` triad isn't a `Numbering`) or bloat the structural classes with fields
that only apply to one domain. See
[TYPE_SYSTEM_ARCHITECTURE.md](../specs/TYPE_SYSTEM_ARCHITECTURE.md) for the
original design rationale (still `Status: Draft`) and
[TYPE_ADDITION_POLICY.md](../policies/TYPE_ADDITION_POLICY.md) for when a new
domain earns a new flat class versus reusing an existing one.

Full field tables for every class: [generated/DATA_MODEL_FIELDS.md](generated/DATA_MODEL_FIELDS.md).

## The `class` discriminator

Every `InputReference` carries a `class` field (e.g. `"monograph"`,
`"patent"`) that determines which struct its other fields deserialize into.
Unknown `class` values degrade to `ReferenceClass::Unknown` rather than
failing to parse — the forward-compatibility contract this depends on is
covered below. Full mechanics:
[INPUT_REFERENCE_CLASS_DISCRIMINATOR.md](../specs/INPUT_REFERENCE_CLASS_DISCRIMINATOR.md).

## Containers: `WorkRelation`

Structural classes relate to their containers (and other classes relate to
associated works) through `WorkRelation`, which is untagged over two shapes:

- `WorkRelation::Id(RefID)` — the target is referenced by its id, resolved
  against the rest of the bibliography at render time.
- `WorkRelation::Embedded(Box<InputReference>)` — the target is inlined
  directly, with no separate entry required.

This is used for `container` (the immediate parent — a `Collection` for a
`CollectionComponent`, a `Serial` for a `SerialComponent`) and for associative
relations like `original` (e.g. a translation's source text) and `reviewed`
(the work a review covers). See
[GENERALIZED_RELATIONAL_CONTAINER_MODEL.md](../specs/GENERALIZED_RELATIONAL_CONTAINER_MODEL.md)
for the full design and [NATIVE_FORMAT.md](NATIVE_FORMAT.md) for worked
examples of both shapes.

**BibLaTeX prior art.** BibLaTeX has no single equivalent of `WorkRelation`;
the closest analogues are three different mechanisms, all resolved before
Citum's mapping code ever sees the entry:

- `crossref`/`xdata` — resolved and spliced by the `biblatex` crate's parser
  itself (`Bibliography::parse`), so fields inherited from the referenced
  entry are already merged into `entry.fields` by the time extraction runs.
  Citum never constructs a `WorkRelation` for these.
- `related`/`relatedtype` — *not* resolved by the crate, and the closest
  actual prior art to `WorkRelation`: a typed link (`multivolume`, `origpub`,
  `reprint`, `translationof`, `reviewof`, …) to another entry. This is
  currently in [BIBLATEX_MAPPING.md](BIBLATEX_MAPPING.md)'s "Not Yet Mapped"
  section — see `docs/specs/ORIGINAL_PUBLICATION_RELATION_SUPPORT.md`.

## Contributors

A contributor can be a plain string (`SimpleName`, for corporate/organization
names), a `StructuredName` (family/given/particle/suffix parts), a
`MultilingualName`, or a `ContributorList` of any of those — this union is
`Contributor`, used for the `author`/`editor`/`translator` shorthand fields
still present on structural classes.

The canonical representation, though, is `contributors: Vec<ContributorEntry>`
— each entry pairs a `Contributor` with an explicit `ContributorRole`
(`author`, `editor`, `translator`, `director`, …), so a reference can carry
an arbitrary mix of roles without one field per role. `normalize_contributors`
reconciles the two representations: when a reference is constructed or
converted with only the shorthand fields set, it folds them into
`contributors` so downstream code (the renderer, disambiguation, sorting) has
one canonical list to read regardless of which shape the input used.

## Dates, titles, rich text, forward compatibility

- **Dates** — `DateValue` wraps an EDTF string (plus an optional `note`),
  distinguishing `created` (origination) from `issued` (formal publication)
  and other date roles. Full model: [DATE_MODEL.md](../specs/DATE_MODEL.md).
- **Titles** — `Title` is either a plain string (`Title::Single` in effect,
  via the untagged union) or a `StructuredTitle` with a separate `Subtitle`,
  plus multilingual variants. Structured titles exist so styles can render
  main title and subtitle with different punctuation/casing rules rather
  than treating the whole thing as one opaque string.
- **Rich text** — `note` and `abstract-text` (and title fields) use
  `RichText`, which accepts a plain string or a `{ djot: "..." }` object.
  Djot is Citum's inline markup dialect for these fields; see
  [DJOT_RICH_TEXT.md](../specs/DJOT_RICH_TEXT.md).
- **Forward compatibility** — an unrecognized `class` or enum value degrades
  to an `Unknown(String)` variant rather than a hard parse failure
  (`ReferenceClass::Unknown`, `UnknownClassData`), so a newer-schema document
  opened by an older engine build loses fidelity gracefully instead of
  refusing to load. See [FORWARD_COMPATIBILITY.md](../specs/FORWARD_COMPATIBILITY.md)
  for the full SoftDegrade/HardFail contract this follows.

## Prior art: BibLaTeX §2.2.1 datatypes

BibLaTeX's manual (§2.2.1) defines a closed vocabulary of field datatypes
(`literal`, `name`, `key`, `date`, `range`, `verbatim`, `uri`, …), used
throughout [BIBLATEX_MAPPING.md](BIBLATEX_MAPPING.md)'s field table. Citum's
scalar types don't map one-to-one, but the correspondence is close enough to
be a useful grounding reference:

| BibLaTeX §2.2.1 datatype | Closest Citum scalar type | Notes |
|---|---|---|
| `literal` | `RichText` / `String` | BibLaTeX literal fields are plain text with citeproc/LaTeX markup; Citum's `RichText` adds a typed Djot variant on top. |
| `name` | `Contributor` | BibLaTeX names are `Person` structs (family/given/prefix/suffix); Citum's `StructuredName` mirrors that shape directly. |
| `key` | closed-vocabulary enum (e.g. `MonographType`) | BibLaTeX keys are freeform strings matched case-insensitively against a controlled list; Citum uses typed enums with `tolerant_enum!`-style forward compatibility instead. |
| `date` | `DateValue` (EDTF) | BibLaTeX dates are ISO 8601-2/EDTF-like already; Citum's `DateValue` is EDTF directly. |
| `range` | `NumOrStr` / plain string | BibLaTeX ranges are typed (`Vec<Range<u32>>`); Citum currently stores page ranges as raw strings (see `pages` in [BIBLATEX_MAPPING.md](BIBLATEX_MAPPING.md)). |
| `verbatim` | `String` | Both treat these as opaque, unprocessed text (DOIs, ISBNs, URLs). |
| `separated-value` | `Vec<String>` (`keywords`) | Both split on a separator into a list; BibLaTeX defaults to comma-separated. |
| `entrykey` | `WorkRelation` (partial) | See the containers section above — only `related`/`relatedtype` is comparable prior art; `crossref`/`xdata` are resolved before Citum's mapping runs. |
| — | `Numbering` | No BibLaTeX equivalent: `Numbering` is Citum's typed volume/issue/number-kind identifier, used where BibLaTeX has several distinct flat fields (`volume`, `number`, `part`, `chapter`, …) for what is conceptually one "identifier within a container" concept. |
| — | `LangID` | Closest BibLaTeX equivalent is `langid`/`hyphenation`, a `key`-typed field; Citum's `LangID` is a validated BCP 47 tag. |
