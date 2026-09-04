---
# csl26-wbak
title: Given-first long-name assembly misorders a numeral generational suffix
status: completed
type: bug
priority: normal
tags:
    - style
    - chicago
    - fidelity
    - contributors
created_at: 2026-09-04T13:02:57Z
updated_at: 2026-09-04T14:29:59Z
parent: csl26-h7oc
---

Root cause was mis-diagnosed in the original scoping: both
`assemble_given_first_long_name` and `assemble_inverted_long_name`
(crates/citum-engine/src/values/contributor/names.rs) already position
a populated `suffix` correctly for their respective forms. The actual
defect is upstream, in CSL-JSON ingestion
(crates/citum-schema-data/src/reference/conversion/mod.rs, the
`From<Vec<csl_legacy::csl_json::Name>> for Contributor` impl): when a
source item embeds a suffix in `given` with no separate `suffix`
field (e.g. `{"family": "DeYeso", "given": "Robert, III"}`,
fixture 6188419/JJW86NR2), citum kept the whole string as `given`
verbatim. citeproc-js's reference `parseSuffix` (unconditional in the
vendored citeproc-js, scripts/node_modules/citeproc/citeproc_commonjs.js
~line 24949) splits at the first comma into given + suffix whenever no
explicit suffix is set, which is why oracle renders "Robert DeYeso
III" but citum rendered "Robert, III DeYeso".

Fixed by `split_unparsed_given_suffix` in conversion/mod.rs, ported
from citeproc-js's parseSuffix (including its non-dropping-particle
guard). Verified against the single-item oracle diff (exact match
after fix) and 4 new unit tests. Full-portfolio per-entry diff (35
embedded styles) shows zero regressions but also zero rows newly
passing today: all 26 comma-in-given fixtures across the portfolio
(12 in chicago-18th.json alone) are entangled with at least one other
unresolved defect on the same row (title case, missing genre/date
detail, or DOI+URL duplication) — same entanglement pattern already
documented for PR1/PR3. The fix is still correct and structural
(suffix is now a real field, affecting sorting/export too, not just
this render path) and is a precondition for those other waves to
fully flip these rows later. The "4 confirmed rows" language in the
original plan was an audit label-carrying count, not a re-verified
sole-cause flip — same caveat as csl26-3m0u (PR2). See plan:
/home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md

## Summary of Changes

Fixed in crates/citum-schema-data/src/reference/conversion/mod.rs
(`split_unparsed_given_suffix`), not names.rs — see corrected root
cause above. Landed as a stacked PR on fix/chicago-terminal-punct-collision.
