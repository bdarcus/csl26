---
# csl26-oxdn
title: T&F CSE bibliography entries missing terminal period
status: todo
type: bug
priority: normal
tags:
    - rendering
    - bibliography
    - style-migration
created_at: 2026-08-16T18:50:56Z
updated_at: 2026-08-16T18:51:01Z
---

Found while checking csl26-8nrt's exactParity impact on
taylor-and-francis-council-of-science-editors-author-date. Citation-side
exactParity is 20/20 after csl26-8nrt; the remaining 29/67 overall gap is
almost entirely (~38 of 47 bibliography entries) this single missing
terminal period -- unrelated to disambiguation.

- Oracle:  "Hawking S. 1988. A Brief History of Time. New York: Bantam Dell Publishing Group."
- Citum:   "Hawking S. 1988. A Brief History of Time. New York: Bantam Dell Publishing Group"

Real CSL (styles-legacy/taylor-and-francis-council-of-science-editors-author-date.csl,
bibliography <layout>) wraps the whole entry body in
`<group suffix="." delimiter=". ">` (line ~164) -- every bibliography entry
ends in a period by construction. The migrated core style
(crates/citum-schema-style/embedded/styles/taylor-and-francis-council-of-science-editors-author-date-core.yaml)
has `bibliography.options.separator: ". "` (the inter-field delimiter) but no
terminal suffix -- 12 other embedded core styles already use
`bibliography.options.entry-suffix` for exactly this, so the mechanism
exists; T&F CSE's migration/authoring just didn't set it.

## Investigation needed
- [ ] Confirm `entry-suffix: "."` is the right fix (vs. a per-type-variant
      suffix, in case some type-variants already emit their own terminal
      punctuation and would double up).
- [ ] Apply and re-run report-core.js --style
      taylor-and-francis-council-of-science-editors-author-date; expect most
      of the ~38 period-only bibliography mismatches to resolve.
- [ ] Check whether NLM/CSE sibling styles (see csl26-vdum, csl26-r4dm) share
      the same gap.
- [ ] The 3 remaining non-period bibliography mismatches
      (TLIB-SEL-STANDARD-1, TLIB-SEL-MAP-1, TLIB-SEL-DICT-1 -- webpage-type
      [Internet]/[accessed] field-set divergences) are a separate issue, not
      in scope here.
