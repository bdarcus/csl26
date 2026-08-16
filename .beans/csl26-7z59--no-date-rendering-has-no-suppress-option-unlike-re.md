---
# csl26-7z59
title: No-date rendering has no suppress option, unlike real CSL
status: todo
type: bug
priority: normal
tags:
    - rendering
    - dates
    - engine
    - citation
created_at: 2026-08-16T00:16:33Z
updated_at: 2026-08-16T00:16:33Z
---

Found while validating csl26-p7a8's title-quote flip for
taylor-and-francis-council-of-science-editors-author-date via
report-core.js exactParity comparison against citeproc-js.

Real CSL's year-date macro is a bare
<date variable="issued"><date-part name="year"/></date> with no
fallback branch. CSL 1.0 renders nothing when the date variable is
entirely absent unless the style explicitly authors a fallback (e.g.
<if variable="issued">...<else><text term="no date"/></else></if>).
This style doesn't. citeproc-js confirms: an author-less-except-for-a-
surname-that-happens-to-be-"Forthcoming" reference with no `issued`
field renders "(Forthcoming)" -- author only, no date placeholder.

Citum always injects a "n.d." (or "no date") term when a date:issued
template component's date is absent:
"(Forthcoming n.d.)" instead of "(Forthcoming)".

Confirmed via crates/citum-schema-style/src/options/dates.rs that
there is no schema field to suppress this -- `DateConfig.no_date_form`
only selects which term to render (`short` -> "n.d.", `long` -> "no
date"), not whether to render one. This is a pure engine-level
default with no per-style override, unlike the disambiguate and
substitute-title gaps in sibling beans csl26-8nrt/csl26-ecfn.

## Investigation needed
- [ ] Confirm this is a genuine engine gap (not a schema field I
      missed) by grepping the full date-rendering path in
      citum-engine for where the no-date fallback gets applied.
- [ ] Decide the fix shape: a new DateConfig field (e.g.
      `no-date-form: none` alongside `short`/`long`), or treat
      "no fallback" as the new default and require styles to opt
      IN to a no-date term (matches CSL's actual default -- likely
      the more correct direction, but is a broader behavior change
      needing its own before/after report-core.js sweep across the
      full embedded-style corpus, not just this one style).
- [ ] Whichever direction: add tests and a DIVERGENCE_REGISTER.md
      entry documenting the decision.
- [ ] Re-run report-core.js --style
      taylor-and-francis-council-of-science-editors-author-date
      after the fix to confirm impact.
