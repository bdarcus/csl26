---
# csl26-ecfn
title: Unconditional same-author collapse contradicts CSL's opt-in semantics
status: todo
type: bug
priority: normal
tags:
    - rendering
    - disambiguation
    - citation
    - divergence
created_at: 2026-08-16T00:16:16Z
updated_at: 2026-08-16T00:16:16Z
---

Found while validating csl26-p7a8's title-quote flip for
taylor-and-francis-council-of-science-editors-author-date via
report-core.js exactParity comparison against citeproc-js.

The real CSL (taylor-and-francis-council-of-science-editors-author-date.csl)
has no `collapse` attribute at all on <citation>. In CSL 1.0, collapse
is opt-in -- with no attribute, same-author multi-item citations stay
as separate grouped entries joined by the citation's own delimiter.
citeproc-js confirms this:

- disambiguate-year-suffix: oracle "(Garcia 2019a; Garcia 2019b)"
- subsequent-author-consecutive: oracle "(Chen 2022; Chen 2024)"

Citum instead unconditionally collapses same-author consecutive
cites into a comma-joined year list ("(Garcia 2019a, 2019b)",
"(Chen 2022, 2024)"), regardless of the style's `collapse` setting.

This is not a bug in the sense of unintended behavior -- it's
csl26-dpep ("fix(engine): same-author multicite collapse --
first-class rule for both modes"), a deliberate Citum design
decision to always collapse same-author multicites. But that
decision was never recorded in docs/adjudication/DIVERGENCE_REGISTER.md,
and this T&F CSE oracle comparison is concrete evidence it diverges
from real citeproc-js output for a style that doesn't request
collapsing.

## Needs adjudication, not just a fix
- [ ] Determine whether Citum's always-collapse behavior should
      become conditional on a style-level setting (mirroring CSL's
      collapse attribute more faithfully), or whether this stays a
      deliberate Citum design principle that gets registered as an
      accepted divergence (matching the pattern of other entries in
      DIVERGENCE_REGISTER.md, e.g. div-014).
- [ ] If conditional: design the schema field, default it to
      preserve csl26-dpep's current behavior for styles that don't
      set it explicitly (or default to opt-in per real CSL
      semantics -- this is the actual adjudication question).
- [ ] Either way, add a DIVERGENCE_REGISTER.md entry.
- [ ] Re-run report-core.js --style
      taylor-and-francis-council-of-science-editors-author-date
      after the decision lands to confirm impact.

Related: csl26-dpep (the engine change establishing this behavior).
