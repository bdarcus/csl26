---
# csl26-157x
title: 'docs: restore FEATURES.md feature-status registry'
status: todo
type: task
priority: normal
created_at: 2026-07-27T22:06:04Z
updated_at: 2026-07-27T22:06:04Z
---

docs/reference.html and docs/schemas/index.html link to docs/reference/FEATURES.md, which 404s: it and features.yaml were authored on commit 98d3925b ('docs(docs): restructure docs IA') but that commit is not an ancestor of main -- the branch was reworked and the files were lost. Content is recoverable via 'git show 98d3925b:docs/reference/FEATURES.md' and ':docs/reference/features.yaml', but its since_schema/since_engine/status values are ~2.5 months stale as of 2026-07-27 and need review/correction before republishing (content decision, not mine to make unilaterally). When restored, add the page to build-doc-pages.js's PAGES list rather than linking raw .md (see csl26-6p3d for that pattern). Blocks csl26-0y0z (version badge injection from features.yaml), which depends on this file existing. The dangling links themselves were removed in csl26-6p3d without restoring the content.
