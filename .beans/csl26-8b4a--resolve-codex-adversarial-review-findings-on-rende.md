---
# csl26-8b4a
title: Resolve Codex adversarial-review findings on render-when specs
status: in-progress
type: task
priority: high
tags:
    - engine
    - architecture
    - schema
    - fidelity
    - style
created_at: 2026-09-06T23:10:33Z
updated_at: 2026-09-06T23:18:37Z
---

Codex adversarial review of docs/render-when-alternatives-decision returned needs-attention (4 findings). Plan verified all 4 plus 3 more the advisor cross-check surfaced. See /home/bruce/.claude/plans/do-another-style-improvement-purring-eclipse.md.

## Todo
- [x] ALTERNATIVES.md: fix evaluation-rule wording (leaf vs group semantics)
- [x] ALTERNATIVES.md: fix Implementation Notes to name Renderer/grouped/core.rs, not values/
- [x] ALTERNATIVES.md: add tracker clone-and-discard-on-loss rule
- [x] ALTERNATIVES.md: drop the NLM-DOI worked example (belongs to ArticleJournalNoPageFallback, not alternatives)
- [x] ALTERNATIVES.md: note place-unknown term doesn't exist yet
- [x] ALTERNATIVES.md: expand Acceptance Criteria with new behavior-test cases
- [x] MEDIUM_DESIGNATOR.md: replace exclude-types with container_title_category != Default
- [x] MEDIUM_DESIGNATOR.md: replace access_phrase/SubstituteMessage with term.retrieved locale-override composition
- [x] MEDIUM_DESIGNATOR.md: correct ArticleJournalNoPageFallback cross-reference
- [x] MEDIUM_DESIGNATOR.md: add fixture-driven residual-risk acceptance criterion
- [x] Correct decision record's 'Verified before recommending it' paragraph
- [x] File bean: tracker-merge-before-empty-check quirk (under csl26-8m2p) -- csl26-2hr4
- [x] File bean: extend ArticleJournalNoPageFallback for volume absence (NLM/CSE) -- csl26-8z39
- [ ] Re-run Codex adversarial review to confirm resolution
