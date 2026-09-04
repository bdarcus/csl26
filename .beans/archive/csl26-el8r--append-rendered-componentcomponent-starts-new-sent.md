---
# csl26-el8r
title: append_rendered_component/component_starts_new_sentence read raw last_char, not visible
status: completed
type: task
priority: low
tags:
    - punctuation
    - rendering
    - engine
created_at: 2026-08-03T14:05:53Z
updated_at: 2026-09-04T11:42:02Z
---

Follow-up from csl26-l4tv. Part B of that fix made the *closing-quote* side of punctuation-in-quote markup-aware (move_punctuation_into_quote, ends_with_close_quote), but append_rendered_component and component_starts_new_sentence (render/bibliography.rs) still call entry_output.chars().last() directly -- a raw read. Under Html (or any format with trailing markup after the visible last char, e.g. a closing </span>/</em>), last_char is always the markup's closing character (>) and never whitespace, which affects:
- The "both sides non-space, insert separator" branch in append_rendered_component
- The final whitespace-driven decision in the "leads with whitespace, period-prefixed separator" branch
- component_starts_new_sentence's own last_char-based checks (though ends_with_sentence_ending_visible_punctuation and ends_with_close_quote already went through the visible-text fix)

Not fixed as part of csl26-l4tv because the two-arm parity sweep across 19 embedded styles x citations+bibliography (old-vs-new binary, plain/html/latex/typst/djot/markdown) found zero diffs attributable to this path -- it doesn't appear to be exercised by any embedded style's current entry shapes. File this as a latent correctness gap, not an active bug: fix if/when a style or fixture exposes it, using last_visible_non_space_char (already exists, used elsewhere in the same file) as the replacement.

## Summary of Changes

Bean was stale: the production fix already landed in `6e99c724a fix(engine):
punctuation-in-quote under markup` (PR #1135), as a review follow-up committed
~1 hour after this bean was filed. All three sites the bean names now go
through `last_visible_char::<F>` (added by that same commit):

- `append_rendered_component`'s "both sides non-space" branch (:455)
- `append_rendered_component`'s "leads with whitespace, period-prefixed
  separator" branch (:458)
- `component_starts_new_sentence`'s own last-char checks (:146)

`last_visible_char::<F>` -- not this bean's suggested
`last_visible_non_space_char` -- is the correct replacement: these branches
are whitespace-sensitivity checks (deciding whether to insert a separator
based on whether either side is already spaced), so collapsing whitespace
via the non-space variant would defeat them. `last_visible_non_space_char`
remains correctly used elsewhere in the same file for punctuation
deduplication, a genuinely different question.

What was actually missing: `append_rendered_component` had a markup-aware
regression pin
(`append_rendered_component_reads_the_visible_last_char_not_the_markups_raw_last_byte`,
:1262, Html) added alongside the original fix, but every existing
`component_starts_new_sentence` test was `PlainText`-only -- a format where
raw and visible reads are identical, so none of them would have caught a
revert of :146. Added
`given_html_entry_output_when_checking_sentence_start_then_reads_visible_not_raw_last_char`
(2 rstest cases, Html) to close that gap. Verified it actually pins the fix
by temporarily reverting :146 to a raw `chars().last()` read and confirming
case 1 fails, then restoring.

While verifying, found a genuinely separate but related latent defect one
file over -- `pop_left_context` in
`processor/document/note_support.rs` has the same raw-last-char-under-markup
class of bug for note-marker placement. Filed as its own bean (successor,
same tag set) since remediation there is a design task, not a helper swap --
see that bean for detail.

`cargo test -p citum-engine --lib bibliography::` -- 50 passed, 0 failed.
