---
# csl26-5plq
title: Implement migration variable debugger
status: todo
type: feature
priority: normal
created_at: 2026-02-07T06:53:18Z
updated_at: 2026-02-07T07:40:14Z
blocking:
    - csl26-hz9n
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