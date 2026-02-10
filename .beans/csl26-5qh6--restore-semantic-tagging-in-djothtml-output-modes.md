---
# csl26-5qh6
title: Restore semantic tagging in Djot/HTML output modes
status: todo
type: bug
priority: normal
created_at: 2026-02-10T16:22:34Z
updated_at: 2026-02-10T16:22:34Z
---

The introduction of list/group rendering using PlainText internally caused Djot and HTML output modes to lose semantic tagging for authors and issued dates. A generic refactor of ProcTemplateComponent or a TargetFormat-aware rendering pipeline is needed to fix this correctly without massive code duplication.
