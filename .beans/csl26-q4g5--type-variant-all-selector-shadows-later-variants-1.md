---
# csl26-q4g5
title: 'type-variant ''all'' selector shadows later variants: 16 dead in elsevier-with-titles'
status: todo
type: bug
priority: high
tags:
    - engine
    - style
    - fidelity
created_at: 2026-08-05T13:52:35Z
updated_at: 2026-08-05T13:52:51Z
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
