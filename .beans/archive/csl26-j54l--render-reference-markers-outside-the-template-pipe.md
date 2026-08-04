---
# csl26-j54l
title: Render reference markers outside the template pipeline
status: completed
type: task
priority: normal
tags:
    - schema
    - engine
    - citations
created_at: 2026-08-04T16:23:19Z
updated_at: 2026-08-04T21:20:13Z
---

Finish the reference-marker redesign: stop materializing the marker as a synthetic TemplateComponent so NumberVariable::CitationNumber and CitationLabel can be deleted from the enum entirely.

Spec: docs/specs/REFERENCE_MARKERS.md (Acceptance criteria -> Outstanding)

Authored markers are already rejected at parse time and every shipped style declares label-mode, so the authoring surface is closed. What remains is internal: the processor still synthesizes a Number node and renders it through the template pipeline.

- [ ] Render the marker slot directly instead of synthesizing a template node.
- [ ] Delete NumberVariable::CitationNumber and CitationLabel.
- [ ] Retire ProcTemplateComponent::label_only (it flags the marker slot so the entry separator is suppressed after it).
- [ ] Retire clear_citation_label_presentation (numeric presentation is applied then cleared so collapse sees a bare number).
- [ ] Hold report-core fidelity and exact-parity green.

## Summary of Changes

The marker is now a value in the render model, not a synthesized template node.

- `MarkerValue` / `CitationMarkerSpec` / `BibliographyMarkerSpec` in `processor/rendering/marker.rs`; resolvers return a spec and insert nothing.
- `CitationChunk.marker` and `ProcEntry.marker` carry it; collapse reads `MarkerValue::Number` directly.
- `NumberVariable::CitationNumber` and `CitationLabel` deleted. The two names stay *reserved* in `from_key`: `NumberVariable` is an open vocabulary, so without a reservation they would parse as `Custom` kinds and render nothing.
- `ProcTemplateComponent::label_only`, `is_citation_number_component` and `is_citation_number_label` deleted; the compound-numeric partition reads `entry.marker`.
- Digit localization preserved for markers via `values::number::localize_digits`.
- citum-migrate: the locator fixup gates on the CSL layout, and a layout group wrapping the citation number becomes declarative `item-wrap`.

Fixed two pre-existing double-wrap bugs: `elsevier-with-titles` and `springer-basic-brackets` rendered `[[1]]` (cluster wrap plus a redundant per-marker `label-wrap`).

Verified: `just pre-commit` green (2,389 tests); `just check-core-quality` passed (36 styles fidelity 1.0, exact-parity >= baseline for 19 embedded-core styles); `just validate-production-styles` passed.
