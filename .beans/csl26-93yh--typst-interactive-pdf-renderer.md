---
# csl26-93yh
title: Typst Interactive PDF Renderer
status: todo
type: feature
priority: normal
created_at: 2026-02-16T02:56:33Z
updated_at: 2026-02-16T02:56:33Z
blocking:
    - csl26-li63
---

## Overview

Add Typst as a fourth OutputFormat implementation (alongside Plain, HTML, Djot) to produce beautifully typeset PDFs with advanced interactive features. This positions CSLN as a modern alternative to LaTeX/biblatex for academic document preparation.

**Key Innovation:** Accept ANY input format (Djot, Markdown, reStructuredText, etc.) → Convert to Typst markup → Compile to interactive PDF with professional typography.

**Status:** OutputFormat trait already exists in `crates/csln_processor/src/render/format.rs` with working Plain, HTML, and Djot renderers. This bean adds Typst as the fourth implementation.

## Interactive PDF Features

* **Clickable Citations:** Forward links from in-text citations to bibliography entries
* **Bidirectional Navigation:** Backward links from bibliography to citation locations (with page references)
* **Live External Links:** DOI/URL hyperlinks to external resources
* **Metadata Tooltips:** Hover tooltips showing reference metadata (if Typst annotation API supports)
* **Professional Fonts:** Optional use of high-quality fonts (Adobe Minion Pro, Garamond Premier Pro, etc.) if installed

## Architecture

```
Input Document (Djot/Markdown/etc.)
    ↓
CSLN Processor → TypstRenderer (implements Renderer trait)
    ↓
Typst Markup (.typ file)
    ↓
Typst Compiler (external process)
    ↓
Interactive PDF
    • Citation → bibliography links
    • Bibliography → citation backlinks
    • External DOI/URL hyperlinks
    • Metadata annotations (if supported)
    • Professional typography with font control
```

## Implementation Steps

1. Create `Typst` struct implementing `OutputFormat` trait in `crates/csln_processor/src/render/typst.rs`
2. Implement trait methods (text, emph, strong, small_caps, quote, link, etc.)
3. Add Typst variant to `DocumentFormat` enum
4. Add CLI integration (spawn `typst compile` process via std::process::Command)
5. Implement DOI/URL hyperlinks via `#link(url)[text]` syntax
6. Generate unique bibliography anchor IDs for forward navigation using `#label(<id>)`
7. Implement two-pass rendering for citation → bibliography links
8. Add bibliography → citation backlinks with page references
9. Research Typst annotation API for metadata tooltips
10. Add font configuration (specify custom font paths/families in generated .typ file)
11. Build PDF validation test suite (verify links/tooltips in Adobe Acrobat, Preview.app, Evince)
12. Document Typst installation, font setup, and configuration

## Font Support

* Detect installed professional fonts (Adobe Minion Pro, Garamond Premier Pro, EB Garamond, etc.)
* Allow style authors to specify preferred font families
* Fallback to Typst default fonts if professional fonts unavailable
* Document font licensing and installation for style authors

## Input Format Flexibility

Any document format can be converted to Typst for PDF output:
* **Djot** (default, clean semantic markup)
* **Markdown** (CommonMark, GFM)
* **reStructuredText** (Python documentation standard)
* **AsciiDoc** (technical writing)
* **Org-mode** (Emacs users)

The renderer accepts CSLN-processed citations/bibliographies and embeds them into the document structure, regardless of input format.

## Dependencies

* **OutputFormat trait** (already exists in `crates/csln_processor/src/render/format.rs`)
* Typst compiler (external binary, must be installed by user)
* Professional fonts (optional, user-provided)

## Success Criteria

* APA 7th style → Typst → PDF with:
  - Working citation → bibliography links
  - Working bibliography → citation backlinks
  - DOI/URL hyperlinks functional
  - Bidirectional navigation tested in Adobe Acrobat, Preview.app, Evince
  - Professional font rendering (if Adobe Minion Pro installed)
* Support at least 2 input formats (Djot + Markdown)
* Documentation for Typst installation and font configuration

## Alignment

* Issue #105 (pluggable renderers)
* Issue #155 (hyperlink configuration)
*docs/architecture/PRIOR_ART.md (citeproc-rs/jotdown renderer trait pattern)
* CLAUDE.md hybrid processing goals (batch + interactive modes)

## Estimated Complexity

Medium (7-9 weeks):
* Basic links: straightforward (Typst native)
* Tooltips: research required (Typst annotation API)
* Two-pass rendering: manageable for large bibliographies
* Font integration: configuration + documentation

## Risks & Mitigations

* **Risk:** Typst annotation API may not support tooltips
  - **Mitigation:** Fallback to text-based metadata display
* **Risk:** Two-pass rendering for large bibliographies (1000+ refs)
  - **Mitigation:** Benchmark and optimize if needed, most academic docs <100 refs
* **Risk:** Font licensing restrictions for distribution
  - **Mitigation:** Document user-provided font installation, don't bundle fonts

