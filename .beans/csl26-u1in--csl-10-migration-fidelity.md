---
# csl26-u1in
title: CSL 1.0 Migration & Fidelity
status: todo
type: epic
priority: critical
created_at: 2026-02-07T12:11:33Z
updated_at: 2026-02-07T12:11:33Z
blocking:
    - csl26-yxvz
---

Parser completeness, migration fidelity, oracle verification, and style coverage. This epic tracks the critical path to achieving high-fidelity CSL 1.0 → CSLN conversion, enabling the In-Text Styles Foundation milestone.

Goals:
- 95%+ component coverage (no silent data loss)
- 90%+ oracle match rate across top 50 styles
- 100% fidelity for top 10 parent styles (APA, Elsevier, Springer, IEEE, etc.)
- Robust migration debugger and verification tools

**Multilingual Portability (Frank Bennett insight):**
- Detect CSL 1.0 macros with prefix/suffix attributes during migration
- Emit warnings about potential multilingual punctuation conflicts
- Where possible, hoist punctuation to parent delimiter attributes
- Ensures migrated CSLN styles work across multilingual rendering modes

Refs: STYLE_PRIORITY.md, docs/RENDERING_WORKFLOW.md, Frank Bennett CSL-M guidance