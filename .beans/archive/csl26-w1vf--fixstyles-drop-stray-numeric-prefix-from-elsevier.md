---
# csl26-w1vf
title: 'fix(styles): drop stray numeric prefix from elsevier-vancouver-author-date bibliography'
status: completed
type: bug
priority: normal
created_at: 2026-07-30T14:18:11Z
updated_at: 2026-07-30T14:33:05Z
parent: csl26-arly
---

Class J, 47 of 67 sampled bibliography entries: elsevier-vancouver-author-date (an author-date style) emits a leaked numeric/bracket prefix Citum-side that the oracle doesn't have, e.g. O: 'Bengio Y. The Future of Artificial Intelligence 2023.' vs C: '[2]. Bengio Y. The Future of Artificial Intelligence 2023.' Confine the fix so the elsevier-vancouver-core sibling (elsevier-vancouver, 82.1% parity, CSL reach 502) does not regress. Part of PR-1 (fix/embedded-parity-wave-1).

## Summary of Changes

Root cause: styles/elsevier-vancouver-author-date.yaml extends elsevier-vancouver -> elsevier-vancouver-core (the numeric Vancouver family), which sets bibliography.options.label-mode: numeric and also directly authors bracket-wrapped 'number: citation-number' components into several inherited type-variant/default templates. The author-date variant didn't override bibliography.options.label-mode, so it inherited numeric labeling wholesale -- both the auto-inserted plain number ('33. ') and the pre-authored bracketed number ('[2]. ') for reference types it doesn't itself redefine.

Fix: set bibliography.options.label-mode: author-date on elsevier-vancouver-author-date.yaml. This is a first-class engine feature (crates/citum-schema-style/src/options/scoped.rs update_label_mode) that strips any citation-number/citation-label component from every resolved bibliography template regardless of origin -- exactly matches this case.

Verified via direct 'citum render refs --json' before/after diff: all 47 of 47 bibliography entries changed, every change is exactly the leading numeric-prefix removal with no other text disturbed (matches class J from the taxonomy exactly). elsevier-vancouver (the numeric embedded sibling) confirmed byte-identical before/after -- no regression, since it explicitly sets its own citation-number components and doesn't rely on the parent's auto-insertion path being available for override. node scripts/report-core.js --styles elsevier-vancouver-author-date,elsevier-vancouver: elsevier-vancouver unchanged (fidelity 0.97, exactParity 55/67); elsevier-vancouver-author-date fidelity 0.955->0.97, exactParity 0/67 (0.0%) -> 14/67 (20.9%), citations unaffected (20/20 both before and after). Style YAML only, no .rs touched -- Rust pre-commit gate not applicable.
