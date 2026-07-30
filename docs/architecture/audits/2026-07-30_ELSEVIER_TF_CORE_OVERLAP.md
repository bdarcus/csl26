# Elsevier / Taylor & Francis `-core` Sibling Overlap — Measurement

- **Date:** 2026-07-30
- **Bean:** `csl26-edjj` (under epic `csl26-s2rw`, follow-up from
  [2026-07-28_STYLE_INHERITANCE_PORTFOLIO_AUDIT.md](2026-07-28_STYLE_INHERITANCE_PORTFOLIO_AUDIT.md))
- **Question:** Three Elsevier and three Taylor & Francis embedded `-core`
  styles are currently standalone (no shared `extends` parent between
  siblings). Is that an unjustified omission, or is the overlap between
  them too thin to justify a synthesized shared base?
- **Instrument:** `scripts/measure-style-overlap.js` (new). Order-independent
  by design: scalar `options.*` key-paths compared by identical-value
  fraction, template/type-variant components compared as key-sorted-JSON
  sets (Jaccard), and every shared option path checked against the preset
  enum names in `crates/citum-schema-style/src/presets.rs` so generic
  boilerplate doesn't get counted as a novel-structure signal. Deliberately
  not `measure-delta-derivability.js` — that scores what citum-migrate can
  synthesize from *legacy CSL* with `--minimize-wrapper`; it says nothing
  about whether these hand-tuned embedded artifacts share extractable
  structure. Artifact: `scripts/report-data/style-overlap-2026-07-30.tsv`.

## Elsevier: `elsevier-{harvard,vancouver,with-titles}-core`

| Pair | Option overlap | Component overlap | Preset-expressible |
|---|---|---|---|
| harvard vs vancouver | 30.4% (7/23) | 8.3% (5/60) | 1 |
| harvard vs with-titles | 14.3% (4/28) | 17.8% (13/73) | 1 |
| vancouver vs with-titles | 21.7% (5/23) | 10.0% (7/70) | 1 |
| **3-way common** | **3 option paths** | — | 1 |

The 3-way common option paths are `options.multilingual` (preset-expressible),
`options.punctuation-in-quote`, and `bibliography.options.entry-suffix` — two
single scalar flags, not a coherent shared configuration block. Component
overlap tops out at 17.8% between two of the three pairs and is lower for the
third. These three styles carry ~1,840 known CSL dependents between them
(`KNOWN_DEPENDENTS` in `scripts/report-core.js`), making them the
highest-reach embedded styles in the portfolio — the bar for restructuring
them should be high, and this overlap does not clear it.

**Finding: no shared parent justified for the Elsevier family.** The three
styles are correctly independent; each already reuses
`options.multilingual` via preset and there is no further coherent block to
hoist.

## Taylor & Francis: `taylor-and-francis-{chicago-author-date,council-of-science-editors-author-date,national-library-of-medicine}-core`

| Pair | Option overlap | Component overlap | Preset-expressible |
|---|---|---|---|
| chicago vs cse | 5.7% (2/35) | 22.5% (9/40) | 1 |
| chicago vs nlm | 8.7% (2/23) | 16.3% (7/43) | 1 |
| **cse vs nlm** | **30.6% (11/36)** | **46.4% (13/28)** | 1 |
| 3-way common | 1 option path | — | 1 |

`taylor-and-francis-chicago-author-date-core` is correctly an outlier here —
it already `extends: chicago-author-date-18th`, a different family entirely,
so its low overlap with the other two is expected and not evidence against
them.

CSE vs NLM is the one pair with real overlap (30.6% options, 46.4%
components). Inspecting the ten non-preset-expressible shared option paths:

```
options.punctuation-in-quote
bibliography.options.contributors.display-as-sort
bibliography.options.contributors.name-form
bibliography.options.contributors.initialize-with
bibliography.options.contributors.shorten.and-others
bibliography.options.contributors.shorten.delimiter-precedes-last
bibliography.options.contributors.delimiter
bibliography.options.contributors.delimiter-precedes-last
bibliography.options.contributors.sort-separator
bibliography.options.separator
```

Nine of ten are `bibliography.options.contributors.*` sub-fields. Both
styles hand-write the same Vancouver-style contributor configuration
(family-first initials, no period-space, comma sort-separator) as explicit
nested fields rather than the `options.contributors: vancouver` preset
shorthand. This is exactly the generic-boilerplate case the preset-
attribution step exists to catch: **the overlap is real, but it is
preset-shaped, not template-shaped.**

**Finding: no synthesized shared parent for CSE/NLM either.** The
recommended fix is smaller than a refactor — both styles should collapse
their explicit `bibliography.options.contributors.*` block to
`bibliography.options.contributors: vancouver` (or the closest matching
preset, to be confirmed field-by-field) and rely on `extends`-independent
preset reuse. That is a preset-conversion task, distinct from and much lower
risk than introducing a new `-core` parent, and is the only actionable
follow-up from this measurement.

## Gate for any future refactor

Per the plan this audit was scoped under, a shared parent (or preset
conversion) is only landed if, on a clean-worktree baseline:

- `node scripts/report-core.js --styles <family members>` shows fidelity
  flat or better for every member;
- SQI rises — specifically Concision (25%) and Preset Usage (15%), which is
  exactly what factoring should move;
- `just check-core-quality` holds against
  `scripts/report-data/core-quality-baseline.json`.

Neither Elsevier nor T&F crossed the overlap threshold that would justify
attempting a new shared `-core` parent, so no refactor is proposed from this
audit. The CSE/NLM preset-conversion follow-up is a separate, smaller,
lower-risk task and is filed as such rather than folded into this measurement.

## Reproduction

```bash
node scripts/measure-style-overlap.js --family elsevier --family taylor-and-francis \
  --out scripts/report-data/style-overlap-2026-07-30.tsv
```
