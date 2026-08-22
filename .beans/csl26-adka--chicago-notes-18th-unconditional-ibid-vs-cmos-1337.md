---
# csl26-adka
title: 'Fix chicago-notes-18th: shortened form instead of ibid for repeat citations'
status: in-progress
type: bug
priority: normal
tags:
    - style
    - chicago
    - fidelity
created_at: 2026-08-07T18:39:26Z
updated_at: 2026-08-21T18:13:33Z
parent: csl26-h7oc
---

chicago-notes-18th.yaml should stop rendering "Ibid." for an immediately repeated citation and use the same shortened author-title form it already uses for every other repeat citation — matching both its own CSL source and Chicago's own stated preference.

Evidence:
- The style's declared CSL source (`chicago-notes.diff`, csl-id `chicago-notes`) never renders "Ibid." at all — its citation macro doesn't test citation position.
- CMOS18 §13.37 (§12.77 for in-text) states Chicago's preference is shortened citations over "ibid.", partly because "ibid." is only correct when it refers to the immediately preceding note — an adjacency dependency the shortened form doesn't have — and it saves no space over the shortened form. (Manual text not reproduced here; cite by section number only, per `docs/specs/NOTE_SHORTENING_POLICY.md`'s scope note on copyrighted excerpts.)

Likely fix: delete the `citation.ibid:` block (lines 47–54 of `chicago-notes-18th.yaml`). Per `NOTE_SHORTENING_POLICY.md` rule 4, an immediate repeat with no `citation.ibid` block falls back to `citation.subsequent`, which is already the shortened form this style should use.

- [ ] Delete (or otherwise disable) the `citation.ibid:` block so immediate repeats fall back to `citation.subsequent`
- [ ] Confirm locator (page-number) rendering still comes out correctly via that fallback, across the reference types that currently hit the ibid block — don't assume deleting it is the whole fix until this is checked
- [ ] Re-render the `chicago-shared-corpus` fixtures; grep the citation-side failure diffs for "Ibid" to see whether this defect explains any of the style's current 7/15 citation pass rate
- [ ] Confirm `chicago-shortened-notes-bibliography-core` and `chicago-notes-18th-script` (both extend `chicago-notes-18th`) don't regress
- [ ] `node scripts/report-core.js` and `cargo nextest run` — confirm no exact-parity regression elsewhere
- [ ] Note the outcome in this bean's summary when done

\n\nOutcome (Chicago style-only wave, 2026-08-21): Implemented in chicago-notes-18th.yaml by removing the incorrect citation.ibid branch. Fresh notes report contains no lexical Ibid output; exact parity is now 24/72. The report's generic ibid-conformance profile is not authoritative for this style because the shipped CSL and Chicago policy require shortened repeats.

\n\nThe style change is intentional and source-backed, but the existing Rust test format_document_note_style_repeat_citations_produce_ibid still asserts Ibid. Rust/test-contract work is explicitly excluded from PR A; keep this bean open until that follow-up is handled.
