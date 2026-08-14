---
# csl26-rmjp
title: 'Triage: 13 core styles show unexplained positional bibliography-order divergence'
status: todo
type: bug
priority: normal
tags:
    - sorting
    - bibliography
    - oracle
    - migrate
    - review-followup
created_at: 2026-08-14T16:40:22Z
updated_at: 2026-08-14T18:17:34Z
---

csl26-7u16's new positional bibliography-order check (scripts/lib/oracle-divergences.js's
compareBibliographyOrder, in the oracle.js/oracle-fast.js JSON output as bibliographyOrder,
and report-core.js's per-style bibliographyOrderMismatch) found that 17/35 core styles render
their bibliography in a different entry sequence than citeproc-js despite per-entry text
matching. After a Copilot review fix corrected an unsound 'explained' check (deleting div-008's
affected ids before comparing residuals discarded their position relative to the rest of the
sequence — see csl26-7u16's post-review-corrections section), only 4 are explained by
registered divergences (div-004 anonymous-item placement; div-008 same-family secondary-sort
now only genuinely explains harvard-cite-them-right). 13 are unexplained; the 8 triaged below
plus 5 more the unsound check had been masking (american-medical-association-alphabetical,
apa-7th, elsevier-vancouver-author-date, springer-basic-author-date,
taylor-and-francis-council-of-science-editors-author-date) — not yet triaged.

## Confirmed root cause

- **elsevier-harvard** (fidelity 1.0 — every entry matches, order alone diverges): the legacy
  CSL (styles-legacy/elsevier-harvard.csl:205-208) declares
  `<key macro="issued" sort="descending"/>` as the bibliography's secondary sort key after
  author. The migrated styles/embedded/elsevier-harvard.yaml has no `sort:` block at all, so
  same-author entries (e.g. three "Chen, Wei" solo-author items, ITEM-38/37/11) fall to
  Citum's default ascending year instead of citeproc's descending. This is a citum-migrate
  sort-key translation gap, not an engine bug — verify whether other `sort="descending"`
  legacy styles have the same gap before scoping a fix.

## Plausible but unconfirmed

- **american-mathematical-society-label** (sorts by `citation-number`, full reorder from
  position 0): likely the same class as csl26-a19q (citation-number bibliography sort not
  fully supported outside numeric processing families).
- **international-journal-of-wildland-fire** (sorts by `issued` then `author` — year-first,
  full reorder from position 0): same class as elsevier-harvard's descending-year gap,
  unconfirmed — its legacy CSL's issued key sort direction needs checking.

## Not yet characterized

- **modern-language-association**, **oscola**, **oscola-no-ibid**,
  **entomological-society-of-america**: divergence starts mid-list, clustered around
  anonymous/TLIB-SEL-* entries — resembles div-004 (anonymous-item placement) but isn't
  matched by the current detector's exact swapped-id set, so either a second, narrower
  anonymous-entry-placement gap, or (for oscola/oscola-no-ibid specifically) a defect in
  their unusually complex multi-key ibid/footnote sort
  (`<key macro="sort-type"/><key macro="author" names-min="1" names-use-first="1"/>...`).
  Not the same phenomenon as csl26-92kv (a citation year-suffix regression for a different
  entomological-society-of-america fixture pair, ITEM-31/32).
- **gb-t-7714-2025-author-date**: needs its own GB/T-fixture-aware oracle run (an ad-hoc
  default-fixture run against this style is meaningless — see
  crates/citum-engine/CLAUDE.md's oracle-routing gotcha and MULTILINGUAL.md).
- **american-medical-association-alphabetical**, **apa-7th**,
  **elsevier-vancouver-author-date**, **springer-basic-author-date**,
  **taylor-and-francis-council-of-science-editors-author-date**: surfaced only after the
  csl26-7u16 post-review 'explained' fix; previously misreported as explained by
  div-004/div-008 because the old residual check discarded the div-008 cluster's position
  relative to the rest of the sequence. Not yet inspected for their real root cause.

## Todo
- [ ] Confirm and fix the elsevier-harvard citum-migrate descending-sort-key gap; audit
      other legacy styles with `sort="descending"` for the same gap
- [ ] Investigate american-mathematical-society-label against csl26-a19q
- [ ] Investigate international-journal-of-wildland-fire's year-first sort direction
- [ ] Root-cause the modern-language-association / oscola / oscola-no-ibid /
      entomological-society-of-america mid-list divergence
- [ ] Root-cause gb-t-7714-2025-author-date with its correct fixture set
- [ ] Root-cause the 5 styles surfaced by the csl26-7u16 post-review 'explained' fix
      (american-medical-association-alphabetical, apa-7th, elsevier-vancouver-author-date,
      springer-basic-author-date, taylor-and-francis-council-of-science-editors-author-date)
