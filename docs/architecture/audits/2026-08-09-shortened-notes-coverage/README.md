# Chicago Shortened-Notes Coverage Audit

This record replaces the uncommitted pilot's provisional `23/482` headline and
`119` uncovered count. The generated packet is the complete, stable observation
index; this note distinguishes observed facts from maintainer inference.

## Observed facts

- The audited source revision is `727dba21f0af95c9554d1dc35ee62ad0dddc9976`.
  An explicit `cargo build -q --bin citum` followed by a fresh-cache report with
  `--citum-bin target/debug/citum` produced **34/473** exact parity with **16**
  not-comparable outputs.
- The baseline evidence-run join is **28/80**: 23/34 citations and 5/46
  bibliography entries. The supplemental Chicago shared corpus contributes six
  more exact outputs among 393 comparable outputs and 16 not-comparable outputs.
- The packet contains **565** populated field observations. Thirty-seven are
  excluded fixture metadata (`citation-key`, `license`, `language`, or `note`).
  The 528 relevant observations partition into 157 rendered, 160 fallback, 17
  suppressed, and 194 uncovered observations after the article-journal grammar
  experiment.
- The 17 suppressions are narrow authority-backed publication-place omissions.
  Bibliography `ITEM-1` remains uncovered because citeproc-js renders its
  `publisher-place` as `(Chicago)`.
- Coverage is inferred from the Citum-resolved style. It does not prove runtime
  field consumption, and the 194 uncovered observations are an investigation
  queue rather than 194 confirmed style defects.
- The report-time post-change join measures the current source-built Citum
  output against the authority run: exact parity moved from **34/473** to
  **48/473**. This is output evidence, not proof that an uncovered field caused
  any individual text mismatch.
- A control run at the same source revision and with an empty report cache reused
  a stale managed binary and produced **20/473**. The full report hashes and both
  commands are recorded in `authority-report.json`.

## Maintainer adjudication

| Classification | Status | Finding | Evidence / action |
|---|---|---|---|
| architecture | inference | The packet adds no counterexample requiring a macro or general conditional mechanism. | Keep the spec's counterexample admission rule; legal and treaty gaps remain unresolved. |
| schema/engine | fact | Moving citation-spec punctuation through the locale-aware quote boundary accounts for 14 exact outputs. | Clean parent/child reports moved from 20/473 to 34/473 without denominator drift; retain the focused engine test. |
| harness | fact | A fresh data cache does not force the default managed `citum` binary to match source. | Track and fix under `csl26-b68i`; do not use the stale 20/473 control as authority. |
| fixture | fact | Thirty-seven populated observations are non-rendering fixture metadata under the declared audit policy. | Keep them visible as excluded rather than inflating coverage gaps. |
| style-data | fact | Bibliography `ITEM-1` publisher place is authority-visible but structurally uncovered. | Preserve row 6 as a real candidate gap; do not apply a blanket publisher-place suppression. |
| style-data | inference | Article issue/year grammar and the legal, patent, media, and interview clusters remain the strongest bounded repair candidates. | Repair one cluster at a time and regenerate with the same manifest and denominators. |
| QA | fact | Render disposition, comparison eligibility, and exact match are independent in every row, and the complete Markdown contains all 565 observations. | Use the committed packet freshness check before accepting later baselines. |
| migration | fact | The audited style chain is hand-authored; this packet contains no converter experiment. | Do not assign residuals to migration without separate converter evidence. |

## Unresolved questions

- Which of the 200 structurally uncovered observations are runtime-consumed by
  conditional behavior that static resolution cannot see?
- Is the authority-visible `ITEM-1` publication place a missing Citum template
  component, or does another typed field own that output?
- Do the legal, treaty, and hearing cases expose a bounded schema gap after their
  basic style-data omissions are repaired?

The next bounded style experiment remains article-journal issue and year grammar.
The harness prerequisite is `csl26-b68i`, because source/binary identity must be
trustworthy before a later exact-parity delta is promoted to a baseline.
