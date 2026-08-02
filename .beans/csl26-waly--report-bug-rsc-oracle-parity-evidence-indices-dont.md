---
# csl26-waly
title: 'Report bug: RSC oracle-parity evidence indices don''t match rendered output'
status: todo
type: bug
priority: low
created_at: 2026-07-30T14:28:57Z
updated_at: 2026-08-02T12:57:36Z
parent: csl26-ccdt
---

royal-society-of-chemistry's oracleDetail entries show a Citum-side bibliography number present on some entries and absent on others, all tagged evidenceRunId: baseline (looks like within-run inconsistency). Direct CLI rendering (citum render refs -s styles/royal-society-of-chemistry.yaml --mode bib, and --mode both with a citations fixture) produces zero numbered entries, consistently, for every reference -- the numbered oracleDetail entries don't correspond to what the style actually renders against references-expanded.json. Likely an evidence-index mislabeling/merging bug in report-core.js's benchmark-run assembly, not a rendering defect. Not root-caused; time-boxed out of the 2026-07-30 class-A spike. See docs/architecture/audits/2026-07-30_EMBEDDED_PARITY_CLASS_A.md finding 3.
