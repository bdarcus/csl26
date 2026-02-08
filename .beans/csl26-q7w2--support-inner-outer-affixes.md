---
# csl26-q7w2
title: Support inner and outer affixes for wrapped components
status: completed
type: feature
priority: low
created_at: 2026-02-08T12:00:00Z
updated_at: 2026-02-08T12:00:00Z
labels:
    - category: rendering
---

Current `Rendering` logic in `csln_processor` places `prefix` and `suffix` strings
*inside* the characters provided by the `wrap` property.

Example:
- Configuration: `wrap: brackets`, `suffix: " "`
- Current Output: `[value ]`
- Desired Output: `[value] `

We need to evaluate if the `Rendering` struct should distinguish between:
1. **Inner affixes**: Inside the wrap (e.g., `[(ed.)]`).
2. **Outer affixes**: Outside the wrap (e.g., `[1] `).

This would avoid the need for "hacky" prefixes on subsequent components to manage 
spacing after bracketed elements.
