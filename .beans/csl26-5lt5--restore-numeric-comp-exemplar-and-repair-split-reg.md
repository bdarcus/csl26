---
# csl26-5lt5
title: Restore numeric-comp exemplar and repair split registry URLs
status: in-progress
type: task
priority: high
created_at: 2026-07-30T12:43:33Z
updated_at: 2026-07-30T12:55:08Z
parent: csl26-s2rw
---

PR A of style-corpus split cleanup. Restore styles/numeric-comp.yaml as the sole compound-numeric exemplar (biblatex is the only non-citeproc-js authority; without a shipped compound style the whole apparatus is orphaned). Delete orphaned chem-* biblatex snapshots/policy blocks. Repair 123 registry.default.yaml URLs still pointing at citum-core for styles that moved to citum-styles. Fix SKIPPED_STYLES stale entries. Full plan: /home/bruce/.claude/plans/there-are-some-style-imperative-wand.md

## Todo
- [x] A1: restore styles/numeric-comp.yaml, update disposition TSV, styles/README.md tier counts
- [x] A2: delete orphaned chem-* biblatex snapshots + policy blocks; fix report-core.test.js assertions (new scripts/verification-policy.test.js or scripts/lib test, check CI glob); fix SKIPPED_STYLES
- [x] A3: retarget 122 broken registry URLs to citum-styles; add guard test
- [x] just pre-commit green
- [ ] PR opened
