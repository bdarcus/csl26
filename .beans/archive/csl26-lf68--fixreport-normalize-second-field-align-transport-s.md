---
# csl26-lf68
title: 'fix(report): normalize second-field-align transport + spike class-A2 rendering gap'
status: completed
type: task
priority: high
created_at: 2026-07-30T14:18:11Z
updated_at: 2026-07-30T14:29:07Z
parent: csl26-arly
---

citeproc-js emits second-field-align bibliographies as sibling <div class=csl-left-margin>/<div class=csl-right-inline>; stripping tags in scripts/oracle-utils.js / scripts/report-core.js (~line 1308) concatenates them with no separator (class A1, harness bug — join with a single space, extend scripts/oracle.test.js's existing csl-left-margin fixture at line 969). Separately: class A2 is real and larger (318 mismatches, 12.7% of all parity failures) — Citum's bibliography string omits the number entirely for some styles (AMA: 0/14 sampled have it) but includes it for others (IEEE: 14/14) and is inconsistent within a single style (RSC: mixed). Time-box a spike to determine whether this is an engine rendering gap or a report-harness invocation gap; file the finding as a follow-up bean, do not fix A2 rendering in this task. Part of PR-1 (fix/embedded-parity-wave-1).

## Summary of Changes

Time-boxed spike, findings written up in docs/architecture/audits/2026-07-30_EMBEDDED_PARITY_CLASS_A.md. No production fix lands here.

**Corrected the plan's own premise:** the 'A1 join-space bug' hypothesis (normalizeExactText concatenating csl-left-margin/csl-right-inline transport without a boundary space) does not hold -- verified zero of 2501 mismatches are whitespace-only, and the IEEE control case (already flush-concatenated on both sides, already exactMatch:true) proves inserting a join space would have been a regression, not a fix. No change made to scripts/oracle-utils.js. Added a regression test in scripts/oracle.test.js documenting why, so this dead end isn't re-walked.

**Root-caused class A2** (the real defect, 318/2501 mismatches, largest single class): Citum has no processor-level equivalent to citeproc-js's second-field-align auto-numbering. A style's bibliography only gets a numeric label if its template explicitly includes a 'number: citation-number' component (ieee.yaml does; american-medical-association.yaml does not) -- confirmed via direct 'citum render refs --json' output on both styles. Filed as a follow-up task under this epic.

**Found and filed a separate report-harness bug**, not root-caused (out of time-box): royal-society-of-chemistry's oracleDetail shows a Citum-side number present on some entries/absent on others under the same evidenceRunId, but direct CLI rendering of that style produces zero numbered entries consistently -- the report's evidence for those indices doesn't match what the style actually renders. Filed as a follow-up bug under this epic.
