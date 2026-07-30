# Embedded-Tier Oracle Parity — Class A (Bibliography Numeric Label) Spike

- **Date:** 2026-07-30
- **Bean:** `csl26-lf68` (under epic `csl26-arly`)
- **Question:** the largest single defect class in the embedded+exemplar
  oracle text-parity taxonomy (318 mismatches, 12.7% of 2,501) is styles
  where the citeproc-js oracle's bibliography entry has a leading list number
  (`1.`, `[2]`, `11`) that Citum's rendered text lacks. Is this a comparator
  join-space bug (`normalizeExactText` stripping tags without inserting a
  boundary space) or a real rendering gap? Time-boxed spike, not a fix.
- **Instrument:** direct `citum render refs -b tests/fixtures/references-expanded.json
  -s <style>.yaml --json --mode bib` against `american-medical-association`,
  `ieee`, and `royal-society-of-chemistry`, cross-checked against the
  `oracleDetail[]` entries in a full `report-core.js` run.

## Finding 1: there is no join-space bug (the "A1" hypothesis is wrong)

The initial hypothesis, based on eyeballing one AMA entry
(`1.Kuhn TS. …` vs `Kuhn TS. …`), was that citeproc-js's
`second-field-align` transport — sibling `<div class="csl-left-margin">` /
`<div class="csl-right-inline">` divs with no whitespace between them in the
raw HTML — was being tag-stripped into a flush join that should have had a
space inserted at the boundary.

This does not hold. Computed over every `exactMatch: false` entry in the
full oracle-parity corpus (2,501 mismatches): **zero** are resolved by
collapsing all whitespace on both sides before comparing (i.e., there is no
case where the only difference is a missing/extra space at this boundary).
The control case proves the opposite of the hypothesis: IEEE already
concatenates flush with no space (`[2]S. Hawking, …` on both the oracle and
Citum side) and **is currently `exactMatch: true`** for that pairing — the
visual gap between the two `<div>`s is CSS-rendered, not a text character,
and citeproc-js's actual text output has none either. Inserting a join space
would have *regressed* every currently-passing flush-numbered entry (IEEE,
and any other style whose template renders its own literal number). No
change to `scripts/oracle-utils.js` was made; a regression test
(`compareText exact parity: second-field-align left-margin and
right-inline concatenate flush, no inserted space`, `scripts/oracle.test.js`)
guards against reintroducing this.

## Finding 2: the real defect is a genuine rendering gap (confirmed "A2")

Direct CLI rendering confirms the number is absent from Citum's own output,
not lost in transport:

```
$ citum render refs -b tests/fixtures/references-expanded.json \
    -s styles/embedded/american-medical-association.yaml --json --mode bib
{"id": "ITEM-1", "text": "Kuhn TS. The Structure of Scientific Revolutions. …"}

$ citum render refs -b tests/fixtures/references-expanded.json \
    -s styles/embedded/ieee.yaml --json --mode bib
{"id": "ITEM-1", "text": "[1]T. S. Kuhn, “The Structure of Scientific Revolutions,” …"}
```

The `entries[].text` field has no separate label/number field alongside it
— whatever isn't in `text` isn't rendered anywhere. Comparing the two
styles' bibliography YAML explains the split:

- `styles/embedded/ieee.yaml` — every `type-variants` entry opens with a
  literal template component, `- number: citation-number` (wrapped in
  `punctuation: brackets`), as the first element of the component group. The
  number is bibliography template *content*.
- `styles/embedded/american-medical-association.yaml` — no bibliography
  `type-variant` includes a `number: citation-number` component anywhere;
  entries open directly with the author.

citeproc-js's `second-field-align` is a *processor-level* feature: for
numeric CSL styles that declare it, the processor generates the list number
itself and places it in `csl-left-margin`, independent of whether the CSL
`<bibliography>` layout also renders a number inline. Citum has no
equivalent processor-generated numbering — a style only gets a bibliography
number if its template explicitly renders one via a `number:
citation-number` component. AMA's original CSL relies on the processor
behavior and therefore has no such component to migrate; Citum's rendering
is faithful to AMA's template as authored, but the template alone can't
reproduce citeproc-js's implicit list numbering. This is a genuine
style/engine gap, not a comparator or harness defect.

## Finding 3: Royal Society of Chemistry's apparent within-run inconsistency does not reproduce

The original class taxonomy flagged RSC as inconsistent *within a single
run* — some entries in `oracleDetail` show a Citum-side number (`11W. Chen,
PhD thesis…`), others don't (`T. S. Kuhn, International…`), all tagged with
the same `evidenceRunId: "baseline"`. Direct CLI rendering of RSC's
bibliography (`styles/royal-society-of-chemistry.yaml`, both `--mode bib`
alone and `--mode both` with a citations fixture) produces **zero** entries
with any numeric label — consistently, for every reference. The numbered
entries seen in `oracleDetail` do not match what `citum render refs`
actually outputs for this style against `references-expanded.json`, which
means the report's evidence for those specific indices is drawn from a
different fixture or evidence source than the one implied by the shared
`evidenceRunId` label. This is a distinct report-harness bug (evidence-index
mislabeling/merging), separate from the class-A2 rendering gap above, and
was **not** root-caused further within this spike's time-box.

## Disposition

- No class-A fix lands in this PR (a different commit in the same PR fixes
  an unrelated defect, class J — see below). `csl26-lf68` is being closed
  as a completed spike; findings above are the deliverable for class A.
- Follow-up bean filed for class A2 (bibliography numeric-label rendering
  gap for `second-field-align`-derived styles: AMA, ACS, nature, and others)
  under `csl26-arly`.
- Follow-up bean filed for the RSC evidence-mislabeling report bug, also
  under `csl26-arly`.
- `scripts/oracle-utils.js` is unchanged; the added regression test in
  `scripts/oracle.test.js` documents why a join-space fix is wrong so the
  next person investigating class A doesn't re-walk this same dead end.

Separately, this PR *does* fix class J (47 mismatches, a leaked numeric
bibliography label in `elsevier-vancouver-author-date`) — see
`fix(styles): elsevier-vancouver label-mode leak` and `csl26-w1vf`. That fix
is unrelated to the class-A/A2 finding above (different style, different
root cause: an inherited `label-mode: numeric` option, not a missing
template component) and does not change any of the conclusions in this
document.
