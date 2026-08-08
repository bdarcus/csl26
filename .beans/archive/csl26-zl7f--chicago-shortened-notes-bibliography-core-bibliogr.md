---
# csl26-zl7f
title: 'chicago-shortened-notes-bibliography-core: bibliography separator uses comma instead of period'
status: completed
type: bug
priority: critical
tags:
    - style
    - fidelity
    - chicago
created_at: 2026-08-08T13:35:24Z
updated_at: 2026-08-08T13:56:42Z
parent: csl26-h7oc
---

bibliography.options.separator is ", " (chicago-shortened-notes-bibliography-core.yaml:92) where every other Chicago bibliography style (e.g. chicago-author-date-18th.yaml:193) uses ". ". This joins every top-level bibliography component with a comma instead of a period across the whole entry -- e.g. oracle 'Chen, Wei. "Urban Heat Island Mitigation Strategies." Urban Climate 15 (2022): 1-18.' renders in Citum as 'Chen, Wei, "Urban Heat Island Mitigation Strategies," Urban Climate, 15, 2022: 1-18,' -- wrong throughout, not at one boundary.

Evidence (node scripts/report-core.js, references-expanded + chicago-18th corpora, 2026-08-08): of 422 currently-failing exactParity bibliography entries for chicago-shortened-notes-bibliography, 326 (77%) match this exact signature (citum text has at most one internal '. ' where oracle has two or more). This is almost certainly the single largest contributor to the style's 13/473 (2.7%) exact-parity score -- far larger than any of the shared defect-cluster work in csl26-h7oc's children, and specific to this one style/one option.

Likely fix: change bibliography.options.separator from ', ' to '. ' in chicago-shortened-notes-bibliography-core.yaml. One-line change; per project policy style YAML changes skip the Rust pre-commit gate, but still need citeproc-js verification before landing (see csl26-j1wp -- verify against node scripts/report-core.js, don't assume from the signature alone that every one of the 326 clears, since some entries will have other independent defects layered on top).

- [x] Confirm the fix against a handful of the 326 flagged entries individually, not just the aggregate count
- [x] node scripts/report-core.js before/after on chicago-shortened-notes-bibliography specifically
- [x] cargo nextest run
- [x] Check whether this also affects chicago-shortened-notes-bibliography.yaml's variant siblings (16th/17th-edition, archive-place-first, etc.) that extend -core -- no other embedded style extends -core, nothing else affected
- [x] Note the outcome in this bean's summary when done


## Summary of Changes

Fixed: `bibliography.options.separator` changed from `", "` to `". "` in `chicago-shortened-notes-bibliography-core.yaml`. Confirmed correct at the render level (`citum render refs`): "Chen, Wei. "Title." Container. 15, 2022: 1-18." now uses periods between top-level components, matching every other Chicago bibliography style.

**Correction to this bean's own estimate.** The "326/422 (77%) match this signature" evidence was a text-pattern match (period-count heuristic), not a measurement of how many entries would become byte-exact after the fix -- and it overstated the real impact. Measured before/after with `node scripts/report-core.js --style chicago-shortened-notes-bibliography` (had to force a `cargo build --bin citum` first; the default report-core binary-reuse path doesn't reliably pick up an embedded-style YAML edit -- `touch`ing the changed file was needed to get cargo to recompile):

- bibliography exact-match: 2/424 -> 9/424 (+7)
- overall exactParity: 13/473 (2.7%) -> 20/473 (4.2%)
- citations unaffected (40/49 before and after), as expected since this option is bibliography-only

Most of the 326 flagged entries have a second, independent defect layered on top (e.g. `ITEM-37`: separator now correct, but the volume/issue/date/pages group still renders "Urban Climate. 15, 2022: 1-18" where oracle has "Urban Climate 15 (2022): 1-18" -- a different, unfixed group-formatting bug), so fixing the separator alone doesn't flip them to exact. The fix is still correct and worth keeping -- every entry's top-level delimiter is now right regardless -- it just isn't the single dominant fix the signature count suggested. Real lesson: text-pattern signature matching counts symptom occurrence, not independent-defect count; don't extrapolate exact-match gains from it again without spot-checking a sample first (this bean did spot-check one entry before writing the "77%" claim and should have caught this then).

Also updated two Rust unit tests in `crates/citum-engine/src/processor/document/tests.rs` (`test_note_style_integral_citation_keeps_prose_anchor`, `test_real_chicago_note_style_generates_djot_footnotes`) that hardcoded the old comma-separated expected output ("Doe, John, _Book One_, 2020.") -- updated to the new correct output ("Doe, John. _Book One_, 2020.").

No other embedded style extends `chicago-shortened-notes-bibliography-core` besides `chicago-shortened-notes-bibliography.yaml` itself, so no sibling variants were affected.
