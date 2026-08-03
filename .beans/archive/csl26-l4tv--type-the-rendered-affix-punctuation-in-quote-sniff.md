---
# csl26-l4tv
title: Type the rendered-affix punctuation-in-quote sniffing (first_visible_char)
status: completed
type: task
priority: normal
tags:
    - punctuation
    - multilingual
    - engine
created_at: 2026-08-01T19:25:19Z
updated_at: 2026-08-03T14:06:37Z
---

Follow-up from csl26-2vcg: first_visible_char (render/bibliography.rs) and punctuation.rs's part.chars().next() sniff the already-rendered component string for a self-supplied leading period/comma (e.g. Chicago's prefix: ". Aired "). This can't be typed by widening config -- the affix is baked into the string by the time the join site sees it. Requires ProcTemplateComponent to carry structured affixes and render_component_with_format to return more than a bare String, touching all seven output formats. This is the architectural core of docs/specs/PUNCTUATION_NORMALIZATION.md phase 3. Note: first_visible_char also handles '(' and value-supplied leading characters, so even full phase-3 work likely retains part of it.

## Todo
- [x] Add `RenderedComponent{text}` in render/component.rs (leading_mark field dropped -- see design correction below)
- [x] Thread RenderedComponent through bibliography.rs, citation.rs, punctuation.rs, grouped/core.rs, grouped/sentence_initial.rs, values/list.rs join sites
- [x] Extract `visible_projection` (visible text + raw byte positions) from cleanup_dangling_punctuation, reuse in move_punctuation_into_quote/leading_movable_mark
- [x] Make move_punctuation_into_quote markup-aware (generic over F, projection-based), fold legacy bare-`"` branch through same projection
- [x] Make ends_with_close_quote markup-aware (via visible_text)
- [x] New unit tests: punctuation.rs projection movement (PlainText/Html/Latex), bibliography.rs/citation.rs Html/Latex cases, CJK-shaped leading_movable_mark regression
- [x] Manual e2e verification: chicago-author-date-18th broadcast/quoted-title entries in html now match plain (period moved inside quote)
- [x] just pre-commit green (fmt, clippy -D warnings, 2382 tests)
- [x] just schema-gen: zero diff, confirmed
- [x] Two-arm parity sweep: 19 embedded styles x citations+bibliography, old-vs-new binary. Plain: zero byte diff. Html/djot: 26 diffs, every one verified by character-histogram equality to be a pure mark reposition (no content added/removed). Latex/typst/markdown: zero diff (no semantic wrapping in those backends).
- [x] report-core.js/oracle.js use the default (plain) format -- confirmed via the zero-diff plain arm of the parity sweep, which is exactly what those tools measure
- [x] Manual smoke: chicago-author-date-18th html render shows moved period, matches plain
- [x] docs/specs/PUNCTUATION_NORMALIZATION.md changelog entry added
- [x] Filed follow-up bean csl26-el8r for raw last_char divergence in append_rendered_component/component_starts_new_sentence
- [x] Scope note: bean's "touches all seven output formats" was wrong — visible_runs already exists on the trait; no format-impl changes needed

## Design correction (discovered during implementation)

The approved plan's Part A design typed `RenderedComponent::leading_mark` from
the component's *realized outer `prefix`* alone, gating the punctuation-in-quote
move on that typed field. The full workspace test suite caught this as wrong:
`test_chinese_article_three_part_title` (crates/citum-engine/tests/multilingual.rs)
regressed -- its container-title's leading `, ` comes from something other than
a template-level `rendering.prefix` (a value-extraction prefix or a nested
group's own join), so the typed field was `None` even though a real movable
mark was present, and the move silently stopped firing.

Root cause re-diagnosed: the bean's original complaint ("sniffing is fragile")
was right, but the actual bug is simpler than "can't be typed by widening
config" -- it's that the old sniff compared a **raw** first character against
a **visible** first character (`raw_first_char == Some(first_char)`), which is
never equal once a semantic wrapper's markup precedes the mark. The fix is to
make the *view* consistent (visible vs. visible), not to narrow *what counts*
as a movable mark to one typed source.

Landed instead: `render::punctuation::leading_movable_mark::<F>(text) ->
Option<(char, String)>`, which detects a leading `.`/`,` from the rendered
text's own *visible* content (via the same `visible_runs`-backed projection
Part B uses) and returns it together with the markup-safe remainder. This is a
strict superset of the old raw-sniff behavior -- it catches every source the
old code caught, plus the markup-hidden case the old code missed -- with no
typed field, no gate narrower than "does the visible text start with `.`/`,`".

`RenderedComponent` is kept (satisfies "return more than a bare String" and
avoids a second revert of the 5-file `.text` threading already in place and
compiling) but now carries only `text: String` -- the typing turned out to
buy nothing once the detection had to fall back to visible-content sniffing
for correctness anyway.

Added `push_delimiter_moves_next_part_own_leading_mark_when_it_is_not_a_typed_prefix`
(render/citation.rs) as a unit-level regression pinning this exact case, so it's
cheap to catch next time instead of only surfacing via the CJK integration test.

## Summary of Changes

Fixed the fragile leading-affix/quote-movement sniffing this bean named, and
in doing so found and fixed a deeper bug: punctuation-in-quote was entirely
dead in every markup output format (Html, Latex, Typst, Djot, Markdown, Org),
not just fragile in edge cases. Two independent raw-string assumptions, both
now routed through `visible_runs` (the existing raw-byte-accurate primitive
`cleanup_dangling_punctuation` already used for the same purpose):

- `render::punctuation::leading_movable_mark::<F>(text)` detects a movable
  leading `.`/`,` from a component's *visible* rendered text (not raw), so a
  semantic wrapper's markup preceding it (`apply_component_semantics` wraps
  *after* affixes are applied) no longer defeats detection.
- `move_punctuation_into_quote::<F>` locates the closing quote glyph via the
  same visible projection, so trailing markup (`</span>`, `\emph{...}`, a
  LaTeX `}`) no longer defeats the move.

`RenderedComponent { text: String }` threads through the join sites
(bibliography.rs, citation.rs, punctuation.rs, grouped/core.rs,
grouped/sentence_initial.rs, values/list.rs) so a future increment has a
named type to extend, satisfying the bean's "return more than a bare String"
without an invasive typed-mark design that turned out to be wrong (see design
correction above) -- no format-trait change, no per-format-impl edits.

Verification: `just pre-commit` green (fmt, clippy -D warnings, 2382 tests);
`just schema-gen` zero diff; two-arm old-binary-vs-new-binary parity sweep
across 19 embedded styles x citations+bibliography x 6 output formats --
plain byte-identical (the safety net; also what report-core.js/oracle.js
measure, so fidelity/exact-parity numbers are unaffected), html/djot show 26
diffs all verified by character-histogram equality to be pure mark
repositioning with zero content added or removed, latex/typst/markdown zero
diff (no semantic wrapping in those backends so nothing to fix). Manual smoke
against the original repro (`chicago-author-date-18th` html render) confirms
html now matches plain.

Filed csl26-el8r for a related but unexercised raw-`last_char` divergence in
the same file, found but out of scope (zero diffs attributable to it in the
sweep).
