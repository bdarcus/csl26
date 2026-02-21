---
# csl26-zoou
title: 'Design decision: citation-level suppress-author'
status: todo
type: task
priority: normal
created_at: 2026-02-21T14:07:32Z
updated_at: 2026-02-21T14:07:32Z
---

Moved suppress-author from per-item (ItemVisibility on CitationItem) to citation-level (suppress_author: bool on Citation). Applies uniformly to all items in a citation.

Rationale: suppress-author is a sentence-level decision (the author is already named in prose), not a per-item one. Mixed-visibility citations have no real-world use case -- the oracle fixture case was LLM-generated for coverage, not from actual authoring scenarios. CSL 1.0's per-item design was driven by Zotero's UI checkbox model, not semantic necessity.

If you need to reverse: add ItemVisibility enum back to CitationItem, remove suppress_author from Citation, update RenderOptions.visibility, and update oracle fixture. Commit to reverse: run 'git log --oneline' on branch refactor/macro-test-refactoring, the forward commit is a0de41d.
