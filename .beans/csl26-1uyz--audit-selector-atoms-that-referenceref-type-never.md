---
# csl26-1uyz
title: Audit selector atoms that Reference::ref_type never returns
status: todo
type: task
priority: normal
tags:
    - style
    - schema
    - fidelity
created_at: 2026-08-05T19:33:48Z
updated_at: 2026-08-05T19:33:48Z
---

Eight selector atoms are authored by embedded styles but are not values `Reference::ref_type()` can return: article, pamphlet, periodical, graphic, event, song, bill-proceeding, bill-record. They are CSL genre values or pre-conversion input types (see conversion/scholarly.rs:890 for the genre set, conversion/legal.rs:213 for the bill pair), so the type-variants keyed on them are probably unreachable — the same defect class as the dead `legislation` selector fixed in csl26-q4g5.

They were added to KNOWN_REFERENCE_TYPE_NAMES so the published schema does not reject styles Citum ships, with a comment pointing here. That is a holding position, not a verdict.

Affected: apa-7th (event, song), chicago-author-date-18th (bill-proceeding, bill-record), gb-t-7714-2025-base / -author-date / -note (article, pamphlet, periodical, graphic).

Prove reachability per atom, delete the unreachable variants, and measure fidelity per style before and after — the elsevier precedent showed unshadowing can move parity in either direction. Then drop the names from the vocabulary and tighten the drift test to assert the vocabulary equals the ref_type output set rather than merely containing classified_ref_types.
