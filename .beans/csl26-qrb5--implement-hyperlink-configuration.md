---
# csl26-qrb5
title: Implement hyperlink configuration
status: todo
type: feature
priority: normal
created_at: 2026-02-12T22:06:57Z
updated_at: 2026-02-12T22:06:57Z
---

Add link configuration to styles for hyperlink generation:

- Link target: url, doi, url-or-doi, pubmed, etc.
- Anchor text: title, url, doi, whole-entry
- URL construction from DOI/PubMed IDs

Example YAML:
```yaml
links:
  target: url-or-doi  # prefer url, fallback to DOI
  anchor: title       # link the title text
```

Requires:
- DOI deserialization fix (csl26-j9ej)
- Schema extension for link configuration
- Renderer support for hyperlinks (HTML, Typst)

Refs: csln#155
