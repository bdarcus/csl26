---
# csl26-q67h
title: Restore gb-t-7714-2025-author-date's own bibliography.sort
status: todo
type: bug
priority: normal
created_at: 2026-08-06T13:30:40Z
updated_at: 2026-08-06T13:30:58Z
parent: csl26-ccdt
---

gb-t-7714-2025-author-date.yaml has no explicit bibliography.sort of its own; it silently inherits sort: citation-number from its numeric base (gb-t-7714-2025-base.yaml), which resolves to an empty group-sort template and renders the bibliography in registry order rather than the upstream CSL's author+date-intext order (see csl26-a19q for the missed-warning gap this causes).

## Attempted in csl26-m8la, reverted — not a targeted patch

Two changes were tried together in the csl26-m8la session and both reverted
before shipping, because together they caused a severe regression elsewhere
in the corpus:

1. Adding `bibliography.sort: {template: [{key: author}, {key: issued}]}` to
   the YAML. `key: issued` is only a scoped approximation of the upstream
   CSL's conditional `date-intext` macro (issued -> available-date ->
   accessed -> "no date" term, branching further by reference type) — see
   "GroupSortKey has no macro/conditional date key" below, since citum's
   `GroupSortKey` enum can't express that macro's conditional/coalescing
   logic.
2. To make (1) not regress gb-t-7714-2025-author-date's own oracle numbers,
   `ReferenceSorter::extract_author_sort_key_opt` (`sorting.rs`) needed a
   companion fix distinguishing a substitute chain that genuinely resolves to
   title (`EffectivePrimary::Title`) from one that's fully exhausted with no
   promotable value at all (`None` — a true tie, not a title-derived key).
   This is a real bug, independent of (1), but it's a *shared* code path
   across all 157 styles.

Measured together (defect1 fix + gate + this YAML change + the sorting.rs
change): gb-t-7714-2025-author-date landed at 170/203 passed — *worse* than
shipping the engine fix alone (173/203) — and the sorting.rs change caused
`american-medical-association-alphabetical` to collapse from 21/67 to 1/67
exact-parity passed in the full 35-style exemplar corpus check. Net: adding
both changes was strictly negative given the current state of the codebase,
so csl26-m8la shipped only its engine fix and left this style's own sort
missing.

## GroupSortKey has no macro/conditional date key

Upstream CSL group-sort keys can be macro-based (`<key macro="..."/>`),
whose comparison is the macro's *rendered text*, and the macro itself can be
type-conditional. citum's `GroupSortKey` enum (Author, Title, Issued,
RefType, Field) can only express variable-based sort keys, so `key: issued`
cannot faithfully transcribe a conditional macro like `date-intext`.
Restoring this style's sort faithfully likely needs either a
coalescing/fallback-chain sort key type, or a macro-aware sort key that
renders text and compares strings (closer to citeproc-js's actual semantics
— see also csl26-huuz, the related disambiguation collision-grouping gap).
Design work, not a targeted patch.

## Scope for a real fix

Any future attempt needs to: (a) design a faithful sort-key representation
for the conditional date macro (or explicitly accept the `key: issued`
approximation and gate it on the fixture families that don't exercise
available-date/accessed), and (b) land the `sorting.rs` Author-key fix
*separately*, verified against the full 157-style corpus on its own before
being combined with this style's sort — not bundled together as one change
the way this session tried.
