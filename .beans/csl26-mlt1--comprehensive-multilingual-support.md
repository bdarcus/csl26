---
# csl26-mlt1
title: Comprehensive Multilingual Support
status: in-progress
type: feature
priority: high
created_at: 2026-02-11T15:35:00Z
updated_at: 2026-02-12T10:00:00Z
---

Implement "elegant" multilingual support in CSL Next, moving away from procedural logic toward a declarative, type-safe system handling parallel metadata.

Refined Design:
* Parallel metadata fields using `MultilingualString` (untagged enum)
* Holistic `MultilingualName` approach for contributors (original, transliterations, translations as full `StructuredName` objects)
* Tagged maps for `translations` (keyed by LangID) and `transliterations` (keyed by script/tag)
* Explicit `original` and `lang` fields in complex objects
* Declarative style options for script selection (title-mode, preferred-script)
* Script-aware ordering (FamilyGiven vs GivenFamily) and delimiters
* UCA-based locale-aware sorting (ICU4X)
* PID-based disambiguation

Refs: docs/architecture/MULTILINGUAL.md
