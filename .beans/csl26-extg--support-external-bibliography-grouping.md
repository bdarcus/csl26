---
# csl26-extg
title: Support external idiosyncratic bibliography grouping
status: todo
type: feature
priority: normal
created_at: 2026-02-16T16:15:00Z
updated_at: 2026-02-16T16:15:00Z
---

Enable bibliography grouping to be configured outside the style (e.g., in the bibliography file) to support idiosyncratic grouping needs that are not specified in a style guide.

**Requirements:**
- Add `groups` field to `InputBibliography` in `csln_core`.
- Update the processor's `render_grouped_bibliography_with_format` to prioritize these external groups over style-defined ones.
- Ensure CLI can load and pass these groups to the processor.

**Related:** csl26-group
