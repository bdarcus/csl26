---
# csl26-0dca
title: Substituted titles don't get category emph/strong/small-caps rendering
status: completed
type: bug
priority: normal
tags:
    - title
    - substitute
    - rendering
    - engine
created_at: 2026-08-14T12:30:52Z
updated_at: 2026-08-14T20:05:05Z
---

When a style's substitute chain promotes a title into the author slot (e.g. APA's author -> editor -> title fallback for author-less references), the substituted title bypasses TemplateTitle's normal rendering entirely -- it flows through resolve_title_substitute() in crates/citum-engine/src/values/contributor/substitute.rs, not through the title category's TitleRendering (emph/strong/small_caps/vertical_align).

Repro: apa-7th sets titles.monograph.emph: true. An author-less monograph reference renders its title unitalicized in the substitute slot:

    citum render refs -b refs.json -s apa-7th -m bib --json
    # refs.json: {"id":"z","class":"monograph","type":"book","title":"Some Book Title","issued":{"date-parts":[[2020]]}}
    # => "text": "Some Book Title. (2020)"   (APA italicizes monograph titles; should be emphasized)

Compare: the same title WITH an author present correctly gets wrapped via component.rs's rendering.emph application (confirmed via the underscore-wrapped plain-text render: "_Library of Congress and _more__").

Discovered while fixing csl26-d3kj (Djot markup leaking through this same substitute path -- that fix did NOT touch category-level rendering flags, only Djot inline rendering/case/quotes). resolve_title_substitute() would need to consult the title category's TitleRendering (via the same get_title_category_title_rendering used by resolve_effective_title_rendering in title.rs) and apply emph/strong/small-caps through the OutputFormat, mirroring what component.rs does for the normal (non-substitute) title path.

Affects any style with per-category title emphasis (emph/strong/small_caps/vertical_align) applied to a title category that can also be author-substituted -- at minimum apa-7th's monograph/periodical categories.

## Summary of Changes

`resolve_title_substitute()` in `crates/citum-engine/src/values/contributor/substitute.rs`
now applies the title category's `emph`/`strong`/`small-caps` (via
`get_title_category_title_rendering`, the same lookup the normal `title:`
component path uses) whenever the substituted title is not being quoted.
Per div-011's either/or contract, quoting and category emphasis never both
apply to a substituted title — this keeps the historical unconditional-quote
default byte-identical while making bibliography output (never quoted) and
the `by-category` title-quote mode pick up emphasis for the first time.

Correction to the original report: `TitleRendering` (the style's `titles:`
category config) has no `vertical_align` field — that's only on the
per-component `template::Rendering` type — so this fix covers
`emph`/`strong`/`small_caps` only, not `vertical_align`, as the title
originally speculated.

Added 7 rstest/unit cases to `crates/citum-engine/src/values/tests.rs`
covering bibliography emphasis (emph/strong/small-caps), the unchanged
default-quoted citation path, the completed `by-category` quoting+emphasis
mode, and composition with the existing Djot/nocase substitute path
(csl26-d3kj).

Updated `docs/adjudication/DIVERGENCE_REGISTER.md` div-011 to record that
`by-category` is now fully implemented (previously only un-quoted; never
applied category emphasis).

Verified via `just pre-commit` (2540/2540 tests, fmt/clippy clean) and a
before/after `report-core.js --all-features` diff on a detached HEAD
worktree: aggregate fidelity/exactParity counts unchanged on the 35-style
gated corpus (the specific author-less refs this touches aren't in that
corpus's fixture pairing). A direct byte-level sweep across the full
174-ref combined corpus (`tests/fixtures/references-*.json`) confirmed the
expected, small, correct diffs: `apa-7th`/`modern-language-association`
bibliography gain italics on one translator-only book each
(`sr-translator-only`); `chicago-author-date-18th` citations gain italics
on 11 author-less legal-case/serial-component entries via the
`parent-serial` substitute key — consistent with that style's own existing
`emph: true` on `title: parent-serial` elsewhere in the same YAML (e.g. the
`broadcast` type-variant). No other core style in the sweep changed.

Filed a follow-up bean to evaluate `contributors.substitute.title-quote:
by-category` per embedded style (e.g. `apa-7th`), which is what's needed to
get APA-style `(*Some Book Title*, 2020)` in citation context — a separate
style-behavior change with its own parity surface, out of scope here.
