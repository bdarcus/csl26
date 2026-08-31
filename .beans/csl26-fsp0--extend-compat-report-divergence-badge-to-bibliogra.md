---
# csl26-fsp0
title: Extend compat-report divergence badge to bibliography findings
status: todo
type: task
priority: low
tags:
    - tooling
    - dx
created_at: 2026-08-31T12:24:47Z
updated_at: 2026-08-31T17:53:28Z
parent: csl26-h7oc
blocked_by:
    - csl26-lr1p
---

csl26-lr1p fixes the citation-side 'Citation Findings' table in scripts/report-core.js to show a distinct 'Known Divergence (div-NNN)' status for entries masked by a registered divergence, instead of the misleading generic 'Unresolved Oracle Drift'/'Compatibility Fail'. renderBibliographyEvidence's per-entry table likely has the identical raw-vs-adjusted gap for div-004/005/008/009/010/011 (bibliography-scoped divergences). Out of scope for csl26-lr1p (citation-scoped PR); apply the same fix pattern (merge appliedDivergence from oracleResult.adjusted.bibliography.entries onto the raw bibliography entries by index, branch the status label).
