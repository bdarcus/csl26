---
# csl26-lr1p
title: Fix chicago-author-date-18th webpage citation substitute and mislabeled compat-report divergence
status: completed
type: bug
priority: high
tags:
    - chicago
    - citation
    - fidelity
    - tooling
created_at: 2026-08-31T12:24:11Z
updated_at: 2026-08-31T17:53:51Z
parent: csl26-h7oc
---

Chicago-author-date-18th's webpage citation type-variant applies publisher unconditionally instead of as an author substitute (CMOS 18 14.104), producing malformed citations. Separately, docs/compat.html's Citation Findings table labels div-017-masked (and other divergence-masked) citations as 'Unresolved Oracle Drift', indistinguishable from a genuine open gap -- this mislabel caused a wasted investigation into 'fixing' (Garcia 2019b, 2019a) which is actually correct per div-017.

## Todo
- [x] scripts/report-core.js: merge appliedDivergence onto citationEntries (augment, not replace) + branch status label to "Known Divergence (div-NNN)" with tooltip note
- [x] scripts/report-core.test.js: new test case for the Known Divergence label/tooltip
- [x] chicago-author-date-18th.yaml: remove FIX/REVIEW annotations, clarify comments (div-017 pointer + multi-cite-delimiter scope)
- [x] chicago-author-date-18th.yaml: delete broken webpage citation type-variant, leave pointer comment to follow-up bean
- [x] scripts/report-data/known-divergences.json: register chicago-author-date-18th webpage-anonymous-site-owner-substitution divergence
- [x] crates/citum-engine/tests/domain_fixtures.rs: rstest cases for the three webpage shapes (native Monograph construction)
- [x] just pre-commit clean
- [x] node --test scripts/report-core.test.js clean
- [x] core-quality report: confirmed fidelity moves -0.002 for chicago-author-date-18th and taylor-and-francis-chicago-author-date, entirely from one already-registered, deliberate trade-off (chi-webpage: authorless+publisher regresses from accidentally-correct to title-fallback, per csl26-f3hx). No other citation or bibliography entry changed; div-017 rows now render as "Known Divergence (div-017)" with tooltip; Citum Extensions box shows the new webpage divergence.

## Summary of Changes

- scripts/report-core.js: augmented citationEntries with appliedDivergence (index-merged from oracleResult.adjusted.citations.entries, raw match/other fields untouched -- protects analyze-migration-gaps.js/style-coverage-review.js/analyze-parity-residuals.js semantics). Citation Findings status now branches to "Known Divergence (div-NNN)" with a tooltip note, ahead of Unresolved Oracle Drift/Compatibility Fail.
- scripts/report-core.test.js: new test locking the label/tooltip behavior.
- chicago-author-date-18th.yaml: removed the FIX/REVIEW annotations; collapse stays `same-author: {}` (map form, delimiter still unset -- div-017 is correct, not a bug, comment explains why); multi-cite-delimiter comment clarifies between- vs within-group scope; deleted the broken webpage citation.type-variants entry (was unconditionally appending publisher) with a comment pointing at csl26-f3hx.
- scripts/report-data/known-divergences.json: registered chicago-author-date-18th's new webpage-anonymous-site-owner-substitution divergence.
- domain_fixtures.rs: 3 new rstest cases (native Monograph construction) covering the three webpage shapes, loading the live embedded style via get_embedded_style (not load_style, which silently redirects to a separately-pinned, already-stale test fixture -- see the code comment; this is a pre-existing test-infra gap, not something this PR fixes).

Verified: just pre-commit clean (2722 tests). Scoped before/after report-core.js run (git stash of chicago-author-date-18th.yaml) shows fidelity moves -0.002 for chicago-author-date-18th and taylor-and-francis-chicago-author-date, entirely from the one already-registered chi-webpage trade-off; nothing else in citations or bibliography moved, confirming div-017 (Garcia/Chen comma-join) is untouched.
