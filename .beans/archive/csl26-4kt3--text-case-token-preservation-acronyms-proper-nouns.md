---
# csl26-4kt3
title: Text-case token preservation (acronyms + proper nouns)
status: completed
type: task
priority: normal
tags:
    - nocase
    - policy
    - engine
    - title
created_at: 2026-06-21T17:46:30Z
updated_at: 2026-08-11T22:25:09Z
---

Deferred from csl26-maim. Filed against the premise that citeproc-js applies a stop-word/proper-noun heuristic under sentence case, so titles like NIPS->Nips, AI->Ai, and Cambridge->cambridge would be fixed by replicating it.

## Scope correction

That premise does not hold. `CSL.Output.Formatters.sentence` (scripts/node_modules/citeproc/citeproc_commonjs.js:21988) capitalizes only the first word and lowercases every other word, unconditionally — no stop-word list, no lexicon. The stop-word logic (`skipWordsRex`) exists only in `CSL.Output.Formatters.title`, for *title* case, not sentence case. Two of the three original examples were already resolved before this bean was picked up:

- Audit row 135 (title-case lowering NIPS->Nips): fixed by commit 34181280 (2026-07-04), which added `has_internal_uppercase` guarding `to_title_case_with_language_id`.
- Audit row 213 (Springer Vancouver sentence-casing Cambridge->cambridge): the style no longer applies `text-case: sentence` at all — styles-legacy/springer-vancouver-brackets.csl and styles/embedded/springer-vancouver-brackets-core.yaml both use only capitalize-first.

A proper-noun heuristic for sentence case was evaluated and rejected: a single-capital word (`Cambridge`) is structurally indistinguishable from title-case source data without a lexicon; it is wrong outside English (German capitalizes every noun); and Citum already has the correct, explicit mechanism — data-declared `.nocase` protection (Djot spans, CSL-JSON `<span class="nocase">`, biblatex braces), which survives sentence case end-to-end. `Cambridge`/`La Ciotat`-class proper nouns remain out of scope by design: they need `.nocase` markup in the source data, not an engine guess. Recorded as policy: docs/policies/TEXT_CASE_PROTECTION.md.

What *was* still a real, fixable bug — and is what this bean closes on — was a genuine engine inconsistency: sentence case had two divergent implementations depending on whether a title happened to contain Djot markup. The plain-text path (to_sentence_case_with_language_id) preserved internally-capitalized words (NIPS, AI, iPhone) via has_internal_uppercase; the Djot-markup path (make_case_transform, values/title.rs) flat-lowercased every non-first text leaf regardless of casing, so the same title rendered differently depending on an unrelated emphasis span. Root: crates/citum-engine/src/values/text_case.rs, crates/citum-engine/src/values/title.rs.

## Summary of Changes

- Extracted a shared, script-agnostic word-level sentence-case rule (`sentence_case_words`, crates/citum-engine/src/values/text_case.rs) and rewired both the plain-text path and the Djot-markup path (`make_case_transform`, crates/citum-engine/src/values/title.rs) to use it. Fixes the bean's own NIPS/AI examples wherever the title contains Djot markup (emphasis, strong, etc.) — previously only the plain-text path preserved them.
- Added regression tests: `crates/citum-engine/src/values/text_case.rs` (word-level acronym preservation), `crates/citum-engine/src/values/tests.rs` (markup-path acronym preservation + an explicit plain-vs-markup equivalence assertion). Existing `.nocase` protection tests (mRNA, DNA, leading-nocase-advances-state) verified unaffected.
- Added docs/policies/TEXT_CASE_PROTECTION.md recording the rejected heuristic and enumerating every case-transform surface, plus the deliberate citeproc-js divergence (internal-caps preservation) so a future parity sweep doesn't revert it unknowingly.
- Corrected the stale comment in styles/embedded/taylor-and-francis-chicago-author-date-core.yaml, which promised removing its monograph/container title-case workaround "once csl26-4kt3 lands" — that was never possible (Cambridge/La Ciotat have no internal capitalization to guard on), so it now points at data-side .nocase protection as the actual mechanism. The workaround itself (title-case for monograph/container titles) stays.

Verification: `just pre-commit` green (2453 tests), `just check-core-quality` passed (35 styles, fidelity=1.0), before/after `report-core.js --all-features` diff shows zero fidelity/exact-parity movement across all styles.
