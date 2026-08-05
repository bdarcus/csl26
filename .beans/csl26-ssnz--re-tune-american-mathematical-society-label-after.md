---
# csl26-ssnz
title: Re-tune american-mathematical-society-label after all-selector removal
status: todo
type: task
priority: normal
tags:
    - style
    - fidelity
created_at: 2026-08-05T18:36:57Z
updated_at: 2026-08-05T18:36:57Z
---

ams-label extends elsevier-with-titles and had a local `all` type-variant that shadowed every inherited variant. Removing the `all` selector (csl26-q4g5) makes the parent's 16 variants live in the dependent, which renders richer entries (container editors, edition, 'PhD thesis', page prefix) that diverge from citeproc-js for this style.

Measured, full embedded sweep --all-features: exact parity 27/67 -> 24/67, fidelity bib 45 -> 44, citations unmoved at 17/20. Tier net was +5 (elsevier-with-titles +8).

Confirmed the drop is not from deleting the `all` block: restoring the block leaves it at 24/67, because the selector no longer matches either way. The inherited variants are the cause.

Re-tune with local type-variants that suit AMS rather than re-encoding the shadowing. Changed rows: ITEM-4 (chapter), ITEM-6 (book), ITEM-11 (thesis) and 9 others.
