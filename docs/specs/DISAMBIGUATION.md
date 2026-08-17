# Disambiguation Specification

**Status:** Active
**Date:** 2026-05-29
**Related:** [`docs/reference/DISAMBIGUATION.md`](../reference/DISAMBIGUATION.md),
[`docs/specs/MULTILINGUAL.md`](./MULTILINGUAL.md),
[`docs/specs/MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md`](./MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md),
[`docs/specs/DATE_FALLBACK.md`](./DATE_FALLBACK.md),
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
appended to the author key, when that year is present and parses cleanly.
When it is not — no `issued` value at all, or one that doesn't reduce to a
clean numeric year (e.g. a literal like `"c1988"`) — the key falls through to
the **date-slot discriminant** described below (`csl26-huuz`). No other date
field ever substitutes for a *present* issued year in the key.

#### Date-slot discriminant when the issued year is absent (csl26-huuz)

The first recursively encountered `date: issued` component is the identity
slot. Disambiguation resolves that slot with the same effective
`options.date-fallback.first-issued` rule used by rendering; it does not inspect
or clone template fallback components.

- A real issued value contributes its rendered year identity.
- A date fallback contributes its rendered value, affixes, wrapping, and note,
  except that `accessed` remains retrieval metadata and contributes no work
  identity.
- A message fallback contributes its locale-resolved text, so anonymous
  no-date works that look alike collide consistently.
- Omitted policy, an unmatched selector, or `none` contributes an empty
  discriminant. Existing standalone year-suffix rendering remains available
  for a colliding blank slot.

The effective bibliography policy is preferred for the one style-wide suffix
assignment, with citation policy used when no bibliography slot exists. See
[`DATE_FALLBACK.md`](./DATE_FALLBACK.md) for lane, selector, scope, and clear
semantics.

<details>
<summary>Historical v1 implementation notes (superseded)</summary>

Grouping by abstract variable equality alone is wrong once a style's date
slot is type-conditional: two undated references can render *different* text
for their date position (one style's `article-journal` branch renders
nothing at all where every other type renders the locale's "no date" term,
GB/T 7714 §7714.7.2), and two references *are* already visually
distinguishable whenever their date slots render differently — grouping them
for year-suffix purposes would be wrong on Citum's own terms, independent of
what citeproc-js does. Conversely, references whose date slots render
*identical* text must still collide, or the group won't get the shared
suffix sequence a reader would expect.

The fix keeps `build_group_key`'s existing shape (author key + a date
discriminant) but computes the discriminant from the reference's **resolved
template** — the first date component under the author not marked
`suppress-disamb-suffix` — instead of assuming a uniform outcome:

1. **The date variable resolves to a real, non-empty value** → the text it
   would actually *render* is the discriminant: `form`-restricted formatting
   plus uncertainty/approximation markers, the same pipeline
   `TemplateDate::values` applies — not the raw stored value, whose `Display`
   can carry more precision than `form` shows (a day-precision `copyright`
   date under `form: year` renders as a bare year but its raw value still has
   the day; reading the raw value could split a group whose members render
   identically, flagged in PR review). For a fallback candidate specifically,
   this also includes the candidate's own prefix/suffix/wrap and the
   resolved value's `note` — the same extra text
   `apply_fallback_component_rendering`/`append_note` add to the bare value
   during real rendering (GB/T's `book,thesis,map` chain prefixes
   `copyright` with `c` and suffixes `printing` with `印刷`; two references
   resolving each can share a bare year while rendering visibly different
   text, and must not collide).
2. **The variable is empty, and the resolved fallback chain (explicit
   `fallback:`, or the implicit no-`fallback:` branch) renders the locale's
   no-date term** → a discriminant for that term, scoped by the reference's
   effective language (mirrors the anonymous-fallback author key below — the
   term itself varies by language, `无日期` vs `n.d.`). The implicit and an
   explicit `fallback: [message: term.no-date]` render identical text, so
   both compute the identical discriminant.
3. **An access date (`DateVariable::Accessed`) is the only thing that would
   resolve** — whether it's the slot's own primary variable or a fallback
   candidate — **the discriminant is empty**, matching case 4, never the
   access date's value. An access date is retrieval metadata, not part of a
   work's identity; two references differing only in *when* they were
   accessed must not be treated as distinguishable. This mirrors, but is not
   derived from, citeproc-js's `just_looking`-time suppression of accessed
   dates during its own ambiguity computation — the two independently land
   on the same rule because it's the correct rule, not because one copies
   the other.
4. **Nothing resolves at all** (an explicit `fallback: []`, or no template
   configured) → empty discriminant, the same value case 3 produces.
   Expressing "this reference type's date logic has nothing else to show for
   a missing date" as an empty `fallback:` list overloads an empty
   collection as a semantic signal — flagged as a design concern, not fixed
   here. A `date-substitute` options-level mechanism (mirroring
   `author-substitute`/`Substitute` in `options/substitute.rs`) is planned as
   a stacked follow-up that would express this declaratively instead —
   `csl26-qbmd`, designed together with this discriminant rather than as an
   independent fix, since it will own most of what this function reads.

Case 3's "stop, don't fall through" behavior matters: once a candidate in the
fallback chain *would* be selected (its underlying variable has a value,
even if that value contributes no discriminant), the search stops there — it
does not continue scanning for a later candidate. This mirrors the
if/else-if/else shape the fallback chain represents: a present access date
selects that branch, whose content is then suppressed for grouping purposes;
it does not make the chain fall through to the no-date term as if the access
branch had never matched.

**Spec resolution order is bibliography-preferred, not citation-preferred**
(`Disambiguator::date_slot_discriminant`, `first_date_component_for_bibliography`
before `first_date_component_for_citation`, `sorting.rs`) — the opposite
order from the author-key list-primary resolution in `build_reference_cache`.
This was confirmed empirically, not assumed: a style's `citation:` template
is commonly a simpler, non-type-differentiated form of the same date logic
(`gb-t-7714-2025-author-date`'s `citation:` section has one flat
`date: issued` component with no `type-variants:` at all, unlike its
`bibliography:` section). Preferring `citation_spec` first — the initial
design — let that undifferentiated template collapse every undated reference
onto one discriminant regardless of type, silently defeating the
type-conditional split this mechanism exists to make; verified against the
GB/T oracle before landing the bibliography-first order.

The selected slot carries its effective scope configuration with it:
bibliography slots use the effective bibliography options, while the citation
fallback slot uses the effective citation options. This keeps date markers and
candidate-note behavior aligned with the scope whose template supplied the
identity slot.

**Why not compute citation and bibliography membership separately** (raised
in PR review): a reference's year-suffix letter must be identical everywhere
it's cited — a citation showing "2020a" and the bibliography showing "2020b"
for the same reference is a worse, more visible defect than the asymmetry a
per-scope split would fix. It's also not what the oracle does: this
mechanism's design evidence was citeproc-js's own `registry.ambigcites` for
this exact style, captured once for the whole style (无日期×23, n.d.×10,
empty×3, empty×2) — one set of groups, used for both citation and
bibliography rendering, because upstream's `date-intext` macro is literally
the same macro referenced from both the `<citation>` and `<bibliography>`
layouts in the CSL source. Bibliography-preferred is correct precisely
*because* citum's bibliography template is the more complete mirror of that
shared macro. The citation template's own lack of type-conditional structure
is a separate, pre-existing migration gap, not something this mechanism can
or should absorb by fragmenting the letter itself; tracked as a follow-up
under `csl26-6eak`.

Both `first_date_component_for_citation` and
`first_date_component_for_bibliography` resolve their language via
`crate::values::effective_item_language`, matching the real render path
(`processor/rendering/mod.rs::locale_for_reference`) and this mechanism's
own no-date-term discriminant scoping — not the bare `reference.language()`
the pre-existing `primary_contributor_for_citation`/
`primary_contributor_for_bibliography` use (a latent, unrelated
inconsistency, out of scope here; flagged in PR review).

No new schema surface: `TemplateDate.fallback: Option<Vec<TemplateComponent>>`
already existed and the renderer already walked it in order
(`values/date.rs`); the discriminant reads data styles already carry.

**Risk containment:** when the issued year is present and parses, the key is
byte-identical to before this change. The discriminant path only ever
*splits* a group whose members' date slots resolve differently — it never
merges references that previously formed separate groups.

**Rendering fix required alongside grouping.** `values/date.rs`'s fallback
render path previously only inlined a year-suffix letter for a `message:`
fallback candidate (`csl26-6eak`), appending it *after* that candidate's own
wrap/affixes were applied. Two more cases needed the same treatment once
grouping could split on them:
- A `date:` fallback candidate (e.g. an access-year fallback rendering
  `Anon，[2020a]`) needs the letter inlined into the raw formatted text
  *before* the candidate's own wrap is applied, so it lands inside the
  brackets rather than after them.
- An empty resolution (nothing in the fallback chain renders anything) still
  needs the group's disamb suffix rendered standalone — upstream's bare
  `<text variable="year-suffix"/>` after an empty date, oracle: `Anon，b.` —
  otherwise an entry whose date slot is entirely empty silently loses its
  disambiguator rather than getting the wrong one.

**Scope: membership, not order.** This mechanism fixes collision-group
*membership*. Within-group *letter order* still rides on §3's resolved-sort
machinery (`csl26-m8la`) — a style with no real `bibliography.sort` of its
own (like `gb-t-7714-2025-author-date`, which inherits `citation-number`
registry order from its numeric base) will still assign letters in registry
order, which can disagree with the oracle's actual bibliography-sort order
for groups this mechanism newly separates out. That gap is `csl26-q67h`.

</details>

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

`build_author_slot_key` resolves the same effective `substitute` policy used by
the selected citation or bibliography scope. A promoted title remains a
per-reference identity. When the chain instead reaches a constant
`substitute.otherwise` locale message (for example, GB/T's `佚名` anonymous
placeholder), its rendered message becomes the shared author discriminant.
Works that display the same anonymous label therefore collide by year like
works with the same named author. There is no template fallback or separate
disambiguation-only substitute chain.

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
`apply_name_partitions` / `select_givenname_resolution` /
`apply_year_suffix`.

### 2.1 `givenname-disambiguation-rule`

Specifies which author positions receive given-name expansion. The field lives on
`Disambiguation` in `citum-schema-style/src/options/processing.rs` as
`givenname_rule: GivennameRule`. Default: `by-cite`.

**Invariant, all rules:** the ambiguity universe is always every reference in the
document, never a subset. A collision group is computed once, globally, in
`Processor::calculate_hints` (`processor/setup.rs`), and that is the only
computation that runs — no rule recomputes or narrows it. This mirrors
citeproc-js, whose ambiguity pool (`CSL.Registry.prototype.ambigcites`) is a
property of the registry, populated over every registered item, with no
per-cite scoping at any point in `getAmbiguousCite`. Two different authors
named "Smith" cannot be told apart by looking at one citation in isolation —
telling them apart requires comparing every "Smith" in the bibliography, which
is what "disambiguation" means. See §2.1.1 for what actually varies between
rules.

| CSL value | Engine scope | Notes |
|---|---|---|
| `by-cite` *(default)* | global collision detection, per-cite expansion ceiling | see §2.1.1 |
| `all-names` | global expansion | all affected citations use the expanded form consistently |
| `all-names-with-initials` | expand all positions | initials vs full controlled by contributor config `initialize-with` |
| `primary-name` | expand **first author only** | required by Chicago author-date |
| `primary-name-with-initials` | expand **first author only** | required by APA 7; initials via contributor config |

**Key invariant:** initials vs full given name is always driven by the contributor
config's `initialize-with` / `name-form` settings, not by this rule. The rule
controls only *which positions* are eligible for expansion.

#### 2.1.1 `by-cite` (csl26-lvib, corrected 2026-08-16)

`by-cite` is a **given-name** rule (cascade strategy 2, §2). It has no authority
over strategy 1 (et-al expansion via `disambiguate-add-names`) or strategy 3
(year suffix); both remain global under every `givenname_rule` value, including
`by-cite`.

citeproc-js is explicit that `by-cite` does not change *which references are
compared*: internal to `CSL.Disambiguation`, a `by-cite` rule is rewritten to
`all-names` for the purpose of selecting eligible positions
(`if (gdropt === "by-cite") { gdropt = "all-names"; }`), and the ambiguity pool
those positions are checked against is the same registry-wide pool every other
rule uses. What `by-cite` actually changes is narrower: it caps how far a
*given* rendered cite is allowed to escalate (`givensMax`), so a cite showing
two authors is not forced to add given names for a third author hidden behind
`et al.` in that same cite. It does not, and cannot, shrink the set of
references consulted to decide *whether* a collision exists — doing so would
make disambiguation depend on which references happen to be cited together,
which defeats disambiguation's purpose (§2.1's invariant above).

**2026-06-02–2026-08-16 implementation (csl26-lvib) was wrong.** It approximated
`by-cite`'s per-cite escalation cap by *narrowing the comparison set* instead:
`citation_scoped_by_cite_hints` cleared the global hints for every reference in
a citation and recomputed them from a bibliography containing only that
citation's items, short-circuiting to "no collision" for any citation naming
fewer than two references. This silently under-disambiguated any reference
cited alone that belonged to a global collision group — including via strategy
1 (et-al depth), which `by-cite` has no claim on at all — and made `by-cite`
observably *narrower* than `all-names` rather than narrower-in-a-different-axis,
inverting the relationship citeproc-js implements. Found via csl26-8nrt: a
`disambiguate-add-names` collision (two Smith/Lee/2021 papers diverging at the
third author) rendered the correct three-name expansion only when both
colliding references were cited together, and silently collapsed to one name
whenever either was cited alone.

**Current state (csl26-5753, 2026-08-17):** `by-cite` now implements a true
per-position given-name expansion ceiling, distinguishable from `all-names`.
The collision group is still computed once, globally (§2.1's invariant is
untouched — `by-cite`'s per-position search still runs against the full
document-wide group), but instead of the uniform "escalate the whole
reference to Initials or Full" flag every other rule uses, `by-cite` tracks
escalation per author position (`ProcHints.expand_given_names_full_positions`,
`Option<Vec<bool>>`, index-aligned to the rendered name list). The uniform
`expand_given_names` trigger still governs *whether* a position's given name
is revealed at all — this is Citum's own simplification, not a citeproc-js
behavior: real citeproc-js keeps that reveal itself per position too (a
`request_base` floor in `evalname`), so a shared, non-disambiguating
position on a short-form citation baseline can stay at bare family form
while only the colliding position promotes to a given name at all
(`disambiguate_ByCiteBaseNameCountOnFailureIfYearSuffixAvailable`). Citum's
`Disambiguator` has no visibility into the citation template's
`ContributorForm` to replicate that promotion-level distinction, so it
promotes every currently-shown position uniformly and only varies *depth*
per position (tracked as csl26-7jej). Once a position is revealed, it
escalates to the full given name only if the default depth (initials, when
reachable; otherwise the full given name directly) still collides with
another member of the group. A position identical across every colliding
reference (e.g. a shared first author) never escalates past its default
depth, since doing so could never help distinguish anything — matching the
official CSL test suite's `disambiguate_ByCiteMinimalGivennameExpandMinimalNames`
and `disambiguate_ByCiteGivennameExpandCrossNestedNames` fixtures, including
the latter's varying per-reference shown name count within one collision
group. Growing the shown name count itself remains strategy 1
(`disambiguate-add-names`), which `by-cite` has no authority over — the
search only grows `n` when that strategy is separately enabled. Implemented
in `Disambiguator::select_by_cite_resolution` /
`resolve_by_cite_positions` (`processor/disambiguation.rs`).

**Rejected alternative:** keep the citation-scoped overlay but carve out
`min_names_to_show` from the clear whenever the global hint's
`expand_given_names` is `false` (i.e. the collision was resolved by pure
et-al expansion, strategy 1, with no given-name involvement). This is a
minimal, test-preserving patch — every existing `by-cite` test keeps its
current expected output — but it was rejected: it only patches the strategy-1
leak this bean found, leaves the deeper error (comparing against a
citation-scoped subset at all) in place for strategy 2, and requires
inventing an unwritten rule ("et-al depth is global only when given names
weren't also involved") with no counterpart in citeproc-js to justify it.

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

APA still needs `primary-name` rather than `by-cite`, but for the reason §2.1's table
actually gives: `primary-name` restricts expansion to the **first author only** — the
correct rendering under APA §8.20, which shows initials on the first author of each
colliding surname, not on every author. `by-cite` carries no such restriction and, per
§2.1.1, is document-wide like every other rule; it would expand every colliding
position rather than only the first. Initials vs. full given name is then driven by
each style's contributor config (`initialize-with` / `name-form`), per the §2.1
invariant — so one rule serves APA (initials) and Chicago (full given name).

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
- [x] Name disambiguation (et-al expansion and given-name expansion alike) always
  compares against every reference in the document; no `givenname-disambiguation-rule`
  value narrows the comparison set (§2.1.1, csl26-8nrt — corrects the 2026-06-02
  citation-scoped `by-cite` implementation)
- [x] `by-cite` implements a true per-position given-name expansion ceiling
  (distinguishable from `all-names`); csl26-5753, §2.1.1
- [x] Upstream CSL disambiguation fixtures that distinguish `by-cite` and
  `all-names` are mirrored natively (`disambiguation_givenname_escalation_is_minimal_per_position`,
  `disambiguation_givenname_escalation_splits_positions_independently_per_collision`
  in `crates/citum-engine/tests/citations.rs`), matching
  `disambiguate_ByCiteMinimalGivennameExpandMinimalNames` and
  `disambiguate_ByCiteGivennameExpandCrossNestedNames`
- [x] Year-suffix order follows the article-stripped/locale-collated bibliography
  sort, not a raw lowercased title (csl26-2zy6, audit row 138)
- [x] APA-7th carries `add-givenname` + `primary-name-with-initials` (global) so
  same-surname authors get initials, not a spurious year suffix (csl26-2zy6, row 114)
- [x] MLA disables `year_suffix` and disambiguates same-author works via the
  `disambiguate-only` short title (csl26-2zy6, row 173)
- [x] When the issued year is absent, collision-group membership follows a
  date-slot discriminant computed from the resolved template rather than a
  uniform "no date" assumption (csl26-huuz)
- [x] An access date never contributes to the discriminant, whether primary
  or fallback, present or absent (csl26-huuz)
- [x] The implicit no-`fallback:` no-date-term path and an explicit
  `fallback: [message: term.no-date]` compute the identical discriminant
  (csl26-huuz)
- [x] A resolving candidate's discriminant reflects `form`-restricted
  rendered text, prefix/suffix/wrap, and `note` — not the raw stored date
  value's precision (csl26-huuz)
- [ ] A declarative, type-scoped mechanism (not an empty `fallback:` list)
  expresses "nothing else to show for a missing date" (`csl26-qbmd`,
  stacked follow-up)

## Related specs

- [CITATION_REGIME](CITATION_REGIME.md) — disambiguation is regime-scoped.
  Author-date and label disambiguation settings must not leak into numeric
  styles through style inheritance; the regime guard in `merge_style_overlay`
  prevents this for `citation.non_integral` (which carries disambiguation-derived
  author-date citations).

## Changelog

- 2026-08-17: Implemented csl26-5753: `by-cite` now escalates given names per
  author position instead of uniformly across the whole reference. `ProcHints`
  gained `expand_given_names_full_positions: Option<Vec<bool>>`, index-aligned
  to the rendered name list; every other `givenname_rule` leaves it `None` and
  keeps the existing uniform `expand_given_names_full` path byte-for-byte.
  `Disambiguator::select_by_cite_resolution` tries strategy 1 (name-count
  growth) first, exactly as every other rule, then hands any still-colliding
  family bucket to `resolve_by_cite_positions`, which escalates positions left
  to right — only committing an escalation when it actually reduces the
  bucket's remaining collisions — and grows the shown name count further only
  when `disambiguate-add-names` is enabled: growing `n` is strategy 1's
  property, which `by-cite` has no authority over (elsevier-harvard,
  elsevier-vancouver-author-date, and gb-t-7714-2025-author-date don't enable
  strategy 1, so an unconditional grow would render full given-name
  expansions where the oracle prefers plain year-suffix). `report-core.js
  --all-features` corpus sweep across the embedded portfolio: zero
  regressions, one genuine gain (`gb-t-7714-2025-author-date` bibliography,
  +2 exact-parity entries, given names correctly capped at initials instead
  of escalating to full). Native regressions mirror the official CSL test
  suite's `disambiguate_ByCiteMinimalGivennameExpandMinimalNames` and
  `disambiguate_ByCiteGivennameExpandCrossNestedNames` fixtures. The
  promotion-vs-depth simplification noted in the paragraph above is tracked
  separately as csl26-7jej.
- 2026-08-16: Corrected §2.1.1: `by-cite` is document-wide, not citation-local
  (csl26-8nrt). The 2026-06-02 `by-cite` implementation (csl26-lvib) approximated
  a per-cite given-name expansion ceiling by narrowing the comparison set to the
  current citation's references instead — this cleared `min_names_to_show` for
  every citation-scoped hint, silently discarding et-al-expansion results
  (strategy 1) that `by-cite`, a given-name rule (strategy 2), has no authority
  over. Found via a `disambiguate-add-names` collision (two same-year references
  diverging only at the third author) that expanded correctly only when both
  colliding references were cited together, collapsing to one name whenever
  either was cited alone. citeproc-js confirms the correction: its ambiguity
  pool (`CSL.Registry.ambigcites`) is populated over the whole registry with no
  per-cite scoping, and `by-cite` is internally rewritten to `all-names` for
  position selection — the rule only caps escalation depth, never the
  comparison set. Engine: removed `citation_scoped_by_cite_hints` and
  `uses_by_cite_givenname` from `processor/citation.rs`; citations render
  directly from the global hint map like every other rule. `by-cite` and
  `all-names` are now behaviorally identical pending a real per-position
  expansion mask (tracked as `csl26-5753`). Rejected a narrower patch that
  would have carved out `min_names_to_show` from the clear without removing the
  citation-scoped comparison itself — see §2.1.1 for why.
- 2026-08-16: Centralized author and issued-date fallback policy in options.
  Anonymous messages and first-issued date resolution now share the exact
  rendering policy; template fallback chains no longer exist.
- 2026-08-12: Review follow-up on the date-slot discriminant (csl26-huuz).
  A resolving candidate's discriminant now reflects `form`-restricted
  rendered text plus prefix/suffix/wrap/`note` (`values/date.rs` gained
  `fallback_candidate_discriminant`) instead of the raw stored date value,
  which could over-collapse two candidates that render visibly different
  text (a `c`-prefixed `copyright` year and a `印刷`-suffixed `printing`
  year sharing a bare year, or a day-precision date under `form: year`).
  `first_date_component_for_citation`/`_for_bibliography` now resolve
  language via `effective_item_language`, matching the real render path,
  instead of bare `reference.language()`. Investigated and declined a
  fourth review finding (splitting citation and bibliography disambiguation
  membership into separate hints) — see the bibliography-preferred
  rationale above; a reference's letter must stay identical across both
  scopes. A `TemplateDate.suppress-no-date-term` flag was tried and reverted
  before landing — the empty-`fallback:`-list-as-signal concern it answered
  is real, but a per-component flag would just be one more thing for a
  planned `date-substitute` options mechanism (`csl26-qbmd`, mirroring
  `author-substitute`) to deprecate; `gb-t-7714-2025-author-date.yaml`'s
  `article-journal,article-magazine` stays on `fallback: []` until that
  mechanism lands, designed together with this discriminant rather than
  built independently.
- 2026-08-11: Added the date-slot discriminant (csl26-huuz), closing the gap
  §1's changelog entry below flagged as unfixed. `build_group_key`'s
  no-issued-year fallthrough now reads the reference's resolved date
  component instead of collapsing every undated reference onto one key.
  Engine: `sorting.rs` gained `first_date_component_for_bibliography`/
  `_for_citation` (mirroring the existing contributor-resolution helpers);
  `disambiguation.rs` gained `date_slot_discriminant`/
  `date_component_discriminant`; `values/date.rs`'s fallback render path
  (`render_date_fallback_chain`) gained in-wrap suffix inlining for a
  `date:` fallback candidate and a standalone-suffix case for an empty
  resolution. Style: `gb-t-7714-2025-author-date.yaml`'s
  `article-journal,article-magazine` type-variant lost its `term.no-date`
  fallback (upstream's date-intext macro never reaches it for that branch);
  its `webpage,post,post-weblog` type-variant gained an access-year fallback
  ahead of the no-date term. `gb-t-7714-2025-author-date`'s diagnostic
  upstream-corpus bibliography scope (`count_toward_fidelity: false`, so no
  fidelity-gate impact) went 147/203 → 176/203; zero regressions across the
  35-style exemplar corpus (`report-core.js --all-features`) or
  `cargo nextest run`. The five entries whose date-slot grouping is now
  correct but whose letters still depend on registry order remain tracked
  in `csl26-q67h`, not closed here.
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
  opt-out for a redundant `issued` occurrence) and the initial anonymous
  sentinel key. The 2026-08-16 fallback-policy change replaced the associated
  component fallback and scoped-config gap with shared options resolution.
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
