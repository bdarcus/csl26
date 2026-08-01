---
# csl26-2jls
title: 'Audit: CSL-direct interpretation vs. CSL→Citum translation'
status: completed
type: task
priority: normal
tags:
    - engine
    - fidelity
    - research
    - migrate
created_at: 2026-08-01T20:13:18Z
updated_at: 2026-08-01T20:15:15Z
---

Bruce asked whether citum-migrate's plateau (28.5k LOC, diminishing returns) argues for a different path: extending citum-engine to interpret CSL 1.0 directly instead of translating to Citum YAML. The 2026-07-17 strategic review (csl26-bv8w) never evaluated this — it compared deterministic conversion vs. hand-tuning, both translation. Produce a date-stamped audit in docs/architecture/audits/ that: corrects the 42.6% embedded-tier exact-parity headline (Chicago cluster is half the denominator and one row is a duplicate measurement via extends), shows the plateau claim is unfalsified given AMA's 23.9%->71.6% move, concedes interpretation's real advantage (avoids migrate's decompilation problem) while naming its real cost (permanent dual semantics, can't replace the oracle), and recommends a single parity-targeted tuning wave on 'ieee' as the discriminating measurement before any interpreter work is scoped. Relates to csl26-6th8 (exact-parity refocus) and csl26-m2t1 (tuning cost telemetry).

## Tasks

- [x] Verify parity-arithmetic slices against scripts/report-data/embedded-parity-baseline.json (all/Chicago/non-Chicago)
- [x] Confirm taylor-and-francis-chicago-author-date extends chicago-author-date-18th and reports byte-identical exactParity numbers
- [x] Confirm ieee and modern-language-association carry no style-level extends (probe candidates)
- [x] Write docs/architecture/audits/2026-08-01_CSL_DIRECT_INTERPRETATION_ANALYSIS.md
- [x] Run ./scripts/validate-frontmatter.sh --copilot-strict
- [x] Commit (docs-only), push, open PR, watch CI

## Summary of Changes

Published docs/architecture/audits/2026-08-01_CSL_DIRECT_INTERPRETATION_ANALYSIS.md. Verdict: not yet decidable from current evidence. The 42.6% embedded-tier exact-parity headline used to motivate the interpreter idea does not hold — the Chicago cluster (half the denominator) includes a duplicate measurement via extends (taylor-and-francis-chicago-author-date reports byte-identical numbers to its chicago-author-date-18th parent), and the tier carries a known unexplained regression (springer-vancouver-brackets). Non-Chicago slice reads ~64%, not 43%. The plateau claim is also unfalsified: AMA moved 23.9%->71.6% and chicago-author-date-18th 19.3%->30.6% from untargeted work in days. Interpretation's real advantage (skips migrate's decompilation problem, csl-legacy AST already exists) is conceded, but its real cost (permanent dual semantics, cannot replace the citeproc-js oracle it would be judged against) means it should not be scoped from unverified numbers. Recommended a single parity-targeted tuning wave on ieee (verified standalone, 84/149) as the discriminating measurement, with a decision rule fixed in advance, extending the open csl26-m2t1 telemetry work rather than opening a new direction.
