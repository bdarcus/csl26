---
# csl26-pf22
title: docs/compat.html report cache/baseline stale relative to fresh runs
status: todo
type: bug
priority: normal
tags:
    - reporting
    - fidelity
created_at: 2026-08-01T12:26:19Z
updated_at: 2026-08-01T12:26:19Z
---

Discovered while verifying csl26-1hya (punctuation-in-quote fix): running node scripts/report-core.js --style springer-basic-brackets against a fully clean checkout (no code changes at all, .report-cache cleared) produces exactParity 20/67 (rate 0.299), but the currently committed docs/compat.html shows 49/67 (rate 0.731) for the same style. Confirmed via direct citum binary render that citum's own output is byte-identical between the old and new commits for this style -- the discrepancy is entirely on the reporting side (cached oracle/pairing results, or citeproc-js/fixture drift since the report was last committed), not an engine regression. Also observed for taylor-and-francis-national-library-of-medicine (fidelity 0.97 in committed HTML vs lower on a fresh run) with byte-identical citum output. A full docs/compat.html regeneration should NOT be bundled into an unrelated engine-fix PR until this staleness is understood -- investigate whether .report-cache entries need invalidation, whether citeproc-js/fixtures changed since the baseline was committed, or whether the fuzzy-pairing algorithm is non-deterministic across runs.
