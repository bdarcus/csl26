---
# csl26-wlg7
title: Declare second-field-align on shipped numeric styles
status: completed
type: task
priority: low
tags:
    - style
    - fidelity
    - csl
created_at: 2026-08-20T23:55:42Z
updated_at: 2026-08-21T16:39:02Z
---

Follow-up to csl26-qdff (Implement CSL second-field-align rendering, docs/specs/SECOND_FIELD_ALIGN.md). The mechanism landed mechanism-only: csl-legacy parses the attribute, citum-schema-style declares it, citum-engine renders sibling citum-entry-marker/citum-entry-body HTML slots when declared, citum-migrate extracts it — but no shipped style declares second-field-align, so today's HTML output is unaffected everywhere. This bean is the corpus-adoption pass: declare bibliography.options.second-field-align: flush (or margin) on the ~12 shipped numeric styles whose CSL 1.0 source actually carries the attribute (ieee, american-medical-association, royal-society-of-chemistry, numeric-comp, and the other REFERENCE_MARKERS.md-listed marker styles are good starting candidates — verify against each style's original CSL source, not assumed). This is a visible HTML markup change (new sibling divs replacing flush text concatenation) for every style touched, so it needs its own parity review: node scripts/report-core.js before/after per style, plus a direct render diff to confirm plain-text output is unchanged (the OutputFormat::entry_slots seam guarantees this by construction, but verify empirically). Not urgent.

## Summary of Changes

Declared `bibliography.options.second-field-align: flush` on all 12 shipped numeric styles whose CSL 1.0 source carries the attribute:

- Exemplar (`styles/`): american-chemical-society, american-mathematical-society-label, american-medical-association-alphabetical, american-society-of-mechanical-engineers, nature, royal-society-of-chemistry
- Embedded (`crates/citum-schema-style/embedded/styles/`): american-medical-association, elsevier-vancouver, elsevier-with-titles, ieee, springer-basic-brackets, taylor-and-francis-national-library-of-medicine

Corrections to the bean's original candidate list: `numeric-comp` excluded (no CSL counterpart; also uses `compound-numeric`, which bypasses the `entry_slots` seam entirely) and `alpha` excluded (no CSL counterpart).

Declared explicitly on all 12 rather than relying on `extends` inheritance for the 3 that could have inherited it (ASME←ieee, ama-alphabetical←ama, ams-label←elsevier-with-titles), per user decision.

Updated RSC's prose comment (now stale) and the RSC PlainText test comment in `crates/citum-engine/tests/bibliography.rs` to reflect the declared option instead of asserting it in prose.

### Verification

- Plain-text output: byte-identical to `main` for all 12 styles (confirmed via baseline-worktree render diff).
- HTML output: mathematically confirmed the *only* change is the marker+body fuse becoming sibling `citum-entry-marker`/`citum-entry-body` divs — verified programmatically (strip the wrapper back out, compare to baseline byte-for-byte) across all 47 bibliography entries × 12 styles.
- `elsevier-vancouver-author-date` (inherits the option from `elsevier-vancouver` but its own CSL doesn't carry it) confirmed byte-identical in both plain-text and HTML — inert because `label-mode: author-date` produces no marker.
- Boundary check vs. citeproc-js oracle snapshots: marker-text split position matches citeproc's `csl-left-margin`/`csl-right-inline` boundary for 10/12 styles exactly. The other 2 (`american-mathematical-society-label`, `american-medical-association-alphabetical`) show marker-*content* mismatches (trigraph/alphabetic-numbering divergences) that are pre-existing, already-tracked fidelity gaps (open beans csl26-ssnz, csl26-x9oi) unrelated to this change — not a boundary/split-position bug.
- `node scripts/report-core.js --all-features`: every overall metric (fidelity, exact-parity, pairing, quality) and every per-style metric for the 12 candidates + the inheritance case is identical between baseline and branch — confirms parity is measured on normalized text and this change cannot move it, by construction.
- `just check-core-quality`: passes on both baseline and branch with identical warning sets (including the pre-existing `ieee` preset-usage warning, unrelated to this change).
- `cargo nextest run --all-features`: 2708/2708 passed. `cargo clippy --all-targets --all-features -- -D warnings`: clean. `cargo fmt --check`: clean. `just schema-gen`: no diff (schema landed with csl26-qdff).
