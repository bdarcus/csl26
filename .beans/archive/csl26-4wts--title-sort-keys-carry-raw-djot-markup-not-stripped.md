---
# csl26-4wts
title: Title sort keys carry raw Djot markup, not stripped/rendered text
status: completed
type: bug
priority: normal
tags:
    - sorting
    - title
    - rendering
    - engine
created_at: 2026-08-14T12:31:11Z
updated_at: 2026-08-20T22:20:26Z
---

title_sort_key_with_options() -> title_sort_text() in crates/citum-engine/src/sort_support.rs (line ~298) uses title.to_string() directly for the non-multilingual branch:

    fn title_sort_text(title: &Title, options: &SortKeyOptions) -> String {
        match title {
            Title::Multilingual(complex) => multilingual_complex_sort_text(complex, options),
            _ => title.to_string(),
        }
    }

This is the raw title string, including any Djot markup ([...]{.nocase}, _emph_, [text](url), etc.) -- it is never passed through the Djot-aware rendering path (crate::render::rich_text / crate::values::title::render_part_with_case) that strips/renders markup for display. A title like "[Library of Congress]{.nocase}" sorts under '[' instead of 'L'.

Affects both:
- GroupSortKeyType::Title comparisons (compare_by_title in sort_partitioning.rs) for any style sorting bibliography entries by title
- The author-substitute fallback to title-as-sort-key (extract_author_sort_key_opt in sorting.rs, EffectivePrimary::Title case) for author-less references

Discovered while fixing csl26-d3kj (Djot markup leaking through the *rendering* of substituted titles). That fix only addressed display text (resolve_title_substitute in crates/citum-engine/src/values/contributor/substitute.rs); it does not touch sort-key derivation, which is a separate code path with its own markup leak.

Fix should strip Djot markup to its visible-text form for sort-key purposes (there is likely a "visible_text" style helper already used elsewhere for LaTeX/markup stripping -- e.g. crates/citum-engine/src/render/latex.rs's visible_text -- worth checking whether an existing Djot equivalent can be reused before writing a new one).

## Summary of Changes

Fixed by stripping Djot markup at the single sort-key funnel point,
`title_sort_text()` in `crates/citum-engine/src/sort_support.rs`, using the
existing `Djot::visible_text` lexer (not the display-render pipeline, which
would leave emphasis delimiters like `_..._` in place for `PlainText`).

- `crates/citum-engine/src/values/title.rs`: promoted `looks_like_djot_markup`
  to `pub(crate)` so sort-key derivation gates on the same "is this Djot?"
  predicate the display path uses.
- `crates/citum-engine/src/sort_support.rs`: `title_sort_text` now strips
  markup from both branches (plain title and `multilingual_complex_sort_text`'s
  `complex.original`) before returning. The strip runs before
  `Locale::strip_sort_articles` in `title_sort_key_with_options`, so a leading
  article hidden behind markup (`[The Library]{.nocase}`) is still stripped.
  Added an `#[rstest]` case table covering `.nocase` spans, emphasis, links,
  markup-before-article ordering, and the literal-`[Dataset]`-bracket
  carve-out (not markup).

All three sort call sites (`sorting.rs::title_sort_key`,
`sort_support.rs`'s author-substitute fallback, `disambiguation.rs`) funnel
through `title_sort_key_with_options`, so no other edit sites were needed.

Verified: `cargo nextest run -p citum-engine sort_support` (26/26 pass),
`just pre-commit` (2659/2659 pass), and `node scripts/report-core.js
--all-features` shows zero drift against a `main` baseline (identical
totals/exactParity/quality scores) — the core-styles fixture corpus has no
title with Djot markup whose sort position this changes.

## Follow-up (not fixed here)

`multilingual_string_sort_text` (same file) has the identical leak for
**contributor name** sort keys — an institutional author authored as
`[IBM]{.nocase}` would sort under `[`. Proposing a follow-up bean.
