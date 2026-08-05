---
# csl26-q4g5
title: 'type-variant ''all'' selector shadows later variants: 16 dead in elsevier-with-titles'
status: completed
type: bug
priority: high
tags:
    - engine
    - style
    - fidelity
created_at: 2026-08-05T13:52:35Z
updated_at: 2026-08-05T18:20:54Z
parent: csl26-ccdt
---

`resolve_type_variant` (crates/citum-engine/src/processor/rendering/grouped/component_predicates.rs:25) is `iter().find_map(...)` over an insertion-ordered IndexMap: **first authored match wins, with no specificity ordering anywhere in the engine**. `TypeSelector::matches` treats `all` as matching every reference type (template.rs:442).

## Evidence (2026-08-05)

`elsevier-with-titles-core.yaml` is the only embedded style using `all`. Its own comment states the rule correctly:

> `all` matches every type and is resolved first (engine returns the first matching selector), so any type-specific variant must precede it to take effect.

The file then violates it. `all` is at line 109; **16 type-variant keys are authored after it and are unreachable**: article-magazine, article-newspaper, book, chapter, dataset, interview, legal-case, paper-conference, patent, report, thesis, webpage, [webpage, dataset], [article-newspaper, broadcast], motion-picture, personal-communication.

Confirmed from rendered output, not inferred: ITEM-14 is a book, but renders with the `all` variant's editor-label shape (`(Eds.)` from `prefix: " ("`, `suffix: ")"`, `capitalize-first`) rather than the `book` variant's. Style sits at **27/67 exact parity**.

## Decision needed first

Specificity-based precedence (exact type > multi-type key > `all`) vs. keeping authored-order and just reordering the style file. Spec decision is being made in the TEMPLATE_V3 revision on PR #1142.

## Why this is not a quick fix

Changing precedence makes 16 variants live at once; elsevier-with-titles' 27/67 will move, possibly a lot, in either direction. Needs its own PR and a full embedded sweep, not a fold-in.

## Todo

- [ ] Land the precedence semantics in TEMPLATE_V3 (PR #1142)
- [ ] Implement specificity ordering in resolve_type_variant, or reorder the style if the decision goes the other way
- [ ] Lint: flag any type-variant authored after `all` under authored-order semantics
- [ ] Full embedded sweep before/after; record elsevier-with-titles delta

## Summary of Changes

Redirected: the bean framed this as a precedence problem needing a
specificity-vs-authored-order decision. The section `template` is already the
wildcard — it renders for unmatched types and is the implicit parent of every
variant omitting `extends` — so `all` was a redundant second wildcard that
could only shadow. Removed it instead of deciding precedence for it.

- elsevier's `all` body differed from its section template by three lines of
  contributor initialization that `options.contributors: numeric-given-dot`
  already supplied. Deleting it unshadowed 16 variants: **exact parity
  27/67 → 35/67**, 8 gained, 0 lost, no fidelity change.
- `all` removed from `validate_type_name` and `TypeSelector::matches`.
  `default` retained — it names a reference type rather than matching all.
- Rejecting the removed name is the schema's job, and the schema could not do
  it: `propertyNames` appeared nowhere in any of the eight published schemas,
  so every map-keyed vocabulary was an open string space. Added a real
  `JsonSchema` impl for `TypeSelector` (it had been publishing externally-tagged
  `{"Single": …}` objects that did not describe the authored YAML) plus
  `propertyNames` on the four reference-type maps.
- That exposed a live vocabulary drift: `entry`, `entry-encyclopedia`,
  `post-weblog` and `preprint` render correctly but were absent from
  `KNOWN_REFERENCE_TYPE_NAMES`, so styles using them warned wrongly. Added, with
  a test asserting `classified_ref_types() ⊆ KNOWN_REFERENCE_TYPE_NAMES`.
- Authored selectors are no longer underscore-normalized, so the engine and the
  hyphenated schema enum accept the same spellings. Incoming reference data is
  still normalized. `citum-migrate` was emitting the underscore spellings
  (`template_compiler/types.rs`) and a dead `legislation` selector; both fixed
  at the source, then in the six embedded and five tracked styles.
- `scripts/validate-schemas.js` had zero callers. Wired into `just
  schema-validate` and CI for styles and locales, with a new `styles` path
  filter so a style edit actually triggers it.

Follow-ups: [[csl26-qr1h]] (nine remaining propertyNames vocabularies),
[[csl26-xrom]] (dead delimiter_suppressing_terminal_marks config).
