---
# csl26-mlt2
title: Implement multilingual processor logic
status: todo
type: feature
priority: high
created_at: 2026-02-12T00:00:00Z
updated_at: 2026-02-12T00:00:00Z
---

Implement resolve_multilingual_string and resolve_multilingual_name in csln_processor with BCP 47 matching (exact → prefix → fallback).

**Punctuation Portability (Frank Bennett insight):**
- Apply component prefix/suffix AFTER multilingual resolution, not before
- Prevents double-punctuation in combined modes (e.g., "Tanaka Tarō [田中太郎]. . Title")
- Test cases: presets with suffix across all multilingual modes (original/transliterated/combined)

Add integration tests with multilingual reference data.

Refs:docs/architecture/MULTILINGUAL.md Section 3, csl26-mlt1, Frank Bennett CSL-M guidance
