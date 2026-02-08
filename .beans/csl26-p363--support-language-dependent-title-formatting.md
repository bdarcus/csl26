---
# csl26-p363
title: Support language-dependent title formatting
status: todo
type: feature
priority: normal
created_at: 2026-02-07T06:53:34Z
updated_at: 2026-02-08T21:04:12Z
parent: csl26-m3lb
---

In current CSL it's impossible to apply different rules for title-casing to title vs book-title. Common for edited volumes in German containing English articles.

Requirements:
- Entry-level language field support (biblatex/CSL-M pattern)
- Language-specific formatting rules per field
- Locale-specific template sections (CSL-M pattern)
- Support for multilingual documents with field-level language tagging

Impact: Multilingual bibliographies
Effort: 1-2 weeks

Refs: GitHub #139, csln#66, GitHub #97