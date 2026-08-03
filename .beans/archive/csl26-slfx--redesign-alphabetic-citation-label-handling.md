---
# csl26-slfx
title: Redesign alphabetic citation-label handling and migrate existing styles
status: completed
type: task
priority: deferred
tags:
    - schema
    - engine
    - citations
created_at: 2026-08-03T18:50:04Z
updated_at: 2026-08-03T22:05:25Z
---

Track the follow-up redesign of processor-owned alphabetic `citation-label` handling after numeric citation labels. Make labels declarative while preserving compatibility for existing template-owned labels, and revise those existing styles to use the feature; no new styles are required.

- [x] Define declarative semantics for citation-label generation and wrapping.
- [x] Implement schema, migration, renderer, and collapse behavior.
- [x] Revise existing styles to use the declarative feature and add parity coverage.

## Summary of Changes

Extends PR #1136's declarative numeric labels to the alphabetic (trigraph) half,
and unifies the two paths rather than duplicating them.

**Schema** — `CitationLabelMode` and `BibliographyLabelMode` each gain
`alphabetic`; both expose `label_variable()` so the mode names the generated
`NumberVariable`. New `bibliography.options.label-separator` supplies the gap
between label and entry body (unset = flush, matching citeproc-js
`second-field-align` flattening); AMS-label's CSL authors `suffix="] "`, so it
needs `' '`.

**Engine** — the numeric-named helpers are parameterized on the label variable
(`template_has_label`, `strip_citation_labels`, `apply_citation_label_wrap`,
`template_is_label_only`), `materialize_citation_template` returns a
`MaterializedCitationTemplate`, and `numeric_label_only: bool` becomes
`label_only: Option<CitationLabelMode>`. Collapse stays numeric-gated: only
`Some(Numeric)` builds a `NumericCitationLabel`, so trigraphs never enter
`collapse_numeric_citation_chunks`. Declaring a mode now also strips a label of
the *other* variable, in both citations and bibliographies — that is what stops
an inherited numeric label from rendering alongside a declared alphabetic one.
`is_citation_number_component` becomes `is_bibliography_label_component` and
matches both variables.

**Migration** — `Processing::Label` emits `citation.options.label-mode:
alphabetic` plus `label-wrap: none` (CSL alphabetic styles bracket the layout,
not the label) and strips the simple leading `citation-label`, sharing
`strip_simple_leading_label` with the numeric path.

**Styles** — `alpha.yaml` and `american-mathematical-society-label.yaml` drop
every authored `number: citation-label` (7 sites) for scoped options. AMS-label
needs `label-wrap: none` to cancel the per-label `brackets` inherited from
elsevier-with-titles; its cluster-level `citation.wrap` is the authoritative
bracket.

## Verification

- `just pre-commit` — 2,412 tests passed, fmt and clippy clean.
- `just validate-production-styles` — passed, 0 violations.
- `just check-core-quality` — 36 styles at fidelity 1.0, exact-parity >= baseline
  for all 19 embedded-core styles.
- Rendered output for `alpha` and `american-mathematical-society-label` is
  byte-identical to a `main` baseline worktree build (citations and
  bibliography).
- Oracle, before -> after, all unchanged: ieee 20/20 47/47, AMA 20/20 47/47,
  springer-vancouver-brackets 20/20 47/47, elsevier-vancouver 20/20 45/47,
  nature 20/20 45/47, elsevier-with-titles 20/20 46/47,
  elsevier-vancouver-author-date 20/20 45/47,
  american-mathematical-society-label 18/20 45/47.

- Migrate arm verified end-to-end: a fresh conversion of
  `styles-legacy/american-mathematical-society-label.csl` emits
  `label-mode: alphabetic` / `label-wrap: none`, an empty citation template,
  and renders `[Kuhn62]`.

## Notes

`american-mathematical-society-label`'s remaining gaps (name-form, missing
type-variants) stay with `csl26-x9oi`. That bean's first fix item — "count
CitationLabel as has_label" — is resolved here.

STYLE004 warns that AMS-label's `bibliography.type-variants.all` duplicates its
default template. It is not removable: `all` is what blocks the type-specific
variants inherited from elsevier-with-titles-core, and dropping it costs oracle
bibliography parity (45/47 -> 36/47). The rule is warn-only; the style carries a
comment explaining why.
