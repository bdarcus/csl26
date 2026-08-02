---
# csl26-pf22
title: docs/compat.html report cache/baseline stale relative to fresh runs
status: completed
type: bug
priority: high
tags:
    - reporting
    - fidelity
created_at: 2026-08-01T12:26:19Z
updated_at: 2026-08-02T15:01:57Z
parent: csl26-6th8
---

Restore one trustworthy canonical measurement path for embedded parity. The committed artifacts currently disagree: docs/compat.html shows 1260/3255, the CI floor records 1377/3235, and a default-feature diagnostic run at fcfc7407 produced 1415/3235.

The canonical node scripts/report-core.js --all-features --parallelism 1 path also fails because cargo run enables the dhat-heap feature, producing profiler output/artifacts and failed oracle subprocesses. Resolve this together with cache and artifact synchronization rather than refreshing stale output.

Acceptance criteria:
- [ ] The canonical all-features report completes successfully and creates no dhat-heap.json artifact.
- [ ] Cache keys invalidate on all renderer, fixture, locale, and oracle inputs that can change output.
- [ ] A fresh-cache and warm-cache run at the same commit produce identical embedded results.
- [ ] embedded-parity-baseline.json and docs/compat.html are generated from the same successful report commit.
- [ ] csl26-20l7 is re-verified and completed if its remaining report-refocus contract is satisfied.
- [ ] Add regression coverage for the root causes fixed here.
