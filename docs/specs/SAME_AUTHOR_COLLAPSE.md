# Same-Author Collapse Specification

**Status:** Draft
**Version:** 1.0
**Date:** 2026-08-18
**Supersedes:** N/A
**Related:** `csl26-ecfn`, `csl26-m11m`, `docs/specs/CITATION_CLUSTER_RENDERING.md`,
`docs/specs/CITATION_REGIME.md`, `docs/adjudication/DIVERGENCE_REGISTER.md` (div-017),
`crates/citum-schema-style/src/style/sections/citation.rs`,
`crates/citum-engine/src/processor/rendering/grouped/grouping.rs`,
`crates/citum-migrate/src/assembly.rs`

## Purpose

Adjudicates `csl26-ecfn` ("Unconditional same-author collapse contradicts CSL's
opt-in semantics") and, as a consequence, resolves `csl26-m11m` ("Same-author
collapse produces malformed note citations"). Makes Citum's same-author
citation collapse an explicit, opt-in style setting instead of always-on engine
behavior, closing the gap between the existing `citation.collapse` field (which
already covers CSL's `collapse="citation-number"`) and CSL's other three
`collapse` values (`year`, `year-suffix`, `year-suffix-ranged`), which Citum
today applies unconditionally and migrate silently discards.

## Scope

In scope:
- Extending `citation.collapse` (`CitationCollapse`) with a same-author variant,
  and its shape.
- Gating the engine's same-author collapse path
  (`render_grouped_citation_group_with_format`,
  `group_citation_items_by_author`) on that field.
- Migrate's mapping from CSL's `collapse` attribute to the new variant.
- Regime-coherence validation between `collapse` and `Processing`.
- The resulting behavior change for note-regime styles (`csl26-m11m`).

Out of scope (tracked separately):
- Implementing `year-suffix: merged` / `year-suffix-ranged: ranged` rendering —
  parsed and round-tripped here, rendering is a follow-up bean.
- Per-item position tracking inside a citation cluster (duplicate-id repeats
  getting citeproc-js's shortened second-occurrence form) — explicitly out of
  scope per `docs/specs/REPEATED_NOTE_CITATION_STATE_MODEL.md`.
- Extending `tests/fixtures/citations-humanities-note.json` with a same-author
  cluster — a separate change with its own oracle-snapshot blast radius.
- The independent `4 (2019), 257` vs oracle `4 (2019): 257` locator-punctuation
  defect noticed while investigating `csl26-m11m` (reproduces on single-item
  citations; unrelated to collapse).
- What `collapse="citation-number"` should mean on the 4 `class="note"`
  corpus styles where it's declared but verified to produce same-author-style
  merging rather than numeric range compression (§6). Migrate drops the
  attribute for these; a follow-up would need to determine whether that's the
  right call or whether they should migrate to `same-author` instead.

## Problem

`citation.collapse: Option<CitationCollapse>`
(`crates/citum-schema-style/src/style/sections/citation.rs:99`) already exists
and is genuinely opt-in — it has exactly one variant,
`CitationCollapse::CitationNumber`, mirroring CSL's `collapse="citation-number"`.

Same-author collapse — one author name, years/notes merged across a multi-item
cluster — is a *different* mechanism, implemented unconditionally in
`render_grouped_citation_group_with_format`
(`crates/citum-engine/src/processor/rendering/grouped/core.rs:271`) for any
group with more than one item. It never consults `citation.collapse`, so
Citum implements one quarter of CSL's `collapse` vocabulary as a declarative
setting and the other three quarters as always-on engine behavior.

Migrate compounds this: `extract_citation_collapse`
(`crates/citum-migrate/src/assembly.rs:685`) reads the CSL `collapse` attribute
but discards everything except `citation-number`:

```rust
fn extract_citation_collapse(citation: &csl_legacy::model::Citation) -> Option<CitationCollapse> {
    match citation.collapse.as_deref() {
        Some("citation-number") => Some(CitationCollapse::CitationNumber),
        _ => None,
    }
}
```

A CSL style declaring `collapse="year"` and a CSL style declaring no `collapse`
attribute at all migrate to the identical Citum representation (`collapse: None`)
and render identically (collapsed) — even though CSL's own semantics say the
first should collapse and the second should not.

### Corpus measurement (`styles-legacy/`, 2 844 independent CSL styles)

| CSL `collapse` value | files | Citum today |
|---|---|---|
| `year` | 919 | collapses (right, by accident) |
| `year-suffix` | 232 | collapses as `year` — suffixes not merged |
| `year-suffix-ranged` | 14 | collapses as `year` — not ranged |
| `citation-number` | 916 | honored via the existing field |
| **absent** | **763** | **collapses anyway — wrong** |

Of the 763 with no `collapse` attribute: 44 render `variable="citation-number"`
(numeric regime — same-author collapse is never visible there because the
citation itself is just a number), 473 declare `class="note"` — Citum's own
migrate signal for `Processing::Note` (`detect_processing_mode`,
`crates/citum-migrate/src/options_extractor/processing.rs:41`; `position="ibid"`
is not a reliable note-regime proxy — `chicago-author-date.csl` and other
in-text author-date-with-ibid-shorthand styles use `position="ibid"` too) —
and the remaining 250 are neither: genuinely in-text, non-numeric, non-label
styles with no `collapse` attribute, the direct analog of `csl26-ecfn`'s
original finding. `csl26-ecfn` found the defect on one style in that last
group; `csl26-m11m` is the note-regime face of the same defect — but, per the
next section, "note-regime" does not mean "never wants collapse."

### Regime lock is strong but not total — Note styles use both

`citation-number` collapse is overwhelmingly numeric, and the year forms are
overwhelmingly author-date — but the lock is not 1:1 with `RegimeFamily`.
69 of 542 `class="note"` styles (12.7%) declare `collapse` — 65 a year form, 4
`citation-number`. `detect_processing_mode` routes *every* `class="note"`
style to `Processing::Note` unconditionally
(`options_extractor/processing.rs:41`, checked before the numeric/author-date
heuristics that follow it), regardless of what the citation's actual content
looks like, and note-class styles vary widely in content:

- `american-journal-of-archaeology.csl` (`class="note"`, `collapse="year"`)
  renders `<text macro="contributors-short"/><text macro="date"/>` — a short
  `Author Year, locator` footnote, effectively an author-date citation placed
  in a note. Confirmed via A/B on real citeproc-js (strip the attribute,
  re-render the same items): with `collapse="year"`, `Garcia 2019a; 2019b;
  Forthcoming.`; without it, `Garcia 2019a; Garcia 2019b; Forthcoming.` This
  is evidence about the CSL corpus and citeproc-js, not about Citum: no
  embedded Citum style currently has an author-date-shaped citation under
  `processing: note`, and this spec doesn't add one. What it does establish
  is that Citum's schema and engine don't structurally block the shape — a
  minimal hand-built `processing: note` style with a bare
  `contributor: author` + `date: issued` template validates and renders
  end-to-end through the document pipeline (footnote numbering included; no
  embedded style exercises this today, so it hasn't been exercised beyond
  that smoke test).
- `chicago-notes-bibliography.csl` (also `class="note"`, no `collapse`
  attribute) renders an entire bibliographic sentence per citation (title,
  container, date, DOI) with no year-group to collapse onto — this is
  `csl26-m11m`'s style. `Processing::Note` is a big tent covering both
  shapes; the bug is specific to the *second* shape, not to "Note regime" as
  such.

So `RegimeFamily` alone cannot determine whether `same-author` collapse is
*meaningful* for a given `Note`-regime style — both shapes are real CSL, and
nothing in Citum's schema or engine restricts `Processing::Note` to only the
full-sentence one. This bears directly on §5 (why an explicit value name
isn't redundant) and on §6's regime-coherence rule, which must not exclude
`Processing::Note` from `same-author`.

**`Label` licenses neither value.** `din-1505-2.csl` and
`american-mathematical-society-label.csl` are both `Processing::Label` in
name, one declaring `collapse="year"` and the other `collapse="citation-number"`
— but checking the actual citation content, not just the attribute string,
shows neither is real evidence that `Label` needs either value:

- `din-1505-2.csl` is `class="in-text"` (not note) and its `<citation>`
  renders `<text macro="author-short"/><text macro="cite-year"/>` — a plain
  author-date citation. It is not `Processing::Label` at all (its citation
  never renders `variable="citation-label"`); `has_citation_label` would not
  route it there. It's an ordinary author-date style, already counted in the
  919 `year` total above, and gives no evidence about `Label`.
- `american-mathematical-society-label.csl` genuinely is `Processing::Label`
  — its citation renders `<text variable="citation-label"/>` in brackets,
  sorted by `citation-number`. But `collapse="citation-number"` is
  **verified inert** on it: three adjacent same-cluster items render as
  `[Garc19, Garc19, Fort00]` identically with or without the attribute — no
  range compression, no suppression, nothing. citeproc-js's numeric-range
  collapsing evidently requires the template to render the number itself to
  compute a display range; a template that prints a generated label instead
  gives it nothing to collapse. There is no meaning or intended output for
  `collapse="citation-number"` on this style — verified by A/B test, not
  asserted.

Neither case supports treating `Label` as licensing either value for a real
reason — see §6.

### Ground truth for the note case (`csl26-m11m`)

citeproc-js on `chicago-notes-bibliography.csl`, given two same-author items
(ITEM-31, ITEM-32, both Garcia), repeats the full author on every citation and
joins with `"; "` — no collapse of any kind:

> `Maria Garcia, "Methods for Robust Climate Attribution," … 55–80, https://…; Maria Garcia, "Methods for Probabilistic Climate Attribution," … 81–104, https://…`

Citum drops the second author and joins with `", "`, producing a malformed
run-on sentence — the bug `csl26-m11m` reports. The delta on
`chicago-notes-18th`'s `note-disambiguate-year-suffix` fixture citation is
*exactly* the missing `; Maria Garcia,`. Two adjacent cases already prove the
non-collapsed rendering path is correct: `note-disambiguate-add-names-et-al`
(collapse already suppressed by disambiguation hints) and a different-author
cluster (`[@ITEM-31; @ITEM-33]`) both match citeproc-js byte for byte. Only the
collapse *gate* is wrong — nothing about the per-item note rendering needs to
change.

## Design

### 1. `citation.collapse` becomes a two-axis enum

CSL's `collapse` attribute flattens "what is collapsed" (citation-number vs.
same-author group) and "how far the same-author collapse goes" (plain vs.
merged-suffix vs. ranged-suffix) into one four-value string. That makes
`citation-number` a lopsided peer of three values that live on a different
axis entirely. Citum separates the two axes, following the shape already used
by `Processing` — bare-string variants alongside a config-map variant
(`{ label: { preset: alpha } }`,
`crates/citum-schema-style/src/options/processing.rs:146`):

```yaml
citation:
  collapse: citation-number     # unit variant — genuinely has no degrees

citation:
  collapse: same-author         # unit form — plain same-author collapse

citation:
  collapse:
    same-author:
      year-suffix: merged       # or: ranged
```

```rust
/// `Deserialize`/`Serialize` are hand-written (§ Implementation Notes), not
/// derived — a plain `#[derive]` on a tuple variant like `SameAuthor(..)`
/// only accepts the tagged-map form (`{ "same-author": {...} }`), not the
/// bare-string shorthand shown above. This is the same reason `Processing`
/// (`options/processing.rs:146`) hand-writes its own impls for `Label(..)`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename_all = "kebab-case"))]
pub enum CitationCollapse {
    /// Collapse adjacent citation numbers into a numeric range, e.g. `1–3`.
    CitationNumber,
    /// Collapse a same-author multi-item group onto one author name with a
    /// joined year/date list.
    SameAuthor(SameAuthorCollapse),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct SameAuthorCollapse {
    /// How same-year disambiguation suffixes render inside a collapsed group.
    #[serde(default)]
    pub year_suffix: YearSuffixCollapse,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum YearSuffixCollapse {
    /// `Smith (2020a, 2020b)` — Citum's existing behavior; each suffixed year
    /// token stays atomic (see `CITATION_CLUSTER_RENDERING.md` §Same-Year
    /// Disambiguation).
    #[default]
    Separate,
    /// `Smith (2020a, b)` — CSL `collapse="year-suffix"`. Parses and
    /// round-trips; **not yet implemented** by the renderer (falls back to
    /// `Separate` with a one-time load warning). Follow-up bean.
    Merged,
    /// `Smith (2020a–c)` — CSL `collapse="year-suffix-ranged"`. Same status as
    /// `Merged`.
    Ranged,
}
```

`same-author`, not CSL's `year`: Citum's own vocabulary for this mechanism is
the grouping key used everywhere else in the engine and docs
(`group_citation_items_by_author`, both bean titles, this spec's own
terminology). This is additive: the existing `CitationCollapse::CitationNumber`
and its serialized form (`collapse: citation-number`) are unchanged.

The generated JSON schema advertises `citation-number` as a bare string
(`CitationNumber` is a plain unit variant, so schemars' default external
tagging matches it exactly) and `same-author` as an object,
`{ "same-author": <SameAuthorCollapse schema> } `, never as a bare string —
matching `Processing`'s own published schema, where `{ "label": {...} }` is
schema-advertised but bare `"label"` is not (`docs/schemas/style.json`,
`Processing`'s `oneOf`). The bare `collapse: same-author` shorthand in the
YAML above is real, hand-parsed sugar for `{ "same-author": {} }` — exactly
how `processing: label` is real, hand-parsed sugar today — it just isn't
schema-visible, which is an existing, accepted asymmetry in this codebase,
not a new one.

### 2. Default is `None` — no collapse

CSL-faithful opt-in, resolving the `csl26-ecfn` adjudication question as
"become conditional on a style-level setting," not "register an accepted
divergence." A style that declares neither `citation.collapse` nor an
inherited one renders every citation item in a multi-item cluster in full,
joined by the ordinary intra-cluster delimiter — exactly like a hint-suppressed
cluster does today.

### 3. Same-author collapse gates on the field

`render_grouped_citation_group_with_format`
(`grouped/core.rs:271`) takes the collapse path only when the resolved
`CitationSpec`'s `collapse` is `Some(CitationCollapse::SameAuthor(_))`. This
gate is **not regime-conditional** — the engine never checks
`Processing`/`RegimeFamily` here, only the `collapse` field itself. That is
what closes `csl26-m11m` with **no note-specific code**: the three note styles
at the center of that bug (`chicago-notes-18th`,
`chicago-shortened-notes-bibliography-core`, `gb-t-7714-2025-note`) simply
don't declare `collapse`, because their source CSL doesn't either (§8) — not
because `Processing::Note` is special-cased to refuse it. A future embedded
note style whose source *does* declare `collapse="year"` (§"Regime lock…"
above shows 65 corpus examples) is fully supported by the same mechanism with
zero additional engine work.

### 4. Migration is lossless — for values that are legal on the source style's regime

All four CSL values now map with nothing discarded and no warning on the
common case:

| CSL `collapse` | Citum `collapse` |
|---|---|
| `citation-number` | `citation-number` |
| `year` | `same-author` (`year-suffix: separate`) |
| `year-suffix` | `{ same-author: { year-suffix: merged } }` |
| `year-suffix-ranged` | `{ same-author: { year-suffix: ranged } }` |
| *(absent)* | *(absent)* — no collapse |

This table assumes the CSL value is legal under §6's regime rule for the
source style's detected `Processing`. It is, for all but 6 of the 2 081
CSL-derived styles that declare `collapse` — the 2 `Label`-regime styles and
4 `Note`-regime styles §6 carves out. For those 6, migrate drops the
attribute (mapping to *absent*, same as a style with no `collapse` at all)
rather than emitting a value that would fail validation.

This is otherwise the entire point of the two-axis shape: today's `_ => None` arm
silently drops 1 165 styles' declared intent (232 `year-suffix` + 14
`year-suffix-ranged` + 919 `year`, of which only the `year` forms happen to
render correctly by coincidence). `merged` / `ranged` parse and round-trip
faithfully into the new enum; because the renderer doesn't implement them yet,
migrate additionally emits a one-time warning for those two cases (not for
`year`, which is fully implemented) so the gap is visible rather than silent.
The style data is preserved correctly regardless, and lights up correctly the
moment the renderer lands — a strict improvement over flattening it at migrate
time.

### 5. Naming: why `citation-number` isn't renamed, and why the value can't be inferred from regime

`citation-number` and `same-author` can look like symmetric peers on one "what
collapses" axis, which invites two simplifications: renaming `citation-number`
(`CitationCollapse::CitationNumber` stutters "Citation," and within a
numeric-regime style there is nothing else a collapse setting *could* mean —
so the name reads as redundant), or dropping the named value entirely in favor
of a boolean whose meaning is inferred from `Processing`. Neither holds up:

- **Not a rename.** `citation-number` is not merely this field's discriminant
  label — it is CSL's own variable name (`variable="citation-number"`), reused
  verbatim across sorting, warnings, bibliography grouping, migrate, the
  already-published JSON schema (`docs/schemas/style.json`), docs, and dozens
  of CSL-suite fixtures, independent of and older than this field. Renaming
  the enum variant would sever that continuity and break the two embedded
  styles that already declare `collapse: citation-number`
  (`american-medical-association`, `gb-t-7714-2025-numeric`) for a purely
  lexical objection, not a structural one.
- **Not regime-inferred.** The redundancy intuition assumes `RegimeFamily`
  determines the value, so naming it is repeating information the schema
  already has. `AuthorDate` and `Numeric` do have a fixed citation shape —
  every author-date citation renders an author, every numeric citation
  renders just a number — which is exactly why `citation-number` reads as
  redundant there. `Note` does not: `american-journal-of-archaeology.csl`'s
  citation is author-year-shaped and `same-author` collapse is verified
  operative on it, while `chicago-notes-bibliography.csl`'s citation is a
  full bibliographic sentence with no year-group to collapse onto at all —
  same `Processing::Note`, two incompatible shapes. A boolean whose meaning
  is "inferred from `Processing`" has no single coherent meaning within
  `Note`, because `Note` doesn't fix the shape the way `AuthorDate` and
  `Numeric` do — the value must be named, not just present.

The apparent redundancy is real for the dominant case — `AuthorDate`/`Numeric`
account for the overwhelming majority of the 2 081 CSL-derived styles that
declare `collapse` — but not universal, and schema fields routinely stay
explicit even where one value dominates a context (an XML attribute doesn't
become implicit just because it's usually the same). Keeping both variants
named and explicit is the option that is correct in every observed case, not
just the common one.

### 6. Regime coherence: what's actually licensed, and by what evidence

Each permission below is backed by a verified, operative example, not by
attribute presence alone. `Label` is denied both values (§"Regime lock…"):

- `collapse: same-author` is legal on `AuthorDate`-family and `Note` (verified
  operative: `american-journal-of-archaeology.csl`), and on `Custom` per
  `docs/specs/CITATION_REGIME.md`'s existing rule that a base-less `Custom`
  "never triggers automatic regime resets." It is **invalid on `Numeric`**
  (no author is ever rendered) and **on `Label`** — no genuine Label-regime
  example needs it, and the parallel `Label` claim for `citation-number`
  (below) is verified inert on the one real corpus style that declares it, so
  there is no basis for assuming the reverse case is any different.
- `collapse: citation-number` is legal **only** on `Numeric` (plus `Custom`).
  This matches the engine's own existing gate exactly —
  `should_collapse_citation_numbers`
  (`crates/citum-engine/src/processor/rendering/mod.rs:490`) already requires
  `Processing::Numeric` before it does anything, regardless of what
  `collapse` is set to. It is invalid on `AuthorDate`-family (no
  `citation-number` variable is ever rendered), on `Label` (verified inert on
  `american-mathematical-society-label.csl` — the citation renders
  `variable="citation-label"` in brackets, sorted by `citation-number` for
  ordering only; stripping the attribute and re-rendering the same
  three-item cluster produces byte-identical output, `[Garc19, Garc19,
  Fort00]` either way), and **on `Note`**, with one caveat below.
- The `Note` + `citation-number` caveat: 4 real `class="note"` corpus styles
  (e.g. `proinflow.csl`) declare it, and stripping the attribute *does*
  change their output — but what it changes is the same author-suppression
  behavior `same-author` collapse produces (`GARCIA, Maria. …; GARCIA, Maria.
  …` becomes `GARCIA, Maria. …; …` with the second author dropped), not
  numeric range compression. Whether these 4 styles should migrate to
  `same-author` instead of being rejected, or whether citeproc-js's collapse
  mechanics for full-sentence note styles work differently than either
  variant currently models, is not resolved here — it is out of scope (see
  Scope) and migrate should treat them the same as the other 473 no-`collapse`
  note styles (drop the attribute) until a follow-up investigates.

This means the deny-list is **not** "everything not yet disproven by a corpus
scan" — `Label` is excluded from both values on the strength of one verified
inert case per value, and `citation-number` is scoped tightly to the regime
the engine already requires, not loosened just because the schema now allows
more expressive intent elsewhere. Consistent with the existing
regime-coherence enforcement in `docs/specs/CITATION_REGIME.md` (the
inheritance invariant) and `StyleLineage::apply_regime_guard` in migrate.

### 7. Embedded styles requiring `collapse: same-author`

Source CSL attribute noted in parentheses; declaring this preserves rendering
exactly as it is today:

| style | source CSL `collapse` | Citum `collapse` after this spec |
|---|---|---|
| `apa-7th` | `year` | `same-author` |
| `chicago-author-date-18th` | `year` | `same-author` |
| `elsevier-harvard-core` | `year` | `same-author` |
| `springer-basic-author-date-core` | `year-suffix` | `{ same-author: { year-suffix: merged } }` |
| `gb-t-7714-2025-author-date` | `year` | `same-author` |

`taylor-and-francis-chicago-author-date-core` inherits `collapse` via `extends:
chicago-author-date-18th`; no separate declaration needed.
`springer-basic-author-date-core` is the one style that gains real fidelity
from the two-axis shape rather than just parity: its source declares
`year-suffix`, which the flat-enum alternative below would have flattened to
plain `year` behavior exactly as migrate does today.

### 8. Embedded styles that must NOT declare it

Their *specific* source CSL has no `collapse` attribute — not "every style in
their regime," per §"Regime lock…" above — so under this spec they stop
collapsing same-author clusters:

- `taylor-and-francis-council-of-science-editors-author-date-core` — `csl26-ecfn`'s
  original finding; oracle already confirms `(Garcia 2019a; Garcia 2019b)`,
  not `(Garcia 2019a, 2019b)`.
- `modern-language-association` — no `collapse` attribute in
  `modern-language-association.csl`.
- `chicago-notes-18th`, `chicago-shortened-notes-bibliography-core`,
  `gb-t-7714-2025-note` — all three sources
  (`chicago-notes-bibliography.csl`, `chicago-shortened-notes-bibliography.csl`,
  `tests/fixtures/csl-m/gb-t-7714-2025-note.csl`) verified to have no
  `collapse` attribute, closing `csl26-m11m`. This is a property of these
  three specific styles' full-sentence-per-citation note content, not of
  `Processing::Note` as a family — see §"Regime lock…" for the 65 real
  `class="note"` corpus styles that legitimately declare `collapse:
  same-author` because their note content is author-year-shaped, not
  full-sentence.

### 9. `div-017` is unaffected

`chicago-author-date.csl` carries `collapse="year"`, so
`chicago-author-date-18th` still declares `collapse: same-author` and still
collapses. The registered comma-vs-semicolon divergence
(`docs/adjudication/DIVERGENCE_REGISTER.md`, div-017) continues to apply
unchanged to `disambiguate-year-suffix` and `subsequent-author-consecutive` in
`tests/fixtures/citations-expanded.json`. Implementation should not expect any
movement there.

### 10. Interaction with existing suppressors

- **Disambiguation hints** continue to suppress collapse cluster-wide,
  unchanged (`CITATION_CLUSTER_RENDERING.md` §Exception: disambiguation hints
  suppress collapse). This check runs first; `collapse: same-author` only
  matters when it doesn't fire.
- **Locator escalation** (`csl26-uctc`: any item with a locator escalates the
  intra-group join to `multi-cite-delimiter`) is part of the same-author
  collapse path and is therefore only reachable when `collapse: same-author` is
  set. `CITATION_CLUSTER_RENDERING.md`'s "Same-author collapse with locators"
  section is scoped accordingly in the companion update.

### 11. Duplicate-id clusters are out of scope, but must not regress

`[@A, p. 10; @A, p. 20]` — two citation items with the *same* id in one
cluster — currently key identically in `group_citation_items_by_author`
(`grouped/grouping.rs`, keyed on `item.id`), so two consecutive equal keys
merge into one group and take the collapse path regardless of the new gate.
When `collapse` is unset, the implementation must key on `(index, id)` instead
of bare `id` so duplicate ids cannot silently re-merge — otherwise the fix in
(3) has a hole for exactly the shape it's meant to close. citeproc-js's true
behavior here additionally *shortens* the repeat
(`Garcia, "Methods…," 20.` rather than a second full clause) via per-item
position tracking inside a cluster, which
`docs/specs/REPEATED_NOTE_CITATION_STATE_MODEL.md` explicitly lists as
out of scope. This spec requires the *structural* fix (no merged/malformed
output) and a pinned regression test, not oracle-exact shortening.

### 12. MLA note

`csl26-uctc` deferred "MLA drops the locator delimiter entirely in collapsed
groups" as out of scope. `modern-language-association.csl` has no `collapse`
attribute, so under this spec MLA stops collapsing same-author clusters
entirely, which likely moots that defect (nothing left to drop a delimiter
from). Implementation should confirm, not assume.

## Rejected Alternatives

**(a) Regime-keyed suppression.** Gate same-author collapse on
`Processing::regime_family() == RegimeFamily::Note` inside
`group_citation_items_by_author`, with no schema change. Cheap and closes
`csl26-m11m` alone. Rejected: it hardcodes behavior the schema already has a
field for, leaves the 763-style author-date/numeric divergence measured above
untouched, keeps migrate silently discarding `collapse="year"`, and would have
to be unwound the moment `csl26-ecfn` is properly adjudicated. It would also
have been **empirically wrong**, not just suboptimal: §"Regime lock…" shows 65
real `class="note"` corpus styles legitimately declare `collapse: same-author`
(`american-journal-of-archaeology.csl` and 64 others) — a blanket
`RegimeFamily::Note ⇒ never collapse` rule would silently misrender all of
them the moment any one of them entered the embedded/tracked style set,
exactly the kind of regime-as-proxy mistake this spec's design avoids by
gating on the field itself (§3) rather than on `Processing`.

**(b) Boolean `collapse: true | false` with regime-derived meaning.** No
impossible value/regime combinations by construction (the engine infers
"collapse years" vs. "collapse citation numbers" from `Processing`). Rejected
for two independent reasons: `year-suffix` and `year-suffix-ranged` are real,
distinct behaviors 232 + 14 corpus styles declare — a boolean cannot express
them, repeating exactly the information loss this spec exists to fix — and,
more fundamentally, `Processing::Note` doesn't fix a citation shape the way
`AuthorDate` and `Numeric` do:
`american-journal-of-archaeology.csl` (`class="note"`) has an author-year
citation body where `same-author` collapse is verified operative, while
`chicago-notes-bibliography.csl` (also `class="note"`) has a full-sentence
body with nothing to collapse. A boolean whose meaning is "inferred from
`Processing`" has no single coherent meaning within `Note` — the value must
be named, not just present.

**(c) Adopt CSL's flat four-value enum directly**
(`citation-number | year | year-suffix | year-suffix-ranged`). Closest to CSL,
least new design. Rejected: makes `citation-number` a lopsided peer of three
values that live on an orthogonal axis (what collapses vs. how far a
same-author collapse goes), and doesn't let a future same-author-collapse
enhancement outside CSL's vocabulary attach cleanly without reopening the enum
shape again.

**(d) Keep always-on, register a `DIVERGENCE_REGISTER.md` entry.** The other
half of `csl26-ecfn`'s original adjudication question. Rejected: the oracle
comparison in `csl26-ecfn` and the note-regime ground truth in `csl26-m11m`
both show citeproc-js's *non-collapsed* output is what a style without
`collapse` actually produces — calling that a "divergence" would mean
registering Citum's own bug as an accepted design choice, not documenting a
genuine authority-basis disagreement (contrast with div-017, which is a real
CMOS-vs-citeproc-js disagreement that survives this spec unchanged).

## Acceptance Criteria

- [ ] `collapse: same-author` (bare-string) and `{ same-author: { year-suffix:
      … } }` (config-map) both parse; the config-map form serializes back
      identically (the bare-string form is deserialize-only sugar and is not
      expected to round-trip byte-for-byte, matching `processing: label`'s
      existing behavior). The generated JSON schema's `CitationCollapse`
      `oneOf` advertises `"citation-number"` as a bare-string const and
      `same-author` only as `{ "same-author": <SameAuthorCollapse> }`, mirroring
      `Processing`'s published schema (`docs/schemas/style.json`) exactly —
      `"label"` is schema-advertised only as an object, never as a bare
      string, even though the Rust deserializer accepts both.
- [ ] Absent `citation.collapse` (directly or via inheritance) produces no
      same-author collapse, in every regime and both citation modes.
- [ ] Migrate maps all four CSL `collapse` values with no information
      discarded on `Numeric`, `AuthorDate`-family, and `Note` styles whose
      content matches one of the two collapse mechanisms; `year-suffix` /
      `year-suffix-ranged` additionally emit a one-time load warning naming
      the unimplemented-rendering gap. The 6 corpus styles where `collapse` is
      declared but doesn't fit either mechanism (2 inert `Label` styles, 4
      `Note` + `citation-number` styles per §6's caveat) are a known,
      out-of-scope gap — migrate drops the attribute for them, matching
      today's behavior for a no-`collapse` style, not a validation error.
- [ ] `collapse: same-author` is accepted on `AuthorDate`-family, `Note`, and
      `Custom`; rejected on `Numeric` and `Label`. `collapse: citation-number`
      is accepted on `Numeric` and `Custom`; rejected on `AuthorDate`-family,
      `Note`, and `Label`. Every rejection carries an actionable error (§6).
- [ ] The five embedded styles in §7 show **zero** exactParity movement on
      `node scripts/report-core.js --style <name>`.
- [ ] `chicago-notes-18th`'s `note-disambiguate-year-suffix` fixture citation
      becomes exact.
- [ ] `div-017` continues to apply to `disambiguate-year-suffix` and
      `subsequent-author-consecutive` on `chicago-author-date-18th`.
- [ ] Duplicate-id clusters (no `collapse` set) render each item in full with
      no merged/malformed output — regression test pinned, not compared
      against citeproc-js's shortened form.
- [ ] A corpus sweep is run and net exactParity movement is reported per style
      touched, including
      `taylor-and-francis-council-of-science-editors-author-date`.

## Implementation Notes

Sketch only — implementation is a separate, stacked PR per this repo's
schema-changes-need-a-docs-PR-first rule.

1. **Schema**: add `SameAuthor(SameAuthorCollapse)` to `CitationCollapse`
   (`crates/citum-schema-style/src/style/sections/citation.rs:25`),
   `#[non_exhaustive]`. Hand-write `Serialize`/`Deserialize` on
   `CitationCollapse` itself, mirroring `Processing`'s existing impls
   (`options/processing.rs:469` `impl Serialize for Processing`, `:495`
   `impl<'de> Deserialize<'de> for Processing`) field-for-field: unit variants
   serialize as bare strings; the `SameAuthor` payload always serializes as a
   `{ "same-author": <config> }` map (never collapsed back to a bare string,
   even when the config is the default — `Processing::Label` does the same);
   `visit_str` additionally accepts bare `"same-author"` as sugar for
   `SameAuthor(SameAuthorCollapse::default())`, matching `Processing`'s
   `"label" => Processing::Label(LabelConfig::default())` arm exactly.
   `SameAuthorCollapse` and `YearSuffixCollapse` derive normally
   (`#[derive(Deserialize, Serialize)]` with `#[serde(rename_all =
   "kebab-case")]`) — only the outer `CitationCollapse` enum needs hand-written
   impls. Keep `#[cfg_attr(feature = "schema", derive(JsonSchema))]` plus
   `#[cfg_attr(feature = "schema", schemars(rename_all = "kebab-case"))]` on
   `CitationCollapse` so schemars' externally-tagged default derives a schema
   that matches the hand-written impls by construction (same reason
   `Processing`'s doc comment says "`rename_all` is retained for `JsonSchema`
   derive"). Overlay/merge already threads the `collapse` field through
   (`sections/citation.rs:300`, `:384`; `style/overlay.rs:306`) — no merge
   logic changes needed. Add the regime-coherence validation. Regenerate
   schemas and the data-model reference docs in the same commit:
   `just schema-gen`.
2. **Engine**: gate the collapse branch in
   `render_grouped_citation_group_with_format` (`grouped/core.rs:271`) on
   `collapse == Some(SameAuthor(_))`; when unset, key
   `group_citation_items_by_author` (`grouped/grouping.rs`) on `(index, id)`
   per §11. Warn once per style load on `year_suffix != Separate`.
3. **Migrate**: extend `extract_citation_collapse`
   (`crates/citum-migrate/src/assembly.rs:685`) per the table in §4.
4. **Embedded styles**: declare `collapse` on the five styles in §7.
5. **Tests**: `crates/citum-engine/tests/domain_fixtures.rs`, native
   `InputReference` construction, BDD `given/when/then` names, `#[rstest]`
   where parameterized, `assert_eq!` on full captured strings — covering the
   note-regime fix (oracle-exact no-locator case; locator-on-second-item with
   the pre-existing punctuation defect called out, not silently asserted as
   parity), the duplicate-id regression, the shortened-notes short form,
   an author-date regression guard proving `collapse: same-author` still
   collapses (keeping `csl26-uctc`'s four existing tests meaningful), and
   schema round-trip / validation-rejection tests for both new value/regime
   mismatches.
6. **Verification**: `just pre-commit`; per-style
   `node scripts/report-core.js --style …` for each style in §7 and §8 plus
   `chicago-shortened-notes-bibliography`; `git status tests/snapshots/`
   clean (those snapshots hold citeproc-js output, unaffected by an engine
   change); one sequential, low-concurrency corpus sweep
   (`systemd-run … MemoryMax=6G`) diffed against a worktree baseline for the
   net movement number.

## Changelog

- v1.0 (2026-08-18): Initial draft. Adjudicates `csl26-ecfn` (opt-in, not a
  registered divergence) and resolves `csl26-m11m` as a consequence.
