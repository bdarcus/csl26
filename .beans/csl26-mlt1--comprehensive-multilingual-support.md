---
# csl26-mlt1
title: Comprehensive Multilingual Support
status: completed
type: feature
priority: high
created_at: 2026-02-11T15:35:00Z
updated_at: 2026-02-12T12:00:00Z
---

Implement "elegant" multilingual support in CSL Next, moving away from procedural logic toward a declarative, type-safe system handling parallel metadata.

Status:
* ✅ Core Data Model: `MultilingualString` and `MultilingualName` (holistic names) implemented with explicit language/script tagging.
* ✅ Style Schema: `multilingual` options added (`title-mode`, `name-mode`, `preferred-script`, `script-config`).
* ✅ Input Support: CSL-JSON and BibLaTeX conversion updated with `language` field support.
* ✅ Verification: Comprehensive deserialization and merging tests added.
* ✅ Expert Feedback Integration: BCP 47 variant tags for transliteration methods, disambiguation via displayed strings (not PIDs).
* ✅ Documentation: MULTILINGUAL.md Section 1.3 (transliteration methods), Section 5 (disambiguation strategy), tests for multiple transliteration methods.
* ✅ Project Integration: Added Multilingual Support principle to CLAUDE.md Development Principles.
* ⏳ Processor Logic (csl26-mlt2): Implement value resolution and script-aware ordering in `csln_processor`.
* ⏳ Sorting (future): Integrate ICU4X for UCA-based locale-aware sorting.

Refs: docs/architecture/MULTILINGUAL.md, CLAUDE.md
