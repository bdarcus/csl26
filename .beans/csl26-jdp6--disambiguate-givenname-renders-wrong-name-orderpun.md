---
# csl26-jdp6
title: disambiguate-givenname renders wrong name order/punctuation vs oracle
status: todo
type: bug
priority: normal
tags:
    - rendering
    - disambiguation
    - engine
    - citation
created_at: 2026-08-16T18:26:30Z
updated_at: 2026-08-16T18:26:37Z
---

Found while investigating csl26-8nrt (by-cite disambiguation scope) via
taylor-and-francis-council-of-science-editors-author-date's citations-expanded.json
fixture. For citation id `disambiguate-givenname`:

- Oracle (citeproc-js): (A. Johnson 2020; B. Johnson 2020)
- Citum:                (Johnson A 2020; Johnson B 2020)

Two divergences bundled together:
1. Name order — oracle puts the given-name initial before the family name;
   Citum puts family first. Style sets `display-as-sort: all` (sort order for
   all contributor positions, not just first).
2. Initial punctuation — oracle's initial has a trailing period ("A."); Citum's
   does not ("A").

## Investigation needed
- [ ] Confirm whether `display-as-sort: all` should force name-order to
      family-first in citation context per CSL semantics, or whether Citum is
      wrongly applying it to a position/form where citeproc-js does not.
- [ ] Check initialize-with punctuation handling for the disambiguation-driven
      given-name expansion path specifically (contributors.initialize-with is
      empty string in this style's core options, but citeproc-js still
      appends "." — check whether initial-formatting is being bypassed for
      hint-driven expansion).
- [ ] Locate the render path for given-name expansion in citum-engine
      (crates/citum-engine/src/values/contributor/names.rs and related name
      formatting) and compare against real CSL name-part rendering rules for
      form=short/initials.
- [ ] Fix and add regression tests; re-run report-core.js --style
      taylor-and-francis-council-of-science-editors-author-date to confirm
      exactParity improvement.
