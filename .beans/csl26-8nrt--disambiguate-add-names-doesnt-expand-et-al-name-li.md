---
# csl26-8nrt
title: Disambiguate-add-names doesn't expand et-al name-list depth
status: in-progress
type: bug
priority: normal
tags:
    - rendering
    - disambiguation
    - engine
    - citation
created_at: 2026-08-16T00:16:01Z
updated_at: 2026-08-16T18:26:40Z
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
- [ ] Confirm whether these two cases involve an actual colliding
      author-group (another citation in the same test corpus sharing
      "Smith" as first author) that would explain citeproc-js's
      3-name expansion, or whether it's unconditional for 3+ authors
      regardless of collision.
- [ ] Locate the et-al/disambiguate-add-names expansion-depth logic
      in citum-engine (likely near the disambiguation module) and
      compare its widening rule against citeproc-js's.
- [ ] Fix and add regression tests; re-run report-core.js --style
      taylor-and-francis-council-of-science-editors-author-date to
      confirm exactParity improvement.
