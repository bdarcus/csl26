---
# csl26-0u0f
title: 'Chicago author-date-18th: bibliography substitute path drops quote'
status: todo
type: bug
priority: high
tags:
    - engine
    - chicago
    - title
    - fidelity
    - punctuation
created_at: 2026-08-24T17:52:16Z
updated_at: 2026-08-24T18:43:39Z
parent: csl26-h7oc
---

## Problem

Chicago author-date styles render anonymous entries (no author, editor, or
translator) by substituting the title into the author slot
(`substitute.candidates: [editor, translator, parent-serial, title]` in
`chicago-author-date-18th.yaml`). In **bibliography** context this never
quotes the title, even for reference types where CMOS 18 and the real
citeproc-js oracle require quotes.

Example (`chicago-author-date-18th`, fixture `ITEM-15`, an
`article-journal` entry with no author):

- Oracle: `"The Role of Theory in Research." 2018. Journal of Theoretical Psychology 28 (3): 201-15.`
- Citum: `The Role of Theory in Research. 2018. Journal of Theoretical Psychology 28 (3): 201-15.`

Only the missing quote marks differ. This is the dominant remaining cause
in the "B title quote boundary" defect class, which the leverage-ranking
tool (`scripts/analyze-parity-residuals.js`) still ranks #1 for
`chicago-author-date-18th` after three prior waves targeted quote-boundary
issues.

## Root cause

`resolve_title_substitute`, `crates/citum-engine/src/values/contributor/substitute.rs:612`:

```rust
let quoted = options.context == RenderContext::Citation && quote_in_citation;
```

In `RenderContext::Bibliography`, `quoted` is unconditionally `false` --
the right-hand side is never evaluated. No `substitute.title_quote` mode
and no `titles.<category>.quote` config can make a substituted title quote
in bibliography context; the gate short-circuits before either is read.

This is deliberate, tested behavior from bean `csl26-0dca` / divergence
`div-011` (`docs/adjudication/DIVERGENCE_REGISTER.md`, 2026-08-14):
bibliography-context substituted titles never quote, but pick up the
title's category `emph`/`strong`/`small-caps` instead (an either/or
contract). It was validated against APA/Elsevier-Harvard-shaped styles,
where the desired bibliography result is italics, not quotes. For
`chicago-author-date-18th`, the relevant category (`component`) has no
`emph` configured either, so affected rows get neither quote nor emphasis.

`chicago-notes-18th` / `chicago-shortened-notes-bibliography-core` don't
exhibit this: their `substitute:` preset (`editor-translator-short`)
resolves to `[editor, translator]`, with no `title` candidate, so
anonymous entries never reach `resolve_title_substitute` at all -- the
title renders through its own normal, correctly-quoted `title: primary`
node instead. This is a structural difference in candidate lists, not a
working version of the same code path.

## Why the obvious fixes don't work

- **`titles.component.quote: true`.** No effect -- the bibliography gate
  never reads category quote config (see root cause above).
- **`quote: true` on the `contributor: author` node itself.** `Rendering.quote`
  is a shared, generic field already consumed by every component kind's own
  render path; on a contributor node it means "quote this contributor's
  rendered value." Every affected row today is author-less, so a
  zero-regression corpus diff would pass while planting a landmine for the
  same type *with* a real author (its name would get wrapped in quotes).
- **Drop `title` from `substitute.candidates`.** `docs/specs/SUBSTITUTED_VALUE_FORMATTING.md`
  §1 traces the real `chicago-author-date.csl` citation-context substitute
  chain (`author-inline -> author-title-substitute-short -> title-short ->
  title-primary-short`) and confirms it *does* promote title into the
  parenthetical citation slot for these same rows. Removing the candidate
  would fix bibliography by accident while breaking citation-context
  rendering for the identical rows.

## Why this needs a spec, not a style tweak

`docs/specs/SUBSTITUTED_VALUE_FORMATTING.md` (Draft, v1.3) already covers
this subsystem in depth, including a status row for `chicago-author-date-18th`
("Not yet oracle-verified... blocked on: explicit fallback coverage + its
own oracle run"). But its entire analysis and its proposed
`TitleQuoteCondition`/`quote-when` engine capability are **citation-scoped
only** (§5: "Put `title-quote: by-category`... in `citation.options` by
default. A global title change requires independent evidence that normal
citation and bibliography titles should also change."). It does not
address bibliography-context substitute quoting at all -- which is exactly
what this bean's oracle evidence requires.

Per this repo's schema-change policy, any fix here (whether a bibliography-
scoped counterpart to `title-quote: by-category`, or a new per-type
`Substitute` override -- the struct already has a precedent for the latter:
`overrides: HashMap<String, SubstituteCandidates>`, keyed by ref-type) needs
a docs-only spec PR before implementation.

## Measured scope

Only a fraction of the "B title quote boundary" class is this specific
defect -- much of the rest is unrelated content (missing container fields,
corporate-author-as-title confusion, reordering) that happens to also carry
a quote-boundary symptom. Don't trust the raw label count; re-measure with
the commands below before scoping a fix.

## Reproduction

```bash
cargo build --release --bin citum
node scripts/report-core.js --all-features --citum-bin target/release/citum \
    --style chicago-author-date-18th > /tmp/report.json
node scripts/analyze-parity-residuals.js /tmp/report.json \
    --list "B title quote boundary" --json > /tmp/b-rows.json
node -e '
const d = JSON.parse(require("fs").readFileSync("/tmp/b-rows.json"));
const sole = d.entries.filter(e => e.labels.length === 1);
const strip = s => s.replace(/[""''"'"'“”‘’"]/g, "");
const clean = sole.filter(e => strip(e.oracle) === strip(e.citum));
console.log(`sole-cause: ${sole.length}, clean quote-only: ${clean.length}`);
'
```

As of this writing: 41 sole-cause rows, 19 clean quote-only diffs (the rest
have other, unrelated defects the labeler did not separately catch).

## Recommended next step

Write a spec revision (new section or new companion doc, referencing
`SUBSTITUTED_VALUE_FORMATTING.md`) that:

1. Decides the bibliography-scoped mechanism: an analogue to
   `title-quote: by-category` gated on `RenderContext::Bibliography`, or a
   per-type `Substitute` override (e.g. `title-quote-types: [document]`),
   or something else -- with rejected alternatives recorded.
2. Runs the oracle-verified per-style survey `SUBSTITUTED_VALUE_FORMATTING.md`
   §3/§4 already uses for the citation-context candidates, scoped to
   bibliography context, for `chicago-author-date-18th` and its
   `taylor-and-francis-chicago-author-date` descendant (inherits the same
   `substitute:` config per `docs/specs/STYLE_INHERITANCE.md` rule 4 --
   needs independent verification, not assumed identical).
3. Confirms APA / Elsevier-Harvard's already-validated bibliography-italic
   behavior does not regress under whatever mechanism is chosen.

Get the spec reviewed before implementing -- this is a schema/engine
change, not a style-config fix.
