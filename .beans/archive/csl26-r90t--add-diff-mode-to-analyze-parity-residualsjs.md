---
# csl26-r90t
title: Add --diff mode to analyze-parity-residuals.js
status: completed
type: task
priority: high
tags:
    - tooling
    - style
    - chicago
    - fidelity
created_at: 2026-08-24T11:50:01Z
updated_at: 2026-08-24T12:01:06Z
parent: csl26-h7oc
---

Add a --diff <before.json> mode to scripts/analyze-parity-residuals.js: exactMatch delta (newly-passing/newly-failing), per-label instance-count delta, rows-by-label-count histogram, and a near-miss queue (rows with exactly 1 remaining label). Wire into docs/guides/STYLE_WORKFLOW_EXECUTION.md's Waves section + step 3c, and .claude/skills/style-tune/SKILL.md's Execution Loop/Verification/Output Contract. See docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md and bean csl26-jxco for the motivating analysis (wave 1+2 topline pass-count was flat/small while label-instance count and near-miss queue moved substantially -- this closes the gap between what's mandated and what's tooled).

## Summary of Changes

Added `--diff <before.json>` mode to `scripts/analyze-parity-residuals.js`:
new functions `allRows`, `rowKey`, `exactMatchMap`, `diffExactMatch`,
`diffLabelCounts`, `mergeLabelCounts`, `labelCountHistogram`,
`nearMissQueue`, `diffStyles`, `diffReports`, plus `printDiffHuman`/
`printDiffSection` for the human-readable output and `--json` support.
Verified against this session's real wave 1/2 before/after reports --
reproduces the hand-computed numbers exactly (exactMatch flat 498/1629,
label-instances 1936->1874, histogram 598/348/148 -> 653/300/141 at
1/2/3-label buckets). Self-diff sanity check (same file as before/after)
confirms all-zero deltas. 12 new unit tests added to
`scripts/analyze-parity-residuals.test.js` (26/26 passing), full
`node --test scripts/*.test.js scripts/lib/*.test.js` suite green (341/341).

Caught and fixed a real bug during verification: `rowKey` was built with a
stray NUL-byte separator between kind and id while `diffExactMatch`'s
`key.split` used a plain space, silently breaking the id/kind decomposition
for every newly-passing/newly-failing row (and turning the source file into
a "binary" file for grep). Fixed by switching the separator to a plain
space (reference ids in this corpus never contain spaces).

Wired into `docs/guides/STYLE_WORKFLOW_EXECUTION.md` (Waves section: hard
requirement to report exactMatch delta + label-instance delta + near-miss
count via `--diff`; step 3c: names the concrete command; version bumped
1.4->1.5, changelog entry added) and `.claude/skills/style-tune/SKILL.md`
(Execution Loop step 3, Verification "Regression check" + new "Next-target
queue" bullet, Output Contract).

No wave-ledger file added (decided: hold off, `--diff --json` covers the
need on demand).
