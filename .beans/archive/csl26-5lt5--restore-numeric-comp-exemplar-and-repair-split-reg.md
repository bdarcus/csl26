---
# csl26-5lt5
title: Restore numeric-comp exemplar and repair split registry URLs
status: completed
type: task
priority: high
created_at: 2026-07-30T12:43:33Z
updated_at: 2026-07-30T12:57:57Z
parent: csl26-s2rw
---

PR A of style-corpus split cleanup. Restore styles/numeric-comp.yaml as the sole compound-numeric exemplar (biblatex is the only non-citeproc-js authority; without a shipped compound style the whole apparatus is orphaned). Delete orphaned chem-* biblatex snapshots/policy blocks. Repair 123 registry.default.yaml URLs still pointing at citum-core for styles that moved to citum-styles. Fix SKIPPED_STYLES stale entries. Full plan: /home/bruce/.claude/plans/there-are-some-style-imperative-wand.md

## Todo
- [x] A1: restore styles/numeric-comp.yaml, update disposition TSV, styles/README.md tier counts
- [x] A2: delete orphaned chem-* biblatex snapshots + policy blocks; fix report-core.test.js assertions (new scripts/verification-policy.test.js or scripts/lib test, check CI glob); fix SKIPPED_STYLES
- [x] A3: retarget 122 broken registry URLs to citum-styles; add guard test
- [x] just pre-commit green
- [x] Committed on fix/style-split-fallout (c6890fec); PR not yet opened pending PR B sequencing

## Summary of Changes

Restored styles/numeric-comp.yaml as the sole compound-numeric exemplar, keeping the
biblatex verification apparatus alive (its only remaining subject). Removed orphaned
chem-acs/rsc/biochem/angewandte-chemie snapshots and verification-policy.yaml blocks.
Retargeted 122 embedded registry URLs from citum-core to citum-styles and added
scripts/registry-integrity.test.js as a regression guard. Fixed report-core.test.js
assertions and moved the scope-authority unit test to scripts/verification-policy.test.js
with a synthetic fixture. Dropped two stale SKIPPED_STYLES entries. Fixed the CI
node --test glob to include scripts/lib/*.test.js (previously not run in CI; all 14
pre-existing tests there pass). All 216 JS tests pass, 2293 Rust tests pass, just
pre-commit green. report-core.js now reports 35 styles (16 exemplar) with zero errors.

Commit: c6890fec on branch fix/style-split-fallout.

PR B (citum-styles): commit a46cc85 on branch fix/split-shadowing-and-relocation. Removed styles/numeric-comp.yaml, taylor-and-francis-council-of-science-editors-author-date.yaml, taylor-and-francis-national-library-of-medicine.yaml (all shadowed embedded citum-core parents); dropped their disposition rows; documented the id-collision rule in README.md. Live-URL end-to-end verification of the retargeted registry (PR A A3) is blocked on network access in this sandbox and on both branches being pushed/merged; file existence and id-set matching were verified locally instead (122/122 ids confirmed present in citum-styles/styles/).
