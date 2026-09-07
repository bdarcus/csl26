# Render-When Contract Specification

**Status:** Active (vocabulary frozen — see v1.2)
**Version:** 1.2
**Date:** 2026-09-06
**Supersedes:** None
**Related:** `csl26-qyub`, `csl26-h8ja`,
`docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md`,
`docs/specs/ALTERNATIVES.md`

## Purpose

`render-when` is a field-presence conditional on `TemplateGroup`. A group
carrying `render-when` renders only when the reference data matches the
condition; otherwise it and everything inside it is skipped and claims no
variables. This spec defines its wire contract, field vocabulary, evaluation
semantics, and validation rules.

The mechanism is intentionally bounded: independent `field-present` and
`field-absent` probes combined with AND only. No OR, value comparisons, or
arbitrary boolean expressions.

**As of v1.2, the field vocabulary below is frozen — no further fields will
be added.** A full-corpus inventory (`docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md`)
found that every one of the 125 existing uses is one of two shapes: a
fallback where the tested field is the same one the branch renders (now
served by `docs/specs/ALTERNATIVES.md`'s ordered-candidate-list primitive,
which needs no predicate at all), or a structural policy gate where the
tested field never appears in what it guards (not yet served by anything —
see that audit's "Work-form routing" section). New style needs that look
like a `render-when` field addition should be routed to one of those two
efforts, not to this contract.

## Scope

In scope:

- the wire contract for `TemplateGroup.render_when`,
  `TemplateGroupCondition`, and `TemplateConditionField`;
- the typed field vocabulary and its extension rules;
- validation and nesting semantics.

Out of scope:

- value comparisons, disjunction, arbitrary boolean expressions, or
  CSL-style `choose`/`if`/`else` control flow;
- migration emission — `citum-migrate` does not emit this feature, and this
  spec does not make it a migration target.

## Design

### Wire contract

`render-when` is valid only on `TemplateGroup`:

```yaml
- group:
  - contributor: author
  - contributor: recipient
    prefix: " to "
  render-when:
    field-present: recipient
    field-absent: title
```

`field-present` and `field-absent` are each optional, but at least one must
be supplied. When both are supplied they combine with AND: the group renders
only when the first field is present and the second is absent.

Conditions are evaluated from reference source accessors before the group
renders and before it can claim variables. They do not inspect formatted
text, substitution results, or whether another component already consumed a
value.

### Field vocabulary

| Field | Presence source |
|---|---|
| `author` | primary author contributor exists |
| `editor` | editor contributor exists |
| `recipient` | recipient contributor exists |
| `translator` | translator contributor exists |
| `title` | primary title accessor returns a value |
| `collection-title` | collection-title accessor returns a value |
| `issued` | effective issued date exists |
| `original-published` | original publication date exists |
| `publisher` | publisher string exists |
| `original-publisher` | original publisher string exists |
| `original-publisher-place` | original publisher place exists |
| `original-title` | original title exists |
| `doi` | DOI exists |
| `genre` | genre exists |
| `archive` | archive or repository name exists |
| `archive-location` | archive location or shelfmark exists |
| `volume-or-issue` | the volume number, or the issue number when volume is absent — "does this serial component have any volume/issue identifier at all?" |
| `part-number` | the document-level part number used by multivolume and multipart works |
| `part-number-numeric` | a document-level part number whose value is a bare numeric value |
| `part-number-non-numeric` | a document-level part number whose value already contains a textual label |
| `number-of-volumes` | the total number of volumes in a multivolume work |
| `volume-title` | the title of an individual volume within a multivolume work |

**Frozen as of v1.2 — no new fields will be added to this table.** The
extension criteria that governed v1.0/v1.1 growth are recorded here for
history:

- presence has one unambiguous, documented reference accessor meaning;
- a real style forcing case needs it, and the need is a field-presence
  layout or value selection within one reference type — not a stand-in for
  a distinction an option, preset, or type-variant should own instead;
- schema parse, present, absent, and combined-condition tests cover it;
- engine behavior stays generic and does not inspect style identity;
- this contract and generated schema documentation are updated.

The 2026-09-06 disposition audit found that every candidate field a wave-3
parity pass wanted (`url`, `pages`, `publisher-place`) failed the second
criterion on inspection — each was a stand-in for a fallback that
`docs/specs/ALTERNATIVES.md` now serves declaratively. Rather than
re-litigate that same question per proposal, the vocabulary is closed: a
proposed new field is evidence that a need belongs to `alternatives:` or to
work-form routing (see the audit), not to this contract.

Field growth does not imply operator growth. Multiple-field lists, OR,
comparisons, arbitrary expressions, and new branch forms each require a
separate design proposal, and remain out of scope regardless of the freeze.

### Validation

Style validation rejects:

- `render-when: {}`, an unconditional no-op;
- the same field in both `field-present` and `field-absent`, which can never
  match;
- `render-when` on any component other than a group — already impossible in
  the typed schema.

### Nesting

Conditioned groups may nest. Each condition is evaluated independently
before its own group renders; a suppressed group claims no variables.

## Implementation Notes

Validation lives in `TemplateResourceBudget::check_component`
(`crates/citum-schema-style/src/style/validation.rs`), reached through
`Style::from_yaml_str`. Rejection tests are in
`crates/citum-schema-style/src/tests.rs`
(`style_loader_reports_empty_render_when`,
`style_loader_reports_contradictory_render_when`). Behavior tests for
present, absent, combined-AND, and nested evaluation are in
`crates/citum-engine/tests/bibliography.rs`.

The empty/same-field constraint is not expressible in the generated JSON
Schema (`schemars` has no cross-field `not`/`oneOf` for this shape); `just
schema-gen` was run and produced no diff, which is expected, not an omission.

`citum-migrate` continues to not emit `render-when`.

## Acceptance Criteria

- [x] Schema validation rejects empty and same-field present/absent
      conditions.
- [x] Behavior tests cover present, absent, combined-AND, and nested cases.
- [x] `just schema-gen` run; no diff, since the constraint isn't
      schema-expressible.
- [x] Status promoted to Active in the implementation commit.

## Changelog

- v1.2 (2026-09-06, corrected same day): Froze the field vocabulary — no new
  fields will be added. A-shape (fallback) uses route to
  `docs/specs/ALTERNATIVES.md`; B-shape (structural policy) uses await a
  work-form-routing design under `csl26-40n4`. See
  `docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md`.
  A Codex adversarial review of that freeze found the table itself was
  incomplete — six existing `TemplateConditionField` variants
  (`volume-or-issue`, `part-number`, `part-number-numeric`,
  `part-number-non-numeric`, `number-of-volumes`, `volume-title`, the
  corpus's *highest-volume* fields) had never been added to this table in
  v1.0/v1.1. Added them same-day, before this freeze could be read as
  documenting a complete vocabulary that it didn't. See `csl26-8b4a`.
- v1.1 (2026-07-13): Implemented validation and behavior tests; promoted to
  Active.
- v1.0 (2026-07-13): Initial contract specification.
