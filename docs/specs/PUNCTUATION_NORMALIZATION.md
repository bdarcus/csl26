# Punctuation Normalization Design

**Status:** Active
**Date:** 2026-02-15
**Updated:** 2026-07-12 (locale-configurable collision policy activated; see
Changelog)
**Related:** CSL schema#379 (upstream, "Make punctuation collapsing
localisable" — tracked as bucket-1-partial in
[2026-07-12_CSL_SCHEMA_ISSUE_TRIAGE.md](../architecture/audits/2026-07-12_CSL_SCHEMA_ISSUE_TRIAGE.md));
bean `csl26-zfqr` (structured-title delimiter suppression, blocked on this
spec's design decision)

## Current State

Citum currently handles punctuation placement with:
- Boolean `punctuation-in-quote: true/false` (American style only)
- Ad-hoc logic scattered throughout `render.rs` that moves periods inside/outside quotes during rendering
- Only handles curly and straight double quotes
- Tightly coupled to the rendering process

**Problems:**
- Hard to reason about correctness (3 separate locations in render.rs doing similar logic)
- Only supports American English convention
- Fragile: quote style changes (straight → curly) require updating all punctuation logic
- Not locale-aware
- Cannot support other language conventions (German, French, etc.)

## Better Approach: Separate Normalization Phase

Based on org-cite's `org-cite-adjust-punctuation` design (see [mailing list post](https://lists.nongnu.org/archive/html/emacs-orgmode/2021-05/msg00714.html) and [source code comments](https://github.com/bzg/org-mode/blob/main/lisp/oc.el)), punctuation normalization should be:

1. **A separate processing phase** that runs after component assembly but before final rendering
2. **Language-aware** based on document locale
3. **Configurable** with three orthogonal parameters instead of one boolean

### Three-Parameter Model

```yaml
punctuation:
  movement: inside | outside | strict
  citation-position: inside | outside  # relative to quotes
  citation-order: before | after       # relative to punctuation
```

**Language conventions:**
- **American English**: `movement: inside, citation-position: outside, citation-order: after`
  - "Text." → citation → more text
  - Periods/commas move inside closing quotes

- **British English**: `movement: outside, citation-position: outside, citation-order: after`
  - "Text". → citation → more text
  - Punctuation stays outside quotes

- **German**: `movement: strict, citation-position: outside, citation-order: after`
  - Punctuation doesn't move
  - Citation comes after quotes

- **French**: `movement: strict, citation-position: inside, citation-order: before`
  - Punctuation doesn't move
  - Citation comes inside quotes before punctuation

### Processing Order

Nicolas's key insight: **"Call adjust-punctuation first, before wrap-citation"**

This suggests the pipeline should be:
1. Assemble components with their content
2. **Normalize punctuation** (separate phase, locale-aware)
3. Wrap citations in delimiters
4. Apply formatting (italics, quotes, etc.)
5. Concatenate with separators

Currently we do #2 and #4 together, which is why quote style changes break punctuation logic.

## Migration Path

### Phase 1: Refactor current code (no breaking changes)
- Extract punctuation logic into a single `normalize_punctuation()` function
- Make it handle both straight and curly quotes uniformly
- Keep existing `punctuation-in-quote: bool` as interface

### Phase 2: Extend for multilingual (breaking schema change)
- Replace boolean with three-parameter model
- Add locale-awareness (derive from document `lang` field or style metadata)
- Default to current behavior for backwards compatibility

### Phase 3: Separate phase (architectural)
- Move punctuation normalization to its own processing step
- Run after template assembly, before formatting
- Easier to test, reason about, and extend

## Related Work

### CSL 1.0
Has `punctuation-in-quote` attribute but it's underspecified:
- Only handles periods and commas
- No guidance on interaction with citations
- Assumes American convention

### CSL-M (legal citations)
Extended for legal citations but still American-centric.

### biblatex
Has `autopunct` feature that's more sophisticated:
- Handles multiple punctuation marks
- Language-aware via babel/polyglossia integration
- Separate from formatting logic

## Cross-Locale Prior Art (2026-07-12 research addendum)

This section covers the second half of the punctuation-normalization
problem this doc names but never designed: not quote *movement* (covered
above), but **collision resolution at join points** — what happens when a
title/field already ends in punctuation and a style delimiter or suffix
would add more. This is the class of bug described in CSL schema#379: a
German style needs `Titel!,` collapsed to `Titel!`, which English does not
require. Findings below are external research (dated, attributed), not a
committed decision — see "Recommended Design" for the decision itself.

### Per-locale adjacency patterns

- **German**: suppress a trailing comma after `!`/`?` when the mark comes
  from the source text and the comma from the style (the schema#379
  motivating case). Also a general "don't stack a weak mark after a strong
  one" tendency at quote boundaries.
- **English (US/UK)**: `..` → `.`; `.,` → `.` (most engines treat as
  malformed, drop the comma); `?,`/`!,` conventionally kept as-is — this is
  Citum's current, hardcoded default (see `resolve_punctuation_collision`,
  `crates/citum-engine/src/render/citation.rs:15-55`). Quote-vs-terminal
  ordering (period/comma inside vs. outside closing quotes) is the
  US-vs-British split this doc's existing three-parameter model already
  covers.
- **French**: required (often narrow-NBSP U+202F, or NBSP for `:`/guillemets)
  space before two-part punctuation `: ; ! ?` and around `« »`; a documented
  France-vs-Québec variant (Québec often omits the space before `! ? ;` but
  keeps it before `:` and around guillemets); and a collision pattern
  structurally identical to the German case — a full stop inside guillemets
  is often suppressed when an external comma follows: `« titre. »,` →
  `« titre »,`.
- **Spanish**: adds no new collision *pairs* beyond the quote/punctuation-
  ordering question this doc's `citation-position`/`citation-order`
  parameters already model; opening `¿`/`¡` marks don't interact with
  citation delimiters.
- **CJK**: full-width punctuation (`。 ， 、 ！ ？`) rarely collides with
  Latin delimiter punctuation in practice; where it would, the convention
  favors collapsing to the CJK mark rather than stacking both.

### Standards coverage: CLDR/ICU/Unicode

No existing standard defines "when text ends in X and a style adds Y,
rewrite XY to Z" directly — there is no punctuation-collision table to
adopt wholesale. What *is* useful: Unicode UAX #14 (line breaking) and UAX
#29 (text segmentation) classify punctuation into categories (sentence
terminal, mid-sentence continuation, open/close) that map reasonably onto
the `StrongTerminal`/`WeakTerminal`/`CommaLike` classes below, giving a principled
basis for the engine's internal character classification instead of an
ad-hoc one. CLDR locale data does encode quote-style and some spacing
conventions (e.g. French NBSP usage) but not collision/collapsing rules.

### biblatex prior art (collision-specific)

Beyond `autopunct` (noted above): biblatex's citation commands do
"punctuation recognition" to avoid doubling trailing punctuation from a
macro against external punctuation — its own documented convention is that
`?,` is "OK" (kept), matching Citum's current English default. `\nopunct`
provides a one-shot suppression marker consumed by the next punctuation
command. `\setunit*` vs `\setunit`/`\newunitpunct` is field-boundary
"only punctuate if the previous field produced output" — a different but
related abstraction (unit punctuation, not collision resolution).

### CSL-M / citeproc-js prior art — and why it's not the model here

The concrete proposal on schema#379 itself
(`Juris-M/citeproc-js#154`) is a fully general per-style `<punct-handling>`
XML table: an explicit input-pattern → output-pattern rewrite rule for
every punctuation-mark pair a style author wants to override. This is
**deliberately not** the shape recommended below — see "Recommended
Design" for why.

## Recommended Design

Decision, informed by the research above: extend `grammar-options` with a
small number of **narrow, named fields** (locale-level default, per-style
override), matching the existing precedent of `note-punctuation` /
`note-number` / `note-marker-order` (default in `grammar-options`,
overridable via `options.notes.*` — see
[AUTHORING_LOCALES.md](../guides/AUTHORING_LOCALES.md)) — **not** a general
pattern-rewrite table like CSL-M's `<punct-handling>` proposal.

**Why not the general table:** it's exactly the kind of open-ended,
free-form style-authoring surface Citum's other `grammar-options` fields
deliberately avoid (see [DESIGN_PRINCIPLES.md](../architecture/DESIGN_PRINCIPLES.md),
"explicit over magic"). A full N×N rewrite table also can't be validated or
schema-documented meaningfully — every rule is opaque strings. The research
above suggests the realistic cross-locale need is small and enumerable
(a handful of class-pair policies plus one mark-set field), which is a much
better fit for Citum's existing style.

**Punctuation classes** (informed by UAX #14/#29 categories):
- `StrongTerminal`: `?`, `!`, `…`
- `WeakTerminal`: `.`, `:`
- `CommaLike`: `,`, `;`

**Collision-policy fields** are locale defaults under `grammar-options` and
may be overridden per style under `options.punctuation`:

- `strong-terminal-comma-policy: keep-both | keep-terminal` controls only a
  `StrongTerminal` followed by a style-supplied comma. English, Spanish,
  Basque, and Turkish default to `keep-both`; German and French default to
  `keep-terminal`.
- `delimiter-suppressing-terminal-marks` is a string-set field with default
  `"?!…"`. It is the single field reserved for `csl26-zfqr`: when a
  structured-title part ends in one of these marks, that follow-up will drop
  the punctuation core of the configured delimiter while preserving its
  whitespace tail.

All other punctuation pairs retain Citum's existing resolver matrix. This is
an explicit compatibility decision: the first localized policy does not
reinterpret weak/strong/comma-like combinations that existing styles and
fidelity tests already expose. Broader class-matrix normalization requires
separate evidence and a separate compatibility review.

The style override has this shape:

```yaml
options:
  punctuation:
    strong-terminal-comma-policy: keep-terminal
    delimiter-suppressing-terminal-marks: "?!…"
```

Locale defaults are resolved first and any non-null style field replaces the
corresponding locale value.

- A suppressing-mark set for delimiter suppression (not collision resolution
  itself) is therefore shared with `csl26-zfqr`, rather than introducing a
  second title-specific field.
- French spacing (NBSP/narrow-NBSP before `: ; ! ?` and around guillemets,
  with a France/Québec variant) is a real, researched need but is scoped as
  a **follow-up**, not required for the schema#379/zfqr fix — track
  separately once the collision-policy fields above are settled, to avoid
  scope creep in the first implementation pass.

The compatibility matrix and the two field names above are normative for this
first implementation.

## Implementation Notes

### Locale-supplied `punctuation-in-quote` default

`punctuation-in-quote` is a *style* option (`Config.punctuation_in_quote`), but every
bundled locale also declares `grammar-options.punctuation-in-quote` (`en-US: true`, all
other bundled locales: `false`) — matching citeproc-js, which reads the option from the
resolved locale rather than the style. `Processor::resolve_punctuation_defaults`
(`crates/citum-engine/src/processor/setup.rs`) closes that gap: when a style leaves the
option unset, the engine fills it from the active locale.

Two rules govern this, both enforced in `resolve_punctuation_defaults` /
`punctuation_defaults_require_resolution`:

- **Style-explicit wins, where that can be told apart from unset.** The other two
  grammar options are `Option<T>` on `Config`, so "unset" is a native, unambiguous
  state. `punctuation_in_quote` is a plain `bool`
  (`skip_serializing_if = "std::ops::Not::not"`), so an authored `false` and an unset
  field defaulting to `false` are typed identically — the flattened `Config` alone can't
  tell them apart. `Processor::punctuation_in_quote_explicitly_authored` closes most of
  that gap by consulting the raw-YAML-aware sources still available at this point: the
  style's own captured `raw_yaml` (top-level `options.punctuation-in-quote`), and the
  extends-chain-merged `scoped_raw_options` for the citation/bibliography scope being
  resolved. Not caught: an *ancestor* style's global `options.punctuation-in-quote` that
  no descendant repeats anywhere (`raw_yaml` is leaf-only; unlike `scoped_raw_options`, it
  is not chain-merged across `extends`). No embedded or exemplar style hits this today —
  none sets `punctuation-in-quote: false` at all — and closing it fully needs
  csl26-yxay's `Option<bool>` migration.
- **A substituted locale is not authoritative.** `Locale::load` and
  `citum_store::load_locale_or_default` silently fall back to `Locale::en_us()` (or a
  language-prefix match) when the style's declared locale can't be resolved — e.g. a
  style declaring `en-GB`, which isn't bundled. That fallback locale carries the
  *substitute* language's conventions, not the one the style asked for, so none of its
  grammar options — not just `punctuation-in-quote` — are applied as style defaults. The
  fallback is recorded on `Locale::resolved_by_fallback`, set by the two loader sites via
  `Locale::resolved_for(requested)`.

### Current bugs to watch for:
1. **Quote character assumptions**: Any code that checks `ends_with('"')` must also check `ends_with('\u{201D}')`
2. **Separator conflicts**: Default separator `. ` interacts with quote normalization
3. **Multiple punctuation**: What if title ends with `?` or `!` - do we still add `.`?
4. **Nested quotes**: Single quotes inside double quotes not currently handled

### Testing strategy:
- Unit tests for `normalize_punctuation()` with all language conventions
- Integration tests with real styles (APA, Chicago, German DIN, French CNRS)
- Regression tests for current American behavior

## References

- org-cite design: https://github.com/bzg/org-mode/blob/main/lisp/oc.el
- CSL 1.0 spec: https://docs.citationstyles.org/en/stable/specification.html#punctuation-in-quote
- biblatex autopunct: https://www.ctan.org/pkg/biblatex (sec 3.9)
- CSL schema#379 (upstream): https://github.com/citation-style-language/schema/issues/379
- Juris-M/citeproc-js#154 (German comma-suppression, `<punct-handling>` proposal): https://github.com/Juris-M/citeproc-js/issues/154
- Unicode UAX #14 (line breaking, punctuation classes): https://www.unicode.org/reports/tr14/
- Unicode UAX #29 (text segmentation): https://www.unicode.org/reports/tr29/
- biblatex punctuation recognition discussion: https://tex.stackexchange.com/questions/428478/biblatex-punctuation-recognition
- French spacing conventions (two-part punctuation, guillemets): https://french.stackexchange.com/questions/43153/does-whitespace-before-punctuation-apply-in-all-conditions

## Priority

**Medium-High** for multilingual support (originally tracked as `csln#66`,
pre-rename; now motivated concretely by upstream CSL schema#379 and by
`csl26-zfqr`, which is blocked on this spec's design decision)
**Low-Medium** for current English-only work

However, refactoring current ad-hoc code into a clean function would prevent bugs and make the codebase more maintainable even before multilingual support.

## Related Issues

- CSL schema#379 (upstream) - Make punctuation collapsing localisable
- `csl26-zfqr` - Structured title delimiter suppression after terminal punctuation (blocked on this spec)
- `csln#66` - Multilingual/multiscript support (pre-rename issue tracker; historical reference only)
- PR #51 (pre-rename) - Curly quote rendering (exposed fragility of current approach)

## Acceptance Criteria

- [x] Locale grammar options expose a strong-terminal/comma policy and one
  shared delimiter-suppressing mark set.
- [x] Styles can override either locale value independently.
- [x] Citation and bibliography joins honor `keep-both` and `keep-terminal`
  without changing established results for other punctuation pairs.
- [x] German/French and English-compatible embedded locales declare their
  intended defaults.
- [x] French punctuation spacing remains explicitly out of scope.

## Changelog

- **2026-08-13**: `punctuation-in-quote` now resolves from the active locale's
  `grammar-options` when a style leaves it unset, closing the last follow-up
  from the 2026-08-03 entry below (`csl26-8e75`). A locale substituted for
  one that couldn't be resolved (`Locale::resolved_by_fallback`) is never
  treated as authoritative for this or the other two grammar-option
  defaults. Style-authored `punctuation-in-quote` wins when it can be told
  apart from unset — for YAML-authored styles, at both global and
  citation/bibliography scope — with one narrow residual gap (an ancestor's
  global-only authorship with no descendant repeat) left to csl26-yxay. See
  "Locale-supplied `punctuation-in-quote` default" above.
- **2026-08-03**: Punctuation-in-quote quote movement now applies in every
  output format, not just plain text. Two prior gaps, both raw-string
  assumptions: (1) the join sites that detect a component's own leading
  `.`/`,` (e.g. a `prefix: ". Aired "`) compared a raw first character
  against the rendered text's *visible* first character, which are never
  equal once `apply_component_semantics` wraps the affixed output in a
  format's semantic markup (an HTML `<span>`, ...); (2) `move_punctuation_into_quote`
  matched the closing quote glyph against the raw end of the accumulated
  string, which fails once markup (an HTML `</span>`, a LaTeX `}`) trails it.
  Both are fixed via a `visible_runs`-backed projection (`render::punctuation::
  leading_movable_mark`, `visible_suffix_raw_index`) that locates the mark in
  the *visible* text and edits the *raw* string at the corresponding byte
  offset. No schema or format-trait change — `visible_runs` already existed
  for this exact purpose (`cleanup_dangling_punctuation`).
- **2026-07-12**: Activated the compatibility-first collision policy. Finalized
  `strong-terminal-comma-policy`,
  `delimiter-suppressing-terminal-marks`, direct style overrides, locale
  defaults, and the fixed compatibility boundary for all other pairs.
- **2026-07-12**: Added "Cross-Locale Prior Art" and "Recommended Design"
  sections covering the punctuation-*collision* half of this problem
  (distinct from the quote-*movement* model above), motivated by CSL
  schema#379 and reconciled with `csl26-zfqr`'s independently-designed need.
  Decision: narrow named `grammar-options` fields, not a general rewrite
  table. Updated stale `csln#66`/PR #51 references.
- **2026-02-15**: Initial draft (quote-movement three-parameter model,
  migration path).
