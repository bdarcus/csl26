# Disambiguation Specification

**Status:** Active
**Date:** 2026-05-29
**Related:** [`docs/reference/DISAMBIGUATION.md`](../reference/DISAMBIGUATION.md),
[`docs/specs/MULTILINGUAL.md`](./MULTILINGUAL.md),
[`docs/specs/MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md`](./MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md),
[CSL styles#7667](https://github.com/citation-style-language/styles/issues/7667),
[CSL schema#452](https://github.com/citation-style-language/schema/issues/452)

## Purpose

Defines the normative model for disambiguation in Citum: when it activates, which
strategies are applied in which order, how keys are constructed, and how multilingual
and grouped bibliographies interact with it. The how-to reference for style authors
is [`docs/reference/DISAMBIGUATION.md`](../reference/DISAMBIGUATION.md); this
document is the design authority.

## Scope

**In scope:**
- Collision-key construction (what variables constitute a "same cite")
- Strategy cascade order and early-exit semantics
- Year-suffix assignment, including the issued-year-only keying rule
- Multilingual-aware key generation
- Group-aware suffix assignment and the `disambiguate: locally` option

**Out of scope:**
- Rendering specifics (see [`docs/reference/DISAMBIGUATION.md`](../reference/DISAMBIGUATION.md))

## Design

### 1. Collision key

A collision group is a set of references that share the same **author key** and
**year key**. These keys are computed in
[`processor/disambiguation.rs`](../../crates/citum-engine/src/processor/disambiguation.rs)
and are the only inputs to the disambiguation decision — the *rendered output* is
never consulted.

**Author key** (`build_author_key`): lowercased family names of contributing
names, joined by commas, with the et-al suffix included when the name list is
abbreviated by the style's `shorten` config. This matches what the style will
visually show, so disambiguation keys track rendered output without re-rendering.

**Year key** (`build_group_key`): the `issued` year (`effective_issued_date()`),
appended to the author key. **No other date field participates in the key.**

#### Year-suffix when the original-publication date differs

Our working assumption is that in author–date styles, year-suffix disambiguation
(a, b, c…) is based only on the author name(s) and the publication year of the
edition cited, not on any original-date or dual-date information. This matches the
general "same author, same year" rules and reprint guidance in major author–date
systems such as APA and Chicago, which treat the year of the edition consulted as
the operative year for citations and disambiguation. We are not aware of any major
style that clearly requires a different rule; if such a case emerges, we can revisit
this assumption and introduce a style-specific override.

For three reprints by one author — originally 1926, 1926, 1927, all published 1967
— Citum produces `(1926/1967a) (1926/1967b) (1927/1967c)`. Because only the `issued`
year enters `build_group_key`, all three reprints form one collision group and all
receive a suffix. The original-publication date is rendered as part of the output
but plays no role in the collision test.

citeproc-js produces `(1926/1967a) (1926/1967b) (1927/1967)` because it gates
suffix assignment on the full rendered date string; this diverges from our working
assumption and has no evidence of user dependence. See `div-009` in the
[Divergence Register](../adjudication/DIVERGENCE_REGISTER.md).

**Verification:** `apa_reprint_year_suffix_attaches_to_issued_year_only` in
`crates/citum-engine/tests/citations.rs` locks this behavior against regression.

#### Suppressing the rendered suffix on a redundant date occurrence

A style that legitimately renders `issued` more than once per item (a short
front year plus a full-precision date later in the body — GB/T author-date,
`csl26-6eak`) must not inline the same year-suffix into both occurrences.
`TemplateDate.suppress_disamb_suffix: Option<bool>`
(`crates/citum-schema-style/src/template.rs`, kebab
`suppress-disamb-suffix`) opts a component out of `inline_disamb_suffix`
regardless of `hints.disamb_condition`. This is purely a rendering-level
mirror of `csl26-gl0n`'s `TemplateDate.suppress_note` (see
[`CALENDAR_DATE_ANNOTATIONS.md`](./CALENDAR_DATE_ANNOTATIONS.md)) — it does
not affect collision-key construction above, only which occurrence's
formatted text the suffix is spliced into.

#### Anonymous-fallback author key

`build_author_slot_key` (`processor/disambiguation.rs`) previously gave
every reference with no contributor and no matched `substitute` field a
*unique* singleton key — correct when the substitute promotes a per-reference
*title* (already distinguishing), wrong when a component-level
`TemplateContributor.fallback` renders a *constant* term (e.g. GB/T's `佚名`
anonymous-author placeholder). Such references now share a stable sentinel
key (`Disambiguator::ANONYMOUS_FALLBACK_KEY`) so they collide by year like a
real shared author, instead of each silently forming its own group.

**Known gap:** the `Disambiguator` that computes *bibliography* hints is
currently constructed with the **citation**-scoped `Substitute` config
(`processor/setup.rs::calculate_hints`), not the bibliography-scoped one —
a pre-existing scoping inconsistency, invisible until a style's bibliography
and citation `substitute.template` chains actually differ (as
`gb-t-7714-2025-author-date`'s now do). The sentinel key above is inert in
practice until that scoping bug is fixed; tracked in `csl26-6eak`'s residual
findings, not resolved here — the fix touches a shared code path used by
every style's disambiguation and needs cross-corpus verification before it
lands.

### 2. Strategy cascade

Strategies are attempted in increasing order of disruptiveness and stop at the
first that resolves every collision in the group:

1. **Et-al expansion** (`names: true`) — reveal additional names beyond the
   et-al threshold.
2. **Given-name expansion** (`add_givenname: true`) — add initials or full given
   names when family-name collisions remain. Scoping controlled by
   `givenname-disambiguation-rule` (see §2.1).
3. **Year suffix** (`year_suffix: true`) — append a letter sequence (a–z, aa–az,
   …) to the issued year.

Each strategy is tried against the current collision group; if it splits the
group into singletons, no further strategies run. If et-al expansion produces
sub-groups that are still ambiguous, given-name expansion and/or year suffix are
applied to those sub-groups independently.

Implemented in `apply_group_hints` →
`try_apply_name_partitions` / `try_apply_givenname_resolution` /
`apply_year_suffix`.

### 2.1 `givenname-disambiguation-rule`

Specifies which author positions receive given-name expansion. The field lives on
`Disambiguation` in `citum-schema-style/src/options/processing.rs` as
`givenname_rule: GivennameRule`. Default: `by-cite`.

| CSL value | Engine scope | Notes |
|---|---|---|
| `by-cite` *(default)* | citation-local expansion overlay | see §2.1.1 |
| `all-names` | global expansion | all affected citations use the expanded form consistently |
| `all-names-with-initials` | expand all positions | initials vs full controlled by contributor config `initialize-with` |
| `primary-name` | expand **first author only** | required by Chicago author-date |
| `primary-name-with-initials` | expand **first author only** | required by APA 7; initials via contributor config |

**Key invariant:** initials vs full given name is always driven by the contributor
config's `initialize-with` / `name-form` settings, not by this rule. The rule
controls only *which positions* are eligible for expansion.

#### 2.1.1 `by-cite` citation-local expansion (csl26-lvib)

`by-cite` is applied at citation render time, not by mutating the processor's
global disambiguation hints. The renderer clones the global hint map for the
current citation, clears global given-name expansion for the references in that
citation, then overlays the name-expansion fields computed from only the current
citation's references. Year suffixes, group order, citation numbers, note
position, and bibliography rendering continue to use the global hints.

`all-names` and `all-names-with-initials` deliberately keep the global hint map:
if a name is expanded for disambiguation anywhere in the document, all rendered
uses of that affected reference receive the expanded form. This makes `by-cite`
and `all-names` observably different on fixtures where a reference belongs to a
global collision group but appears alone, or outside the citation that needs
expansion.

The current hint model still expresses given-name expansion as all eligible
rendered positions or primary-name-only; it does not store an arbitrary mask of
individual name positions. The `by-cite` implementation therefore scopes the
decision to the current citation and preserves the existing position model.

### 3. Year-suffix assignment ordering

Within a collision group, suffixes (`a`, `b`, `c`…) **follow the effective
bibliography sort order** — never citation order. This matches the CSL spec and
APA/Chicago guidance: `a`/`b`/`c` correspond to the order in which entries appear
in the sorted reference list, so suffixes are *derived* from bibliography order.
When the bibliography context changes, suffixes are recomputed; they are not
user-stable keys.

Author and year are equal across a same-author/same-year group, so what breaks the
tie depends on whether the style has a resolved `group_sort` at all:

- **No resolved bibliography sort** (a style with no `bibliography.sort` and no
  `processing` preset supplying one): the tiebreaker is the title, sorted with the
  *same* normalization the bibliography uses — leading-article stripping plus
  locale collation via `sort_support::title_sort_key`. A raw lowercased title is
  **not** used — it sorts "An Ecology…" before "Biology…", yielding `2019b` before
  `2019a` (fixed for csl26-2zy6, guide-conformance audit row 138). The raw title
  survives only as a deterministic final tiebreaker.
- **A resolved bibliography sort exists but doesn't fully order the group**
  (either its template is empty — e.g. the `citation-number` preset, which is
  registry order by definition — or its keys tie for every member): entries take
  **registry order**, the same tiebreak the renderer's stable sort falls back to
  (`ReferenceSorter::sort_references_impl`'s empty-template early return, or a
  stable sort over registry-ordered input when keys tie). An empty template stays
  registry-order only — the finer tiebreaks below are a renderer no-op for it, per
  `sort_references_impl`'s early return, so `Disambiguator` must not apply them
  either. For a non-empty template, `Disambiguator` calls the same
  `ReferenceSorter::sort_by_keys` the bibliography renderer itself uses — so
  every key in the resolved template, including a full-date-aware `Issued`
  comparison (year, month, and day; see below), applies identically in both
  places. Only entries that remain fully tied after every resolved key fall
  through to a final tiebreak: present ids compare as text and a missing id
  sorts after every present one, when the resolved sort carries the opt-in id
  tiebreak (`ReferenceSorter::sort_references_with_id_tiebreak`); then
  registry order. There is no separate date-comparison step here — comparing
  dates twice, once in the shared `Issued` key and again as a
  Disambiguator-only pre-sort, would just risk the two drifting apart. Fixed
  for csl26-m8la — the previous behavior pre-sorted every group
  title-alphabetically before applying the resolved sort, which could diverge
  from the renderer's own tiebreak whenever the resolved sort's keys didn't
  fully determine order (observed in `gb-t-7714-2025-author-date`'s large
  anonymous-author collision bucket, where render position and assigned
  letter had no correlation at all).

  A same-year, no-title collision pair in `chicago-author-date-18th`
  (two Gourmet magazine entries, May and September 2000) exposed a second,
  independent defect during this fix: `ReferenceSorter`'s `Issued` sort key
  used to compare only the year, so `sort_by_keys` couldn't distinguish them
  and fell through to a stable-sort tie — silently wrong for any style
  sorting same-year entries by date, not specific to this bean's grouping
  logic. Fixed by widening the `Issued` sort key to compare the full
  `(year, month, day)` (`ReferenceSorter::issued_date_parts`, `sorting.rs`),
  with a missing month or day defaulting to `0` (sorts before any real
  value, matching EDTF's own less-precise-could-be-anywhere-in-range
  convention).

When a `BibliographyGroup` defines a `sort`, that sort takes precedence within the
group (see §5 below).

The letter sequence is base-26 with wrapping: 1→a, 26→z, 27→aa, 52→az, 53→ba,
… Implemented in `int_to_letter` in `values/date.rs`.

### 3.1 Per-guide application (engine default vs. style flags)

The disambiguation cascade is style-driven: the engine ships a **CSL-faithful
default** (`names: false`, `add_givenname: false`, matching citeproc-js, which
defaults both off), and each style carries the flags its guide requires — exactly
as the upstream CSL sources do. There is no contradiction between the conservative
engine default and a guide that disambiguates aggressively; the guide's behavior is
expressed in the style, not the default.

The decisive rule from the major guides (APA §8.20, MLA 9 §6.9, Chicago 17/18
author–date): **different authors who share a surname are distinguished by given
names/initials, never by a year suffix.** Year suffixes apply only to *same author,
same year*. Mapping to engine flags:

| Style | `names` | `add_givenname` | `givenname_rule` | `year_suffix` | Expressed as |
|---|---|---|---|---|---|
| APA 7 | true | true | `primary-name` | true | `processing: author-date-full` preset |
| Chicago 17/18 AD | true | true | `primary-name` | true | `processing: author-date-full` preset |
| MLA 9 | true | true | `by-cite` | **false** | custom block (author-*page*: no suffix; falls through to the `disambiguate-only` short title, §6) |
| IEEE / AMA | — | — | — | — | numeric; no author-date disambiguation |

**`by-cite` is citation-local in Citum** (§2.1.1): it compares only references that
appear together in one citation. Same-surname authors usually appear in *separate*
citations, so a guide that wants initials in every cite (APA §8.20) needs **global**
detection — the `primary-name` rule, not `by-cite`. Initials vs. full given name is
then driven by each style's contributor config (`initialize-with` / `name-form`), per
the §2.1 invariant — so one rule serves APA (initials) and Chicago (full given name).

The **`author-date-full` preset** encodes exactly this guide profile —
names + add-givenname + **`primary-name`** + year-suffix — and APA and Chicago use it
directly (`processing: author-date-full`). Because it is a real preset (not
`Processing::Custom`) it also supplies `default_bibliography_sort` (`author-date-title`),
so suffix order (§3) is correct without an explicit `bibliography.sort`. The other
`author-date*` presets keep the CSL-default `by-cite` rule; the migration extractor
only folds a CSL style to `author-date-full` when the source explicitly declares
`primary-name`, preserving `by-cite` fidelity otherwise.

A style that instead uses a custom `processing` block (e.g. MLA, to disable
`year_suffix`) becomes `Processing::Custom`, which supplies **no**
`default_bibliography_sort` — such a style must declare `bibliography.sort` explicitly
or its reference list (and year-suffix order) falls back to insertion order.

### 4. Multilingual-aware keys

When the style's multilingual config specifies a display mode other than
`primary`, the author key must reflect the same surface form the style will
render. `render_name_for_disambiguation` selects the appropriate name variant
(transliteration, translation, or original) before lowercasing and joining.

This ensures that if a style shows transliterated names, two references whose
transliterations collide are treated as a disambiguation collision, not two
distinct authors.

Monolingual references (no `MultilingualComplex`) always fall through to the
original via the `Display` trait chain.

### 5. Group-aware disambiguation (`disambiguate: locally`)

When a `BibliographyGroup` sets `disambiguate: locally`, the disambiguator is
instantiated per group rather than globally. Consequences:

- **Year-suffix sequences restart** at `a` within each group. A "Cases" group
  and a "Books" group can each begin `(2020a) (2020b)` independently.
- **Group sort** (if set) drives suffix ordering instead of the global title
  sort.
- No suffix escapes a group's boundary; within-group collision detection is
  scoped to that group's reference set.

Without `disambiguate: locally`, disambiguation runs globally across the full
bibliography.

### 6. Short-title suppression via `first-reference-note-number`

A note style may add a short title *only* to resolve a collision, but that same
title should not appear in `first-reference-note-number` cross-ref citations,
where the note number is already sufficient identification.

**Example** (two works by same author, same year):

| Context | Output |
|---|---|
| First cite of *Rome* | Smith, *Rome*, 2020, 45. |
| First cite of *Greece* | Smith, *Greece*, 2020, 67. |
| Later short cite of *Rome* | Smith, see n. 1. |

**Implementation:** `ProcHints.suppress_disambiguation_title` is set when a
subsequent-position citation has a `first_reference_note_number` (populated by
`normalize_note_context` from the `first_note_by_id` map on `Processor`).
The renderer in `values/title.rs` checks this flag and suppresses any template
title component with `disambiguate_only: true`. The first-reference note number
itself is available as `number: first-reference-note-number` in templates.
Suppression is gated on the template actually rendering the note-number identifier
(`template_uses_first_ref_note_number`) to prevent silent reintroduction of
ambiguity.

This suppression is automatic and not currently style-configurable; if a future
style needs different behavior, a `suppress_disambiguation_title` option can be
added to the citation context.

## Implementation Notes

- The collision-key layer is intentionally separated from the rendering layer so
  that disambiguation decisions are reproducible without re-rendering all
  references.
- `DisambiguationFlags` (derived from `Disambiguation` struct) is the only
  per-style knob passed into the disambiguator; it must not grow to include
  rendering concerns.
- All native disambiguation tests in `crates/citum-engine/tests/citations.rs`
  (the `citations` nextest target) pass.

## Acceptance Criteria

- [x] Year-suffix collision key uses only `issued` year (no original-date gate)
- [x] All native disambiguation tests passing
- [x] Group-aware suffix restart implemented (`disambiguate: locally`)
- [x] Multilingual key generation respects display mode
- [x] Native fixture asserting `(1926/1967a) (1926/1967b) (1927/1967c)` for the APA §8.15 reprint scenario
- [x] Short-title suppression via `first-reference-note-number` implemented and tested
- [x] `givenname-disambiguation-rule` field exists on `Disambiguation`; `primary-name` and
  `primary-name-with-initials` restrict expansion to the first author only (csl26-4ada)
- [x] `primary-name` falls back to year-suffix when primary-author expansion cannot resolve
  the collision (identical primary authors); et-al expansion retained alongside suffix (csl26-wu1l)
- [x] `by-cite` given-name expansion is citation-local and distinguishable from
  `all-names` global expansion (csl26-lvib)
- [x] Upstream CSL disambiguation fixtures that distinguish `by-cite` and
  `all-names` are tracked in the disambiguation fixture generator
- [x] Year-suffix order follows the article-stripped/locale-collated bibliography
  sort, not a raw lowercased title (csl26-2zy6, audit row 138)
- [x] APA-7th carries `add-givenname` + `primary-name-with-initials` (global) so
  same-surname authors get initials, not a spurious year suffix (csl26-2zy6, row 114)
- [x] MLA disables `year_suffix` and disambiguates same-author works via the
  `disambiguate-only` short title (csl26-2zy6, row 173)

## Related specs

- [CITATION_REGIME](CITATION_REGIME.md) — disambiguation is regime-scoped.
  Author-date and label disambiguation settings must not leak into numeric
  styles through style inheritance; the regime guard in `merge_style_overlay`
  prevents this for `citation.non_integral` (which carries disambiguation-derived
  author-date citations).

## Changelog

- 2026-08-06: Fixed §3's resolved-`group_sort` case (csl26-m8la).
  `Disambiguator::sort_group_for_year_suffix` pre-sorted every collision group
  title-alphabetically before applying the resolved sort, regardless of whether
  a `group_sort` was configured; this could diverge from the renderer's own
  tiebreak (registry order, or id order when the sort carries the opt-in id
  tiebreak) whenever the resolved sort's keys didn't fully order the group. Now
  calls the same `ReferenceSorter::sort_by_keys` the renderer itself uses for a
  non-empty template — see above — falling back to id (when set) and then
  registry order only for entries still tied after every resolved key; an
  empty template stays registry order only.

  A second, independent engine defect surfaced while fixing this: the
  `Issued` sort key used by that shared `sort_by_keys` path only ever compared
  the year, silently mis-ordering any style's same-year entries with
  different months (exposed by `chicago-author-date-18th`'s May/September
  Gourmet magazine pair). Widened `CachedSortValue::Issued` and its
  comparators to the full `(year, month, day)` (`sorting.rs`), and added
  `DateValue::month()` alongside the existing `.year()`/`.day()`
  (`citum-schema-data`) to support it.

  `gb-t-7714-2025-author-date`'s adjusted bibliography oracle failures went from
  42 to 30 (out of 203), with zero regressions across citum-engine's test suite.
  This is not full oracle parity — the style's bibliography still renders in
  registry order rather than a real author+date order, since its own
  `bibliography.sort` is still missing (`csl26-q67h`, deliberately not
  restored in the same change — see below) — it makes citum's own suffix
  letters consistent with citum's own render, which is the failure mode this
  bean reported. A residual ~9-entry gap in the English anonymous-undated
  bucket traces to a separate, architectural mismatch between citum's
  variable-based collision grouping and citeproc-js's render-text-based
  grouping — tracked in `csl26-huuz`, not fixed here.

  Two further defects were found and investigated but deliberately **not**
  shipped in this change, because measuring them together showed no benefit
  and a severe regression elsewhere:
  - Restoring `gb-t-7714-2025-author-date.yaml`'s own `bibliography.sort`
    (lost during migration — the leaf silently inherits `citation-number`,
    registry order, from its numeric base).
  - A companion fix to `ReferenceSorter::extract_author_sort_key_opt`
    (`sorting.rs`), needed to keep that restoration net-neutral for this
    style: it unconditionally fell back to a title-derived sort key whenever
    the substitute chain resolved to nothing promotable, even when title was
    never in the active chain.

  Measured together, gb-t-7714-2025-author-date's own oracle numbers were no
  better than the engine fix alone, and the `sorting.rs` change — a shared
  path across all 157 styles — caused `american-medical-association-alphabetical`
  to collapse from 21/67 to 1/67 exact-parity in the full corpus check. Both
  are tracked together in `csl26-q67h` with the measurements, for a future
  attempt that lands them independently and verifies each against the full
  corpus on its own.
- 2026-07-23: Added `TemplateDate.suppress_disamb_suffix` (rendering-level
  opt-out for a redundant `issued` occurrence) and a stable sentinel author
  key for references falling through to a component-level
  `TemplateContributor.fallback` term, surfaced by `csl26-6eak`'s GB/T
  author-date work. Documented the known citation-vs-bibliography
  `Substitute`-config scoping gap that currently makes the sentinel key
  inert in the real style; not fixed in this change.
- 2026-06-21: Promoted the major author-date guide disambiguation profile to the
  `author-date-full` preset (csl26-2zy6 follow-on). `Processing::AuthorDateFull` now
  carries the global `primary-name` rule (names + add-givenname + primary-name +
  year-suffix); the other `author-date*` presets keep CSL-default `by-cite`. `apa-7th`
  and `chicago-author-date-18th` now use `processing: author-date-full` (replacing
  their custom blocks; APA drops the explicit `bibliography.sort` since the preset
  supplies it). This fixed Chicago's latent same-surname collision (it had been
  `by-cite`). The migration extractor (`fold_to_named_processing`) folds a CSL style to
  `author-date-full` only when it explicitly declares `primary-name`, preserving
  `by-cite` fidelity otherwise. No corpus regression (154 styles, fidelity=1.0).
- 2026-06-21: Guide-conformance disambiguation pass (csl26-2zy6). Added §3.1
  (per-guide application) and rewrote §3 to state that year-suffix order follows the
  effective bibliography sort. Engine: `build_reference_cache` now keys the
  year-suffix sort on `sort_support::title_sort_key` (article-stripped, locale
  collated) instead of a raw `to_lowercase()` — fixes `2019b`-before-`2019a` (audit
  row 138). Styles: `apa-7th.yaml` switched to a custom `processing` block with
  `add-givenname` + `givenname-rule: primary-name-with-initials` + explicit
  `bibliography.sort: author-date-title` (row 114); `modern-language-association.yaml`
  set `year-suffix: false` with `names`/`add-givenname` on, relying on its existing
  `disambiguate-only` short title (row 173). Added native regressions
  `year_suffix_follows_article_stripped_title_order`,
  `givenname_expansion_preferred_over_year_suffix`,
  `primary_name_initials_expand_globally_across_citations`, and
  `year_suffix_off_emits_no_letter` in the `citations` target. No engine default
  change (stays CSL-faithful); corpus fidelity held at 1.0/154.
- 2026-06-02: Fixed `primary-name` cascade fallback (csl26-wu1l). When the primary
  author's given name cannot resolve a collision (identical primary authors), the engine
  now falls back to year-suffix while retaining the et-al expansion that was found.
  Fixed `try_apply_combined_resolution` and the `try_apply_name_partitions` subgroup path
  in `processor/disambiguation.rs` to validate expansion under primary-only rendering
  before committing; added a new `primary_only` flag to `check_givenname_resolution` /
  `append_givenname_resolution_key`. Added unit test
  `test_primary_name_identical_primary_falls_back_to_year_suffix` and integration tests
  for both the fallback and success paths.
- 2026-06-02: Implemented `by-cite` citation-local given-name expansion
  (csl26-lvib). Citation rendering now overlays current-citation name-expansion
  hints for `GivennameRule::ByCite`, while `all-names` keeps global expansion.
  Added native regressions distinguishing `by-cite` from `all-names` and tracked
  the relevant CSL disambiguation fixtures in `tests/fixtures/update_disambiguation_tests.py`.
- 2026-06-02: Added §2.1 `givenname-disambiguation-rule` (csl26-4ada). Documents
  `GivennameRule` enum (5 CSL values), engine scoping behavior, and acceptance
  criterion for primary-name scoping.
- 2026-05-31: Implemented `render_name_for_disambiguation` (csl26-54jn). Flattens
  contributors via `resolve_multilingual_name` so the collision key matches the style's
  active display mode (transliterated/translated/primary). Covered by
  `test_multilingual_key_generation_respects_display_mode` in
  `crates/citum-engine/src/processor/disambiguation.rs`. All acceptance criteria now met.
- 2026-05-31: Test soundness audit (csl26-ucs3). Corrected `[x]` → `[ ]` for
  multilingual key generation — `render_name_for_disambiguation` not yet
  implemented; `disambiguation.rs` always reads `Contributor::Multilingual.original`.
  The follow-up implementation is recorded in the 2026-05-31 csl26-54jn changelog
  entry above.
- 2026-05-29: Initial version. Consolidates `DISAMBIGUATION_IMPLEMENTATION_PLAN.md` (now deleted)
  and `DISAMBIGUATION_MULTILINGUAL_GROUPING.md` (now deleted).
- 2026-05-29: All acceptance criteria implemented; status set to Active.
- 2026-05-29: Removed `disambiguate.ignore` (doubly-redundant no-op option); added `div-009`
  to Divergence Register grounding the issued-only keying decision in APA §8.15 / Chicago.
