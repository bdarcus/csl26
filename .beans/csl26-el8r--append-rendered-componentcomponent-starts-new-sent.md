---
# csl26-el8r
title: append_rendered_component/component_starts_new_sentence read raw last_char, not visible
status: todo
type: task
priority: low
tags:
    - punctuation
    - rendering
    - engine
created_at: 2026-08-03T14:05:53Z
updated_at: 2026-08-03T14:06:04Z
---

Follow-up from csl26-l4tv. Part B of that fix made the *closing-quote* side of punctuation-in-quote markup-aware (move_punctuation_into_quote, ends_with_close_quote), but append_rendered_component and component_starts_new_sentence (render/bibliography.rs) still call entry_output.chars().last() directly -- a raw read. Under Html (or any format with trailing markup after the visible last char, e.g. a closing </span>/</em>), last_char is always the markup's closing character (>) and never whitespace, which affects:
- The "both sides non-space, insert separator" branch in append_rendered_component
- The final whitespace-driven decision in the "leads with whitespace, period-prefixed separator" branch
- component_starts_new_sentence's own last_char-based checks (though ends_with_sentence_ending_visible_punctuation and ends_with_close_quote already went through the visible-text fix)

Not fixed as part of csl26-l4tv because the two-arm parity sweep across 19 embedded styles x citations+bibliography (old-vs-new binary, plain/html/latex/typst/djot/markdown) found zero diffs attributable to this path -- it doesn't appear to be exercised by any embedded style's current entry shapes. File this as a latent correctness gap, not an active bug: fix if/when a style or fixture exposes it, using last_visible_non_space_char (already exists, used elsewhere in the same file) as the replacement.
