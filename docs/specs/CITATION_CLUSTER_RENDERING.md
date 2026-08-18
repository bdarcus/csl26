# Citation Cluster Rendering Specification

**Status:** Active
**Version:** 1.3
**Date:** 2026-08-18
**Related:** `crates/citum-schema-data/src/citation.rs`,
`crates/citum-engine/src/processor/citation.rs`,
`crates/citum-engine/src/processor/rendering/grouped/core.rs`,
`docs/specs/INTEGRAL_NAME_MEMORY.md`,
`docs/specs/REPEATED_NOTE_CITATION_STATE_MODEL.md`

---

## Purpose

Define the authoritative vocabulary for the in-text citation model, document
the rendering behavior for multi-work clusters in both integral (narrative) and
non-integral (parenthetical) modes, specify the same-author collapse rule,
locator handling, same-year disambiguation, the `sentence_start` signal for
sentence-initial capitalization, and record future extension points.

---

## Terminology and Glossary

| Concept | Citum name | Serialized form | Notes |
|---|---|---|---|
| One in-text citation event | **citation cluster** | `Citation` (struct) | A single call-out in the text; may cite one or more works |
| One reference within a cluster | **cite** / **citation item** | `CitationItem` (struct) | Carries the citekey, locator, item-level prefix/suffix |
| Narrative / in-text form | **integral** | `"mode": "integral"` | Author rendered inline; e.g. "Smith (2020) argues…" |
| Parenthetical form | **non-integral** | `"mode": "non-integral"` (default) | Author inside parens; e.g. "(Smith, 2020)" |

### Key invariant: mode is cluster-scoped

`CitationMode` (`Integral` / `NonIntegral`) is a property of the **cluster**,
not of individual citation items. A mixed-mode cluster — some items narrative,
others parenthetical — is typographically incoherent and is not supported.
`Citation.mode` applies uniformly to all items in `Citation.items`. The same
invariant applies to `suppress_author`: per-item suppression is not supported.

---

## Multi-Work Clusters with Different Authors

### Integral (narrative) — default behavior

When a cluster cites multiple works by **different authors** in integral mode,
the default rendering is **list-like prose**:

> Smith (2020) and Jones (2021) both argue…

> Smith (2020), Jones (2021), and Brown (2022) all contend…

The join is locale-aware (conjunction term, optional serial comma) and is
handled by `join_integral_groups` in
`crates/citum-engine/src/processor/citation.rs`.

The **ordering** of groups within a cluster is style-defined (`citation.sort`)
and documented at the style level. This spec governs the *joining* behavior, not
sort order.

#### Non-goal: cluster-like (semicolon) narrative form

A style might conceptually want to preserve semicolons in a narrative cluster:

> Smith (2020); Jones (2021) both argue…

This form is **not currently supported** and is not on the roadmap. If a
concrete style requirement arises, it can be implemented as an opt-in under
`citation.integral.multi-cite-form: cluster` (list-like remains the default)
using the existing `multi-cite-delimiter` override as an internal escape hatch.
Until a real request exists, that escape hatch is intentionally undocumented.

### Non-integral (parenthetical) — default behavior

When a cluster cites multiple works by **different authors** in non-integral
mode, items are joined by `multi_cite_delimiter` (default `"; "`):

> (Smith, 2020; Jones, 2021)

---

## Same-Author Collapse (both modes)

### The rule

When a cluster cites multiple works by **the same author**, the author name
appears **once** with all year/date references collapsed — regardless of
citation mode:

| Mode | Input | Collapsed output |
|---|---|---|
| Integral | `\textcites{chen2017}{chen2020}` | `Chen (2017, 2020)` |
| Non-integral | `\cites{chen2017}{chen2020}` | `(Chen, 2017, 2020)` |

### Implementation

Grouping by author is performed by `group_citation_items_by_author`
(`grouped/grouping.rs`). Consecutive items sharing an author key form one
group; every group is same-author by construction.

For multi-item groups (>1 item), the renderer takes the **collapse path** in
`render_grouped_citation_group_with_format` for both modes:

- The author name is rendered once via `render_author_for_grouping_with_format`.
- For **integral** groups: the year-group wrap (e.g. `wrap: parentheses`) is
  **captured** from the first item's filtered template and **stripped** from all
  items, so each year renders bare ("2017", "2020"). After joining ("2017,
  2020"), the captured wrap is applied once to produce "(2017, 2020)". The
  final output is `"{author} {wrapped_years}"` — e.g. "Chen (2017, 2020)".
  The wrap is **not hardcoded**: a style that wraps integral years in brackets
  renders "Chen [2017, 2020]" correctly.
- For **non-integral** groups: per-item wraps are preserved (they may be the
  primary wrapping when no cluster-level wrap exists). The items are joined by
  `intra_delimiter` and the cluster-level `wrap` (if any) is applied once
  around the whole cluster, as for single-item non-integral citations.

Single-item groups always use the per-item explicit template path (no change
from pre-existing behavior).

### Exception: disambiguation hints suppress collapse

If **any** item in the cluster carries active disambiguation hints
(`min_names_to_show` or `expand_given_names`), `group_citation_items_by_author`
switches every item to per-item keying (using `item.id` as the group key
instead of the author string). This means same-author collapse does **not**
occur for the entire cluster, even for items that share an author and carry no
hints themselves. The engine's grouping contract is cluster-wide: one item
requiring expanded author display forces all items in that cluster to render
individually.

---

## Author Grouping — Interaction with Multiple Author Groups

When a cluster contains works by **different authors**, items are first grouped
by author key. Each same-author group is collapsed independently per the rule
above. After each group is rendered, the groups are joined by citation mode:

| Mode | Joining mechanism | Example |
|---|---|---|
| Integral | Locale-aware prose join | Smith (2017, 2020) and Jones (2021) |
| Non-integral | `multi_cite_delimiter` (default `"; "`) | (Smith, 2017, 2020; Jones, 2021) |

The ordering of groups follows `citation.sort`; each collapsed group is treated
as a single unit for sort purposes.

---

## Same-Year Disambiguation (a/b/c)

When a disambiguation pass assigns year-suffixes (`a`, `b`, `c`, …) to
distinguish same-author same-year works, the engine treats the **rendered year
token** (already containing the suffix) as an **atomic string**. The same-author
collapse joins these tokens as-is; no further merging is attempted:

| Items | Collapsed output |
|---|---|
| smith2020a, smith2020b | Smith (2020a, 2020b) |
| smith2019, smith2020a, smith2020b | Smith (2019, 2020a, 2020b) |

The year-suffix is part of the token and is never stripped or re-computed during
collapse.

---

## Locator Handling in Clusters

A `CitationItem` may carry a **locator** (page, section, chapter, …) specified
by the document author. Locators appear as a postnote immediately after the year
token in the template rendering.

### Single item with locator

The locator is rendered inside the year-group wrap for integral mode and before
the cluster wrap closes for non-integral mode:

| Mode | Output |
|---|---|
| Integral | `Smith (2020, p. 5)` |
| Non-integral | `(Smith, 2020, p. 5)` |

### Same-author collapse with locators

When items in a same-author group carry locators, the **year + locator** pair is
the joinable unit:

- **No locators on any item**: years joined by `,` — `Chen (2017, 2020)`. This is
  Citum's own choice per CMOS 15.30 and is a registered, intentional divergence
  from citeproc-js's Chicago output (`div-017`) — see
  [`DIVERGENCE_REGISTER.md`](../adjudication/DIVERGENCE_REGISTER.md).
- **Any item carries a locator**: the whole intra-group join escalates from the
  ordinary group delimiter to `citation.multi-cite-delimiter` (default `"; "`),
  matching CMOS 15.30's "if page numbers are also cited, use semicolons instead
  of commas" and citeproc-js's `after-collapse-delimiter`:
  - Integral: `Chen (2017, p. 10; 2020, p. 35)`
  - Non-integral: `(Chen, 2017, p. 10; 2020, p. 35)`
- **Same locator on all items**: no special engine-level merging is attempted;
  each item's fragment is joined in order (still under the escalated
  delimiter, since a locator is present). Style authors may craft templates
  that suppress duplicate postnotes, but the engine does not auto-deduplicate.

Implemented in
`crates/citum-engine/src/processor/rendering/grouped/core.rs`: the escalation
is computed once (`group_has_locator`) in
`render_fallback_grouped_citation_with_format` and applied at both live join
sites — the non-integral `repeated_item_delimiter` in
`build_grouped_citation_content`, and the integral `pre_wrapped_years`
computation in `render_fallback_grouped_citation_with_format` itself (not
`build_grouped_citation_content`'s integral branch, which is a fallback that
never runs for collapsed groups). Per-item locators are included naturally in
each item's bare "year+postnote" fragment and survive into the joined output.
See `csl26-uctc`.

### Multiple author groups with locators

Each author group (possibly collapsed with its own locators) is joined by the
mode-appropriate mechanism. Groups remain independent:

```
Non-integral: (Brown, 2015, p. 3; Smith, 2018, pp. 10–11)
Integral: Brown (2015, p. 3) and Smith (2018, pp. 10–11)
```

---

## Cluster-Level Affixes ("see also", prefixes)

Cluster-level affixes are placed by **conventional position relative to the
citation wrap**:

| Mode | Position | Prefix example | Suffix example |
|---|---|---|---|
| Non-integral | **Inside** the parentheses | `(see also Smith, 2020)` | `(Smith, 2020, e.g.)` |
| Integral | **Outside** / part of prose | `see also Smith (2020)` | `Smith (2020), e.g.` |

This is the current implemented behavior (`apply_citation_input_affixes` +
`apply_spec_wrap_and_affixes` in `citation.rs`) and is the documented contract.

Cluster-level affixes do **not** alter author collapsing or locator rendering;
they are applied after the cluster is fully composed.

For integral mode, a prefix such as "see also" appears **before** the rendered
author+years block — outside and prior to the year-group wrap. When same-author
collapse is active and items carry locators, the composition order is:

1. Each item's bare year+postnote fragment is rendered and joined inside the
   captured year-group wrap: `(2017, p. 10; 2020, p. 35)`
2. The author name is prepended: `Chen (2017, p. 10; 2020, p. 35)`
3. The cluster-level prefix is prepended: `see also Chen (2017, p. 10; 2020, p. 35)`

The corresponding non-integral form places the prefix inside the cluster
parentheses: `(see also Chen, 2017, p. 10; 2020, p. 35)`.

### Future enhancement: configurable affix placement

Some styles may want to override the convention (e.g. keep affixes inside
parentheses even for narrative clusters). This can be added as
`citation.affix-placement: inside | outside` (defaulting to the conventional
behavior) if users request it. This is **not currently implemented**.

---

## Sentence-Initial Capitalization (`sentence_start`)

### The problem

When an integral cluster begins a sentence and the cluster starts with a
lowercase affix, the leading character must be capitalized:

```
# With prefix "see also" (lowercase)
    integral, mid-sentence:    see also Smith (2020) argues …
    integral, sentence-start:  See also Smith (2020) argues …
```

The standalone processor cannot detect sentence position from rendered text. An
author-led cluster (e.g. "Smith (2020)") is already capitalized by virtue of
the author's name, so this only matters when a lowercase element leads.

### Prior art: natbib / biblatex capitalized commands

LaTeX resolves the same ambiguity by requiring the author to choose a command
variant at the call site:

| System | Lowercase | Capitalized |
|---|---|---|
| natbib | `\citet{key}` | `\Citet{key}` |
| biblatex | `\parencite{key}` | `\Parencite{key}` |
| biblatex | `\textcite{key}` | `\Textcite{key}` |
| biblatex | `\citeauthor{key}` | `\Citeauthor{key}` |

The capital variant signals to the formatter that the citation opens a sentence
and any lowercase prefix (or name particle such as "von" / "van der") should be
capitalized. Citum adopts the same explicit-signal model: the *host* supplies
the flag; the *processor* applies the transform.

### `Citation.sentence_start`

```rust
pub sentence_start: bool,  // default false
```

When `true`, the engine applies `CapitalizeFirst` to the **first alphabetic
character** of the fully-composed cluster output (after affixes and wrap).
The markup-aware variant is used, so leading punctuation (e.g. an opening
parenthesis), HTML tags, and LaTeX/Typst command prefixes are skipped rather
than corrupted:

```
"(see also Smith 2020)"   →  "(See also Smith 2020)"
"see also Smith (2020)"   →  "See also Smith (2020)"
"Smith (2020)"            →  "Smith (2020)"   (no-op: already capital)
```

The transform is applied regardless of citation mode and is a no-op when the
composed output already starts with a capital letter. It applies only to the
**first alphabetic character** of the fully-composed output; it does not change
internal capitalization within author names, titles, or locator labels.

### Who sets the flag?

The flag is **caller-supplied**, not auto-detected:

- **Document pipeline / `format_document`**: the Djot/Markdown document
  processor can inspect surrounding text to auto-populate the flag. This is a
  **deferred follow-up** (see below).
- **WASM bridge / hub editor**: the editor knows cursor position and can set
  `sentence_start` when inserting a citation after a sentence boundary.
- **FFI callers**: pass `sentence_start: true` when the citation is known to
  begin a sentence.
- **Standalone `process_citation` API**: default `false`; callers that know
  the context set it explicitly.

### Deferred: auto-detection in the document pipeline

Automatic detection of sentence boundaries in `format_document` (Djot/Markdown)
is a follow-up task. Until implemented, the flag must be set explicitly by the
host. This matches natbib/biblatex behavior: the author chooses the form.
