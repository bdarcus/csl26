---
# csl26-83ut
title: pop_left_context reads raw last_char under markup, breaking note-marker placement
status: todo
type: bug
priority: low
tags:
    - punctuation
    - rendering
    - engine
created_at: 2026-09-04T11:42:20Z
updated_at: 2026-09-04T11:52:22Z
---

Follow-up from csl26-el8r. Sibling bug to the one that bean fixed, found while
verifying it, one file over.

pop_left_context (crates/citum-engine/src/processor/document/note_support.rs:407)
peels trailing whitespace, a closing quote glyph, and movable punctuation off
the accumulated document (`result`) before placing a note marker, via three
raw `result.chars().last()` reads (:408, :413, :419). When the prose ends in
markup (e.g. HTML/LaTeX/Typst/Djot/Markdown output where the accumulated
document ends `...said."</em>`), all three reads see the markup's closing
character, find neither quote nor punctuation, and the entire punctuation/
quote/note-marker relocation silently no-ops.

Unlike csl26-el8r's sites, this is not a mechanical helper swap to
last_visible_char. Two findings from reading the re-emission half
(render_note_reference_in_prose, note_support.rs:349-357) make it a design
task:

1. Pop-and-append moves punctuation out of its markup. The function pops the
   quote and punctuation off `result`, then re-emits with
   `result.push_str(&inside); result.push(quote_char); result.push_str(&outside)`
   -- at the string's *current physical end*. A visible-aware pop of
   `...said."</em>` yields `...said</em>`, and the re-append gives
   `...said</em>".[^1]`: the author's quote and period have left the emphasis
   span. move_punctuation_into_quote (render/punctuation.rs) deliberately does
   the opposite -- it inserts at the raw position *inside* the markup. Getting
   this right needs pop_left_context to hand back a raw insertion offset (not
   just the popped chars) and the re-emission to insert_str at it, per
   NoteRule/NoteOrder variant -- not a swapped helper.

2. F is the wrong lens for most of this string. notes.rs:129,150 always emit
   the note marker as Markdown `[^n]` regardless of F, so `result` is a
   Markdown document body with F-rendered citations spliced into it.
   Html::visible_runs (or any format's) over an author's Markdown prose is a
   category error; F is only authoritative for the rendered-citation segments
   that precede the marker when an integral anchor was just pushed
   (notes.rs:123), not when the preceding text is author prose (notes.rs:105).

## Remediation sketch

Step 1 (do before any code): produce a table of expected raw output for
`...said."</em>` under each NoteRule combination (PunctuationRule x
NumberRule x NoteOrder; note_support.rs:360,374,390). Starting hypothesis to
confirm/refute against citeproc-js or pandoc note output: restore the quote +
its punctuation at the raw offset the original cluster occupied (inside the
markup); place the [^n] marker at the accumulated end (outside trailing
markup) since an emphasised footnote marker isn't intended. This splits the
cluster, which the current single-append `inside + quote + outside` shape
can't express -- expect render_note_reference_in_prose's signature to change.
Also settle the F-vs-Markdown-lens question above.

Step 2: add `last_visible_char_and_raw_range::<F>` to render/punctuation.rs,
mirroring the existing first_visible_char_and_raw_range (:116) and its
is_fully_visible::<F> fast path (:85). Reuse visible_projection (:65,
pub(crate)) directly in pop_left_context rather than a closure-driven
abstraction -- the three-stage state machine (whitespace -> quote ->
punctuation) has stage-specific stop conditions and reads better inline. One
visible_projection pass per pop_left_context call, not one per pop (result is
the whole document so far).

## Not in scope here

inspect_right_context (note_support.rs:428) is the symmetric right-hand read
and has the same class of gap, but a different remediation shape: its
consumed_len is a raw byte length fed to last_idx, so consuming a visible `.`
that sits after markup ([@smith]</em>.) would swallow </em> and break the
document. Leaving it raw is strictly additive -- the left-side fix adds
handled cases without creating new breakage. Track as a further follow-up,
not silently bundled here.

## Blast radius / risk

Only the five formats overriding visible_runs (html.rs:270, typst.rs:285,
djot.rs:145, latex.rs:255, markdown.rs:189) can change behavior; PlainText is
bit-identical (what report-core.js/oracle.js measure). Watch: these lexers
were written for engine-generated output, not author prose -- a Markdown
document ending `*"quoted"*` no-ops today (raw last char is `*`); after the
fix the visible last char is `"` and the relocation fires. Needs explicit
tests/document.rs coverage, not just an assumption.

## Investigation update (2026-09-04) — before writing code

Followed up on the design questions above with concrete evidence rather than
speculation. Corrects two things and settles two others.

**1. Confirmed latent, not active.** `NoteRule` resolution
(notes.rs:476-491) falls through to locale grammar defaults
(`GrammarOptions::note_punctuation/note_number/note_marker_order`,
`crates/citum-schema-style/src/locale/types.rs:623-635`) unless a style sets
`options.notes.*` -- and no style in `styles/*.yaml` does (`grep -rln
"^notes:" styles/*.yaml` is empty). The type defaults are `Adaptive` /
`Outside` / `After` (`crates/citum-schema-style/src/options/mod.rs:589-626`).
Under `Outside`+`After`, `desired_note_side` always returns `Outside`
regardless of quote detection, and the plain-append fallback path
(`note_support.rs:353-357`) is what a correctly-Outside/After placement would
also produce for the common "author types the citation marker right after the
closing quote" case -- so today's silent no-op is currently
indistinguishable from correct output for every embedded style. This is the
same shape as csl26-el8r itself: a real latent gap, invisible to the parity
sweep and the fixture corpus, not an active regression.

**2. Corrected: the lens is not "always Html".** Original note above
proposed treating the fix as Html-vs-not; that was wrong on inspection of the
markup lexers, in two ways:

- `result` at the point `pop_left_context` runs is the raw *document source*
  regardless of `DocumentFormat`/`F` -- traced end to end from
  `Processor::process_document` (pipeline.rs) through
  `render_document_body::<F>`. For `DocumentFormat::Html`, content is *not*
  yet HTML; conversion happens later via `finalize_html_output`, after
  citations are spliced in as placeholders specifically so a later Djot/
  Markdown inline pass doesn't mangle them. For the Typst/Latex + note-style
  path, content also stays in source form (comment at pipeline.rs:337-339:
  "note styles still emit source footnote syntax the terminal body renderer
  does not yet model"). So `F` never matches what `result` is actually
  written in.
- More importantly, I originally guessed Markdown/Djot's own emphasis syntax
  (`*word*`, `_word_`) wouldn't matter since it's single ASCII chars already
  visible to the raw checks -- **wrong**. Both `Markdown::visible_runs`
  (render/markdown.rs:189) and `Djot::visible_runs` (render/djot.rs:145)
  *do* strip `*`/`**` (and Djot also `_`) as invisible markup, alongside raw
  `<tag>` HTML passthrough. So `*"quoted."*` -- the bean's original repro
  shape -- genuinely is fixed by using the document's own source lexer, and
  is not a narrower case than first scoped.
- There are exactly two `CitationParser` implementations
  (`processor/document/markdown.rs`, `processor/document/djot/mod.rs`), so
  the correct lens is a small closed choice between the `Markdown` and
  `Djot` `OutputFormat`s' `visible_runs` -- selected by which source parser
  `P` produced the document, not by `F` (the citation-rendering format) or
  `DocumentFormat` (the final target format). None of `pop_left_context`,
  `render_note_reference_in_prose`, or their caller in `notes.rs:126`
  currently has access to that choice -- `CitationParser` doesn't expose it
  as a marker; it would need to be threaded down, e.g. an associated
  `OutputFormat` type or a small enum passed alongside `F`.

**3. Placement question resolved by a no-regression argument, not an 18-row
table.** Dropped the "verify against citeproc-js" idea -- citeproc-js has no
document-level note-marker placement logic; there is no external oracle for
this. Instead: two candidate re-emission placements for the recovered
`quote`+`punctuation` cluster --
(a) whole `inside + quote + outside` cluster at the raw pop offset (inside
    trailing markup), or
(b) `inside + quote` at the raw offset, `outside` after trailing markup.
For the default rule (`Adaptive`/`Outside`/`After`) on `*"said."*[@x]`,
(a) would italicize the note marker (regression against today's output);
(b) is byte-identical to today. (b) is therefore the only non-regressing
choice and is confirmed, not merely hypothesized. The one open content
question, only relevant if a style ever sets a non-default `order`/
`punctuation` rule: for `NoteOrder::Before` + `PunctuationRule::Outside`,
does the punctuation that joins the marker outside leave the emphasis too
(`*"said"*[^1].`) or stay inside it (`*"said"[^1].*`)? Deferred -- no shipped
style exercises it, so no answer is needed to close this bean's design
question for the current corpus.

## Revised remediation shape

1. Add a source-lexer selector (`Markdown` vs `Djot` `OutputFormat`, chosen
   from `P`, not `F`) and thread it to `pop_left_context` /
   `render_note_reference_in_prose` / `notes.rs:126`.
2. `pop_left_context` returns raw insertion offsets alongside the popped
   quote/punctuation (via a `last_visible_char_and_raw_range`-style helper on
   `render/punctuation.rs`'s existing `visible_projection`), not just the
   characters.
3. Re-emission builds `inside + quote` and `insert_str`s it at the raw
   offset, then `push_str`s `outside` at the current end -- placement (b)
   above. A test should assert that for fully-visible input the raw offset
   equals `result.len()`, so `insert_str` degenerates to today's `push_str`
   and the fully-visible-tail behavior stays byte-identical.
4. `inspect_right_context`'s matching gap stays a further, separate
   follow-up (different remediation shape -- see "Not in scope" above).

Given (1) is now confirmed latent-only for the entire current corpus, and (2)
above shows the fix requires wiring a new source-lexer selector through
several call layers rather than a local rewrite, this is materially bigger
than the "stacked PR, ship soon" framing it was approved under. Flagged back
to the user before starting implementation.
