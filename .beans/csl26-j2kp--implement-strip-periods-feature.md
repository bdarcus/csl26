---
# csl26-j2kp
title: Implement strip-periods feature
status: todo
type: feature
priority: high
created_at: 2026-02-14T12:33:07Z
updated_at: 2026-02-14T12:33:07Z
---

Add strip-periods support at three tiers (global, context, component) following CSLN options architecture. Needed for springer-basic-author-date and 1,600+ CSL styles.

Scope:
1. Schema: Add to csln_core Config/Rendering
2. Processor: Implement stripping in term/label/date rendering
3. Migration: Extract from CSL XML
4. Tests: Rendering + migration coverage
5. Docs: Update RENDERING_WORKFLOW.md

Refs: springer-basic-author-date migration (79% → ~93% with this)
