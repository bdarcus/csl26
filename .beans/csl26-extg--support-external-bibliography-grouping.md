---
# csl26-extg
title: Support document-level bibliography grouping configuration
status: todo
type: feature
priority: normal
created_at: 2026-02-16T16:15:00Z
updated_at: 2026-02-16T16:45:00Z
---

Allow bibliography grouping criteria to be defined within the document (e.g., in Djot metadata or via placeholders), enabling idiosyncratic grouping needs for a specific paper without modifying the style. This mimics Biblatex's capability to customize bibliography output per-document.

**Requirements:**
- **Processor Extension**: Update `Processor` to accept an optional `groups` override that takes precedence over the style-defined groups.
- **Document Metadata**: Update the processor's document handling to extract `bibliography` configuration from YAML frontmatter (e.g., in `.djot` files).
- **Bibliography Placeholders**: Support `::: bibliography :::` blocks in Djot documents that can take attributes for filtering/selection (e.g., `::: bibliography {type=legal-case title="Table of Cases"} :::`).
- **Partial Rendering**: Support rendering multiple bibliographies within a single document based on different selectors.
- **CLI Integration**: Add a CLI flag (e.g., `--bib-config`) to pass a YAML file containing document-specific bibliography settings.

**Related:** csl26-group
