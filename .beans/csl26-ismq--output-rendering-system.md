---
# csl26-ismq
title: Output & Rendering System
status: todo
type: epic
priority: high
created_at: 2026-02-07T12:12:16Z
updated_at: 2026-02-07T12:12:16Z
blocking:
    - csl26-li63
---

Pluggable output formats and document processing integration.

Goals:
- Abstract renderer trait for multiple output formats
- Implement HTML renderer with semantic classes
- Implement Djot renderer with clean markup
- Support full document processing (citations in context)
- Optional: LaTeX, Typst renderers in future

Architecture:
- Trait-based design allows easy format addition
- Semantic markup (csln-title, csln-author, etc.)
- Clean separation: processor → renderer → output

Integration:
- Works with batch mode (CLI/Pandoc)
- Works with server mode (real-time processing)
- Supports round-trip editing (preserve structure)

Refs: csln#105 (pluggable renderers), csln#86 (Djot), PRIOR_ART.md (citeproc-rs/jotdown)