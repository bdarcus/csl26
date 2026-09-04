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
updated_at: 2026-09-04T11:42:26Z
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
