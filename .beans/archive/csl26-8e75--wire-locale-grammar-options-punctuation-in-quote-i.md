---
# csl26-8e75
title: Wire locale grammar-options punctuation-in-quote into resolve_punctuation_defaults
status: completed
type: task
priority: normal
tags:
    - punctuation
    - engine
created_at: 2026-08-01T11:58:12Z
updated_at: 2026-08-13T13:46:07Z
---

crates/citum-engine/src/processor/setup.rs resolve_punctuation_defaults only resolves strong_terminal_comma_policy and delimiter_suppressing_terminal_marks from locale grammar-options, not punctuation_in_quote, even though en-US.yaml declares punctuation-in-quote: true. Wiring it would flip the default for every style that doesn't set punctuation-in-quote explicitly -- a cross-style parity event, not a bug fix, so it needs its own review pass. Follow-up from csl26-1hya.

## Checklist
- [x] Add `resolved_by_fallback` field to `Locale` + `resolved_for` helper
- [x] Flag both loader fallback sites (Locale::load, citum_store::load_locale_or_default)
- [x] Wire punctuation_in_quote in resolve_punctuation_defaults, guarded on locale authority
- [x] Fix punctuation_defaults_require_resolution to match
- [x] Rewrite stale doc comments (options/mod.rs, PUNCTUATION_NORMALIZATION.md, AUTHORING_LOCALES.md)
- [x] New unit tests (locale loader flag, engine resolution, rendered-output)
- [x] Update existing test expectations against citeproc-js, full nextest green
- [x] Baseline report-core run (before)
- [x] After report-core run + diff; confirm embedded-parity-baseline.json unchanged
- [x] Byte-level confirm mhra-notes unchanged + flipped styles correct
- [x] just pre-commit green; just schema-gen ran (style.json doc-comment text updated as expected, locale.json/docs-reference unaffected)
- [x] Two commits + branch + PR (branch: csl26-8e75/wire-locale-punctuation-in-quote, PR #1179)

## Summary of Changes

Wired `punctuation-in-quote` into `Processor::resolve_punctuation_defaults` alongside the two grammar options it already resolved. A style that leaves the option unset now inherits it from the active locale's `grammar-options.punctuation-in-quote` (en-US: true, most other bundled locales: false), matching citeproc-js; a style that sets it explicitly is never overridden.

Added `Locale::resolved_by_fallback` (set at both silent-fallback loader sites: `Locale::load`, `citum_store::load_locale_or_default`) so the engine can tell a locale that was actually resolved from one substituted after a failed lookup, and guarded all three grammar-option resolutions on it — a style declaring an unbundled locale (mhra-notes' en-GB) keeps its current rendering rather than silently inheriting en-US punctuation.

Measured before/after with `report-core.js --all-features`: only `oscola`/`oscola-no-ibid` (+1/81 exact parity each) and one `thomson-reuters-legal-tax-and-accounting-australia` citation entry changed; every other style in the 35-style corpus, including `mhra-notes`, was byte-identical. No embedded-tier style moved (verified by walking every extends chain before implementing), so `embedded-parity-baseline.json` needed no regeneration; `check-core-quality.js` output is byte-identical before/after.

Two commits: `33b3e58c` (locale fallback flag) and `e1369f1c` (engine wiring, docs, tests). Full workspace `cargo nextest run`: 2513/2513 passing. `just pre-commit` green. PR: https://github.com/citum/citum-core/pull/1179

Deliberately out of scope, both already tracked: csl26-yxay (`bool` → `Option<bool>` so an explicit `false` can override a locale `true`) and csl26-dnzc (bundle en-GB so mhra-notes gets correct British punctuation instead of just staying unaffected).
