---
# csl26-x08y
title: Support automatic foot/endnoting of citations
status: todo
type: feature
priority: normal
created_at: 2026-02-07T06:53:39Z
updated_at: 2026-02-08T21:04:19Z
parent: csl26-u1in
blocking:
    - csl26-5t6s
---

Ensure the processor supports automatic foot/endnoting to enable seamless style switching between in-text and note styles.

Requirements:
- Citations in manually-placed foot/endnotes (position doesn't change)
- Automatically created footnotes (tool creates/removes notes based on style)
- Corner cases around surrounding punctuation
- Must work in both batch (Pandoc) and interactive (Word/Zotero) workflows

Reference implementations:
- Zotero/citeproc-js
- org-cite oc-csl.el

Impact: Note styles (19% of corpus)
Effort: 2-3 weeks

Refs: GitHub #141, csln#88