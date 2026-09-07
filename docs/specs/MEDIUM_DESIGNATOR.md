# Medium Designator Specification

**Status:** Draft
**Version:** 1.3
**Date:** 2026-09-06
**Supersedes:** None
**Related:** `csl26-zs9y`, `csl26-8b4a`, `csl26-8z39`, `csl26-la9t`, `csl26-ro72`,
`docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md`,
`docs/specs/ARTICLE_JOURNAL_NO_PAGE_FALLBACK.md`

## Purpose

The NLM/Vancouver citation family marks references it has only ever seen
online with a bracketed `[Internet]` designator, an "Available from:" access
phrase, and a `[cited …]` bracket around the accessed date — three
components that all key off one condition, a URL exists, plus a second
condition selecting *where* the `[Internet]` marker attaches: to the
reference's container title if it has one (a periodical's own volume/issue/
page locator already establishes it's a real, citable, located work
regardless of how it was accessed), or to the reference's own title if it
doesn't.

This is 29+10 = 39 embedded exact-parity rows across
`taylor-and-francis-national-library-of-medicine`,
`springer-vancouver-brackets`, and
`taylor-and-francis-council-of-science-editors-author-date`. It is not
covered by `docs/specs/ALTERNATIVES.md`: there is no fallback here (nothing
is tried and abandoned), it is a marker that appears *in addition to* the
title, not instead of anything.

## Scope

In scope:

- one bibliography option controlling the online-access marker bundle
  (title suffix, access phrase, cited-date bracket) for the vancouver/NLM
  style family;
- the anchor-selection rule (container-title vs. the reference's own title).

Out of scope:

- other families' access-date conventions (Chicago's own `csl26-fsp0`/`ADD`
  URL/DOI policy work is tracked separately under `csl26-h7oc`);
- a general "conditional literal" mechanism — this spec proposes a named
  option, not a reusable template conditional, per the same reasoning
  `ARTICLE_JOURNAL_NO_PAGE_FALLBACK.md` used for its narrower case;
- NLM's DOI-instead-of-detail-block rule for page/volume-less
  `article-journal` entries — that is not this bundle (no URL/medium marker
  involved) and does not belong to `alternatives:` either; see
  `csl26-8z39` (extending `ArticleJournalNoPageFallback`) and
  `docs/specs/ALTERNATIVES.md`'s correction note.

## Evidence

`taylor-and-francis-national-library-of-medicine.csl:133-150` (`title` macro):

```xml
<macro name="title">
  <text variable="title"/>
  <choose>
    <if type="article-journal article-magazine chapter paper-conference article-newspaper" match="none">
      <choose>
        <if variable="URL">
          <text term="internet" prefix=" [" suffix="]" text-case="capitalize-first"/>
        </if>
      </choose>
      <text macro="edition" prefix=". "/>
    </if>
  </choose>
</macro>
```

`springer-vancouver-brackets.csl:113-120` (`accessed-date` macro):

```xml
<macro name="accessed-date">
  <choose>
    <if variable="URL">
      <group prefix="[" suffix="]" delimiter=" ">
        <text term="cited" text-case="lowercase"/>
        <date variable="accessed" form="text"/>
      </group>
    </if>
  </choose>
</macro>
```

Confirmed against 11 T&F-NLM oracle rows (webpage, dictionary, legislation,
map, standard, bill, hearing, software, regulation — all non-periodical).

**A third style in this wave uses a different gate.**
`taylor-and-francis-council-of-science-editors-author-date.csl:58-125`
(`title` and `container` macros) places the same marker, but the condition
is data presence, not a reference-type list:

```xml
<macro name="title">
  <group delimiter=" ">
    <text variable="title"/>
    ...
    <choose>
      <if variable="container-title" match="none">
        <choose>
          <if variable="URL">
            <text term="internet" prefix=" [" suffix="]" text-case="capitalize-first"/>
          </if>
        </choose>
      </if>
    </choose>
  </group>
</macro>
<macro name="container">
  <text variable="container-title" form="short" strip-periods="true"/>
  <choose>
    <if variable="URL">
      <text term="internet" prefix=" [" suffix="]" text-case="capitalize-first"/>
    </if>
  </choose>
</macro>
```

No type list at all: the marker goes on `container` whenever
`container-title` is present, and on `title` only when it's absent —
unconditional on reference type. See "Anchor selection" below for why this
turns out to coincide with NLM/springer's type-based rule rather than
requiring a second mechanism.

**CSE also uses a different term for the accessed-date bracket, not just a
different anchor.** Its macro — confusingly also named `cited`, matching
NLM/springer's macro *name* but not their *term* —
(`taylor-and-francis-council-of-science-editors-author-date.csl:96-105`):

```xml
<macro name="cited">
  <choose>
    <if variable="URL">
      <group delimiter=" " prefix=" [" suffix="]">
        <text term="accessed"/>
        <date variable="accessed">...</date>
      </group>
    </if>
  </choose>
</macro>
```

renders `[accessed …]`, not `[cited …]` — `term.accessed` is a distinct,
already-existing `GeneralTerm` variant
(`crates/citum-schema-style/src/locale/message_ids.rs:28`), not an alias for
`term.cited`. NLM's and springer's own macros both use `term="cited"`
(confirmed above). A shared option that hardcodes one term for all three
styles gets one of them wrong. See "Cited-date label" below.

`taylor-and-francis-council-of-science-editors-author-date.csl:77-83`
(`publisher-place` macro) has the same "if absent, render a bracketed
literal instead" shape for `[place unknown]` (7 rows) — that one is
`ALTERNATIVES`-shaped
(`alternatives: [{variable: publisher-place}, {message: term.place-unknown}]`),
not a medium designator; it's listed here only because it appears in the same
oracle rows as the `[Internet]` gap and should not be conflated with it in
implementation.

## Design

**Revised (2026-09-06)** after a Codex adversarial review found the original
`exclude-types` design didn't fit T&F-CSE, and after checking the locale
vocabulary for the original `access-phrase` field. Both are fixed below by
reusing existing engine machinery instead of inventing new vocabulary.

**Revised again (2026-09-06, second review round)** after a follow-up
review found `cited-date-form` only controls date *formatting* and never
named which *term* backs the bracket — the spec had silently assumed
`term.cited` for every style, which is wrong for T&F-CSE (see "CSE also
uses a different term" in Evidence above). Added `cited-date-label` below.

**Revised again (2026-09-06, third review round)** after a follow-up review
found two more concrete defects: the wire example's field values were bare
scalars where the actual message type requires a mapping, and the proposed
Rust field lived on the wrong struct — style-authored `bibliography:
options:` deserializes into `BibliographyOptions`
(`crates/citum-schema-style/src/style/sections/bibliography.rs:32`), a
*different* type from the runtime `BibliographyConfig` this spec had been
targeting exclusively. Both fixed below.

### New option

```yaml
options:
  locale-override: en-US-nlm  # existing mechanism; overrides term.retrieved
bibliography:
  options:
    online-access:
      medium-marker: {message: term.internet}
      access-phrase: true
      cited-date-label: {message: term.cited}   # {message: term.accessed} for T&F-CSE
      cited-date-form: text
```

`medium-marker` and `cited-date-label` are mapping-shaped
(`{message: ...}`), not bare strings: `SubstituteMessage`
(`crates/citum-schema-style/src/options/substitute.rs:69-81`) is a plain
struct with `deny_unknown_fields` and no string-shorthand `Deserialize`
impl, so a scalar like `term.internet` would either fail to parse or (if
authored under a permissive container) be silently captured as an unknown
field and have no effect — exactly Codex's finding. There is precedent
elsewhere in the schema for string-shorthand message fields, but
`SubstituteMessage` specifically is not one of them; this spec uses it
as-is rather than proposing a second message-reference shape.

**Schema placement — wired in three places, following `article_journal`'s
exact precedent** (`ArticleJournalBibliographyConfig`, the shape
`ArticleJournalNoPageFallback` uses, already does this correctly):

1. `BibliographyOptions` (`crates/citum-schema-style/src/options/mod.rs:365`)
   — the authoring-time type `bibliography: options:` actually deserializes
   into. Add `online_access: Option<OnlineAccessConfig>` here, alongside the
   existing `pub article_journal: Option<ArticleJournalBibliographyConfig>`
   at the same struct.
2. `BibliographyConfig` (`crates/citum-schema-style/src/options/bibliography.rs:19`)
   — the resolved runtime type the engine actually reads. Add the same
   field here too; these are two separate struct definitions with no
   automatic field-sharing.
3. `BibliographyOptions::to_bibliography_config()`
   (`crates/citum-schema-style/src/options/mod.rs:981-1000`) — a **hand-written**,
   field-by-field conversion function, not a derive. It must gain
   `online_access: self.online_access.clone(),` explicitly, in the same
   pattern as its existing `article_journal: self.article_journal.clone(),`
   line, or an authored `online-access:` block is silently dropped between
   authoring and runtime — never reaching the engine at all, with no error.
   This is the mechanism Codex's finding warned about, confirmed by reading
   the function directly rather than assuming a merge path exists.

Proposed schema shape:

- `BibliographyOptions.online_access` and `BibliographyConfig.online_access`:
  both `Option<OnlineAccessConfig>`, identical shape, per the wiring above.
- `OnlineAccessConfig`:
  - `medium_marker: Option<SubstituteMessage>` — locale message rendered
    bracketed, capitalized-first, anchored per "Anchor selection" below.
  - `access_phrase: Option<bool>` — when `true`, compose `term.retrieved` +
    `term.from` immediately before the rendered URL (see "Access phrase"
    below); when `false`/`None`, the URL renders on its own, as it does for
    most styles today. This is a plain toggle — it does *not* carry the
    wording itself. The wording ("retrieved" vs. NLM's "available") is
    supplied entirely by the style's existing, general-purpose
    `options.locale-override` field
    (`crates/citum-schema-style/src/options/mod.rs:114`), not by a new field
    here: whether a style shows an access phrase and what word it uses are
    orthogonal, and only the first belongs to this bundle.
  - `cited_date_label: Option<SubstituteMessage>` — locale message naming the
    term inside the accessed-date bracket (see "Cited-date label" below).
    `None` disables the bracket even when a URL exists, regardless of
    `cited_date_form`.
  - `cited_date_form: Option<DateForm>` — form for the accessed-date bracket;
    only meaningful when `cited_date_label` is set.

### Anchor selection

**Revised twice.** First from a flat `exclude-types` list (didn't fit
T&F-CSE's data-presence gate) to `container_title_category(ref_type) !=
Default`, a type classification. A third review round found that
classification still knowingly diverges from real reference shapes — a
container-less `chapter`, or a non-listed type authored with a populated
`container-title`, would pick the wrong anchor — and correctly pointed out
that a *type* proxy shouldn't stand in when the *actual data being
anchored to* is directly checkable.

It is: `Reference::container_title()`
(`crates/citum-schema-data/src/reference/accessors.rs:1108`) returns
`Option<Title>` — the exact data T&F-CSE's own rule tests
(`<if variable="container-title" match="none">`). The anchor rule is
`reference.container_title().is_some()`: attach the marker to the
container-title component when it exists, to the reference's own title
otherwise. This is not a proxy for the real condition, it *is* the real
condition — the same accessor the container-title template component
itself would consult. For NLM/springer's own listed types
(article-journal/magazine/newspaper, chapter, paper-conference) this
subsumes the type-based rule as a consequence, not a coincidence: those are
exactly the types given an embedded container `WorkRelation`, so
`container_title()` returns `Some` for them via `p.title()`.

**New residual risk found while verifying this fix, narrower than the one
it replaces:** `container_title()` also returns `Some` for
`ClassExtension::LegalCase`/`Statute`/`Regulation`/`Treaty`
(`accessors.rs:1124-1127`) — from a flat `reporter`/`code` string field, not
an embedded work's title. NLM's own title macro excludes only its 5
periodical/monograph type list from the marker
(`match="none"` over that exact list) — bill, legislation, and the other
legal types are *not* excluded, so NLM's real behavior puts the marker on
their own title, not a container. A bare `container_title().is_some()`
check would misroute these four legal classes to the container anchor.
**Fix:** gate the container anchor on `container_title().is_some()` *and*
the container coming from an embedded `WorkRelation` with its own title
(i.e., exclude `LegalCase`/`Statute`/`Regulation`/`Treaty` explicitly, or
add a second accessor that distinguishes "container is an embedded titled
work" from "container is a flat reporter/code string"). Not yet decided
which; either is a small, mechanical addition. Flagged in Acceptance
Criteria as a fixture check covering at least one legal-type reference
(e.g. `TLIB-SEL-BILL-1`, already used elsewhere in this wave's fixtures)
before this anchor rule is treated as correct.

### Access phrase

**Revised from an invented `access-phrase: term.available-from`.** No such
term exists (checked `crates/citum-schema-style/src/locale/message_ids.rs`);
`pattern.retrieved-from`/`pattern.available-at` exist but carry different
wording ("retrieved from" / "available at") than NLM's "Available from".

The shipped CSL doesn't invent a phrase either — it locally overrides an
*existing* term:

```xml
<locale xml:lang="en"><terms><term name="retrieved">available</term></terms></locale>
<macro name="access">
  <text term="retrieved" text-case="capitalize-first"/>
  <text term="from"/>
  ...
  <text variable="URL"/>
</macro>
```

`term.retrieved` and `term.from` both already exist
(`crates/citum-schema-style/src/locale/message_ids.rs:29-36`), and the
project already has the exact mechanism for a style-scoped term override:
`crates/citum-schema-style/embedded/locales/overrides/en-US-chicago.yaml`
and the `en-US-ieee`/`en-US-springer` precedent (`csl26-fz2e`) both define a
`messages:` block on a `locale-override` file, referenced from a style via
the existing, general-purpose `options.locale-override` field. NLM/springer/
CSE need a new file of that same kind — e.g.
`locales/overrides/en-US-nlm.yaml` with `messages: {term.retrieved:
"available"}` — set via their own `options.locale-override`, no schema
change required for the wording itself. `access_phrase: true` (this spec's
only new field for this piece) then composes `term.retrieved` + `term.from`
immediately before the pre-existing `variable: url` component, reading
whatever the active locale (overridden or not) resolves those two terms to
— not a new `{$url}`-argument pattern. `csl26-9l88` (open question about
full-locale-replacement semantics) is unrelated to this narrower per-message
overlay, which has two working precedents already; worth a one-line sanity
check before implementation, not a blocker.

### Cited-date label

**Added (2026-09-06, second review round).** NLM and springer both name the
term `term.cited`; T&F-CSE's own macro of the same name (`cited`) renders
`term.accessed` instead (see Evidence). These are two distinct, already-
existing `GeneralTerm` variants
(`crates/citum-schema-style/src/locale/message_ids.rs:28-29`) — not a
formatting difference `cited_date_form` could express, and not a case where
one style is "wrong" and can be normalized to the other's wording. Each
style sets `cited_date_label` to name its own term:

```yaml
# NLM, springer-vancouver-brackets:
online-access:
  cited-date-label: {message: term.cited}
# T&F-CSE:
online-access:
  cited-date-label: {message: term.accessed}
```

No locale override needed for this piece — both terms already exist
verbatim in the base `en-US` locale.

### Semantics

When a reference has a URL:

1. The medium marker renders ` [<medium_marker>]` (bracketed,
   capitalized-first) on the container-title or the title component per
   "Anchor selection".
2. If `access_phrase: true`, the access rendering composes `term.retrieved`
   + `term.from` (whatever the active locale, overridden or not, resolves
   them to) before `variable: url`, matching the shipped `": "` / `" "`
   delimiters.
3. If `cited_date_label` is set, the accessed-date component wraps in
   `[<cited_date_label> …]` using `cited_date_form` — `term.cited` for
   NLM/springer, `term.accessed` for CSE, per "Cited-date label" above.

When a reference has no URL, none of the three render — matching today's
behavior, and unaffected for styles that don't set this option at all.

## Implementation Notes

Independent of the `render-when` disposition — this is a new domain-specific
bibliography option, not a candidate for `alternatives:` or `render-when`.
NLM's DOI-instead-of-detail-block rule for page/volume-less
`article-journal` entries is a *separate* concern with no URL/medium
involvement — see `csl26-8z39` (extending `ArticleJournalNoPageFallback`),
not this option.

## Acceptance Criteria

- [ ] Schema: `OnlineAccessConfig` added to **both** `BibliographyOptions`
      (`options/mod.rs:365`) and `BibliographyConfig`
      (`options/bibliography.rs:19`), plus an explicit
      `online_access: self.online_access.clone(),` line in
      `BibliographyOptions::to_bibliography_config()` (`options/mod.rs:981`)
      — verified with a round-trip test analogous to
      `test_article_journal_no_page_fallback_roundtrip`, not just schema
      validation, so a style-authored value is confirmed to actually reach
      the runtime config.
- [ ] Schema: `medium_marker`/`cited_date_label` deserialize only in mapping
      form (`{message: ...}`); a bare scalar is rejected or fails to parse,
      not silently captured as an unknown field.
- [ ] Engine: medium-marker anchor selection via
      `reference.container_title().is_some()`, excluding
      `LegalCase`/`Statute`/`Regulation`/`Treaty` from the container anchor
      (see "Anchor selection" residual risk) — access-phrase composition and
      cited-date-bracket wiring, each independently gated on URL presence.
- [ ] Fixture check covering at least one legal-type reference (e.g.
      `TLIB-SEL-BILL-1`) confirming the marker anchors on the reference's
      own title, not a container, before treating the anchor-selection
      exclusion as correct.
- [ ] New locale-override file (e.g. `en-US-nlm.yaml`) overriding
      `term.retrieved`, following the `en-US-ieee`/`en-US-springer`
      precedent.
- [ ] Exact-output fixture check, per style, confirming the accessed-date
      bracket text: `[cited …]` for NLM and springer, `[accessed …]` for
      CSE — not asserted from reading the CSL, verified against a real
      rendered fixture for each of the three, before treating
      `cited_date_label` as correct.
- [ ] `taylor-and-francis-national-library-of-medicine-core.yaml`,
      `springer-vancouver-brackets-core.yaml`, and
      `taylor-and-francis-council-of-science-editors-author-date-core.yaml`
      updated; `report-core.js --diff` shows the targeted `[Internet]`/
      `[cited …]`/`[accessed …]` rows flip with 0 regressions. (T&F-NLM's
      DOI rows are `csl26-8z39`'s scope, not this option's — don't count
      them here.)
- [ ] `just schema-gen` run, schema docs updated.
- [ ] Status promoted to Active in the implementation commit.

## Changelog

- v1.3 (2026-09-06): Corrected per a third Codex adversarial-review round:
  the wire example used bare-scalar message fields where `SubstituteMessage`
  requires a mapping (`{message: ...}`), and the proposed field lived only
  on `BibliographyConfig` when style-authored `bibliography: options:`
  actually deserializes into a separate type, `BibliographyOptions` — fixed
  by wiring `online_access` into both types plus the hand-written
  `to_bibliography_config()` conversion, following `article_journal`'s
  exact precedent. Replaced the `container_title_category` anchor rule with
  the actual data accessor, `Reference::container_title().is_some()` —
  which in turn surfaced a narrower, previously-unnoticed edge case (legal
  reference types populate `container_title()` from a flat reporter/code
  field, not an embedded work, and NLM's real behavior doesn't treat them
  as container-anchored), now flagged precisely rather than hidden behind
  the classification it replaced. See `csl26-ro72`.
- v1.2 (2026-09-06): Corrected per a second Codex adversarial-review round:
  the shared `cited_date_form` field only controlled formatting and silently
  assumed `term.cited` for every style; T&F-CSE's own macro (also confusingly
  named `cited`) actually renders `term.accessed`. Added `cited_date_label`
  so each style names its own term, and an exact-output fixture check to
  Acceptance Criteria rather than asserting the wording from reading the CSL
  alone. See `csl26-8b4a`.
- v1.1 (2026-09-06): Corrected per a Codex adversarial review and follow-up
  verification: replaced the flat `exclude-types` list with the engine's
  existing `container_title_category` classification (added a documented
  residual-risk check against T&F-CSE's data-presence gate); replaced the
  invented `access-phrase: term.available-from` field with a plain
  `access_phrase: bool` toggle composing the existing `term.retrieved` +
  `term.from`, with wording supplied by the pre-existing
  `options.locale-override` mechanism rather than a new field; corrected the
  cross-reference to `ArticleJournalNoPageFallback` (extend it directly,
  `csl26-8z39` — not a candidate for generalization via `alternatives:`).
  See `csl26-8b4a`.
- v1.0 (2026-09-06): Initial draft.
