---
# csl26-2vcg
title: Type the punctuation join sites' separators
status: completed
type: task
priority: normal
tags:
    - punctuation
    - multilingual
    - style
created_at: 2026-08-01T11:58:12Z
updated_at: 2026-08-01T19:25:12Z
---

Enabler for PUNCTUATION_NORMALIZATION.md phase 3: replace the character-sniffing in citum-engine's punctuation-in-quote join sites (default_separator.chars().next(), first_visible_char) with typed marks per docs/specs/PUNCTUATION_REALIZATION.md. Not required to fix punctuation-in-quote itself (csl26-1hya) -- realize_wrap returns None for WrapPunctuation::Quotes, so quote glyphs never touch the realization table. Follow-up from csl26-1hya.

## Todo
- [x] Retitle bean to reflect the real task (title described already-shipped increment 3)
- [x] Add RealizedPunctuation decomposition to render/format.rs
- [x] Add PunctuationClass enum to render/punctuation.rs
- [x] Thread decomposed value through push_delimiter, join_with_quote_movement, grouped/core.rs group join
- [x] Widen BibliographyConfig::separator and entry_suffix to DelimiterPunctuation + From<String> impl
- [x] Update citum-migrate construction sites
- [x] Route bibliography.rs append_rendered_component / entry-suffix arm / component_starts_new_sentence
- [x] Update processor/rendering/grouped/sentence_initial.rs caller
- [x] New unit tests for RealizedPunctuation
- [x] just pre-commit green (fmt, clippy -D warnings, 2331 tests)
- [x] just schema-gen (docs/schemas/style.json only; no reference-docs diff)
- [x] Old-vs-new binary parity sweep: 31 embedded styles, zero byte diff (citations+bibliography)

## Status
Paused for external review before pre-commit/schema-gen/commit. Code changes are in place on branch feat/csl26-2vcg-typed-punctuation-join-separators (uncommitted, working tree). Whole workspace compiles clean (cargo check --workspace --all-features --tests). New RealizedPunctuation unit tests added but not yet confirmed passing (test run was in progress when paused).

## Remediation in progress (post Codex review)
- [x] Removed RealizedPunctuation::class() (dead/wrong-for-CJK accessor), trimmed test column
- [x] Fixed citum-migrate to keep extracted CSL separator literal (Custom), not semantic; added 2 regression tests
- [x] Fixed clippy single_match_else in realize_bibliography_punctuation
- [x] Added 2 e2e tests: semantic bibliography separator/entry-suffix realization (CJK full-width + Latin half-width)
- [x] cargo fmt clean
- [x] cargo clippy --all-targets --all-features -D warnings clean
- [x] cargo nextest run: 2331 passed
- [x] just schema-gen
- [x] old-vs-new binary parity sweep
- [x] commit + PR (user approved)

## Summary of Changes

Replaced glyph-sniffing (`.chars().next()`) at the five punctuation-in-quote join sites with a decomposed `RealizedPunctuation` type (core char + tail), built once at realization from the already-typed `DelimiterPunctuation` mark. Widened `BibliographyConfig::separator`/`entry_suffix` from `Option<String>` to `Option<DelimiterPunctuation>` so bibliography-level separators/suffixes can now be authored as semantic marks (`{ mark: comma }`), realized per item script, matching the delimiter/prefix/suffix token form template fields already had.

Scope was deliberately narrowed to separators/delimiters only (confirmed with user) -- the rendered-affix side (`first_visible_char`) is out of scope since it requires structured component output, filed as a follow-up.

Codex review caught three real issues before commit, all fixed:
1. Removed a `RealizedPunctuation::class()` accessor that reclassified the *rendered glyph* (wrong for CJK, unused/dead code) rather than exposing anything semantically meaningful -- the mark identity was never actually lost since callers already hold the source `DelimiterPunctuation`.
2. Fixed `citum-migrate` silently promoting literal CSL delimiters (","/"."),  into semantic marks via `from_csl_string`, which would have let migrated styles start realizing full-width under a later `realization-default: cjk`. Restored literal (`Custom`) output, added 2 regression tests.
3. Ran `just schema-gen` (missed before first review pass).

Verification: cargo fmt/clippy(-D warnings)/nextest all green (2331 tests), plus 2 new e2e tests (semantic bibliography separator/suffix realizing full-width for CJK, half-width for Latin) and 2 new citum-migrate regression tests. Confirmed zero-byte parity across all 31 embedded styles via direct old-binary-vs-new-binary render diff (citations+bibliography) in a disk-backed git worktree at HEAD.
