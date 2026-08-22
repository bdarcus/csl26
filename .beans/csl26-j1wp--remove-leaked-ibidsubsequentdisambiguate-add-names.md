---
# csl26-j1wp
title: 'Fix chicago-shortened-notes-bibliography-core: remove ibid/subsequent grammar leaked from chicago-notes-18th'
status: in-progress
type: bug
priority: high
tags:
    - style
    - fidelity
    - chicago
created_at: 2026-08-08T12:30:24Z
updated_at: 2026-08-21T18:13:16Z
parent: csl26-h7oc
blocked_by:
    - csl26-adka
---

chicago-shortened-notes-bibliography-core.yaml should stop rendering "Ibid." for an immediately repeated citation, and should not apply chicago-notes-18th's full→short position-based name-shortening either — matching its own CSL source, which has no position-dependent citation grammar at all.

Evidence, checked directly against the shipped `.csl` (not the style-variant-builder template/diff, which shows derivation structure but not exact grammar content):
- `styles-legacy/chicago-shortened-notes-bibliography.csl:4184`'s `<citation>` layout calls `citation-notes-shortened-author-title` unconditionally — no `position=` branch anywhere in it. No lexical "Ibid." appears anywhere in the file outside one unrelated book-review sub-case (`description-review-short`).
- Contrast with `chicago-notes.csl:4937`, which does branch on `position="subsequent"` (full form first, shortened form after) with its own `et-al-subsequent-min`/`et-al-subsequent-use-first` attributes. chicago-shortened-notes-bibliography-core is right to differ from chicago-notes-18th here, not just inherit it.
- `chicago-shortened-notes-bibliography-core.yaml` currently defines its own `citation.ibid` block (an "Ibid." template) and defines no `citation.subsequent` of its own, so per `NOTE_SHORTENING_POLICY.md` rule 4 it inherits `chicago-notes-18th`'s `citation.subsequent` (family-only name form + per-type `add: locator` patches) as the ibid fallback too. Neither matches the source. Symptom: `note-disambiguate-year-suffix` drops the author name on the second of two citations where oracle repeats it.
- `docs/specs/NOTE_SHORTENING_POLICY.md`'s normative family model classifies "Chicago shortened-note family" as using lexical ibid for immediate repeats — the opposite of what the shipped CSL does. Reconcile this alongside csl26-adka (same tension, full-note family) rather than deciding it in isolation here.

Likely fix: delete `citation.ibid` from `chicago-shortened-notes-bibliography-core.yaml`, and override `citation.subsequent` so it no longer inherits chicago-notes-18th's family-only shortening — the style's base citation form should apply uniformly regardless of position.

- [ ] Land csl26-adka first (chicago-notes-18th's own ibid removal) — this bean's fix and verification build on that one
- [ ] Delete the `citation.ibid:` block
- [ ] Override `citation.subsequent` so repeats use the base citation form, not chicago-notes-18th's inherited shortening — confirm what mechanism actually clears an inherited nested block in this schema before assuming a bare `subsequent: null` does it
- [ ] Confirm locator (page-number) rendering still comes out correctly across the reference types that currently hit the ibid/subsequent blocks
- [ ] Re-render the `chicago-shared-corpus` fixtures; grep the citation-side failure diffs for "Ibid" to see how much of the style's current 13/473 exact-parity gap this explains
- [ ] `node scripts/report-core.js` and `cargo nextest run` — confirm no exact-parity regression elsewhere
- [ ] `note-disambiguate-add-names-et-al` (Citum expands "Smith et al." to "Smith, Lee, Kumar, et al." where oracle doesn't) is a separate, unresolved symptom — `disambiguate-add-names="true"` is a correct, expected attribute on this style's `<citation>` element (shared template default, also present on chicago-notes.csl), so the fix here is not "suppress it." Note whether this fix changes that symptom at all; if not, it needs its own investigation.
- [ ] Note the outcome in this bean's summary when done

\n\nOutcome (Chicago style-only wave, 2026-08-21): Implemented shortened-notes citation isolation in chicago-shortened-notes-bibliography-core.yaml by clearing inherited ibid, subsequent, and type-variant citation grammar. Fresh shortened report remains stable on the family floor and the final style wave is 62/473 exact parity.

\n\nThe YAML inheritance isolation is complete and measured. The existing note-style conformance/test expectations still encode lexical ibid behavior; leave this bean open for the excluded Rust/test-contract follow-up.
