---
# csl26-uctc
title: Same-author collapse ignores locator when picking join delimiter
status: completed
type: bug
priority: high
tags:
    - chicago
    - fidelity
    - engine
    - rendering
    - citation
created_at: 2026-08-18T12:48:54Z
updated_at: 2026-08-18T17:49:27Z
parent: csl26-h7oc
---

Escalate the same-author-collapse intra-group join delimiter to multi-cite-delimiter
when any item in the group carries a locator, matching CMOS 15.30 and
citeproc-js's after-collapse-delimiter mechanism.

Verified against the release binary (chicago-author-date-18th):
- [@ITEM-31; @ITEM-32, p. 257] -> "(Garcia 2019b, 2019a, 257)", wanted "(Garcia 2019b; 2019a, 257)"
- [@ITEM-31, p. 100; @ITEM-32] -> "(Garcia 2019b, 100, 2019a)", wanted "(Garcia 2019b, 100; 2019a)"

Not Chicago-specific -- hits every author-date style that collapses by author
(apa-7th, springer-basic-author-date also verified).

Already specified in docs/specs/CITATION_CLUSTER_RENDERING.md:183-200
("Same-author collapse with locators"); this closes the implementation gap.

## Plan

- [x] crates/citum-engine/src/processor/rendering/grouped/core.rs: compute
      group_has_locator, escalate join to multi_cite_delimiter at BOTH live
      sites -- non-integral repeated_item_delimiter (build_grouped_citation_content)
      and integral pre_wrapped_years computation (render_fallback_grouped_citation_with_format,
      NOT the build_grouped_citation_content integral fallback, which doesn't
      run for collapsed groups)
- [x] Rust test replaying tests/csl-test-suite/processor-tests/machines/collapse_ChicagoAfterCollapse.json
      natively, asserting against citeproc-js's own strings (oracle-grade, zero
      new fixtures) -- test_same_author_collapse_matches_csl_suite_chicago_after_collapse,
      exact match on first try after fixing the template shape
- [x] Supporting tests in crates/citum-engine/tests/domain_fixtures.rs (locator
      on first item, locator on second item, no-locator regression guard,
      integral mode) -- all 4 passing
- [x] Add div-017 to docs/adjudication/DIVERGENCE_REGISTER.md for the no-locator
      case, where Citum (CMOS 15.30-correct, comma join) diverges from
      citeproc-js (semicolon, leaked from chicago-author-date.csl's
      <layout delimiter="; "> via cite-group-delimiter's default) -- pre-existing,
      unchanged by this fix, intentional. Also bumped CITATION_CLUSTER_RENDERING.md
      to v1.3 naming the escalation source.
- [x] Wired div-017 masking in scripts/report-data/verification-policy.yaml +
      scripts/lib/oracle-divergences.js. Had to guard on exactMatch, not the
      coarse fuzzy match field like the other div-XXX explainers -- this
      divergence's whole delta is one punctuation char, which already clears
      the similarity threshold with match:true before any divergence applies,
      so gating on match like the others meant the function never fired.
      Compares against exactOracle/exactCitum (what exactMatch actually
      failed on), not oracle/citum. Verified against apa-7th (0 excluded,
      correctly doesn't fire there -- APA's oracle never uses semicolon here).
- [x] Verified report-core.js --style chicago-author-date-18th: zero snapshot
      churn (git status tests/snapshots/ clean). exactParity moved from
      174/546 (baseline, unmasked) to 174/542 excluded=4 (divergence-adjusted)
      -- expected movement from adjudication, not a regression; appliedDivergence
      now shows div-017 for disambiguate-year-suffix and
      subsequent-author-consecutive.

## Out of scope (filed separately)

- Same-author collapse produces malformed note citations (chicago-notes-18th) --
  blocks extending note-style multi-cite fixture coverage
- MLA drops the locator delimiter entirely in collapsed groups
- taylor-and-francis-chicago-author-date stray space before locator comma
  (already enshrined in domain_fixtures.rs:288 assert_eq!)
- csl26-ecfn (always-collapse vs CSL's opt-in collapse) -- unrelated, do not touch

## Summary of Changes

Fixed same-author collapse to escalate the intra-group join delimiter to
`multi-cite-delimiter` when any item in the group carries a locator, per
CMOS 15.30 and citeproc-js's `after-collapse-delimiter` mechanism. Fixed at
both live join sites in `crates/citum-engine/src/processor/rendering/grouped/core.rs`
(non-integral `repeated_item_delimiter` and integral `pre_wrapped_years`) --
the second site matters because a naive fix touching only the
`build_grouped_citation_content` integral fallback would compile, pass a
shallow test, and fix nothing (that fallback never runs for collapsed groups).

New tests: a primary test replaying `collapse_ChicagoAfterCollapse.json`
from the CSL test suite natively in Rust, asserting against citeproc-js's own
published strings (oracle-grade, zero new fixtures) -- exact match. Four
supporting tests in `domain_fixtures.rs` (locator on first/second item,
no-locator regression guard, integral mode). Fixed one pre-existing test
(`test_integral_locator_does_not_duplicate_group_delimiter`) whose expected
string enshrined the old bug.

Filed `div-017` in DIVERGENCE_REGISTER.md for the companion no-locator case,
where Citum's CMOS-correct comma join diverges from citeproc-js's semicolon
(itself an artifact of chicago-author-date.csl's layout delimiter, not a
considered CMOS-following choice) -- wired into the oracle-divergences.js
masking machinery so it shows as an explained, divergence-adjusted mismatch
in report-core.js rather than an unexplained delta.

Full pre-commit gate green (fmt, clippy, 2598/2598 tests). Zero snapshot
churn -- no shared fixture combines same-author collapse with a locator,
which is exactly why the bug went unnoticed until now.
