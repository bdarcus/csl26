---
# csl26-ro72
title: Fix third-round Codex adversarial-review findings
status: in-progress
type: task
priority: high
tags:
    - schema
    - fidelity
    - style
    - engine
    - architecture
created_at: 2026-09-07T00:49:24Z
updated_at: 2026-09-07T00:49:40Z
---

Third Codex adversarial review of docs/render-when-alternatives-decision (after the second fix pass, csl26-la9t) found 4 more findings (2 high, 2 medium) -- deeper than the first two rounds: a worked example was not semantics-preserving, and a wire contract couldn't load through the actual schema types.

## Todo
- [x] ALTERNATIVES.md: replace Chicago volume-title worked example (dropped a real interacting part-number-non-numeric guard) with a re-verified T&F-CSE publisher-place example
- [x] ALTERNATIVES.md: withdraw 'valid anywhere' claim; add v1 placement restriction (no nesting, not primary title/contributor, not inside article-journal type-variant templates); file csl26-57a7 for the 18-file consumer audit
- [x] ALTERNATIVES.md: add TemplateResourceBudget accounting requirement for the candidate list
- [x] MEDIUM_DESIGNATOR.md: fix wire example to mapping-shaped SubstituteMessage fields ({message: ...}, not bare scalars)
- [x] MEDIUM_DESIGNATOR.md: wire online_access into BibliographyOptions AND BibliographyConfig AND the hand-written to_bibliography_config() conversion (all three, following article_journal's precedent)
- [x] MEDIUM_DESIGNATOR.md: replace container_title_category anchor rule with reference.container_title().is_some(); found and documented a narrower legal-type (LegalCase/Statute/Regulation/Treaty) edge case in the replacement
- [ ] Re-run Codex adversarial review a fourth time to confirm resolution
