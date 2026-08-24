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
updated_at: 2026-08-26T12:54:11Z
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

## Progress: spec written, pending review

Per this bean's own "Recommended next step," wrote
`docs/specs/SUBSTITUTED_TITLE_BIBLIOGRAPHY_QUOTING.md` (Draft v1.0) — the
bibliography-context companion to `SUBSTITUTED_VALUE_FORMATTING.md`, which is
citation-scoped only. Docs-only PR; no engine/schema change yet.

Evidence re-measured at HEAD (`8cc47b8b`, `cd372f76` already applied): residual
is now 40 sole-cause / 18 quote-only-clean (down from the bean's 41/19),
17 real after excluding one quote-character-normalization false positive,
16 in scope after excluding one citation-context row. All 16 are
`document`/`manuscript`/`article-journal`, all resolving to the `component`
title category — but that category is not internally uniform for quoting
(`map`/`graphic`/`classic`/`hearing` never quote via a type-template override,
confirmed via `Rendering::merge`/`TitleRendering::merge` semantics that a
naive `titles.component.quote: true` fix would regress those four types,
undoing `cd372f76`/`8cc47b8b`). `taylor-and-francis-chicago-author-date-core`
checked independently: identical current residual, and its own narrowed
`titles.type-mapping` (missing `manuscript`) is a concrete inheritance hazard
for any type-keyed fix. `apa-7th`/`elsevier-harvard` confirmed to have zero
bibliography-context defects of this kind today — no regression risk measured
for either.

Spec recommends mechanism (ii): an explicit per-type quote declaration on the
bibliography substitute config, scoped to `chicago-author-date-18th` and its
T&F descendant, both updated in the same stage-2 commit. Two other mechanisms
considered and rejected — see the spec's Design/Rejected Alternatives sections.

Bean stays `in-progress` — implementation is a separate stacked PR after the
spec is reviewed, per repo schema-change policy.

## Progress update: broadened from quoting to all formatting axes

Bruce's PR #1231 review correctly flagged that the spec's initial v1.0
(quoting only) was too narrow. Traced the cause: the bean's own residual was
sized via `scripts/analyze-parity-residuals.js`'s `"B title quote boundary"`
label, which regex-matches quote characters in markup-stripped diffs — italic
markup never survives that stripping, so the tool used to size this bean was
structurally blind to any non-quote formatting mismatch. Direct
markup-preserving comparison (new script:
`scripts/audit-substitute-bibliography-formatting.py`) found a real
**emphasis-gap** (`map`/`hearing`/`software`/`song`/`speech` should
italicize, render plain) alongside the quote-gap, both caused by one root
issue: the substitute path resolves title-category config only, never the
resolved template node a style may already declare formatting on.

Also surfaced, and explicitly scoped **out** of this spec: a
**candidate-gap** (some `article-magazine`/`article-newspaper` rows should
promote `container-title`, not `title` — a different defect surface,
`SubstituteField::Title` resolution, not formatting), plus one CMOS-14.102
"anonymous review" macro-shape case. Both documented in the spec's taxonomy;
candidate-gap needs its own follow-up bean (not yet filed).

Spec renamed `SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md` (was
`..._QUOTING.md`), v1.1. Mechanism recommendation changed from a per-type
quote list to deriving formatting from the reference type's own resolved
bibliography template node (gated behind an explicit opt-in to preserve
div-011's backward-compatibility contract) — the render-when ambiguity that
blocked this mechanism in v1.0 resolves by reusing the engine's existing
`group_condition_matches` function.

Bean stays `in-progress`; implementation remains a separate stacked PR after
this revised spec is reviewed.
