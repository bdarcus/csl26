---
# csl26-group
title: Implement configurable bibliography grouping
status: todo
type: feature
priority: normal
created_at: 2026-02-15T00:00:00Z
updated_at: 2026-02-15T00:00:00Z
---

Develop a more flexible and configurable system for bibliography grouping. 

Current implementation provides basic hardcoded support for separating visible vs silent (nocite) citations under an "Additional Reading" heading. This should be generalized.

Requirements:
- Allow styles to define arbitrary bibliography groups in YAML.
- Support grouping by:
    - Citation status (cited, uncited/nocite).
    - Reference type (e.g., separate section for "Primary Sources" or "Datasets").
    - Custom metadata fields.
- Support configurable headings for each group.
- Support group ordering.
- Ensure bibliography sorting rules apply within each group.

Architecture thoughts:
- Add a `groups` field to `BibliographySpec`.
- Generalize the filtering logic in `Processor::render_grouped_bibliography_with_format`.
- Consider how to handle items that might match multiple groups (default to first match or allow duplicates?).
