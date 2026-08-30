---
# csl26-rgys
title: '9 embedded styles lost collapse: citation-number in migration'
status: completed
type: bug
priority: normal
tags:
    - style
    - migrate
    - chicago
created_at: 2026-08-26T22:30:13Z
updated_at: 2026-08-30T22:41:26Z
parent: csl26-awlo
---

Cross-checked every style's info.source.csl-id against styles-legacy/*.csl <citation collapse=...>: 9 styles declare a collapse in CSL and have none in the YAML. 5 embedded -core styles already carry processing: numeric (elsevier-vancouver-core, elsevier-with-titles-core, springer-basic-brackets-core, springer-vancouver-brackets-core, taylor-and-francis-national-library-of-medicine-core) — one-line fix, add collapse: citation-number. 2 have no processing: key at all (american-medical-association-alphabetical, american-society-of-mechanical-engineers), which is why extract_citation_collapse (crates/citum-migrate/src/assembly.rs:707) dropped it — adding processing: numeric changes more than collapse, needs per-style oracle evidence. entomological-society-of-america declares collapse="year" (same-author mechanism, not citation-number) — separate check. american-mathematical-society-label is correctly absent (inert Label regime). Overlaps surface #9 of csl26-awlo's audit; wait for that spec before implementing so the fix lands in the coherent model, not ad hoc.

## Summary of Changes

6 of 9 styles fixed with `collapse: citation-number` added to their `citation:` block; 2 adjudicated out with measured evidence; 1 confirmed as already-correct with no gap.

**Fixed (6):** elsevier-vancouver-core, elsevier-with-titles-core, springer-basic-brackets-core, springer-vancouver-brackets-core, taylor-and-francis-national-library-of-medicine-core (the 5 embedded `-core` styles the bean identified, one-line `collapse: citation-number` each) + american-medical-association-alphabetical (extends `american-medical-association`, which already carries `processing: numeric` — no `processing:` re-declaration needed, just the collapse field). Confirmed with a synthetic 3-item multi-cite render: `[1],[2],[3]` → `[1–3]` for all 6. Oracle: identical pass/fail counts to baseline for all 6 (0 regressions).

**Adjudicated out (american-society-of-mechanical-engineers):** `collapse: citation-number` would be inert. This style's citation numbers render doubly-bracketed (`[[1],[2],[3]]`) via a pre-existing, unrelated engine/YAML-interaction defect — `should_collapse_citation_numbers` (`crates/citum-engine/src/processor/rendering/mod.rs`) requires `CitationMode::NonIntegral`, which this style never reaches. Confirmed the blocker is NOT `processing:` inheritance (explicitly added `options.processing: numeric` to ASME's own file as a test — no change in output) and NOT the redundant `citation.wrap: punctuation: brackets` (removed it as a test — no change either). Root cause is deeper than this bean's scope. Filed **csl26-aafz** with full reproduction; a code comment at `styles/american-society-of-mechanical-engineers.yaml:34-41` points to it and explains why `collapse:` isn't added.

**Confirmed no gap (entomological-society-of-america):** declares `collapse="year"` (same-author mechanism, `collapse: same-author: {}` in YAML terms, per the `apa-7th.yaml` precedent). Tested adding it explicitly — zero output difference in both the full oracle corpus and a synthetic same-author/multi-year probe (`(Smith 2001, 2003)` identical before and after). This style already renders the collapsed form without the field. Left absent with an explanatory comment rather than shipping dead config.

**No action (american-mathematical-society-label):** confirmed `label-mode: alphabetic`, not numeric — correctly has no `collapse:` field, matches the bean's original assessment exactly.

Verification: `cargo nextest run -p citum-schema-style -p citum-engine`: 1935/1935 pass. `cargo fmt --check`: clean. Oracle run on all 7 touched legacy CSL sources shows identical pass/fail counts to pre-edit baseline.

## Follow-up fix (post-merge, pre-push CI catch)

Enabling collapse surfaced two further defects not visible in this bean's own verification (which only checked bibliography-side oracle diffs, not the citations-expanded.json corpus):

1. **collapse: citation-number leaked through extends** into american-mathematical-society-label (via elsevier-with-titles -> elsevier-with-titles-core), where citation-number collapse is invalid for label-mode processing. Fixed by explicit `collapse: null` on the leaf style.
2. **Engine bug: numeric collapse required only 2+ consecutive numbers**, not 3+. citeproc-js's `NumericBlob.checkNext` state machine only starts suppressing a run into a range at the *third* consecutive item -- a pair stays comma-separated (`29,30`), never collapses to a hyphenated range (`29–30`). This defect predates this bean (already silently present in american-medical-association's baseline) but was dormant for elsevier-with-titles/springer-basic-brackets until this bean enabled their collapse. Fixed in `crates/citum-engine/src/processor/rendering/collapse.rs` (`block_ids.len() < 2` -> `< 3`). Full 35-style corpus diff: zero regressions, three unrelated improvements (american-chemical-society, gb-t-7714-2025-numeric, royal-society-of-chemistry, +4 exact-parity entries each).
