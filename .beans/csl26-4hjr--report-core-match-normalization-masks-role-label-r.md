---
# csl26-4hjr
title: report-core match normalization masks role-label regressions
status: in-progress
type: bug
priority: normal
created_at: 2026-07-07T18:09:13Z
updated_at: 2026-07-28T11:58:48Z
---

scripts/report-core.js normalizes rendered output before comparison aggressively enough that role-label differences vanish: citeproc ' (Eds.)' vs Citum ' (eds.)' vs missing label all count as match=True (found while validating csl26-xve4/PR #1028 — raw text changed in 22 style/suite groups while pass counts moved +2). The stored 'oracle'/'citum' entry fields are the normalized strings, so the report can't distinguish 'label rendered wrong' from 'label rendered right' from 'label missing'. Consider: keep lenient match as a component-level score but add a raw byte-equality tier (rawMatch appears to exist but was True for divergent strings — audit it), and store raw strings in entries.



Implementation checklist:
- [ ] Preserve raw oracle and Citum comparison strings.
- [ ] Add exact-text normalization and parity state without changing lenient matching.
- [ ] Surface lenient near-matches as exact-parity drift.
- [ ] Add regression coverage for role-label, bracket, punctuation, and case drift.

Specification: `docs/specs/STYLE_COMPATIBILITY_INHERITANCE_REPORT.md`
