---
# csl26-1hya
title: Fix punctuation-in-quote at all join boundaries
status: completed
type: bug
priority: high
tags:
    - rendering
    - punctuation
    - engine
    - multilingual
created_at: 2026-08-01T11:15:31Z
updated_at: 2026-08-01T12:29:00Z
---

Chicago author-date and other punctuation-in-quote styles put trailing period/comma outside the closing quote in three of the engine's join sites: prefix-led incoming components (Gap A, render/bibliography.rs append_rendered_component ordering), group: delimiters (Gap B, values/list.rs TemplateGroup::values bare fmt.join), and non-en-US quote glyphs (Gap C, hardcoded '"'/'\u{201D}' matching instead of locale QuoteMarks). See docs/specs/PUNCTUATION_NORMALIZATION.md and plan at /home/bruce/.claude/plans/in-the-chicago-author-date-quirky-ullman.md.

## Checklist
- [x] Extract move_punctuation_into_quote helper in render/punctuation.rs
- [x] Route bibliography.rs append_rendered_component (reorder + boundary-aware)
- [x] Route bibliography.rs entry-suffix arm
- [x] Route citation.rs push_delimiter
- [x] Route grouped/core.rs grouped-author delimiter
- [x] Route values/list.rs TemplateGroup::values join (Gap B)
- [x] Fix misleading doc comment options/mod.rs:159-162
- [x] New unit tests: prefix-led, group-delimiter (biblio + citation), non-default quote marks, disabled-unchanged
- [x] just pre-commit green
- [x] report-core.js / oracle.js explicit --locale commit
- [x] Confirm parity delta matches expected set (broader than originally scoped -- 14+ embedded styles set punctuation-in-quote: true, not just the 4 named families; verified via direct old-vs-new binary render diffs, not the generated report, since docs/compat.html carries unrelated pre-existing cache staleness -- see csl26-pf22)
- [x] Skipped intentionally: docs/compat.html full regeneration surfaces large pre-existing staleness unrelated to this fix (csl26-pf22); reverted to committed baseline rather than bundling an unrelated report refresh into this PR
- [x] File follow-up beans: csl26-2vcg (semantic marks), csl26-8e75 (locale wiring), csl26-yxay (bool->Option<bool>), csl26-t0m4 (chicago broadcast), csl26-4q7v (chicago interview ordering), csl26-dnzc (en-GB locale), csl26-pf22 (report cache staleness)

## Summary of Changes

Fixed punctuation-in-quote at all three engine join boundaries where it silently failed to move a trailing period/comma inside a closing quote:

- Gap A (prefix-led incoming text, e.g. Chicago's broadcast variant with prefix: ". Aired " on the date following a quoted title): render/bibliography.rs::append_rendered_component checked starts_with_separator before the punctuation-in-quote branch, so prefix-supplied punctuation never reached it. Reordered, and extended render/citation.rs::push_delimiter the same way for the citation surface.
- Gap B (group: delimiters): found in TWO separate, duplicate group-join implementations -- values/list.rs::TemplateGroup::values (a narrower message/pattern-fallback path) and the actual top-level renderer processor/rendering/grouped/core.rs::render_group_component_with_format (used for real bibliography/citation group: blocks, e.g. Chicago's interview: variant). Both did a bare fmt.join with zero punctuation dynamics. Fixed both via a shared join_with_quote_movement helper.
- Gap C (non-en-US quote glyphs): all sites matched only straight and curly ASCII quote chars instead of the locale-resolved QuoteMarks.close. Fixed via a shared move_punctuation_into_quote primitive that takes the close-quote string.

Also fixed the misleading doc comment on punctuation_in_quote (options/mod.rs) claiming the en-US locale sets it automatically, which the engine does not implement.

Separately, made the fidelity report locale comparison explicit rather than coincidental: oracle.js gained an opt-in --locale/forceLang path, and report-core.js now resolves each style's declared locale once and passes it to both the citum CLI and citeproc-js -- guarded so a non-embedded declared locale (e.g. mhra-notes' en-GB) does not turn today's silent engine-side fallback into a report-time hard error.

Verification: 15 new unit/integration tests (plain names + rstest per project convention -- BDD naming is reserved for tests/ integration suites, not inline unit tests), including a native-InputReference regression test for the exact real-world Bengio-interview shape that caught Gap B. Confirmed both real-world cases (Chicago broadcast and interview bibliography entries) via direct old-binary-vs-new-binary rendering, not just the generated report -- docs/compat.html was found to carry large pre-existing cache/baseline staleness unrelated to this change (filed as csl26-pf22) and was deliberately left unregenerated in this PR. Full workspace suite (2320 tests), cargo fmt --check, and cargo clippy --all-targets --all-features -- -D warnings all green.
