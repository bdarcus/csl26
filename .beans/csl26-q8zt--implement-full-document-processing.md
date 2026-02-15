---
# csl26-q8zt
title: Implement full document processing
status: completed
type: feature
priority: normal
created_at: 2026-02-07T06:53:47Z
updated_at: 2026-02-14T00:00:00Z
parent: csl26-ismq
---

Implemented a prototype WinnowCitationParser for Djot citation syntax ([@key]{.cite}) which enables full document processing.

Completed:
- Added `Processor::process_document` for end-to-end rendering.
- Implemented `WinnowCitationParser` for robust Djot/Pandoc-style citation extraction.
- Supports multi-cites, prefixes, suffixes, and basic locators.
- Added `csln doc` CLI command.

Requirements:
- Support for processing complete documents (not just individual citations/bibliography)
- Integration with document formats (Djot integration)
- Proper handling of citation context within paragraphs
- Output formatting for different document types (Plain, HTML, Djot)

Impact: Core functionality for end users
Effort: 2-4 weeks (Prototype completed in 1 day)

Refs: GitHub #140, csln#86, GitHub #99