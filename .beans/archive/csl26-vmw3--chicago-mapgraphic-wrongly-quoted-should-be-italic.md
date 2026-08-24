---
# csl26-vmw3
title: 'Chicago: map/graphic wrongly quoted (should be italic, no quotes)'
status: completed
type: bug
priority: high
tags:
    - style
    - chicago
    - title
    - fidelity
    - punctuation
created_at: 2026-08-24T13:24:27Z
updated_at: 2026-08-24T13:45:45Z
parent: csl26-h7oc
---

chicago-author-date-18th.yaml (and taylor-and-francis-chicago-author-date via extends) route map/graphic titles through the file's default fallback bibliography template, which has an unconditional wrap: punctuation: quotes on title:primary. Per the shipped chicago-author-date.csl's real title choose-block, map/graphic are in the never-quote, always-italic set (book classic graphic hearing map -> italic, title case). Concrete example: 'Cable, D. 2013. The Racial Dot Map. Map. University of Virginia...' -- oracle has no quotes, citum wrongly quotes it. Fix needs a dedicated bibliography type-variant for map/graphic (title: primary, emph: true, no wrap) since category reassignment alone can't remove a node-level wrap a resolved type-variant doesn't carry. Also check chicago-notes-18th/chicago-shortened-notes-bibliography-core for the same defect shape. Wave 3 of the title-quote-boundary work (csl26-jxco is the sibling task bean this supersedes/continues for this specific sub-defect) and the checkpoint wave for whether the leverage-ordered process (csl26-r90t tooling) is actually converging: near-miss queue currently 654 rows across the 4 Chicago styles.

## Summary of Changes

chicago-author-date-18th.yaml: added an explicit map, graphic: bibliography
type-variant (structurally identical to the file's default fallback
template, only title:primary changed from wrap:quotes to
text-case:title+emph:true). Taylor & Francis inherits this automatically
via extends (type-variants merge per-key per
docs/specs/STYLE_INHERITANCE.md rule 4) -- no separate T&F edit needed,
confirmed by the diff below covering both styles identically.

chicago-notes-18th.yaml + chicago-shortened-notes-bibliography-core.yaml:
changed map's type-mapping from component (quote:true, wrongly picked up
in wave 1 for map's title-case fix) to monograph (title-case + emph, no
quote); added graphic: monograph (previously unmapped, no title-case at
all).

Verified per-entry via analyze-parity-residuals.js --diff across all four
Chicago styles: zero regressions (0 newly failing in every style).
Zero newly-passing too -- expected and predicted going in: map/graphic
residuals are heavily entangled with other unaddressed defect classes
(F genre/medium -- dimensions/medium fields like "Oil on canvas, 9 1/2 x
13 in." never wired for these types; C year-suffix; J URL/DOI). Real
progress landed and measured: B (title quote boundary) label-instance
count dropped 273->253 (-20) family-wide, A1 dropped 125->121 (-4).
check-core-quality.js --parity-baseline: no exact-parity-baseline
regression anywhere in the 35-style portfolio. Full pre-commit gate
green: fmt clean, clippy clean, cargo nextest run 2701/2701 passed
(unchanged from before -- no Rust code changed).

Tool-limitation finding worth recording: the near-miss queue (rows with
exactly 1 label) is not a reliable "one fix from passing" guarantee.
map 6188419/CE2ZR4MM in chicago-shortened-notes-bibliography showed only
"B title quote boundary" pre-fix, but its full oracle/citum diff has
substantial other missing content (archive location, dimensions, date)
that no LABEL_RULES pattern happens to match -- labelsFor only adds
Z unclassified when *zero* rules match, not when some hunks match a rule
and others don't. The near-miss count is therefore an optimistic lower
bound on remaining defects, not an exact one. Not fixed here (tool
behavior is defensible as designed -- documenting the caveat, not
proposing a change).
