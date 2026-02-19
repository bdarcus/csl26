---
# csl26-tuk7
title: Multilingual disambiguation keys
status: completed
type: task
priority: normal
created_at: 2026-02-16T14:06:41Z
updated_at: 2026-02-16T14:06:41Z
---

Phase 1: Add multilingual-aware key generation for disambiguation.

Files:
- crates/csln_processor/src/processor/disambiguation.rs

Tasks:
1. Add render_name_for_disambiguation() method
2. Update make_group_key() to use multilingual rendering
3. Add unit tests for multilingual key generation

Acceptance:
- Vietnamese names match when style shows given-family order
- Japanese romanization matches when style shows Hepburn transliteration
- Chinese transliteration matches original characters

Refs: docs/architecture/DISAMBIGUATION_MULTILINGUAL_GROUPING.md Phase 1
