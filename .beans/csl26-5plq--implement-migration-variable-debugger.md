---
# csl26-5plq
title: Implement migration variable debugger
status: todo
type: feature
priority: normal
created_at: 2026-02-07T06:53:18Z
updated_at: 2026-02-07T12:11:40Z
parent: csl26-u1in
---

Add --debug-variable flag to csln_migrate to trace variable provenance.

50% reduction in migration debugging time expected.

Features:
- Track CSL source nodes → intermediate → final YAML
- Show deduplication decisions
- Display override propagation
- Output ordering transformations

Example:
csln_migrate styles/apa.csl --debug-variable volume

Output shows: Source CSL nodes, compiled template position, rendering options, overrides

Refs: GitHub #124, WORKFLOW_ANALYSIS.md Phase 2