---
# csl26-0u0f
title: 'Chicago author-date-18th: bibliography substitute path drops quote'
status: in-progress
type: bug
priority: high
tags:
    - engine
    - chicago
    - title
    - fidelity
    - punctuation
created_at: 2026-08-24T17:52:16Z
updated_at: 2026-08-24T18:17:27Z
parent: csl26-h7oc
---

chicago-author-date-18th's bibliography renders anonymous/no-author entries by substituting the title into the author position (`substitute.candidates: [editor, translator, parent-serial, title]`). The greedy-set-cover leverage ranking still puts "B title quote boundary" first after two prior waves targeted it (90 rows, 42 sole-cause) -- this cluster is why.

## Root cause (isolated)

`resolve_title_substitute` (`crates/citum-engine/src/values/contributor/substitute.rs:612`):

```rust
let quoted = options.context == RenderContext::Citation && quote_in_citation;
```

In `RenderContext::Bibliography`, `quoted` is **unconditionally `false`**, regardless of `substitute.title_quote` mode or any `titles.<category>.quote` config. This is deliberate, tested behavior from bean `csl26-0dca` / div-011 (2026-08-14): bibliography-context substituted titles never quote, but *do* pick up the category's `emph`/`strong`/`small-caps` instead (an either/or contract, validated against APA/Elsevier-Harvard-shaped styles where the desired result is italics, not quotes).

For chicago-author-date-18th, the relevant category for `document`/`article-journal`/etc. is `component` (`text-case: title`, no `emph` configured) -- so under the either/or contract these rows get **neither** quote nor emphasis, rendering plain where the oracle wants quotes. Confirmed against 19 mechanically-verified clean rows (see below) where quote marks are the *only* diff from oracle.

Setting `titles.component.quote: true` does **not** fix this: that gate never reads category quote config in bibliography context at all (the `quoted` computation short-circuits on `context == Citation` before `quote_in_citation` is even evaluated). Confirmed this isn't a config gap -- it's an architectural one.

## Why the notes-family styles don't share this defect

`chicago-notes-18th` / `chicago-shortened-notes-bibliography-core` use `substitute: editor-translator-short` -- a preset resolving to `[editor, translator]`, with **no `title` candidate at all**. Truly anonymous entries in the notes family never reach `resolve_title_substitute`; the title just renders through its own normal `title: primary` node (with ordinary node-level `wrap: {punctuation: quotes}`), which is unaffected by the substitute-quote gate. This is why the notes family "already works" -- it sidesteps the mechanism entirely, not because it has some working version of the same code path.

This also rules out "just remove `title` from author-date-18th's candidate list" as a quick fix: `docs/specs/SUBSTITUTED_VALUE_FORMATTING.md` §1 confirms chicago-author-date.csl's real citation-context substitute chain (`author-inline -> author-title-substitute-short -> title-short -> title-primary-short`) *does* promote title into the parenthetical citation slot for these same anonymous entries. Dropping `title` from candidates would fix bibliography by accident while breaking citation-context substitution for the same rows -- worse, not better.

## Measured impact (mechanical, not estimated)

Ran a normalized quote-stripped comparison across all 41 sole-cause "B" rows in chicago-author-date-18th (post-wave-3 report, `/tmp/wave3-after-chicago-author-date-18th.json`):

- **19/41 (46%) are clean quote-only diffs** -- fixing the bibliography substitute-quote gate would flip exactly these to passing.
- **22/41 (54%) have larger structural diffs** the labeler didn't catch: missing container/publisher fields (`Modern Love.` dropped entirely), corporate-author-as-title confusion (`City of Chicago`, `Forbes`, `Endangered Languages Project` rendering as trailing text instead of as author), and content reordering (Coolidge speech, Hitchcock film credit). These are separate, unrelated defects that happen to also carry a quote-boundary symptom -- the near-miss queue's "exactly 1 label" is confirmed here to be an optimistic lower bound, not a reliable flip-count, consistent with the caveat already on file.

So the achievable win from fixing *only* the bibliography substitute-quote gate is **~19 rows in chicago-author-date-18th**, not 41-42. Likely similar order-of-magnitude in `taylor-and-francis-chicago-author-date` (inherits the same `substitute:` config per `STYLE_INHERITANCE.md` rule 4) once independently verified.

## Why this isn't a quick fix

This is a genuine engine/architecture question, not a style-config tweak:

- `docs/adjudication/DIVERGENCE_REGISTER.md` div-011 and its implementing bean `csl26-0dca` pin the current bibliography-never-quotes rule as *tested, intentional* behavior (5+ tests in `crates/citum-engine/src/values/tests.rs` ~L5118-5320), validated for APA/Elsevier-Harvard-shaped styles where bibliography wants italics.
- `docs/specs/SUBSTITUTED_VALUE_FORMATTING.md` (Draft, v1.3) already covers this exact subsystem in depth -- but its entire analysis and its proposed `TitleQuoteCondition`/`quote-when` engine capability are **citation-scoped only** ("Put `title-quote: by-category` ... in `citation.options` by default. A global title change requires independent evidence that normal citation and bibliography titles should also change" -- §5). It does not address a case where **bibliography**-context substituted titles need to quote, which is exactly what chicago-author-date-18th's oracle output demonstrates.
- Rejected approaches investigated and ruled out:
  - **Reuse `Rendering.quote` on the `contributor: author` node itself** (set `quote: true` directly on the type-variant's `contributor: author` component). Rejected: `Rendering.quote` is a shared, generic field already consumed by every component kind's own render path -- on a contributor node it means "quote this contributor's rendered value." The corpus is entirely author-less for the affected rows, so a zero-regression diff would pass while planting a landmine for the same type *with* a real author (would wrap the author's name in quotes). Field-semantics conflation the project's explicit-over-magic principle forbids.
  - **Set `titles.component.quote: true`.** Verified ineffective (see root cause above) -- the bibliography gate never reads it.
  - **Drop `title` from `substitute.candidates`.** Would fix bibliography by removing the mechanism, but breaks the citation-context substitute chain for the same rows (confirmed via `SUBSTITUTED_VALUE_FORMATTING.md` §1's citation-context trace of the real `chicago-author-date.csl`).
  - **A new per-type `Substitute` field** (e.g. `title-quote-types: [document]`). Structurally sound (the `Substitute` struct already has a precedent: `overrides: HashMap<String, SubstituteCandidates>` keyed by ref-type), but this is a schema addition requiring `just schema-gen` and, per this repo's own policy ("schema changes need a docs-only PR first"), a docs-only spec PR before implementation -- not appropriate to land inside a leverage wave.

## Recommendation

Do not implement a fix in this wave. This needs to go through `docs/specs/SUBSTITUTED_VALUE_FORMATTING.md` as a new revision (or a companion spec) that extends its scope to bibliography-context substitute quoting -- specifically:
1. Whether the div-011 bibliography-never-quotes rule should gain a bibliography-scoped override analogous to the citation-scoped `title-quote: by-category`, or a different mechanism (e.g. per-type `Substitute` override).
2. An oracle-verified per-style survey (same rigor as the existing spec's §3/§4) confirming which embedded styles actually want bibliography-context substitute quoting, since APA/Elsevier-Harvard's validated bibliography-italic behavior must not regress.
3. `chicago-author-date-18th`'s own status row already exists in that spec ("Not yet oracle-verified... Blocked on: Explicit fallback coverage + its own oracle run") -- this bean's findings are exactly that oracle-verification groundwork.

## What landed instead (this wave, csl26-vmw3-adjacent)

Confirmed via the shipped `chicago-author-date.csl` choose-block (`book classic graphic hearing map` -- the same never-quote/always-italic set wave 3 fixed for map/graphic) that `classic`/`hearing` share the identical gap wave 3 already fixed and verified for `map`/`graphic`. Extended the same proven pattern:
- `chicago-author-date-18th.yaml`: renamed the `map, graphic:` type-variant key to `map, graphic, classic, hearing:` (identical block, no other changes).
- `chicago-notes-18th.yaml` / `chicago-shortened-notes-bibliography-core.yaml`: added `classic: monograph` / `hearing: monograph` to `titles.type-mapping`, alongside the existing `map`/`graphic` entries.

This is a separate, safe, already-validated-pattern fix -- unrelated to the substitute-quote gap above, which remains open pending spec review.
