---
# csl26-omqk
title: Fix title-case stop-word gaps (about/after/before/into/than/under/versus/via/without)
status: completed
type: bug
priority: high
tags:
    - fidelity
    - title-case
    - engine
    - chicago
created_at: 2026-08-24T11:50:11Z
updated_at: 2026-08-24T12:45:01Z
parent: csl26-h7oc
---

TITLE_CASE_STOP_WORDS in crates/citum-engine/src/values/text_case.rs is missing 9 words confirmed present in citeproc-js's real CSL.SKIP_WORDS array and backed by concrete oracle-vs-citum mismatches in current Chicago residuals: about, after, before, into, than, under, versus, via, without. Add with a provenance comment (word-list-only scope, not a full citeproc-js algorithm port) and an rstest-parameterized test. Engine-level change (TextCase::Title), portfolio-wide blast radius -- verify with full-portfolio report-core.js + the new analyze-parity-residuals.js --diff mode + check-core-quality.js --parity-baseline, not just Chicago. Explicitly excludes 'past' (false positive, actually a post/post-weblog sentence-case routing bug), 'c' (false positive, D.C. abbreviation collision), and 'de' (real but non-English-title scoped, needs language-aware handling) -- see follow-up beans.

## Summary of Changes

Added 9 verified words to TITLE_CASE_STOP_WORDS in
crates/citum-engine/src/values/text_case.rs: about, after, before, into,
than, under, versus, via, without. Each backed by a concrete oracle-
lowercase/citum-capitalized row in real Chicago corpus data and confirmed
present in citeproc-js's real CSL.SKIP_WORDS array
(scripts/node_modules/citeproc/citeproc_commonjs.js:1076). Added a
provenance comment on the constant documenting the source and the
deliberate word-list-only scope (citeproc-js's case algorithm never
force-lowercases; that architectural difference is not ported here).

Test: one parameterized rstest
(given_citeproc_skip_word_when_title_case_then_interior_occurrence_stays_lowercase)
covering all 9 words plus a first/last-position guard case, matching the
existing rstest pattern in the same file. 66/66 text_case tests pass.

Verified full-portfolio (35 styles, not just Chicago -- this is a shared
TextCase::Title primitive): report-core.js --all-features before/after +
analyze-parity-residuals.js --diff (the tool from csl26-r90t, its first
real dogfood use) shows exactMatch 1811/4425 -> 1814/4425, +3 newly
passing, **zero newly failing** across all 35 styles. A2 (title-case
over-applied/stop-word) label-instance count dropped 105 -> 67 (-38).
check-core-quality.js --parity-baseline shows no exact-parity-baseline
regression (the pre-existing ~30 fidelity-gate failures are unrelated,
portfolio-wide pre-existing state, unchanged by this commit). Full
just pre-commit equivalent green: cargo fmt --check clean, clippy -D
warnings clean, cargo nextest run 2701/2701 passed (+10 vs pre-fix,
matching the new test cases).
