---
# csl26-8nrt
title: Disambiguate-add-names doesn't expand et-al name-list depth
status: completed
type: bug
priority: normal
tags:
    - rendering
    - disambiguation
    - engine
    - citation
created_at: 2026-08-16T00:16:01Z
updated_at: 2026-08-16T18:53:55Z
---

Found while validating csl26-p7a8's title-quote flip for
taylor-and-francis-council-of-science-editors-author-date via
report-core.js exactParity (23/67 -> 26/67 after fixing the
disambiguate.names/add-givenname migration bug in the same style, see
bean csl26-p7a8 and PR #1192).

Real CSL: <citation et-al-min="3" et-al-use-first="1"
disambiguate-add-names="true" .../>. Fixing the migrated
disambiguate.names: false -> true resolved one et-al-related
divergence (disambiguate-add-names-et-al) but two remain:

- et-al-single-long-list: oracle "(Smith, Lee, Kumar, et al. 2021)"
  (3 names before et al.) vs Citum "(Smith et al. 2021)" (1 name).
- et-al-with-locator: same pattern, "(Smith, Lee, Nguyen, et al.
  2021, p. 205)" vs "(Smith et al. 2021, p. 205)".

Both fixture items are citations-expanded.json entries under
taylor-and-francis-council-of-science-editors-author-date. With
disambiguate.names: true now set, Citum enables name-list expansion
but doesn't compute the same expansion DEPTH citeproc-js does --
citeproc-js's disambiguate-add-names widens the visible et-al-use-first
count until a colliding group is distinguishable (or some other rule
determines 3 names is needed here); Citum's expansion doesn't appear
to widen past its own default at all for these cases, or widens by a
different rule.

## Investigation needed
- [x] Confirm whether these two cases involve an actual colliding
      author-group (another citation in the same test corpus sharing
      "Smith" as first author) that would explain citeproc-js's
      3-name expansion, or whether it's unconditional for 3+ authors
      regardless of collision.
- [x] Locate the et-al/disambiguate-add-names expansion-depth logic
      in citum-engine (likely near the disambiguation module) and
      compare its widening rule against citeproc-js's.
- [x] Fix and add regression tests; re-run report-core.js --style
      taylor-and-francis-council-of-science-editors-author-date to
      confirm exactParity improvement.

## Summary of Changes

Both references (ITEM-29/ITEM-30) genuinely collide -- same Smith/Lee/.../2021, diverging only at the third author -- so 3 names is the real minimum needed. Root cause was not the et-al widening rule itself but `citation_scoped_by_cite_hints` (crates/citum-engine/src/processor/citation.rs): under the style's default `givenname-disambiguation-rule: by-cite`, this overlay cleared `min_names_to_show` for every citation and recomputed it from a bibliography scoped to just that citation's items, short-circuiting to "no collision" whenever a colliding reference was cited alone. `by-cite` is a given-name rule (cascade strategy 2) and had no authority over et-al depth (strategy 1) in the first place -- confirmed against citeproc-js's source (CSL.Registry.ambigcites is registry-wide for every rule; by-cite only caps escalation depth, never the comparison set).

Landed as a stack:
- docs(spec) PR: corrected DISAMBIGUATION.md sec2.1/2.1.1/3.1 and the reference doc -- by-cite is document-wide like every other rule, not citation-local. Filed csl26-5753 (real by-cite position-minimal expansion, not yet implemented -- by-cite and all-names are now behaviorally identical pending that) and csl26-jdp6 (unrelated disambiguate-givenname name-order/punctuation divergence noticed in the same fixture).
- fix(engine) PR (stacked): removed `citation_scoped_by_cite_hints` / `uses_by_cite_givenname`; citations render directly from the global hint map. Re-expected 3 existing by-cite tests that asserted the old citation-local behavior. Added a native regression (`given_a_by_cite_style_with_an_et_al_collision_when_a_colliding_reference_is_cited_then_the_global_expansion_depth_applies`, 2 cases) mirroring the ITEM-29/30 shape.

Verification: both target citations now match the oracle exactly. T&F CSE exactParity 27/67 -> 29/67 (baselined via detached worktree at main), citations 20/20 exact. Zero regression across the other 3 styles that could reach `by-cite` + `add-givenname` (elsevier-harvard, modern-language-association, gb-t-7714-2025-author-date -- byte-identical exactParity/compat before and after). Full corpus sweep (`report-core.js --all-features`, sandboxed) run with zero regressions. Full `cargo nextest run` (2585 tests) and `just pre-commit` clean.

Filed csl26-oxdn separately: T&F CSE's remaining exactParity gap (29/67, not 67/67) is ~38 bibliography entries missing a terminal period (style-authoring gap, unrelated to disambiguation) plus 3 unrelated webpage-type field-set divergences.
