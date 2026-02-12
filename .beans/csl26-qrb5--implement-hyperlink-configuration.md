---
# csl26-qrb5
title: Implement hyperlink configuration
status: completed
type: feature
priority: normal
created_at: 2026-02-12T22:06:57Z
updated_at: 2026-02-12T22:45:00Z
---

Add link configuration to styles for hyperlink generation:

- Link target: url, doi, url-or-doi, pubmed, etc. ✅
- Anchor text: title, url, doi, whole-entry ✅
- URL construction from DOI/PubMed IDs ✅

## Hyperlink Configuration Guide

CSLN supports declarative hyperlink generation through the `links` configuration. It can be applied globally in `options` or locally on template components.

### 1. Global Configuration (Automatic Linking)

Setting `links` in global `options` automatically applies links to components that match the specified `anchor`.

```yaml
options:
  links:
    target: doi      # Link to DOI (https://doi.org/...)
    anchor: title    # Automatically link ALL Title components
```

**Supported Targets:**
- `url`: Use the `url` field from the reference.
- `doi`: Use the `doi` field, prefixed with `https://doi.org/`.
- `url-or-doi`: Prefer `url`, fall back to `doi`.
- `pubmed`: Use reference ID if it starts with `pmid:`.
- `pmcid`: Use reference ID if it starts with `pmc:`.

**Supported Anchors:**
- `title`: Links `Title` components.
- `url`: Links the `URL` variable component.
- `doi`: Links the `DOI` variable component.
- `entry`: Links the entire bibliography entry.

### 2. Local Configuration (Fine-grained Overrides)

You can override or specifically enable links on any template component.

```yaml
bibliography:
  template:
    - title: primary
      links:
        target: url
        anchor: component # Link THIS component
```

**Local Anchors:**
- `component`: Always links the current component if a target URL can be resolved.
- `title`, `url`, `doi`: Links if the component matches that type.

## Implementation Plan

1. **Schema Expansion (`csln_core`)**:
   - Define `LinkTarget` and `LinkAnchor` enums in `options/mod.rs`. ✅
   - Update `LinksConfig` and add to all relevant `TemplateComponent` types. ✅
   - Add global `links` to `Config`. ✅
2. **Processor Enhancement (`csln_processor`)**:
   - Implement URL construction and target resolution logic in `values` module. ✅
   - Handle "whole-entry" linking. ✅
3. **Renderer Support**:
   - Verify/Implement link wrapping in HTML and Djot renderers. ✅
4. **Style Migration**:
   - Update styles in `styles/` to use new config. ✅
5. **Verification**:
   - Add integration tests for targets/anchors. ✅

Example YAML:
```yaml
links:
  target: url-or-doi  # prefer url, fallback to DOI
  anchor: title       # link the title text
```

Requires:
- DOI deserialization fix (csl26-j9ej) ✅
- Schema extension for link configuration
- Renderer support for hyperlinks (HTML, Djot)

Refs: csln#155
