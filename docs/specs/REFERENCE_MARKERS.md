---
title: Reference Markers
status: Draft
created: 2026-08-04
---

# Reference Markers

## Summary

A **reference marker** is a processor-generated token that stands in for a full
reference at the point of citation: `[1]`, `[Kuh62]`. It is generated, not
authored — the style declares that it wants one and how it should look, and the
processor supplies the value from bibliography position or from name/date data.

Two of the regimes in [CITATION_REGIME](CITATION_REGIME.md) produce a marker:

| Regime | Marker | Value source |
|---|---|---|
| Numeric | `[1]` | Processor-assigned citation number |
| Label | `[Kuh62]` | Trigraph generated from names and year |

Two do not, and this spec says so normatively rather than leaving it open:

- **Author-date** (`Smith 2020`) is *template-composed*. Its primary key is built
  by rendering contributor and date components with et-al truncation, name
  disambiguation and date formatting. It is a small citation in its own right,
  not a token, and modelling it as a marker would buy nothing while forcing a
  lowest-common-denominator abstraction on the two regimes that do have tokens.
- **Note anchors** are an output-format contract. What varies across note output
  is *who owns the note container* — the word processor's field, TeX's
  `\footnote`, or the engine in djot/markdown — not what a marker looks like.
  See [Non-goals](#non-goals).

This spec gives markers a first-class representation in the render model and
removes the label component from the template surface entirely.

## Motivation

### How markers are expressed today

`citation.options.label-mode` (numeric, shipped) materializes a marker by
**cloning the effective template after type-variant resolution and inserting a
synthetic `TemplateComponent::Number` into the clone**. A style may also author
the same component directly as `number: citation-number`. Both spellings exist,
and the marker ends up as an AST node indistinguishable from ordinary authored
content.

So every downstream consumer must re-derive "is this a marker?" by
pattern-matching the AST. As of this writing that is ten helpers — six that
answer identity, three that apply or unapply presentation, and one flag threaded
through the render model:

| Kind | Helpers |
|---|---|
| Identity | `template_has_citation_number`, `strip_citation_numbers`, `template_is_numeric_label_only`, `template_has_label_presentation`, `template_has_bibliography_label`, `strip_bibliography_labels` (`processor/rendering/mod.rs`); `is_citation_number_component` (`processor/rendering/grouped/component_predicates.rs`) |
| Presentation | `apply_citation_label_wrap`, `apply_bibliography_label_wrap`, `clear_citation_label_presentation` (`processor/rendering/mod.rs`) |
| Flag | `ProcTemplateComponent::label_only` (`render/component.rs`), consumed by `render/bibliography.rs` |

### The cost, concretely

Consequences observed while extending this to a second marker kind (PR #1137,
closed unmerged):

- **Spacing became schema.** The synthetic marker is joined to the entry body by
  a group delimiter, so "space between marker and entry" had to surface as a new
  `bibliography.options.label-separator` option — schema surface that exists only
  to serve the injection mechanism.
- **Presentation is applied then undone.** Numeric markers must stay bare until
  collapse runs, so the code sets wrap/vertical-align and immediately clears it.
- **Two code paths compute marker identity**, and they disagreed: a template
  holding both marker kinds rendered both. Caught only in review.
- **Invariants live in comments.** `clear_citation_label_presentation` hardcodes
  `CitationNumber` while its sibling takes a variable; that is safe only because
  a caller three frames up gates on the numeric regime.

None of these are bugs in the marker feature. They are the shape of expressing
processor-owned semantics as authored-template data.

### The redundancy is already shipped

`ieee`, `american-medical-association` and `numeric-comp` each declare
`label-mode: numeric` **and** author a `number: citation-number` component. The
two spellings coexist in the corpus, and nothing tells an author which one wins.

## Design

### Marker model

A marker is a value plus its presentation. Nothing else.

```rust
enum MarkerValue {
    /// Numeric regime: the processor-assigned citation number, plus an
    /// optional compound-entry sub-label such as the `a` in `1a`.
    Number { number: usize, sub_label: Option<String> },
    /// Label regime: a generated trigraph such as `Kuh62`, disambiguation
    /// suffix already attached.
    Token(String),
}

struct Marker {
    value: MarkerValue,
    /// Wrap and vertical alignment around the marker alone (`label-wrap`).
    rendering: MarkerRendering,
    /// Wrap around the marker together with the item body (`item-wrap`).
    item_rendering: MarkerRendering,
    /// Text between the marker slot and the body it is joined to.
    separator: Separator,
}
```

`MarkerValue::Number` carries the integer, not a rendered string, so collapse
reads a number rather than parsing text.

### No template component ever denotes a marker

`NumberVariable::CitationNumber` and `NumberVariable::CitationLabel` are removed
from the template surface. There is exactly one way to ask for a marker: declare
`label-mode` on the citation or bibliography spec. Templates describe the body;
the processor owns the marker.

This is what makes the ten helpers deletable. A predicate that answers "is this
AST node a marker?" cannot exist when no AST node can be one.

`CitationCollapse::CitationNumber` is a distinct enum
(`style/sections/citation.rs`) and is unaffected; `collapse: citation-number`
remains the spelling for numeric collapse.

### Composition

The marker occupies a **slot** in the rendered item, not a position in a
template. It is placed leading in non-integral mode and trailing in integral
mode, and joined to the rest of the item body by the item delimiter.

Wrapping punctuation is what distinguishes shipped numeric styles from one
another, and it operates at **three distinct scopes**. Conflating them is why the
current mechanism needs a template group to express one of the three.

| Scope | Declared as | Encloses | Applied by |
|---|---|---|---|
| Cluster | `citation.wrap` | the whole assembled citation, all items | `apply_spec_wrap_and_affixes`, outside all per-item rendering |
| Item | `citation.options.item-wrap` | marker **and** item body (locator, and in integral mode nothing else) | the marker slot |
| Marker | `citation.options.label-wrap` | the marker alone | the marker slot |

The bibliography slot uses `label-wrap` (marker alone), joined to the entry body
by `label-separator`.

The three scopes are not interchangeable, and each is pinned by observed output
from a shipped style:

| Style | Declares | Single cite | Multi-cite |
|---|---|---|---|
| `ieee` | `item-wrap: brackets` | `[1, p. 737]` | `[1], [2], [3]` |
| `american-medical-association` | `label-wrap: brackets` | `[1](p737)` | `[1],[2],[3]` |
| `springer-vancouver-brackets` | `label-wrap: brackets` | `[1]` | `[1],[2],[3]` |
| `gb-t-7714-2025-numeric` | `citation.wrap: brackets` | `[1]` | `[1–3]` |
| `royal-society-of-chemistry` | neither | `1` | `1–3` |
| `alpha`, `american-mathematical-society-label` | `label-wrap: none` under a cluster bracket | `[ABC96]` | `[ABC96, DEF97]` |

IEEE and AMA are the discriminating pair: IEEE's locator renders **inside** the
bracket and AMA's **outside**. A single wrap knob cannot express both, which is
why `item-wrap` exists as a scope of its own rather than as a special case of
`label-wrap`.

Spacing is a property of the slot — `label-separator`, defaulting to empty — and
never a group delimiter. Empty is the correct default: citeproc-js
`second-field-align` output flattened to text is **flush** (`[1]J. Smith`, not
`[1] J. Smith`), and IEEE and AMA depend on it. A style that wants the space
declares it.

### Presentation is applied once

`label-wrap` and `item-wrap` resolve to `MarkerRendering` when the plan is built,
and are realized after collapse has run. There is no apply-then-undo step, because the
marker's presentation is never expressed as template data that some other pass
might read.

### Collapse

Collapse consumes `MarkerValue::Number` directly. An item is collapse-eligible
when its marker is numeric and its body is empty — that is, the marker is the
entire rendered item. This is a fact about the resolved slot, not about template
shape.

Collapse remains numeric-only. CSL's author-date collapse modes (`year`,
`year-suffix`, `year-suffix-ranged`) are unimplemented; the marker model must not
preclude them, but implementing them is out of scope.

### Disambiguation

Disambiguation keeps *deciding* — collision keys, group index, whether a
year-suffix applies — and the marker owns *rendering* the result. A trigraph's
`a` suffix is attached during marker-value generation, so `MarkerValue::Token`
arrives complete. No ownership moves. See [DISAMBIGUATION](DISAMBIGUATION.md).

## Non-goals

- **Note-container numbering.** Which output formats own note numbering, and how
  the engine signals "anchor here, you number it", is an output-format and FFI
  question, not a marker question. See
  [NOTE_STYLE_DOCUMENT_NOTE_CONTEXT](NOTE_STYLE_DOCUMENT_NOTE_CONTEXT.md).
- Modelling author-date primary keys as markers.
- Implementing CSL `second-field-align` (bean `csl26-qdff`).
- Implementing author-date collapse modes.
- Changing bibliography sort, grouping, or the regime-inheritance invariants in
  [CITATION_REGIME](CITATION_REGIME.md).
- Re-litigating the declarative `label-mode` surface. It is not the problem.

## Implementation notes

**Parity is the gate, continuously.** This is the highest-parity-risk area of the
engine. `node scripts/report-core.js --all-features` fidelity and exact-parity
must be green at every step, not only at the end; and `just check-core-quality`
must hold its baselines. A redesign that goes red for "just a few commits" in
this area is how it becomes a multi-week stall.

Byte-diffing rendered output against a `main` baseline worktree catches drift
that oracle percentages hide — the report-core matcher normalizes role labels and
tolerates differences. Both label styles were byte-identical through PR #1137;
that is the standard to hold.

Twelve shipped styles author a label component and must be converted in the same
change that removes the variables: `royal-society-of-chemistry`, `numeric-comp`,
`american-medical-association-alphabetical`, `alpha`,
`american-mathematical-society-label`, `ieee`, `american-medical-association`,
`elsevier-with-titles-core`, `springer-vancouver-brackets-core`,
`springer-basic-brackets-core`, `gb-t-7714-2025-numeric`,
`taylor-and-francis-national-library-of-medicine-core`.

`citum-migrate` must emit `label-mode` options rather than label components, so
a fresh conversion of a numeric or label CSL style produces the declarative form.

**Known limitation.** `TemplateComponent` is an untagged serde enum, which
discards the inner error, so authoring a marker reports only "data did not match
any variant of untagged enum TemplateComponent" rather than naming `label-mode`.
The reservation still holds — the style is rejected, not silently degraded.

## Acceptance criteria

- [ ] No template component can denote a marker: `NumberVariable::CitationNumber`
      and `CitationLabel` are gone from the schema, and the two names are
      reserved so they cannot degrade into `Custom` numbering kinds.
- [ ] The marker is a value in the render model, not a synthesized template
      node: `CitationChunk` and `ProcEntry` carry it, and nothing pattern-matches
      the template AST to find one.
- [ ] Every helper that re-derived marker identity is gone, including
      `ProcTemplateComponent::label_only` and `is_citation_number_component`.
- [ ] Marker presentation is applied exactly once, after collapse; no code path
      sets wrap or vertical-align and then clears it.
- [ ] All shipped styles declare their marker; none authors one.
- [ ] `item-wrap`, `label-wrap` and `citation.wrap` are independently
      observable: `ieee` renders `[1, p. 737]` and `american-medical-association`
      renders `[1](p737)` from the same locator.
- [ ] `citum-migrate` emits `label-mode` and no marker component.
- [ ] `just pre-commit` and `just check-core-quality` green.

## Related specs

- [CITATION_REGIME](CITATION_REGIME.md) — defines the regimes whose primary keys
  these markers render; the vocabulary this spec builds on.
- [TEMPLATE_V3](TEMPLATE_V3.md) §2.3 — the shipped declarative numeric
  `label-mode` surface.
- [UNIFIED_SCOPED_OPTIONS](UNIFIED_SCOPED_OPTIONS.md) — where `label-mode` and
  `label-wrap` live in the scoped-option model.
- [DISAMBIGUATION](DISAMBIGUATION.md) — regime-scoped disambiguation.
- [CITATION_CLUSTER_RENDERING](CITATION_CLUSTER_RENDERING.md) — cluster-level
  wrapping and delimiters, which markers must stay distinct from.
- [NOTE_STYLE_DOCUMENT_NOTE_CONTEXT](NOTE_STYLE_DOCUMENT_NOTE_CONTEXT.md) — note
  containers, which this spec excludes.

## Changelog

- 2026-08-04: Initial version.
