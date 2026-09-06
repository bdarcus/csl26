# Alternatives Specification

**Status:** Draft
**Version:** 1.0
**Date:** 2026-09-06
**Supersedes:** None
**Related:** `docs/specs/RENDER_WHEN_CONTRACT.md`,
`docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md`, `csl26-h8ja`,
`csl26-x79y`, `csl26-zs9y`

## Purpose

`alternatives:` is an ordered list of template components on a template
position. The first one that renders non-empty output wins; the rest are not
evaluated. It replaces the "fallback" half of `render-when` usage — cases
where a style tries one rendering, and failing that, another — with a
primitive that has no predicate at all: no field-presence test, no boolean
expression, just an ordered set of things to try.

This generalizes a pattern the schema already has three narrower versions of:
`Substitute.candidates` (contributor role fallback), `DateFallbackCandidate`
(issued-date fallback), and `ArticleJournalNoPageFallback` (a single
hardcoded DOI fallback for one reference type). See the companion decision
record for the evidence that motivated extracting the general case instead of
adding a fourth narrow one.

## Scope

In scope:

- the wire contract for a new `TemplateComponent::Alternatives` variant;
- evaluation order and "did this render" semantics, reusing the engine's
  existing group-suppression rule;
- interaction with `Substitute` and `DateFallbackCandidate` (naming, not
  merging);
- validation rules.

Out of scope:

- predicates, field-presence tests, or any condition on source data —
  `render-when` continues to own that, frozen at its current vocabulary (see
  `RENDER_WHEN_CONTRACT.md` v1.2);
- the structural work-form routing problem (`volume-or-issue`,
  `part-number-numeric` / `part-number-non-numeric` editor/container
  routing) — that is a separate, not-yet-designed primitive; see the decision
  record's "Work-form routing" section;
- migrating `Substitute` or `DateFallbackCandidate` onto this primitive —
  both carry role- or date-specific semantics (substitution slot formatting,
  message-vs-date-vs-variable candidate kinds) that a bare component list
  does not model. They stay as they are.
- `citum-migrate` emission — a first migrate target is plausible (see
  "Future migration target" below) but is not part of this spec.

## Design

### Wire contract

```yaml
- alternatives:
  - variable: volume-title
    emph: true
  - title: primary
```

Reads as: *render the volume title; if that produces nothing, render the
primary title instead.* Compare to today's `render-when` encoding of the same
rule, which needs two separate groups and a repeated field name:

```yaml
- group:
  - variable: volume-title
    emph: true
  render-when:
    field-present: volume-title
- group:
  - title: primary
  render-when:
    field-absent: volume-title
```

`alternatives:` is valid as a top-level template item, inside a `group`, and
inside `type-variants`/`bibliography.template` entries — anywhere a
`TemplateComponent` is valid today. Each entry in the list is itself a full
`TemplateComponent` (a leaf variable, a group, a message, another
`alternatives:` block, etc.) with its own `Rendering` (prefix/suffix/wrap/etc).

### Evaluation

1. Evaluate list entries in order.
2. An entry "renders" if its component-level `values()` computation returns
   non-empty output — the same rule `TemplateGroup` already applies at
   `crates/citum-engine/src/values/list.rs:70` (a group counts only
   non-term content; a literal-only or term-only render does not count as
   content). `alternatives:` reuses this rule rather than defining a second
   one.
3. The first entry that renders wins. Its output, prefix, and suffix are used
   as-is. No further entries are evaluated (no side effects to worry about,
   but this also bounds cost: a long alternatives list does not evaluate
   every branch on every reference).
4. If no entry renders, the `alternatives:` component itself renders nothing
   — same as a `group` with no content, so it is invisible to surrounding
   delimiter/join logic.

Unlike `render-when`, there is no notion of "field absent" to check up front:
the mechanism is purely "try, then try the next," discovered from actual
output rather than declared in advance. This is why it cannot express the
B-shape (structural policy) uses found in the disposition audit — those
require knowing *which* branch to pick before rendering anything, based on a
property that never appears in the rendered output at all.

### Validation

Style validation rejects:

- `alternatives: []`, an empty list (no-op, same rule as
  `render-when: {}`);
- `alternatives:` with exactly one entry (write the component directly
  instead — a single-entry list has no fallback behavior to express).

### Relationship to existing candidate-list types

`alternatives:` is a template-level primitive. `Substitute.candidates` and
`DateFallbackCandidate` are options-level and carry semantics specific to
their domain (a substituted editor inherits the author slot's name
formatting and sort position; a date fallback distinguishes date-shaped,
message-shaped, and variable-shaped candidates). `alternatives:` does not
replace them — it is the shape underneath all three, available directly in
templates for cases that are not contributor substitution or date fallback.

`ArticleJournalNoPageFallback::Doi` (`options/bibliography.rs:136`) is a
plausible future deprecation target once `alternatives:` ships: NLM's
"article detail block, else DOI" rule (`csl26-h8ja`'s wave-3 motivation) is
naturally `alternatives: [{<detail block>}, {variable: doi, prefix: "doi:"}]`,
which generalizes the one-off enum to any style, not just
`no-page-fallback: doi` on `article-journal`.

### Rejected: an options-level construct instead of a template component

The three precedents this spec generalizes are all options-level
(`Substitute`, `DateFallbackCandidate`, `ArticleJournalNoPageFallback`), so
the natural question is why this isn't a fourth one — say,
`bibliography.options.fallbacks: { volume-title: [...] }` — instead of a new
`TemplateComponent` variant.

The precedents work as named options because each anchors to exactly **one
semantic slot that exists in every reference regardless of style**: *the*
contributor position, *the* issued date, *the* article-journal detail block.
An option can afford to skip saying "where in the template" because there is
only one place it could mean.

The 49 A-shape uses this spec targets have no such single slot. `volume-title`
vs `title: primary` only matters inside Chicago's multivolume-chapter
shape; `collection-title`, `recipient`, `archive-location`, `original-title`,
and `publisher` each guard a different, unrelated position, found in a
different type-variant, with different surrounding prefix/suffix/emphasis
that belongs to that exact spot in that exact template. An options table
keyed by field name would need to smuggle back in everything a template
position already carries (which type-variant, what delimiter joins it to its
neighbors, what emphasis this specific style wants) — at which point it is a
template fragment wearing an options key, not a genuine cross-cutting policy.

The deciding cost, not just the modeling awkwardness: treating each field as
its own named option recreates exactly the one-off proliferation
`RENDER_WHEN_CONTRACT.md`'s extension criteria were trying to close off — a
new Rust type for every field a style author discovers needs a fallback,
forever. A template-level primitive is the one that lets a style express a
new fallback with no schema change at all.

### Future migration target

`RENDER_WHEN_CONTRACT.md` states that `citum-migrate` does not emit
`render-when`. `alternatives:` is a plausible target for future migrate work:
a CSL `<choose><if variable="X">…rendering X…<else>…</else></choose>` where
`X` appears inside the `if` branch is exactly the A-shape pattern this spec
covers. That is future work, not part of this spec's acceptance criteria.

## Implementation Notes

Expected shape: a new `TemplateComponent::Alternatives(TemplateAlternatives)`
variant, `TemplateAlternatives { alternatives: Vec<TemplateComponent> }`,
alongside the existing `Group`, `Variable`, `Message`, etc. variants in
`crates/citum-schema-style/src/template.rs:625`. Evaluation lives in
`crates/citum-engine/src/values/`, sharing the "does this render" test
`TemplateGroup::values` already implements
(`crates/citum-engine/src/values/list.rs:15-72`) rather than duplicating it.

## Acceptance Criteria

- [ ] Schema: `TemplateComponent::Alternatives` variant, validated (empty and
      single-entry rejected).
- [ ] Engine: evaluation order, first-non-empty-wins, reusing group's
      "non-term content" rule.
- [ ] Behavior tests: first entry renders, first entry empty falls through,
      all entries empty renders nothing, nested `alternatives:` inside
      `group` and vice versa.
- [ ] `just schema-gen` run, schema docs updated.
- [ ] At least one embedded style migrated as a worked example (candidate:
      T&F-NLM's DOI-fallback rule, or Chicago's volume-title fallback) with a
      report-core.js diff showing 0 regressions.
- [ ] Status promoted to Active in the implementation commit.

## Changelog

- v1.0 (2026-09-06): Initial draft.
