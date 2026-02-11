---
# csl26-mlt1
title: Comprehensive Multilingual Support
status: todo
type: feature
priority: high
created_at: 2026-02-11T15:35:00Z
updated_at: 2026-02-11T15:35:00Z
---

Implement "elegant" multilingual support in CSL Next, moving away from procedural logic toward a declarative, type-safe system handling parallel metadata.

Requirements:
* Parallel metadata fields (Original, Transliteration, Translation)
* Declarative style options for script selection (title-mode, name-mode)
* UCA-based locale-aware sorting (ICU4X)
* PID-based disambiguation

Refs: docs/architecture/MULTILINGUAL.md
