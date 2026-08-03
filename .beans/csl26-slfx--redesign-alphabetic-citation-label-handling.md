---
# csl26-slfx
title: Redesign alphabetic citation-label handling and migrate existing styles
status: todo
type: task
priority: deferred
tags:
    - schema
    - engine
    - citations
created_at: 2026-08-03T18:50:04Z
updated_at: 2026-08-03T19:18:41Z
---

Track the follow-up redesign of processor-owned alphabetic `citation-label` handling after numeric citation labels. Make labels declarative while preserving compatibility for existing template-owned labels, and revise those existing styles to use the feature; no new styles are required.

- [ ] Define declarative semantics for citation-label generation and wrapping.
- [ ] Implement schema, migration, renderer, and collapse behavior.
- [ ] Revise existing styles to use the declarative feature and add parity coverage.
