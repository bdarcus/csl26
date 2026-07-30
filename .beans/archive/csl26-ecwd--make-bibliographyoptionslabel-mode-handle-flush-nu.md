---
# csl26-ecwd
title: Make bibliography.options.label-mode handle flush numeric labels without per-type-variant duplication
status: completed
type: task
priority: high
created_at: 2026-07-30T16:02:04Z
updated_at: 2026-07-30T16:47:20Z
parent: csl26-arly
---

Design smell surfaced by csl26-j7uc (bibliography numeric-label rendering gap): CSL 1.0 specifies second-field-align numbering once at the bibliography level; Citum currently requires hand-authoring a 'number: citation-number' component into every single bibliography type-variant plus the default template (18 type-variants for american-medical-association.yaml alone, repeated again for nature/ACS/AMA-alphabetical -- ~25 near-identical edits total across 4 style files).

The engine already has a partial mechanism for exactly this, but it's unusable as-is:
1. crates/citum-schema-style/src/options/scoped.rs apply_bibliography_options reads bibliography.options.label-mode (BibliographyLabelMode: none/numeric/author-date) and label-wrap (BibliographyLabelWrap: none/parentheses/brackets), meant to auto-insert the label once via a style option instead of per-type-variant authoring.
2. Bug: update_label_mode's Numeric branch inserts the citation-number component as a bare list item (template.insert(0, TemplateComponent::Number(..Default::default()))) with NO delimiter: "" wrapping against the next component. This means the outer list's default separator (e.g. bibliography.options.separator: '. ') leaks in between the number and the following text -- confirmed empirically: trying label-mode: numeric on american-medical-association.yaml produced '1. Kuhn TS.' instead of the oracle-correct flush '1.Kuhn TS.', which is why csl26-j7uc had to hand-author every type-variant instead of using this one-line option.
3. Gap: BibliographyLabelWrap only covers none/parentheses/brackets. Three of the four styles fixed in csl26-j7uc (american-medical-association, nature, american-medical-association-alphabetical) needed a plain period suffix ('1.'), which has no representation in BibliographyLabelWrap at all -- only american-chemical-society's parenthetical '(1)' format maps onto an existing variant.

Fix would be: (a) wrap the auto-inserted number + the original first component in a delimiter: "" group in update_label_mode's Numeric branch, matching the pattern now hand-authored in ieee.yaml, elsevier-vancouver-core.yaml, and the 4 csl26-j7uc styles; (b) add a period/full-stop variant to BibliographyLabelWrap (or a separate label-suffix option). Once both land, all 4 csl26-j7uc styles' ~25 hand-authored wrap blocks could collapse to a single 'label-mode: numeric' + 'label-wrap: ...' declaration each, and any future numeric-style migration gets this for free instead of repeating the same per-type-variant surgery.

Not urgent -- the 4 styles already fixed render correctly today. This is a maintainability/DRY improvement for future numeric-style work, not a correctness bug.

## Plan (2026-07-30, post-#1118 review)

Sequencing agreed: fix this bean first, merge it, THEN revise fix/bib-numeric-label-gap (#1118) to use the resulting one-line label-mode/label-wrap declarations instead of its current ~25 hand-authored delimiter:"" wrap blocks across american-medical-association.yaml, nature.yaml, american-chemical-society.yaml, and american-medical-association-alphabetical.yaml.

**Acceptance target:** styles/embedded/ieee.yaml and styles/embedded/elsevier-vancouver-core.yaml already hand-author the correct delimiter:""-wrapped pattern -- the fixed label-mode/label-wrap machinery should be able to reproduce their rendered output exactly via the declarative option instead.

**#1118 is intentionally held unmerged** pending this bean, to avoid landing the verbose hand-authored version and then immediately superseding it.

## Summary of Changes

- `crates/citum-schema-style/src/options/scoped.rs`: `update_label_mode`'s `Numeric`
  branch now wraps the auto-inserted `citation-number` and the template's original
  first component in a `delimiter: ""` group instead of inserting a bare list item,
  matching the hand-authored pattern. `BibliographyLabelWrap` gained a `Period`
  variant (sets `rendering.suffix = "."` and clears `wrap`; the other variants now
  symmetrically clear `suffix` when applied). `has_label` detection, wrap
  application, and the `None`/`AuthorDate` strip all recurse into groups now, and
  the strip collapses groups left empty or with a single trivial child — required
  for idempotent re-application across inheritance levels (parent + child both
  declaring `label-mode: numeric`) and for `elsevier-vancouver-author-date`'s
  `author-date` override to strip a label its numeric parent inserted into a group.
- Converted `styles/embedded/ieee.yaml` and
  `styles/embedded/elsevier-vancouver-core.yaml` to the declarative form: removed
  the hand-authored `citation-number` component from the default template and
  every `Full` type-variant, added `bibliography.options.label-mode: numeric` +
  `label-wrap: brackets`. `ieee.yaml`'s `patent`/`standard` variants (a
  non-flush, comma-separated label style, not the group pattern) were left
  untouched — the option only inserts/rewraps a label when none is present, and
  update_label_wrap is idempotent on their existing bare `wrap: brackets`, so
  they're unaffected either way.
- Verified with `scripts/report-core.js`: fidelity, citation, and bibliography
  match rates for `ieee`, `elsevier-vancouver`, and `elsevier-vancouver-author-date`
  are byte-identical before/after (only the SQI quality score changed, and it
  improved, since the duplicated per-variant number components are gone).
- Added resolution-level tests in `crates/citum-schema-style/src/tests.rs` and
  render-level tests in `crates/citum-engine/tests/bibliography.rs` covering the
  flush-group shape, `label-wrap: period`, idempotent double-resolution, and the
  author-date strip-from-inherited-group case.
- `just schema-gen` run for the new `Period` enum variant (`docs/schemas/style.json`).

Follow-up (separate, not done here): revise PR #1118
(`fix/bib-numeric-label-gap`) to replace its hand-authored wrap blocks in
`american-medical-association.yaml`, `nature.yaml`,
`american-chemical-society.yaml`, and `american-medical-association-alphabetical.yaml`
with `label-mode: numeric` + `label-wrap: period`/`parentheses`.
