---
# csl26-ro72
title: Fix third-round Codex adversarial-review findings
status: completed
type: task
priority: high
tags:
    - schema
    - fidelity
    - style
    - engine
    - architecture
created_at: 2026-09-07T00:49:24Z
updated_at: 2026-09-07T01:20:07Z
---

Third Codex adversarial review of docs/render-when-alternatives-decision (after the second fix pass, csl26-la9t) found 4 more findings (2 high, 2 medium) -- deeper than the first two rounds: a worked example was not semantics-preserving, and a wire contract couldn't load through the actual schema types.

## Todo
- [x] ALTERNATIVES.md: replace Chicago volume-title worked example (dropped a real interacting part-number-non-numeric guard) with a re-verified T&F-CSE publisher-place example
- [x] ALTERNATIVES.md: withdraw 'valid anywhere' claim; add v1 placement restriction (no nesting, not primary title/contributor, not inside article-journal type-variant templates); file csl26-57a7 for the 18-file consumer audit
- [x] ALTERNATIVES.md: add TemplateResourceBudget accounting requirement for the candidate list
- [x] MEDIUM_DESIGNATOR.md: fix wire example to mapping-shaped SubstituteMessage fields ({message: ...}, not bare scalars)
- [x] MEDIUM_DESIGNATOR.md: wire online_access into BibliographyOptions AND BibliographyConfig AND the hand-written to_bibliography_config() conversion (all three, following article_journal's precedent)
- [x] MEDIUM_DESIGNATOR.md: replace container_title_category anchor rule with reference.container_title().is_some(); found and documented a narrower legal-type (LegalCase/Statute/Regulation/Treaty) edge case in the replacement
- [x] Re-run Codex adversarial review a fourth time to confirm resolution

## Round 4 (fourth Codex review)

Found 3 more findings (1 high, 2 medium):
- [x] ALTERNATIVES.md: v1 placement restriction mixed content and position (forbade the primary title/contributor slot and article-journal top level positionally, while structural consumers walk the whole tree regardless of position) -- rewrote as a pure content restriction: reject any candidate that is/contains Title, Contributor, Date(Issued), Number(Volume), Variable(Url), Variable(Doi), or a pattern.* Message, recursively through Group children and Message args. Added the pattern.* check (template_policy.rs's second structural walk), previously unlisted.
- [x] RENDER_WHEN_CONTRACT.md: frozen-vocabulary note's blanket "url/pages/publisher-place route to alternatives: or work-form routing" claim contradicted the actual per-field routing -- replaced with an explicit routing table (publisher-place->alternatives:, url->MEDIUM_DESIGNATOR.md, pages/volume->ArticleJournalNoPageFallback via csl26-8z39).
- [x] MEDIUM_DESIGNATOR.md: required en-US-nlm.yaml locale-override wasn't wired into the embedded-build registry (EMBEDDED_LOCALE_OVERRIDE_IDS/get_locale_override_bytes in embedded/locales.rs, which today lists only en-US-chicago/de-DE-chicago) -- added as its own Acceptance Criteria item; corrected the cited en-US-ieee/en-US-springer "precedent" language since csl26-fz2e is unimplemented, not a second shipped example.

Per advisor consultation: stopping the review-fix loop here. The four rounds converged from design holes (1-3) to consistency debt (4: one internal contradiction in round 3's own fix, one stale sentence from round 1, one acceptance-criteria omission) -- diminishing-depth findings, not new design flaws. Specs stay Draft; residual gaps (partial alternatives: placement coverage pending csl26-57a7, legal-type anchor edge case, term.place-unknown authoring) are documented in-spec, not hidden. Bruce reviews before promotion to Active.
