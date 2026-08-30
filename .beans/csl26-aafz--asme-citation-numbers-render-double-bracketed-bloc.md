---
# csl26-aafz
title: ASME citation numbers render double-bracketed, blocking collapse
status: todo
type: bug
priority: normal
created_at: 2026-08-30T18:33:37Z
updated_at: 2026-08-30T18:33:37Z
---

american-society-of-mechanical-engineers.yaml (extends: ieee) renders
multi-item citations as `[[1],[2],[3]]` instead of `[1],[2],[3]` (or,
correctly, `[1-3]` once collapse works) -- a pre-existing defect surfaced
while investigating csl26-rgys.

Root cause (partial): citation.options.label-mode: numeric is set on the
PARENT (ieee.yaml:67) but ASME's own citation: block fully re-declares
template-ref/wrap/multi-cite-delimiter without options.label-mode. Whatever
this style resolves to for citation mode causes each item to independently
render through the numeric-integral chunk path (double-bracketed per item)
AND the whole group to also pick up ASME's own top-level
citation.wrap: punctuation: brackets -- hence the outer bracket too.

Confirmed empirically: adding options.processing: numeric explicitly at
ASME's own top-level options (redundant with ieee's inherited value, in
case cascade was the issue) made no difference -- the bug is not
processing-inheritance, it is citation.options.label-mode (or equivalent
mode resolution) not reaching ASME's overriding citation: block.

should_collapse_citation_numbers (crates/citum-engine/src/processor/rendering/mod.rs:~505)
requires CitationMode::NonIntegral; this style evidently never reaches that
mode, so collapse: citation-number would be silently inert if added --
NOT added to the YAML for this reason (see comment at
styles/american-society-of-mechanical-engineers.yaml:34-39).

Reproduction: render 3 sequential numeric citations
(citum render refs -b <3-ref fixture> -c <3-item citation> -s
american-society-of-mechanical-engineers -m cite) -> [[1],[2],[3]].
Expected (once fixed AND collapse added): [1-3].

Oracle: 0 bibliography-side regression risk confirmed (44/47 baseline,
citation rendering untouched by oracle.js's default corpus -- the bracket
defect wasn't caught by the standard fixture set, only by an ad-hoc
3-item multi-cite probe).
