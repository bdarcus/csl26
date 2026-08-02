---
# csl26-7xhp
title: Root-cause concurrent-read race in report-core.js snapshot oracle (exit 2 under default parallelism)
status: completed
type: bug
priority: normal
tags:
    - scorecard
    - scripts
    - flaky
    - ci
created_at: 2026-07-31T14:00:44Z
updated_at: 2026-08-02T15:01:58Z
parent: csl26-6th8
---

Discovered while validating the exact-parity gate for csl26-6th8. The identical `node scripts/report-core.js --all-features --styles ...` invocation (19 embedded-core styles), at the same commit, produced different exactParity totals across two runs at the default --parallelism 4 (e.g. apa-7th total: 146 vs total: 80 on a second run), each time paired with a "Snapshot oracle failed for <style>: exit 2" warning (oracle-fast.js's missing/stale-snapshot exit code) for the affected styles — a partial result that report-core.js does not treat as a run failure. Isolated single-style reruns and --parallelism 1 runs were consistently correct. See docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md, "Determinism" section.

Workaround shipped in the same PR: check-core-quality.js and the fidelity.yml mode=selected gate now hard-fail (rather than silently compare) any style carrying style.error/qualityBreakdown.error, and both CI/justfile invocations pin --parallelism 1.

Acceptance criteria:
- [ ] Identify the concurrent-read race (likely a shared snapshot cache file/handle read by scripts/oracle-fast.js under mapWithConcurrency without per-worker isolation).
- [ ] Fix the race so default parallelism is safe again.
- [ ] Remove the --parallelism 1 pin from justfile's check-core-quality recipe and .github/workflows/fidelity.yml once confirmed stable.
