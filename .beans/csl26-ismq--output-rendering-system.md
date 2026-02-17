---
# csl26-ismq
title: Output & Rendering System
status: in-progress
type: epic
priority: high
created_at: 2026-02-07T12:12:16Z
updated_at: 2026-02-17T12:00:00Z
blocking:
    - csl26-li63
---

Pluggable output formats and document processing integration.

Goals:
- [x] Abstract renderer trait for multiple output formats
- [x] Implement HTML renderer with semantic classes
- [x] Implement Djot renderer with clean markup
- [x] Implement LaTeX renderer with native escaping
- [ ] Implement Typst renderer
- [x] Support full document processing (citations in context)

Architecture:
- Trait-based design allows easy format addition
- Semantic markup (csln-title, csln-author, etc.)
- Clean separation: processor → renderer → output

Integration:
- Works with batch mode (CLI/Pandoc)
- Works with server mode (real-time processing)
- Supports round-trip editing (preserve structure)

Refs: csln#105 (pluggable renderers), csln#86 (Djot),docs/architecture/PRIOR_ART.md (citeproc-rs/jotdown)
