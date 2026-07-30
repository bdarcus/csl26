---
# csl26-edjj
title: Build style-overlap measurement harness for Elsevier/T&F -core siblings
status: completed
type: task
priority: normal
created_at: 2026-07-30T12:58:04Z
updated_at: 2026-07-30T13:03:56Z
parent: csl26-s2rw
---

PR C of style-corpus split cleanup. New scripts/measure-style-overlap.js measuring option-key-path and component-set overlap between elsevier-{harvard,vancouver,with-titles}-core and taylor-and-francis-{chicago-author-date,council-of-science-editors-author-date,national-library-of-medicine}-core, plus preset attribution against crates/citum-schema-style/src/presets.rs. Deliverable is the harness + a recorded recommendation (negative result is legitimate), not a refactor. Full plan: /home/bruce/.claude/plans/there-are-some-style-imperative-wand.md section C

## Summary of Changes

Added scripts/measure-style-overlap.js: order-independent option-key-path and
component-set overlap measurement with preset attribution against
crates/citum-schema-style/src/presets.rs. Ran against both families; wrote
scripts/report-data/style-overlap-2026-07-30.tsv and recorded findings in
docs/architecture/audits/2026-07-30_ELSEVIER_TF_CORE_OVERLAP.md.

Result: no shared parent justified for either family. Elsevier 3-way common
option overlap is 3 paths (two trivial scalar flags); the highest-CSL-reach
styles in the portfolio stay independent. Taylor & Francis: chicago-author-date
is correctly an outlier (different family, already extends
chicago-author-date-18th). CSE vs NLM has real overlap (30.6% options, 46.4%
components) but 9 of 10 non-preset-expressible shared paths are
bibliography.options.contributors.* sub-fields duplicating the vancouver
preset by hand — preset-shaped, not template-shaped. Filed csl26-vdum as the
smaller, lower-risk follow-up (collapse both to the preset) rather than
folding it into this measurement PR.

216 -> 214 JS tests pass on this branch (base is main, not the PR A branch;
no Rust/style files touched so no nextest/pre-commit re-run needed).
